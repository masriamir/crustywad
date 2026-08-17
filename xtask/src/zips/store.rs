//! Per-id phase-2 results log with §5.4 `body_hash` invalidation.
//!
//! The log is append-only JSONL — crash-safe by construction (a torn final
//! line is skipped on load, everything before it survives), which is what
//! makes the phase resumable (§5.4). Each line binds a [`WadRecord`] to the
//! Phase-1 `body_hash` of its containing directory at inspection time; a
//! cached result is reused ONLY while that hash still matches (§5.4: "only
//! when Phase 1's `body_hash` for the containing directory changed, or the
//! entry's id is new"). A directory with no Phase-1 cache envelope yields
//! no current hash, which never matches — degrading to a refetch, never to
//! stale reuse.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write as _;
use std::path::PathBuf;

use anyhow::Context as _;
use serde::{Deserialize, Serialize};

use crate::api::model::FileRecord;
use crate::cache::ApiCache;
use crate::schema::WadRecord;

/// One log line: the record plus the invalidation key it was made under.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredZip {
    /// `body_hash` of the containing dir's Phase-1 envelope at write time.
    dir_body_hash: String,
    /// The phase-2 record.
    record: WadRecord,
}

/// Append-only per-id results store (§5.4 resumability).
#[derive(Debug)]
pub struct ZipsStore {
    path: PathBuf,
    entries: BTreeMap<u64, StoredZip>,
}

impl ZipsStore {
    /// Open the results log at `path`, loading any existing entries.
    ///
    /// A missing file is an empty store (first run). Lines that fail to
    /// parse — e.g. a torn trailing line from a crash mid-write — are
    /// skipped with a `tracing::warn!` rather than failing the open;
    /// earlier, well-formed lines still load (§5.4 resumability). When the
    /// same id appears more than once, the later line wins.
    #[must_use]
    pub fn open(path: PathBuf) -> Self {
        let mut entries = BTreeMap::new();
        // Read bytes, not a String: a crash-torn final line can split a
        // multi-byte UTF-8 sequence, and `read_to_string` would then fail
        // the WHOLE load — silently degrading the store to empty instead
        // of recovering every prior well-formed line. Parsing per line via
        // `from_slice` confines any invalid UTF-8 to the line that carries
        // it, which lands in `skipped` like any other torn write.
        match std::fs::read(&path) {
            Ok(bytes) => {
                let mut skipped = 0_u64;
                for line in bytes
                    .split(|b| *b == b'\n')
                    .filter(|l| !l.iter().all(u8::is_ascii_whitespace))
                {
                    match serde_json::from_slice::<StoredZip>(line) {
                        Ok(stored) => {
                            entries.insert(stored.record.id, stored);
                        }
                        Err(_) => skipped += 1,
                    }
                }
                if skipped > 0 {
                    tracing::warn!(skipped, path = %path.display(), "skipped unparseable zips-log lines");
                }
            }
            // No log yet is the normal first-run state; any OTHER read
            // failure (permissions, disk fault) silently emptying the
            // resumable cache would masquerade as a legitimate full
            // rescan — surface it loudly instead.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                tracing::warn!(
                    path = %path.display(),
                    error = %e,
                    "zips log unreadable; starting with an empty store"
                );
            }
        }
        Self { path, entries }
    }

    /// Look up a cached record for `id`, valid only while `current_dir_hash`
    /// matches the hash it was stored under (§5.4). `None` for either a
    /// hash mismatch, a directory with no current Phase-1 envelope
    /// (`current_dir_hash` is `None`), or an id never seen before.
    #[must_use]
    pub fn lookup(&self, id: u64, current_dir_hash: Option<&str>) -> Option<&WadRecord> {
        self.entries
            .get(&id)
            .filter(|s| current_dir_hash == Some(s.dir_body_hash.as_str()))
            .map(|s| &s.record)
    }

    /// Insert `record` under `dir_body_hash` and append it to the log,
    /// flushed immediately so a subsequent crash cannot lose it.
    ///
    /// # Errors
    /// Serialization or filesystem failure. The parent directory of `path`
    /// is expected to already exist — the orchestrator creates it once up
    /// front, not this per-record call.
    pub fn record(&mut self, dir_body_hash: &str, record: WadRecord) -> anyhow::Result<()> {
        let stored = StoredZip {
            dir_body_hash: dir_body_hash.to_owned(),
            record,
        };
        let mut line = serde_json::to_string(&stored).context("serializing zips-log entry")?;
        line.push('\n');
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .with_context(|| format!("opening {}", self.path.display()))?;
        file.write_all(line.as_bytes())
            .with_context(|| format!("writing {}", self.path.display()))?;
        file.flush()
            .with_context(|| format!("flushing {}", self.path.display()))?;
        self.entries.insert(stored.record.id, stored);
        Ok(())
    }
}

/// Resolve each distinct `dir` in `records` to its current Phase-1
/// `getcontents` cache envelope's `body_hash`. A directory with no
/// envelope in `cache` is omitted rather than mapped to a placeholder, so
/// [`ZipsStore::lookup`] degrades to a refetch for it.
#[must_use]
pub fn dir_hashes(records: &[FileRecord], cache: &ApiCache) -> BTreeMap<String, String> {
    let dirs: BTreeSet<&str> = records.iter().map(|r| r.dir.as_str()).collect();
    dirs.into_iter()
        .filter_map(|dir| {
            let envelope = cache.lookup("getcontents", dir)?;
            Some((dir.to_owned(), envelope.body_hash))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{FetchStatus, WadRecord};

    fn record(id: u64) -> WadRecord {
        WadRecord {
            id,
            dir: "levels/doom/0-9/".into(),
            filename: format!("f{id}.zip"),
            zip_size: 100,
            date: String::new(),
            rating: None,
            votes: 0,
            is_zip: true,
            zip64: false,
            member_count: 1,
            wads: Vec::new(),
            other_members: vec!["readme.txt".into()],
            mirror: "infania".into(),
            fetch_status: FetchStatus::Ok,
        }
    }

    #[test]
    fn store_roundtrips_across_reopen() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("zips-log.jsonl");
        let mut store = ZipsStore::open(path.clone());
        store.record("blake3:aaa", record(7)).unwrap();
        store.record("blake3:aaa", record(9)).unwrap();
        drop(store);
        let store = ZipsStore::open(path);
        assert!(store.lookup(7, Some("blake3:aaa")).is_some());
        assert!(store.lookup(9, Some("blake3:aaa")).is_some());
    }

    #[test]
    fn hash_change_and_unknown_hash_both_invalidate() {
        let tmp = tempfile::tempdir().unwrap();
        let mut store = ZipsStore::open(tmp.path().join("log.jsonl"));
        store.record("blake3:aaa", record(7)).unwrap();
        // §5.4: invalidate ONLY on body_hash change or new id.
        assert!(store.lookup(7, Some("blake3:aaa")).is_some());
        assert!(store.lookup(7, Some("blake3:bbb")).is_none(), "hash moved");
        assert!(store.lookup(7, None).is_none(), "no envelope → refetch");
        assert!(store.lookup(8, Some("blake3:aaa")).is_none(), "new id");
    }

    #[test]
    fn later_entries_win_and_corrupt_lines_are_skipped() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("log.jsonl");
        let mut store = ZipsStore::open(path.clone());
        store.record("blake3:aaa", record(7)).unwrap();
        let mut updated = record(7);
        updated.mirror = "gamers".into();
        store.record("blake3:ccc", updated).unwrap();
        drop(store);
        // Corrupt trailing line (torn write) must not poison the log.
        {
            use std::io::Write as _;
            let mut f = std::fs::OpenOptions::new()
                .append(true)
                .open(&path)
                .unwrap();
            f.write_all(b"{ torn").unwrap();
        }
        let store = ZipsStore::open(path);
        assert!(store.lookup(7, Some("blake3:aaa")).is_none(), "superseded");
        assert_eq!(
            store.lookup(7, Some("blake3:ccc")).unwrap().mirror,
            "gamers"
        );
    }

    #[test]
    fn torn_non_utf8_final_line_does_not_discard_prior_entries() {
        // A crash mid-append can tear a line inside a multi-byte UTF-8
        // sequence, making the WHOLE file invalid UTF-8 — a String-based
        // read would then fail the entire load and silently degrade the
        // store to empty. The byte-based loader must confine the damage
        // to the torn line.
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("log.jsonl");
        let mut store = ZipsStore::open(path.clone());
        store.record("blake3:aaa", record(7)).unwrap();
        store.record("blake3:aaa", record(9)).unwrap();
        drop(store);
        {
            use std::io::Write as _;
            let mut f = std::fs::OpenOptions::new()
                .append(true)
                .open(&path)
                .unwrap();
            // "café" cut after the first byte of the two-byte 'é'.
            f.write_all(b"{\"dir_body_hash\":\"blake3:bbb\",\"record\":{\"filename\":\"caf\xC3")
                .unwrap();
        }
        let store = ZipsStore::open(path);
        assert!(store.lookup(7, Some("blake3:aaa")).is_some());
        assert!(store.lookup(9, Some("blake3:aaa")).is_some());
    }

    #[test]
    fn missing_log_is_an_empty_store() {
        let tmp = tempfile::tempdir().unwrap();
        let store = ZipsStore::open(tmp.path().join("absent.jsonl"));
        assert!(store.lookup(1, Some("blake3:aaa")).is_none());
    }

    #[cfg(unix)]
    #[test]
    fn unreadable_log_degrades_to_empty_without_panicking() {
        // The warn itself isn't captured here; the pinned behavior is the
        // degradation path: unreadable (not absent) → empty store, no
        // panic, no partial state.
        use std::os::unix::fs::PermissionsExt as _;
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("log.jsonl");
        let mut store = ZipsStore::open(path.clone());
        store.record("blake3:aaa", record(7)).unwrap();
        drop(store);
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();
        let store = ZipsStore::open(path.clone());
        assert!(store.lookup(7, Some("blake3:aaa")).is_none());
        // Restore so the tempdir can clean up.
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    }

    #[test]
    fn dir_hashes_resolves_present_dirs_and_omits_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let cache =
            crate::cache::ApiCache::new(tmp.path().join("api"), chrono::Duration::days(7)).unwrap();
        let env = cache
            .store(
                "getcontents",
                "levels/doom/0-9/",
                3,
                serde_json::json!({"content": {}}),
            )
            .unwrap();
        let recs: Vec<crate::api::model::FileRecord> = vec![
            serde_json::from_value(serde_json::json!({
                "id": 1, "dir": "levels/doom/0-9/", "filename": "a.zip",
                "size": 10, "age": 0
            }))
            .unwrap(),
            serde_json::from_value(serde_json::json!({
                "id": 2, "dir": "levels/doom/a-c/", "filename": "b.zip",
                "size": 10, "age": 0
            }))
            .unwrap(),
        ];
        let hashes = dir_hashes(&recs, &cache);
        assert_eq!(hashes.get("levels/doom/0-9/"), Some(&env.body_hash));
        assert!(
            !hashes.contains_key("levels/doom/a-c/"),
            "no envelope → absent"
        );
    }
}
