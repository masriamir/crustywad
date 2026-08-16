//! Mirror pool and conditional ls-laR.gz fetch (DESIGN.md §5.0–§5.1).
//!
//! Never fetch from doomworld.com (web frontend, not a file host). Mirror
//! fetches are exempt from the 1 req/s API gate (ADR-0030 §4). At most one
//! ls-laR.gz transfer happens per run; on the warm path the single
//! conditional GET answers 304 and no body moves at all.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::cache::atomic_write;
use crate::lslar::{ArchiveTree, parse_ls_lar_gz};

/// One archive mirror.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct Mirror {
    /// Short name for manifests and logs.
    pub key: &'static str,
    /// Base URL, trailing slash included.
    pub base: &'static str,
}

/// §5.1 verified pool: infania primary (same-day Last-Modified when
/// spiked), gamers.org fallback. Expected to change over the years —
/// update DESIGN §5.1 in the same commit as this constant.
#[allow(dead_code)]
pub const MIRRORS: [Mirror; 2] = [
    Mirror {
        key: "infania",
        base: "https://ftpmirror1.infania.net/pub/idgames/",
    },
    Mirror {
        key: "gamers",
        base: "https://www.gamers.org/pub/idgames/",
    },
];

/// Response size guard: `ls-laR.gz` is ~418 KB; a mirror suddenly serving
/// gigabytes must not OOM the tool (ADR-0016 posture toward untrusted
/// bytes).
const MAX_BODY_BYTES: u64 = 64 * 1024 * 1024;

/// Where this run's tree came from (recorded in the manifest).
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum BootstrapSource {
    /// A mirror served a fresh listing.
    Fresh {
        /// Mirror key.
        mirror: String,
    },
    /// The conditional refetch answered 304; the cached listing is current.
    NotModified {
        /// Mirror key.
        mirror: String,
    },
    /// Every mirror failed but a previously-cached listing exists.
    StaleCache,
    /// No mirror answered and no cache exists — BFS fallback territory.
    Unavailable,
}

impl BootstrapSource {
    /// Manifest label.
    #[allow(dead_code)]
    pub fn label(&self) -> String {
        match self {
            BootstrapSource::Fresh { mirror } => format!("ls-lar-fresh:{mirror}"),
            BootstrapSource::NotModified { mirror } => format!("ls-lar-304:{mirror}"),
            BootstrapSource::StaleCache => "ls-lar-stale-cache".into(),
            BootstrapSource::Unavailable => "unavailable".into(),
        }
    }
}

/// Sidecar metadata for the cached `ls-laR.gz`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct LsLarMeta {
    /// Which mirror served the cached bytes.
    pub mirror: String,
    /// The mirror's `Last-Modified` header, verbatim, if it sent one.
    pub last_modified: Option<String>,
    /// RFC 3339 fetch time (cache metadata, not a harvest output).
    pub fetched_at: String,
}

/// The stored validator to echo as `If-Modified-Since` — only meaningful
/// against the mirror that produced it.
pub(crate) fn if_modified_since(meta: Option<&LsLarMeta>, mirror_key: &str) -> Option<String> {
    meta.filter(|m| m.mirror == mirror_key)
        .and_then(|m| m.last_modified.clone())
}

/// Fetch (or revalidate) the §5.0 bootstrap listing. Infallible by
/// contract: failures degrade through the pool, then the stale cache,
/// then `Unavailable`.
#[allow(dead_code)]
pub async fn fetch_ls_lar(
    http: &reqwest::Client,
    cache_dir: &Path,
) -> (Option<ArchiveTree>, BootstrapSource) {
    let gz_path = cache_dir.join("ls-laR.gz");
    let meta_path = cache_dir.join("ls-laR.meta.json");
    let meta: Option<LsLarMeta> = std::fs::read(&meta_path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok());

    for mirror in &MIRRORS {
        let url = format!("{}ls-laR.gz", mirror.base);
        let mut req = http.get(&url);
        if let Some(ims) = if_modified_since(meta.as_ref(), mirror.key) {
            req = req.header(reqwest::header::IF_MODIFIED_SINCE, ims);
        }
        let resp = match req.send().await {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(mirror = mirror.key, error = %e, "mirror unreachable");
                continue;
            }
        };

        if resp.status() != reqwest::StatusCode::NOT_MODIFIED {
            if let Some(result) = persist_and_parse(resp, mirror, &gz_path, &meta_path).await {
                return result;
            }
            continue;
        }

        // 304: the cache must be readable, or we refetch unconditionally
        // rather than ever surface Unavailable when a clean refetch could
        // succeed.
        if let Some(tree) = read_cached_tree(&gz_path) {
            tracing::info!(mirror = mirror.key, "ls-laR.gz unchanged (304)");
            return (
                Some(tree),
                BootstrapSource::NotModified {
                    mirror: mirror.key.to_owned(),
                },
            );
        }
        tracing::warn!(
            mirror = mirror.key,
            "304 but cache unreadable; refetching unconditionally"
        );
        let _ = std::fs::remove_file(&meta_path);
        match http.get(&url).send().await {
            Ok(resp) => {
                if let Some(result) = persist_and_parse(resp, mirror, &gz_path, &meta_path).await {
                    return result;
                }
            }
            Err(e) => {
                tracing::warn!(mirror = mirror.key, error = %e, "refetch failed");
            }
        }
    }

    match read_cached_tree(&gz_path) {
        Some(tree) => {
            tracing::warn!("all mirrors failed; using STALE cached ls-laR.gz");
            (Some(tree), BootstrapSource::StaleCache)
        }
        None => (None, BootstrapSource::Unavailable),
    }
}

/// Read and parse the cached gz, if present and parseable. `None` covers
/// both "no cache" and "cache is corrupt" — both mean "cannot serve from
/// disk, must fetch".
fn read_cached_tree(gz_path: &Path) -> Option<ArchiveTree> {
    let bytes = std::fs::read(gz_path).ok()?;
    parse_ls_lar_gz(&bytes).ok()
}

/// Persist a 200 response and parse it. `None` means "try the next
/// mirror" (bad status, oversized body, torn download, or parse failure).
async fn persist_and_parse(
    resp: reqwest::Response,
    mirror: &Mirror,
    gz_path: &Path,
    meta_path: &Path,
) -> Option<(Option<ArchiveTree>, BootstrapSource)> {
    let status = resp.status();
    if !status.is_success() {
        tracing::warn!(mirror = mirror.key, %status, "mirror answered non-success");
        return None;
    }
    if resp
        .content_length()
        .is_some_and(|len| len > MAX_BODY_BYTES)
    {
        tracing::warn!(
            mirror = mirror.key,
            "ls-laR.gz implausibly large; skipping mirror"
        );
        return None;
    }
    let last_modified = resp
        .headers()
        .get(reqwest::header::LAST_MODIFIED)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let bytes = match resp.bytes().await {
        Ok(b) if u64::try_from(b.len()).unwrap_or(u64::MAX) <= MAX_BODY_BYTES => b,
        Ok(_) => {
            tracing::warn!(mirror = mirror.key, "ls-laR.gz body exceeded cap");
            return None;
        }
        Err(e) => {
            tracing::warn!(mirror = mirror.key, error = %e, "body read failed");
            return None;
        }
    };
    let tree = match parse_ls_lar_gz(&bytes) {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!(mirror = mirror.key, error = %e, "ls-laR.gz did not parse");
            return None;
        }
    };
    let meta = LsLarMeta {
        mirror: mirror.key.to_owned(),
        last_modified,
        fetched_at: chrono::Utc::now().to_rfc3339(),
    };
    // Best-effort persistence: a cache write failure degrades future runs
    // to refetching, it must not fail this one.
    if let Err(e) = atomic_write(gz_path, &bytes)
        .and_then(|()| atomic_write(meta_path, &serde_json::to_vec(&meta).unwrap_or_default()))
    {
        tracing::warn!(error = %e, "could not persist ls-laR cache");
    }
    tracing::info!(
        mirror = mirror.key,
        dirs = tree.dirs.len(),
        "fetched fresh ls-laR.gz"
    );
    Some((
        Some(tree),
        BootstrapSource::Fresh {
            mirror: mirror.key.to_owned(),
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mirror_pool_matches_design_5_1() {
        assert_eq!(MIRRORS[0].key, "infania");
        assert_eq!(
            MIRRORS[0].base,
            "https://ftpmirror1.infania.net/pub/idgames/"
        );
        assert_eq!(MIRRORS[1].key, "gamers");
        assert_eq!(MIRRORS[1].base, "https://www.gamers.org/pub/idgames/");
        for m in &MIRRORS {
            assert!(m.base.ends_with('/'));
            assert!(
                !m.base.contains("doomworld.com"),
                "never pull binaries from doomworld"
            );
        }
    }

    #[test]
    fn if_modified_since_only_for_the_serving_mirror() {
        let meta = LsLarMeta {
            mirror: "infania".into(),
            last_modified: Some("Wed, 12 Aug 2026 06:00:00 GMT".into()),
            fetched_at: "2026-08-12T06:01:00Z".into(),
        };
        assert_eq!(
            if_modified_since(Some(&meta), "infania").as_deref(),
            Some("Wed, 12 Aug 2026 06:00:00 GMT")
        );
        assert_eq!(if_modified_since(Some(&meta), "gamers"), None);
        assert_eq!(if_modified_since(None, "infania"), None);
        let no_lm = LsLarMeta {
            last_modified: None,
            ..meta
        };
        assert_eq!(if_modified_since(Some(&no_lm), "infania"), None);
    }

    #[test]
    fn meta_roundtrips_through_json() {
        let meta = LsLarMeta {
            mirror: "infania".into(),
            last_modified: Some("Wed, 12 Aug 2026 06:00:00 GMT".into()),
            fetched_at: "2026-08-12T06:01:00Z".into(),
        };
        let json = serde_json::to_string(&meta).unwrap();
        let back: LsLarMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(back.mirror, "infania");
        assert_eq!(
            back.last_modified.as_deref(),
            Some("Wed, 12 Aug 2026 06:00:00 GMT")
        );
    }

    #[test]
    fn bootstrap_source_labels() {
        assert_eq!(
            BootstrapSource::Fresh {
                mirror: "infania".into()
            }
            .label(),
            "ls-lar-fresh:infania"
        );
        assert_eq!(
            BootstrapSource::NotModified {
                mirror: "infania".into()
            }
            .label(),
            "ls-lar-304:infania"
        );
        assert_eq!(BootstrapSource::StaleCache.label(), "ls-lar-stale-cache");
        assert_eq!(BootstrapSource::Unavailable.label(), "unavailable");
    }
}

#[cfg(test)]
mod network_tests {
    use super::*;

    #[tokio::test]
    async fn fetches_and_parses_the_real_tree() {
        if std::env::var_os("XTASK_NETWORK_TESTS").is_none() {
            eprintln!("skipping: set XTASK_NETWORK_TESTS=1 to run network tests");
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let http = reqwest::Client::builder()
            .user_agent(crate::api::client::BROWSER_UA)
            .timeout(std::time::Duration::from_mins(2))
            .build()
            .unwrap();
        let (tree, source) = fetch_ls_lar(&http, dir.path()).await;
        let tree = tree.expect("bootstrap should succeed against live mirrors");
        assert!(matches!(source, BootstrapSource::Fresh { .. }));
        // Spike 2026-08-12: 462 directories, 21,375 zips. Assert loose floors.
        assert!(tree.dirs.len() > 400, "dirs: {}", tree.dirs.len());
        assert!(tree.zip_count("") > 20_000);
        assert!(tree.dirs.contains_key("levels/doom/0-9/"));

        // Second call: conditional refetch — 304 or (if the file rolled) fresh.
        let (tree2, source2) = fetch_ls_lar(&http, dir.path()).await;
        assert!(tree2.is_some());
        assert!(matches!(
            source2,
            BootstrapSource::NotModified { .. } | BootstrapSource::Fresh { .. }
        ));
    }
}
