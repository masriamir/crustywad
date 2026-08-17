//! Phase-2 orchestrator: true WAD sizes via HTTP range reads (DESIGN.md §5).
//!
//! Input is Phase 1's `idgames-files.jsonl` (§9.3: every entry there ends
//! this phase with a record or a ledger entry — `unaccounted_entries` in
//! the manifest witnesses 0). Entries run through a [`MIRROR_CONCURRENCY`]-
//! wide pool; results land in the per-id log (resumable, §5.4) and the
//! §5.6 outputs. The §5.2 fallback breaker — and the runaway range-path
//! byte ceiling below it — abort the phase by writing the partial outputs
//! and an `aborted` manifest, then failing the process so `just harvest`
//! never chains into stats on a poisoned run.
//!
//! **Cache-scoping policy (§5.4, fix round 1 / I2):** only a *conclusive*
//! outcome — a fact about the archive itself — is written into the
//! resumable per-id log: `Ok`, `FullDownload`, `Mirror404All`,
//! `ZipParseError`. `NoRangeSupport` and `FetchError` are properties of
//! *this run* (a budget/breaker state, or a transient mirror/transport
//! blip), not the entry, and are deliberately never cached — they retry
//! live on the next run against a fresh budget and mirror state. The §5.4
//! cache exists so a warm rerun does no network work for settled entries;
//! caching a transient failure would defeat that (the cache invalidates
//! only on a `body_hash` change, which a transient failure never causes,
//! so a cached failure would otherwise be permanent). One consequence:
//! `wads-errors.jsonl` is a *per-run* diagnostic, not a cross-run one — a
//! cached conclusive failure (e.g. a `Mirror404All` reused from the log)
//! produces a record on a warm rerun but no fresh ledger line, since the
//! ledger is only populated by *this run's* live failures. The
//! manifest's `status_counts` (derived from every record, cached or live)
//! is the cross-run source of truth for "how many entries are in each
//! state right now."

pub mod inspect;
pub mod range_reader;
pub mod store;

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use anyhow::Context as _;
use chrono::Utc;

use crate::api::model::{FileRecord, normalize_dir};
use crate::schema::{self, FetchStatus, LedgerEntry, LedgerKind, WadRecord, ZipsManifest};
use crate::zips::inspect::{FetchFailure, InspectError, Inspection, RangeSource};
use crate::zips::range_reader::{
    FallbackBudget, FallbackDecision, MIRROR_CONCURRENCY, MirrorRanges, TAIL_LEN, TransferCounters,
};
use crate::zips::store::{ZipsStore, dir_hashes};

/// Controller addition (ADR-0030 §4 runaway-cost posture): a single entry
/// needing the full-download fallback must never be allowed to consume a
/// large slice of the shared [`range_reader::FALLBACK_BYTE_BUDGET`] by
/// itself. An entry over this size is refused outright
/// (`no_range_support`), never charged against the shared budget — see
/// [`handle_no_range_support`].
const FALLBACK_PER_ENTRY_CAP: u64 = 512 * 1024 * 1024;

/// Controller addition (ADR-0030 §4): total range-path bytes (tracked by
/// [`TransferCounters`], independent of [`FallbackBudget`]) this run may
/// transfer before the phase aborts. A pathological EOCD/central-directory
/// shape can push several ×64 MiB tail fetches down the ranged path per
/// entry — bytes the fallback budget never sees at all — so this is a
/// second, independent breaker over the same run.
const RANGE_BYTE_CEILING: u64 = 4 * 1024 * 1024 * 1024;

/// Run phase 2 (`xtask harvest-zips`). `root`/`limit` are the §4.6 dev
/// flags; when either is set the run is scoped: input is read from and
/// outputs are written to `data/dev/`.
///
/// # Errors
/// Environmental failures (missing Phase-1 input, output writes) and the
/// §5.2 fallback circuit breaker / range-path byte ceiling.
pub fn run(root: Option<&str>, limit: Option<usize>) -> anyhow::Result<()> {
    // Current-thread runtime (fix round 1 / M8), not phase 1's
    // multi-thread `Runtime::new()`: the whole worker pool below runs on a
    // `tokio::task::LocalSet` because `EntrySource`'s futures are
    // structurally `!Send` (see `EntrySource`'s doc comment) — a
    // multi-thread runtime's extra worker threads would sit permanently
    // idle here, so building one would be misleading, not just wasteful.
    // Phase 1's runtime is untouched; it has no such constraint.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("building tokio runtime")?;
    let limit = limit.map(|l| u64::try_from(l).unwrap_or(u64::MAX));
    runtime.block_on(run_async(root, limit))
}

async fn run_async(root: Option<&str>, limit: Option<u64>) -> anyhow::Result<()> {
    // Captured before `run_core` runs (fix round 1 / I1) so
    // `patch_manifest_provenance` can tell "the manifest `run_core` just
    // wrote" apart from "a prior run's manifest that a failed-before-any-
    // write `run_core` call left untouched on disk" — `run_core` takes its
    // own `started_at` strictly after this instant, so the `>=` guard
    // there is exact, never a race.
    let run_start = Utc::now();
    let scoped = root.is_some() || limit.is_some();
    let out_dir = crate::phase1::output_dir(scoped);
    let cache_dir = crate::phase1::data_root().join("cache");
    let input_path = out_dir.join("idgames-files.jsonl");
    let Some(phase1_records) = schema::read_files_jsonl(&input_path) else {
        anyhow::bail!(
            "no Phase-1 output at {} — run `xtask harvest-api` first",
            input_path.display()
        );
    };
    let limit_usize = limit.map(|l| usize::try_from(l).unwrap_or(usize::MAX));
    let entries = worklist(phase1_records, root, limit_usize);

    let client = crate::mirror::build_zips_http()?;
    let make_source = move |rec: &FileRecord, counters: Arc<TransferCounters>| -> LiveSource {
        match MirrorRanges::new(client.clone(), &rec.dir, &rec.filename, rec.size, counters) {
            Ok(mirrors) => LiveSource::Mirror(Box::new(mirrors)),
            Err(e) => LiveSource::UrlError(e.to_string()),
        }
    };

    let result = run_core(entries, &out_dir, &cache_dir, make_source).await;
    // Best-effort: the only two provenance fields `run_core` cannot know on
    // its own (it never sees `root`/`limit` — see `run_core`'s doc comment
    // for why) are patched onto the just-written manifest, on both the
    // success and the abort path (the manifest exists either way) — but
    // ONLY when that manifest is actually the one this call just wrote,
    // not a prior run's file left over from an early `run_core` failure
    // (`create_dir_all`/`ApiCache::new`/etc. can all return `Err` before
    // any manifest write happens at all).
    if let Err(e) =
        patch_manifest_provenance(&out_dir.join("wads-manifest.json"), root, limit, run_start)
    {
        tracing::warn!(error = %e, "could not patch harvest-zips manifest provenance");
    }
    let stats = result?;
    tracing::info!(
        cache_hits = stats.cache_hits,
        live_entries = stats.live_entries,
        records = stats.records,
        ledger = stats.ledger,
        "harvest-zips complete"
    );
    Ok(())
}

/// Patch the `scoped_root`/`limit` provenance fields of an already-written
/// manifest. `run_core` is generic over the network layer and is exercised
/// directly by tests with a fixed 4-argument call (§ task-6 brief Step 1)
/// that carries no `root`/`limit` — so it always writes those two fields as
/// `None`/`None`. Only [`run_async`], which actually knows the CLI flags,
/// can fill them in correctly; it does so with a small read-modify-write
/// immediately after `run_core` returns (success or abort — the manifest
/// exists on disk either way).
///
/// Fix round 1 / I1: `run_core` can return `Err` *before ever writing a
/// manifest at all* (`create_dir_all`, `ApiCache::new`, the worker pool's
/// own `?`, `write_wads_jsonl`, `write_ledger` all run before the manifest
/// write). On that path, an unconditional patch would silently relabel a
/// **previous** run's `wads-manifest.json` with the current invocation's
/// `--root`/`--limit` — provenance for a run that never happened. `run_start`
/// (captured in [`run_async`] strictly before `run_core` is called) is the
/// guard: only a manifest whose own `started_at` is `>= run_start` can
/// possibly be the one this call just wrote, since `run_core` always stamps
/// `started_at` with a timestamp taken after `run_start`. An older manifest
/// — or one whose `started_at` fails to parse at all — is left untouched,
/// with a `tracing::warn!` so a silently-skipped patch is still visible.
fn patch_manifest_provenance(
    path: &Path,
    root: Option<&str>,
    limit: Option<u64>,
    run_start: chrono::DateTime<Utc>,
) -> anyhow::Result<()> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let mut manifest: ZipsManifest =
        serde_json::from_slice(&bytes).context("parsing zips manifest")?;
    match chrono::DateTime::parse_from_rfc3339(&manifest.started_at) {
        Ok(started_at) if started_at.with_timezone(&Utc) >= run_start => {
            manifest.scoped_root = root.map(str::to_owned);
            manifest.limit = limit;
            schema::write_zips_manifest(path, &manifest)
        }
        Ok(_) => {
            tracing::warn!(
                path = %path.display(),
                "on-disk manifest predates this run — not patching provenance \
                 (a previous run's file, untouched by this one)"
            );
            Ok(())
        }
        Err(e) => {
            tracing::warn!(
                path = %path.display(),
                error = %e,
                "on-disk manifest's started_at did not parse — not patching provenance"
            );
            Ok(())
        }
    }
}

/// Sorted, scoped, limited worklist from the Phase-1 records.
fn worklist(
    mut records: Vec<FileRecord>,
    root: Option<&str>,
    limit: Option<usize>,
) -> Vec<FileRecord> {
    records.sort_by_key(|r| r.id);
    let mut records = if let Some(root) = root {
        let prefix = normalize_dir(root);
        records
            .into_iter()
            .filter(|r| r.dir.starts_with(&prefix))
            .collect()
    } else {
        records
    };
    if let Some(limit) = limit {
        records.truncate(limit);
    }
    records
}

/// §5.6 record for a non-`.zip` entry — no mirror contact (§5.5).
fn not_zip_record(entry: &FileRecord) -> WadRecord {
    WadRecord {
        id: entry.id,
        dir: entry.dir.clone(),
        filename: entry.filename.clone(),
        zip_size: entry.size,
        date: entry.date.clone(),
        rating: entry.rating,
        votes: entry.votes,
        is_zip: false,
        zip64: false,
        member_count: 0,
        wads: Vec::new(),
        other_members: Vec::new(),
        mirror: String::new(),
        fetch_status: FetchStatus::NotZip,
    }
}

/// What one live entry produced.
#[derive(Debug)]
enum EntryOutcome {
    /// The archive was inspected (over ranges, or via the full-download
    /// fallback — `full_download` distinguishes the two for `fetch_status`).
    Inspected {
        /// The parsed central-directory summary.
        inspection: Inspection,
        /// Whether the bytes came from the §5.2 full-download fallback
        /// rather than ranged reads.
        full_download: bool,
    },
    /// The entry could not be inspected at all.
    Failed(FailKind),
}

/// Failure classification carrying ledger detail.
#[derive(Debug)]
enum FailKind {
    /// Every usable mirror answered 404.
    Mirror404All,
    /// Every mirror refused ranges; `budget_refused` distinguishes a
    /// [`FallbackBudget`] refusal (byte budget exhausted or the ~2%
    /// breaker tripped) from the controller's per-entry cap
    /// ([`FALLBACK_PER_ENTRY_CAP`]), which refuses without ever touching
    /// the shared budget.
    NoRange {
        /// `true`: [`FallbackBudget::admit`] returned `Skip`/`Abort`.
        /// `false`: the entry's size alone exceeded
        /// [`FALLBACK_PER_ENTRY_CAP`], refused before `admit` could even
        /// be asked to grant real budget.
        budget_refused: bool,
    },
    /// Bytes arrived but the zip didn't parse (or a driver guard tripped).
    ZipParse(String),
    /// Transport-level failure after retries on every usable mirror.
    Fetch(String),
}

/// Build the §5.6 record (and, for failures, push the diagnostic ledger
/// line — action `"harvest-zips"`, kinds `HttpError`/`ParseError`).
fn outcome_to_record(
    entry: &FileRecord,
    outcome: EntryOutcome,
    mirror: &str,
    ledger: &mut Vec<LedgerEntry>,
) -> WadRecord {
    let (fetch_status, zip64, member_count, wads, other_members) = match outcome {
        EntryOutcome::Inspected {
            inspection,
            full_download,
        } => {
            let status = if full_download {
                FetchStatus::FullDownload
            } else {
                FetchStatus::Ok
            };
            (
                status,
                inspection.zip64,
                inspection.member_count,
                inspection.wads,
                inspection.other_members,
            )
        }
        EntryOutcome::Failed(fail) => {
            let (fetch_status, kind, detail) = fail_ledger_detail(&fail);
            // `mirror` is non-empty only when some fetch for this entry
            // actually pinned a mirror before the failure that's being
            // ledgered here happened (e.g. a later-round Content-Range
            // mismatch, or a zip-parse failure after a successful full
            // download) — attribute the detail to that mirror when known,
            // rather than leaving every ledger line mirror-silent.
            let detail = if mirror.is_empty() {
                detail
            } else {
                format!("{detail}: last mirror {mirror}")
            };
            ledger.push(LedgerEntry {
                path: format!("{}{}", entry.dir, entry.filename),
                action: "harvest-zips".into(),
                kind,
                detail,
                // Always 1: unlike Phase 1 (which counts per-HTTP-call
                // attempts on the ledger line itself), this line aggregates
                // per *entry* — the real per-mirror retry/failover count
                // for this entry lives inside `MirrorRanges`
                // (§5.2/§5.4's `MAX_MIRROR_ATTEMPTS` per candidate mirror),
                // never surfaced onto the ledger.
                attempts: 1,
            });
            (fetch_status, false, 0, Vec::new(), Vec::new())
        }
    };
    WadRecord {
        id: entry.id,
        dir: entry.dir.clone(),
        filename: entry.filename.clone(),
        zip_size: entry.size,
        date: entry.date.clone(),
        rating: entry.rating,
        votes: entry.votes,
        is_zip: true,
        zip64,
        member_count,
        wads,
        other_members,
        mirror: mirror.to_owned(),
        fetch_status,
    }
}

/// The `(fetch_status, ledger kind, ledger detail)` for one [`FailKind`].
fn fail_ledger_detail(fail: &FailKind) -> (FetchStatus, LedgerKind, String) {
    match fail {
        FailKind::Mirror404All => (
            FetchStatus::Mirror404All,
            LedgerKind::HttpError,
            "404 on all mirrors".to_owned(),
        ),
        FailKind::NoRange { budget_refused } => (
            FetchStatus::NoRangeSupport,
            LedgerKind::HttpError,
            if *budget_refused {
                "no-range fallback refused: budget exhausted or breaker tripped".to_owned()
            } else {
                "entry exceeds per-entry fallback cap".to_owned()
            },
        ),
        FailKind::ZipParse(detail) => (
            FetchStatus::ZipParseError,
            LedgerKind::ParseError,
            detail.clone(),
        ),
        FailKind::Fetch(detail) => (
            FetchStatus::FetchError,
            LedgerKind::HttpError,
            detail.clone(),
        ),
    }
}

/// §9.3 witness: worklist entries with no record.
fn unaccounted(entries: &[FileRecord], records: &[WadRecord]) -> u64 {
    let recorded: std::collections::BTreeSet<u64> = records.iter().map(|r| r.id).collect();
    let n = entries.iter().filter(|e| !recorded.contains(&e.id)).count();
    u64::try_from(n).unwrap_or(u64::MAX)
}

/// Record count per `fetch_status` wire value (§ `ZipsManifest` doc).
fn status_counts(records: &[WadRecord]) -> BTreeMap<String, u64> {
    let mut counts = BTreeMap::new();
    for record in records {
        let label = serde_json::to_value(record.fetch_status)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_default();
        *counts.entry(label).or_insert(0_u64) += 1;
    }
    counts
}

/// `"harvest-zips-YYYYMMDDTHHMMSSZ"` from the run start.
fn zips_manifest_id(started_at: &chrono::DateTime<Utc>) -> String {
    format!("harvest-zips-{}", started_at.format("%Y%m%dT%H%M%SZ"))
}

/// Factory yielding one [`RangeSource`] (plus its mirror-key getter and
/// full-download hook) per entry. Production: [`LiveSource`] over a shared
/// reqwest client + counters. Tests: fixture-backed fakes with a fetch
/// counter.
///
/// Deliberately **not** `: Send` (a deviation from the task-6 brief's
/// sketch, discovered at compile time, not guessed): [`inspect::inspect_zip`]
/// holds a `&Cell<Option<(u64, u64)>>` (the `RangeReader` miss-reporting
/// slot) across `.await` points by construction — `inspect.rs`'s own docs
/// call this trait chain "internal-only, driven as `&mut impl RangeSource`
/// on a single call chain" — so `Cell`'s `!Sync` makes any future built
/// over it structurally `!Send`, independent of any bound written here.
/// [`run_core`] gets its concurrency from a [`tokio::task::LocalSet`] +
/// `JoinSet::spawn_local` instead of plain `spawn` for exactly this reason
/// (see its doc comment) — real concurrency for I/O-bound mirror waits
/// doesn't need OS-thread parallelism, only overlapping outstanding
/// requests.
#[allow(async_fn_in_trait)]
trait EntrySource: RangeSource {
    /// The mirror that served (or would serve) bytes; `""` before any
    /// fetch has succeeded.
    fn mirror_key(&self) -> &'static str;

    /// §5.2 full-download fallback for a no-range-support entry.
    ///
    /// # Errors
    /// [`FetchFailure`] — the same failure shape as [`RangeSource::fetch`].
    async fn download_full(&mut self, expected_size: u64) -> Result<Vec<u8>, FetchFailure>;
}

impl EntrySource for MirrorRanges {
    fn mirror_key(&self) -> &'static str {
        MirrorRanges::mirror_key(self)
    }

    async fn download_full(&mut self, expected_size: u64) -> Result<Vec<u8>, FetchFailure> {
        MirrorRanges::download_full(self, expected_size).await
    }
}

/// Production [`EntrySource`]: a working [`MirrorRanges`], or a pre-failed
/// placeholder for the rare case where [`MirrorRanges::new`] itself fails
/// (a hostile `dir`/`filename` that can't build a URL — DESIGN §5.1: "never
/// panic or skip", so this still yields a `fetch_error` record and a
/// ledger line on first contact rather than aborting the run or dropping
/// the entry silently).
enum LiveSource {
    /// A URL was built successfully; ranged/full reads go to the mirror
    /// pool as normal. Boxed: `MirrorRanges` is far larger than the
    /// `UrlError` variant (two `Url`s, an HTTP client, retry/pin state),
    /// and every `LiveSource` — including the common, successful case —
    /// would otherwise pay that size even when it never needs it.
    Mirror(Box<MirrorRanges>),
    /// `MirrorRanges::new` failed; every fetch/download fails with this
    /// detail instead of touching the network.
    UrlError(String),
}

impl RangeSource for LiveSource {
    async fn fetch(&mut self, offset: u64, len: u64) -> Result<Vec<u8>, FetchFailure> {
        match self {
            LiveSource::Mirror(mirrors) => mirrors.fetch(offset, len).await,
            LiveSource::UrlError(detail) => Err(FetchFailure::Http(detail.clone())),
        }
    }
}

impl EntrySource for LiveSource {
    fn mirror_key(&self) -> &'static str {
        match self {
            LiveSource::Mirror(mirrors) => mirrors.mirror_key(),
            LiveSource::UrlError(_) => "",
        }
    }

    async fn download_full(&mut self, expected_size: u64) -> Result<Vec<u8>, FetchFailure> {
        match self {
            LiveSource::Mirror(mirrors) => mirrors.download_full(expected_size).await,
            LiveSource::UrlError(detail) => Err(FetchFailure::Http(detail.clone())),
        }
    }
}

/// Everything `run_core` needs to know about a finished run, for logging
/// (production) and assertions (tests). No `aborted` field: `run_core`
/// only ever constructs `RunStats` on its success path (an abort takes the
/// `anyhow::bail!` path instead, after writing the manifest with
/// `aborted: Some(reason)` — see the manifest, not this struct, for that
/// story), so a mirrored field here would always read `None` and never be
/// worth reading.
#[derive(Debug)]
pub(crate) struct RunStats {
    /// Entries served from the per-id results log.
    pub(crate) cache_hits: u64,
    /// Entries drained from the live mirror pool (not dispatched — an
    /// aborted run cancels some already-dispatched entries mid-flight and
    /// those are never drained, so never counted here either).
    pub(crate) live_entries: u64,
    /// `idgames-wads.jsonl` lines written.
    pub(crate) records: u64,
    /// `wads-errors.jsonl` lines written.
    pub(crate) ledger: u64,
}

/// Run one phase-2 pass over `entries`, network-generic via `make_source`.
///
/// This is the testable core: it never learns the real `--root`/`--limit`
/// CLI flags (only the already-scoped `entries` and `out_dir`), so the
/// `scoped_root`/`limit` fields it writes into the manifest are always
/// `None` — [`run_async`] patches those in afterward. Everything else in
/// the manifest (byte/record/breaker accounting) is exact.
///
/// # Errors
/// Environmental failures (directories, output writes) and the §5.2
/// fallback breaker / [`RANGE_BYTE_CEILING`] runaway breaker — both abort
/// by writing the partial outputs and the manifest (`aborted: Some(..)`)
/// first, then failing.
async fn run_core<S, F>(
    entries: Vec<FileRecord>,
    out_dir: &Path,
    cache_dir: &Path,
    make_source: F,
) -> anyhow::Result<RunStats>
where
    S: EntrySource + 'static,
    F: Fn(&FileRecord, Arc<TransferCounters>) -> S,
{
    let started_at = Utc::now();
    std::fs::create_dir_all(out_dir).with_context(|| format!("creating {}", out_dir.display()))?;
    let entries_total = u64::try_from(entries.len()).unwrap_or(u64::MAX);

    let mut store = ZipsStore::open(cache_dir.join("zips-log.jsonl"));
    // The `Duration::days(7)` TTL is inert here: `dir_hashes` below only
    // ever calls `ApiCache::lookup` (documented "at any age"), never
    // `is_fresh` — the value is supplied only because `ApiCache::new`
    // requires one, not because Phase 2 checks staleness of its own.
    let api_cache = crate::cache::ApiCache::new(cache_dir.join("api"), chrono::Duration::days(7))?;
    let hashes = dir_hashes(&entries, &api_cache);

    let mut ledger: Vec<LedgerEntry> = Vec::new();
    let (mut records, live, cache_hits) = partition_entries(&entries, &hashes, &store);
    let (live_entries, aborted, range_requests, bytes_transferred, full_downloads, fallback_bytes) =
        drive_worker_pool(
            live,
            &hashes,
            &make_source,
            entries_total,
            &mut records,
            &mut ledger,
            &mut store,
        )
        .await?;

    // §9.3/whole-output stats: unlike `full_downloads`/`fallback_bytes`
    // (run-scoped — see `drive_worker_pool`'s doc comment for why),
    // `unaccounted_entries`/`status_counts`/`zip64_entries` describe the
    // full output (cache hits included), matching `records_written`.
    let unaccounted_entries = unaccounted(&entries, &records);
    let status_counts = status_counts(&records);
    let zip64_entries =
        u64::try_from(records.iter().filter(|r| r.zip64).count()).unwrap_or(u64::MAX);

    let records_written = schema::write_wads_jsonl(&out_dir.join("idgames-wads.jsonl"), records)?;
    let ledger_count = schema::write_ledger(&out_dir.join("wads-errors.jsonl"), ledger)?;

    let duration = (Utc::now() - started_at).num_seconds().max(0);
    let manifest = ZipsManifest {
        id: zips_manifest_id(&started_at),
        started_at: started_at.to_rfc3339(),
        duration_secs: u64::try_from(duration).unwrap_or(0),
        tool_version: schema::tool_version(),
        git_rev: schema::git_rev(),
        scoped_root: None,
        limit: None,
        entries_total,
        records_written,
        ledger_count,
        cache_hits,
        live_entries,
        range_requests,
        bytes_transferred,
        full_downloads,
        fallback_bytes,
        zip64_entries,
        status_counts,
        unaccounted_entries,
        aborted,
    };
    schema::write_zips_manifest(&out_dir.join("wads-manifest.json"), &manifest)?;

    if let Some(reason) = manifest.aborted {
        anyhow::bail!("harvest-zips aborted: {reason}");
    }
    Ok(RunStats {
        cache_hits,
        live_entries,
        records: records_written,
        ledger: ledger_count,
    })
}

/// Split `entries` into immediate not-`.zip` records, already-cached
/// records (§5.4: reused only while the containing dir's `body_hash`
/// still matches), and the live worklist that still needs a mirror.
/// Returns `(records, live, cache_hits)`.
fn partition_entries(
    entries: &[FileRecord],
    hashes: &BTreeMap<String, String>,
    store: &ZipsStore,
) -> (Vec<WadRecord>, Vec<FileRecord>, u64) {
    let mut records = Vec::new();
    let mut live = Vec::new();
    let mut cache_hits = 0_u64;
    for entry in entries {
        if !entry.filename.to_ascii_lowercase().ends_with(".zip") {
            records.push(not_zip_record(entry));
            continue;
        }
        let current_hash = hashes.get(&entry.dir).map(String::as_str);
        if let Some(cached) = store.lookup(entry.id, current_hash) {
            records.push(cached.clone());
            cache_hits += 1;
            continue;
        }
        live.push(entry.clone());
    }
    (records, live, cache_hits)
}

/// Whether `status` is a *conclusive* fact about the archive itself,
/// eligible to persist into the resumable per-id log (fix round 1 / I2 —
/// see the module doc's cache-scoping policy paragraph): `true` for `Ok`,
/// `FullDownload`, `Mirror404All`, `ZipParseError`; `false` for
/// `NoRangeSupport`/`FetchError` (run-scoped, never cache-worthy).
/// `NotZip` is never passed here at all — `partition_entries` assigns it
/// directly, with no mirror contact and hence no `store` interaction —
/// but would also read `false` if it were, since it isn't one of the four
/// matched arms.
fn is_conclusive(status: FetchStatus) -> bool {
    matches!(
        status,
        FetchStatus::Ok
            | FetchStatus::FullDownload
            | FetchStatus::Mirror404All
            | FetchStatus::ZipParseError
    )
}

/// Drive `live` through the [`MIRROR_CONCURRENCY`]-wide pool (§5.4),
/// pushing a record (and any ledger line) for every entry that completes
/// into `records`/`ledger`, persisting only *conclusive* results into
/// `store` (see [`is_conclusive`] and the module doc's cache-scoping
/// policy), and stopping early on either breaker (§5.2 fallback /
/// [`RANGE_BYTE_CEILING`] range-path). Returns `(live_entries,
/// aborted_reason, range_requests, bytes_transferred, full_downloads,
/// fallback_bytes)` — the last four are run-scoped (this call's
/// `TransferCounters`/`FallbackBudget` are fresh each time `drive_worker_pool`
/// runs), not a whole-corpus total: a full download resolved on a *prior*
/// run and now served from `store`'s cache is not recounted here, matching
/// `range_requests`/`bytes_transferred`'s existing run-scoped semantics
/// (their `TransferCounters` never sees a cache hit at all) rather than the
/// whole-output semantics of e.g. `records_written`.
///
/// `EntrySource`'s futures are structurally `!Send` (see its doc comment),
/// so this runs on a [`tokio::task::LocalSet`] via `JoinSet::spawn_local`
/// rather than plain `spawn` — this still gets real concurrency
/// (overlapping outstanding mirror requests) because the work is
/// I/O-bound, just cooperatively scheduled on one OS thread instead of
/// parallel across a thread pool.
///
/// # Errors
/// A worker task panicking, or a [`ZipsStore::record`] write failure.
async fn drive_worker_pool<S, F>(
    live: Vec<FileRecord>,
    hashes: &BTreeMap<String, String>,
    make_source: &F,
    entries_total: u64,
    records: &mut Vec<WadRecord>,
    ledger: &mut Vec<LedgerEntry>,
    store: &mut ZipsStore,
) -> anyhow::Result<(u64, Option<String>, u64, u64, u64, u64)>
where
    S: EntrySource + 'static,
    F: Fn(&FileRecord, Arc<TransferCounters>) -> S,
{
    let counters = Arc::new(TransferCounters::new());
    let budget = Arc::new(tokio::sync::Mutex::new(FallbackBudget::new(entries_total)));
    let semaphore = Arc::new(tokio::sync::Semaphore::new(MIRROR_CONCURRENCY));
    let bar = progress_bar(live.len());

    let mut live_entries = 0_u64;
    let mut full_downloads = 0_u64;
    let mut fallback_bytes = 0_u64;
    let mut aborted: Option<String> = None;
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            // Every live entry is spawned up front — the semaphore, not
            // JoinSet occupancy, is what bounds real concurrency (§5.4:
            // 4–8 connections). A task not yet holding a permit is cheap
            // to cancel outright, so on an abort "stop spawning" is
            // already satisfied by construction; only `abort_all` below
            // is needed.
            let mut join_set: tokio::task::JoinSet<(FileRecord, EntryOutcome, &'static str, bool)> =
                tokio::task::JoinSet::new();
            for entry in live {
                let sem = Arc::clone(&semaphore);
                let budget = Arc::clone(&budget);
                let mut source = make_source(&entry, Arc::clone(&counters));
                join_set.spawn_local(async move {
                    let _permit = sem
                        .acquire_owned()
                        .await
                        .expect("semaphore is never closed during a run");
                    let (outcome, breaker_aborted) =
                        process_entry(&mut source, &entry, &budget).await;
                    let mirror_key = source.mirror_key();
                    (entry, outcome, mirror_key, breaker_aborted)
                });
            }

            while let Some(joined) = join_set.join_next().await {
                let (entry, outcome, mirror_key, breaker_aborted) =
                    joined.context("harvest-zips worker task panicked")?;
                live_entries += 1;
                bar.inc(1);
                let record = outcome_to_record(&entry, outcome, mirror_key, ledger);
                if record.fetch_status == FetchStatus::FullDownload {
                    full_downloads += 1;
                    fallback_bytes += record.zip_size;
                }
                // I2: only a conclusive outcome is fit to persist — a
                // `NoRangeSupport`/`FetchError` must retry live next run
                // (module doc's cache-scoping policy), so `store.record`
                // is skipped for those, though the record itself always
                // goes into `records` either way (§9.3 completeness).
                let conclusive = is_conclusive(record.fetch_status);
                records.push(record.clone());
                if conclusive && let Some(hash) = hashes.get(&entry.dir) {
                    store.record(hash, record)?;
                }
                // M3: the live shared counter, not a per-task snapshot —
                // a pathological entry's bytes must trip this the instant
                // any entry's drain observes the ceiling crossed, not only
                // the one entry that happened to push it over.
                if breaker_aborted {
                    aborted = Some(
                        "fallback breaker: more than ~2% of entries needed full downloads"
                            .to_owned(),
                    );
                } else if counters.bytes.load(Ordering::Relaxed) > RANGE_BYTE_CEILING {
                    aborted = Some(
                        "range-path byte ceiling exceeded — mirror or archive shape pathology"
                            .to_owned(),
                    );
                }
                if aborted.is_some() {
                    break;
                }
            }
            bar.finish_and_clear();
            if aborted.is_some() {
                // Stop spawning is already satisfied (everything was
                // spawned up front); this cancels whatever hasn't
                // finished. Those in-flight entries deliberately never
                // get a record — the manifest's `aborted` reason is what
                // tells that story.
                join_set.abort_all();
                while join_set.join_next().await.is_some() {}
            }
            Ok::<(), anyhow::Error>(())
        })
        .await?;

    Ok((
        live_entries,
        aborted,
        counters.requests.load(Ordering::Relaxed),
        counters.bytes.load(Ordering::Relaxed),
        full_downloads,
        fallback_bytes,
    ))
}

/// One live entry: ranged inspect; `RangeUnsupported` → the per-entry cap,
/// then [`FallbackBudget::admit`] → `download_full` → parse in memory
/// ([`parse_downloaded`]). Returns the outcome plus whether this call just
/// tripped the fallback breaker (the orchestrator's cue to abort the
/// phase after recording this entry).
async fn process_entry<S: EntrySource>(
    source: &mut S,
    entry: &FileRecord,
    budget: &tokio::sync::Mutex<FallbackBudget>,
) -> (EntryOutcome, bool) {
    match inspect::inspect_zip(source, entry.size).await {
        Ok(inspection) => (
            EntryOutcome::Inspected {
                inspection,
                full_download: false,
            },
            false,
        ),
        Err(InspectError::Fetch(FetchFailure::NotFound)) => {
            (EntryOutcome::Failed(FailKind::Mirror404All), false)
        }
        Err(InspectError::Fetch(FetchFailure::RangeUnsupported)) => {
            handle_no_range_support(source, entry, budget).await
        }
        Err(InspectError::Fetch(FetchFailure::Http(detail))) => {
            (EntryOutcome::Failed(FailKind::Fetch(detail)), false)
        }
        Err(e @ (InspectError::CdTooLarge { .. } | InspectError::TooChatty { .. })) => (
            EntryOutcome::Failed(FailKind::ZipParse(e.to_string())),
            false,
        ),
        Err(InspectError::Parse(detail)) => {
            (EntryOutcome::Failed(FailKind::ZipParse(detail)), false)
        }
    }
}

/// §5.2 budgeted fallback for an entry whose mirrors refused ranges, plus
/// the controller's per-entry cap: a `.zip` bigger than
/// [`FALLBACK_PER_ENTRY_CAP`] is refused outright, without ever touching
/// [`FallbackBudget`]'s byte pool. It still counts toward the breaker:
/// `admit(u64::MAX)` can never be granted (no real remaining budget is
/// ever that large), so the call only exercises `FallbackBudget`'s
/// `needed`/breaker bookkeeping and always returns `Skip` or `Abort`,
/// never `Download` — the entry's real (possibly enormous) size never
/// reaches `bytes_remaining`.
async fn handle_no_range_support<S: EntrySource>(
    source: &mut S,
    entry: &FileRecord,
    budget: &tokio::sync::Mutex<FallbackBudget>,
) -> (EntryOutcome, bool) {
    if entry.size > FALLBACK_PER_ENTRY_CAP {
        let decision = budget.lock().await.admit(u64::MAX);
        let aborted = matches!(decision, FallbackDecision::Abort);
        return (
            EntryOutcome::Failed(FailKind::NoRange {
                budget_refused: false,
            }),
            aborted,
        );
    }
    let decision = budget.lock().await.admit(entry.size);
    match decision {
        FallbackDecision::Abort => (
            EntryOutcome::Failed(FailKind::NoRange {
                budget_refused: true,
            }),
            true,
        ),
        FallbackDecision::Skip => (
            EntryOutcome::Failed(FailKind::NoRange {
                budget_refused: true,
            }),
            false,
        ),
        FallbackDecision::Download => match source.download_full(entry.size).await {
            Ok(bytes) => (parse_downloaded(&bytes), false),
            Err(f) => (EntryOutcome::Failed(FailKind::Fetch(f.to_string())), false),
        },
    }
}

/// Parse a fully in-memory zip (the §5.2 fallback path) via the same
/// central-directory walk ranged inspection uses. A `Cell` that is never
/// set stands in for the miss-reporting cell a ranged `RangeReader` would
/// use — a `Cursor` over the whole file never misses.
fn parse_downloaded(bytes: &[u8]) -> EntryOutcome {
    match zip::ZipArchive::new(std::io::Cursor::new(bytes)) {
        Ok(mut archive) => {
            let tail_len = usize::try_from(TAIL_LEN).unwrap_or(usize::MAX);
            let tail_start = bytes.len().saturating_sub(tail_len);
            let zip64 = inspect::zip64_present(&bytes[tail_start..]);
            let missing = std::cell::Cell::new(None);
            let inspection = inspect::inspection_from_archive(&mut archive, zip64, &missing);
            EntryOutcome::Inspected {
                inspection,
                full_download: true,
            }
        }
        Err(e) => EntryOutcome::Failed(FailKind::ZipParse(e.to_string())),
    }
}

/// Hidden under 2 (phase-1 `api::traverse::progress_bar` style — no
/// visible progress bar for a trivial run).
fn progress_bar(len: usize) -> indicatif::ProgressBar {
    if len < 2 {
        return indicatif::ProgressBar::hidden();
    }
    let bar = indicatif::ProgressBar::new(u64::try_from(len).unwrap_or(u64::MAX));
    bar.set_style(
        indicatif::ProgressStyle::with_template("{bar:40} {pos}/{len} entries ({eta} left) {msg}")
            .expect("static template is valid"),
    );
    bar
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::FetchStatus;
    use std::io::Write as _;
    use std::sync::atomic::AtomicU64;

    fn rec(id: u64, dir: &str, filename: &str, size: u64) -> crate::api::model::FileRecord {
        serde_json::from_value(serde_json::json!({
            "id": id, "dir": dir, "filename": filename, "size": size, "age": 0,
            "date": "2019-04-02", "rating": 4.5, "votes": 3
        }))
        .unwrap()
    }

    #[test]
    fn worklist_scopes_by_root_sorts_by_id_and_applies_limit() {
        let records = vec![
            rec(9, "levels/doom2/a-c/", "c.zip", 10),
            rec(3, "levels/doom/0-9/", "a.zip", 10),
            rec(5, "levels/doom/0-9/", "b.zip", 10),
        ];
        let all = worklist(records.clone(), None, None);
        assert_eq!(all.iter().map(|r| r.id).collect::<Vec<_>>(), vec![3, 5, 9]);
        let scoped = worklist(records.clone(), Some("levels/doom/"), None);
        assert_eq!(scoped.iter().map(|r| r.id).collect::<Vec<_>>(), vec![3, 5]);
        let limited = worklist(records, None, Some(2));
        assert_eq!(limited.iter().map(|r| r.id).collect::<Vec<_>>(), vec![3, 5]);
    }

    #[test]
    fn non_zip_entries_are_recorded_without_mirror_contact() {
        let record = not_zip_record(&rec(4, "levels/doom/0-9/", "old.exe", 77));
        assert_eq!(record.fetch_status, FetchStatus::NotZip);
        assert!(!record.is_zip);
        assert_eq!(record.mirror, "");
        assert_eq!(record.member_count, 0);
        // Phase-1 metadata still carried (§5.6).
        assert_eq!(record.date, "2019-04-02");
        assert_eq!(record.votes, 3);
    }

    #[test]
    fn outcome_to_record_maps_every_status_and_ledgers_failures() {
        let entry = rec(7, "levels/doom/0-9/", "a.zip", 100);
        let inspection = crate::zips::inspect::Inspection {
            zip64: true,
            member_count: 2,
            wads: Vec::new(),
            other_members: vec!["x.txt".into(), "inner.zip".into()],
        };
        let mut ledger = Vec::new();

        let ok = outcome_to_record(
            &entry,
            EntryOutcome::Inspected {
                inspection: inspection.clone(),
                full_download: false,
            },
            "infania",
            &mut ledger,
        );
        assert_eq!(ok.fetch_status, FetchStatus::Ok);
        assert!(ok.zip64);
        assert!(ledger.is_empty());

        let full = outcome_to_record(
            &entry,
            EntryOutcome::Inspected {
                inspection,
                full_download: true,
            },
            "gamers",
            &mut ledger,
        );
        assert_eq!(full.fetch_status, FetchStatus::FullDownload);
        assert_eq!(full.mirror, "gamers");
        assert!(
            ledger.is_empty(),
            "full download is a recorded outcome, not a failure"
        );

        let e404 = outcome_to_record(
            &entry,
            EntryOutcome::Failed(FailKind::Mirror404All),
            "",
            &mut ledger,
        );
        assert_eq!(e404.fetch_status, FetchStatus::Mirror404All);
        assert_eq!(ledger.len(), 1);
        assert_eq!(ledger[0].path, "levels/doom/0-9/a.zip");
        assert_eq!(ledger[0].action, "harvest-zips");
        assert_eq!(
            ledger[0].detail, "404 on all mirrors",
            "an unknown mirror (never pinned before the 404) must not be attributed"
        );

        let parse = outcome_to_record(
            &entry,
            EntryOutcome::Failed(FailKind::ZipParse("bad magic".into())),
            "infania",
            &mut ledger,
        );
        assert_eq!(parse.fetch_status, FetchStatus::ZipParseError);
        assert_eq!(ledger.len(), 2);
        assert!(matches!(
            ledger[1].kind,
            crate::schema::LedgerKind::ParseError
        ));
        assert_eq!(
            ledger[1].detail, "bad magic: last mirror infania",
            "a known mirror must be attributed in the ledger detail"
        );

        let skipped = outcome_to_record(
            &entry,
            EntryOutcome::Failed(FailKind::NoRange {
                budget_refused: true,
            }),
            "",
            &mut ledger,
        );
        assert_eq!(skipped.fetch_status, FetchStatus::NoRangeSupport);
        assert!(ledger[2].detail.contains("budget"));

        let fetch = outcome_to_record(
            &entry,
            EntryOutcome::Failed(FailKind::Fetch("timeout".into())),
            "",
            &mut ledger,
        );
        assert_eq!(fetch.fetch_status, FetchStatus::FetchError);
        assert_eq!(ledger.len(), 4);
    }

    #[test]
    fn every_worklist_entry_is_accounted_for() {
        // §9.3 completeness: records ∪ ledger covers the whole worklist.
        // Records alone must cover it — the ledger adds detail, never
        // substitutes.
        let entries = vec![
            rec(1, "levels/doom/0-9/", "a.zip", 10),
            rec(2, "levels/doom/0-9/", "not-a-zip.txt", 10),
        ];
        let records = vec![
            not_zip_record(&entries[1]),
            outcome_to_record(
                &entries[0],
                EntryOutcome::Failed(FailKind::Fetch("x".into())),
                "",
                &mut Vec::new(),
            ),
        ];
        assert_eq!(unaccounted(&entries, &records), 0);
        assert_eq!(unaccounted(&entries, &records[..1]), 1);
    }

    /// Fixture-backed [`EntrySource`] fake: serves ranges/full-downloads
    /// from shared in-memory bytes and counts fetches (both kinds) via a
    /// shared counter — the `run_core` test's cache-reuse regression
    /// instrument.
    struct FakeEntrySource {
        bytes: Arc<Vec<u8>>,
        fetches: Arc<AtomicU64>,
    }

    impl RangeSource for FakeEntrySource {
        async fn fetch(&mut self, offset: u64, len: u64) -> Result<Vec<u8>, FetchFailure> {
            self.fetches.fetch_add(1, Ordering::SeqCst);
            let start = usize::try_from(offset).unwrap();
            let end = start + usize::try_from(len).unwrap();
            self.bytes
                .get(start..end)
                .map(<[u8]>::to_vec)
                .ok_or_else(|| FetchFailure::Http("range beyond EOF".into()))
        }
    }

    impl EntrySource for FakeEntrySource {
        fn mirror_key(&self) -> &'static str {
            "fake"
        }

        async fn download_full(&mut self, _expected_size: u64) -> Result<Vec<u8>, FetchFailure> {
            self.fetches.fetch_add(1, Ordering::SeqCst);
            Ok((*self.bytes).clone())
        }
    }

    /// [`SourceFactory`]-shaped closure serving `zip_bytes` and bumping
    /// `fetches`, ignoring the run-wide counters (the byte-ceiling breaker
    /// isn't exercised by this fixture).
    fn make_fake_factory(
        zip_bytes: Arc<Vec<u8>>,
        fetches: Arc<AtomicU64>,
    ) -> impl Fn(&crate::api::model::FileRecord, Arc<TransferCounters>) -> FakeEntrySource {
        move |_entry, _counters| FakeEntrySource {
            bytes: Arc::clone(&zip_bytes),
            fetches: Arc::clone(&fetches),
        }
    }

    /// Local copy of the Task-3 `stored_zip` builder (inspect.rs's fixture
    /// helper is private to that module): a small stored zip, two members,
    /// one `.wad`.
    fn tests_zip_fixture() -> Vec<u8> {
        let mut w = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        w.start_file("MAP01.WAD", opts).unwrap();
        w.write_all(b"wad bytes").unwrap();
        w.start_file("README.TXT", opts).unwrap();
        w.write_all(b"txt").unwrap();
        w.finish().unwrap().into_inner()
    }

    #[tokio::test]
    // Locked test body verbatim per the task-6 brief (Step 1); the
    // `match pass { 0 => .., _ => .. }` below reads more naturally as the
    // brief wrote it (parallel to a possible future third pass) than as an
    // `if`/`else`, so the lint is suppressed rather than the locked code
    // restructured.
    #[allow(clippy::single_match_else)]
    async fn run_core_reuses_cache_and_writes_outputs() {
        // End-to-end over fixture bytes with a fake source factory: run
        // twice, assert the second run does zero fetches (§5.4 warm rerun)
        // and outputs are complete and sorted.
        let tmp = tempfile::tempdir().unwrap();
        let out_dir = tmp.path().join("out");
        let cache_dir = tmp.path().join("cache");
        std::fs::create_dir_all(&out_dir).unwrap();
        std::fs::create_dir_all(&cache_dir).unwrap();
        // Phase-1 cache envelope for the dir → a current body_hash exists.
        let api =
            crate::cache::ApiCache::new(cache_dir.join("api"), chrono::Duration::days(7)).unwrap();
        api.store(
            "getcontents",
            "levels/doom/0-9/",
            3,
            serde_json::json!({"content": {}}),
        )
        .unwrap();

        let zip_bytes = std::sync::Arc::new(tests_zip_fixture()); // helper below
        let entries = vec![rec(
            5,
            "levels/doom/0-9/",
            "a.zip",
            u64::try_from(zip_bytes.len()).unwrap(),
        )];
        let fetches = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));

        for pass in 0..2_u64 {
            let stats = run_core(
                entries.clone(),
                &out_dir,
                &cache_dir,
                make_fake_factory(zip_bytes.clone(), fetches.clone()),
            )
            .await
            .unwrap();
            match pass {
                0 => {
                    assert_eq!(stats.cache_hits, 0);
                    assert!(fetches.load(std::sync::atomic::Ordering::SeqCst) >= 1);
                }
                _ => {
                    assert_eq!(stats.cache_hits, 1, "warm rerun hits the id log");
                    assert_eq!(fetches.load(std::sync::atomic::Ordering::SeqCst), 1);
                }
            }
        }
        let text = std::fs::read_to_string(out_dir.join("idgames-wads.jsonl")).unwrap();
        assert_eq!(text.lines().count(), 1);
        let v: serde_json::Value = serde_json::from_str(text.lines().next().unwrap()).unwrap();
        assert_eq!(v["id"], 5);
        assert_eq!(v["fetch_status"], "ok");
        let manifest: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(out_dir.join("wads-manifest.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(manifest["unaccounted_entries"], 0);
        assert_eq!(manifest["entries_total"], 1);
    }

    // ---- Fix round 1 regression tests ----

    /// I1: `patch_manifest_provenance` must never rewrite a manifest that
    /// predates this run's `run_start` — otherwise a `run_core` call that
    /// errors out before ever writing a manifest (an environmental
    /// failure in `create_dir_all`/`ApiCache::new`/etc.) would leave a
    /// *previous* run's manifest on disk, and an unconditional patch would
    /// silently relabel that old file's `scoped_root`/`limit` with the
    /// current invocation's flags.
    #[test]
    fn patch_manifest_provenance_skips_a_manifest_older_than_this_run() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("wads-manifest.json");
        let old_manifest = ZipsManifest {
            id: "harvest-zips-20200101T000000Z".into(),
            started_at: "2020-01-01T00:00:00+00:00".into(),
            duration_secs: 1,
            tool_version: schema::tool_version(),
            git_rev: None,
            scoped_root: None,
            limit: None,
            entries_total: 1,
            records_written: 1,
            ledger_count: 0,
            cache_hits: 0,
            live_entries: 0,
            range_requests: 0,
            bytes_transferred: 0,
            full_downloads: 0,
            fallback_bytes: 0,
            zip64_entries: 0,
            status_counts: BTreeMap::new(),
            unaccounted_entries: 0,
            aborted: None,
        };
        schema::write_zips_manifest(&path, &old_manifest).unwrap();
        let before = std::fs::read(&path).unwrap();

        // `run_start` is "now" — long after the fake manifest's 2020
        // `started_at` — exactly the "a previous run's leftover file"
        // shape the guard exists for.
        let run_start = Utc::now();
        patch_manifest_provenance(&path, Some("levels/doom/"), Some(5), run_start).unwrap();

        let after = std::fs::read(&path).unwrap();
        assert_eq!(
            before, after,
            "a manifest older than this run must not be touched"
        );
    }

    /// [`EntrySource`] fake whose `fetch` fails with an ordinary transport
    /// error (`FetchFailure::Http`) while `fail` is `true`, and serves real
    /// bytes once it flips to `false` — the I2 cache-scoping regression
    /// instrument: a transient `fetch_error` must never land in the
    /// resumable per-id log, or a single flaky mirror response on run N
    /// would silently poison every future run for that entry (the log only
    /// invalidates on a `body_hash` change, which a transient failure never
    /// causes).
    struct FlakyFakeSource {
        bytes: Arc<Vec<u8>>,
        fail: Arc<std::sync::atomic::AtomicBool>,
    }

    impl RangeSource for FlakyFakeSource {
        async fn fetch(&mut self, offset: u64, len: u64) -> Result<Vec<u8>, FetchFailure> {
            if self.fail.load(Ordering::SeqCst) {
                return Err(FetchFailure::Http("simulated transient failure".into()));
            }
            let start = usize::try_from(offset).unwrap();
            let end = start + usize::try_from(len).unwrap();
            self.bytes
                .get(start..end)
                .map(<[u8]>::to_vec)
                .ok_or_else(|| FetchFailure::Http("range beyond EOF".into()))
        }
    }

    impl EntrySource for FlakyFakeSource {
        fn mirror_key(&self) -> &'static str {
            "fake"
        }

        async fn download_full(&mut self, _expected_size: u64) -> Result<Vec<u8>, FetchFailure> {
            Ok((*self.bytes).clone())
        }
    }

    fn make_flaky_factory(
        zip_bytes: Arc<Vec<u8>>,
        fail: Arc<std::sync::atomic::AtomicBool>,
    ) -> impl Fn(&crate::api::model::FileRecord, Arc<TransferCounters>) -> FlakyFakeSource {
        move |_entry, _counters| FlakyFakeSource {
            bytes: Arc::clone(&zip_bytes),
            fail: Arc::clone(&fail),
        }
    }

    #[tokio::test]
    async fn transient_fetch_errors_are_not_cached_and_retry_live_next_run() {
        let tmp = tempfile::tempdir().unwrap();
        let out_dir = tmp.path().join("out");
        let cache_dir = tmp.path().join("cache");
        std::fs::create_dir_all(&out_dir).unwrap();
        std::fs::create_dir_all(&cache_dir).unwrap();
        let api =
            crate::cache::ApiCache::new(cache_dir.join("api"), chrono::Duration::days(7)).unwrap();
        api.store(
            "getcontents",
            "levels/doom/0-9/",
            3,
            serde_json::json!({"content": {}}),
        )
        .unwrap();

        let zip_bytes = Arc::new(tests_zip_fixture());
        let entries = vec![rec(
            9,
            "levels/doom/0-9/",
            "a.zip",
            u64::try_from(zip_bytes.len()).unwrap(),
        )];
        let fail = Arc::new(std::sync::atomic::AtomicBool::new(true));

        // First run: every fetch fails. `run_core` itself still succeeds
        // (a single entry's `fetch_error` doesn't trip either breaker) —
        // the entry is recorded as `fetch_error`, but I2 says that must
        // never reach the resumable per-id log.
        let stats1 = run_core(
            entries.clone(),
            &out_dir,
            &cache_dir,
            make_flaky_factory(zip_bytes.clone(), fail.clone()),
        )
        .await
        .unwrap();
        assert_eq!(stats1.cache_hits, 0);
        let text1 = std::fs::read_to_string(out_dir.join("idgames-wads.jsonl")).unwrap();
        let v1: serde_json::Value = serde_json::from_str(text1.lines().next().unwrap()).unwrap();
        assert_eq!(v1["fetch_status"], "fetch_error");

        // Second run: the mirror is healthy now. If the first run's
        // `fetch_error` had been cached, this would be a `cache_hits == 1`
        // no-op reusing the stale failure — it must instead be a fresh
        // live fetch that succeeds.
        fail.store(false, Ordering::SeqCst);
        let stats2 = run_core(
            entries,
            &out_dir,
            &cache_dir,
            make_flaky_factory(zip_bytes, fail),
        )
        .await
        .unwrap();
        assert_eq!(
            stats2.cache_hits, 0,
            "a fetch_error must never be cached — this must be a fresh live fetch"
        );
        let text2 = std::fs::read_to_string(out_dir.join("idgames-wads.jsonl")).unwrap();
        let v2: serde_json::Value = serde_json::from_str(text2.lines().next().unwrap()).unwrap();
        assert_eq!(v2["fetch_status"], "ok");
    }

    /// [`EntrySource`] fake that always reports "mirrors refuse ranges"
    /// and counts `download_full` calls separately — the per-entry-cap
    /// (I3a) regression instrument: an over-cap entry must be refused
    /// before `download_full` (and therefore the shared fallback budget)
    /// is ever touched.
    struct NoRangeFakeSource {
        download_full_calls: Arc<AtomicU64>,
    }

    impl RangeSource for NoRangeFakeSource {
        async fn fetch(&mut self, _offset: u64, _len: u64) -> Result<Vec<u8>, FetchFailure> {
            Err(FetchFailure::RangeUnsupported)
        }
    }

    impl EntrySource for NoRangeFakeSource {
        fn mirror_key(&self) -> &'static str {
            "fake"
        }

        async fn download_full(&mut self, _expected_size: u64) -> Result<Vec<u8>, FetchFailure> {
            self.download_full_calls.fetch_add(1, Ordering::SeqCst);
            Ok(Vec::new())
        }
    }

    #[tokio::test]
    // Five over-cap entries, not one: with a single entry, `admit(u64::MAX)`
    // returns `Skip` whether `FallbackBudget::admit` increments `needed`
    // before or after its byte-budget check (`u64::MAX` always exceeds
    // `bytes_remaining` either way, and one call can never itself exceed
    // `fallback_limit`'s floor of 2) — a single-entry version of this test
    // could NOT have told the two orderings apart. `fallback_limit` floors
    // at 2 for any small entry count (`fallback_limit_has_a_small_run_floor`
    // in `range_reader.rs`), so a 5th over-cap `admit(u64::MAX)` call is
    // guaranteed to observe `needed > limit` and trip the breaker — but
    // ONLY if every earlier call actually incremented `needed` first, i.e.
    // only if `admit` still increments before it checks the byte budget.
    // Were that order flipped, every one of these five calls would return
    // `Skip` without ever touching `needed`, the breaker would never trip,
    // and `run_core` below would return `Ok` instead of the `Err` this test
    // asserts — that's the "flip" this test pins.
    async fn per_entry_fallback_cap_refuses_before_touching_download_full() {
        let tmp = tempfile::tempdir().unwrap();
        let out_dir = tmp.path().join("out");
        let cache_dir = tmp.path().join("cache");
        std::fs::create_dir_all(&out_dir).unwrap();
        std::fs::create_dir_all(&cache_dir).unwrap();

        let download_full_calls = Arc::new(AtomicU64::new(0));
        let over_cap = 600 * 1024 * 1024_u64; // over FALLBACK_PER_ENTRY_CAP (512 MiB)
        let entries: Vec<_> = (1..=5)
            .map(|id| rec(id, "levels/doom/0-9/", &format!("huge{id}.zip"), over_cap))
            .collect();
        let calls_for_factory = Arc::clone(&download_full_calls);
        let make_source = move |_entry: &crate::api::model::FileRecord,
                                _counters: Arc<TransferCounters>| {
            NoRangeFakeSource {
                download_full_calls: Arc::clone(&calls_for_factory),
            }
        };

        let err = run_core(entries, &out_dir, &cache_dir, make_source)
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("fallback breaker"),
            "5 over-cap entries (limit floors at 2) must trip the breaker: {err}"
        );

        assert_eq!(
            download_full_calls.load(Ordering::SeqCst),
            0,
            "over-cap entries must never reach download_full — zero budget bytes ever consumed"
        );
        let text = std::fs::read_to_string(out_dir.join("idgames-wads.jsonl")).unwrap();
        assert!(
            text.lines().all(|line| {
                let v: serde_json::Value = serde_json::from_str(line).unwrap();
                v["fetch_status"] == "no_range_support"
            }),
            "every entry that completed before the abort must be no_range_support: {text}"
        );
        let ledger_text = std::fs::read_to_string(out_dir.join("wads-errors.jsonl")).unwrap();
        assert!(ledger_text.contains("entry exceeds per-entry fallback cap"));
    }

    /// [`EntrySource`] fake whose every `fetch` inflates the shared
    /// [`TransferCounters`] far past [`RANGE_BYTE_CEILING`] before failing
    /// — the range-path runaway-breaker (I3b) regression instrument.
    struct ByteBombFakeSource {
        counters: Arc<TransferCounters>,
    }

    impl RangeSource for ByteBombFakeSource {
        async fn fetch(&mut self, _offset: u64, _len: u64) -> Result<Vec<u8>, FetchFailure> {
            self.counters
                .bytes
                .fetch_add(5 * 1024 * 1024 * 1024, Ordering::SeqCst);
            Err(FetchFailure::Http("simulated pathological transfer".into()))
        }
    }

    impl EntrySource for ByteBombFakeSource {
        fn mirror_key(&self) -> &'static str {
            "fake"
        }

        async fn download_full(&mut self, _expected_size: u64) -> Result<Vec<u8>, FetchFailure> {
            Err(FetchFailure::Http("not used".into()))
        }
    }

    #[tokio::test]
    async fn range_byte_ceiling_aborts_the_run_but_still_writes_outputs() {
        let tmp = tempfile::tempdir().unwrap();
        let out_dir = tmp.path().join("out");
        let cache_dir = tmp.path().join("cache");
        std::fs::create_dir_all(&out_dir).unwrap();
        std::fs::create_dir_all(&cache_dir).unwrap();

        let entries = vec![rec(1, "levels/doom/0-9/", "a.zip", 10)];
        let make_source = |_entry: &crate::api::model::FileRecord,
                           counters: Arc<TransferCounters>| {
            ByteBombFakeSource { counters }
        };

        let err = run_core(entries, &out_dir, &cache_dir, make_source)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("range-path byte ceiling"));

        let manifest: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(out_dir.join("wads-manifest.json")).unwrap(),
        )
        .unwrap();
        assert!(
            manifest["aborted"]
                .as_str()
                .unwrap()
                .contains("range-path byte ceiling")
        );

        // Outputs are still written despite the abort.
        assert!(out_dir.join("idgames-wads.jsonl").exists());
        assert!(out_dir.join("wads-errors.jsonl").exists());
    }
}
