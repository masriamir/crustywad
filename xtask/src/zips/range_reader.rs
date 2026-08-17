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
pub const TAIL_LEN: u64 = 66 * 1024;

/// Byte ranges of a remote file fetched so far, keyed by start offset.
/// Segments never overlap and are never adjacent — `insert` coalesces.
#[derive(Debug)]
pub struct SparseBuffer {
    file_size: u64,
    segments: BTreeMap<u64, Vec<u8>>,
}

impl SparseBuffer {
    /// Empty buffer for a file of `file_size` bytes.
    pub fn new(file_size: u64) -> Self {
        Self {
            file_size,
            segments: BTreeMap::new(),
        }
    }

    /// Total size of the remote file.
    pub fn file_size(&self) -> u64 {
        self.file_size
    }

    /// Insert `bytes` at `offset`, merging any overlapping or adjacent
    /// segments so reads never see artificial seams. An insert that would
    /// overflow `u64` or spill past `file_size` is discarded (see below)
    /// rather than corrupting the segment map.
    pub fn insert(&mut self, offset: u64, bytes: Vec<u8>) {
        if bytes.is_empty() {
            return;
        }
        let len = u64::try_from(bytes.len()).expect("len fits u64");
        // Defensive bound (ADR-0016 posture): the drivers only insert
        // ranges they fetched, clamped to [0, file_size) — but that
        // invariant lives in other files. Discarding an out-of-bounds
        // insert here keeps every stored segment inside [0, file_size],
        // which in turn makes the `seg_start + seg_len` arithmetic below
        // provably overflow-free. A discarded insert is fail-closed: the
        // driver's next read simply misses again and the bounded round
        // budget converts that into a recorded TooChatty, never silent
        // corruption.
        let Some(end) = offset.checked_add(len) else {
            return;
        };
        if end > self.file_size {
            return;
        }

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
#[derive(Debug)]
pub struct RangeReader<'a> {
    buf: &'a SparseBuffer,
    missing: &'a Cell<Option<(u64, u64)>>,
    pos: u64,
}

impl<'a> RangeReader<'a> {
    /// Wrap `buf` for sequential `Read + Seek` access; misses are recorded
    /// into `missing`.
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
        // `Read` contract: an empty buffer reads zero bytes, full stop —
        // it must not record a miss (which would trigger a spurious,
        // zero-byte-motivated fetch round) even at an uncovered offset.
        if out.is_empty() || self.pos >= self.buf.file_size() {
            return Ok(0);
        }
        if let Some(n) = self.buf.read_at(self.pos, out) {
            // Saturate: the `unwrap_or(u64::MAX)` fallback is unreachable
            // on 64-bit, but a wrapping `+=` there would corrupt the
            // cursor; pinning EOF is the harmless failure mode.
            self.pos = self
                .pos
                .saturating_add(u64::try_from(n).unwrap_or(u64::MAX));
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
pub const MIRROR_CONCURRENCY: usize = 6;

/// Transient-failure retries per mirror per fetch. Lighter than the API
/// client's 6: mirror failover is the real second chance here.
pub const MAX_MIRROR_ATTEMPTS: u32 = 3;

/// §5.2: global byte budget for the full-download fallback.
pub const FALLBACK_BYTE_BUDGET: u64 = 2 * 1024 * 1024 * 1024;

/// Run-wide transfer accounting (§9.3: total bytes transferred must stay a
/// small fraction of the archive). Every response-body byte this module
/// reads off the wire — from a ranged fetch or a full download, whether the
/// attempt ultimately succeeds or is discarded as over/short — is counted
/// here as it's read (never derived from a `Content-Length` header), along
/// with every request issued: Task 6's run-wide runaway breaker depends on
/// these being exact, not an under-count of failed attempts.
#[derive(Debug, Default)]
pub struct TransferCounters {
    /// GET requests issued (ranged and full).
    pub requests: AtomicU64,
    /// Response-body bytes read.
    pub bytes: AtomicU64,
}

impl TransferCounters {
    /// Zeroed counters for a fresh run.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

/// Breaker outcome for one entry that needs the full-download fallback.
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
/// fast when the pattern is systemic. `saturating_mul` rather than `*`:
/// `total_entries` is a corpus-wide count that in principle could approach
/// `u64::MAX / 2`, and this must never panic on it (match inspect.rs's
/// `saturating_add` posture toward values that cross a trust boundary).
fn fallback_limit(total_entries: u64) -> u64 {
    (total_entries.saturating_mul(2) / 100).max(2)
}

/// Build a mirror URL for one archive entry, keeping every constructed URL
/// on the mirror's own scheme/host/port no matter what `dir`/`filename`
/// contain — both come from the third-party idgames API (ADR-0030 §3/§5)
/// and are therefore untrusted. Built by appending percent-encoded path
/// *segments* (`Url::path_segments_mut`), never by parsing `dir`/`filename`
/// as a relative URL: `Url::join` would let an absolute (`https://evil...`),
/// protocol-relative (`//evil...`), or rooted (`/etc/passwd`) `filename`
/// replace the host or escape the base path outright, and would silently
/// truncate at an embedded `#`/`?` into a URL fragment/query instead of
/// sending those bytes as part of the request path — manufacturing a false
/// `not_found` fact for a real filename like `10nm####.zip`.
///
/// `filename` must be a single non-empty segment — a `/` in it is rejected
/// outright, never silently folded into an extra segment or percent-coded
/// away, and it may not be exactly `.` or `..` either (see below). `dir`
/// splits on `/` into its own segments, each held to the same `.`/`..`
/// rule. Every segment (from `base`, `dir`, and `filename` alike) is
/// percent-encoded independently by `Url`, so a literal `#`/`?`/space can
/// never cross a segment boundary or turn into a fragment/query.
///
/// A same-origin, same-path-prefix check runs on the built URL as defense
/// in depth: segment-by-segment construction should make it unreachable
/// (unlike `Url::join`, segment pushes cannot rewrite the scheme or host).
/// It catches a *resolved* escape — one that shortens the built path below
/// `base`'s own prefix — but it cannot catch a `.`/`..` **segment**: per
/// `url`'s own documented contract (verified against `url` 2.5.8's
/// `path_segments.rs`) such a segment is silently DROPPED, not resolved or
/// kept literal, so an unrejected `dir = "levels/../../x/"` would still
/// build a URL rooted under `base` — just the WRONG one (`.../x/`, not an
/// escape) — and the post-condition would never see anything amiss. That
/// silent-drop case is exactly what the explicit `.`/`..` segment rejection
/// above exists to catch instead.
///
/// # Errors
/// `filename` empty, containing `/`, or exactly `.`/`..`; a `dir` segment
/// that is exactly `.` or `..`; `base` not a valid URL or not hierarchical
/// (`path_segments_mut` fails on e.g. a `mailto:` URL); or the built URL
/// fails the same-origin/same-path-prefix post-condition.
pub(crate) fn entry_url(base: &str, dir: &str, filename: &str) -> anyhow::Result<reqwest::Url> {
    if filename.is_empty() || filename.contains('/') || filename == "." || filename == ".." {
        anyhow::bail!(
            "filename must be a single non-empty path segment, not \".\" or \"..\": {filename:?}"
        );
    }
    let base_url = reqwest::Url::parse(base)?;
    let mut url = base_url.clone();
    {
        let mut segments = url
            .path_segments_mut()
            .map_err(|()| anyhow::anyhow!("mirror base URL is not hierarchical: {base}"))?;
        segments.pop_if_empty();
        for seg in dir.split('/').filter(|s| !s.is_empty()) {
            if seg == "." || seg == ".." {
                anyhow::bail!(
                    "dir segment must not be \".\" or \"..\" — the url crate silently \
                     drops such segments instead of resolving or rejecting them, which \
                     would build an in-base but wrong URL: {dir:?}"
                );
            }
            segments.push(seg);
        }
        segments.push(filename);
    }
    let same_origin = url.scheme() == base_url.scheme()
        && url.host_str() == base_url.host_str()
        && url.port_or_known_default() == base_url.port_or_known_default();
    if !same_origin || !url.path().starts_with(base_url.path()) {
        anyhow::bail!("entry URL escaped the mirror base ({base}): {url}");
    }
    Ok(url)
}

/// Parse a `Content-Range: bytes {start}-{end}/{total}` response header
/// into inclusive `(start, end)` byte offsets plus the parsed `total`. The
/// `bytes` unit token is matched case-insensitively (RFC 9110 §8.4: range
/// units are case-insensitive), so `Bytes 0-9/100` parses the same as
/// `bytes 0-9/100`. `total` is `None` for the RFC-legal `*` ("total length
/// unknown") form and `Some(n)` for a numeric one; a *non*-`*`,
/// non-numeric total is garbage and fails the whole parse (`None`
/// overall), the same fail-closed posture as a malformed start/end. The
/// whole function returns `None` for anything else too, including the
/// `bytes */{total}` "range not satisfiable" form (no start/end to compare
/// against) and unparseable garbage — the caller treats an overall `None`
/// as a mismatch, never a pass: RFC 7233 §4.2 requires this header on
/// every `206`, so a missing or malformed one is itself suspicious, not a
/// shape to tolerate.
fn parse_content_range(value: &str) -> Option<(u64, u64, Option<u64>)> {
    let (unit, range) = value.split_once(' ')?;
    if !unit.eq_ignore_ascii_case("bytes") {
        return None;
    }
    let (range, total) = range.split_once('/')?;
    let (start, end) = range.split_once('-')?;
    let total = match total.trim() {
        "*" => None,
        n => Some(n.parse().ok()?),
    };
    Some((start.trim().parse().ok()?, end.trim().parse().ok()?, total))
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
                // `saturating_add`: an abusive/misbehaving mirror is
                // exactly the untrusted input this cap exists to police,
                // so the overflow check itself must never panic on it.
                if bytes.len().saturating_add(chunk.len()) > cap_usize {
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

/// What to do next after one body-read attempt: succeed, or retry/give up
/// with `attempt` already folded into the decision (both callers of
/// [`accept_body`] would otherwise repeat the same `attempt <
/// MAX_MIRROR_ATTEMPTS` check).
enum StreamOutcome {
    /// The body streamed cleanly within its cap.
    Success(Vec<u8>),
    /// A failed attempt with retries remaining.
    Retry(String),
    /// A failed attempt with no retries left — the candidate mirror is
    /// exhausted for this call.
    GiveUp(String),
}

/// [`stream_capped`] plus the shared attempt/retry bookkeeping used by both
/// [`RangeSource::fetch`]'s success paths and [`MirrorRanges::download_full`]:
/// success returns the bytes, failure is classified into `Retry` or
/// `GiveUp` by whether `attempt` has hit [`MAX_MIRROR_ATTEMPTS`].
async fn accept_body(
    resp: reqwest::Response,
    cap: u64,
    counters: &TransferCounters,
    over_detail: &str,
    attempt: u32,
) -> StreamOutcome {
    match stream_capped(resp, cap, counters, over_detail).await {
        Ok(bytes) => StreamOutcome::Success(bytes),
        Err(detail) => retry_or_give_up(detail, attempt),
    }
}

/// Classify a non-body failure detail (transport error, unexpected/retryable
/// status, mismatched `Content-Range`, ...) into `Retry` or `GiveUp` by
/// whether `attempt` has hit [`MAX_MIRROR_ATTEMPTS`] — the one place that
/// comparison is written, shared by every attempt-level call site so a
/// branch can never forget it (and, as a side effect, can never forget to
/// carry a detail string for [`MirrorRanges::fetch_exhausted`] either).
fn retry_or_give_up(detail: String, attempt: u32) -> StreamOutcome {
    if attempt < MAX_MIRROR_ATTEMPTS {
        StreamOutcome::Retry(detail)
    } else {
        StreamOutcome::GiveUp(detail)
    }
}

/// Validate a `206`'s `Content-Range` against the requested
/// `[offset, want_end]`, and its `total` against `expected_file_size`,
/// before trusting its body at all (fix round 1, Finding 2; total check:
/// review round 1): a proxy/CDN node that answers `206` for a *different*
/// range — or for the right range but a different-sized underlying file —
/// than requested must never have its bytes accepted and spliced into the
/// sparse buffer at `offset`, since either would silently fabricate sizes
/// in the durable output. A `*` total (RFC-legal "length unknown") is
/// accepted; a missing/unparseable header, a start/end mismatch, or a
/// numeric total that disagrees with `expected_file_size` are all treated
/// as a mismatch, never a pass (RFC 7233 §4.2 requires this header on
/// every `206`).
async fn accept_partial_content(
    resp: reqwest::Response,
    offset: u64,
    want_end: u64,
    len: u64,
    expected_file_size: u64,
    counters: &TransferCounters,
    attempt: u32,
) -> StreamOutcome {
    let content_range = resp
        .headers()
        .get(reqwest::header::CONTENT_RANGE)
        .and_then(|v| v.to_str().ok())
        .and_then(parse_content_range);
    match content_range {
        Some((start, end, total)) if start == offset && end == want_end => {
            if let Some(t) = total
                && t != expected_file_size
            {
                return retry_or_give_up(
                    format!("content-range total {t} != declared size {expected_file_size}"),
                    attempt,
                );
            }
            accept_body(resp, len, counters, "range over-delivery", attempt).await
        }
        Some((start, end, _)) => retry_or_give_up(
            format!("Content-Range mismatch: got {start}-{end}, wanted {offset}-{want_end}"),
            attempt,
        ),
        None => retry_or_give_up(
            "206 with a missing or unparseable Content-Range".to_owned(),
            attempt,
        ),
    }
}

// Compile-time tripwire: `MirrorRanges` hardcodes the pool arity in its
// `[Url; 2]`/`[bool; 2]` fields below rather than sizing them off
// `MIRRORS::len()`. If the mirror pool ever grows or shrinks, this assert
// fails the build instead of silently truncating/ignoring a mirror — a
// prompt to update those field types (and every array literal built
// against them) in the same change.
const _: () = assert!(
    MIRRORS.len() == 2,
    "MirrorRanges hardcodes the pool arity — update its [Url; 2]/[bool; 2] fields when the pool changes"
);

/// One archive entry's byte source: the §5.1 mirror pool with per-mirror
/// retries, 404/no-range failover, and capped body reads. Pins to the
/// first mirror that serves bytes so multi-round fetches never mix
/// mirrors (their copies could momentarily differ).
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
    #[must_use]
    pub fn mirror_key(&self) -> &'static str {
        self.pinned.map_or("", |i| MIRRORS[i].key)
    }

    /// Candidate mirror indices for [`RangeSource::fetch`]: the pinned
    /// mirror alone once one is set (multi-round fetches must never mix
    /// mirrors), else every mirror not yet disqualified by a `404` or a
    /// range refusal, in [`MIRRORS`] order.
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
    fn fetch_exhausted(&self, last_detail: Option<String>) -> FetchFailure {
        // §5.2: the budgeted fallback fires only when BOTH mirrors refuse
        // ranges. `RangeUnsupported` therefore requires a conclusive
        // verdict (404 or range refusal) from EVERY mirror, with at least
        // one refusal — a mirror that only failed transiently leaves the
        // entry `Http`, which records as an uncached `fetch_error` and
        // retries live next run instead of spending fallback budget on a
        // download that a healthy mirror may make unnecessary.
        let conclusive = self
            .not_found
            .iter()
            .zip(self.range_refused.iter())
            .all(|(&nf, &rr)| nf || rr);
        if self.not_found.iter().all(|&nf| nf) {
            FetchFailure::NotFound
        } else if conclusive && self.range_refused.iter().any(|&rr| rr) {
            FetchFailure::RangeUnsupported
        } else {
            FetchFailure::Http(last_detail.unwrap_or_else(|| "no mirrors available".into()))
        }
    }

    /// One (mirror, attempt) round of [`Self::download_full`]: send the
    /// plain GET, classify the response, and read the body if accepted.
    /// Only touches the disqualification flags (`not_found`) that are a
    /// permanent fact about mirror `i`; the caller owns `last_detail`,
    /// `pinned`, and the retry loop.
    async fn attempt_full_download(
        &mut self,
        i: usize,
        expected_size: u64,
        attempt: u32,
    ) -> StreamOutcome {
        self.counters.requests.fetch_add(1, Ordering::Relaxed);
        let outcome = self.http.get(self.urls[i].clone()).send().await;
        let resp = match outcome {
            Ok(r) => r,
            Err(e) => return retry_or_give_up(format!("transport: {e}"), attempt),
        };
        let status = resp.status();
        if status == reqwest::StatusCode::OK {
            accept_body(
                resp,
                expected_size,
                &self.counters,
                "body exceeded declared size",
                attempt,
            )
            .await
        } else if status == reqwest::StatusCode::NOT_FOUND {
            self.not_found[i] = true;
            StreamOutcome::GiveUp(format!("HTTP {status}"))
        } else if is_retryable(Some(status)) {
            retry_or_give_up(format!("HTTP {status}"), attempt)
        } else {
            StreamOutcome::GiveUp(format!("HTTP {status}"))
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
    pub async fn download_full(&mut self, expected_size: u64) -> Result<Vec<u8>, FetchFailure> {
        // The brief mandates this exact signature (Task 6 passes
        // `expected_size` explicitly even though it's also captured at
        // construction) — but the two must always agree; a caller passing
        // something else is a bug to catch in debug builds, not a case to
        // silently honor.
        debug_assert_eq!(
            expected_size, self.expected_file_size,
            "caller passed a different size than MirrorRanges::new was built with"
        );
        let mut last_detail: Option<String> = None;
        for i in self.full_candidates() {
            for attempt in 1..=MAX_MIRROR_ATTEMPTS {
                match self.attempt_full_download(i, expected_size, attempt).await {
                    StreamOutcome::Success(bytes) => {
                        self.pinned = Some(i);
                        return Ok(bytes);
                    }
                    StreamOutcome::Retry(detail) => {
                        last_detail = Some(detail);
                        tokio::time::sleep(backoff_delay(attempt, &mut self.rng)).await;
                    }
                    StreamOutcome::GiveUp(detail) => {
                        last_detail = Some(detail);
                        break;
                    }
                }
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

    /// One (mirror, attempt) round of [`RangeSource::fetch`]: send the
    /// ranged GET, classify the response — including validating a `206`'s
    /// `Content-Range` (fix round 1, Finding 2) — and read the body if
    /// accepted. Only touches the disqualification flags (`range_refused`,
    /// `not_found`) that are a permanent fact about mirror `i`; the caller
    /// owns `last_detail`, `pinned`, and the retry loop.
    async fn attempt_range_fetch(
        &mut self,
        i: usize,
        offset: u64,
        len: u64,
        want_end: u64,
        whole_file: bool,
        attempt: u32,
    ) -> StreamOutcome {
        self.counters.requests.fetch_add(1, Ordering::Relaxed);
        let outcome = self
            .http
            .get(self.urls[i].clone())
            .header(reqwest::header::RANGE, format!("bytes={offset}-{want_end}"))
            .send()
            .await;
        let resp = match outcome {
            Ok(r) => r,
            Err(e) => return retry_or_give_up(format!("transport: {e}"), attempt),
        };

        let status = resp.status();
        if status == reqwest::StatusCode::PARTIAL_CONTENT {
            accept_partial_content(
                resp,
                offset,
                want_end,
                len,
                self.expected_file_size,
                &self.counters,
                attempt,
            )
            .await
        } else if status == reqwest::StatusCode::OK && whole_file {
            accept_body(resp, len, &self.counters, "range over-delivery", attempt).await
        } else if status == reqwest::StatusCode::OK {
            // The server ignored the range but this wasn't a whole-file
            // request: a policy refusal, not a hiccup — no retry, just
            // disqualify this mirror.
            self.range_refused[i] = true;
            StreamOutcome::GiveUp(format!("HTTP {status}"))
        } else if status == reqwest::StatusCode::NOT_FOUND {
            self.not_found[i] = true;
            StreamOutcome::GiveUp(format!("HTTP {status}"))
        } else if is_retryable(Some(status)) {
            retry_or_give_up(format!("HTTP {status}"), attempt)
        } else {
            StreamOutcome::GiveUp(format!("HTTP {status}"))
        }
    }
}

impl RangeSource for MirrorRanges {
    /// §5.2: up to [`MAX_MIRROR_ATTEMPTS`] retries per candidate mirror
    /// (transient statuses/transport errors, backed off per
    /// [`backoff_delay`]), then failover to the next candidate; a `200` to
    /// a non-whole-file range or a `404` disqualifies a mirror outright
    /// (no retry — policy, not a hiccup). A `206` whose `Content-Range`
    /// doesn't echo back the requested extent is treated as an ordinary
    /// attempt failure (retried, then failed over) rather than trusted: a
    /// proxy/CDN node that silently serves a *different* range must never
    /// get its bytes spliced into the sparse buffer at the wrong offset.
    /// See [`Self::candidates`], [`Self::attempt_range_fetch`], and
    /// [`Self::fetch_exhausted`] for selection, per-attempt classification,
    /// and final classification respectively.
    async fn fetch(&mut self, offset: u64, len: u64) -> Result<Vec<u8>, FetchFailure> {
        let whole_file = offset == 0 && len == self.expected_file_size;
        // `saturating_add`/`saturating_sub`: `offset`/`len` ultimately trace
        // back to CD-derived, untrusted-input-adjacent values (inspect.rs's
        // posture), so the inclusive end computed here must never panic.
        let want_end = offset.saturating_add(len.saturating_sub(1));
        let mut last_detail: Option<String> = None;

        for i in self.candidates() {
            for attempt in 1..=MAX_MIRROR_ATTEMPTS {
                match self
                    .attempt_range_fetch(i, offset, len, want_end, whole_file, attempt)
                    .await
                {
                    StreamOutcome::Success(bytes) => {
                        self.pinned = Some(i);
                        return Ok(bytes);
                    }
                    StreamOutcome::Retry(detail) => {
                        last_detail = Some(detail);
                        tokio::time::sleep(backoff_delay(attempt, &mut self.rng)).await;
                    }
                    StreamOutcome::GiveUp(detail) => {
                        last_detail = Some(detail);
                        break;
                    }
                }
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

        // An empty read is Ok(0) even at an uncovered offset — the Read
        // contract; it must never record a (zero-length) miss.
        missing.set(None);
        r.seek(SeekFrom::Start(10)).unwrap();
        assert_eq!(r.read(&mut []).unwrap(), 0);
        assert_eq!(missing.get(), None);
    }

    #[test]
    fn out_of_bounds_inserts_are_discarded_not_corrupting() {
        let mut buf = SparseBuffer::new(100);
        // Spills past file_size → discarded.
        buf.insert(96, vec![1; 10]);
        let mut out = [0_u8; 4];
        assert_eq!(buf.read_at(96, &mut out), None);
        // Offset + len would overflow u64 → discarded, no panic.
        buf.insert(u64::MAX - 2, vec![1; 10]);
        assert_eq!(buf.read_at(u64::MAX - 2, &mut out), None);
        // An exactly-bounded insert still lands.
        buf.insert(90, vec![7; 10]);
        assert_eq!(buf.read_at(90, &mut out), Some(4));
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

    /// Fix round 1, Finding 1: `#` in a real filename (`10nm####.zip` is an
    /// actual idgames archive name) must become a percent-encoded path
    /// byte, never a URL fragment — a fragment would silently truncate the
    /// request path and manufacture a false `not_found`.
    #[test]
    fn hash_characters_in_filename_are_percent_encoded_not_a_fragment() {
        let url = entry_url(
            "https://ftpmirror1.infania.net/pub/idgames/",
            "lmps/some/dir/",
            "10nm####.zip",
        )
        .unwrap();
        assert!(url.as_str().ends_with("/10nm%23%23%23%23.zip"), "got {url}");
        assert!(url.fragment().is_none());
    }

    /// Finding 1: a `filename` is a single path segment by contract. An
    /// embedded `/` — whether from an absolute URL, a protocol-relative
    /// URL, a rooted path, or a `../` traversal sequence handed to us as
    /// "the filename" — is rejected outright rather than silently encoded
    /// into a segment or (worse, under the old `Url::join` construction)
    /// left to replace the host.
    #[test]
    fn filename_containing_a_slash_is_rejected() {
        let base = "https://ftpmirror1.infania.net/pub/idgames/";
        let dir = "levels/doom/0-9/";
        assert!(entry_url(base, dir, "https://evil.example/pwn.zip").is_err());
        assert!(entry_url(base, dir, "//evil.example/pwn.zip").is_err());
        assert!(entry_url(base, dir, "/absolute.zip").is_err());
        assert!(entry_url(base, dir, "../../evil.zip").is_err());
        assert!(entry_url(base, dir, "").is_err(), "empty filename");
    }

    /// Finding 1: a scheme-shaped or colon-bearing filename (no `/`, so not
    /// rejected by the single-segment rule) must stay a literal path
    /// segment on the mirror's own host — it must never be reparsed as if
    /// it introduced its own scheme/authority.
    #[test]
    fn scheme_shaped_filename_stays_on_the_mirror_host() {
        let url = entry_url(
            "https://ftpmirror1.infania.net/pub/idgames/",
            "levels/doom/0-9/",
            "weird:name.zip",
        )
        .unwrap();
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.host_str(), Some("ftpmirror1.infania.net"));
        assert!(
            url.path().starts_with("/pub/idgames/levels/doom/0-9/weird"),
            "got path {}",
            url.path()
        );
    }

    /// Final wave: a `dir` containing `../` dot-segments must be a definite
    /// `Err`, not merely "safe if it happens to succeed." Before the
    /// explicit per-segment `.`/`..` rejection, this was a vacuous
    /// either/or: the `url` crate silently DROPS a `.`/`..` segment rather
    /// than resolving it, so `"levels/../../x/"` built an URL rooted under
    /// `base` (passing the same-origin/prefix post-condition) but pointing
    /// at the WRONG path (`.../x/`, not an escape) — a false conclusive
    /// fact (e.g. `mirror_404_all`) cached as if it were about the real
    /// entry, or worst case a size attributed to the wrong file.
    #[test]
    fn dir_traversal_segments_stay_rooted_or_are_rejected() {
        let base = "https://ftpmirror1.infania.net/pub/idgames/";
        assert!(entry_url(base, "levels/../../x/", "f.zip").is_err());
        assert!(entry_url(base, "levels/./doom/", "f.zip").is_err());
    }

    /// Final wave: `filename` exactly `.` or `..` isn't caught by the
    /// existing "no `/`" rule, and the `url` crate would otherwise silently
    /// drop it as a path segment — building a mirror URL with no filename
    /// at all instead of failing loudly.
    #[test]
    fn filename_dot_and_dotdot_are_rejected() {
        let base = "https://ftpmirror1.infania.net/pub/idgames/";
        let dir = "levels/doom/0-9/";
        assert!(entry_url(base, dir, ".").is_err());
        assert!(entry_url(base, dir, "..").is_err());
    }

    /// Finding 2: `Content-Range` parsing — happy path, the `*/total`
    /// "range not satisfiable" form (no start/end to compare, so `None`),
    /// garbage, and a well-formed but different range (still parses; it's
    /// the caller's offset/len comparison that decides "mismatch").
    /// Review round 1: the `total` portion — a numeric total parses to
    /// `Some(n)`, the RFC-legal `*` ("length unknown") to `None`, and a
    /// non-`*`, non-numeric total fails the *whole* parse.
    #[test]
    fn parse_content_range_cases() {
        assert_eq!(
            parse_content_range("bytes 100-199/2000"),
            Some((100, 199, Some(2000)))
        );
        assert_eq!(parse_content_range("bytes */2000"), None);
        assert_eq!(parse_content_range("not a content range"), None);
        assert_eq!(parse_content_range(""), None);
        // A different-but-well-formed range: parsing succeeds; a caller
        // expecting e.g. (0, 9, ..) is the one that must treat this as a
        // mismatch, not this function.
        assert_eq!(
            parse_content_range("bytes 500-599/2000"),
            Some((500, 599, Some(2000)))
        );
        // RFC 9110 §8.4: range units are case-insensitive.
        assert_eq!(
            parse_content_range("Bytes 0-9/100"),
            Some((0, 9, Some(100)))
        );
        assert_eq!(
            parse_content_range("BYTES 0-9/100"),
            Some((0, 9, Some(100)))
        );
        // `*` total ("length unknown", RFC-legal): parses to `None`, not a
        // parse failure — the caller decides whether an unknown total is
        // acceptable.
        assert_eq!(parse_content_range("bytes 0-9/*"), Some((0, 9, None)));
        // A non-`*`, non-numeric total is garbage: fail the whole parse,
        // same fail-closed posture as a malformed start/end.
        assert_eq!(parse_content_range("bytes 0-9/garbage"), None);
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

    #[test]
    fn exhaustion_classification_requires_conclusive_range_refusal() {
        let counters = std::sync::Arc::new(TransferCounters::default());
        let mut m = MirrorRanges::new(
            reqwest::Client::new(),
            "levels/doom/0-9/",
            "a.zip",
            100,
            counters,
        )
        .unwrap();
        // Transient-only failures: Http, detail preserved.
        assert!(matches!(
            m.fetch_exhausted(Some("HTTP 500".into())),
            FetchFailure::Http(d) if d == "HTTP 500"
        ));
        // One refusal + one transient: still Http (§5.2 — the fallback
        // fires only when BOTH mirrors refuse ranges).
        m.range_refused[0] = true;
        assert!(matches!(m.fetch_exhausted(None), FetchFailure::Http(_)));
        // Refusal + 404: conclusive on every mirror → RangeUnsupported.
        m.not_found[1] = true;
        assert!(matches!(
            m.fetch_exhausted(None),
            FetchFailure::RangeUnsupported
        ));
        // Both refused: RangeUnsupported.
        m.not_found[1] = false;
        m.range_refused[1] = true;
        assert!(matches!(
            m.fetch_exhausted(None),
            FetchFailure::RangeUnsupported
        ));
        // Both 404: NotFound wins over everything.
        m.range_refused = [false, false];
        m.not_found = [true, true];
        assert!(matches!(m.fetch_exhausted(None), FetchFailure::NotFound));
    }
}
