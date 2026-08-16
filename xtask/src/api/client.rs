//! Rate-limited API client with backoff (DESIGN.md §4.6).
//!
//! Politeness invariants (ADR-0030 §4): one request per second on a single
//! connection against the shared volunteer-run endpoint; a mainstream
//! browser UA; exponential backoff with jitter on 429/5xx capped at ~5
//! minutes and 6 attempts, after which the failure is returned for the
//! caller to ledger — never a process abort.

use std::time::Duration;

use tokio::time::Instant;

use crate::api::model::{
    ContentListing, EnvelopeError, LatestFileRecord, normalize_dir, parse_envelope,
    parse_latest_envelope,
};
use crate::cache::ApiCache;

/// The one API endpoint (DESIGN.md §4.1).
pub const API_URL: &str = "https://www.doomworld.com/idgames/api/api.php";

/// Mainstream browser UA (§4.6): politeness is enforced by the request
/// rate, not the UA; an identifying tool UA risks incidental anti-bot
/// layers (the docs page already blocks them).
pub const BROWSER_UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) \
    AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";

const MAX_ATTEMPTS: u32 = 6;
const RATE_INTERVAL: Duration = Duration::from_secs(1);

/// Cap on an API response body (ADR-0016 posture: network bytes are
/// untrusted everywhere, not just on mirrors). The largest spike-observed
/// listing is ~60 KB; 8 MiB is orders-of-magnitude headroom.
const API_BODY_CAP: usize = 8 * 1024 * 1024;

/// Interval gate: at most one `wait()` completion per interval.
#[derive(Debug)]
pub(crate) struct RateGate {
    interval: Duration,
    last: Option<Instant>,
}

impl RateGate {
    pub(crate) fn new(interval: Duration) -> Self {
        Self {
            interval,
            last: None,
        }
    }

    /// Await until a full interval has passed since the previous request
    /// *started*, then stamp the new request start — start-to-start
    /// spacing, which is what "one request per second" means.
    pub(crate) async fn wait(&mut self) {
        if let Some(last) = self.last {
            let due = last + self.interval;
            let now = Instant::now();
            if due > now {
                tokio::time::sleep_until(due).await;
            }
        }
        self.last = Some(Instant::now());
    }
}

/// Live/cached call counters for the run manifest.
#[derive(Debug, Default, Clone, Copy)]
pub struct ClientStats {
    /// HTTP requests actually sent to the API.
    pub live_calls: u64,
    /// `getcontents` calls answered from the disk cache.
    pub cache_hits: u64,
}

/// A `getcontents` result plus its cache provenance.
#[derive(Debug)]
pub struct FetchOutcome {
    /// The parsed listing.
    pub listing: ContentListing,
    /// Whether the disk cache answered without a network request. Per-call
    /// provenance; no caller reads this today (the run manifest sources its
    /// cache/live breakdown from `ClientStats` instead) — flagged as a
    /// vestigial-candidate in the #405 final-fix report.
    #[allow(dead_code)]
    pub from_cache: bool,
    /// On a live refetch of a previously-cached path: whether the scrubbed
    /// body hash moved (§4.5 change detection for phase 2). `None` on
    /// cache hits and first fetches.
    pub changed: Option<bool>,
}

/// Terminal failure of one API call, after retries.
#[derive(Debug)]
pub enum ApiCallError {
    /// The API's own error envelope (e.g. missing argument).
    Api {
        /// The API's error class.
        fault_kind: String,
        /// The API's message.
        message: String,
    },
    /// HTTP-level failure that survived the retry policy.
    Http {
        /// Attempts made (== 6 unless non-retryable).
        attempts: u32,
        /// Last status or transport error, for the ledger.
        detail: String,
    },
    /// The response parsed as JSON but matched no known envelope.
    Shape(String),
}

impl std::fmt::Display for ApiCallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiCallError::Api {
                fault_kind,
                message,
            } => {
                write!(f, "API fault ({fault_kind}): {message}")
            }
            ApiCallError::Http { attempts, detail } => {
                write!(f, "HTTP failure after {attempts} attempt(s): {detail}")
            }
            ApiCallError::Shape(msg) => write!(f, "unrecognized response: {msg}"),
        }
    }
}

impl std::error::Error for ApiCallError {}

/// Rate-limited, cache-aware Doomworld API client.
pub struct ApiClient {
    http: reqwest::Client,
    cache: ApiCache,
    gate: RateGate,
    rng: fastrand::Rng,
    stats: ClientStats,
    api_version: Option<u64>,
}

impl ApiClient {
    /// Build a client over `cache`.
    ///
    /// # Errors
    /// Fails if the TLS backend cannot initialize.
    pub fn new(cache: ApiCache) -> anyhow::Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(BROWSER_UA)
            .timeout(Duration::from_mins(1))
            .connect_timeout(Duration::from_secs(20))
            .build()?;
        Ok(Self {
            http,
            cache,
            gate: RateGate::new(RATE_INTERVAL),
            rng: fastrand::Rng::new(),
            stats: ClientStats::default(),
            api_version: None,
        })
    }

    /// Run counters so far.
    pub fn stats(&self) -> ClientStats {
        self.stats
    }

    /// `meta.version` from the most recent parsed envelope, if any call
    /// went live this run.
    pub fn observed_api_version(&self) -> Option<u64> {
        self.api_version
    }

    /// The underlying cache (Task 10 invalidates through this).
    pub fn cache(&self) -> &ApiCache {
        &self.cache
    }

    /// `action=getcontents` for one directory, trailing slash enforced
    /// (§4.1), cache-aware (§4.5).
    ///
    /// # Errors
    /// [`ApiCallError`] after the retry policy is exhausted, on an API
    /// fault, or on an unrecognized body. Faults are never cached.
    pub async fn getcontents(&mut self, dir: &str) -> Result<FetchOutcome, ApiCallError> {
        let dir = normalize_dir(dir);
        let prior = self.cache.lookup("getcontents", &dir);
        if let Some(env) = &prior
            && self.cache.is_fresh(env, chrono::Utc::now())
        {
            let (version, listing) = parse_envelope(&env.body).map_err(envelope_to_call_error)?;
            self.note_version(version);
            self.stats.cache_hits += 1;
            return Ok(FetchOutcome {
                listing,
                from_cache: true,
                changed: None,
            });
        }
        let body = self
            .request(&[("action", "getcontents"), ("name", &dir), ("out", "json")])
            .await?;
        // Parse before caching: an API fault or an unrecognized shape must
        // never be persisted (§4.5). Only a body that parses as a listing
        // is stored, and the STORED (scrubbed) bytes are what get parsed
        // again for the return value below, so the live and warm paths
        // always deserialize identical bytes.
        let (version, _) = parse_envelope(&body).map_err(envelope_to_call_error)?;
        let stored = self
            .cache
            .store("getcontents", &dir, version, body)
            .map_err(|e| ApiCallError::Shape(format!("cache store: {e}")))?;
        let changed = prior.map(|p| p.body_hash != stored.body_hash);
        let (version, listing) = parse_envelope(&stored.body).map_err(envelope_to_call_error)?;
        self.note_version(version);
        Ok(FetchOutcome {
            listing,
            from_cache: false,
            changed,
        })
    }

    /// `action=latestfiles&limit=N`. Never cached (§4.5 — it *is* the
    /// freshness check). Returns the abbreviated [`LatestFileRecord`] shape
    /// the live API actually sends for this action — only `id` is
    /// load-bearing there (§4.5's max-id probe).
    ///
    /// # Errors
    /// As for [`Self::getcontents`].
    pub async fn latestfiles(&mut self, limit: u32) -> Result<Vec<LatestFileRecord>, ApiCallError> {
        let limit = limit.to_string();
        let body = self
            .request(&[
                ("action", "latestfiles"),
                ("limit", &limit),
                ("out", "json"),
            ])
            .await?;
        let (version, records) = parse_latest_envelope(&body).map_err(envelope_to_call_error)?;
        self.note_version(version);
        Ok(records)
    }

    fn note_version(&mut self, version: u64) {
        if version != 0 {
            if version != 3 {
                tracing::warn!(version, "API version is not the spike-verified 3");
            }
            self.api_version = Some(version);
        }
    }

    /// One rate-gated request with the §4.6 retry policy. Returns the
    /// parsed JSON body.
    async fn request(&mut self, query: &[(&str, &str)]) -> Result<serde_json::Value, ApiCallError> {
        let mut attempt = 0_u32;
        loop {
            attempt += 1;
            self.gate.wait().await;
            self.stats.live_calls += 1;
            let outcome = self.http.get(API_URL).query(query).send().await;
            let (status, detail) = match outcome {
                Ok(mut resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        // Bounded chunked read instead of `resp.json()`:
                        // an abusive body must fail cleanly, not buffer
                        // without limit (same posture as the mirror path).
                        let mut bytes: Vec<u8> = Vec::new();
                        loop {
                            match resp.chunk().await {
                                Ok(Some(chunk)) => {
                                    if bytes.len() + chunk.len() > API_BODY_CAP {
                                        return Err(ApiCallError::Shape(format!(
                                            "response body exceeded {API_BODY_CAP} bytes"
                                        )));
                                    }
                                    bytes.extend_from_slice(&chunk);
                                }
                                Ok(None) => break,
                                Err(e) => {
                                    return Err(ApiCallError::Shape(format!("body: {e}")));
                                }
                            }
                        }
                        return serde_json::from_slice(&bytes)
                            .map_err(|e| ApiCallError::Shape(format!("body: {e}")));
                    }
                    (Some(status), format!("HTTP {status}"))
                }
                Err(e) => (None, format!("transport: {e}")),
            };
            if attempt >= MAX_ATTEMPTS || !is_retryable(status) {
                return Err(ApiCallError::Http {
                    attempts: attempt,
                    detail,
                });
            }
            let delay = backoff_delay(attempt, &mut self.rng);
            tracing::warn!(%detail, attempt, delay_secs = delay.as_secs(), "retrying");
            tokio::time::sleep(delay).await;
        }
    }
}

fn envelope_to_call_error(e: EnvelopeError) -> ApiCallError {
    match e {
        EnvelopeError::Api(f) => ApiCallError::Api {
            fault_kind: f.kind,
            message: f.message,
        },
        EnvelopeError::Shape(s) => ApiCallError::Shape(s),
    }
}

/// §4.6: retry 429 and 5xx (and transport errors, `None`); nothing else.
pub(crate) fn is_retryable(status: Option<reqwest::StatusCode>) -> bool {
    match status {
        None => true,
        Some(s) => s == reqwest::StatusCode::TOO_MANY_REQUESTS || s.is_server_error(),
    }
}

/// Delay before retry `attempt` (1-based): nominal `min(5·3^(attempt−1),
/// 300)` seconds, jittered uniformly into `[nominal/2, nominal]`.
pub(crate) fn backoff_delay(attempt: u32, rng: &mut fastrand::Rng) -> Duration {
    let exponent = i32::try_from(attempt.saturating_sub(1).min(8)).unwrap_or(8);
    let nominal = (5.0_f64 * 3.0_f64.powi(exponent)).min(300.0);
    let secs = rng.f64().mul_add(nominal / 2.0, nominal / 2.0);
    Duration::from_secs_f64(secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_schedule_grows_and_caps() {
        let mut rng = fastrand::Rng::with_seed(42);
        let mut prev_max = 0.0_f64;
        for attempt in 1..=5 {
            let d = backoff_delay(attempt, &mut rng).as_secs_f64();
            let exponent = i32::try_from(attempt - 1).unwrap();
            let nominal = (5.0 * 3.0_f64.powi(exponent)).min(300.0);
            assert!(
                d >= nominal / 2.0 - f64::EPSILON,
                "attempt {attempt}: {d} too small"
            );
            assert!(
                d <= nominal + f64::EPSILON,
                "attempt {attempt}: {d} exceeds nominal"
            );
            assert!(
                d <= 300.0 + f64::EPSILON,
                "attempt {attempt}: {d} exceeds the 5-minute cap"
            );
            prev_max = prev_max.max(d);
        }
        // Different seeds jitter differently.
        let a = backoff_delay(3, &mut fastrand::Rng::with_seed(1));
        let b = backoff_delay(3, &mut fastrand::Rng::with_seed(2));
        assert_ne!(a, b);
    }

    #[test]
    fn retryability_classification() {
        use reqwest::StatusCode;
        assert!(is_retryable(Some(StatusCode::TOO_MANY_REQUESTS)));
        assert!(is_retryable(Some(StatusCode::INTERNAL_SERVER_ERROR)));
        assert!(is_retryable(Some(StatusCode::BAD_GATEWAY)));
        assert!(is_retryable(None)); // transport error
        assert!(!is_retryable(Some(StatusCode::NOT_FOUND)));
        assert!(!is_retryable(Some(StatusCode::BAD_REQUEST)));
        assert!(!is_retryable(Some(StatusCode::OK)));
    }

    #[tokio::test(start_paused = true)]
    async fn rate_gate_spaces_requests_by_one_second() {
        let mut gate = RateGate::new(std::time::Duration::from_secs(1));
        let t0 = tokio::time::Instant::now();
        gate.wait().await; // first request: immediate
        assert_eq!(t0.elapsed(), std::time::Duration::ZERO);
        gate.wait().await; // second: gated to +1s
        assert!(t0.elapsed() >= std::time::Duration::from_secs(1));
        gate.wait().await;
        assert!(t0.elapsed() >= std::time::Duration::from_secs(2));
    }

    #[tokio::test(start_paused = true)]
    async fn rate_gate_does_not_stack_when_idle() {
        let mut gate = RateGate::new(std::time::Duration::from_secs(1));
        gate.wait().await;
        tokio::time::advance(std::time::Duration::from_secs(5)).await;
        let before = tokio::time::Instant::now();
        gate.wait().await; // already past the interval: immediate
        assert_eq!(before.elapsed(), std::time::Duration::ZERO);
    }

    #[test]
    fn browser_ua_looks_mainstream() {
        assert!(BROWSER_UA.starts_with("Mozilla/5.0"));
        assert!(!BROWSER_UA.to_ascii_lowercase().contains("xtask"));
        assert!(!BROWSER_UA.to_ascii_lowercase().contains("crustywad"));
    }
}

#[cfg(test)]
mod network_tests {
    use super::*;

    fn network_enabled() -> bool {
        if std::env::var_os("XTASK_NETWORK_TESTS").is_none() {
            eprintln!("skipping: set XTASK_NETWORK_TESTS=1 to run network tests");
            return false;
        }
        true
    }

    fn client() -> ApiClient {
        let dir = tempfile::tempdir().unwrap().keep();
        ApiClient::new(crate::cache::ApiCache::new(dir, chrono::Duration::days(7)).unwrap())
            .unwrap()
    }

    #[tokio::test]
    async fn getcontents_small_known_directory() {
        if !network_enabled() {
            return;
        }
        let mut c = client();
        let out = c.getcontents("levels/doom/0-9/").await.unwrap();
        assert!(!out.from_cache);
        let (files, _dirs) = out.listing.into_parts();
        assert!(!files.is_empty());
        assert!(files.iter().all(|f| f.size > 0));
        assert!(files.iter().all(|f| f.dir.ends_with('/')));
        assert_eq!(c.observed_api_version(), Some(3));

        // Second call: served from cache, no live request.
        let live_before = c.stats().live_calls;
        let out2 = c.getcontents("levels/doom/0-9/").await.unwrap();
        assert!(out2.from_cache);
        assert_eq!(c.stats().live_calls, live_before);
    }

    #[tokio::test]
    async fn latestfiles_probe_returns_current_max() {
        if !network_enabled() {
            return;
        }
        let mut c = client();
        let files = c.latestfiles(1).await.unwrap();
        assert_eq!(files.len(), 1);
        // Spike observed id 22083 on 2026-08-12; ids are monotonic.
        assert!(files[0].id >= 22_083);
    }
}
