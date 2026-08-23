//! ls-laR.gz listing parser — the §5.0 mirror bootstrap tree.
//!
//! One ~418 KB gzipped `ls -laR` from a §5.1 mirror yields the full
//! directory tree, every filename, and every zip's byte size. This parser
//! is deliberately forgiving: a malformed line is counted and skipped,
//! never fatal (record, don't die).

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader};

/// One regular file in the mirror listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeFile {
    /// Filename (no path). May contain spaces.
    pub name: String,
    /// Byte size as listed by the mirror.
    pub size: u64,
}

/// The archive tree from one `ls-laR.gz` listing. Directory keys carry a
/// trailing slash and are archive-root-relative (`"levels/doom/"`); the
/// archive root itself is the empty key.
#[derive(Debug, Default)]
pub struct ArchiveTree {
    /// Directory path → regular files listed in it (sorted map for
    /// deterministic iteration).
    pub dirs: BTreeMap<String, Vec<TreeFile>>,
    /// Count of unparseable entry lines (diagnostics only).
    pub skipped_lines: u64,
}

impl ArchiveTree {
    /// Number of `.zip` files (ASCII case-insensitive) under `prefix`.
    pub fn zip_count(&self, prefix: &str) -> u64 {
        self.dirs
            .iter()
            .filter(|(dir, _)| dir.starts_with(prefix))
            .flat_map(|(_, files)| files)
            .filter(|f| f.name.to_ascii_lowercase().ends_with(".zip"))
            .count()
            .try_into()
            .unwrap_or(u64::MAX)
    }

    /// Listed size of `file` in `dir` (trailing-slash key), if present.
    pub fn size_of(&self, dir: &str, file: &str) -> Option<u64> {
        // ASCII-case-insensitive, consistent with zip detection and the
        // tree-diff invalidation: a mirror/API case divergence must not
        // silently skip the §5.0 size cross-check.
        self.dirs
            .get(dir)?
            .iter()
            .find(|f| f.name.eq_ignore_ascii_case(file))
            .map(|f| f.size)
    }
}

/// Parse a plain-text `ls -laR` listing.
///
/// # Errors
/// Fails only on I/O errors from `reader`; malformed content is skipped
/// and counted in [`ArchiveTree::skipped_lines`].
pub fn parse_ls_lar(reader: impl BufRead) -> anyhow::Result<ArchiveTree> {
    let mut tree = ArchiveTree::default();
    let mut current: Option<String> = None;

    let mut raw = Vec::new();
    let mut reader = reader;
    loop {
        raw.clear();
        if reader.read_until(b'\n', &mut raw)? == 0 {
            break;
        }
        let line = String::from_utf8_lossy(&raw);
        let line = line.trim_end_matches(['\n', '\r']);
        if line.is_empty() || line.starts_with("total ") {
            continue;
        }
        if let Some(section) = line
            .strip_suffix(':')
            .filter(|s| *s == "." || s.starts_with("./"))
        {
            let key = match section.strip_prefix("./") {
                Some(rest) => format!("{}/", rest.trim_end_matches('/')),
                None => String::new(), // "." — the archive root
            };
            tree.dirs.entry(key.clone()).or_default();
            current = Some(key);
            continue;
        }
        let Some(dir) = &current else {
            tree.skipped_lines += 1;
            continue;
        };
        match parse_entry(line) {
            EntryLine::File(file) => tree
                .dirs
                .get_mut(dir)
                .expect("current section always inserted")
                .push(file),
            EntryLine::Ignored => {}
            EntryLine::Malformed => tree.skipped_lines += 1,
        }
    }
    Ok(tree)
}

/// Upper bound on the *decompressed* ls-laR stream. The real listing is
/// ~4 MiB decompressed; 256 MiB is generous headroom while still bounding
/// the gzip-bomb class of attack (gzip can amplify ~1000:1, so the 64 MiB
/// compressed mirror cap alone bounds nothing about memory use once
/// decompressed — ADR-0016/ADR-0030 §5 adversarial-mirror posture).
const DECOMPRESSED_CAP: u64 = 256 * 1024 * 1024;

/// Gunzip `bytes` and parse the contained listing.
///
/// # Errors
/// Fails on gzip or I/O errors, or if the decompressed stream exceeds
/// [`DECOMPRESSED_CAP`] (a truncated parse would otherwise masquerade as a
/// small, legitimate tree).
pub fn parse_ls_lar_gz(bytes: &[u8]) -> anyhow::Result<ArchiveTree> {
    parse_ls_lar_gz_with_cap(bytes, DECOMPRESSED_CAP)
}

/// [`parse_ls_lar_gz`] with an injectable cap, so tests can exercise the
/// truncation path without materializing hundreds of MiB.
///
/// # Errors
/// See [`parse_ls_lar_gz`].
fn parse_ls_lar_gz_with_cap(bytes: &[u8], cap: u64) -> anyhow::Result<ArchiveTree> {
    let decoder = flate2::read::GzDecoder::new(bytes);
    // Read one byte past the cap: if that sentinel byte is ever consumed,
    // `bounded.limit()` reaches exactly 0 and we know the true stream was
    // larger than `cap`, not merely equal to it.
    let mut bounded = std::io::Read::take(decoder, cap + 1);
    let tree = parse_ls_lar(BufReader::new(&mut bounded))?;
    if bounded.limit() == 0 {
        anyhow::bail!(
            "ls-laR.gz decompresses to more than {cap} bytes — refusing (possible gzip bomb)"
        );
    }
    Ok(tree)
}

enum EntryLine {
    File(TreeFile),
    /// Directories, symlinks, `.`/`..`, device nodes — listed but not files.
    Ignored,
    Malformed,
}

/// `mode links owner group size month day year-or-time name...` — the name
/// is everything after the 8th column and may contain spaces.
fn parse_entry(line: &str) -> EntryLine {
    let mode = match line.chars().next() {
        Some('-') => line,
        Some('d' | 'l' | 'b' | 'c' | 'p' | 's') => return EntryLine::Ignored,
        _ => return EntryLine::Malformed,
    };
    let mut rest = mode;
    let mut size: Option<u64> = None;
    for col in 0..8 {
        let trimmed = rest.trim_start();
        let end = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
        let (field, tail) = trimmed.split_at(end);
        if field.is_empty() {
            return EntryLine::Malformed;
        }
        if col == 4 {
            size = field.parse().ok();
        }
        rest = tail;
    }
    let name = rest.trim_start();
    match (size, name.is_empty()) {
        (Some(size), false) => EntryLine::File(TreeFile {
            name: name.to_owned(),
            size,
        }),
        _ => EntryLine::Malformed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
.:
total 460
drwxr-xr-x  17 ftp  ftp   4096 Aug 12 06:00 .
drwxr-xr-x  17 ftp  ftp   4096 Aug 12 06:00 ..
-rw-r--r--   1 ftp  ftp 428032 Aug 12 06:00 ls-laR.gz
drwxr-xr-x   9 ftp  ftp   4096 Aug 12 06:00 levels

./levels:
total 40
drwxr-xr-x  9 ftp ftp 4096 Aug 12 06:00 .
drwxr-xr-x 17 ftp ftp 4096 Aug 12 06:00 ..
drwxr-xr-x 30 ftp ftp 4096 Aug 12 06:00 doom

./levels/doom:
total 8
drwxr-xr-x 30 ftp ftp 4096 Aug 12 06:00 0-9

./levels/doom/0-9:
total 5678
-rw-r--r--  1 ftp ftp 552251 Jun  2  2003 example.zip
-rw-r--r--  1 ftp ftp   1024 Jun  2  2003 with spaces.zip
-rw-r--r--  1 ftp ftp    100 Jun  2  2003 EXAMPLE2.ZIP
-rw-r--r--  1 ftp ftp    100 Jun  2  2003 readme.txt
lrwxrwxrwx  1 ftp ftp     11 Jun  2  2003 alias.zip -> example.zip
this line is garbage

./empty:
total 0
drwxr-xr-x 2 ftp ftp 4096 Aug 12 06:00 .
drwxr-xr-x 17 ftp ftp 4096 Aug 12 06:00 ..
";

    fn tree() -> ArchiveTree {
        parse_ls_lar(SAMPLE.as_bytes()).unwrap()
    }

    #[test]
    fn sections_become_slash_terminated_keys() {
        let t = tree();
        assert!(t.dirs.contains_key(""));
        assert!(t.dirs.contains_key("levels/"));
        assert!(t.dirs.contains_key("levels/doom/0-9/"));
        assert!(t.dirs.contains_key("empty/"));
    }

    #[test]
    fn only_regular_files_are_recorded() {
        let t = tree();
        let names: Vec<&str> = t.dirs["levels/doom/0-9/"]
            .iter()
            .map(|f| f.name.as_str())
            .collect();
        assert_eq!(
            names,
            vec![
                "example.zip",
                "with spaces.zip",
                "EXAMPLE2.ZIP",
                "readme.txt"
            ]
        );
        // Directory sections carry their subdir entries only as sections.
        assert!(t.dirs["levels/"].is_empty());
        assert!(t.dirs["empty/"].is_empty());
    }

    #[test]
    fn sizes_and_space_names_parse() {
        let t = tree();
        assert_eq!(t.size_of("levels/doom/0-9/", "example.zip"), Some(552_251));
        assert_eq!(t.size_of("levels/doom/0-9/", "with spaces.zip"), Some(1024));
        assert_eq!(t.size_of("levels/doom/0-9/", "missing.zip"), None);
        assert_eq!(t.size_of("nope/", "example.zip"), None);
        // Case-insensitive, consistent with zip detection: an API/mirror
        // case divergence must not silently skip the size cross-check.
        assert_eq!(t.size_of("levels/doom/0-9/", "EXAMPLE.ZIP"), Some(552_251));
    }

    #[test]
    fn zip_count_is_case_insensitive_and_prefix_scoped() {
        let t = tree();
        assert_eq!(t.zip_count("levels/"), 3);
        assert_eq!(t.zip_count(""), 3);
        assert_eq!(t.zip_count("empty/"), 0);
    }

    #[test]
    fn garbage_lines_are_counted_not_fatal() {
        let t = tree();
        assert_eq!(t.skipped_lines, 1);
    }

    #[test]
    fn gz_roundtrip() {
        let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        std::io::Write::write_all(&mut enc, SAMPLE.as_bytes()).unwrap();
        let gz = enc.finish().unwrap();
        let t = parse_ls_lar_gz(&gz).unwrap();
        assert_eq!(t.zip_count(""), 3);
    }

    /// A gzip-bomb-shaped payload: a small compressed size that expands to
    /// well over a (small, test-only) cap. Written to the encoder in
    /// repeated small chunks rather than materialized in memory up front —
    /// with highly repetitive content the compressed stream stays tiny
    /// regardless of the decompressed size.
    #[test]
    fn oversized_decompressed_stream_is_rejected() {
        let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        std::io::Write::write_all(&mut enc, b".:\ntotal 1\n").unwrap();
        let line = b"-rw-r--r--  1 ftp  ftp   10 Aug 12 06:00 a.zip\n";
        // ~9.6 KiB decompressed from a handful of bytes compressed.
        for _ in 0..200 {
            std::io::Write::write_all(&mut enc, line).unwrap();
        }
        let gz = enc.finish().unwrap();
        let err = parse_ls_lar_gz_with_cap(&gz, 1024)
            .expect_err("decompressed stream exceeds the 1 KiB test cap");
        assert!(err.to_string().contains("gzip bomb"), "{err}");
    }

    #[test]
    fn decompressed_stream_exactly_at_cap_still_parses() {
        let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        std::io::Write::write_all(&mut enc, SAMPLE.as_bytes()).unwrap();
        let gz = enc.finish().unwrap();
        let cap = u64::try_from(SAMPLE.len()).unwrap();
        let t = parse_ls_lar_gz_with_cap(&gz, cap).unwrap();
        assert_eq!(t.zip_count(""), 3);
    }

    #[test]
    fn decompressed_stream_one_byte_over_cap_errors() {
        let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        std::io::Write::write_all(&mut enc, SAMPLE.as_bytes()).unwrap();
        let gz = enc.finish().unwrap();
        let cap = u64::try_from(SAMPLE.len()).unwrap() - 1;
        assert!(parse_ls_lar_gz_with_cap(&gz, cap).is_err());
    }

    #[test]
    fn non_utf8_bytes_do_not_abort() {
        let mut bytes = b".:\ntotal 1\n-rw-r--r-- 1 f f 10 Jan 1 2020 ok.zip\n".to_vec();
        bytes.extend_from_slice(b"-rw-r--r-- 1 f f 20 Jan 1 2020 bad\xFFname.zip\n");
        let t = parse_ls_lar(&bytes[..]).unwrap();
        assert_eq!(t.dirs[""].len(), 2);
    }
}
