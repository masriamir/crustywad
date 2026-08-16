//! Request-keyed disk cache with tiered TTL and email scrubbing (DESIGN.md §4.5).
//!
//! The cache key is the **request** (`action` + path), never the response.
//! Bodies are email-scrubbed at write time and `body_hash` is computed over
//! the scrubbed body (ADR-0030 §3): raw responses are never persisted.

use std::io;
use std::path::{Path, PathBuf};

use anyhow::Context as _;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// On-disk cache entry envelope (DESIGN.md §4.5).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEnvelope {
    /// API action, e.g. `"getcontents"`.
    pub action: String,
    /// Request path (with trailing slash), e.g. `"levels/doom/"`.
    pub path: String,
    /// RFC 3339 UTC timestamp of the fetch.
    pub fetched_at: String,
    /// `meta.version` observed on the response.
    pub api_version: u64,
    /// `"blake3:<hex>"` over the **scrubbed** body's compact serialization.
    pub body_hash: String,
    /// The email-scrubbed response body.
    pub body: serde_json::Value,
}

/// Directory-backed cache for API responses.
#[derive(Debug)]
pub struct ApiCache {
    root: PathBuf,
    ttl: Duration,
}

impl ApiCache {
    /// Open (creating if needed) a cache rooted at `root` with the given
    /// freshness TTL.
    ///
    /// # Errors
    /// Fails if the directory cannot be created.
    pub fn new(root: PathBuf, ttl: Duration) -> anyhow::Result<Self> {
        std::fs::create_dir_all(&root)
            .with_context(|| format!("creating cache dir {}", root.display()))?;
        Ok(Self { root, ttl })
    }

    /// Get the filesystem path for a cache entry key.
    pub(crate) fn entry_path(&self, key: &str) -> PathBuf {
        self.root.join(format!("{key}.json"))
    }

    /// Fetch the stored envelope for a request, at any age. Corrupt or
    /// mismatched entries read as misses.
    pub fn lookup(&self, action: &str, path: &str) -> Option<CacheEnvelope> {
        let bytes = std::fs::read(self.entry_path(&cache_key(action, path))).ok()?;
        let env: CacheEnvelope = serde_json::from_slice(&bytes).ok()?;
        // Hash-collision paranoia is cheap: confirm the entry is really ours.
        (env.action == action && env.path == path).then_some(env)
    }

    /// Whether `envelope` is within the TTL as of `now`. Unparseable
    /// timestamps are stale.
    pub fn is_fresh(&self, envelope: &CacheEnvelope, now: DateTime<Utc>) -> bool {
        DateTime::parse_from_rfc3339(&envelope.fetched_at)
            .is_ok_and(|t| now - t.with_timezone(&Utc) < self.ttl)
    }

    /// Scrub `body`, hash it, and persist the envelope atomically.
    ///
    /// # Errors
    /// Fails on serialization or filesystem errors.
    pub fn store(
        &self,
        action: &str,
        path: &str,
        api_version: u64,
        mut body: serde_json::Value,
    ) -> anyhow::Result<CacheEnvelope> {
        scrub_emails(&mut body);
        let envelope = CacheEnvelope {
            action: action.to_owned(),
            path: path.to_owned(),
            fetched_at: Utc::now().to_rfc3339(),
            api_version,
            body_hash: body_hash(&body),
            body,
        };
        let bytes = serde_json::to_vec(&envelope).context("serializing cache envelope")?;
        let target = self.entry_path(&cache_key(action, path));
        atomic_write(&target, &bytes)
            .with_context(|| format!("writing cache entry {}", target.display()))?;
        Ok(envelope)
    }

    /// Drop a cached entry if present.
    ///
    /// # Errors
    /// Fails on filesystem errors other than the entry not existing.
    pub fn invalidate(&self, action: &str, path: &str) -> io::Result<()> {
        match std::fs::remove_file(self.entry_path(&cache_key(action, path))) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }
}

/// Remove every object key containing `email` (ASCII case-insensitive),
/// recursively (ADR-0030 §3). Applied to every body before it touches disk.
pub fn scrub_emails(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            map.retain(|k, _| !k.to_ascii_lowercase().contains("email"));
            map.values_mut().for_each(scrub_emails);
        }
        serde_json::Value::Array(items) => items.iter_mut().for_each(scrub_emails),
        _ => {}
    }
}

/// Hash a request identity to a filesystem-safe filename stem.
pub fn cache_key(action: &str, path: &str) -> String {
    blake3::hash(format!("{action}\n{path}").as_bytes())
        .to_hex()
        .to_string()
}

/// `"blake3:<hex>"` over the compact JSON serialization of `body`.
/// Change-detection only — never a cache key (§4.5).
pub fn body_hash(body: &serde_json::Value) -> String {
    let bytes = serde_json::to_vec(body).unwrap_or_default();
    format!("blake3:{}", blake3::hash(&bytes).to_hex())
}

/// Write `bytes` to `path` via a `.tmp` sibling + rename, so readers never
/// observe a torn file.
///
/// # Errors
/// Propagates filesystem errors.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use serde_json::json;

    fn cache() -> (tempfile::TempDir, ApiCache) {
        let dir = tempfile::tempdir().unwrap();
        let cache = ApiCache::new(dir.path().join("api"), Duration::days(7)).unwrap();
        (dir, cache)
    }

    #[test]
    fn scrub_removes_email_keys_recursively() {
        let mut v = json!({
            "content": {
                "file": [
                    {"id": 1, "email": "a@example.com", "author": "A"},
                    {"id": 2, "Email": "b@example.com", "nested": {"EMAIL": "c@d"}}
                ],
                "dir": null
            },
            "email": "top@example.com"
        });
        scrub_emails(&mut v);
        crate::api::model::tests::assert_no_email_keys(&v);
        // Non-email content survives.
        assert_eq!(v["content"]["file"][0]["author"], "A");
        assert_eq!(v["content"]["file"][1]["id"], 2);
    }

    #[test]
    fn cache_key_is_stable_and_distinct() {
        assert_eq!(
            cache_key("getcontents", "levels/doom/"),
            cache_key("getcontents", "levels/doom/")
        );
        assert_ne!(
            cache_key("getcontents", "levels/doom/"),
            cache_key("getcontents", "levels/doom2/")
        );
        assert_ne!(
            cache_key("getcontents", "levels/doom/"),
            cache_key("getdirs", "levels/doom/")
        );
        // Safe filename: hex only.
        assert!(cache_key("a", "b").chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn store_scrubs_and_lookup_roundtrips() {
        let (_tmp, cache) = cache();
        let body = json!({"content": {"file": [{"id": 7, "email": "x@y.z"}], "dir": null}});
        let stored = cache.store("getcontents", "levels/doom/", 3, body).unwrap();
        crate::api::model::tests::assert_no_email_keys(&stored.body);

        let found = cache.lookup("getcontents", "levels/doom/").unwrap();
        assert_eq!(found.action, "getcontents");
        assert_eq!(found.path, "levels/doom/");
        assert_eq!(found.api_version, 3);
        assert_eq!(found.body_hash, stored.body_hash);
        // §9.3: a stored body carries no email anywhere.
        crate::api::model::tests::assert_no_email_keys(&found.body);
        assert!(found.body_hash.starts_with("blake3:"));
    }

    #[test]
    fn body_hash_is_over_the_scrubbed_body() {
        // Two bodies differing only in email fields must hash identically
        // (§4.5: an email-only change goes undetected by design).
        let (_tmp, cache) = cache();
        let a = json!({"content": {"file": [{"id": 7, "email": "x@y.z"}]}});
        let b = json!({"content": {"file": [{"id": 7, "email": "other@y.z"}]}});
        let ha = cache.store("getcontents", "a/", 3, a).unwrap().body_hash;
        let hb = cache.store("getcontents", "b/", 3, b).unwrap().body_hash;
        assert_eq!(ha, hb);
        // And a real content change must move the hash.
        let c = json!({"content": {"file": [{"id": 8, "email": "x@y.z"}]}});
        let hc = cache.store("getcontents", "c/", 3, c).unwrap().body_hash;
        assert_ne!(ha, hc);
    }

    #[test]
    fn freshness_respects_ttl() {
        let (_tmp, cache) = cache();
        let env = cache
            .store("getcontents", "levels/doom/", 3, json!({"content": {}}))
            .unwrap();
        let now = Utc::now();
        assert!(cache.is_fresh(&env, now));
        assert!(cache.is_fresh(&env, now + Duration::days(6)));
        assert!(!cache.is_fresh(&env, now + Duration::days(8)));
        // Unparseable fetched_at == stale, never fresh.
        let mut bad = env.clone();
        bad.fetched_at = "not-a-date".into();
        assert!(!cache.is_fresh(&bad, now));
    }

    #[test]
    fn lookup_misses_and_invalidate() {
        let (_tmp, cache) = cache();
        assert!(cache.lookup("getcontents", "levels/doom/").is_none());
        cache
            .store("getcontents", "levels/doom/", 3, json!({"content": {}}))
            .unwrap();
        assert!(cache.lookup("getcontents", "levels/doom/").is_some());
        cache.invalidate("getcontents", "levels/doom/").unwrap();
        assert!(cache.lookup("getcontents", "levels/doom/").is_none());
        // Invalidating a missing entry is not an error.
        cache.invalidate("getcontents", "levels/doom/").unwrap();
    }

    #[test]
    fn lookup_ignores_corrupt_entries() {
        let (_tmp, cache) = cache();
        let key = cache_key("getcontents", "levels/doom/");
        std::fs::write(cache.entry_path(&key), b"{ not json").unwrap();
        assert!(cache.lookup("getcontents", "levels/doom/").is_none());
    }

    #[test]
    fn atomic_write_replaces_existing() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("out.json");
        atomic_write(&p, b"one").unwrap();
        atomic_write(&p, b"two").unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), b"two");
        // No stray .tmp left behind.
        assert!(!tmp.path().join("out.json.tmp").exists());
    }
}
