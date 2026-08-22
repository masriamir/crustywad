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
    /// transport error or a retryable status per [`should_retry`]), then
    /// fall back to a single-byte ranged-GET size probe
    /// ([`Self::discover_size_via_range_probe`]) in **two** cases: the HEAD
    /// succeeded but carried no usable `Content-Length`, or the HEAD failed
    /// outright with anything other than a `404`. That second case matters
    /// on its own: a host that blocks `HEAD` (`403`/`405`, a WAF rule, a
    /// CDN edge that simply doesn't implement the method) but serves
    /// ranged `GET`s just fine would otherwise be ledgered a dishonest
    /// `fetch_error` and never analyzed at all, even though the archive is
    /// perfectly reachable — the fallback is this method's only chance to
    /// find that out. A `404`, in contrast, is authoritative on either
    /// path (the resource is absent, full stop) and returns immediately
    /// without ever trying the other path. Either way, the learned size is
    /// recorded for later [`RangeSource::fetch`] calls to recognize a
    /// whole-file request.
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
    /// - A `404` on the HEAD → [`FetchFailure::NotFound`] immediately,
    ///   never falling through to the range-probe fallback: a `404` is
    ///   already the authoritative answer.
    /// - Any other HEAD failure (a non-retryable status after retries are
    ///   exhausted, or a transport failure) → the fallback
    ///   [`Self::discover_size_via_range_probe`] is tried instead, and
    ///   **its** error — not the HEAD's — is what this method returns if
    ///   the fallback also fails. The probe's classification is the more
    ///   truthful one: e.g. a host that blocks HEAD but ignores ranges on
    ///   GET ends as [`FetchFailure::RangeUnsupported`] (→ the caller's
    ///   honest `no_range_support`), not the HEAD failure's generic
    ///   `Http` (→ a misleading `fetch_error`) — see
    ///   [`Self::discover_size_via_range_probe`]'s own `# Errors` for the
    ///   full set of shapes it can return.
    /// - A HEAD success with no usable `Content-Length` behaves
    ///   identically to a HEAD failure (other than `404`): the range-probe
    ///   fallback runs, and its error (if any) is what's returned.
    pub async fn discover_size(&mut self) -> Result<u64, FetchFailure> {
        // The actual branch is delegated to `size_discovery_step` — a pure
        // function over `head_content_length`'s result — rather than
        // matched inline, so the decision this doc comment describes is
        // the one under unit test, not a hand-kept parallel copy of it.
        let head_result = self.head_content_length().await;
        let size = match size_discovery_step(&head_result) {
            SizeDiscoveryStep::UseSize(size) => size,
            // Only ever produced from `Err(FetchFailure::NotFound)` — see
            // `size_discovery_step` — so this is exact, not a lossy stand-in.
            SizeDiscoveryStep::Terminal => return Err(FetchFailure::NotFound),
            SizeDiscoveryStep::Fallback => self.discover_size_via_range_probe().await?,
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
    /// A `206` is not trusted on status alone: its `Content-Range` header
    /// is validated via [`validate_content_range`] against the range
    /// actually requested (and, when known, this type's discovered file
    /// size) before the body is ever read — the same guard
    /// `MirrorRanges::fetch`'s `accept_partial_content` applies (the one
    /// that caught 1,099 stale-size phase-2 entries). A mismatch — the
    /// wrong extent, a disagreeing numeric total, or a missing/malformed
    /// header — is terminal immediately, not retried: unlike a mirror
    /// pool, a single URL has no different host to fail over to, so
    /// retrying against the same misbehaving proxy/CDN would likely just
    /// reproduce the same wrong answer. Trusting an unvalidated `206`
    /// would risk splicing a proxy/CDN's *different* range into
    /// `inspect_zip`'s sparse buffer at the wrong offset.
    ///
    /// `whole_file` is `offset == 0 && len == file_size` — see
    /// [`is_whole_file_request`] — but only once [`Self::discover_size`]
    /// has actually learned `file_size`; before that (or if it was never
    /// called) it's unconditionally `false` — an unknown file size can
    /// never be confirmed to match a "whole file" request, so treating it
    /// as partial is the only sound default. The comparison is exact
    /// (`==`, not `>=`): the `200` path below reads the body through
    /// [`read_capped_body`], which *errors* on a body shorter than `len`,
    /// so a `len` merely tolerated as "close enough" (e.g. `len >
    /// file_size`) would make a perfectly legitimate range-ignoring `200`
    /// — which only ever delivers `file_size` bytes — fail as a short
    /// body; a `>=` "tolerance" here was self-defeating, not generous.
    /// This matches
    /// [`crate::zips::range_reader::MirrorRanges::fetch`]'s own
    /// `offset == 0 && len == self.expected_file_size` check, and costs
    /// nothing in practice: the only realistic whole-file caller
    /// (`inspect_zip`'s small-file rule) always requests exactly
    /// `(0, file_size)`.
    async fn fetch(&mut self, offset: u64, len: u64) -> Result<Vec<u8>, FetchFailure> {
        let whole_file = is_whole_file_request(offset, len, self.file_size);
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
                RangeOutcome::UsePartial => {
                    // Read (and drop) the header text before `resp` moves
                    // into `read_capped_body` below — the borrow must not
                    // outlive that move.
                    let content_range = resp
                        .headers()
                        .get(reqwest::header::CONTENT_RANGE)
                        .and_then(|v| v.to_str().ok())
                        .map(str::to_owned);
                    if let Err(detail) = validate_content_range(
                        content_range.as_deref(),
                        offset,
                        want_end,
                        self.file_size,
                    ) {
                        return Err(FetchFailure::Http(detail));
                    }
                    return read_capped_body(resp, len, &self.counters).await;
                }
                RangeOutcome::UseFullBody => {
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

/// Stream `resp`'s body, counting every byte actually *read* into
/// `counters.bytes` — not every byte the server sends. The loop reads
/// whole chunks (`resp.chunk()` never hands back a partial one), and every
/// chunk it reads is counted in full before the cap is applied: if the
/// final accepted chunk pushes the buffer past `len`, its excess bytes
/// were still read off the wire, so they still count, even though only
/// the first `remaining` of them are kept in the returned buffer (the rest
/// are truncated away, matching `range_reader`'s accounting posture of
/// counting what moved over the wire). The loop condition
/// (`while bytes.len() < cap`) then stops *before* requesting the next
/// chunk once the cap is reached, so anything the server would have sent
/// beyond that accepted chunk is neither read nor counted — a lying or
/// misbehaving host can push at most one chunk past `len` through this
/// method, never an unbounded amount, and can't OOM this process. This
/// truncates rather than erroring on an over-long body (unlike
/// `MirrorRanges::fetch`'s hard cap-exceeded error — a lone URL with no
/// failover partner gets the body it asked for and nothing more, rather
/// than failing an entry outright over a chatty extra byte or two).
/// Never retried, matching `ApiClient::request`'s posture toward a body
/// that fails mid-stream (§4.6): a body-stage failure is exchange-level,
/// not a fresh-request-worthy transient, so [`UrlRanges::fetch`]'s retry
/// loop never re-enters once a response has started streaming.
///
/// A body that ends up short of `len` (host closed early, or genuinely
/// under-delivered) is reported as [`FetchFailure::Http`] naming both
/// sides of the mismatch — accepting a short body would silently
/// under-report an outlier's declared central-directory/member sizes.
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RangeOutcome {
    /// `206` — validate its `Content-Range` via [`validate_content_range`]
    /// before reading the body, capped at the requested length.
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
pub(crate) fn classify_range_response(status: u16, whole_file: bool) -> RangeOutcome {
    match status {
        206 => RangeOutcome::UsePartial,
        200 if whole_file => RangeOutcome::UseFullBody,
        200 => RangeOutcome::RangeUnsupported,
        404 => RangeOutcome::NotFound,
        _ => RangeOutcome::Http,
    }
}

/// Whether a ranged request `(offset, len)` covers the entire file, given
/// the file's known size (`None` before [`UrlRanges::discover_size`] has
/// run — always `false` in that case, since an unknown size can never be
/// confirmed to match). Exact match only (`offset == 0 && len ==
/// file_size`), matching
/// [`crate::zips::range_reader::MirrorRanges::fetch`]'s own check: see
/// [`UrlRanges::fetch`]'s doc comment for why `>=` would be
/// self-defeating rather than tolerant here.
pub(crate) fn is_whole_file_request(offset: u64, len: u64, file_size: Option<u64>) -> bool {
    file_size.is_some_and(|file_size| offset == 0 && len == file_size)
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

/// Parse a `Content-Range: bytes START-END/TOTAL` header value into its
/// three parts. `TOTAL` is `None` for the RFC 7233 §4.2 "length unknown"
/// form (`bytes START-END/*`). The whole result is `None` for:
/// - the "unsatisfied range" form (`bytes */TOTAL` — no start/end at all);
/// - a `START-END` part that isn't literally two `u64`s separated by `-`
///   (e.g. `bytes garbage/123`), or whose `START` exceeds its `END` (e.g.
///   `bytes 5-2/123`) — this validates the whole header shape, not just
///   the `TOTAL` it returns, precisely so a misbehaving host can't feed a
///   size through a malformed range part;
/// - anything else that doesn't parse as `bytes <range>/<total>` at all,
///   or whose `TOTAL` digits don't fit a `u64`.
///
/// This is the shared low-level parse both [`parse_content_range_total`]
/// (the size probe's `bytes=0-0`-only caller) and
/// [`validate_content_range`] (`fetch`'s arbitrary-requested-range
/// validator) build on — it doesn't itself know which range was requested
/// or what `TOTAL` is expected; that's each caller's own job.
fn parse_content_range_header(value: &str) -> Option<(u64, u64, Option<u64>)> {
    let (unit, range) = value.split_once(' ')?;
    if !unit.eq_ignore_ascii_case("bytes") {
        return None;
    }
    let (range, total) = range.split_once('/')?;
    if range.trim() == "*" {
        return None;
    }
    let (start, end) = range.split_once('-')?;
    let start: u64 = start.trim().parse().ok()?;
    let end: u64 = end.trim().parse().ok()?;
    if start > end {
        return None;
    }
    let total = match total.trim() {
        "*" => None,
        n => Some(n.parse().ok()?),
    };
    Some((start, end, total))
}

/// Extract `TOTAL` from a `Content-Range` header value, as sent on a `206`
/// to [`UrlRanges::discover_size_via_range_probe`]'s `bytes=0-0` probe.
/// This parser exists *solely* for that one caller, so beyond everything
/// [`parse_content_range_header`] itself rejects, it additionally requires
/// `START == 0 && END == 0` — the exact (and only) range the probe ever
/// requests. A `206` answering a *different* range through this path would
/// otherwise let a misbehaving host attach an unrelated `TOTAL` to a range
/// nobody asked for; this is deliberately narrower than `fetch`'s
/// general-purpose [`validate_content_range`], which checks the header
/// against whatever range `fetch` actually requested rather than a single
/// hardcoded one. `None` also for a `TOTAL` of exactly `0` — same
/// invariant [`classify_head_response`] enforces on the HEAD path ("0 is
/// never passed onward as a real size"): a zero-length remote zip is bogus
/// input regardless of which path reported it, so this is the
/// Content-Range-side half of that same guarantee, not a second,
/// independent decision to keep in sync by hand. Filtered here, in the
/// pure parser, rather than at the call site: every caller inherits the
/// "0 means unusable" contract for free, with no risk of a future call
/// site forgetting to filter it out itself.
///
/// This function doesn't need to distinguish *why* a total wasn't
/// recoverable, only whether one was — every unusable shape is treated
/// identically by the caller (surfaced as a `FetchFailure::Http` detail).
pub(crate) fn parse_content_range_total(value: &str) -> Option<u64> {
    let (start, end, total) = parse_content_range_header(value)?;
    if start != 0 || end != 0 {
        return None;
    }
    total.filter(|&n| n != 0)
}

/// Validate a `206`'s `Content-Range` header against the range
/// [`UrlRanges::fetch`] actually requested (`[offset, want_end]`) and,
/// when known, the file's declared total size — before the body is
/// trusted at all. Mirrors
/// [`crate::zips::range_reader::MirrorRanges::fetch`]'s own
/// `accept_partial_content` validation (`range_reader.rs`), the guard that
/// caught 1,099 stale-size phase-2 entries: a proxy/CDN answering `206`
/// for a *different* range must never have its bytes spliced into
/// `inspect_zip`'s sparse buffer at the wrong offset, and a `206` whose
/// declared total disagrees with a size this type already discovered is
/// equally untrustworthy.
///
/// `header_value` is `None` for both a missing `Content-Range` and one
/// that isn't valid UTF-8 (the caller collapses both before calling this)
/// — either way a mismatch: RFC 7233 §4.2 requires this header on every
/// `206`, so its absence is itself suspicious, never a pass.
///
/// `expected_total` is `self.file_size` (`None` before
/// [`UrlRanges::discover_size`] has run). The simplest honest rule: the
/// header's own `TOTAL` is always accepted when it's the RFC 7233 §4.2
/// "length unknown" `*` form (legal on its own terms, and there is
/// nothing to compare it to either way) or whenever `expected_total` is
/// itself `None` (nothing known yet to compare against); a *numeric*
/// header total is required to equal `expected_total` only when the
/// latter is actually known.
///
/// # Errors
/// A detail string naming got-vs-wanted, matching the phase-2 ledger's
/// `"Content-Range mismatch: got X, wanted Y"` shape.
pub(crate) fn validate_content_range(
    header_value: Option<&str>,
    offset: u64,
    want_end: u64,
    expected_total: Option<u64>,
) -> Result<(), String> {
    let Some((start, end, total)) = header_value.and_then(parse_content_range_header) else {
        return Err("206 with a missing, unparseable, or malformed Content-Range".to_owned());
    };
    if start != offset || end != want_end {
        return Err(format!(
            "Content-Range mismatch: got {start}-{end}, wanted {offset}-{want_end}"
        ));
    }
    if let (Some(total), Some(expected_total)) = (total, expected_total)
        && total != expected_total
    {
        return Err(format!(
            "Content-Range mismatch: got total {total}, wanted {expected_total}"
        ));
    }
    Ok(())
}

/// What [`UrlRanges::discover_size`] should do next, given
/// [`UrlRanges::head_content_length`]'s result. Pure:
/// [`UrlRanges::discover_size`] delegates its actual branching to
/// [`size_discovery_step`] rather than matching inline, so this is the
/// real decision under test, not a hand-kept parallel copy of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SizeDiscoveryStep {
    /// The HEAD alone settled it — use this declared size directly.
    UseSize(u64),
    /// Try the range-probe fallback next: either the HEAD succeeded with
    /// no usable `Content-Length`, or it failed with something other than
    /// `404` (not authoritative — see [`UrlRanges::discover_size`]'s doc
    /// comment for why a blocked/disabled HEAD shouldn't end the attempt).
    Fallback,
    /// A `404` on the HEAD — authoritative, [`UrlRanges::discover_size`]
    /// returns immediately without ever trying the fallback.
    Terminal,
}

/// Pure classifier: `head_content_length`'s `Result` → what
/// [`UrlRanges::discover_size`] should do next. See [`SizeDiscoveryStep`]
/// for the case-by-case rationale. Only `Err(FetchFailure::NotFound)` ever
/// produces [`SizeDiscoveryStep::Terminal`] — every other `Err` (a
/// non-retryable HEAD status after retries, or a transport failure) is
/// [`SizeDiscoveryStep::Fallback`], since a `404` is the only HEAD outcome
/// this type treats as authoritative about the resource itself.
pub(crate) fn size_discovery_step(
    head_result: &Result<Option<u64>, FetchFailure>,
) -> SizeDiscoveryStep {
    match head_result {
        Ok(Some(size)) => SizeDiscoveryStep::UseSize(*size),
        Err(FetchFailure::NotFound) => SizeDiscoveryStep::Terminal,
        // A success HEAD with no usable Content-Length, or any other HEAD
        // failure (a non-404 status, a transport error) — neither is
        // authoritative the way a 404 is, so both fall back to the probe.
        Ok(None) | Err(_) => SizeDiscoveryStep::Fallback,
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
    fn whole_file_request_requires_exact_length_match() {
        // Before discovery (`file_size = None`): unconditionally false,
        // no matter what offset/len say.
        assert!(!is_whole_file_request(0, 100, None));
        // Exact match: offset 0, len == file_size.
        assert!(is_whole_file_request(0, 100, Some(100)));
        // Nonzero offset is never whole-file, even with a matching length.
        assert!(!is_whole_file_request(1, 99, Some(100)));
        // A genuine partial request (len < file_size) is not whole-file.
        assert!(!is_whole_file_request(0, 50, Some(100)));
        // len > file_size must NOT be treated as whole-file: a 200 to this
        // request only ever delivers `file_size` bytes, and
        // `read_capped_body(resp, len, ..)` errors on a body shorter than
        // `len` — treating this as whole-file (the old `>=` behavior)
        // would misreport a legitimate range-ignoring 200 as a short body.
        assert!(!is_whole_file_request(0, 150, Some(100)));
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
        // A malformed START-END part must not let a misbehaving host feed
        // a size through the total anyway (fix round 4: this used to
        // return `Some(123)`, trusting the total while never validating
        // the range part the doc comment claimed was checked).
        assert_eq!(parse_content_range_total("bytes garbage/123"), None);
        // Inverted START > END is not a real range either.
        assert_eq!(parse_content_range_total("bytes 5-2/123"), None);
        // This parser exists solely for the probe's `bytes=0-0` request —
        // any other (even internally valid, satisfiable) START-END must be
        // rejected, not just malformed/inverted ones, since a 206
        // answering a different range through this path could otherwise
        // attach an unrelated TOTAL to a range nobody asked for.
        assert_eq!(parse_content_range_total("bytes 5-5/123"), None);
    }

    #[test]
    fn content_range_validation_against_requested_range() {
        // Exact match, no known total to compare against: ok.
        assert_eq!(
            validate_content_range(Some("bytes 10-19/100"), 10, 19, None),
            Ok(())
        );
        // Exact match, and the numeric total agrees with the known size: ok.
        assert_eq!(
            validate_content_range(Some("bytes 10-19/100"), 10, 19, Some(100)),
            Ok(())
        );
        // "*" (length unknown) total is always accepted — RFC-legal on its
        // own terms — whether or not the file size is already known.
        assert_eq!(
            validate_content_range(Some("bytes 10-19/*"), 10, 19, None),
            Ok(())
        );
        assert_eq!(
            validate_content_range(Some("bytes 10-19/*"), 10, 19, Some(100)),
            Ok(())
        );
        // Wrong start.
        assert!(validate_content_range(Some("bytes 0-19/100"), 10, 19, None).is_err());
        // Wrong end.
        assert!(validate_content_range(Some("bytes 10-18/100"), 10, 19, None).is_err());
        // Numeric total disagrees with the already-known file size — the
        // MirrorRanges-parity check this fix adds: previously any 206 was
        // trusted regardless of its Content-Range.
        let err = validate_content_range(Some("bytes 10-19/999"), 10, 19, Some(100)).unwrap_err();
        assert!(err.contains("999"), "detail should name got: {err}");
        assert!(err.contains("100"), "detail should name wanted: {err}");
        // A numeric total is NOT checked when the file size isn't known
        // yet — nothing to compare it to.
        assert_eq!(
            validate_content_range(Some("bytes 10-19/999"), 10, 19, None),
            Ok(())
        );
        // Missing header (a 206 with no Content-Range at all is malformed).
        assert!(validate_content_range(None, 10, 19, None).is_err());
        // Garbage header.
        assert!(validate_content_range(Some("nonsense"), 10, 19, None).is_err());
    }

    #[test]
    fn size_discovery_step_from_head_result() {
        use SizeDiscoveryStep::*;
        // A usable HEAD size settles it directly.
        assert_eq!(size_discovery_step(&Ok(Some(42))), UseSize(42));
        // A success HEAD with no usable Content-Length falls back to the
        // range probe.
        assert_eq!(size_discovery_step(&Ok(None)), Fallback);
        // 404 is the only HEAD outcome treated as authoritative — terminal,
        // never falling through to the probe.
        assert_eq!(size_discovery_step(&Err(FetchFailure::NotFound)), Terminal);
        // Every other HEAD failure (a blocked/disabled HEAD method, a
        // transport error the retry policy couldn't recover from, ...)
        // falls back to the range probe instead of ending the attempt —
        // this is the S1 fix: a host that blocks HEAD but serves ranged
        // GETs must not be ledgered a dishonest `fetch_error` without ever
        // trying the fallback.
        assert_eq!(
            size_discovery_step(&Err(FetchFailure::Http("HEAD status 403 Forbidden".into()))),
            Fallback
        );
        assert_eq!(
            size_discovery_step(&Err(FetchFailure::Http("HEAD transport: timeout".into()))),
            Fallback
        );
        // RangeUnsupported never actually comes out of `head_content_length`
        // (it's a `fetch`-only classification), but the classifier's
        // contract is general over any non-NotFound `FetchFailure` —
        // exercise that generality explicitly rather than assuming it.
        assert_eq!(
            size_discovery_step(&Err(FetchFailure::RangeUnsupported)),
            Fallback
        );
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

    /// One request the scripted server observed: method + Range header.
    struct RequestSeen {
        method: String,
        range: Option<String>,
    }

    /// Minimal scripted HTTP/1.1 server (#442): binds a loopback listener,
    /// then serves each canned response to one connection in order —
    /// every response carries `Connection: close`, so reqwest reconnects
    /// per request and no keep-alive framing is involved. Requests here
    /// never carry bodies (HEAD/ranged GET), so reading to the blank line
    /// is a complete request read. The thread is deliberately detached and
    /// never joined: it serves exactly `responses.len()` connections, then
    /// the loop ends and the closure returns, dropping the listener. A
    /// request issued after that point is refused rather than left hanging
    /// — measured locally, it surfaces as a transport-level
    /// `FetchFailure::Http` once the retry ladder is exhausted, in
    /// milliseconds under the paused clock — so a test that outruns its
    /// script fails loudly instead of blocking. Observed requests are
    /// recorded before the response is written, so once the client has a
    /// response, the record is visible — no join needed.
    fn scripted_server(
        responses: Vec<String>,
    ) -> (
        reqwest::Url,
        std::sync::Arc<std::sync::Mutex<Vec<RequestSeen>>>,
    ) {
        use std::io::{Read as _, Write as _};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        let addr = listener.local_addr().expect("local addr");
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let seen_thread = std::sync::Arc::clone(&seen);
        std::thread::spawn(move || {
            for response in responses {
                let Ok((mut conn, _)) = listener.accept() else {
                    return;
                };
                let mut buf = Vec::new();
                let mut byte = [0_u8; 1];
                while !buf.ends_with(b"\r\n\r\n") {
                    match conn.read(&mut byte) {
                        Ok(1) => buf.push(byte[0]),
                        _ => break,
                    }
                }
                let head = String::from_utf8_lossy(&buf);
                let method = head
                    .lines()
                    .next()
                    .and_then(|l| l.split(' ').next())
                    .unwrap_or_default()
                    .to_owned();
                let range = head
                    .lines()
                    .find_map(|l| {
                        l.strip_prefix("range: ")
                            .or_else(|| l.strip_prefix("Range: "))
                    })
                    .map(str::to_owned);
                seen_thread
                    .lock()
                    .expect("seen lock")
                    .push(RequestSeen { method, range });
                let _ = conn.write_all(response.as_bytes());
            }
        });
        let url = reqwest::Url::parse(&format!("http://{addr}/outlier.zip")).expect("url");
        (url, seen)
    }

    /// Canned response builder: status line + headers + optional body,
    /// always `Connection: close`.
    fn canned(status: &str, headers: &[(&str, &str)], body: &[u8]) -> String {
        let mut r = format!("HTTP/1.1 {status}\r\nConnection: close\r\n");
        for (k, v) in headers {
            // Appended piecewise rather than via `push_str(&format!(..))`:
            // same bytes, no throwaway String (clippy::format_push_string).
            r.push_str(k);
            r.push_str(": ");
            r.push_str(v);
            r.push_str("\r\n");
        }
        r.push_str("\r\n");
        r.push_str(&String::from_utf8_lossy(body));
        r
    }

    /// A [`UrlRanges`] pointed at a scripted server, plus the counters it
    /// shares — the live-test analog of the pure classifiers' fixtures.
    fn live_source(url: reqwest::Url) -> (UrlRanges, Arc<TransferCounters>) {
        // A bare client: no timeouts, so a paused tokio clock has no
        // cancel-timers to fire spuriously (#442 test-seam note).
        let counters = Arc::new(TransferCounters::new());
        (
            UrlRanges::new(reqwest::Client::new(), url, Arc::clone(&counters)),
            counters,
        )
    }

    #[tokio::test(start_paused = true)]
    async fn discover_size_uses_the_head_content_length() {
        let (url, seen) = scripted_server(vec![canned(
            "200 OK",
            &[("Content-Length", "2012026")],
            b"",
        )]);
        let (mut source, counters) = live_source(url);
        assert_eq!(source.discover_size().await.unwrap(), 2_012_026);
        let seen = seen.lock().unwrap();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].method, "HEAD");
        assert_eq!(counters.requests.load(Ordering::Relaxed), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn discover_size_falls_back_to_the_range_probe_when_head_has_no_length() {
        let (url, seen) = scripted_server(vec![
            canned("200 OK", &[], b""), // HEAD: success, no Content-Length
            canned(
                "206 Partial Content",
                &[("Content-Range", "bytes 0-0/5555"), ("Content-Length", "1")],
                b"x",
            ),
        ]);
        let (mut source, counters) = live_source(url);
        assert_eq!(source.discover_size().await.unwrap(), 5_555);
        let seen = seen.lock().unwrap();
        assert_eq!(seen.len(), 2);
        assert_eq!(seen[0].method, "HEAD");
        assert_eq!(seen[1].method, "GET");
        assert_eq!(seen[1].range.as_deref(), Some("bytes=0-0"));
        assert_eq!(counters.requests.load(Ordering::Relaxed), 2);
        // The probe's single body byte is counted.
        assert_eq!(counters.bytes.load(Ordering::Relaxed), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn discover_size_head_404_is_terminal_with_no_probe() {
        let (url, seen) = scripted_server(vec![canned("404 Not Found", &[], b"")]);
        let (mut source, counters) = live_source(url);
        assert!(matches!(
            source.discover_size().await,
            Err(FetchFailure::NotFound)
        ));
        assert_eq!(seen.lock().unwrap().len(), 1);
        assert_eq!(counters.requests.load(Ordering::Relaxed), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn discover_size_blocked_head_still_probes_and_succeeds() {
        // The S1 behavior, live: HEAD 403 (non-retryable) must not end the
        // attempt — the ranged probe runs and settles it.
        let (url, seen) = scripted_server(vec![
            canned("403 Forbidden", &[], b""),
            canned(
                "206 Partial Content",
                &[("Content-Range", "bytes 0-0/777"), ("Content-Length", "1")],
                b"x",
            ),
        ]);
        let (mut source, _) = live_source(url);
        assert_eq!(source.discover_size().await.unwrap(), 777);
        assert_eq!(seen.lock().unwrap().len(), 2);
    }

    #[tokio::test(start_paused = true)]
    async fn probe_200_means_no_range_support() {
        let (url, _) = scripted_server(vec![
            canned("200 OK", &[], b""), // HEAD without a length
            canned("200 OK", &[("Content-Length", "4")], b"full"), // probe ignored the range
        ]);
        let (mut source, _) = live_source(url);
        assert!(matches!(
            source.discover_size().await,
            Err(FetchFailure::RangeUnsupported)
        ));
    }

    #[tokio::test(start_paused = true)]
    async fn probe_206_with_garbage_content_range_is_an_http_failure() {
        let (url, _) = scripted_server(vec![
            canned("200 OK", &[], b""),
            canned(
                "206 Partial Content",
                &[("Content-Range", "bytes nonsense"), ("Content-Length", "1")],
                b"x",
            ),
        ]);
        let (mut source, _) = live_source(url);
        match source.discover_size().await {
            Err(FetchFailure::Http(detail)) => {
                assert!(detail.contains("Content-Range"), "{detail}");
            }
            other => panic!("expected Http failure, got {other:?}"),
        }
    }

    #[tokio::test(start_paused = true)]
    async fn head_retries_a_500_then_succeeds() {
        // The retry ladder, live, under the paused clock: attempt 1 gets a
        // retryable 500, the backoff sleep auto-advances, attempt 2 wins.
        let (url, seen) = scripted_server(vec![
            canned("500 Internal Server Error", &[], b""),
            canned("200 OK", &[("Content-Length", "99")], b""),
        ]);
        let (mut source, counters) = live_source(url);
        assert_eq!(source.discover_size().await.unwrap(), 99);
        let seen = seen.lock().unwrap();
        assert_eq!(seen.len(), 2);
        assert!(seen.iter().all(|r| r.method == "HEAD"));
        assert_eq!(counters.requests.load(Ordering::Relaxed), 2);
    }

    #[tokio::test(start_paused = true)]
    async fn fetch_valid_206_returns_the_exact_bytes() {
        let (url, seen) = scripted_server(vec![canned(
            "206 Partial Content",
            &[
                ("Content-Range", "bytes 10-14/100"),
                ("Content-Length", "5"),
            ],
            b"hello",
        )]);
        let (mut source, counters) = live_source(url);
        assert_eq!(source.fetch(10, 5).await.unwrap(), b"hello");
        assert_eq!(
            seen.lock().unwrap()[0].range.as_deref(),
            Some("bytes=10-14")
        );
        assert_eq!(counters.requests.load(Ordering::Relaxed), 1);
        assert_eq!(counters.bytes.load(Ordering::Relaxed), 5);
    }

    #[tokio::test(start_paused = true)]
    async fn fetch_206_answering_the_wrong_range_is_terminal_not_retried() {
        let (url, seen) = scripted_server(vec![canned(
            "206 Partial Content",
            &[("Content-Range", "bytes 0-4/100"), ("Content-Length", "5")],
            b"wrong",
        )]);
        let (mut source, _) = live_source(url);
        match source.fetch(10, 5).await {
            Err(FetchFailure::Http(detail)) => {
                assert!(detail.contains("Content-Range mismatch"), "{detail}");
            }
            other => panic!("expected Http mismatch, got {other:?}"),
        }
        // Terminal on the first attempt: a lying host is not retried.
        assert_eq!(seen.lock().unwrap().len(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn fetch_200_to_a_partial_request_is_range_unsupported() {
        let (url, _) =
            scripted_server(vec![canned("200 OK", &[("Content-Length", "5")], b"whole")]);
        let (mut source, _) = live_source(url);
        assert!(matches!(
            source.fetch(10, 5).await,
            Err(FetchFailure::RangeUnsupported)
        ));
    }

    #[tokio::test(start_paused = true)]
    async fn fetch_200_to_a_whole_file_request_reads_the_full_body() {
        // discover_size first (HEAD), so file_size is known and
        // (0, file_size) classifies as whole-file; then a range-ignoring
        // 200 is a legal full-body answer (MirrorRanges precedent).
        let (url, _) = scripted_server(vec![
            canned("200 OK", &[("Content-Length", "5")], b""),
            canned("200 OK", &[("Content-Length", "5")], b"whole"),
        ]);
        let (mut source, counters) = live_source(url);
        assert_eq!(source.discover_size().await.unwrap(), 5);
        assert_eq!(source.fetch(0, 5).await.unwrap(), b"whole");
        assert_eq!(counters.bytes.load(Ordering::Relaxed), 5);
    }

    #[tokio::test(start_paused = true)]
    async fn fetch_404_is_not_found() {
        let (url, _) = scripted_server(vec![canned("404 Not Found", &[], b"")]);
        let (mut source, _) = live_source(url);
        assert!(matches!(
            source.fetch(0, 5).await,
            Err(FetchFailure::NotFound)
        ));
    }

    #[tokio::test(start_paused = true)]
    async fn fetch_retries_a_502_then_succeeds() {
        let (url, seen) = scripted_server(vec![
            canned("502 Bad Gateway", &[], b""),
            canned(
                "206 Partial Content",
                &[("Content-Range", "bytes 0-2/10"), ("Content-Length", "3")],
                b"abc",
            ),
        ]);
        let (mut source, counters) = live_source(url);
        assert_eq!(source.fetch(0, 3).await.unwrap(), b"abc");
        assert_eq!(seen.lock().unwrap().len(), 2);
        assert_eq!(counters.requests.load(Ordering::Relaxed), 2);
    }

    #[tokio::test(start_paused = true)]
    async fn short_body_is_an_http_failure_naming_both_sides() {
        // Content-Length is set to the *actual* (short) body length, not
        // the 10 bytes `fetch` will ask for: a Content-Length that lies
        // about the server's own body is a distinct transport-level
        // failure hyper surfaces itself (as "body transport: ..."), before
        // read_capped_body's own `bytes.len() < cap` check ever runs. This
        // exercises the genuinely-under-delivered case the module doc
        // describes: a well-formed, honestly-terminated body that is just
        // shorter than what `fetch` requested.
        let (url, _) = scripted_server(vec![canned(
            "206 Partial Content",
            &[("Content-Range", "bytes 0-9/100"), ("Content-Length", "5")],
            b"only4", // 5 bytes < the 10 requested by fetch(0, 10)
        )]);
        let (mut source, _) = live_source(url);
        match source.fetch(0, 10).await {
            Err(FetchFailure::Http(detail)) => {
                assert!(detail.contains("short body"), "{detail}");
                assert!(detail.contains("10"), "{detail}");
            }
            other => panic!("expected short-body Http failure, got {other:?}"),
        }
    }

    #[tokio::test(start_paused = true)]
    async fn overlong_body_is_truncated_to_len_but_counted_as_read() {
        // The server sends 8 bytes against a 5-byte ask in one write:
        // read_capped_body keeps the first 5 and counts what it read off
        // the wire (module doc: truncate, don't error — no failover
        // partner to punish a chatty host with).
        let (url, _) = scripted_server(vec![canned(
            "206 Partial Content",
            &[("Content-Range", "bytes 0-4/100"), ("Content-Length", "8")],
            b"12345678",
        )]);
        let (mut source, counters) = live_source(url);
        assert_eq!(source.fetch(0, 5).await.unwrap(), b"12345");
        // ≥5: the accepted chunk is counted in full, and chunk boundaries
        // are the transport's business — assert the floor, not an exact 8.
        assert!(counters.bytes.load(Ordering::Relaxed) >= 5);
    }
}
