//! Zip container reader for [`Archive`](super::Archive) (ADR-0031 §4).
//!
//! Central directory first: the end-of-central-directory record is located by
//! a bounded backward scan, the (optional) ZIP64 records are honored, and the
//! directory is walked with every offset and length bounds-checked against
//! the buffer before use. Nothing is decoded here; [`Container::read_entry`]
//! validates the local header on demand and decodes stored or deflated
//! bodies through `miniz_oxide`'s length-limited inflater, then verifies the
//! CRC-32 the directory recorded. Layout constants follow PKWARE APPNOTE
//! §4.3 and match the harvester's reader (`xtask/src/zips/inspect.rs`).

use super::{ArchiveError, Container, RawEntry};
use crate::util::crc32;

const EOCD_SIG: u32 = 0x0605_4b50;
const EOCD_LEN: usize = 22;
const MAX_COMMENT: usize = 65_535;
const ZIP64_LOCATOR_SIG: u32 = 0x0706_4b50;
const ZIP64_LOCATOR_LEN: usize = 20;
const ZIP64_EOCD_SIG: u32 = 0x0606_4b50;
const ZIP64_EOCD_LEN: usize = 56;
const CENTRAL_SIG: u32 = 0x0201_4b50;
const CENTRAL_LEN: usize = 46;
const ZIP64_EXTRA_ID: u16 = 0x0001;
const LOCAL_SIG: u32 = 0x0403_4b50;
const LOCAL_LEN: usize = 30;
const METHOD_STORED: u16 = 0;
const METHOD_DEFLATE: u16 = 8;

// The three field readers below decode every *field* this module reads, and
// each one is total: `checked_add` keeps an `at` near `usize::MAX` from
// overflowing before `get` can reject it, and `get` rejects everything past
// the end. `None` therefore means "not in the buffer" for every possible `at`,
// never a panic. The module also slices the buffer directly a few times (the
// entry name, the 30 fixed local-header bytes, the member body); every one of
// those is preceded by an explicit range check that fails with
// `CorruptDirectory` rather than panicking.

fn u16_at(bytes: &[u8], at: usize) -> Option<u16> {
    let end = at.checked_add(2)?;
    bytes.get(at..end).map(|b| u16::from_le_bytes([b[0], b[1]]))
}

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    let end = at.checked_add(4)?;
    bytes
        .get(at..end)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

fn u64_at(bytes: &[u8], at: usize) -> Option<u64> {
    let end = at.checked_add(8)?;
    bytes.get(at..end).map(|b| {
        let mut a = [0_u8; 8];
        a.copy_from_slice(b);
        u64::from_le_bytes(a)
    })
}

/// Converts an on-disk offset/length to `usize`, failing on 32-bit hosts for
/// values that cannot index the buffer.
fn to_index(value: u64, index: usize, reason: &'static str) -> Result<usize, ArchiveError> {
    usize::try_from(value).map_err(|_| ArchiveError::CorruptDirectory { index, reason })
}

/// Where the central directory lives, from the EOCD or the ZIP64 EOCD.
struct DirectoryLocation {
    entries: u64,
    size: u64,
    offset: u64,
}

#[derive(Debug)]
pub(crate) struct ZipContainer {
    bytes: Vec<u8>,
    entries: Vec<RawEntry>,
}

impl ZipContainer {
    /// Parses the central directory; refuses more than `max_members`
    /// declared entries before allocating anything proportional to the count.
    pub(crate) fn open(bytes: Vec<u8>, max_members: usize) -> Result<Self, ArchiveError> {
        let location = locate_directory(&bytes)?;
        if location.entries > max_members as u64 {
            return Err(ArchiveError::TooManyMembers {
                declared: location.entries,
                limit: max_members,
            });
        }
        let cd_start = to_index(location.offset, 0, "central directory offset does not fit")?;
        let cd_size = to_index(location.size, 0, "central directory size does not fit")?;
        let cd_end = cd_start
            .checked_add(cd_size)
            .filter(|&end| end <= bytes.len())
            .ok_or(ArchiveError::CorruptDirectory {
                index: 0,
                reason: "central directory lies outside the file",
            })?;
        // Each entry is at least 46 bytes: a count the directory cannot hold
        // is a lie, and this also bounds the allocation by input length.
        let declared = to_index(location.entries, 0, "entry count does not fit")?;
        if declared.saturating_mul(CENTRAL_LEN) > cd_size {
            return Err(ArchiveError::CorruptDirectory {
                index: 0,
                reason: "entry count exceeds the central directory size",
            });
        }
        let mut entries = Vec::with_capacity(declared);
        let mut ptr = cd_start;
        for index in 0..declared {
            let corrupt = |reason: &'static str| ArchiveError::CorruptDirectory { index, reason };
            // Every addition below is `checked_add` so an overflow surfaces as
            // `CorruptDirectory` rather than wrapping past a bounds check;
            // the fixed-field offsets (`ptr + 8` … `ptr + 42`) sit inside the
            // `fixed_end` span this first check proves is in range.
            let fixed_end = ptr
                .checked_add(CENTRAL_LEN)
                .ok_or_else(|| corrupt("entry offset overflows"))?;
            if fixed_end > cd_end {
                return Err(corrupt("entry overruns the central directory"));
            }
            if u32_at(&bytes, ptr) != Some(CENTRAL_SIG) {
                return Err(corrupt("bad central directory signature"));
            }
            let flags = u16_at(&bytes, ptr + 8).unwrap_or(0);
            let method = u16_at(&bytes, ptr + 10).unwrap_or(0);
            let crc = u32_at(&bytes, ptr + 16).unwrap_or(0);
            let mut compressed_size = u64::from(u32_at(&bytes, ptr + 20).unwrap_or(0));
            let mut size = u64::from(u32_at(&bytes, ptr + 24).unwrap_or(0));
            let name_len = usize::from(u16_at(&bytes, ptr + 28).unwrap_or(0));
            let extra_len = usize::from(u16_at(&bytes, ptr + 30).unwrap_or(0));
            let comment_len = usize::from(u16_at(&bytes, ptr + 32).unwrap_or(0));
            let mut local_header_offset = u64::from(u32_at(&bytes, ptr + 42).unwrap_or(0));
            let name_start = fixed_end;
            let extra_start = name_start
                .checked_add(name_len)
                .ok_or_else(|| corrupt("entry name length overflows"))?;
            let extra_end = extra_start
                .checked_add(extra_len)
                .ok_or_else(|| corrupt("entry extra length overflows"))?;
            let next = extra_end
                .checked_add(comment_len)
                .ok_or_else(|| corrupt("entry comment length overflows"))?;
            if next > cd_end {
                return Err(corrupt(
                    "entry name, extra, or comment overruns the central directory",
                ));
            }
            read_zip64_extra(
                &bytes,
                extra_start,
                extra_end,
                index,
                (&mut size, &mut compressed_size, &mut local_header_offset),
            )?;
            let raw_name = &bytes[name_start..extra_start];
            let (path, utf8) = match std::str::from_utf8(raw_name) {
                Ok(s) => (s.to_string(), true),
                Err(_) => (String::from_utf8_lossy(raw_name).into_owned(), false),
            };
            let path = super::semantics::normalize_path(&path);
            let is_directory = path.ends_with('/') && size == 0;
            if !path.is_empty() && !is_directory {
                entries.push(RawEntry {
                    path,
                    utf8,
                    method,
                    flags,
                    crc32: crc,
                    compressed_size,
                    size,
                    local_header_offset,
                });
            }
            ptr = next;
        }
        Ok(Self { bytes, entries })
    }
}

/// Applies the ZIP64 extra field (id `0x0001`) found in `[extra_start,
/// extra_end)`: `u64` replacements, in order, only for the fixed fields
/// (uncompressed size, compressed size, local-header offset) that read
/// `0xFFFF_FFFF`. Every offset addition is `checked_add`; an overflow or an
/// overrun is `CorruptDirectory`.
fn read_zip64_extra(
    bytes: &[u8],
    extra_start: usize,
    extra_end: usize,
    index: usize,
    (size, compressed_size, local_header_offset): (&mut u64, &mut u64, &mut u64),
) -> Result<(), ArchiveError> {
    let corrupt = |reason: &'static str| ArchiveError::CorruptDirectory { index, reason };
    let mut extra = extra_start;
    while let Some(field_start) = extra.checked_add(4).filter(|&start| start <= extra_end) {
        let id = u16_at(bytes, extra).unwrap_or(0);
        let len = usize::from(u16_at(bytes, extra + 2).unwrap_or(0));
        let field_end = field_start
            .checked_add(len)
            .ok_or_else(|| corrupt("extra field length overflows"))?;
        if field_end > extra_end {
            return Err(corrupt("extra field overruns its declared length"));
        }
        if id == ZIP64_EXTRA_ID {
            let mut at = field_start;
            let mut take = |target: &mut u64| -> Result<(), ArchiveError> {
                if *target == u64::from(u32::MAX) {
                    let end = at
                        .checked_add(8)
                        .ok_or_else(|| corrupt("ZIP64 extra field offset overflows"))?;
                    if end > field_end {
                        return Err(corrupt("ZIP64 extra field is too short"));
                    }
                    *target = u64_at(bytes, at).unwrap_or(0);
                    at = end;
                }
                Ok(())
            };
            take(size)?;
            take(compressed_size)?;
            take(local_header_offset)?;
        }
        extra = field_end;
    }
    Ok(())
}

/// Finds the EOCD (scanning back over at most a maximal comment) and, when a
/// ZIP64 locator precedes it, the ZIP64 EOCD record.
fn locate_directory(bytes: &[u8]) -> Result<DirectoryLocation, ArchiveError> {
    if bytes.len() < EOCD_LEN {
        return Err(ArchiveError::NotAnArchive);
    }
    let last = bytes.len() - EOCD_LEN;
    let first = last.saturating_sub(MAX_COMMENT);
    let eocd = (first..=last).rev().find(|&at| {
        // `at ≤ len - 22` so this cannot overflow; `checked_add` keeps the
        // predicate in the same form as every other offset addition here.
        u32_at(bytes, at) == Some(EOCD_SIG)
            && at
                .checked_add(EOCD_LEN)
                .and_then(|end| end.checked_add(usize::from(u16_at(bytes, at + 20).unwrap_or(0))))
                == Some(bytes.len())
    });
    let Some(eocd) = eocd else {
        return Err(ArchiveError::NotAnArchive);
    };
    let entries = u64::from(u16_at(bytes, eocd + 10).unwrap_or(0));
    let size = u64::from(u32_at(bytes, eocd + 12).unwrap_or(0));
    let offset = u64::from(u32_at(bytes, eocd + 16).unwrap_or(0));

    // ZIP64: a locator sits immediately before the EOCD.
    if eocd >= ZIP64_LOCATOR_LEN
        && u32_at(bytes, eocd - ZIP64_LOCATOR_LEN) == Some(ZIP64_LOCATOR_SIG)
    {
        let locator = eocd - ZIP64_LOCATOR_LEN;
        let record = to_index(
            u64_at(bytes, locator + 8).unwrap_or(0),
            0,
            "ZIP64 record offset does not fit",
        )?;
        let fits = record
            .checked_add(ZIP64_EOCD_LEN)
            .is_some_and(|end| end <= bytes.len());
        if !fits || u32_at(bytes, record) != Some(ZIP64_EOCD_SIG) {
            return Err(ArchiveError::CorruptDirectory {
                index: 0,
                reason: "ZIP64 end-of-central-directory record missing or misplaced",
            });
        }
        return Ok(DirectoryLocation {
            entries: u64_at(bytes, record + 32).unwrap_or(0),
            size: u64_at(bytes, record + 40).unwrap_or(0),
            offset: u64_at(bytes, record + 48).unwrap_or(0),
        });
    }
    Ok(DirectoryLocation {
        entries,
        size,
        offset,
    })
}

impl Container for ZipContainer {
    fn entries(&self) -> &[RawEntry] {
        &self.entries
    }

    fn read_entry(&self, index: usize, cap: usize) -> Result<Vec<u8>, ArchiveError> {
        let corrupt = |reason: &'static str| ArchiveError::CorruptDirectory { index, reason };
        // Defense in depth: `Archive::read` already refuses a `Member` from
        // another archive by its archive id, so this cannot fire through the
        // public API — but the seam must answer an out-of-range index with an
        // error rather than a panicking index, and it must not claim the file
        // is corrupt. The container has no path for an entry it does not own,
        // so the index identifies it instead.
        let entry = self
            .entries
            .get(index)
            .ok_or_else(|| ArchiveError::ForeignMember {
                path: format!("entry {index}"),
            })?;
        let path = entry.path.clone();
        // The local header offset and the compressed size are file-derived, so
        // every offset built from them is `checked_add`-bounded against the
        // buffer before a slice is taken (ADR-0016 §1).
        let header = to_index(
            entry.local_header_offset,
            index,
            "local header offset does not fit",
        )?;
        let fixed_end = header
            .checked_add(LOCAL_LEN)
            .filter(|&end| end <= self.bytes.len())
            .ok_or_else(|| corrupt("local header missing or misplaced"))?;
        let fixed = &self.bytes[header..fixed_end];
        if u32_at(fixed, 0) != Some(LOCAL_SIG) {
            return Err(corrupt("local header missing or misplaced"));
        }
        // Only the signature and the two lengths are taken from the local
        // header: its CRC and sizes are zero for data-descriptor entries
        // (general-purpose flag bit 3), so the central directory stays
        // authoritative for those. Both offsets are constants inside the
        // 30 bytes just bounds-checked.
        let name_len = usize::from(u16_at(fixed, 26).unwrap_or(0));
        let extra_len = usize::from(u16_at(fixed, 28).unwrap_or(0));
        let compressed_len =
            to_index(entry.compressed_size, index, "compressed size does not fit")?;
        let data_start = fixed_end
            .checked_add(name_len)
            .and_then(|at| at.checked_add(extra_len))
            .ok_or_else(|| corrupt("member data lies outside the file"))?;
        let data_end = data_start
            .checked_add(compressed_len)
            .filter(|&end| end <= self.bytes.len())
            .ok_or_else(|| corrupt("member data lies outside the file"))?;
        let body = &self.bytes[data_start..data_end];
        let declared = entry.size;
        let declared_len = to_index(declared, index, "uncompressed size does not fit")?;
        if declared_len > cap {
            return Err(ArchiveError::MemberTooLarge {
                path,
                declared,
                limit: cap,
            });
        }

        let decoded = match entry.method {
            METHOD_STORED => {
                if body.len() != declared_len {
                    return Err(ArchiveError::SizeMismatch {
                        path,
                        declared,
                        actual: Some(body.len() as u64),
                    });
                }
                body.to_vec()
            }
            METHOD_DEFLATE => {
                // Cap at the declared size: anything beyond it is a lie, and
                // the declared size is already within `cap`.
                match miniz_oxide::inflate::decompress_to_vec_with_limit(body, declared_len) {
                    Ok(out) => {
                        if out.len() != declared_len {
                            return Err(ArchiveError::SizeMismatch {
                                path,
                                declared,
                                actual: Some(out.len() as u64),
                            });
                        }
                        out
                    }
                    Err(err) if err.status == miniz_oxide::inflate::TINFLStatus::HasMoreOutput => {
                        return Err(ArchiveError::SizeMismatch {
                            path,
                            declared,
                            actual: None,
                        });
                    }
                    Err(_) => return Err(ArchiveError::CorruptStream { path }),
                }
            }
            code => {
                return Err(ArchiveError::UnsupportedMethod {
                    path,
                    method: super::Method::from_code(code),
                });
            }
        };
        if crc32(&decoded) != entry.crc32 {
            return Err(ArchiveError::ChecksumMismatch { path });
        }
        Ok(decoded)
    }
}

#[cfg(test)]
mod tests {
    use super::{Container, ZipContainer, u16_at, u32_at, u64_at};

    /// A one-member zip: local header + body, one central-directory entry
    /// (with `extra` appended to it), EOCD. `declared` and `local_offset`
    /// are written as given so a test can make the directory lie; the CRC
    /// is written as `0` because every case here fails before the checksum.
    fn one_member_zip(
        method: u16,
        body: &[u8],
        declared: u32,
        extra: &[u8],
        local_offset: u32,
    ) -> Vec<u8> {
        let name = b"member.bin";
        let csize = u32::try_from(body.len()).unwrap();
        let mut out = Vec::new();
        out.extend_from_slice(&0x0403_4b50_u32.to_le_bytes());
        out.extend_from_slice(&[20, 0, 0, 0]);
        out.extend_from_slice(&method.to_le_bytes());
        out.extend_from_slice(&[0; 4]);
        out.extend_from_slice(&0_u32.to_le_bytes());
        out.extend_from_slice(&csize.to_le_bytes());
        out.extend_from_slice(&declared.to_le_bytes());
        out.extend_from_slice(&u16::try_from(name.len()).unwrap().to_le_bytes());
        out.extend_from_slice(&0_u16.to_le_bytes());
        out.extend_from_slice(name);
        out.extend_from_slice(body);
        let cd_offset = u32::try_from(out.len()).unwrap();
        let mut cd = Vec::new();
        cd.extend_from_slice(&0x0201_4b50_u32.to_le_bytes());
        cd.extend_from_slice(&[20, 0, 20, 0, 0, 0]);
        cd.extend_from_slice(&method.to_le_bytes());
        cd.extend_from_slice(&[0; 4]);
        cd.extend_from_slice(&0_u32.to_le_bytes());
        cd.extend_from_slice(&csize.to_le_bytes());
        cd.extend_from_slice(&declared.to_le_bytes());
        cd.extend_from_slice(&u16::try_from(name.len()).unwrap().to_le_bytes());
        cd.extend_from_slice(&u16::try_from(extra.len()).unwrap().to_le_bytes());
        cd.extend_from_slice(&[0; 6]); // comment length, disk start, internal attributes
        cd.extend_from_slice(&0_u32.to_le_bytes()); // external attributes
        cd.extend_from_slice(&local_offset.to_le_bytes());
        cd.extend_from_slice(name);
        cd.extend_from_slice(extra);
        let cd_size = u32::try_from(cd.len()).unwrap();
        out.extend_from_slice(&cd);
        out.extend_from_slice(&0x0605_4b50_u32.to_le_bytes());
        out.extend_from_slice(&[0; 4]);
        out.extend_from_slice(&1_u16.to_le_bytes());
        out.extend_from_slice(&1_u16.to_le_bytes());
        out.extend_from_slice(&cd_size.to_le_bytes());
        out.extend_from_slice(&cd_offset.to_le_bytes());
        out.extend_from_slice(&0_u16.to_le_bytes());
        out
    }

    #[test]
    fn the_seam_refuses_an_entry_index_it_does_not_own() {
        let zip = ZipContainer::open(one_member_zip(0, b"hello", 5, &[], 0), 16).unwrap();
        let err = zip.read_entry(3, 1024).unwrap_err();
        assert_eq!(
            err.to_string(),
            "member `entry 3` does not belong to this archive"
        );
    }

    #[test]
    fn the_seam_refuses_a_declared_size_over_the_cap() {
        // `Archive::read` checks this first through the public API; the
        // seam must hold the line on its own for any future backend.
        let zip = ZipContainer::open(one_member_zip(0, b"hello", 5, &[], 0), 16).unwrap();
        let err = zip.read_entry(0, 4).unwrap_err();
        assert_eq!(
            err.to_string(),
            "member `member.bin` declares 5 decoded bytes, more than the limit of 4"
        );
    }

    #[test]
    fn the_seam_refuses_an_unsupported_method() {
        let zip = ZipContainer::open(one_member_zip(14, b"hello", 5, &[], 0), 16).unwrap();
        let err = zip.read_entry(0, 1024).unwrap_err();
        assert_eq!(
            err.to_string(),
            "member `member.bin` uses unsupported compression method lzma"
        );
    }

    #[test]
    fn extra_fields_that_are_not_zip64_are_skipped_and_partial_zip64_keeps_real_fields() {
        // A UT timestamp extra field (id 0x5455) is walked past; a ZIP64 extra
        // that replaces only the local-header offset leaves the sizes alone.
        let ut = [0x55, 0x54, 5, 0, 3, 1, 2, 3, 4];
        let zip = ZipContainer::open(one_member_zip(0, b"hello", 5, &ut, 0), 16).unwrap();
        assert_eq!(zip.entries()[0].size, 5);
        // The body is fine but the directory's CRC is 0, so the read fails
        // at the checksum — after the extra field was walked without error.
        assert_eq!(
            zip.read_entry(0, 1024).unwrap_err().to_string(),
            "member `member.bin` failed its CRC-32 check"
        );

        let mut zip64 = vec![0x01, 0x00, 8, 0];
        zip64.extend_from_slice(&0_u64.to_le_bytes());
        let zip = ZipContainer::open(one_member_zip(0, b"hello", 5, &zip64, u32::MAX), 16).unwrap();
        let entry = &zip.entries()[0];
        assert_eq!(
            (entry.size, entry.compressed_size, entry.local_header_offset),
            (5, 5, 0)
        );
    }

    const SAMPLE: [u8; 8] = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];

    #[test]
    fn field_readers_decode_little_endian_at_the_start() {
        assert_eq!(u16_at(&SAMPLE, 0), Some(0x0201));
        assert_eq!(u32_at(&SAMPLE, 0), Some(0x0403_0201));
        assert_eq!(u64_at(&SAMPLE, 0), Some(0x0807_0605_0403_0201));
    }

    #[test]
    fn field_readers_refuse_a_field_that_runs_past_the_end() {
        let last = SAMPLE.len() - 1;
        assert_eq!(u16_at(&SAMPLE, last), None);
        assert_eq!(u32_at(&SAMPLE, last), None);
        assert_eq!(u64_at(&SAMPLE, last), None);
    }

    #[test]
    fn field_readers_refuse_an_offset_that_would_overflow() {
        // `at + N` overflows here; the readers must answer `None`, not abort.
        assert_eq!(u16_at(&SAMPLE, usize::MAX), None);
        assert_eq!(u32_at(&SAMPLE, usize::MAX), None);
        assert_eq!(u64_at(&SAMPLE, usize::MAX), None);
    }
}
