//! Output record types and deterministic writers (DESIGN.md §4.7).
//!
//! Determinism contract (§9.3): the per-phase manifests
//! (`harvest-manifest.json`, `wads-manifest.json`) are the only outputs
//! carrying wall-clock timestamps. `idgames-files.jsonl`,
//! `harvest-errors.jsonl`, and `idgames-wads.jsonl` are sorted and
//! timestamp-free so a rerun against unchanged inputs is byte-identical.

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
    /// `meta.version` observed on parsed response envelopes this run —
    /// live or cache-fresh (3 as spike-verified). `0` means no envelope
    /// was parsed at all (unknown).
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
    /// Exchange-level failure: retry policy exhausted, a non-retryable
    /// HTTP status, a body-stage failure (over-cap or mid-body transport
    /// error), or the API's own error envelope.
    HttpError,
    /// `content.file` and `content.dir` both null (§4.1).
    SuspectPath,
    /// Bytes arrived whole but did not deserialize: an unrecognized
    /// envelope shape or a record that failed deserialization.
    ParseError,
    /// API `size` disagrees with the ls-laR listing size (§5.0 guard).
    SizeMismatch,
    /// Deliberately not attempted (#442): a curated outlier marked
    /// `skip = true` in `xtask/outliers.toml` — no exchange occurred
    /// (`attempts: 0`), distinguishing "never asked" from
    /// [`Self::HttpError`]'s "asked and failed". Outliers-only; phase 1
    /// never emits it.
    Skipped,
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
    /// Attempts made before giving up (1 for a non-retried finding that
    /// issued its single request).
    ///
    /// For `outliers::run`'s `harvest-outliers` ledger entries this is the
    /// entry's real HTTP request count as of #442 — the
    /// [`crate::zips::range_reader::TransferCounters`] requests delta
    /// around the entry, covering every request it spent (a retried HEAD
    /// ladder, the range probe, `inspect_zip`'s reads). `0` for an entry
    /// that never issued a request: one skipped via `outliers.toml`'s
    /// `skip = true` marker, or (defensively) one whose URL failed to parse.
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
    // The key is the full record: a partial key would leave ties to the
    // (insertion-order-dependent) stable sort and break the §9.3
    // byte-identical rerun contract for duplicate findings.
    entries.sort_by(|a, b| {
        (&a.path, &a.action, &a.kind, &a.detail, a.attempts)
            .cmp(&(&b.path, &b.action, &b.kind, &b.detail, b.attempts))
    });
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

/// Read `harvest-errors.jsonl` — the phase-1 failure ledger (§4.7). `None`
/// when the file is missing/unreadable; unparseable lines are skipped with
/// a warning rather than failing the read, mirroring [`read_files_jsonl`].
pub fn read_ledger(path: &Path) -> Option<Vec<LedgerEntry>> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut entries = Vec::new();
    let mut skipped = 0_u64;
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        match serde_json::from_str(line) {
            Ok(entry) => entries.push(entry),
            Err(_) => skipped += 1,
        }
    }
    if skipped > 0 {
        tracing::warn!(skipped, path = %path.display(), "skipped unparseable ledger lines");
    }
    Some(entries)
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

/// Best-effort short git revision for provenance. Anchored to the xtask
/// manifest dir, not the process CWD — the tool works from any cwd
/// (compile-time paths everywhere else), and provenance must too.
pub fn git_rev() -> Option<String> {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
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

/// §5.6 closed enum — every §5.5 edge case gets a named value, or "record,
/// don't skip" is unenforceable. Do not add variants without a DESIGN §5.6
/// correction in the same commit.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FetchStatus {
    /// Central directory read over ranges; sizes recorded.
    Ok,
    /// The entry's filename is not `.zip` — no mirror contact at all.
    NotZip,
    /// Both mirrors answered 404 for the entry.
    ///
    /// `rename_all = "snake_case"` alone renders this as `mirror404_all`
    /// (serde splits on digit-to-uppercase but not letter-to-digit
    /// transitions) — an explicit rename pins the §5.6 wire value.
    #[serde(rename = "mirror_404_all")]
    Mirror404All,
    /// Both mirrors refused ranges and the budgeted fallback was not taken.
    NoRangeSupport,
    /// Both mirrors refused ranges; sizes come from a budgeted full download.
    FullDownload,
    /// Bytes arrived but the zip central directory did not parse (or hit a cap).
    ZipParseError,
    /// Transport-level failure after retries on every usable mirror.
    FetchError,
    /// Curated outlier marked `skip = true` in `xtask/outliers.toml` (§6.4):
    /// a documented-hostile host, deliberately not probed this run — the
    /// prior refusal is recorded in the TOML entry's `note`. Outliers-only;
    /// phase 2 never emits it.
    SkippedKnownDead,
}

/// One `.wad` member of an archive entry (§5.6 `wads[]`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WadMember {
    /// Member name exactly as the central directory declares it.
    pub name: String,
    /// Declared compressed size in bytes.
    pub compressed: u64,
    /// Declared uncompressed size in bytes — the number this phase exists for.
    pub uncompressed: u64,
    /// Compression method label (`"stored"`, `"deflate"`, or another §5.5
    /// method name).
    pub method: String,
    /// General-purpose-bit-0 encryption flag (§5.5).
    pub encrypted: bool,
}

/// One `idgames-wads.jsonl` line (§5.6 — field order here is the output
/// schema). `date`/`rating`/`votes` are copied from the Phase-1 record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WadRecord {
    /// Archive file ID (Phase-1 `FileRecord::id`).
    pub id: u64,
    /// Directory path with trailing slash.
    pub dir: String,
    /// Zip filename, no path.
    pub filename: String,
    /// Zip size in bytes (Phase-1 `size`).
    pub zip_size: u64,
    /// `YYYY-MM-DD` from Phase 1.
    pub date: String,
    /// Mean rating from Phase 1; `None` when unrated.
    pub rating: Option<f64>,
    /// Vote count from Phase 1.
    pub votes: u64,
    /// Whether the entry is a zip (filename `.zip`, ASCII case-insensitive).
    pub is_zip: bool,
    /// ZIP64 EOCD locator present (§5.3) — detected from the fetched tail.
    pub zip64: bool,
    /// Count of *distinct* central-directory entries by name (directories
    /// included): zip 8's `ZipArchive` keys parsed entries in an
    /// `IndexMap<name, ..>` (last-one-wins on a duplicate name), so an
    /// archive with duplicate member names under-counts here by design —
    /// this is what `zip` itself exposes, not the raw CD record count on
    /// disk.
    pub member_count: u64,
    /// `.wad` members, case-insensitively matched (§5.5).
    pub wads: Vec<WadMember>,
    /// Every non-`.wad` member name (nested archives §5.5 appear here) —
    /// plus any `.wad` member whose local file header could not be read
    /// (genuinely unreadable, not a range-cache miss): its real
    /// central-directory name lands here rather than in `wads`, since a
    /// size can't be fabricated for it (see
    /// `zips::inspect::inspection_from_archive`). Phase-3 consumers must
    /// not assume every name here is wad-free.
    pub other_members: Vec<String>,
    /// Mirror key that served the bytes; `""` when no mirror was contacted.
    pub mirror: String,
    /// Closed §5.6 outcome enum.
    pub fetch_status: FetchStatus,
}

/// Write `idgames-wads.jsonl`: sorted by `id`, deduped by `id` (last
/// occurrence wins), one compact JSON object per line.
///
/// # Errors
/// Serialization or filesystem failure.
pub fn write_wads_jsonl(path: &Path, records: Vec<WadRecord>) -> anyhow::Result<u64> {
    let mut by_id = std::collections::BTreeMap::new();
    for rec in records {
        by_id.insert(rec.id, rec);
    }
    let mut out = String::new();
    for rec in by_id.values() {
        out.push_str(&serde_json::to_string(rec).context("serializing wad record")?);
        out.push('\n');
    }
    atomic_write(path, out.as_bytes()).with_context(|| format!("writing {}", path.display()))?;
    Ok(u64::try_from(by_id.len()).expect("record count fits u64"))
}

/// Parse `idgames-wads.jsonl` lines already read into memory (§5.6).
/// Unparseable lines are skipped with a warning rather than failing the
/// parse; `path` is used only to name the file in that warning. Split out
/// of [`read_wads_jsonl`] so a caller that also needs the raw bytes (e.g.
/// to hash them) can read the file exactly once.
pub fn parse_wads_jsonl(text: &str, path: &Path) -> Vec<WadRecord> {
    let mut records = Vec::new();
    let mut skipped = 0_u64;
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        match serde_json::from_str(line) {
            Ok(rec) => records.push(rec),
            Err(_) => skipped += 1,
        }
    }
    if skipped > 0 {
        tracing::warn!(skipped, path = %path.display(), "skipped unparseable wad lines");
    }
    records
}

/// Read a previous run's `idgames-wads.jsonl` (§5.6). `None` when the file
/// is missing/unreadable; unparseable lines are skipped with a warning
/// rather than failing the read, mirroring [`read_files_jsonl`].
pub fn read_wads_jsonl(path: &Path) -> Option<Vec<WadRecord>> {
    let text = std::fs::read_to_string(path).ok()?;
    Some(parse_wads_jsonl(&text, path))
}

/// Phase-2 run provenance and the §9.3 acceptance witnesses: byte totals
/// (small fraction of the archive or the range reader is broken), ZIP64
/// count (≥1 resolved or absence stated), fallback/budget accounting, and
/// the completeness counter. The only Phase-2 output with wall-clock data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZipsManifest {
    /// `"harvest-zips-YYYYMMDDTHHMMSSZ"` from the run start.
    pub id: String,
    /// RFC 3339 run start.
    pub started_at: String,
    /// Wall-clock run duration.
    pub duration_secs: u64,
    /// `CARGO_PKG_VERSION` of xtask.
    pub tool_version: String,
    /// `git rev-parse --short HEAD`, when available.
    pub git_rev: Option<String>,
    /// `--root` value for dev-scoped runs.
    pub scoped_root: Option<String>,
    /// `--limit` value for dev-scoped runs.
    pub limit: Option<u64>,
    /// Phase-1 entries in this run's worklist.
    pub entries_total: u64,
    /// `idgames-wads.jsonl` lines written.
    pub records_written: u64,
    /// `wads-errors.jsonl` lines written.
    pub ledger_count: u64,
    /// Entries served from the per-id results log.
    pub cache_hits: u64,
    /// Entries drained from the live mirror pool this run. On an aborted
    /// run this counts only entries that finished before the breaker
    /// tripped — in-flight entries cancelled by the abort are not counted
    /// (they also never get a record; see `aborted`).
    pub live_entries: u64,
    /// Ranged/full GET requests issued this run.
    pub range_requests: u64,
    /// Total response-body bytes read from mirrors this run (§9.3: must
    /// stay a small fraction of the archive's ~38 GiB).
    pub bytes_transferred: u64,
    /// Entries resolved via the budgeted full-download fallback this run
    /// (a cache-hit entry that was a full download on a *prior* run is not
    /// recounted here — this mirrors `range_requests`/`bytes_transferred`'s
    /// run-scoped semantics, not a whole-corpus total).
    pub full_downloads: u64,
    /// Bytes consumed by the fallback this run (bounded by the 2 GiB
    /// budget — see `full_downloads` on why this is run-scoped, not
    /// cumulative across warm reruns).
    pub fallback_bytes: u64,
    /// Records with `zip64: true` (0 explicitly states ZIP64 absence, §9.3).
    pub zip64_entries: u64,
    /// Record count per `fetch_status` value.
    pub status_counts: std::collections::BTreeMap<String, u64>,
    /// Worklist entries with no `idgames-wads.jsonl` record — computed from
    /// `records` alone (every failure gets a record too, via the
    /// ledger-writing branch, so this is not "records ∪ ledger" despite
    /// what the name suggests). §9.3 demands 0 on a completed run. It is
    /// EXPECTED to be nonzero on an aborted run (`aborted.is_some()`): the
    /// entries still in flight when a breaker trips are cancelled without
    /// ever getting a record. On a non-aborted run, anything else is a bug
    /// surfaced loudly.
    pub unaccounted_entries: u64,
    /// `Some(reason)` when either abort breaker stopped the phase early:
    /// the ~2% fallback circuit breaker (§5.2), or the 4 GiB range-path
    /// byte ceiling (`RANGE_BYTE_CEILING` in `zips/mod.rs`, tripped by a
    /// pathological EOCD/central-directory shape that pushes payload down
    /// the range path invisibly to the fallback budget). Outputs then
    /// cover only the completed prefix.
    pub aborted: Option<String>,
}

/// Write the phase-2 manifest as pretty JSON with a trailing newline.
///
/// # Errors
/// Serialization or filesystem failure.
pub fn write_zips_manifest(path: &Path, manifest: &ZipsManifest) -> anyhow::Result<()> {
    let mut bytes = serde_json::to_vec_pretty(manifest).context("serializing zips manifest")?;
    bytes.push(b'\n');
    atomic_write(path, &bytes).with_context(|| format!("writing {}", path.display()))
}

/// Read a previous run's phase-2 manifest, if present and parseable.
pub fn read_zips_manifest(path: &Path) -> Option<ZipsManifest> {
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

/// One `.wad` member of a §6.5 sweep-corpus entry: name and declared
/// uncompressed size only — no free text, per the ADR-0030 §3 allowlist
/// (`sweep-corpus.jsonl` is one of the three files that rule binds).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SweepWad {
    /// Member name, as recorded in [`WadRecord::wads`].
    pub name: String,
    /// Declared uncompressed size in bytes.
    pub uncompressed: u64,
}

/// One `data/sweep-corpus.jsonl` line (§6.5): a ready-made fetch list for
/// `CRUSTYWAD_SWEEP_DIR` and `cargo-fuzz` seeds. Only `id`, a mirror URL,
/// and the expected `.wad` member names/sizes — the ADR-0030 §3
/// free-text ban applies here too.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SweepEntry {
    /// Archive file ID (Phase-1 `FileRecord::id`).
    pub id: u64,
    /// Download URL, always built against [`crate::mirror::MIRRORS`]`[0]`
    /// regardless of which mirror actually served the phase-2 harvest
    /// (§6.5) — reproducible independent of that run's mirror-selection
    /// history.
    pub url: String,
    /// `.wad` members expected inside the archive at `url`.
    pub wads: Vec<SweepWad>,
}

/// Build the §6.5 sweep corpus from `idgames-wads.jsonl` records: keeps
/// entries with a usable fetch (`Ok`/`FullDownload`) and at least one
/// `.wad` member, sorted and deduped by `id`. On a duplicate `id` the
/// *first* record in sort order wins (`Vec::dedup_by_key` keeps the first
/// of a run) — the opposite of the writers' `BTreeMap` "last wins"
/// convention, but harmless here since the sole input,
/// `idgames-wads.jsonl`, is itself already deduped by `id` before this
/// function ever sees it.
///
/// # Errors
/// [`crate::zips::range_reader::entry_url`] fails to build a URL for a
/// record's `dir`/`filename` (a malformed name that escapes the mirror
/// base).
pub fn sweep_entries(records: &[WadRecord]) -> anyhow::Result<Vec<SweepEntry>> {
    let mut out = Vec::new();
    for rec in records {
        if !matches!(
            rec.fetch_status,
            FetchStatus::Ok | FetchStatus::FullDownload
        ) || rec.wads.is_empty()
        {
            continue;
        }
        // Deterministic URL: always MIRRORS[0], independent of which mirror
        // served the harvest (§6.5) — entry_url percent-encodes and guards
        // against host-escaping names.
        let url = crate::zips::range_reader::entry_url(
            crate::mirror::MIRRORS[0].base,
            &rec.dir,
            &rec.filename,
        )?;
        out.push(SweepEntry {
            id: rec.id,
            url: url.to_string(),
            wads: rec
                .wads
                .iter()
                .map(|w| SweepWad {
                    name: w.name.clone(),
                    uncompressed: w.uncompressed,
                })
                .collect(),
        });
    }
    out.sort_by_key(|e| e.id);
    out.dedup_by_key(|e| e.id);
    Ok(out)
}

/// Write `data/sweep-corpus.jsonl`: one compact JSON object per line, in
/// the order given. [`sweep_entries`] already sorts and dedupes by `id`,
/// so writing its output is byte-identical across reruns against
/// unchanged inputs (§6.5, §9.3).
///
/// # Errors
/// Serialization or filesystem failure.
pub fn write_sweep_jsonl(path: &Path, entries: &[SweepEntry]) -> anyhow::Result<u64> {
    let mut out = String::new();
    for entry in entries {
        out.push_str(&serde_json::to_string(entry).context("serializing sweep entry")?);
        out.push('\n');
    }
    atomic_write(path, out.as_bytes()).with_context(|| format!("writing {}", path.display()))?;
    Ok(u64::try_from(entries.len()).expect("entry count fits u64"))
}

/// One `data/outliers-wads.jsonl` line (spec §7): a curated non-idgames
/// megawad (§6.4), analyzed with the same central-directory-only machinery
/// as [`WadRecord`] over [`crate::zips::url_source::UrlRanges`] instead of
/// the mirror pool. Reuses [`WadMember`]/[`FetchStatus`] verbatim — the only
/// free text is our own curated `slug`, not author-supplied (ADR-0030 §3).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlierRecord {
    /// Our own curated identifier (`xtask/outliers.toml`'s `slug`), not
    /// author free text.
    pub slug: String,
    /// Source URL, as given in `xtask/outliers.toml`.
    pub url: String,
    /// Size discovered via [`crate::zips::url_source::UrlRanges::discover_size`]
    /// (§6.4): the HEAD probe's `Content-Length` when the host answers one
    /// usefully, or — for a host whose HEAD carries no usable
    /// `Content-Length` — the ranged-GET `Content-Range: bytes 0-0/TOTAL`
    /// fallback probe's `TOTAL`. `0` when discovery itself failed — no size
    /// is known for that entry.
    pub zip_size: u64,
    /// ZIP64 EOCD locator present (§5.3).
    pub zip64: bool,
    /// Count of *distinct* central-directory entries by name (§5.6 caveat
    /// applies here too).
    pub member_count: u64,
    /// `.wad` members, case-insensitively matched (§5.5).
    pub wads: Vec<WadMember>,
    /// Every non-`.wad` member name.
    pub other_members: Vec<String>,
    /// Closed §5.6 outcome enum, minus the fallback-only variants (§6.4:
    /// outliers never take the full-download fallback).
    pub fetch_status: FetchStatus,
}

/// Write `data/outliers-wads.jsonl`: records sorted by `slug`, deduped by
/// `slug` (last occurrence wins), one compact JSON object per line —
/// mirrors [`write_wads_jsonl`]'s convention with `slug` standing in for
/// `id` (outliers have no numeric id).
///
/// # Errors
/// Serialization or filesystem failure.
pub fn write_outliers_jsonl(path: &Path, records: Vec<OutlierRecord>) -> anyhow::Result<u64> {
    let mut by_slug = std::collections::BTreeMap::new();
    for rec in records {
        by_slug.insert(rec.slug.clone(), rec);
    }
    let mut out = String::new();
    for rec in by_slug.values() {
        out.push_str(&serde_json::to_string(rec).context("serializing outlier record")?);
        out.push('\n');
    }
    atomic_write(path, out.as_bytes()).with_context(|| format!("writing {}", path.display()))?;
    Ok(u64::try_from(by_slug.len()).expect("record count fits u64"))
}

/// Read a previous run's `outliers-wads.jsonl` (§6.4). `None` when the file
/// is missing/unreadable; unparseable lines are skipped with a warning
/// rather than failing the read, mirroring [`read_wads_jsonl`].
pub fn read_outliers_jsonl(path: &Path) -> Option<Vec<OutlierRecord>> {
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
        tracing::warn!(skipped, path = %path.display(), "skipped unparseable outlier lines");
    }
    Some(records)
}

/// `data/outliers-manifest.json` (spec §7): the [`ZipsManifest`] shape minus
/// the mirror-pool/fallback fields that don't apply to a single-URL,
/// no-fallback source (§6.4 — spec §2.2's locked "no full-download fallback
/// for outliers" decision). The only phase-3-adjacent file carrying
/// wall-clock data — it logs a network run, unlike the timestamp-free
/// `stats` trio (§9.3).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutliersManifest {
    /// `"harvest-outliers-YYYYMMDDTHHMMSSZ"` from the run start.
    pub id: String,
    /// RFC 3339 run start.
    pub started_at: String,
    /// Wall-clock run duration.
    pub duration_secs: u64,
    /// `CARGO_PKG_VERSION` of xtask.
    pub tool_version: String,
    /// `git rev-parse --short HEAD`, when available.
    pub git_rev: Option<String>,
    /// `--limit` value for a dev-scoped run (`--root` does not apply here).
    pub limit: Option<u64>,
    /// `xtask/outliers.toml` entries in this run's worklist (after `--limit`).
    pub entries_total: u64,
    /// `outliers-wads.jsonl` lines written.
    pub records_written: u64,
    /// `outliers-errors.jsonl` lines written.
    pub ledger_count: u64,
    /// Ranged/HEAD requests issued this run.
    pub range_requests: u64,
    /// Total response-body bytes read this run.
    pub bytes_transferred: u64,
    /// Record count per `fetch_status` wire value.
    pub status_counts: std::collections::BTreeMap<String, u64>,
}

/// Write the outliers manifest as pretty JSON with a trailing newline.
///
/// # Errors
/// Serialization or filesystem failure.
pub fn write_outliers_manifest(path: &Path, manifest: &OutliersManifest) -> anyhow::Result<()> {
    let mut bytes = serde_json::to_vec_pretty(manifest).context("serializing outliers manifest")?;
    bytes.push(b'\n');
    atomic_write(path, &bytes).with_context(|| format!("writing {}", path.display()))
}

/// Read a previous run's outliers manifest, if present and parseable.
pub fn read_outliers_manifest(path: &Path) -> Option<OutliersManifest> {
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

/// Current `data/stats.json` shape. Bump on any breaking field change so a
/// downstream consumer (report generator, regression diff) can detect it.
pub const STATS_SCHEMA_VERSION: u32 = 1;

/// `data/stats.json` (§6.5): the full Phase-3 statistics document. Owns no
/// wall-clock field anywhere in its tree (§9.3/§7): every provenance fact
/// is either an input manifest's `id`, [`crate::mirror::LsLarMeta`]'s
/// `mirror`/`last_modified`, or this run's own `tool_version`/`git_rev` —
/// so re-running against unchanged inputs is byte-identical.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsJson {
    /// [`STATS_SCHEMA_VERSION`] at write time.
    pub schema_version: u32,
    /// Traceability back to the harvest snapshot this document summarizes (§7).
    pub provenance: StatsProvenance,
    /// §6.1–§6.3 statistics over the idgames population.
    pub idgames: IdgamesStats,
    /// §6.4 curated modern-outliers supplement; `None` only when neither
    /// `data/outliers-wads.jsonl` nor `data/outliers-manifest.json` exists
    /// (`xtask harvest-outliers` was never run against this snapshot).
    pub outliers: Option<OutliersStats>,
    /// §8 constant recommendations, filled in by
    /// [`crate::stats::report::recommendations`] before `stats.json` is
    /// written — never the placeholder empty `vec![]` that `build_stats`
    /// itself starts from (see [`crate::stats::run_with_paths`]).
    pub recommendations: Vec<Recommendation>,
}

/// Run provenance for [`StatsJson`] (§7: "a statistics report that can't be
/// traced to a specific archive snapshot is not defensible as the basis for
/// a production constant").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsProvenance {
    /// [`HarvestManifest::id`] of the Phase-1 snapshot this run read.
    pub phase1_manifest: String,
    /// [`ZipsManifest::id`] of the Phase-2 snapshot this run read.
    pub phase2_manifest: String,
    /// [`OutliersManifest::id`], when the §6.4 supplement is present.
    pub outliers_manifest: Option<String>,
    /// [`crate::mirror::LsLarMeta::mirror`] — which mirror served the
    /// cached ls-laR.gz bootstrap used for the §6.3 zip-size join.
    pub bootstrap_mirror: String,
    /// [`crate::mirror::LsLarMeta::last_modified`], verbatim.
    pub bootstrap_last_modified: Option<String>,
    /// `CARGO_PKG_VERSION` of xtask, for this stats run itself.
    pub tool_version: String,
    /// `git rev-parse --short HEAD`, when available, for this stats run itself.
    pub git_rev: Option<String>,
}

/// One numeric population's core statistics (§6.1): nearest-rank
/// percentiles over a sorted `u64` vector, plus mean/stddev. Empty input
/// yields every field `0`/`0.0` (see `stats::Distribution::from_sorted`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Distribution {
    /// Population size.
    pub n: u64,
    /// Minimum observed value.
    pub min: u64,
    /// 50th percentile (nearest-rank, §6.1).
    pub p50: u64,
    /// 75th percentile.
    pub p75: u64,
    /// 90th percentile.
    pub p90: u64,
    /// 95th percentile.
    pub p95: u64,
    /// 99th percentile.
    pub p99: u64,
    /// 99.5th percentile.
    #[serde(rename = "p99.5")]
    pub p99_5: u64,
    /// 99.9th percentile.
    #[serde(rename = "p99.9")]
    pub p99_9: u64,
    /// Maximum observed value.
    pub max: u64,
    /// Arithmetic mean.
    pub mean: f64,
    /// Population standard deviation.
    pub stddev: f64,
}

/// A vote-weighted [`Distribution`] alongside the plain one it sits beside
/// (§6.2: "report the unweighted version alongside so the skew is
/// visible").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightedDistribution {
    /// Weighted percentiles/mean/stddev; `min`/`max`/`n` describe the
    /// *value* domain of the weighted population (unweighted extremes and
    /// count), not vote totals.
    pub core: Distribution,
    /// Sum of `votes` across every member the weighted population includes.
    pub total_votes: u64,
    /// Count of `.wad` members excluded because their parent record's
    /// `votes` was `0` (a zero weight would be meaningless in a
    /// vote-weighted percentile).
    pub zero_vote_members_excluded: u64,
}

/// One log2 histogram bucket (§6.1), the struct form of
/// `stats::percentiles::log2_histogram`'s `(String, u64)` tuples.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramBucket {
    /// `"0"` or `"2^k-2^(k+1)"` — see
    /// [`crate::stats::percentiles::log2_histogram`].
    pub label: String,
    /// Count of values falling in this bucket.
    pub count: u64,
}

/// A full §6.1/§6.2 size population: core distribution, its histogram, the
/// vote-weighted variant, and the §6.2 bucket/year segmentations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizeStats {
    /// Unweighted [`Distribution`] over the population.
    pub core: Distribution,
    /// Log2 histogram over the same population.
    pub histogram: Vec<HistogramBucket>,
    /// Vote-weighted variant (§6.2).
    pub weighted: WeightedDistribution,
    /// Segmented by [`crate::stats::top_bucket`] (§6.2).
    pub by_bucket: std::collections::BTreeMap<String, Distribution>,
    /// Segmented by [`crate::stats::year_of`] (§6.2).
    pub by_year: std::collections::BTreeMap<String, Distribution>,
}

/// How far the idgames API's `size` field (§5.0) disagrees with the ls-laR
/// mirror listing, over WAD-bearing population entries where the join hit
/// (§6.3: "a sanity check on how badly the API's `size` field would have
/// misled this decision").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiDelta {
    /// Entries where the ls-laR join found a listing to compare against.
    pub entries_compared: u64,
    /// Of those, entries where the listing size disagreed with the API size.
    pub mismatched: u64,
    /// Largest absolute byte delta among mismatches.
    pub max_abs_delta: u64,
    /// 50th percentile absolute byte delta among mismatches (nearest-rank).
    pub p50_abs_delta: u64,
    /// 99th percentile absolute byte delta among mismatches (nearest-rank).
    pub p99_abs_delta: u64,
    /// Largest `|delta| / listing` ratio among mismatches.
    pub max_relative: f64,
}

/// Zip-size population (§6.3/§8.1: the wire cap bounds zip uploads, which
/// are always WAD-bearing): listing size where the ls-laR join hits, API
/// `size` on a miss, over WAD-bearing population entries only.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZipSizeStats {
    /// Core distribution over the resolved (listing-or-API) size.
    pub core: Distribution,
    /// Log2 histogram over the same population.
    pub histogram: Vec<HistogramBucket>,
    /// API-vs-listing agreement over the join hits (§5.0 guard, §6.3).
    pub api_delta: ApiDelta,
}

/// A compression-ratio population's percentiles (§6.3): `uncompressed /
/// compressed`, nearest-rank via `stats::percentiles::ratio_at`. Smaller
/// field set than [`Distribution`] — no mean/stddev, no p75/p95/p99.5/p99.9
/// (the ratio populations don't need that resolution).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatioDistribution {
    /// Population size.
    pub n: u64,
    /// Minimum ratio.
    pub min: f64,
    /// 50th percentile ratio.
    pub p50: f64,
    /// 90th percentile ratio.
    pub p90: f64,
    /// 99th percentile ratio.
    pub p99: f64,
    /// Maximum ratio.
    pub max: f64,
}

/// §6.3 compression-ratio statistics: per-member (deflate-only — `stored`
/// members carry no meaningful ratio) and per-entry (aggregate `Σ
/// uncompressed / Σ compressed` across every member of the entry).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatioStats {
    /// Ratio distribution over individual `deflate`-method `.wad` members.
    pub member_deflate: RatioDistribution,
    /// Ratio distribution over per-entry `Σ uncompressed / Σ compressed`,
    /// summed across every `.wad` member of the entry regardless of method
    /// — not literally "every member of the entry": non-`.wad` archive
    /// members carry no size data to sum (§5.6 `other_members`), and a
    /// member counted in `zero_compressed_anomalies` (compressed `0`,
    /// uncompressed `> 0`) is excluded from this sum too, the same as it is
    /// from `member_deflate`.
    pub per_entry: RatioDistribution,
    /// Count of `.wad` members with `compressed == 0 && uncompressed > 0` —
    /// a ratio can't be computed, so these are excluded from both
    /// populations above rather than reported as an infinite ratio.
    pub zero_compressed_anomalies: u64,
}

/// §6.3 decision-driving entry-level counts and distributions, over the
/// idgames population (`fetch_status` `Ok`/`FullDownload`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryStats {
    /// Population size (equal to [`Coverage::population_entries`]).
    pub zip_entries: u64,
    /// Entries with no `.wad` member — the archive-support gap even a zip
    /// path can't close (§6.3).
    pub zero_wad: u64,
    /// `zero_wad / zip_entries`, or `0.0` when the population is empty.
    pub zero_wad_share: f64,
    /// Entries with more than one `.wad` member — sizes the picker UX (§6.3/§8.3).
    pub multi_wad: u64,
    /// `multi_wad / zip_entries`, or `0.0` when the population is empty.
    pub multi_wad_share: f64,
    /// Distribution of [`WadRecord::member_count`] over the population.
    pub member_count: Distribution,
    /// Distribution of per-entry `Σ wads[].uncompressed` — the §8.3 "max
    /// total declared uncompressed bytes per entry" source statistic.
    pub entry_wad_total_uncompressed: Distribution,
    /// §6.3 compression-ratio populations.
    pub ratios: RatioStats,
    /// `.wad` member count per [`WadMember::method`] label.
    pub methods: std::collections::BTreeMap<String, u64>,
    /// Population entries with `zip64: true` — confirms whether §5.3
    /// handling was load-bearing (§6.3).
    pub zip64_entries: u64,
    /// `.wad` members with `encrypted: true`, across the population.
    pub encrypted_members: u64,
    /// `other_members` names (across the population) that end in `.wad`
    /// case-insensitively — a diagnostic count of members that look
    /// WAD-shaped but were never counted as one (see [`WadRecord::other_members`]).
    pub wad_named_other_members: u64,
}

/// §6/coverage bookkeeping: how much of the Phase-1/Phase-2 output this run
/// actually saw, and how the idgames population was carved out of it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coverage {
    /// Phase-1 `harvest-manifest.json` `file_count` for this snapshot —
    /// unaffected by stats' own `--root`/`--limit`; the records this run
    /// loaded are `Σ status_counts`.
    pub phase1_files: u64,
    /// Record count per `fetch_status` wire value, over every loaded record
    /// (not just the population) — mirrors [`ZipsManifest::status_counts`]'s
    /// convention.
    pub status_counts: std::collections::BTreeMap<String, u64>,
    /// Phase-1 `harvest-errors.jsonl` entry count per [`LedgerKind`] wire value.
    pub ledger_kinds: std::collections::BTreeMap<String, u64>,
    /// WAD-bearing population entries whose `(dir, filename)` was absent
    /// from the ls-laR listing tree — the zip-size population fell back to
    /// the API `size` for these.
    pub listing_misses: u64,
    /// Records with `fetch_status` `Ok`/`FullDownload` — the §6
    /// "unit of analysis is one `.wad`" population's entry count.
    pub population_entries: u64,
    /// Total `.wad` members across `population_entries`.
    pub population_wads: u64,
}

/// §6.1–§6.3 statistics over the idgames corpus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdgamesStats {
    /// §6 population/coverage bookkeeping.
    pub coverage: Coverage,
    /// §6.1/§6.2: `wads[].uncompressed` core distribution, histogram,
    /// vote-weighted variant, and bucket/year segmentations.
    pub wad_uncompressed: SizeStats,
    /// §6.3/§8.1: zip-size population (the wire-cap source statistic).
    pub zip_size_listing: ZipSizeStats,
    /// §6.3: decision-driving entry-level counts and distributions.
    pub entries: EntryStats,
}

/// One analyzed §6.4 outlier: the same shape [`OutlierRecord`] carries,
/// reduced to the numbers the report needs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlierSummary {
    /// [`OutlierRecord::slug`].
    pub slug: String,
    /// [`OutlierRecord::zip_size`].
    pub zip_size: u64,
    /// [`OutlierRecord::member_count`].
    pub member_count: u64,
    /// `wads.len()`.
    pub wad_count: u64,
    /// `max(wads[].uncompressed)`, `0` when `wad_count` is `0`.
    pub max_wad_uncompressed: u64,
    /// `Σ wads[].uncompressed`.
    pub total_wad_uncompressed: u64,
}

/// One §6.4 outlier that could not be analyzed (`fetch_status` other than `Ok`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlierSkip {
    /// [`OutlierRecord::slug`].
    pub slug: String,
    /// [`FetchStatus`] wire value.
    pub fetch_status: String,
}

/// §6.4 modern-outliers supplement, reported separately from the idgames
/// population "so the bias stays visible".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutliersStats {
    /// Successfully analyzed entries, sorted by slug.
    pub analyzed: Vec<OutlierSummary>,
    /// Entries that could not be analyzed, sorted by slug.
    pub skipped: Vec<OutlierSkip>,
    /// [`Distribution`] over every analyzed entry's `.wad` member
    /// uncompressed sizes (flattened).
    pub wad_uncompressed: Distribution,
    /// `max(analyzed[].zip_size)`, `0` when `analyzed` is empty.
    pub max_zip_size: u64,
    /// `max(analyzed[].member_count)`, `0` when `analyzed` is empty.
    pub max_member_count: u64,
    /// `max(analyzed[].total_wad_uncompressed)`, `0` when `analyzed` is empty.
    pub max_entry_total_uncompressed: u64,
}

/// One §8 constant recommendation, built by
/// [`crate::stats::report::recommendations`] and rendered verbatim by
/// [`crate::stats::report::render_report`] — see
/// [`StatsJson::recommendations`] for how it reaches `stats.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    /// Which §8 constant this recommends a value for, e.g. `"wire_cap_zip"`.
    pub key: String,
    /// Human-readable recommended value (may carry units).
    pub recommended: String,
    /// The recommended value as a plain number, when it is one.
    pub value: Option<u64>,
    /// The statistic/formula the recommendation was derived from.
    pub formula: String,
    /// Which [`StatsJson`] field the `formula` reads.
    pub source: String,
}

/// Write `data/stats.json` as pretty JSON with a trailing newline —
/// mirrors [`write_manifest`]'s convention. Timestamp-free by construction
/// (see [`StatsJson`]'s doc comment), so a rerun against unchanged inputs
/// is byte-identical (§9.3).
///
/// # Errors
/// Serialization or filesystem failure.
pub fn write_stats_json(path: &Path, stats: &StatsJson) -> anyhow::Result<()> {
    let mut bytes = serde_json::to_vec_pretty(stats).context("serializing stats.json")?;
    bytes.push(b'\n');
    atomic_write(path, &bytes).with_context(|| format!("writing {}", path.display()))
}

#[cfg(test)]
pub(crate) mod tests {
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
    fn ledger_ties_on_partial_key_are_still_deterministic() {
        // Entries agreeing on (path, kind, detail) but differing in
        // attempts/action must not fall back to insertion order.
        let tmp = tempfile::tempdir().unwrap();
        let a_path = tmp.path().join("a.jsonl");
        let b_path = tmp.path().join("b.jsonl");
        let first = LedgerEntry {
            path: "levels/x/".into(),
            action: "getcontents".into(),
            kind: LedgerKind::HttpError,
            detail: "HTTP 500".into(),
            attempts: 2,
        };
        let second = LedgerEntry {
            attempts: 6,
            ..first.clone()
        };
        write_ledger(&a_path, vec![first.clone(), second.clone()]).unwrap();
        write_ledger(&b_path, vec![second, first]).unwrap();
        assert_eq!(
            std::fs::read(&a_path).unwrap(),
            std::fs::read(&b_path).unwrap()
        );
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
        let v = serde_json::to_value(LedgerKind::Skipped).unwrap();
        assert_eq!(v, "skipped");
    }

    #[test]
    fn ledger_roundtrips_through_reader() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("harvest-errors.jsonl");
        let entry = LedgerEntry {
            path: "levels/a/".into(),
            action: "getcontents".into(),
            kind: LedgerKind::HttpError,
            detail: "HTTP 500".into(),
            attempts: 2,
        };
        write_ledger(&p, vec![entry]).unwrap();
        let back = read_ledger(&p).unwrap();
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].path, "levels/a/");
        assert!(read_ledger(&tmp.path().join("missing.jsonl")).is_none());
        // A corrupt line is skipped, not fatal.
        std::fs::write(&p, "{ not json\n").unwrap();
        assert_eq!(read_ledger(&p).map(|v| v.len()), Some(0));
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

    /// Full-shape `WadRecord` (wads + `other_members` populated) used by
    /// the writer-focused tests below.
    fn sample_wad_record(id: u64) -> WadRecord {
        WadRecord {
            id,
            dir: "levels/doom2/Ports/megawads/".into(),
            filename: format!("f{id}.zip"),
            zip_size: 3_145_728,
            date: "2019-04-02".into(),
            rating: Some(4.61),
            votes: 38,
            is_zip: true,
            zip64: false,
            member_count: 3,
            wads: vec![WadMember {
                name: "EXAMPLE.WAD".into(),
                compressed: 3_102_841,
                uncompressed: 14_680_064,
                method: "deflate".into(),
                encrypted: false,
            }],
            other_members: vec!["EXAMPLE.TXT".into(), "README.MD".into()],
            mirror: "infania".into(),
            fetch_status: FetchStatus::Ok,
        }
    }

    /// Minimal `WadMember` builder for the sweep-corpus tests.
    fn wad_member(name: &str, compressed: u64, uncompressed: u64) -> WadMember {
        WadMember {
            name: name.into(),
            compressed,
            uncompressed,
            method: "deflate".into(),
            encrypted: false,
        }
    }

    /// Minimal `WadRecord` builder for the sweep-corpus filter/sort/URL
    /// tests: plausible defaults, caller-controlled `fetch_status`/`wads`.
    fn wad_record(id: u64, fetch_status: FetchStatus, wads: Vec<WadMember>) -> WadRecord {
        let member_count = wads.len() as u64;
        WadRecord {
            id,
            dir: "levels/doom/".into(),
            filename: format!("example{id}.zip"),
            zip_size: 1_048_576,
            date: "2019-04-02".into(),
            rating: None,
            votes: 0,
            is_zip: true,
            zip64: false,
            member_count,
            wads,
            other_members: vec![],
            mirror: "infania".into(),
            fetch_status,
        }
    }

    /// Shared assertion: every object key anywhere in `v` is in
    /// `allowlist` (ADR-0030 §3 no-free-text rule). Task 7 reuses this for
    /// `stats.json`.
    pub(crate) fn assert_keys_within(v: &serde_json::Value, allowlist: &[&str]) {
        match v {
            serde_json::Value::Object(map) => {
                for (k, val) in map {
                    assert!(
                        allowlist.contains(&k.as_str()),
                        "key {k:?} not in allowlist {allowlist:?}"
                    );
                    assert_keys_within(val, allowlist);
                }
            }
            serde_json::Value::Array(items) => {
                for item in items {
                    assert_keys_within(item, allowlist);
                }
            }
            _ => {}
        }
    }

    #[test]
    fn wads_jsonl_is_sorted_deduped_and_matches_design_5_6() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("idgames-wads.jsonl");
        let n = write_wads_jsonl(
            &p,
            vec![
                sample_wad_record(9),
                sample_wad_record(3),
                sample_wad_record(9),
            ],
        )
        .unwrap();
        assert_eq!(n, 2);
        let text = std::fs::read_to_string(&p).unwrap();
        let first: serde_json::Value = serde_json::from_str(text.lines().next().unwrap()).unwrap();
        assert_eq!(first["id"], 3);
        // §5.6 field presence, exact names.
        for key in [
            "id",
            "dir",
            "filename",
            "zip_size",
            "date",
            "rating",
            "votes",
            "is_zip",
            "zip64",
            "member_count",
            "wads",
            "other_members",
            "mirror",
            "fetch_status",
        ] {
            assert!(first.get(key).is_some(), "missing §5.6 key {key}");
        }
        assert_eq!(first["wads"][0]["uncompressed"], 14_680_064_u64);
        assert_eq!(first["fetch_status"], "ok");
    }

    #[test]
    fn wads_jsonl_reruns_are_byte_identical() {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a.jsonl");
        let b = tmp.path().join("b.jsonl");
        write_wads_jsonl(&a, vec![sample_wad_record(2), sample_wad_record(7)]).unwrap();
        write_wads_jsonl(&b, vec![sample_wad_record(7), sample_wad_record(2)]).unwrap();
        assert_eq!(std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());
    }

    #[test]
    fn wads_jsonl_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("idgames-wads.jsonl");
        let rec = sample_wad_record(7);
        write_wads_jsonl(&path, vec![rec.clone()]).unwrap();
        let back = read_wads_jsonl(&path).unwrap();
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].id, 7);
        assert!(read_wads_jsonl(&dir.path().join("missing.jsonl")).is_none());
        // A corrupt line is skipped, not fatal.
        std::fs::write(&path, "{ not json\n").unwrap();
        assert_eq!(read_wads_jsonl(&path).map(|v| v.len()), Some(0));
    }

    #[test]
    fn parse_wads_jsonl_skips_garbage_lines() {
        let a = serde_json::to_string(&sample_wad_record(2)).unwrap();
        let b = serde_json::to_string(&sample_wad_record(7)).unwrap();
        let text = format!("{a}\nnot json\n{b}\n");
        let path = Path::new("idgames-wads.jsonl");
        let records = parse_wads_jsonl(&text, path);
        assert_eq!(records.len(), 2);
        assert_eq!(records.iter().map(|r| r.id).collect::<Vec<_>>(), vec![2, 7]);
    }

    #[test]
    fn fetch_status_serializes_the_closed_snake_case_enum() {
        // §5.6: closed enum, every §5.5 edge case named.
        let cases = [
            (FetchStatus::Ok, "ok"),
            (FetchStatus::NotZip, "not_zip"),
            (FetchStatus::Mirror404All, "mirror_404_all"),
            (FetchStatus::NoRangeSupport, "no_range_support"),
            (FetchStatus::FullDownload, "full_download"),
            (FetchStatus::ZipParseError, "zip_parse_error"),
            (FetchStatus::FetchError, "fetch_error"),
            (FetchStatus::SkippedKnownDead, "skipped_known_dead"),
        ];
        for (status, wire) in cases {
            assert_eq!(serde_json::to_value(status).unwrap(), wire);
        }
    }

    #[test]
    fn zips_manifest_roundtrips_and_writes_trailing_newline() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("wads-manifest.json");
        let m = ZipsManifest {
            id: "harvest-zips-20260816T120000Z".into(),
            started_at: "2026-08-16T12:00:00+00:00".into(),
            duration_secs: 1800,
            tool_version: tool_version(),
            git_rev: git_rev(),
            scoped_root: None,
            limit: None,
            entries_total: 21_375,
            records_written: 21_375,
            ledger_count: 12,
            cache_hits: 20_000,
            live_entries: 1_375,
            range_requests: 2_923,
            bytes_transferred: 190_000_000,
            full_downloads: 2,
            fallback_bytes: 40_000_000,
            zip64_entries: 1,
            status_counts: std::collections::BTreeMap::from([("ok".to_owned(), 21_300)]),
            unaccounted_entries: 0,
            aborted: None,
        };
        write_zips_manifest(&p, &m).unwrap();
        let text = std::fs::read_to_string(&p).unwrap();
        assert!(text.ends_with('\n'));
        let back: ZipsManifest = serde_json::from_str(&text).unwrap();
        assert_eq!(back.entries_total, 21_375);
        assert_eq!(back.zip64_entries, 1);
        assert_eq!(back.aborted, None);
    }

    #[test]
    fn zips_manifest_roundtrips_through_reader() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("wads-manifest.json");
        let m = ZipsManifest {
            id: "harvest-zips-20260816T120000Z".into(),
            started_at: "2026-08-16T12:00:00+00:00".into(),
            duration_secs: 1800,
            tool_version: tool_version(),
            git_rev: git_rev(),
            scoped_root: None,
            limit: None,
            entries_total: 10,
            records_written: 10,
            ledger_count: 0,
            cache_hits: 0,
            live_entries: 10,
            range_requests: 10,
            bytes_transferred: 1_000,
            full_downloads: 0,
            fallback_bytes: 0,
            zip64_entries: 0,
            status_counts: std::collections::BTreeMap::from([("ok".to_owned(), 10)]),
            unaccounted_entries: 0,
            aborted: None,
        };
        write_zips_manifest(&p, &m).unwrap();
        let back = read_zips_manifest(&p).unwrap();
        assert_eq!(back.id, m.id);
        assert_eq!(back.entries_total, 10);
        assert!(read_zips_manifest(&tmp.path().join("missing.json")).is_none());
    }

    #[test]
    fn sweep_entries_filter_sort_and_url() {
        // ok-with-wads: in. full_download-with-wads: in. ok-zero-wads: out.
        // zip_parse_error: out. Sorted by id regardless of input order.
        let recs = vec![
            wad_record(9, FetchStatus::Ok, vec![wad_member("B.WAD", 10, 100)]),
            wad_record(
                3,
                FetchStatus::FullDownload,
                vec![wad_member("A.WAD", 5, 50)],
            ),
            wad_record(1, FetchStatus::Ok, vec![]),
            wad_record(
                2,
                FetchStatus::ZipParseError,
                vec![wad_member("X.WAD", 1, 1)],
            ),
        ];
        let entries = sweep_entries(&recs).unwrap();
        assert_eq!(entries.iter().map(|e| e.id).collect::<Vec<_>>(), [3, 9]);
        assert_eq!(
            entries[1].url,
            "https://ftpmirror1.infania.net/pub/idgames/levels/doom/example9.zip"
        );
        assert_eq!(entries[1].wads[0].uncompressed, 100);
    }

    #[test]
    fn sweep_entries_dedup_keeps_first_of_duplicate_id() {
        // Two Ok-with-wads records sharing an id: `dedup_by_key` keeps the
        // FIRST of a run (opposite of the writers' BTreeMap "last wins"
        // convention) — fine here because the sole input,
        // idgames-wads.jsonl, is already deduped by id before this
        // function ever sees it.
        let recs = vec![
            wad_record(5, FetchStatus::Ok, vec![wad_member("FIRST.WAD", 1, 111)]),
            wad_record(5, FetchStatus::Ok, vec![wad_member("SECOND.WAD", 2, 222)]),
        ];
        let entries = sweep_entries(&recs).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].wads[0].name, "FIRST.WAD");
        assert_eq!(entries[0].wads[0].uncompressed, 111);
    }

    #[test]
    fn sweep_jsonl_has_no_free_text_fields() {
        // serialize and walk keys: allowlist only (ADR-0030 §3).
        let entry = SweepEntry {
            id: 1,
            url: "https://x/e.zip".into(),
            wads: vec![SweepWad {
                name: "E.WAD".into(),
                uncompressed: 4,
            }],
        };
        let val: serde_json::Value = serde_json::to_value(&entry).unwrap();
        assert_keys_within(&val, &["id", "url", "wads", "name", "uncompressed"]);
    }

    #[test]
    fn sweep_jsonl_writes_compact_lines_and_returns_count() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("sweep-corpus.jsonl");
        let recs = vec![
            wad_record(9, FetchStatus::Ok, vec![wad_member("B.WAD", 10, 100)]),
            wad_record(
                3,
                FetchStatus::FullDownload,
                vec![wad_member("A.WAD", 5, 50)],
            ),
        ];
        let entries = sweep_entries(&recs).unwrap();
        let n = write_sweep_jsonl(&p, &entries).unwrap();
        assert_eq!(n, 2);
        let text = std::fs::read_to_string(&p).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2);
        // No pretty-printing: each line is a single compact JSON object.
        assert!(!lines[0].contains('\n'));
        let first: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(first["id"], 3);
    }

    /// Minimal `OutlierRecord` builder for the writer tests.
    fn outlier_record(slug: &str) -> OutlierRecord {
        OutlierRecord {
            slug: slug.to_owned(),
            url: format!("https://example.com/{slug}.zip"),
            zip_size: 1_048_576,
            zip64: false,
            member_count: 1,
            wads: vec![WadMember {
                name: "MAP01.WAD".into(),
                compressed: 900_000,
                uncompressed: 4_000_000,
                method: "deflate".into(),
                encrypted: false,
            }],
            other_members: vec!["README.TXT".into()],
            fetch_status: FetchStatus::Ok,
        }
    }

    #[test]
    fn outliers_jsonl_is_sorted_deduped_by_slug() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("outliers-wads.jsonl");
        let n = write_outliers_jsonl(
            &p,
            vec![
                outlier_record("b"),
                outlier_record("a"),
                outlier_record("b"),
            ],
        )
        .unwrap();
        assert_eq!(n, 2);
        let text = std::fs::read_to_string(&p).unwrap();
        let slugs: Vec<String> = text
            .lines()
            .map(|l| {
                serde_json::from_str::<serde_json::Value>(l).unwrap()["slug"]
                    .as_str()
                    .unwrap()
                    .to_owned()
            })
            .collect();
        assert_eq!(slugs, vec!["a".to_owned(), "b".to_owned()]);
    }

    #[test]
    fn outliers_jsonl_round_trips() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("outliers-wads.jsonl");
        write_outliers_jsonl(&p, vec![outlier_record("simons-destiny")]).unwrap();
        let back = read_outliers_jsonl(&p).unwrap();
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].slug, "simons-destiny");
        assert!(read_outliers_jsonl(&tmp.path().join("missing.jsonl")).is_none());
        // A corrupt line is skipped, not fatal.
        std::fs::write(&p, "{ not json\n").unwrap();
        assert_eq!(read_outliers_jsonl(&p).map(|v| v.len()), Some(0));
    }

    #[test]
    fn read_outliers_jsonl_skips_unparseable_lines_and_keeps_the_rest() {
        // #442: a corrupt line yields a silently partial population — pin
        // the skip-don't-fail contract (and the blank-line tolerance)
        // explicitly so a future "fail fast" refactor is a conscious choice.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("outliers-wads.jsonl");
        let good = serde_json::json!({
            "slug": "a", "url": "https://x/a.zip", "zip_size": 1, "zip64": false,
            "member_count": 0, "wads": [], "other_members": [], "fetch_status": "ok"
        });
        std::fs::write(&path, format!("{good}\nnot json\n\n{good}\n")).unwrap();
        let records = read_outliers_jsonl(&path).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].slug, "a");
        // Missing file stays None, not a panic or an empty Some.
        assert!(read_outliers_jsonl(&dir.path().join("absent.jsonl")).is_none());
    }

    #[test]
    fn outliers_manifest_roundtrips_and_writes_trailing_newline() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("outliers-manifest.json");
        let m = OutliersManifest {
            id: "harvest-outliers-20260817T000000Z".into(),
            started_at: "2026-08-17T00:00:00+00:00".into(),
            duration_secs: 12,
            tool_version: tool_version(),
            git_rev: git_rev(),
            limit: None,
            entries_total: 6,
            records_written: 6,
            ledger_count: 1,
            range_requests: 12,
            bytes_transferred: 2_000_000,
            status_counts: std::collections::BTreeMap::from([("ok".to_owned(), 5)]),
        };
        write_outliers_manifest(&p, &m).unwrap();
        let text = std::fs::read_to_string(&p).unwrap();
        assert!(text.ends_with('\n'));
        let back = read_outliers_manifest(&p).unwrap();
        assert_eq!(back.id, m.id);
        assert_eq!(back.entries_total, 6);
        assert!(read_outliers_manifest(&tmp.path().join("missing.json")).is_none());
    }
}
