//! Output record types and deterministic writers (DESIGN.md §4.7).
//!
//! Determinism contract (§9.3): `harvest-manifest.json` is the ONLY output
//! carrying wall-clock timestamps. `idgames-files.jsonl` and
//! `harvest-errors.jsonl` are sorted and timestamp-free so a rerun against
//! unchanged inputs is byte-identical.

use std::path::Path;

use anyhow::Context as _;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::api::model::FileRecord;
use crate::cache::atomic_write;

/// Run provenance (§4.7): "statistics without provenance are not
/// reproducible". Downstream phases reference [`Self::id`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarvestManifest {
    /// Stable run identifier derived from the start time.
    pub id: String,
    /// RFC 3339 run start.
    pub started_at: String,
    /// Wall-clock run duration.
    pub duration_secs: u64,
    /// `meta.version` observed on live responses (3 as spike-verified).
    pub api_version: u64,
    /// `CARGO_PKG_VERSION` of xtask.
    pub tool_version: String,
    /// `git rev-parse --short HEAD`, when available.
    pub git_rev: Option<String>,
    /// Bootstrap provenance label ([`crate::mirror::BootstrapSource::label`]).
    pub bootstrap: String,
    /// Traversal roots (§4.2 include set, or the `--root` override).
    pub roots: Vec<String>,
    /// `--root` value for dev-scoped runs (`None` on full harvests).
    pub scoped_root: Option<String>,
    /// `--limit` value for dev-scoped runs.
    pub limit: Option<u64>,
    /// Directories enumerated.
    pub dir_count: u64,
    /// File records written.
    pub file_count: u64,
    /// Ledger entries written.
    pub error_count: u64,
    /// `getcontents` calls served from the disk cache.
    pub cache_hits: u64,
    /// Live API requests made.
    pub live_api_calls: u64,
    /// Highest archive file id seen — the `latestfiles` probe baseline.
    pub max_file_id: Option<u64>,
}

/// Failure-ledger category (§4.6/§5.5 discipline: record, don't skip).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum LedgerKind {
    /// Retry policy exhausted or non-retryable HTTP status.
    HttpError,
    /// `content.file` and `content.dir` both null (§4.1).
    SuspectPath,
    /// A record failed deserialization.
    ParseError,
    /// API `size` disagrees with the ls-laR listing size (§5.0 guard).
    SizeMismatch,
}

/// One `harvest-errors.jsonl` line. Deliberately timestamp-free (§4.7).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    /// Archive path the failure concerns.
    pub path: String,
    /// API action (or `"ls-laR"` / `"latestfiles"`).
    pub action: String,
    /// Category.
    pub kind: LedgerKind,
    /// Human-readable detail.
    pub detail: String,
    /// Attempts made before giving up (1 for non-retried findings).
    pub attempts: u32,
}

/// Write `idgames-files.jsonl`: records sorted by `id`, deduped by `id`
/// (last occurrence wins), one compact JSON object per line.
///
/// # Errors
/// Serialization or filesystem failure.
pub fn write_files_jsonl(path: &Path, records: Vec<FileRecord>) -> anyhow::Result<u64> {
    let mut by_id = std::collections::BTreeMap::new();
    for rec in records {
        by_id.insert(rec.id, rec);
    }
    let mut out = String::new();
    for rec in by_id.values() {
        out.push_str(&serde_json::to_string(rec).context("serializing file record")?);
        out.push('\n');
    }
    atomic_write(path, out.as_bytes()).with_context(|| format!("writing {}", path.display()))?;
    Ok(u64::try_from(by_id.len()).expect("record count fits u64"))
}

/// Write `harvest-errors.jsonl`, sorted for determinism.
///
/// # Errors
/// Serialization or filesystem failure.
pub fn write_ledger(path: &Path, mut entries: Vec<LedgerEntry>) -> anyhow::Result<u64> {
    entries.sort_by(|a, b| (&a.path, &a.kind, &a.detail).cmp(&(&b.path, &b.kind, &b.detail)));
    let mut out = String::new();
    for e in &entries {
        out.push_str(&serde_json::to_string(e).context("serializing ledger entry")?);
        out.push('\n');
    }
    atomic_write(path, out.as_bytes()).with_context(|| format!("writing {}", path.display()))?;
    Ok(u64::try_from(entries.len()).expect("entry count fits u64"))
}

/// Write the manifest as pretty JSON with a trailing newline.
///
/// # Errors
/// Serialization or filesystem failure.
pub fn write_manifest(path: &Path, manifest: &HarvestManifest) -> anyhow::Result<()> {
    let mut bytes = serde_json::to_vec_pretty(manifest).context("serializing manifest")?;
    bytes.push(b'\n');
    atomic_write(path, &bytes).with_context(|| format!("writing {}", path.display()))
}

/// Read a previous run's manifest, if present and parseable.
pub fn read_manifest(path: &Path) -> Option<HarvestManifest> {
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

/// Read a previous run's `idgames-files.jsonl` as the tree-diff baseline.
/// `None` when the file is missing/unreadable; unparseable lines are
/// skipped with a warning rather than failing the read (a damaged
/// baseline degrades to broader invalidation, never to a crash).
pub fn read_files_jsonl(path: &Path) -> Option<Vec<FileRecord>> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut records = Vec::new();
    let mut skipped = 0_u64;
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        match serde_json::from_str(line) {
            Ok(rec) => records.push(rec),
            Err(_) => skipped += 1,
        }
    }
    if skipped > 0 {
        tracing::warn!(skipped, path = %path.display(), "skipped unparseable baseline lines");
    }
    Some(records)
}

/// `"harvest-YYYYMMDDTHHMMSSZ"` from the run start.
pub fn manifest_id(started_at: &DateTime<Utc>) -> String {
    format!("harvest-{}", started_at.format("%Y%m%dT%H%M%SZ"))
}

/// Best-effort short git revision for provenance.
pub fn git_rev() -> Option<String> {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

/// xtask package version for provenance.
pub fn tool_version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::model::FileRecord;

    fn record(id: u64) -> FileRecord {
        serde_json::from_value(serde_json::json!({
            "id": id, "title": "T", "dir": "levels/doom/0-9/",
            "filename": format!("f{id}.zip"), "size": 10 * id, "age": 0,
            "date": "2003-06-02", "author": "A", "email": "drop@me.invalid",
            "description": "d", "rating": null, "votes": 0,
            "url": "", "idgamesurl": ""
        }))
        .unwrap()
    }

    #[test]
    fn files_jsonl_is_sorted_deduped_and_email_free() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("idgames-files.jsonl");
        let n = write_files_jsonl(&p, vec![record(3), record(1), record(3), record(2)]).unwrap();
        assert_eq!(n, 3);
        let text = std::fs::read_to_string(&p).unwrap();
        let ids: Vec<u64> = text
            .lines()
            .map(|l| {
                serde_json::from_str::<serde_json::Value>(l).unwrap()["id"]
                    .as_u64()
                    .unwrap()
            })
            .collect();
        assert_eq!(ids, vec![1, 2, 3]);
        // §9.3: no email-shaped field in any output line.
        for line in text.lines() {
            let v: serde_json::Value = serde_json::from_str(line).unwrap();
            crate::api::model::tests::assert_no_email_keys(&v);
        }
    }

    #[test]
    fn files_jsonl_reruns_are_byte_identical() {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a.jsonl");
        let b = tmp.path().join("b.jsonl");
        write_files_jsonl(&a, vec![record(2), record(9), record(4)]).unwrap();
        write_files_jsonl(&b, vec![record(4), record(2), record(9)]).unwrap();
        assert_eq!(std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());
    }

    #[test]
    fn ledger_is_sorted_and_timestamp_free() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("harvest-errors.jsonl");
        let entries = vec![
            LedgerEntry {
                path: "levels/z/".into(),
                action: "getcontents".into(),
                kind: LedgerKind::HttpError,
                detail: "HTTP 500".into(),
                attempts: 6,
            },
            LedgerEntry {
                path: "levels/a/".into(),
                action: "getcontents".into(),
                kind: LedgerKind::SuspectPath,
                detail: "file and dir both null".into(),
                attempts: 1,
            },
        ];
        let n = write_ledger(&p, entries).unwrap();
        assert_eq!(n, 2);
        let text = std::fs::read_to_string(&p).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert!(lines[0].contains("levels/a/"));
        assert!(lines[1].contains("levels/z/"));
        // §4.7: no wall-clock in this output — spot-check field names.
        for line in &lines {
            let v: serde_json::Value = serde_json::from_str(line).unwrap();
            let keys: Vec<&String> = v.as_object().unwrap().keys().collect();
            assert!(
                !keys
                    .iter()
                    .any(|k| k.contains("time") || k.contains("date") || k.ends_with("at")),
                "wall-clock-shaped key in ledger: {keys:?}"
            );
        }
    }

    #[test]
    fn ledger_kind_serializes_snake_case() {
        let v = serde_json::to_value(LedgerKind::SizeMismatch).unwrap();
        assert_eq!(v, "size_mismatch");
    }

    #[test]
    fn files_jsonl_roundtrips_through_reader() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("idgames-files.jsonl");
        write_files_jsonl(&p, vec![record(5), record(2)]).unwrap();
        let back = read_files_jsonl(&p).unwrap();
        assert_eq!(back.len(), 2);
        assert_eq!(back[0].id, 2);
        assert_eq!(back[1].id, 5);
        assert!(read_files_jsonl(&tmp.path().join("missing.jsonl")).is_none());
        // A corrupt line is skipped, not fatal.
        std::fs::write(&p, "{ not json\n").unwrap();
        let salvaged = read_files_jsonl(&p).map(|v| v.len());
        assert_eq!(salvaged, Some(0));
    }

    #[test]
    fn manifest_roundtrips_and_id_format() {
        let started = chrono::DateTime::parse_from_rfc3339("2026-08-15T12:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        assert_eq!(manifest_id(&started), "harvest-20260815T120000Z");
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("harvest-manifest.json");
        let m = HarvestManifest {
            id: manifest_id(&started),
            started_at: started.to_rfc3339(),
            duration_secs: 480,
            api_version: 3,
            tool_version: tool_version(),
            git_rev: git_rev(),
            bootstrap: "ls-lar-fresh:infania".into(),
            roots: vec!["levels/".into()],
            scoped_root: None,
            limit: None,
            dir_count: 462,
            file_count: 21_375,
            error_count: 0,
            cache_hits: 0,
            live_api_calls: 463,
            max_file_id: Some(22_083),
        };
        write_manifest(&p, &m).unwrap();
        let back = read_manifest(&p).unwrap();
        assert_eq!(back.id, m.id);
        assert_eq!(back.max_file_id, Some(22_083));
        assert!(std::fs::read_to_string(&p).unwrap().ends_with('\n'));
        assert!(read_manifest(&tmp.path().join("missing.json")).is_none());
    }
}
