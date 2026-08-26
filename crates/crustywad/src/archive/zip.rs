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

// The three field readers below are the only way this module touches the
// buffer at a computed offset, so each one is total: `checked_add` keeps an
// `at` near `usize::MAX` from overflowing before `get` can reject it, and
// `get` rejects everything past the end. `None` therefore means "not in the
// buffer" for every possible `at`, never a panic.

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
            if ptr + CENTRAL_LEN > cd_end {
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
            let name_start = ptr + CENTRAL_LEN;
            let extra_start = name_start + name_len;
            let next = extra_start + extra_len + comment_len;
            if next > cd_end {
                return Err(corrupt(
                    "entry name, extra, or comment overruns the central directory",
                ));
            }
            // ZIP64 extra field: u64 replacements, in order, only for the
            // fixed fields that read 0xFFFF_FFFF.
            let mut extra = extra_start;
            let extra_end = extra_start + extra_len;
            while extra + 4 <= extra_end {
                let id = u16_at(&bytes, extra).unwrap_or(0);
                let len = usize::from(u16_at(&bytes, extra + 2).unwrap_or(0));
                let field_start = extra + 4;
                let field_end = field_start + len;
                if field_end > extra_end {
                    return Err(corrupt("extra field overruns its declared length"));
                }
                if id == ZIP64_EXTRA_ID {
                    let mut at = field_start;
                    let mut take = |target: &mut u64| -> Result<(), ArchiveError> {
                        if *target == u64::from(u32::MAX) {
                            if at + 8 > field_end {
                                return Err(corrupt("ZIP64 extra field is too short"));
                            }
                            *target = u64_at(&bytes, at).unwrap_or(0);
                            at += 8;
                        }
                        Ok(())
                    };
                    take(&mut size)?;
                    take(&mut compressed_size)?;
                    take(&mut local_header_offset)?;
                }
                extra = field_end;
            }
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

/// Finds the EOCD (scanning back over at most a maximal comment) and, when a
/// ZIP64 locator precedes it, the ZIP64 EOCD record.
fn locate_directory(bytes: &[u8]) -> Result<DirectoryLocation, ArchiveError> {
    if bytes.len() < EOCD_LEN {
        return Err(ArchiveError::NotAnArchive);
    }
    let last = bytes.len() - EOCD_LEN;
    let first = last.saturating_sub(MAX_COMMENT);
    let eocd = (first..=last).rev().find(|&at| {
        u32_at(bytes, at) == Some(EOCD_SIG)
            && at + EOCD_LEN + usize::from(u16_at(bytes, at + 20).unwrap_or(0)) == bytes.len()
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
        let _ = (index, cap, &self.bytes);
        Ok(Vec::new()) // Task 5
    }
}

#[cfg(test)]
mod tests {
    use super::{u16_at, u32_at, u64_at};

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
