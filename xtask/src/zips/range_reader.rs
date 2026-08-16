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
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::api::client::{backoff_delay, is_retryable};
use crate::mirror::MIRRORS;
use crate::zips::inspect::{FetchFailure, RangeSource};

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

/// §5.4: 4–8 concurrent connections against a mirror.
#[allow(dead_code)]
pub const MIRROR_CONCURRENCY: usize = 6;

/// Transient-failure retries per mirror per fetch. Lighter than the API
/// client's 6: mirror failover is the real second chance here.
#[allow(dead_code)]
pub const MAX_MIRROR_ATTEMPTS: u32 = 3;

/// §5.2: global byte budget for the full-download fallback.
#[allow(dead_code)]
pub const FALLBACK_BYTE_BUDGET: u64 = 2 * 1024 * 1024 * 1024;

/// Run-wide transfer accounting (§9.3: total bytes transferred must stay a
/// small fraction of the archive). Every response-body byte this module
/// reads off the wire — from a ranged fetch or a full download, whether the
/// attempt ultimately succeeds or is discarded as over/short — is counted
/// here as it's read (never derived from a `Content-Length` header), along
/// with every request issued: Task 6's run-wide runaway breaker depends on
/// these being exact, not an under-count of failed attempts.
#[allow(dead_code)]
#[derive(Debug, Default)]
pub struct TransferCounters {
    /// GET requests issued (ranged and full).
    pub requests: AtomicU64,
    /// Response-body bytes read.
    pub bytes: AtomicU64,
}

impl TransferCounters {
    /// Zeroed counters for a fresh run.
    #[allow(dead_code)]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

/// Breaker outcome for one entry that needs the full-download fallback.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackDecision {
    /// Within budget and breaker: download, then parse from memory.
    Download,
    /// Byte budget exhausted: record `no_range_support`, continue the run.
    Skip,
    /// More than ~2% of entries needed the fallback: stop the phase (§5.2
    /// — a CDN change has turned metadata reads into mirroring; never
    /// continue).
    Abort,
}

/// §5.2 budgeted-fallback accounting. Lives behind a mutex in the
/// orchestrator; the methods are sync and trivial.
#[allow(dead_code)]
#[derive(Debug)]
pub struct FallbackBudget {
    bytes_remaining: u64,
    needed: u64,
    limit: u64,
}

impl FallbackBudget {
    /// Budget for a run inspecting `total_entries` entries in total: a
    /// [`FALLBACK_BYTE_BUDGET`]-byte allowance and a [`fallback_limit`]
    /// breaker.
    #[allow(dead_code)]
    #[must_use]
    pub fn new(total_entries: u64) -> Self {
        Self {
            bytes_remaining: FALLBACK_BYTE_BUDGET,
            needed: 0,
            limit: fallback_limit(total_entries),
        }
    }

    /// Admit one entry's fallback need of `size` bytes. `needed` counts
    /// first — a breaker trip counts even when this particular need goes
    /// unmet — then the byte budget gates whether it's actually granted.
    #[allow(dead_code)]
    pub fn admit(&mut self, size: u64) -> FallbackDecision {
        self.needed += 1;
        if self.needed > self.limit {
            return FallbackDecision::Abort;
        }
        if size > self.bytes_remaining {
            return FallbackDecision::Skip;
        }
        self.bytes_remaining -= size;
        FallbackDecision::Download
    }
}

/// `max(2, ~2% of entries)` — the floor keeps a 5-entry dev run from
/// aborting on its first legitimately range-less file while still tripping
/// fast when the pattern is systemic.
#[allow(dead_code)]
fn fallback_limit(total_entries: u64) -> u64 {
    (total_entries * 2 / 100).max(2)
}

/// Build a mirror URL for one archive entry. `base` and `dir` both carry a
/// trailing slash (§5.1's [`MIRRORS`] and the Phase-1 invariant on `dir`,
/// respectively), so the two `join`s compose into `<base><dir><filename>`
/// with `filename` percent-encoded per segment — `Url::join` treats a base
/// ending in `/` as a directory rather than a file, so neither join drops
/// the path already accumulated.
#[allow(dead_code)]
pub(crate) fn entry_url(base: &str, dir: &str, filename: &str) -> anyhow::Result<reqwest::Url> {
    Ok(reqwest::Url::parse(base)?.join(dir)?.join(filename)?)
}

/// Stream `resp`'s body, counting every byte actually read into
/// `counters.bytes` as it arrives (§9.3 — a failed or over-delivering
/// attempt's bytes still count; `Content-Length` is only a hint, never the
/// counted quantity). Capped at exactly `cap` bytes: a declared
/// `Content-Length` over `cap` skips the read entirely (mirror.rs
/// `persist_and_parse` precedent — never even start a doomed transfer); a
/// body that grows past `cap` mid-stream fails with `over_detail`, and a
/// body that ends up short of `cap` fails with a generic detail — neither
/// over- nor under-delivery is a partial success.
#[allow(dead_code)]
async fn stream_capped(
    mut resp: reqwest::Response,
    cap: u64,
    counters: &TransferCounters,
    over_detail: &str,
) -> Result<Vec<u8>, String> {
    if resp.content_length().is_some_and(|len| len > cap) {
        return Err(over_detail.to_owned());
    }
    let cap_usize = usize::try_from(cap).unwrap_or(usize::MAX);
    let mut bytes: Vec<u8> = Vec::new();
    loop {
        match resp.chunk().await {
            Ok(Some(chunk)) => {
                counters.bytes.fetch_add(
                    u64::try_from(chunk.len()).unwrap_or(u64::MAX),
                    Ordering::Relaxed,
                );
                if bytes.len() + chunk.len() > cap_usize {
                    return Err(over_detail.to_owned());
                }
                bytes.extend_from_slice(&chunk);
            }
            Ok(None) => break,
            Err(e) => return Err(format!("body transport: {e}")),
        }
    }
    if bytes.len() != cap_usize {
        return Err(format!(
            "short body: got {} bytes, wanted {cap}",
            bytes.len()
        ));
    }
    Ok(bytes)
}

/// One archive entry's byte source: the §5.1 mirror pool with per-mirror
/// retries, 404/no-range failover, and capped body reads. Pins to the
/// first mirror that serves bytes so multi-round fetches never mix
/// mirrors (their copies could momentarily differ).
#[allow(dead_code)]
#[derive(Debug)]
pub struct MirrorRanges {
    /// Shared HTTP client (connection pooling, UA, timeouts — the
    /// caller's concern, not this type's).
    http: reqwest::Client,
    /// One URL per [`MIRRORS`] entry, same order (infania, gamers).
    urls: [reqwest::Url; 2],
    /// Declared size of the whole entry (Phase-1/ls-laR record): lets
    /// [`RangeSource::fetch`] recognize a range request that covers the
    /// entire file (a `200` answer to that is a legal "server ignored the
    /// range", not a policy refusal), and caps [`Self::download_full`].
    expected_file_size: u64,
    /// Index into `urls`/[`MIRRORS`] of the pinned mirror, once one has
    /// served bytes.
    pinned: Option<usize>,
    /// Mirrors that answered `200` to a ranged (non-whole-file) request
    /// (range support absent).
    range_refused: [bool; 2],
    /// Mirrors that answered `404`.
    not_found: [bool; 2],
    /// Run-wide transfer accounting, shared across every entry's source.
    counters: Arc<TransferCounters>,
    /// Backoff jitter source (one per instance, like `ApiClient`'s).
    rng: fastrand::Rng,
}

impl MirrorRanges {
    /// Build a byte source for one archive entry at `dir`/`filename`
    /// (Phase-1 record: `dir` trailing-slashed, `expected_size` the
    /// declared zip size) across the §5.1 mirror pool.
    ///
    /// # Errors
    /// URL construction only (a malformed `dir`/`filename`); every network
    /// failure surfaces later, from [`RangeSource::fetch`] or
    /// [`Self::download_full`].
    #[allow(dead_code)]
    pub fn new(
        http: reqwest::Client,
        dir: &str,
        filename: &str,
        expected_size: u64,
        counters: Arc<TransferCounters>,
    ) -> anyhow::Result<Self> {
        let urls = [
            entry_url(MIRRORS[0].base, dir, filename)?,
            entry_url(MIRRORS[1].base, dir, filename)?,
        ];
        Ok(Self {
            http,
            urls,
            expected_file_size: expected_size,
            pinned: None,
            range_refused: [false, false],
            not_found: [false, false],
            counters,
            rng: fastrand::Rng::new(),
        })
    }

    /// The pinned mirror's key, once a fetch has pinned one; `""` before
    /// that (no bytes served yet).
    #[allow(dead_code)]
    #[must_use]
    pub fn mirror_key(&self) -> &'static str {
        self.pinned.map_or("", |i| MIRRORS[i].key)
    }

    /// Candidate mirror indices for [`RangeSource::fetch`]: the pinned
    /// mirror alone once one is set (multi-round fetches must never mix
    /// mirrors), else every mirror not yet disqualified by a `404` or a
    /// range refusal, in [`MIRRORS`] order.
    #[allow(dead_code)]
    fn candidates(&self) -> Vec<usize> {
        if let Some(i) = self.pinned {
            return vec![i];
        }
        (0..self.urls.len())
            .filter(|&i| !self.range_refused[i] && !self.not_found[i])
            .collect()
    }

    /// Candidate mirror indices for [`Self::download_full`]: unlike a
    /// ranged fetch, a plain GET doesn't care whether a mirror has refused
    /// ranges, so only a `404` disqualifies one; the pinned mirror (if any)
    /// is tried first.
    #[allow(dead_code)]
    fn full_candidates(&self) -> Vec<usize> {
        let mut order = Vec::with_capacity(self.urls.len());
        if let Some(p) = self.pinned
            && !self.not_found[p]
        {
            order.push(p);
        }
        for i in 0..self.urls.len() {
            if !self.not_found[i] && Some(i) != self.pinned {
                order.push(i);
            }
        }
        order
    }

    /// Classify a [`RangeSource::fetch`] that exhausted every candidate
    /// mirror without success (§5.2's failover order: all-404 beats a
    /// range refusal, which beats a generic transport/HTTP detail).
    #[allow(dead_code)]
    fn fetch_exhausted(&self, last_detail: Option<String>) -> FetchFailure {
        if self.not_found.iter().all(|&nf| nf) {
            FetchFailure::NotFound
        } else if self.range_refused.iter().any(|&rr| rr) {
            FetchFailure::RangeUnsupported
        } else {
            FetchFailure::Http(last_detail.unwrap_or_else(|| "no mirrors available".into()))
        }
    }

    /// Full-file download for the §5.2 no-range-support fallback: every
    /// usable mirror answered `200` to a ranged request, so read the whole
    /// entry from the first still-viable mirror (preferring the pinned
    /// one) for in-memory parsing instead of ranged reads.
    ///
    /// # Errors
    /// [`FetchFailure`] after retries/failover are exhausted, or a body
    /// that over- or under-delivers relative to `expected_size` — the
    /// ls-laR-declared size is authoritative, so a mismatch is never
    /// silently accepted.
    #[allow(dead_code)]
    pub async fn download_full(&mut self, expected_size: u64) -> Result<Vec<u8>, FetchFailure> {
        let mut last_detail: Option<String> = None;
        for i in self.full_candidates() {
            for attempt in 1..=MAX_MIRROR_ATTEMPTS {
                self.counters.requests.fetch_add(1, Ordering::Relaxed);
                let outcome = self.http.get(self.urls[i].clone()).send().await;
                let resp = match outcome {
                    Ok(r) => r,
                    Err(e) => {
                        last_detail = Some(format!("transport: {e}"));
                        if attempt < MAX_MIRROR_ATTEMPTS {
                            tokio::time::sleep(backoff_delay(attempt, &mut self.rng)).await;
                            continue;
                        }
                        break;
                    }
                };
                let status = resp.status();
                if status == reqwest::StatusCode::OK {
                    match stream_capped(
                        resp,
                        expected_size,
                        &self.counters,
                        "body exceeded declared size",
                    )
                    .await
                    {
                        Ok(bytes) => {
                            self.pinned = Some(i);
                            return Ok(bytes);
                        }
                        Err(detail) => {
                            last_detail = Some(detail);
                            if attempt < MAX_MIRROR_ATTEMPTS {
                                tokio::time::sleep(backoff_delay(attempt, &mut self.rng)).await;
                                continue;
                            }
                            break;
                        }
                    }
                } else if status == reqwest::StatusCode::NOT_FOUND {
                    self.not_found[i] = true;
                    break;
                } else if is_retryable(Some(status)) {
                    last_detail = Some(format!("HTTP {status}"));
                    if attempt < MAX_MIRROR_ATTEMPTS {
                        tokio::time::sleep(backoff_delay(attempt, &mut self.rng)).await;
                        continue;
                    }
                    break;
                }
                last_detail = Some(format!("HTTP {status}"));
                break;
            }
        }
        if self.not_found.iter().all(|&nf| nf) {
            Err(FetchFailure::NotFound)
        } else {
            Err(FetchFailure::Http(
                last_detail.unwrap_or_else(|| "no mirrors available".into()),
            ))
        }
    }
}

impl RangeSource for MirrorRanges {
    /// §5.2: up to [`MAX_MIRROR_ATTEMPTS`] retries per candidate mirror
    /// (transient statuses/transport errors, backed off per
    /// [`backoff_delay`]), then failover to the next candidate; a `200` to
    /// a non-whole-file range or a `404` disqualifies a mirror outright
    /// (no retry — policy, not a hiccup). See [`Self::candidates`] and
    /// [`Self::fetch_exhausted`] for selection and final classification.
    async fn fetch(&mut self, offset: u64, len: u64) -> Result<Vec<u8>, FetchFailure> {
        let whole_file = offset == 0 && len == self.expected_file_size;
        let range_header = format!("bytes={offset}-{}", offset + len.saturating_sub(1));
        let mut last_detail: Option<String> = None;

        for i in self.candidates() {
            for attempt in 1..=MAX_MIRROR_ATTEMPTS {
                self.counters.requests.fetch_add(1, Ordering::Relaxed);
                let outcome = self
                    .http
                    .get(self.urls[i].clone())
                    .header(reqwest::header::RANGE, range_header.clone())
                    .send()
                    .await;
                let resp = match outcome {
                    Ok(r) => r,
                    Err(e) => {
                        last_detail = Some(format!("transport: {e}"));
                        if attempt < MAX_MIRROR_ATTEMPTS {
                            tokio::time::sleep(backoff_delay(attempt, &mut self.rng)).await;
                            continue;
                        }
                        break;
                    }
                };

                let status = resp.status();
                if status == reqwest::StatusCode::PARTIAL_CONTENT
                    || (status == reqwest::StatusCode::OK && whole_file)
                {
                    match stream_capped(resp, len, &self.counters, "range over-delivery").await {
                        Ok(bytes) => {
                            self.pinned = Some(i);
                            return Ok(bytes);
                        }
                        Err(detail) => {
                            last_detail = Some(detail);
                            if attempt < MAX_MIRROR_ATTEMPTS {
                                tokio::time::sleep(backoff_delay(attempt, &mut self.rng)).await;
                                continue;
                            }
                            break;
                        }
                    }
                } else if status == reqwest::StatusCode::OK {
                    // The server ignored the range but this wasn't a
                    // whole-file request: a policy refusal, not a hiccup —
                    // no retry, just disqualify this mirror.
                    self.range_refused[i] = true;
                    break;
                } else if status == reqwest::StatusCode::NOT_FOUND {
                    self.not_found[i] = true;
                    break;
                } else if is_retryable(Some(status)) {
                    last_detail = Some(format!("HTTP {status}"));
                    if attempt < MAX_MIRROR_ATTEMPTS {
                        tokio::time::sleep(backoff_delay(attempt, &mut self.rng)).await;
                        continue;
                    }
                    break;
                }
                last_detail = Some(format!("HTTP {status}"));
                break;
            }
        }

        Err(self.fetch_exhausted(last_detail))
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

    #[test]
    fn entry_url_joins_and_percent_encodes() {
        let url = entry_url(
            "https://ftpmirror1.infania.net/pub/idgames/",
            "levels/doom2/Ports/megawads/",
            "with space.zip",
        )
        .unwrap();
        assert_eq!(
            url.as_str(),
            "https://ftpmirror1.infania.net/pub/idgames/levels/doom2/Ports/megawads/with%20space.zip"
        );
    }

    #[test]
    fn fallback_budget_grants_skips_and_trips() {
        // 2 GiB budget, breaker at >2% of entries with a small-run floor of 2
        // (§5.2: "more than ~2% of entries hit the fallback → stop the phase").
        let mut b = FallbackBudget::new(1000); // 2% → limit 20
        assert!(matches!(b.admit(1024), FallbackDecision::Download));
        // A grant consumes budget bytes.
        let mut small = FallbackBudget::new(1000);
        assert!(matches!(
            small.admit(FALLBACK_BYTE_BUDGET),
            FallbackDecision::Download
        ));
        // Budget exhausted → Skip (record no_range_support), run continues.
        assert!(matches!(small.admit(1), FallbackDecision::Skip));
        // Breaker: fallback NEEDS beyond the limit abort the phase, granted or not.
        let mut tiny = FallbackBudget::new(10); // 2% floor → 2
        assert!(matches!(tiny.admit(1), FallbackDecision::Download));
        assert!(matches!(tiny.admit(1), FallbackDecision::Download));
        assert!(matches!(tiny.admit(1), FallbackDecision::Abort));
    }

    #[test]
    fn fallback_limit_has_a_small_run_floor() {
        assert_eq!(fallback_limit(0), 2);
        assert_eq!(fallback_limit(10), 2);
        assert_eq!(fallback_limit(1000), 20);
        assert_eq!(fallback_limit(21_375), 427);
    }

    #[test]
    fn mirror_concurrency_stays_in_the_design_5_4_band() {
        assert!((4..=8).contains(&MIRROR_CONCURRENCY));
    }
}
