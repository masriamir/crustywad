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
    /// transport error or a retryable status per [`should_retry`]) and
    /// record its `Content-Length` as the known file size for later
    /// [`RangeSource::fetch`] calls to recognize a whole-file request.
    ///
    /// # Errors
    /// - A `404` response → [`FetchFailure::NotFound`] (consistent with
    ///   [`classify_range_response`]'s `fetch`-side mapping), never retried.
    /// - Any other non-success status (after retries are exhausted, or
    ///   immediately for a non-retryable one) →
    ///   `FetchFailure::Http("HEAD status {status}")`. A HEAD landing on a
    ///   WAF block, a HEAD-disabled host, or a CDN edge case commonly
    ///   answers `403`/`405`/similar with its own small `Content-Length`
    ///   (an error page's, not the file's) — trusting that length
    ///   unconditionally would poison [`Self::file_size`] with a bogus
    ///   value and misclassify a perfectly healthy host's later `fetch` as
    ///   a parse failure, so the status is checked *before* the header is
    ///   ever read.
    /// - Transport failure after retries are exhausted →
    ///   `FetchFailure::Http("HEAD transport: {e}")`.
    /// - A `2xx` response with no `Content-Length` header →
    ///   `FetchFailure::Http("no Content-Length on HEAD")`.
    // consumed from Task 5 (#407)
    #[allow(dead_code)]
    pub async fn discover_size(&mut self) -> Result<u64, FetchFailure> {
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
            if status == reqwest::StatusCode::NOT_FOUND {
                return Err(FetchFailure::NotFound);
            }
            if !status.is_success() {
                if let Some(delay) = should_retry(attempt, Some(status), &mut self.rng) {
                    tokio::time::sleep(delay).await;
                    continue;
                }
                return Err(FetchFailure::Http(format!("HEAD status {status}")));
            }

            let size = resp
                .content_length()
                .ok_or_else(|| FetchFailure::Http("no Content-Length on HEAD".to_owned()))?;
            self.file_size = Some(size);
            return Ok(size);
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
