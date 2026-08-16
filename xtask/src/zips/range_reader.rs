//! §5.2 Read + Seek over HTTP ranges — sparse buffer, miss reporting, mirror source.
//!
//! `RangeReader` is deliberately pure and synchronous: it reads only bytes
//! already fetched into a [`SparseBuffer`]. A read outside the cached
//! ranges records the missing extent and fails; the async driver in
//! `inspect.rs` fetches exactly that extent and re-parses. This keeps every
//! HTTP call on the async path (no `block_on` bridging) and makes the §9.1
//! request-count regression a plain counter assertion.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::io::{self, Read, Seek, SeekFrom};

/// Tail fetched by round 1: covers the worst-case EOCD backward-scan window
/// (22 + 65,535 = 65,557 bytes, §5.3) plus the ZIP64 EOCD locator (20) and
/// ZIP64 EOCD record (56) that precede it. A bare 64 KiB tail can miss the
/// EOCD signature by up to 21 bytes (§5.2).
#[allow(dead_code)]
pub const TAIL_LEN: u64 = 66 * 1024;

/// Byte ranges of a remote file fetched so far, keyed by start offset.
/// Segments never overlap and are never adjacent — `insert` coalesces.
#[allow(dead_code)]
#[derive(Debug)]
pub struct SparseBuffer {
    file_size: u64,
    segments: BTreeMap<u64, Vec<u8>>,
}

impl SparseBuffer {
    /// Empty buffer for a file of `file_size` bytes.
    #[allow(dead_code)]
    pub fn new(file_size: u64) -> Self {
        Self {
            file_size,
            segments: BTreeMap::new(),
        }
    }

    /// Total size of the remote file.
    #[allow(dead_code)]
    pub fn file_size(&self) -> u64 {
        self.file_size
    }

    /// Insert `bytes` at `offset`, merging any overlapping or adjacent
    /// segments so reads never see artificial seams.
    #[allow(dead_code)]
    pub fn insert(&mut self, offset: u64, bytes: Vec<u8>) {
        if bytes.is_empty() {
            return;
        }
        let len = u64::try_from(bytes.len()).expect("len fits u64");
        let end = offset + len;

        // Any segment that touches [offset, end): seg_start <= end and
        // seg_end >= offset. Bounding the range scan by `..=end` is safe
        // because every touching segment's start is, by definition, <= end.
        let touching: Vec<u64> = self
            .segments
            .range(..=end)
            .filter(|&(&seg_start, seg_bytes)| {
                let seg_len = u64::try_from(seg_bytes.len()).expect("len fits u64");
                seg_start + seg_len >= offset
            })
            .map(|(&seg_start, _)| seg_start)
            .collect();

        if touching.is_empty() {
            self.segments.insert(offset, bytes);
            return;
        }

        let mut merged_start = offset;
        let mut merged_end = end;
        for &seg_start in &touching {
            let seg_bytes = &self.segments[&seg_start];
            let seg_len = u64::try_from(seg_bytes.len()).expect("len fits u64");
            merged_start = merged_start.min(seg_start);
            merged_end = merged_end.max(seg_start + seg_len);
        }

        let merged_len = usize::try_from(merged_end - merged_start).expect("merged len fits usize");
        let mut merged = vec![0_u8; merged_len];

        for &seg_start in &touching {
            let seg_bytes = self
                .segments
                .remove(&seg_start)
                .expect("touching key exists");
            let rel = usize::try_from(seg_start - merged_start).expect("offset fits usize");
            merged[rel..rel + seg_bytes.len()].copy_from_slice(&seg_bytes);
        }

        let rel = usize::try_from(offset - merged_start).expect("offset fits usize");
        merged[rel..rel + bytes.len()].copy_from_slice(&bytes);

        self.segments.insert(merged_start, merged);
    }

    /// Copy cached bytes at `offset` into `buf`. `None` when `offset` is
    /// not covered at all; `Some(n)` with `n < buf.len()` when the segment
    /// ends early (a legal short read).
    #[allow(dead_code)]
    pub fn read_at(&self, offset: u64, buf: &mut [u8]) -> Option<usize> {
        let (&seg_start, seg_bytes) = self.segments.range(..=offset).next_back()?;
        let seg_len = u64::try_from(seg_bytes.len()).expect("len fits u64");
        let seg_end = seg_start + seg_len;
        if offset >= seg_end {
            return None;
        }
        let rel = usize::try_from(offset - seg_start).expect("offset fits usize");
        let available = seg_bytes.len() - rel;
        let n = available.min(buf.len());
        buf[..n].copy_from_slice(&seg_bytes[rel..rel + n]);
        Some(n)
    }

    /// Start of the first cached segment at or after `offset` (`offset`
    /// itself when covered); `file_size` when nothing follows. The driver
    /// uses this to widen a miss into "everything up to the cached tail" —
    /// the central-directory extent — in one request.
    #[allow(dead_code)]
    pub fn next_covered_start(&self, offset: u64) -> u64 {
        if let Some((&seg_start, seg_bytes)) = self.segments.range(..=offset).next_back() {
            let seg_len = u64::try_from(seg_bytes.len()).expect("len fits u64");
            if offset < seg_start + seg_len {
                return offset;
            }
        }
        self.segments
            .range(offset..)
            .next()
            .map_or(self.file_size, |(&seg_start, _)| seg_start)
    }
}

/// `Read + Seek` over a [`SparseBuffer`]. A read at an uncovered offset
/// writes the missing `(offset, len)` into the shared `missing` cell and
/// returns `ErrorKind::Other` — the cell outlives the reader because
/// `zip::ZipArchive::new` consumes its reader on failure.
#[allow(dead_code)]
#[derive(Debug)]
pub struct RangeReader<'a> {
    buf: &'a SparseBuffer,
    missing: &'a Cell<Option<(u64, u64)>>,
    pos: u64,
}

impl<'a> RangeReader<'a> {
    /// Wrap `buf` for sequential `Read + Seek` access; misses are recorded
    /// into `missing`.
    #[allow(dead_code)]
    pub fn new(buf: &'a SparseBuffer, missing: &'a Cell<Option<(u64, u64)>>) -> Self {
        Self {
            buf,
            missing,
            pos: 0,
        }
    }
}

impl Read for RangeReader<'_> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if self.pos >= self.buf.file_size() {
            return Ok(0);
        }
        if let Some(n) = self.buf.read_at(self.pos, out) {
            self.pos += u64::try_from(n).unwrap_or(u64::MAX);
            Ok(n)
        } else {
            let len = u64::try_from(out.len()).unwrap_or(u64::MAX);
            self.missing.set(Some((self.pos, len)));
            Err(io::Error::other("range cache miss"))
        }
    }
}

impl Seek for RangeReader<'_> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let target = match pos {
            SeekFrom::Start(offset) => Some(offset),
            SeekFrom::End(delta) => self.buf.file_size().checked_add_signed(delta),
            SeekFrom::Current(delta) => self.pos.checked_add_signed(delta),
        };
        match target {
            Some(target) => {
                self.pos = target;
                Ok(self.pos)
            }
            None => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "seek to a negative or overflowing position",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::io::{Read, Seek, SeekFrom};

    #[test]
    // `TAIL_LEN` is a `const`, so clippy sees this as a compile-time-constant
    // assertion; the test code here is locked verbatim, so the lint is
    // suppressed rather than restructured.
    #[allow(clippy::assertions_on_constants)]
    fn tail_len_covers_worst_case_eocd_and_zip64_structures() {
        // 22-byte EOCD + 65,535-byte max comment = 65,557 (§5.2/§5.3), plus the
        // 20-byte ZIP64 locator and 56-byte ZIP64 EOCD immediately before it.
        assert!(TAIL_LEN >= 65_557 + 20 + 56);
    }

    #[test]
    fn sparse_buffer_reads_within_and_across_segments() {
        let mut buf = SparseBuffer::new(100);
        buf.insert(10, vec![1; 10]); // [10, 20)
        buf.insert(20, vec![2; 10]); // adjacent → coalesces to [10, 30)
        let mut out = [0_u8; 15];
        assert_eq!(buf.read_at(12, &mut out), Some(15)); // spans the seam
        assert_eq!(&out[..8], &[1; 8]);
        assert_eq!(&out[8..], &[2; 7]);
        // Uncovered offset is a miss, not a zero-read.
        assert_eq!(buf.read_at(40, &mut out), None);
        // Read stops at the segment end (short read), never fabricates bytes.
        assert_eq!(buf.read_at(28, &mut out), Some(2));
    }

    #[test]
    fn sparse_buffer_coalesces_overlap_and_reports_next_covered() {
        let mut buf = SparseBuffer::new(1000);
        buf.insert(500, vec![7; 100]); // [500, 600)
        buf.insert(550, vec![7; 100]); // overlap → [500, 650)
        assert_eq!(buf.next_covered_start(0), 500);
        assert_eq!(buf.next_covered_start(510), 510); // inside a segment: itself
        assert_eq!(buf.next_covered_start(651), 1000); // nothing after → file_size
        let mut out = [0_u8; 150];
        assert_eq!(buf.read_at(500, &mut out), Some(150));
    }

    #[test]
    fn range_reader_reads_seeks_and_reports_misses() {
        let mut buf = SparseBuffer::new(100);
        buf.insert(90, (90..100).map(|i| u8::try_from(i).unwrap()).collect());
        let missing = Cell::new(None);
        let mut r = RangeReader::new(&buf, &missing);

        // SeekFrom::End lands on file_size-relative positions.
        assert_eq!(r.seek(SeekFrom::End(-10)).unwrap(), 90);
        let mut out = [0_u8; 4];
        r.read_exact(&mut out).unwrap();
        assert_eq!(out, [90, 91, 92, 93]);

        // A read at an uncovered offset errors AND records the miss range.
        r.seek(SeekFrom::Start(10)).unwrap();
        let err = r.read(&mut out).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::Other);
        assert_eq!(missing.get(), Some((10, 4)));

        // Reading at EOF is a clean zero, not a miss.
        missing.set(None);
        r.seek(SeekFrom::Start(100)).unwrap();
        assert_eq!(r.read(&mut out).unwrap(), 0);
        assert_eq!(missing.get(), None);

        // Negative absolute positions are invalid input.
        assert!(r.seek(SeekFrom::End(-200)).is_err());
    }
}
