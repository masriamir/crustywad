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
//! the cost this type exists to avoid. A single URL also has no failover
//! partner to retry against, so [`UrlRanges::fetch`] is a single attempt
//! with response classification, not `MirrorRanges`'s per-mirror
//! retry/failover loop.
//!
//! Redirects: the phase-2 client
//! ([`crate::mirror::build_zips_http`]) never overrides `reqwest`'s default
//! redirect policy (up to 10 hops), and this type doesn't either — that
//! default is what resolves a ModDB/itch-style download-redirect chain to
//! the real file, with no special-casing needed here.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use crate::zips::inspect::{FetchFailure, RangeSource};
use crate::zips::range_reader::TransferCounters;

/// One arbitrary URL's byte source (§6.4): no mirror pool, no failover, no
/// full-download fallback (spec §2.2) — just ranged GETs against one host.
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
        }
    }

    /// `HEAD` the URL (following the client's redirect policy) and record
    /// its `Content-Length` as the known file size for later
    /// [`RangeSource::fetch`] calls to recognize a whole-file request.
    ///
    /// # Errors
    /// Transport failure, or a response with no `Content-Length` header —
    /// both surface as [`FetchFailure::Http`]. A HEAD's status is never run
    /// through [`classify_range_response`] — `RangeUnsupported`/`NotFound`
    /// are `fetch`-only classifications.
    // consumed from Task 5 (#407)
    #[allow(dead_code)]
    pub async fn discover_size(&mut self) -> Result<u64, FetchFailure> {
        let resp = self
            .http
            .head(self.url.clone())
            .send()
            .await
            .map_err(|e| FetchFailure::Http(format!("HEAD transport: {e}")))?;
        let size = resp
            .content_length()
            .ok_or_else(|| FetchFailure::Http("no Content-Length on HEAD".to_owned()))?;
        self.file_size = Some(size);
        Ok(size)
    }
}

impl RangeSource for UrlRanges {
    /// Single-attempt ranged fetch — no retry/failover loop, unlike
    /// [`crate::zips::range_reader::MirrorRanges::fetch`]: a lone URL has no
    /// candidate to fail over to, so a transport error or an unretryable
    /// status is reported straight through rather than retried in place
    /// (retries/backoff for a single flaky host are `MirrorRanges`
    /// machinery this type deliberately doesn't duplicate — see the module
    /// doc).
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

        self.counters.requests.fetch_add(1, Ordering::Relaxed);
        let resp = self
            .http
            .get(self.url.clone())
            .header(reqwest::header::RANGE, format!("bytes={offset}-{want_end}"))
            .send()
            .await
            .map_err(|e| FetchFailure::Http(format!("transport: {e}")))?;

        let status = resp.status().as_u16();
        match classify_range_response(status, whole_file) {
            RangeOutcome::UsePartial | RangeOutcome::UseFullBody => {
                read_capped_body(resp, len, &self.counters).await
            }
            RangeOutcome::RangeUnsupported => Err(FetchFailure::RangeUnsupported),
            RangeOutcome::NotFound => Err(FetchFailure::NotFound),
            RangeOutcome::Http => Err(FetchFailure::Http(format!("HTTP {status}"))),
        }
    }
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
    /// transport-ish detail for the caller to report.
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
}
