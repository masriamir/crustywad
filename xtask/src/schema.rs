//! Output record types and deterministic writers (DESIGN.md §4.7).
//!
//! Determinism contract (§9.3): `harvest-manifest.json` is the ONLY output
//! carrying wall-clock timestamps. `idgames-files.jsonl`,
//! `harvest-errors.jsonl`, and `idgames-wads.jsonl` are sorted and
//! timestamp-free so a rerun against unchanged inputs is byte-identical.

use std::path::Path;

use anyhow::Context as _;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::api::model::FileRecord;
use crate::cache::atomic_write;

/// Run provenance (§4.7): "statistics without provenance are not
/// reproducible". Downstream phases reference [`Self::id`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarvestManifest {
    /// Stable run identifier derived from the start time.
    pub id: String,
    /// RFC 3339 run start.
    pub started_at: String,
    /// Wall-clock run duration.
    pub duration_secs: u64,
    /// `meta.version` observed on parsed response envelopes this run —
    /// live or cache-fresh (3 as spike-verified). `0` means no envelope
    /// was parsed at all (unknown).
    pub api_version: u64,
    /// `CARGO_PKG_VERSION` of xtask.
    pub tool_version: String,
    /// `git rev-parse --short HEAD`, when available.
    pub git_rev: Option<String>,
    /// Bootstrap provenance label ([`crate::mirror::BootstrapSource::label`]).
    pub bootstrap: String,
    /// Traversal roots (§4.2 include set, or the `--root` override).
    pub roots: Vec<String>,
    /// `--root` value for dev-scoped runs (`None` on full harvests).
    pub scoped_root: Option<String>,
    /// `--limit` value for dev-scoped runs.
    pub limit: Option<u64>,
    /// Directories enumerated.
    pub dir_count: u64,
    /// File records written.
    pub file_count: u64,
    /// Ledger entries written.
    pub error_count: u64,
    /// `getcontents` calls served from the disk cache.
    pub cache_hits: u64,
    /// Live API requests made.
    pub live_api_calls: u64,
    /// Highest archive file id seen — the `latestfiles` probe baseline.
    pub max_file_id: Option<u64>,
}

/// Failure-ledger category (§4.6/§5.5 discipline: record, don't skip).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum LedgerKind {
    /// Exchange-level failure: retry policy exhausted, a non-retryable
    /// HTTP status, a body-stage failure (over-cap or mid-body transport
    /// error), or the API's own error envelope.
    HttpError,
    /// `content.file` and `content.dir` both null (§4.1).
    SuspectPath,
    /// Bytes arrived whole but did not deserialize: an unrecognized
    /// envelope shape or a record that failed deserialization.
    ParseError,
    /// API `size` disagrees with the ls-laR listing size (§5.0 guard).
    SizeMismatch,
}

/// One `harvest-errors.jsonl` line. Deliberately timestamp-free (§4.7).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    /// Archive path the failure concerns.
    pub path: String,
    /// API action (or `"ls-laR"` / `"latestfiles"`).
    pub action: String,
    /// Category.
    pub kind: LedgerKind,
    /// Human-readable detail.
    pub detail: String,
    /// Attempts made before giving up (1 for non-retried findings).
    pub attempts: u32,
}

/// Write `idgames-files.jsonl`: records sorted by `id`, deduped by `id`
/// (last occurrence wins), one compact JSON object per line.
///
/// # Errors
/// Serialization or filesystem failure.
pub fn write_files_jsonl(path: &Path, records: Vec<FileRecord>) -> anyhow::Result<u64> {
    let mut by_id = std::collections::BTreeMap::new();
    for rec in records {
        by_id.insert(rec.id, rec);
    }
    let mut out = String::new();
    for rec in by_id.values() {
        out.push_str(&serde_json::to_string(rec).context("serializing file record")?);
        out.push('\n');
    }
    atomic_write(path, out.as_bytes()).with_context(|| format!("writing {}", path.display()))?;
    Ok(u64::try_from(by_id.len()).expect("record count fits u64"))
}

/// Write `harvest-errors.jsonl`, sorted for determinism.
///
/// # Errors
/// Serialization or filesystem failure.
pub fn write_ledger(path: &Path, mut entries: Vec<LedgerEntry>) -> anyhow::Result<u64> {
    // The key is the full record: a partial key would leave ties to the
    // (insertion-order-dependent) stable sort and break the §9.3
    // byte-identical rerun contract for duplicate findings.
    entries.sort_by(|a, b| {
        (&a.path, &a.action, &a.kind, &a.detail, a.attempts)
            .cmp(&(&b.path, &b.action, &b.kind, &b.detail, b.attempts))
    });
    let mut out = String::new();
    for e in &entries {
        out.push_str(&serde_json::to_string(e).context("serializing ledger entry")?);
        out.push('\n');
    }
    atomic_write(path, out.as_bytes()).with_context(|| format!("writing {}", path.display()))?;
    Ok(u64::try_from(entries.len()).expect("entry count fits u64"))
}

/// Write the manifest as pretty JSON with a trailing newline.
///
/// # Errors
/// Serialization or filesystem failure.
pub fn write_manifest(path: &Path, manifest: &HarvestManifest) -> anyhow::Result<()> {
    let mut bytes = serde_json::to_vec_pretty(manifest).context("serializing manifest")?;
    bytes.push(b'\n');
    atomic_write(path, &bytes).with_context(|| format!("writing {}", path.display()))
}

/// Read a previous run's manifest, if present and parseable.
pub fn read_manifest(path: &Path) -> Option<HarvestManifest> {
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

/// Read a previous run's `idgames-files.jsonl` as the tree-diff baseline.
/// `None` when the file is missing/unreadable; unparseable lines are
/// skipped with a warning rather than failing the read (a damaged
/// baseline degrades to broader invalidation, never to a crash).
pub fn read_files_jsonl(path: &Path) -> Option<Vec<FileRecord>> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut records = Vec::new();
    let mut skipped = 0_u64;
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        match serde_json::from_str(line) {
            Ok(rec) => records.push(rec),
            Err(_) => skipped += 1,
        }
    }
    if skipped > 0 {
        tracing::warn!(skipped, path = %path.display(), "skipped unparseable baseline lines");
    }
    Some(records)
}

/// `"harvest-YYYYMMDDTHHMMSSZ"` from the run start.
pub fn manifest_id(started_at: &DateTime<Utc>) -> String {
    format!("harvest-{}", started_at.format("%Y%m%dT%H%M%SZ"))
}

/// Best-effort short git revision for provenance. Anchored to the xtask
/// manifest dir, not the process CWD — the tool works from any cwd
/// (compile-time paths everywhere else), and provenance must too.
pub fn git_rev() -> Option<String> {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

/// xtask package version for provenance.
pub fn tool_version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

/// §5.6 closed enum — every §5.5 edge case gets a named value, or "record,
/// don't skip" is unenforceable. Do not add variants without a DESIGN §5.6
/// correction in the same commit.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FetchStatus {
    /// Central directory read over ranges; sizes recorded.
    Ok,
    /// The entry's filename is not `.zip` — no mirror contact at all.
    NotZip,
    /// Both mirrors answered 404 for the entry.
    ///
    /// `rename_all = "snake_case"` alone renders this as `mirror404_all`
    /// (serde splits on digit-to-uppercase but not letter-to-digit
    /// transitions) — an explicit rename pins the §5.6 wire value.
    #[serde(rename = "mirror_404_all")]
    Mirror404All,
    /// Both mirrors refused ranges and the budgeted fallback was not taken.
    NoRangeSupport,
    /// Both mirrors refused ranges; sizes come from a budgeted full download.
    FullDownload,
    /// Bytes arrived but the zip central directory did not parse (or hit a cap).
    ZipParseError,
    /// Transport-level failure after retries on every usable mirror.
    FetchError,
}

/// One `.wad` member of an archive entry (§5.6 `wads[]`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WadMember {
    /// Member name exactly as the central directory declares it.
    pub name: String,
    /// Declared compressed size in bytes.
    pub compressed: u64,
    /// Declared uncompressed size in bytes — the number this phase exists for.
    pub uncompressed: u64,
    /// Compression method label (`"stored"`, `"deflate"`, or another §5.5
    /// method name).
    pub method: String,
    /// General-purpose-bit-0 encryption flag (§5.5).
    pub encrypted: bool,
}

/// One `idgames-wads.jsonl` line (§5.6 — field order here is the output
/// schema). `date`/`rating`/`votes` are copied from the Phase-1 record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WadRecord {
    /// Archive file ID (Phase-1 `FileRecord::id`).
    pub id: u64,
    /// Directory path with trailing slash.
    pub dir: String,
    /// Zip filename, no path.
    pub filename: String,
    /// Zip size in bytes (Phase-1 `size`).
    pub zip_size: u64,
    /// `YYYY-MM-DD` from Phase 1.
    pub date: String,
    /// Mean rating from Phase 1; `None` when unrated.
    pub rating: Option<f64>,
    /// Vote count from Phase 1.
    pub votes: u64,
    /// Whether the entry is a zip (filename `.zip`, ASCII case-insensitive).
    pub is_zip: bool,
    /// ZIP64 EOCD locator present (§5.3) — detected from the fetched tail.
    pub zip64: bool,
    /// Count of *distinct* central-directory entries by name (directories
    /// included): zip 8's `ZipArchive` keys parsed entries in an
    /// `IndexMap<name, ..>` (last-one-wins on a duplicate name), so an
    /// archive with duplicate member names under-counts here by design —
    /// this is what `zip` itself exposes, not the raw CD record count on
    /// disk.
    pub member_count: u64,
    /// `.wad` members, case-insensitively matched (§5.5).
    pub wads: Vec<WadMember>,
    /// Every non-`.wad` member name (nested archives §5.5 appear here) —
    /// plus any `.wad` member whose local file header could not be read
    /// (genuinely unreadable, not a range-cache miss): its real
    /// central-directory name lands here rather than in `wads`, since a
    /// size can't be fabricated for it (see
    /// `zips::inspect::inspection_from_archive`). Phase-3 consumers must
    /// not assume every name here is wad-free.
    pub other_members: Vec<String>,
    /// Mirror key that served the bytes; `""` when no mirror was contacted.
    pub mirror: String,
    /// Closed §5.6 outcome enum.
    pub fetch_status: FetchStatus,
}

/// Write `idgames-wads.jsonl`: sorted by `id`, deduped by `id` (last
/// occurrence wins), one compact JSON object per line.
///
/// # Errors
/// Serialization or filesystem failure.
pub fn write_wads_jsonl(path: &Path, records: Vec<WadRecord>) -> anyhow::Result<u64> {
    let mut by_id = std::collections::BTreeMap::new();
    for rec in records {
        by_id.insert(rec.id, rec);
    }
    let mut out = String::new();
    for rec in by_id.values() {
        out.push_str(&serde_json::to_string(rec).context("serializing wad record")?);
        out.push('\n');
    }
    atomic_write(path, out.as_bytes()).with_context(|| format!("writing {}", path.display()))?;
    Ok(u64::try_from(by_id.len()).expect("record count fits u64"))
}

/// Phase-2 run provenance and the §9.3 acceptance witnesses: byte totals
/// (small fraction of the archive or the range reader is broken), ZIP64
/// count (≥1 resolved or absence stated), fallback/budget accounting, and
/// the completeness counter. The only Phase-2 output with wall-clock data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZipsManifest {
    /// `"harvest-zips-YYYYMMDDTHHMMSSZ"` from the run start.
    pub id: String,
    /// RFC 3339 run start.
    pub started_at: String,
    /// Wall-clock run duration.
    pub duration_secs: u64,
    /// `CARGO_PKG_VERSION` of xtask.
    pub tool_version: String,
    /// `git rev-parse --short HEAD`, when available.
    pub git_rev: Option<String>,
    /// `--root` value for dev-scoped runs.
    pub scoped_root: Option<String>,
    /// `--limit` value for dev-scoped runs.
    pub limit: Option<u64>,
    /// Phase-1 entries in this run's worklist.
    pub entries_total: u64,
    /// `idgames-wads.jsonl` lines written.
    pub records_written: u64,
    /// `wads-errors.jsonl` lines written.
    pub ledger_count: u64,
    /// Entries served from the per-id results log.
    pub cache_hits: u64,
    /// Entries drained from the live mirror pool this run. On an aborted
    /// run this counts only entries that finished before the breaker
    /// tripped — in-flight entries cancelled by the abort are not counted
    /// (they also never get a record; see `aborted`).
    pub live_entries: u64,
    /// Ranged/full GET requests issued this run.
    pub range_requests: u64,
    /// Total response-body bytes read from mirrors this run (§9.3: must
    /// stay a small fraction of the archive's ~38 GiB).
    pub bytes_transferred: u64,
    /// Entries resolved via the budgeted full-download fallback this run
    /// (a cache-hit entry that was a full download on a *prior* run is not
    /// recounted here — this mirrors `range_requests`/`bytes_transferred`'s
    /// run-scoped semantics, not a whole-corpus total).
    pub full_downloads: u64,
    /// Bytes consumed by the fallback this run (bounded by the 2 GiB
    /// budget — see `full_downloads` on why this is run-scoped, not
    /// cumulative across warm reruns).
    pub fallback_bytes: u64,
    /// Records with `zip64: true` (0 explicitly states ZIP64 absence, §9.3).
    pub zip64_entries: u64,
    /// Record count per `fetch_status` value.
    pub status_counts: std::collections::BTreeMap<String, u64>,
    /// Worklist entries with no `idgames-wads.jsonl` record — computed from
    /// `records` alone (every failure gets a record too, via the
    /// ledger-writing branch, so this is not "records ∪ ledger" despite
    /// what the name suggests). §9.3 demands 0 on a completed run. It is
    /// EXPECTED to be nonzero on an aborted run (`aborted.is_some()`): the
    /// entries still in flight when a breaker trips are cancelled without
    /// ever getting a record. On a non-aborted run, anything else is a bug
    /// surfaced loudly.
    pub unaccounted_entries: u64,
    /// `Some(reason)` when either abort breaker stopped the phase early:
    /// the ~2% fallback circuit breaker (§5.2), or the 4 GiB range-path
    /// byte ceiling (`RANGE_BYTE_CEILING` in `zips/mod.rs`, tripped by a
    /// pathological EOCD/central-directory shape that pushes payload down
    /// the range path invisibly to the fallback budget). Outputs then
    /// cover only the completed prefix.
    pub aborted: Option<String>,
}

/// Write the phase-2 manifest as pretty JSON with a trailing newline.
///
/// # Errors
/// Serialization or filesystem failure.
pub fn write_zips_manifest(path: &Path, manifest: &ZipsManifest) -> anyhow::Result<()> {
    let mut bytes = serde_json::to_vec_pretty(manifest).context("serializing zips manifest")?;
    bytes.push(b'\n');
    atomic_write(path, &bytes).with_context(|| format!("writing {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::model::FileRecord;

    fn record(id: u64) -> FileRecord {
        serde_json::from_value(serde_json::json!({
            "id": id, "title": "T", "dir": "levels/doom/0-9/",
            "filename": format!("f{id}.zip"), "size": 10 * id, "age": 0,
            "date": "2003-06-02", "author": "A", "email": "drop@me.invalid",
            "description": "d", "rating": null, "votes": 0,
            "url": "", "idgamesurl": ""
        }))
        .unwrap()
    }

    #[test]
    fn files_jsonl_is_sorted_deduped_and_email_free() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("idgames-files.jsonl");
        let n = write_files_jsonl(&p, vec![record(3), record(1), record(3), record(2)]).unwrap();
        assert_eq!(n, 3);
        let text = std::fs::read_to_string(&p).unwrap();
        let ids: Vec<u64> = text
            .lines()
            .map(|l| {
                serde_json::from_str::<serde_json::Value>(l).unwrap()["id"]
                    .as_u64()
                    .unwrap()
            })
            .collect();
        assert_eq!(ids, vec![1, 2, 3]);
        // §9.3: no email-shaped field in any output line.
        for line in text.lines() {
            let v: serde_json::Value = serde_json::from_str(line).unwrap();
            crate::api::model::tests::assert_no_email_keys(&v);
        }
    }

    #[test]
    fn files_jsonl_reruns_are_byte_identical() {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a.jsonl");
        let b = tmp.path().join("b.jsonl");
        write_files_jsonl(&a, vec![record(2), record(9), record(4)]).unwrap();
        write_files_jsonl(&b, vec![record(4), record(2), record(9)]).unwrap();
        assert_eq!(std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());
    }

    #[test]
    fn ledger_ties_on_partial_key_are_still_deterministic() {
        // Entries agreeing on (path, kind, detail) but differing in
        // attempts/action must not fall back to insertion order.
        let tmp = tempfile::tempdir().unwrap();
        let a_path = tmp.path().join("a.jsonl");
        let b_path = tmp.path().join("b.jsonl");
        let first = LedgerEntry {
            path: "levels/x/".into(),
            action: "getcontents".into(),
            kind: LedgerKind::HttpError,
            detail: "HTTP 500".into(),
            attempts: 2,
        };
        let second = LedgerEntry {
            attempts: 6,
            ..first.clone()
        };
        write_ledger(&a_path, vec![first.clone(), second.clone()]).unwrap();
        write_ledger(&b_path, vec![second, first]).unwrap();
        assert_eq!(
            std::fs::read(&a_path).unwrap(),
            std::fs::read(&b_path).unwrap()
        );
    }

    #[test]
    fn ledger_is_sorted_and_timestamp_free() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("harvest-errors.jsonl");
        let entries = vec![
            LedgerEntry {
                path: "levels/z/".into(),
                action: "getcontents".into(),
                kind: LedgerKind::HttpError,
                detail: "HTTP 500".into(),
                attempts: 6,
            },
            LedgerEntry {
                path: "levels/a/".into(),
                action: "getcontents".into(),
                kind: LedgerKind::SuspectPath,
                detail: "file and dir both null".into(),
                attempts: 1,
            },
        ];
        let n = write_ledger(&p, entries).unwrap();
        assert_eq!(n, 2);
        let text = std::fs::read_to_string(&p).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert!(lines[0].contains("levels/a/"));
        assert!(lines[1].contains("levels/z/"));
        // §4.7: no wall-clock in this output — spot-check field names.
        for line in &lines {
            let v: serde_json::Value = serde_json::from_str(line).unwrap();
            let keys: Vec<&String> = v.as_object().unwrap().keys().collect();
            assert!(
                !keys
                    .iter()
                    .any(|k| k.contains("time") || k.contains("date") || k.ends_with("at")),
                "wall-clock-shaped key in ledger: {keys:?}"
            );
        }
    }

    #[test]
    fn ledger_kind_serializes_snake_case() {
        let v = serde_json::to_value(LedgerKind::SizeMismatch).unwrap();
        assert_eq!(v, "size_mismatch");
    }

    #[test]
    fn files_jsonl_roundtrips_through_reader() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("idgames-files.jsonl");
        write_files_jsonl(&p, vec![record(5), record(2)]).unwrap();
        let back = read_files_jsonl(&p).unwrap();
        assert_eq!(back.len(), 2);
        assert_eq!(back[0].id, 2);
        assert_eq!(back[1].id, 5);
        assert!(read_files_jsonl(&tmp.path().join("missing.jsonl")).is_none());
        // A corrupt line is skipped, not fatal.
        std::fs::write(&p, "{ not json\n").unwrap();
        let salvaged = read_files_jsonl(&p).map(|v| v.len());
        assert_eq!(salvaged, Some(0));
    }

    #[test]
    fn manifest_roundtrips_and_id_format() {
        let started = chrono::DateTime::parse_from_rfc3339("2026-08-15T12:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        assert_eq!(manifest_id(&started), "harvest-20260815T120000Z");
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("harvest-manifest.json");
        let m = HarvestManifest {
            id: manifest_id(&started),
            started_at: started.to_rfc3339(),
            duration_secs: 480,
            api_version: 3,
            tool_version: tool_version(),
            git_rev: git_rev(),
            bootstrap: "ls-lar-fresh:infania".into(),
            roots: vec!["levels/".into()],
            scoped_root: None,
            limit: None,
            dir_count: 462,
            file_count: 21_375,
            error_count: 0,
            cache_hits: 0,
            live_api_calls: 463,
            max_file_id: Some(22_083),
        };
        write_manifest(&p, &m).unwrap();
        let back = read_manifest(&p).unwrap();
        assert_eq!(back.id, m.id);
        assert_eq!(back.max_file_id, Some(22_083));
        assert!(std::fs::read_to_string(&p).unwrap().ends_with('\n'));
        assert!(read_manifest(&tmp.path().join("missing.json")).is_none());
    }

    fn wad_record(id: u64) -> WadRecord {
        WadRecord {
            id,
            dir: "levels/doom2/Ports/megawads/".into(),
            filename: format!("f{id}.zip"),
            zip_size: 3_145_728,
            date: "2019-04-02".into(),
            rating: Some(4.61),
            votes: 38,
            is_zip: true,
            zip64: false,
            member_count: 3,
            wads: vec![WadMember {
                name: "EXAMPLE.WAD".into(),
                compressed: 3_102_841,
                uncompressed: 14_680_064,
                method: "deflate".into(),
                encrypted: false,
            }],
            other_members: vec!["EXAMPLE.TXT".into(), "README.MD".into()],
            mirror: "infania".into(),
            fetch_status: FetchStatus::Ok,
        }
    }

    #[test]
    fn wads_jsonl_is_sorted_deduped_and_matches_design_5_6() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("idgames-wads.jsonl");
        let n = write_wads_jsonl(&p, vec![wad_record(9), wad_record(3), wad_record(9)]).unwrap();
        assert_eq!(n, 2);
        let text = std::fs::read_to_string(&p).unwrap();
        let first: serde_json::Value = serde_json::from_str(text.lines().next().unwrap()).unwrap();
        assert_eq!(first["id"], 3);
        // §5.6 field presence, exact names.
        for key in [
            "id",
            "dir",
            "filename",
            "zip_size",
            "date",
            "rating",
            "votes",
            "is_zip",
            "zip64",
            "member_count",
            "wads",
            "other_members",
            "mirror",
            "fetch_status",
        ] {
            assert!(first.get(key).is_some(), "missing §5.6 key {key}");
        }
        assert_eq!(first["wads"][0]["uncompressed"], 14_680_064_u64);
        assert_eq!(first["fetch_status"], "ok");
    }

    #[test]
    fn wads_jsonl_reruns_are_byte_identical() {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a.jsonl");
        let b = tmp.path().join("b.jsonl");
        write_wads_jsonl(&a, vec![wad_record(2), wad_record(7)]).unwrap();
        write_wads_jsonl(&b, vec![wad_record(7), wad_record(2)]).unwrap();
        assert_eq!(std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());
    }

    #[test]
    fn fetch_status_serializes_the_closed_snake_case_enum() {
        // §5.6: closed enum, every §5.5 edge case named.
        let cases = [
            (FetchStatus::Ok, "ok"),
            (FetchStatus::NotZip, "not_zip"),
            (FetchStatus::Mirror404All, "mirror_404_all"),
            (FetchStatus::NoRangeSupport, "no_range_support"),
            (FetchStatus::FullDownload, "full_download"),
            (FetchStatus::ZipParseError, "zip_parse_error"),
            (FetchStatus::FetchError, "fetch_error"),
        ];
        for (status, wire) in cases {
            assert_eq!(serde_json::to_value(status).unwrap(), wire);
        }
    }

    #[test]
    fn zips_manifest_roundtrips_and_writes_trailing_newline() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("wads-manifest.json");
        let m = ZipsManifest {
            id: "harvest-zips-20260816T120000Z".into(),
            started_at: "2026-08-16T12:00:00+00:00".into(),
            duration_secs: 1800,
            tool_version: tool_version(),
            git_rev: git_rev(),
            scoped_root: None,
            limit: None,
            entries_total: 21_375,
            records_written: 21_375,
            ledger_count: 12,
            cache_hits: 20_000,
            live_entries: 1_375,
            range_requests: 2_923,
            bytes_transferred: 190_000_000,
            full_downloads: 2,
            fallback_bytes: 40_000_000,
            zip64_entries: 1,
            status_counts: std::collections::BTreeMap::from([("ok".to_owned(), 21_300)]),
            unaccounted_entries: 0,
            aborted: None,
        };
        write_zips_manifest(&p, &m).unwrap();
        let text = std::fs::read_to_string(&p).unwrap();
        assert!(text.ends_with('\n'));
        let back: ZipsManifest = serde_json::from_str(&text).unwrap();
        assert_eq!(back.entries_total, 21_375);
        assert_eq!(back.zip64_entries, 1);
        assert_eq!(back.aborted, None);
    }
}
