//! Single-URL `RangeSource` for the §6.4 modern-outliers supplement.
//!
//! Curated modern megawads (Cacoward-tier releases) mostly live off idgames,
//! at direct-download URLs rather than the §5.1 mirror pool, so phase-3's
//! `harvest-outliers` command needs a [`RangeSource`] that speaks to exactly
//! one arbitrary URL instead of
//! [`crate::zips::range_reader::MirrorRanges`]'s two-mirror failover.
//! [`UrlRanges`] is that source: it reuses the same `RangeSource` seam
//! `inspect::inspect_zip` already drives, so outlier entries get identical
//! central-directory-only inspection with zero new parsing code
//! (DESIGN.md §6.4).
//!
//! Unlike `MirrorRanges`, there is **no `download_full` analog here** — spec
//! §2.2 makes that a locked decision, not an oversight ("A host that refuses
//! range requests gets a `no_range_support` ledger entry ... a fallback
//! would blow the politeness budget"). Outliers are large by design;
//! downloading one in full to work around a range-refusing host is exactly
//! the cost this type exists to avoid.
//!
//! A single URL has no failover *partner* — there's no second host to try
//! when this one is down — so [`UrlRanges`] never duplicates
//! `MirrorRanges`'s per-mirror failover loop (`attempt_range_fetch`,
//! `StreamOutcome`, `fetch_exhausted` — all private to `range_reader.rs`).
//! It does, however, retry a single transient failure against the *same*
//! host: `api::client`'s retry primitives
//! ([`crate::api::client::is_retryable`], [`crate::api::client::backoff_delay`])
//! are `pub(crate)` for exactly this kind of crate-wide reuse, so
//! [`UrlRanges::discover_size`]/[`UrlRanges::fetch`] apply the same
//! jittered-exponential-backoff policy `ApiClient::request` uses (§4.6),
//! bounded by the same [`MAX_ATTEMPTS`]. A classification outcome —
//! `RangeUnsupported`, `NotFound`, or a non-retryable `Http` status — is
//! terminal on the first attempt; only a transport error or a retryable
//! status (429/5xx) ever gets a second try. See [`should_retry`] for the
//! pure decision and its unit tests.
//!
//! Redirects: the phase-2 client
//! ([`crate::mirror::build_zips_http`]) never overrides `reqwest`'s default
//! redirect policy (up to 10 hops), and this type doesn't either — that
//! default is what resolves a ModDB/itch-style download-redirect chain to
//! the real file, with no special-casing needed here.

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use crate::api::client::{backoff_delay, is_retryable};
use crate::zips::inspect::{FetchFailure, RangeSource};
use crate::zips::range_reader::TransferCounters;

/// Retry bound for [`UrlRanges::discover_size`]/[`UrlRanges::fetch`],
/// matching `api::client::ApiClient::request`'s policy (§4.6) rather than
/// `range_reader::MAX_MIRROR_ATTEMPTS`: a lone URL with no failover partner
/// gets the API client's more patient budget (6 attempts against the one
/// host it has) instead of the mirror pool's per-candidate 3, since giving
/// up early here has no second host to fall back to.
const MAX_ATTEMPTS: u32 = 6;

/// One arbitrary URL's byte source (§6.4): no mirror pool, no failover, no
/// full-download fallback (spec §2.2) — just ranged GETs against one host,
/// with the same bounded retry policy `api::client` uses (see the module
/// doc and [`should_retry`]).
// consumed from Task 5 (#407)
#[allow(dead_code)]
#[derive(Debug)]
pub struct UrlRanges {
    /// Shared HTTP client (the caller's concern — UA, timeouts, redirect
    /// policy: see [`crate::mirror::build_zips_http`]).
    http: reqwest::Client,
    /// The one URL this source ever reads.
    url: reqwest::Url,
    /// Learned from [`Self::discover_size`]; `None` until then. A
    /// [`RangeSource::fetch`] called before discovery treats every request
    /// as *not* whole-file rather than guessing — see that method's doc
    /// comment.
    file_size: Option<u64>,
    /// Run-wide transfer accounting, shared with every other source in the
    /// run (the same [`TransferCounters`] type `MirrorRanges` uses).
    counters: Arc<TransferCounters>,
    /// Backoff jitter source (one per instance, like `MirrorRanges`'s).
    rng: fastrand::Rng,
}

impl UrlRanges {
    /// Build a byte source for one outlier URL. Unlike
    /// [`crate::zips::range_reader::MirrorRanges::new`], this never fails —
    /// `url` is already a parsed [`reqwest::Url`], so there is no
    /// segment-construction step that could reject it.
    #[must_use]
    // consumed from Task 5 (#407)
    #[allow(dead_code)]
    pub fn new(http: reqwest::Client, url: reqwest::Url, counters: Arc<TransferCounters>) -> Self {
        Self {
            http,
            url,
            file_size: None,
            counters,
            rng: fastrand::Rng::new(),
        }
    }

    /// `HEAD` the URL (following the client's redirect policy, retrying a
    /// transport error or a retryable status per [`should_retry`]); if the
    /// HEAD doesn't carry a usable `Content-Length`, fall back to a
    /// single-byte ranged-GET size probe
    /// ([`Self::discover_size_via_range_probe`]). Either way, the learned
    /// size is recorded for later [`RangeSource::fetch`] calls to
    /// recognize a whole-file request.
    ///
    /// **Why not [`reqwest::Response::content_length`]:** that method
    /// returns the *decoded response body's* size hint, not the literal
    /// `Content-Length` response header — for a `HEAD`, the body is empty
    /// by definition, so it reliably returns `Some(0)` even against a host
    /// that sent a perfectly good `Content-Length` header. A prior version
    /// of this method used it directly and silently recorded `file_size =
    /// 0` against two verified-cooperative hosts (a GitHub release asset,
    /// a Squarespace static file) in the Task 8 live smoke — `inspect_zip`
    /// then failed every one of those entries as a bogus
    /// `zip_parse_error("zero-length file")`, with zero bytes ever
    /// transferred and the real cause (a `discover_size` bug, not a bad
    /// archive) completely hidden. This method reads the header text
    /// directly instead — see [`classify_head_response`] — so this class
    /// of bug can't recur silently.
    ///
    /// # Errors
    /// - A `404` (on either the HEAD or the range-probe fallback) →
    ///   [`FetchFailure::NotFound`] (consistent with
    ///   [`classify_range_response`]'s `fetch`-side mapping), never retried.
    /// - Any other non-success HEAD status (after retries are exhausted, or
    ///   immediately for a non-retryable one) →
    ///   `FetchFailure::Http("HEAD status {status}")`. A HEAD landing on a
    ///   WAF block, a HEAD-disabled host, or a CDN edge case commonly
    ///   answers `403`/`405`/similar with its own small `Content-Length`
    ///   (an error page's, not the file's) — trusting that length
    ///   unconditionally would poison [`Self::file_size`] with a bogus
    ///   value and misclassify a perfectly healthy host's later `fetch` as
    ///   a parse failure, so the status is checked *before* the header is
    ///   ever read (see [`classify_head_response`]).
    /// - Transport failure on the HEAD, after retries are exhausted →
    ///   `FetchFailure::Http("HEAD transport: {e}")`.
    /// - A range-probe `200` (the host ignores ranges outright) →
    ///   [`FetchFailure::RangeUnsupported`] — see
    ///   [`Self::discover_size_via_range_probe`] for the fallback's own
    ///   error shapes.
    // consumed from Task 5 (#407)
    #[allow(dead_code)]
    pub async fn discover_size(&mut self) -> Result<u64, FetchFailure> {
        let size = match self.head_content_length().await? {
            Some(size) => size,
            None => self.discover_size_via_range_probe().await?,
        };
        self.file_size = Some(size);
        Ok(size)
    }

    /// `HEAD` the URL, retried per [`should_retry`], classified by
    /// [`classify_head_response`]. `Ok(Some(size))` — a usable declared
    /// size; `Ok(None)` — a success status but no usable `Content-Length`
    /// (the caller should fall back to
    /// [`Self::discover_size_via_range_probe`]); `Err` — terminal
    /// (`NotFound`/`Http`).
    async fn head_content_length(&mut self) -> Result<Option<u64>, FetchFailure> {
        let mut attempt = 0_u32;
        loop {
            attempt += 1;
            self.counters.requests.fetch_add(1, Ordering::Relaxed);
            let outcome = self.http.head(self.url.clone()).send().await;
            let resp = match outcome {
                Ok(r) => r,
                Err(e) => {
                    if let Some(delay) = should_retry(attempt, None, &mut self.rng) {
                        tokio::time::sleep(delay).await;
                        continue;
                    }
                    return Err(FetchFailure::Http(format!("HEAD transport: {e}")));
                }
            };

            let status = resp.status();
            let content_length = resp
                .headers()
                .get(reqwest::header::CONTENT_LENGTH)
                .and_then(|v| v.to_str().ok());
            match classify_head_response(status.as_u16(), content_length) {
                HeadOutcome::UseSize(size) => return Ok(Some(size)),
                HeadOutcome::Fallback => return Ok(None),
                HeadOutcome::NotFound => return Err(FetchFailure::NotFound),
                HeadOutcome::Fail => {
                    if let Some(delay) = should_retry(attempt, Some(status), &mut self.rng) {
                        tokio::time::sleep(delay).await;
                        continue;
                    }
                    return Err(FetchFailure::Http(format!("HEAD status {status}")));
                }
            }
        }
    }

    /// Fallback size discovery for a host whose `HEAD` doesn't carry a
    /// usable `Content-Length` (the Task 8 smoke's GitHub-release-asset and
    /// Squarespace-static-file hosts both need this path). Sends
    /// `Range: bytes=0-0` on the same URL — a single-byte ranged GET,
    /// counted like any other request/body byte via [`read_capped_body`] —
    /// and reads the declared total size back out of a `206`'s
    /// `Content-Range: bytes 0-0/TOTAL` header via
    /// [`parse_content_range_total`].
    ///
    /// # Errors
    /// - `200` → [`FetchFailure::RangeUnsupported`]: the host ignores
    ///   ranges outright, which is hopeless for the central-directory-only
    ///   reads this type exists to do — not a smaller ask to retry.
    /// - `404` → [`FetchFailure::NotFound`].
    /// - A `206` with a missing, unparseable, or RFC-7233-"unsatisfied"
    ///   `Content-Range` → `FetchFailure::Http` naming the defect —
    ///   RFC 7233 §4.2 requires this header on every `206`, so a malformed
    ///   one is itself suspicious, never trusted.
    /// - Any other status, or a transport failure, retried per
    ///   [`should_retry`] then surfaced as `FetchFailure::Http`.
    /// - A short single-byte body on an otherwise-valid `206` →
    ///   whatever [`read_capped_body`] reports (never retried, matching its
    ///   own documented posture).
    async fn discover_size_via_range_probe(&mut self) -> Result<u64, FetchFailure> {
        let mut attempt = 0_u32;
        loop {
            attempt += 1;
            self.counters.requests.fetch_add(1, Ordering::Relaxed);
            let outcome = self
                .http
                .get(self.url.clone())
                .header(reqwest::header::RANGE, "bytes=0-0")
                .send()
                .await;
            let resp = match outcome {
                Ok(r) => r,
                Err(e) => {
                    if let Some(delay) = should_retry(attempt, None, &mut self.rng) {
                        tokio::time::sleep(delay).await;
                        continue;
                    }
                    return Err(FetchFailure::Http(format!("range-probe transport: {e}")));
                }
            };

            let status = resp.status();
            if status == reqwest::StatusCode::NOT_FOUND {
                return Err(FetchFailure::NotFound);
            }
            if status == reqwest::StatusCode::OK {
                return Err(FetchFailure::RangeUnsupported);
            }
            if status != reqwest::StatusCode::PARTIAL_CONTENT {
                if let Some(delay) = should_retry(attempt, Some(status), &mut self.rng) {
                    tokio::time::sleep(delay).await;
                    continue;
                }
                return Err(FetchFailure::Http(format!("range-probe status {status}")));
            }

            let total = resp
                .headers()
                .get(reqwest::header::CONTENT_RANGE)
                .and_then(|v| v.to_str().ok())
                .and_then(parse_content_range_total);
            let Some(total) = total else {
                return Err(FetchFailure::Http(
                    "range-probe 206 with a missing, unparseable, or unsatisfied Content-Range"
                        .to_owned(),
                ));
            };

            // Drain and count the single requested byte (matches this
            // module's "every wire byte read is counted" posture); its
            // content is irrelevant — only `total` matters here. Never
            // retried on a short/failed read, matching `read_capped_body`'s
            // own documented posture.
            read_capped_body(resp, 1, &self.counters).await?;
            return Ok(total);
        }
    }
}

impl RangeSource for UrlRanges {
    /// Ranged fetch against the one URL, retrying a transport error or a
    /// retryable status (429/5xx) per [`should_retry`] — unlike
    /// [`crate::zips::range_reader::MirrorRanges::fetch`]'s per-mirror
    /// failover, there is no second host to fail over *to*, so every retry
    /// is against this same URL, up to [`MAX_ATTEMPTS`]. A classification
    /// outcome (`RangeUnsupported`, `NotFound`, or a non-retryable `Http`
    /// status) is terminal immediately — see the module doc.
    ///
    /// `whole_file` is `offset == 0 && len >= file_size`, but only once
    /// [`Self::discover_size`] has actually learned `file_size`; before
    /// that (or if it was never called) it's unconditionally `false` — an
    /// unknown file size can never be confirmed to match a "whole file"
    /// request, so treating it as partial is the only sound default. The
    /// `>=` (not `==`) tolerates the caller-supplied `len` disagreeing with
    /// the declared size by a few bytes without refusing a legitimate
    /// whole-file `200`.
    async fn fetch(&mut self, offset: u64, len: u64) -> Result<Vec<u8>, FetchFailure> {
        let whole_file = self
            .file_size
            .is_some_and(|file_size| offset == 0 && len >= file_size);
        // `saturating_add`/`saturating_sub`: `offset`/`len` ultimately trace
        // back to CD-derived, untrusted-input-adjacent values (inspect.rs's
        // posture), so the inclusive end computed here must never panic.
        let want_end = offset.saturating_add(len.saturating_sub(1));

        let mut attempt = 0_u32;
        loop {
            attempt += 1;
            self.counters.requests.fetch_add(1, Ordering::Relaxed);
            let outcome = self
                .http
                .get(self.url.clone())
                .header(reqwest::header::RANGE, format!("bytes={offset}-{want_end}"))
                .send()
                .await;
            let resp = match outcome {
                Ok(r) => r,
                Err(e) => {
                    if let Some(delay) = should_retry(attempt, None, &mut self.rng) {
                        tokio::time::sleep(delay).await;
                        continue;
                    }
                    return Err(FetchFailure::Http(format!("transport: {e}")));
                }
            };

            let status = resp.status();
            match classify_range_response(status.as_u16(), whole_file) {
                RangeOutcome::UsePartial | RangeOutcome::UseFullBody => {
                    return read_capped_body(resp, len, &self.counters).await;
                }
                RangeOutcome::RangeUnsupported => return Err(FetchFailure::RangeUnsupported),
                RangeOutcome::NotFound => return Err(FetchFailure::NotFound),
                RangeOutcome::Http => {
                    if let Some(delay) = should_retry(attempt, Some(status), &mut self.rng) {
                        tokio::time::sleep(delay).await;
                        continue;
                    }
                    return Err(FetchFailure::Http(format!("HTTP {status}")));
                }
            }
        }
    }
}

/// Bounded retry policy shared by [`UrlRanges::discover_size`] and
/// [`UrlRanges::fetch`]: reuses `api::client`'s classification
/// ([`is_retryable`]) and backoff shape ([`backoff_delay`]) rather than
/// duplicating either — both are `pub(crate)` precisely for crate-wide
/// reuse; only `MirrorRanges`'s per-mirror failover orchestration
/// (`attempt_range_fetch`/`StreamOutcome`/`fetch_exhausted`) is private to
/// `range_reader.rs`. `status = None` means a transport-level failure
/// (matches `is_retryable`'s own `None` case). Pure and side-effect-free
/// beyond consuming `rng`, so the terminal/retryable split is exercised
/// directly by unit tests with no live server or `reqwest::Response`.
///
/// Returns `None` (terminal — the caller gives up) once `attempt` reaches
/// [`MAX_ATTEMPTS`] or `status` isn't retryable at all (`is_retryable`
/// accepts only `None`/`429`/5xx — a `404`, a client error, or any of this
/// module's own classification outcomes are never passed here in the first
/// place; see the call sites). Otherwise `Some(delay)`: sleep `delay`, then
/// retry.
// consumed from Task 5 (#407)
#[allow(dead_code)]
fn should_retry(
    attempt: u32,
    status: Option<reqwest::StatusCode>,
    rng: &mut fastrand::Rng,
) -> Option<Duration> {
    if attempt >= MAX_ATTEMPTS || !is_retryable(status) {
        return None;
    }
    Some(backoff_delay(attempt, rng))
}

/// Stream `resp`'s body, counting every wire byte into `counters.bytes` as
/// it's read (matching `range_reader`'s accounting posture: a discarded or
/// truncated byte still moved over the wire, so it still counts) — capped
/// at `len`: once `len` bytes are buffered, no further chunks are read at
/// all. A lying or misbehaving host must not be allowed to OOM this
/// process, so this truncates rather than erroring on an over-long body
/// (unlike `MirrorRanges::fetch`'s hard cap-exceeded error — a lone URL
/// with no failover partner gets the body it asked for and nothing more,
/// rather than failing an entry outright over a chatty extra byte or two).
/// Never retried, matching `ApiClient::request`'s posture toward a body
/// that fails mid-stream (§4.6): a body-stage failure is exchange-level,
/// not a fresh-request-worthy transient, so [`UrlRanges::fetch`]'s retry
/// loop never re-enters once a response has started streaming.
///
/// A body that ends up short of `len` (host closed early, or genuinely
/// under-delivered) is reported as [`FetchFailure::Http`] naming both
/// sides of the mismatch — accepting a short body would silently
/// under-report an outlier's declared central-directory/member sizes.
// consumed from Task 5 (#407)
#[allow(dead_code)]
async fn read_capped_body(
    mut resp: reqwest::Response,
    len: u64,
    counters: &TransferCounters,
) -> Result<Vec<u8>, FetchFailure> {
    let cap = usize::try_from(len).unwrap_or(usize::MAX);
    let mut bytes: Vec<u8> = Vec::new();
    while bytes.len() < cap {
        match resp.chunk().await {
            Ok(Some(chunk)) => {
                counters.bytes.fetch_add(
                    u64::try_from(chunk.len()).unwrap_or(u64::MAX),
                    Ordering::Relaxed,
                );
                let remaining = cap - bytes.len();
                let take = remaining.min(chunk.len());
                bytes.extend_from_slice(&chunk[..take]);
            }
            Ok(None) => break,
            Err(e) => return Err(FetchFailure::Http(format!("body transport: {e}"))),
        }
    }
    if bytes.len() < cap {
        return Err(FetchFailure::Http(format!(
            "short body: got {} bytes, wanted {len}",
            bytes.len()
        )));
    }
    Ok(bytes)
}

/// What [`UrlRanges::fetch`] should do with one response, from its status
/// and whether the request covered the entire file. Pure and
/// side-effect-free so the six cases in `range_response_classification`
/// exercise it directly, with no HTTP mock needed (none exists in this
/// crate — see the task-4 brief).
// consumed from Task 5 (#407)
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RangeOutcome {
    /// `206` — read the body, capped at the requested length.
    UsePartial,
    /// `200` to a whole-file request: `MirrorRanges` precedent — the server
    /// ignored the range but happened to return exactly what was asked
    /// for, which is a legal answer, not a refusal.
    UseFullBody,
    /// `200` to a non-whole-file request: the server ignored the `Range`
    /// header on a genuinely partial request — no range support.
    RangeUnsupported,
    /// `404` — the URL doesn't resolve.
    NotFound,
    /// Anything else (5xx, other 4xx, an unexpected 2xx/3xx) — a
    /// transport-ish detail for the caller to report (retried first if
    /// [`is_retryable`] accepts the status; see [`should_retry`]).
    Http,
}

/// Pure classifier: `(status, whole_file)` → what [`UrlRanges::fetch`]
/// should do next. See [`RangeOutcome`] for the case-by-case rationale.
// consumed from Task 5 (#407)
#[allow(dead_code)]
pub(crate) fn classify_range_response(status: u16, whole_file: bool) -> RangeOutcome {
    match status {
        206 => RangeOutcome::UsePartial,
        200 if whole_file => RangeOutcome::UseFullBody,
        200 => RangeOutcome::RangeUnsupported,
        404 => RangeOutcome::NotFound,
        _ => RangeOutcome::Http,
    }
}

/// What [`UrlRanges::head_content_length`] should do with one `HEAD`
/// response, from its status and the raw `Content-Length` header text (if
/// any). Pure and side-effect-free, mirroring [`RangeOutcome`]'s role for
/// `fetch` — so the status/header decision is unit-tested without a live
/// `reqwest::Response`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HeadOutcome {
    /// A success status with a usable declared size (non-zero, parseable).
    UseSize(u64),
    /// A success status but no usable `Content-Length` — the caller should
    /// fall back to [`UrlRanges::discover_size_via_range_probe`].
    Fallback,
    /// `404` — terminal, [`FetchFailure::NotFound`].
    NotFound,
    /// Any other non-success status — terminal `Http` detail, unless
    /// [`should_retry`] accepts it first.
    Fail,
}

/// Pure classifier: `(status, content_length_header)` → what
/// [`UrlRanges::head_content_length`] should do next. `content_length` is
/// the raw header *text*, not [`reqwest::Response::content_length`] (see
/// [`UrlRanges::discover_size`]'s doc comment for why that method is wrong
/// here): missing, unparseable, or literally `"0"` are all "unusable" —
/// [`HeadOutcome::Fallback`] — since a zero-length remote zip is bogus
/// input regardless of how (or whether) the header spelled it, so `0` is
/// never passed onward as a real size.
// consumed from Task 5 (#407)
#[allow(dead_code)]
pub(crate) fn classify_head_response(status: u16, content_length: Option<&str>) -> HeadOutcome {
    if status == 404 {
        return HeadOutcome::NotFound;
    }
    if !(200..300).contains(&status) {
        return HeadOutcome::Fail;
    }
    match content_length
        .and_then(|s| s.trim().parse::<u64>().ok())
        .filter(|&n| n != 0)
    {
        Some(n) => HeadOutcome::UseSize(n),
        None => HeadOutcome::Fallback,
    }
}

/// Extract `TOTAL` from a `Content-Range: bytes START-END/TOTAL` header
/// value, as sent on a `206` to [`UrlRanges::discover_size_via_range_probe`]'s
/// `bytes=0-0` probe. `None` for:
/// - the RFC 7233 §4.2 "unsatisfied range" form (`bytes */TOTAL` — no
///   start/end to anchor the probe's request to);
/// - the "length unknown" form (`bytes START-END/*` — nothing to report);
/// - anything that doesn't parse as `bytes <range>/<total>` at all, or
///   whose `TOTAL` digits don't fit a `u64`;
/// - a `TOTAL` of exactly `0` — same invariant [`classify_head_response`]
///   enforces on the HEAD path ("0 is never passed onward as a real
///   size"): a zero-length remote zip is bogus input regardless of which
///   path reported it, so this is the Content-Range-side half of that same
///   guarantee, not a second, independent decision to keep in sync by
///   hand. Filtered here, in the pure parser, rather than at the call
///   site: every caller of this function inherits the "0 means unusable"
///   contract for free, with no risk of a future call site forgetting to
///   filter it out itself.
///
/// This function doesn't need to distinguish *why* a total wasn't
/// recoverable, only whether one was — every unusable shape is treated
/// identically by the caller (surfaced as a `FetchFailure::Http` detail).
// consumed from Task 5 (#407)
#[allow(dead_code)]
pub(crate) fn parse_content_range_total(value: &str) -> Option<u64> {
    let (unit, range) = value.split_once(' ')?;
    if !unit.eq_ignore_ascii_case("bytes") {
        return None;
    }
    let (range, total) = range.split_once('/')?;
    if range.trim() == "*" {
        return None;
    }
    match total.trim() {
        "*" => None,
        n => n.parse().ok().filter(|&n| n != 0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_response_classification() {
        use RangeOutcome::*;
        assert!(matches!(classify_range_response(206, false), UsePartial));
        assert!(matches!(classify_range_response(206, true), UsePartial));
        // 200 to a whole-file range is a legal "server ignored the range"
        // (MirrorRanges precedent); 200 to a partial range means no range support.
        assert!(matches!(classify_range_response(200, true), UseFullBody));
        assert!(matches!(
            classify_range_response(200, false),
            RangeUnsupported
        ));
        assert!(matches!(classify_range_response(404, false), NotFound));
        assert!(matches!(classify_range_response(500, false), Http));
    }

    #[test]
    fn should_retry_distinguishes_terminal_from_retryable() {
        let mut rng = fastrand::Rng::with_seed(7);
        // Transport error (None): retryable while attempts remain.
        assert!(should_retry(1, None, &mut rng).is_some());
        // Retryable statuses (429/5xx): retry while attempts remain.
        assert!(should_retry(1, Some(reqwest::StatusCode::TOO_MANY_REQUESTS), &mut rng).is_some());
        assert!(should_retry(1, Some(reqwest::StatusCode::BAD_GATEWAY), &mut rng).is_some());
        // Non-retryable statuses are terminal even on the very first attempt.
        assert!(should_retry(1, Some(reqwest::StatusCode::NOT_FOUND), &mut rng).is_none());
        assert!(should_retry(1, Some(reqwest::StatusCode::BAD_REQUEST), &mut rng).is_none());
        assert!(should_retry(1, Some(reqwest::StatusCode::FORBIDDEN), &mut rng).is_none());
        assert!(should_retry(1, Some(reqwest::StatusCode::OK), &mut rng).is_none());
        // Attempt bound exhausted: terminal even for an otherwise-retryable status.
        assert!(
            should_retry(
                MAX_ATTEMPTS,
                Some(reqwest::StatusCode::SERVICE_UNAVAILABLE),
                &mut rng
            )
            .is_none()
        );
        assert!(should_retry(MAX_ATTEMPTS + 1, None, &mut rng).is_none());
    }

    #[test]
    fn head_response_classification() {
        use HeadOutcome::*;
        // Usable, non-zero Content-Length on a success status.
        assert!(matches!(
            classify_head_response(200, Some("2012026")),
            UseSize(2_012_026)
        ));
        // Whitespace around the digits is tolerated.
        assert!(matches!(
            classify_head_response(200, Some(" 42 ")),
            UseSize(42)
        ));
        // Missing header, unparseable header, and literal "0" are all
        // "unusable" — this is the exact bug the Task 8 smoke caught:
        // `resp.content_length()` silently returned `Some(0)` for a HEAD's
        // always-empty body, poisoning `file_size`.
        assert!(matches!(classify_head_response(200, None), Fallback));
        assert!(matches!(
            classify_head_response(200, Some("nope")),
            Fallback
        ));
        assert!(matches!(classify_head_response(200, Some("0")), Fallback));
        // Every 2xx counts as success, not just 200.
        assert!(matches!(
            classify_head_response(204, Some("10")),
            UseSize(10)
        ));
        // 404 is its own terminal case, distinct from other failures.
        assert!(matches!(classify_head_response(404, None), NotFound));
        assert!(matches!(classify_head_response(404, Some("123")), NotFound));
        // Any other non-success status is a generic failure.
        assert!(matches!(classify_head_response(403, None), Fail));
        assert!(matches!(classify_head_response(500, Some("10")), Fail));
        assert!(matches!(classify_head_response(301, None), Fail));
    }

    #[test]
    fn content_range_total_extraction() {
        assert_eq!(
            parse_content_range_total("bytes 0-0/2012026"),
            Some(2_012_026)
        );
        // RFC 7233 §4.2 unsatisfied-range form: no start/end to anchor to.
        assert_eq!(parse_content_range_total("bytes */123"), None);
        // "Length unknown" form: nothing to report.
        assert_eq!(parse_content_range_total("bytes 0-0/*"), None);
        // Garbage doesn't parse as `bytes <range>/<total>` at all.
        assert_eq!(parse_content_range_total("garbage value"), None);
        assert_eq!(parse_content_range_total(""), None);
        // Digits that don't fit a u64 (28 nines) must not panic — just
        // fail to parse.
        assert_eq!(
            parse_content_range_total("bytes 0-0/9999999999999999999999999999"),
            None
        );
        // A literal zero total must never flow through as a real size —
        // the same "0 is unusable" invariant `classify_head_response`
        // enforces on the HEAD path, enforced here on the Content-Range
        // path (fix round 3: this exact shape reached `discover_size` and
        // set `file_size = Some(0)` before this filter existed).
        assert_eq!(parse_content_range_total("bytes 0-0/0"), None);
    }

    #[test]
    fn should_retry_returns_a_positive_backoff_delay() {
        let mut rng = fastrand::Rng::with_seed(3);
        let delay = should_retry(
            2,
            Some(reqwest::StatusCode::INTERNAL_SERVER_ERROR),
            &mut rng,
        )
        .expect("attempt 2 of MAX_ATTEMPTS with a retryable status must retry");
        assert!(delay.as_secs_f64() > 0.0);
    }
}
