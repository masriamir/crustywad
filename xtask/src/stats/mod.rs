//! Phase-3 statistics (DESIGN.md §6). Pure functions over local harvest
//! outputs — no network.
//!
//! `xtask stats` loads Phase 1/2's outputs (`idgames-wads.jsonl`,
//! `harvest-manifest.json`, `wads-manifest.json`, `harvest-errors.jsonl`),
//! the cached ls-laR.gz mirror listing, and the optional §6.4 outliers
//! supplement, builds the §6 populations, computes every statistic
//! including the §8 constant recommendations ([`report::recommendations`]),
//! and emits the full PII-free trio: `data/stats.json`,
//! `data/stats-report.md` ([`report::render_report`]), and
//! `data/sweep-corpus.jsonl` (§6.5).
//!
//! `build_stats` and its classifiers ([`top_bucket`], [`year_of`]) are pure
//! and unit-tested without touching a filesystem; [`run`]/[`run_with_paths`]
//! do the surrounding I/O. Per §9.3/§7, nothing in this module's output ever
//! reads a wall clock — every provenance fact traces back to an input
//! manifest's `id`, [`crate::mirror::LsLarMeta`], or this run's own
//! `tool_version`/`git_rev`.

pub mod percentiles;
mod report;

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::Context as _;

use crate::cache::atomic_write;
use crate::lslar::{ArchiveTree, parse_ls_lar_gz};
use crate::mirror::LsLarMeta;
use crate::schema::{
    ApiDelta, Coverage, Distribution, EntryStats, FetchStatus, HistogramBucket, IdgamesStats,
    LedgerEntry, OutlierRecord, OutlierSkip, OutlierSummary, OutliersStats, RatioDistribution,
    RatioStats, SizeStats, StatsJson, StatsProvenance, WadRecord, WeightedDistribution,
    ZipSizeStats, git_rev, read_ledger, read_manifest, read_outliers_jsonl, read_outliers_manifest,
    read_wads_jsonl, read_zips_manifest, sweep_entries, tool_version, write_stats_json,
    write_sweep_jsonl,
};

/// Every filesystem path `xtask stats` reads or writes. Input paths (all but
/// `out_dir`) are already scoped by [`crate::phase1::output_dir`], except
/// `lslar_gz`/`lslar_meta`, which are mode-independent — the mirror cache
/// under `data/cache` is shared by every run, scoped or not.
pub struct StatsPaths {
    /// Phase-2 `idgames-wads.jsonl`.
    pub wads_jsonl: PathBuf,
    /// Phase-2 `wads-manifest.json`.
    pub zips_manifest: PathBuf,
    /// Phase-1 `harvest-manifest.json`.
    pub phase1_manifest: PathBuf,
    /// Phase-1 `harvest-errors.jsonl`.
    pub phase1_ledger: PathBuf,
    /// Cached `ls-laR.gz` mirror listing (§5.0).
    pub lslar_gz: PathBuf,
    /// Sidecar metadata for `lslar_gz`.
    pub lslar_meta: PathBuf,
    /// §6.4 `outliers-wads.jsonl`, when `xtask harvest-outliers` has run.
    pub outliers_jsonl: PathBuf,
    /// §6.4 `outliers-manifest.json`.
    pub outliers_manifest: PathBuf,
    /// Where `stats.json`/`sweep-corpus.jsonl` are written.
    pub out_dir: PathBuf,
}

/// Run `xtask stats`. `root`/`limit` are the §4.6 dev flags: when either is
/// set, both input and output move to `data/dev/` — the same "scoped"
/// convention Phase 1/2/outliers use — except the ls-laR cache, which is
/// always read from `data/cache` regardless of scope.
///
/// # Errors
/// See [`run_with_paths`].
pub fn run(root: Option<&str>, limit: Option<usize>) -> anyhow::Result<()> {
    let scoped = root.is_some() || limit.is_some();
    let out_dir = crate::phase1::output_dir(scoped);
    let cache_dir = crate::phase1::data_root().join("cache");
    let paths = StatsPaths {
        wads_jsonl: out_dir.join("idgames-wads.jsonl"),
        zips_manifest: out_dir.join("wads-manifest.json"),
        phase1_manifest: out_dir.join("harvest-manifest.json"),
        phase1_ledger: out_dir.join("harvest-errors.jsonl"),
        lslar_gz: cache_dir.join("ls-laR.gz"),
        lslar_meta: cache_dir.join("ls-laR.meta.json"),
        outliers_jsonl: out_dir.join("outliers-wads.jsonl"),
        outliers_manifest: out_dir.join("outliers-manifest.json"),
        out_dir,
    };
    run_with_paths(&paths, root, limit)
}

/// The testable core of [`run`]: everything after path assembly.
///
/// # Errors
/// A required input is missing (`idgames-wads.jsonl`/`wads-manifest.json` →
/// "run `just harvest-zips` first"; `harvest-manifest.json`/`ls-laR.gz`/
/// `ls-laR.meta.json` → "run `just harvest-api` first"), the outliers pair
/// is half-present (one of `outliers-wads.jsonl`/`outliers-manifest.json`
/// exists without the other), a `WadRecord`'s `dir`/`filename` can't be
/// turned into a sweep-corpus URL, or an environmental failure (directory
/// creation, output writes).
pub fn run_with_paths(
    paths: &StatsPaths,
    root: Option<&str>,
    limit: Option<usize>,
) -> anyhow::Result<()> {
    let Some(records) = read_wads_jsonl(&paths.wads_jsonl) else {
        anyhow::bail!(
            "no idgames-wads.jsonl at {} — run `just harvest-zips` first",
            paths.wads_jsonl.display()
        );
    };
    let Some(zips_manifest) = read_zips_manifest(&paths.zips_manifest) else {
        anyhow::bail!(
            "no wads-manifest.json at {} — run `just harvest-zips` first",
            paths.zips_manifest.display()
        );
    };
    let Some(phase1_manifest) = read_manifest(&paths.phase1_manifest) else {
        anyhow::bail!(
            "no harvest-manifest.json at {} — run `just harvest-api` first",
            paths.phase1_manifest.display()
        );
    };
    let Ok(lslar_bytes) = std::fs::read(&paths.lslar_gz) else {
        anyhow::bail!(
            "no ls-laR.gz cache at {} — run `just harvest-api` first",
            paths.lslar_gz.display()
        );
    };
    let tree = parse_ls_lar_gz(&lslar_bytes)
        .with_context(|| format!("parsing {}", paths.lslar_gz.display()))?;
    let Ok(lslar_meta_bytes) = std::fs::read(&paths.lslar_meta) else {
        anyhow::bail!(
            "no ls-laR.meta.json at {} — run `just harvest-api` first",
            paths.lslar_meta.display()
        );
    };
    let lslar_meta: LsLarMeta = serde_json::from_slice(&lslar_meta_bytes)
        .with_context(|| format!("parsing {}", paths.lslar_meta.display()))?;

    let outliers = match (
        read_outliers_jsonl(&paths.outliers_jsonl),
        read_outliers_manifest(&paths.outliers_manifest),
    ) {
        (Some(records), Some(manifest)) => Some((records, manifest)),
        (None, None) => None,
        _ => anyhow::bail!(
            "inconsistent outliers state: exactly one of {} / {} exists — \
             run `just harvest-outliers` to regenerate both",
            paths.outliers_jsonl.display(),
            paths.outliers_manifest.display()
        ),
    };

    let Some(ledger) = read_ledger(&paths.phase1_ledger) else {
        anyhow::bail!(
            "no harvest-errors.jsonl at {} — run `just harvest-api` first",
            paths.phase1_ledger.display()
        );
    };
    let filtered = filtered_records(records, root, limit);

    // `phase1_manifest.file_count` is captured before `.id` is moved out
    // below (C1 fix round 1: `Coverage.phase1_files` must reflect the
    // phase-1 harvest-manifest.json total, unaffected by this run's own
    // `--root`/`--limit` — not the count of records this stats run
    // happened to load, which is already visible as Σ status_counts).
    let phase1_file_count = phase1_manifest.file_count;
    let provenance = StatsProvenance {
        phase1_manifest: phase1_manifest.id,
        phase2_manifest: zips_manifest.id,
        outliers_manifest: outliers.as_ref().map(|(_, m)| m.id.clone()),
        bootstrap_mirror: lslar_meta.mirror,
        bootstrap_last_modified: lslar_meta.last_modified,
        tool_version: tool_version(),
        git_rev: git_rev(),
    };
    let outlier_records = outliers.as_ref().map(|(records, _)| records.as_slice());
    let mut stats = build_stats(
        &filtered,
        &ledger,
        &tree,
        outlier_records,
        phase1_file_count,
        provenance,
    );
    // §8: recommendations are set *before* `stats.json` is written, so the
    // written document always carries them (never the placeholder empty
    // `vec![]` `build_stats` starts from). B2: the `zip64_statement` row
    // cross-checks the record-derived count against `wads-manifest.json`'s
    // independently-tallied `zip64_entries` — read here (not threaded
    // through `build_stats`, which stays pure over already-loaded records)
    // since `zips_manifest` is a `run_with_paths`-local I/O result.
    stats.recommendations = report::recommendations(
        &stats.idgames,
        stats.outliers.as_ref(),
        zips_manifest.zip64_entries,
    );

    // Build the sweep corpus (and render the report) entirely in memory
    // before any output file is written. Writing `stats.json` first (the
    // pre-Task-7 ordering) meant a downstream failure — e.g. sweep's URL
    // construction erroring on a malformed dir/filename — left an orphaned
    // `stats.json` on disk with no sibling outputs. Building everything up
    // front eliminates that *compute-time* orphaning: a failure before the
    // write phase below touches `out_dir` not at all. The write phase
    // itself is only per-file atomic (each write is a single
    // `atomic_write`/`write_*` call) — it is not atomic *across* the three
    // files, so an I/O failure partway through the write phase (e.g. disk
    // full after `sweep-corpus.jsonl` but before `stats.json`) can still
    // leave a partial output set on disk.
    let sweep = sweep_entries(&filtered).context(
        "building sweep-corpus entries: a phase-2 record's dir/filename could not be \
         turned into a mirror URL",
    )?;
    let report = report::render_report(&stats);

    std::fs::create_dir_all(&paths.out_dir)
        .with_context(|| format!("creating {}", paths.out_dir.display()))?;
    write_sweep_jsonl(&paths.out_dir.join("sweep-corpus.jsonl"), &sweep)?;
    write_stats_json(&paths.out_dir.join("stats.json"), &stats)?;
    atomic_write(&paths.out_dir.join("stats-report.md"), report.as_bytes()).with_context(|| {
        format!(
            "writing {}",
            paths.out_dir.join("stats-report.md").display()
        )
    })?;

    tracing::info!(
        population_entries = stats.idgames.coverage.population_entries,
        population_wads = stats.idgames.coverage.population_wads,
        sweep_entries = sweep.len(),
        "stats complete"
    );
    Ok(())
}

/// `--root`/`--limit` scoping over the loaded `idgames-wads.jsonl` records:
/// filter by `dir` prefix, sort by `id`, then truncate to the first `N`.
fn filtered_records(
    records: Vec<WadRecord>,
    root: Option<&str>,
    limit: Option<usize>,
) -> Vec<WadRecord> {
    let mut records = match root {
        Some(root) => {
            let prefix = crate::api::model::normalize_dir(root);
            records
                .into_iter()
                .filter(|r| r.dir.starts_with(&prefix))
                .collect()
        }
        None => records,
    };
    records.sort_by_key(|r| r.id);
    if let Some(limit) = limit {
        records.truncate(limit);
    }
    records
}

/// Pure §6 aggregation: build every statistic from already-loaded records.
/// `records` is the idgames population source (already `--root`/`--limit`
/// scoped by the caller); `ledger` is the Phase-1 failure ledger; `tree` is
/// the parsed ls-laR listing; `outliers` is `Some` only when both §6.4
/// outlier files are present; `phase1_file_count` is
/// `HarvestManifest::file_count` from `harvest-manifest.json` (see
/// [`Coverage::phase1_files`] — deliberately independent of `records`).
pub(crate) fn build_stats(
    records: &[WadRecord],
    ledger: &[LedgerEntry],
    tree: &ArchiveTree,
    outliers: Option<&[OutlierRecord]>,
    phase1_file_count: u64,
    provenance: StatsProvenance,
) -> StatsJson {
    StatsJson {
        schema_version: crate::schema::STATS_SCHEMA_VERSION,
        provenance,
        idgames: build_idgames_stats(records, ledger, tree, phase1_file_count),
        outliers: outliers.map(build_outliers_stats),
        recommendations: Vec::new(),
    }
}

/// §6 "unit of analysis is one `.wad`" population: records whose
/// `fetch_status` is a successful, sized outcome. `NotZip`/`FetchError`/
/// `Mirror404All`/`NoRangeSupport`/`ZipParseError` all carry no reliable
/// `.wad` size data and are excluded (but still counted in
/// [`Coverage::status_counts`]).
fn population(records: &[WadRecord]) -> Vec<&WadRecord> {
    records
        .iter()
        .filter(|r| matches!(r.fetch_status, FetchStatus::Ok | FetchStatus::FullDownload))
        .collect()
}

fn build_idgames_stats(
    records: &[WadRecord],
    ledger: &[LedgerEntry],
    tree: &ArchiveTree,
    phase1_file_count: u64,
) -> IdgamesStats {
    let population = population(records);
    let (zip_size_listing, listing_misses) = zip_size_stats(&population, tree);
    let wad_uncompressed = wad_uncompressed_size_stats(&population);
    let entries = entry_stats(&population);

    let population_entries = u64::try_from(population.len()).unwrap_or(u64::MAX);
    let population_wads = population
        .iter()
        .map(|r| u64::try_from(r.wads.len()).unwrap_or(u64::MAX))
        .sum();

    let coverage = Coverage {
        phase1_files: phase1_file_count,
        status_counts: status_counts(records),
        ledger_kinds: ledger_kind_counts(ledger),
        listing_misses,
        population_entries,
        population_wads,
    };

    IdgamesStats {
        coverage,
        wad_uncompressed,
        zip_size_listing,
        entries,
    }
}

/// §6.1/§6.2: `wads[].uncompressed` core distribution, histogram,
/// vote-weighted variant, and bucket/year segmentations — over every `.wad`
/// member of every population entry (flattened; "unit of analysis is one
/// `.wad`").
fn wad_uncompressed_size_stats(population: &[&WadRecord]) -> SizeStats {
    let mut sizes: Vec<u64> = population
        .iter()
        .flat_map(|r| r.wads.iter().map(|w| w.uncompressed))
        .collect();
    sizes.sort_unstable();
    let core = Distribution::from_sorted(&sizes);
    let histogram = histogram_buckets(&sizes);

    let mut weighted_pairs: Vec<(u64, u64)> = Vec::new();
    let mut zero_vote_members_excluded = 0_u64;
    for record in population {
        for wad in &record.wads {
            if record.votes == 0 {
                zero_vote_members_excluded += 1;
            } else {
                weighted_pairs.push((wad.uncompressed, record.votes));
            }
        }
    }
    weighted_pairs.sort_unstable_by_key(|&(value, _)| value);
    let weighted = WeightedDistribution::from_pairs(&weighted_pairs, zero_vote_members_excluded);

    let mut by_bucket: BTreeMap<String, Vec<u64>> = BTreeMap::new();
    let mut by_year: BTreeMap<String, Vec<u64>> = BTreeMap::new();
    for record in population {
        let bucket = top_bucket(&record.dir);
        let year = year_of(&record.date);
        for wad in &record.wads {
            by_bucket
                .entry(bucket.clone())
                .or_default()
                .push(wad.uncompressed);
            by_year
                .entry(year.clone())
                .or_default()
                .push(wad.uncompressed);
        }
    }
    let by_bucket = segment_distributions(by_bucket);
    let by_year = segment_distributions(by_year);

    SizeStats {
        core,
        histogram,
        weighted,
        by_bucket,
        by_year,
    }
}

/// Sort each segment's values and reduce it to a [`Distribution`].
fn segment_distributions(segments: BTreeMap<String, Vec<u64>>) -> BTreeMap<String, Distribution> {
    segments
        .into_iter()
        .map(|(key, mut values)| {
            values.sort_unstable();
            (key, Distribution::from_sorted(&values))
        })
        .collect()
}

/// §6.3/§8.1 zip-size population and the §5.0 API-vs-listing delta (§6.3):
/// over WAD-bearing population entries only. A ls-laR join hit uses the
/// listing size (and feeds `ApiDelta`); a miss falls back to the API
/// `zip_size` and increments the returned miss count
/// ([`Coverage::listing_misses`]).
fn zip_size_stats(population: &[&WadRecord], tree: &ArchiveTree) -> (ZipSizeStats, u64) {
    let mut sizes: Vec<u64> = Vec::new();
    let mut deltas: Vec<u64> = Vec::new();
    let mut entries_compared = 0_u64;
    let mut mismatched = 0_u64;
    let mut max_relative = 0.0_f64;
    let mut listing_misses = 0_u64;

    for record in population.iter().filter(|r| !r.wads.is_empty()) {
        if let Some(listing) = tree.size_of(&record.dir, &record.filename) {
            sizes.push(listing);
            entries_compared += 1;
            if listing != record.zip_size {
                mismatched += 1;
                let delta = listing.abs_diff(record.zip_size);
                deltas.push(delta);
                if listing > 0 {
                    max_relative = max_relative.max(ratio_f64(delta, listing));
                }
            }
        } else {
            sizes.push(record.zip_size);
            listing_misses += 1;
        }
    }
    sizes.sort_unstable();
    deltas.sort_unstable();

    let core = Distribution::from_sorted(&sizes);
    let histogram = histogram_buckets(&sizes);
    let api_delta = ApiDelta {
        entries_compared,
        mismatched,
        max_abs_delta: deltas.last().copied().unwrap_or(0),
        p50_abs_delta: percentiles::nearest_rank(&deltas, percentiles::P50),
        p99_abs_delta: percentiles::nearest_rank(&deltas, percentiles::P99),
        max_relative,
    };

    (
        ZipSizeStats {
            core,
            histogram,
            api_delta,
        },
        listing_misses,
    )
}

/// §6.3 decision-driving entry-level counts and distributions.
fn entry_stats(population: &[&WadRecord]) -> EntryStats {
    let zip_entries = u64::try_from(population.len()).unwrap_or(u64::MAX);
    let zero_wad =
        u64::try_from(population.iter().filter(|r| r.wads.is_empty()).count()).unwrap_or(u64::MAX);
    let multi_wad =
        u64::try_from(population.iter().filter(|r| r.wads.len() > 1).count()).unwrap_or(u64::MAX);

    let mut member_counts: Vec<u64> = population.iter().map(|r| r.member_count).collect();
    member_counts.sort_unstable();
    let member_count = Distribution::from_sorted(&member_counts);

    let mut entry_totals: Vec<u64> = population
        .iter()
        .map(|r| r.wads.iter().map(|w| w.uncompressed).sum())
        .collect();
    entry_totals.sort_unstable();
    let entry_wad_total_uncompressed = Distribution::from_sorted(&entry_totals);

    let ratios = ratio_stats(population);

    let mut methods: BTreeMap<String, u64> = BTreeMap::new();
    let mut zip64_entries = 0_u64;
    let mut encrypted_members = 0_u64;
    let mut wad_named_other_members = 0_u64;
    for record in population {
        if record.zip64 {
            zip64_entries += 1;
        }
        for wad in &record.wads {
            *methods.entry(wad.method.clone()).or_insert(0) += 1;
            if wad.encrypted {
                encrypted_members += 1;
            }
        }
        for name in &record.other_members {
            if name.to_ascii_lowercase().ends_with(".wad") {
                wad_named_other_members += 1;
            }
        }
    }

    EntryStats {
        zip_entries,
        zero_wad,
        zero_wad_share: ratio_f64(zero_wad, zip_entries),
        multi_wad,
        multi_wad_share: ratio_f64(multi_wad, zip_entries),
        member_count,
        entry_wad_total_uncompressed,
        ratios,
        methods,
        zip64_entries,
        encrypted_members,
        wad_named_other_members,
    }
}

/// §6.3 compression-ratio populations. A member with `compressed == 0 &&
/// uncompressed > 0` can't yield a ratio — it's counted in
/// `zero_compressed_anomalies` and excluded from both the per-member
/// (`deflate`-only) and per-entry (`Σ uncompressed / Σ compressed`, summed
/// over the entry's *other* members) populations. An entry whose surviving
/// members sum to `0` compressed bytes (no wads, or every member excluded)
/// is likewise excluded from `per_entry` — there's nothing to divide by.
fn ratio_stats(population: &[&WadRecord]) -> RatioStats {
    let mut deflate_pairs: Vec<(u64, u64)> = Vec::new();
    let mut per_entry_pairs: Vec<(u64, u64)> = Vec::new();
    let mut zero_compressed_anomalies = 0_u64;

    for record in population {
        let mut entry_uncompressed = 0_u64;
        let mut entry_compressed = 0_u64;
        for wad in &record.wads {
            // A genuine anomaly (compressed 0, uncompressed > 0) is counted
            // and excluded from every ratio population below. A member with
            // compressed == 0 and uncompressed == 0 (a `.wad` member) is NOT
            // an anomaly, but still can't yield a ratio — I1 fix round 1: it
            // must be excluded from `member_deflate` too (an unguarded 0/0
            // pair would be a NaN ratio, which serializes as JSON `null` and
            // makes an intransitive `sort_ratio_pairs` comparator). Spec
            // population for `member_deflate` is "method == deflate AND
            // compressed > 0".
            if wad.compressed == 0 && wad.uncompressed > 0 {
                zero_compressed_anomalies += 1;
                continue;
            }
            if wad.method == "deflate" && wad.compressed > 0 {
                deflate_pairs.push((wad.uncompressed, wad.compressed));
            }
            entry_uncompressed += wad.uncompressed;
            entry_compressed += wad.compressed;
        }
        if entry_compressed > 0 {
            per_entry_pairs.push((entry_uncompressed, entry_compressed));
        }
    }

    percentiles::sort_ratio_pairs(&mut deflate_pairs);
    percentiles::sort_ratio_pairs(&mut per_entry_pairs);

    RatioStats {
        member_deflate: RatioDistribution::from_sorted_pairs(&deflate_pairs),
        per_entry: RatioDistribution::from_sorted_pairs(&per_entry_pairs),
        zero_compressed_anomalies,
    }
}

/// §6.4 modern-outliers supplement: `Ok` records are analyzed, everything
/// else is skipped (with its `fetch_status` recorded so the report can
/// explain the gap). Both lists are sorted by `slug` for a deterministic
/// `stats.json`.
fn build_outliers_stats(records: &[OutlierRecord]) -> OutliersStats {
    let mut analyzed_records: Vec<&OutlierRecord> = records
        .iter()
        .filter(|r| r.fetch_status == FetchStatus::Ok)
        .collect();
    analyzed_records.sort_by(|a, b| a.slug.cmp(&b.slug));
    let mut skipped_records: Vec<&OutlierRecord> = records
        .iter()
        .filter(|r| r.fetch_status != FetchStatus::Ok)
        .collect();
    skipped_records.sort_by(|a, b| a.slug.cmp(&b.slug));

    let analyzed: Vec<OutlierSummary> = analyzed_records
        .iter()
        .map(|record| OutlierSummary {
            slug: record.slug.clone(),
            zip_size: record.zip_size,
            member_count: record.member_count,
            wad_count: u64::try_from(record.wads.len()).unwrap_or(u64::MAX),
            max_wad_uncompressed: record
                .wads
                .iter()
                .map(|w| w.uncompressed)
                .max()
                .unwrap_or(0),
            total_wad_uncompressed: record.wads.iter().map(|w| w.uncompressed).sum(),
        })
        .collect();
    let skipped: Vec<OutlierSkip> = skipped_records
        .iter()
        .map(|record| OutlierSkip {
            slug: record.slug.clone(),
            fetch_status: wire_label(record.fetch_status),
        })
        .collect();

    let mut wad_values: Vec<u64> = analyzed_records
        .iter()
        .flat_map(|r| r.wads.iter().map(|w| w.uncompressed))
        .collect();
    wad_values.sort_unstable();
    let wad_uncompressed = Distribution::from_sorted(&wad_values);

    let max_zip_size = analyzed.iter().map(|s| s.zip_size).max().unwrap_or(0);
    let max_member_count = analyzed.iter().map(|s| s.member_count).max().unwrap_or(0);
    let max_entry_total_uncompressed = analyzed
        .iter()
        .map(|s| s.total_wad_uncompressed)
        .max()
        .unwrap_or(0);

    OutliersStats {
        analyzed,
        skipped,
        wad_uncompressed,
        max_zip_size,
        max_member_count,
        max_entry_total_uncompressed,
    }
}

/// Record count per `fetch_status` wire value, over every loaded record
/// (mirrors `zips::status_counts`'s convention — a separate copy, like
/// `outliers::status_counts`, since both are private to their own module).
fn status_counts(records: &[WadRecord]) -> BTreeMap<String, u64> {
    let mut counts = BTreeMap::new();
    for record in records {
        *counts
            .entry(wire_label(record.fetch_status))
            .or_insert(0_u64) += 1;
    }
    counts
}

/// Phase-1 `harvest-errors.jsonl` entry count per [`crate::schema::LedgerKind`] wire value.
fn ledger_kind_counts(ledger: &[LedgerEntry]) -> BTreeMap<String, u64> {
    let mut counts = BTreeMap::new();
    for entry in ledger {
        let label = serde_json::to_value(entry.kind)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_default();
        *counts.entry(label).or_insert(0_u64) += 1;
    }
    counts
}

/// A `FetchStatus`'s `snake_case` wire value, as it serializes.
fn wire_label(status: FetchStatus) -> String {
    serde_json::to_value(status)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

fn histogram_buckets(sorted: &[u64]) -> Vec<HistogramBucket> {
    percentiles::log2_histogram(sorted)
        .into_iter()
        .map(|(label, count)| HistogramBucket { label, count })
        .collect()
}

/// `numerator / denominator` as `f64`, or `0.0` when `denominator` is `0`
/// (an empty population has no share/ratio to report).
#[allow(
    clippy::cast_precision_loss,
    reason = "reporting a byte-count share/relative-delta as f64; corpus sizes are well within \
              f64's exact integer range in practice, matching percentiles.rs's existing precedent"
)]
fn ratio_f64(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

impl Distribution {
    /// Build a [`Distribution`] from an ascending-sorted `u64` slice
    /// (§6.1). Empty input yields every field `0`/`0.0` — forwards
    /// `percentiles::nearest_rank`/`mean_stddev`'s own empty-slice
    /// conventions.
    fn from_sorted(sorted: &[u64]) -> Distribution {
        let (mean, stddev) = percentiles::mean_stddev(sorted);
        Distribution {
            n: u64::try_from(sorted.len()).unwrap_or(u64::MAX),
            min: sorted.first().copied().unwrap_or(0),
            p50: percentiles::nearest_rank(sorted, percentiles::P50),
            p75: percentiles::nearest_rank(sorted, percentiles::P75),
            p90: percentiles::nearest_rank(sorted, percentiles::P90),
            p95: percentiles::nearest_rank(sorted, percentiles::P95),
            p99: percentiles::nearest_rank(sorted, percentiles::P99),
            p99_5: percentiles::nearest_rank(sorted, percentiles::P99_5),
            p99_9: percentiles::nearest_rank(sorted, percentiles::P99_9),
            max: sorted.last().copied().unwrap_or(0),
            mean,
            stddev,
        }
    }
}

impl WeightedDistribution {
    /// Build a [`WeightedDistribution`] from `(value, weight)` pairs sorted
    /// ascending by value (§6.2). `min`/`max`/`n` describe the value domain
    /// (unweighted); percentiles/mean/stddev are vote-weighted.
    fn from_pairs(sorted_pairs: &[(u64, u64)], zero_vote_members_excluded: u64) -> Self {
        let (mean, stddev) = percentiles::weighted_mean_stddev(sorted_pairs);
        let min = sorted_pairs.first().map_or(0, |&(v, _)| v);
        let max = sorted_pairs.last().map_or(0, |&(v, _)| v);
        let total_votes: u64 = sorted_pairs.iter().map(|&(_, w)| w).sum();
        let core = Distribution {
            n: u64::try_from(sorted_pairs.len()).unwrap_or(u64::MAX),
            min,
            p50: percentiles::weighted_nearest_rank(sorted_pairs, percentiles::P50),
            p75: percentiles::weighted_nearest_rank(sorted_pairs, percentiles::P75),
            p90: percentiles::weighted_nearest_rank(sorted_pairs, percentiles::P90),
            p95: percentiles::weighted_nearest_rank(sorted_pairs, percentiles::P95),
            p99: percentiles::weighted_nearest_rank(sorted_pairs, percentiles::P99),
            p99_5: percentiles::weighted_nearest_rank(sorted_pairs, percentiles::P99_5),
            p99_9: percentiles::weighted_nearest_rank(sorted_pairs, percentiles::P99_9),
            max,
            mean,
            stddev,
        };
        WeightedDistribution {
            core,
            total_votes,
            zero_vote_members_excluded,
        }
    }
}

impl RatioDistribution {
    /// Build a [`RatioDistribution`] from `(uncompressed, compressed)` pairs
    /// already ordered by [`percentiles::sort_ratio_pairs`]. Empty input
    /// yields every field `0`/`0.0`.
    #[allow(
        clippy::cast_precision_loss,
        reason = "reporting a single (uncompressed, compressed) pair's ratio as f64, matching \
                  percentiles::ratio_at's existing precedent"
    )]
    fn from_sorted_pairs(sorted: &[(u64, u64)]) -> Self {
        let Some(&(min_u, min_c)) = sorted.first() else {
            return RatioDistribution {
                n: 0,
                min: 0.0,
                p50: 0.0,
                p90: 0.0,
                p99: 0.0,
                max: 0.0,
            };
        };
        let (max_u, max_c) = *sorted
            .last()
            .expect("first() succeeded, so last() does too");
        debug_assert!(
            sorted.iter().all(|&(_, c)| c > 0),
            "ratio population must never contain a zero-compressed pair (NaN ratio) — \
             I1 fix round 1: callers must gate on compressed > 0 before this point"
        );
        RatioDistribution {
            n: u64::try_from(sorted.len()).unwrap_or(u64::MAX),
            min: min_u as f64 / min_c as f64,
            p50: percentiles::ratio_at(sorted, percentiles::P50),
            p90: percentiles::ratio_at(sorted, percentiles::P90),
            p99: percentiles::ratio_at(sorted, percentiles::P99),
            max: max_u as f64 / max_c as f64,
        }
    }
}

/// §6.2 top-level segmentation bucket for a `dir` such as
/// `"levels/doom2/Ports/megawads/"` or
/// `"levels/doom2/deathmatch/Ports/single/"`: `"levels/<game>"` normally, or
/// `"levels/<game>/Ports"` when the path runs through a `Ports/` subtree at
/// *any* depth below `levels/<game>/` (§4.2: "there is no top-level
/// `ports/` — the per-game `levels/*/Ports/` subtrees"). Review fix I1: the
/// real corpus has `Ports/` one level deeper than the common case —
/// `levels/doom2/deathmatch/Ports/` (500 entries) and
/// `levels/doom/deathmatch/Ports/` (8) — so this checks every segment past
/// `levels/<game>/`, not just the third one; a `Ports` match only at depth
/// 3 silently left those 508 entries in the vanilla `levels/<game>` bucket,
/// contaminating the §6.2 segmentation. Non-`levels` roots (`combos/`,
/// `themes/x/`) collapse to their own top-level segment.
pub(crate) fn top_bucket(dir: &str) -> String {
    let segments: Vec<&str> = dir.split('/').filter(|s| !s.is_empty()).collect();
    let Some(&first) = segments.first() else {
        return String::new();
    };
    if first != "levels" {
        return first.to_owned();
    }
    let Some(&game) = segments.get(1) else {
        return first.to_owned();
    };
    if segments[2..].contains(&"Ports") {
        format!("{first}/{game}/Ports")
    } else {
        format!("{first}/{game}")
    }
}

/// §6.2 year segmentation: the `YYYY` prefix of a `date` field shaped
/// `"YYYY-MM-DD"`. Anything that doesn't start with 4 ASCII digits followed
/// by `-` (including an empty string) maps to `"unknown"` rather than
/// panicking on a short slice.
pub(crate) fn year_of(date: &str) -> String {
    let bytes = date.as_bytes();
    if bytes.len() >= 5 && bytes[..4].iter().all(u8::is_ascii_digit) && bytes[4] == b'-' {
        date[..4].to_owned()
    } else {
        "unknown".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{LedgerKind, OutliersManifest, WadMember};

    fn wad_member(name: &str, compressed: u64, uncompressed: u64) -> WadMember {
        WadMember {
            name: name.into(),
            compressed,
            uncompressed,
            method: "deflate".into(),
            encrypted: false,
        }
    }

    fn wad_record(id: u64, fetch_status: FetchStatus, wads: Vec<WadMember>) -> WadRecord {
        WadRecord {
            id,
            dir: "levels/doom/0-9/".into(),
            filename: format!("f{id}.zip"),
            zip_size: 1_000,
            date: "2019-04-02".into(),
            rating: None,
            votes: 0,
            is_zip: true,
            zip64: false,
            member_count: u64::try_from(wads.len()).unwrap(),
            wads,
            other_members: Vec::new(),
            mirror: "infania".into(),
            fetch_status,
        }
    }

    fn dummy_provenance() -> StatsProvenance {
        StatsProvenance {
            phase1_manifest: "harvest-1".into(),
            phase2_manifest: "harvest-zips-1".into(),
            outliers_manifest: None,
            bootstrap_mirror: "infania".into(),
            bootstrap_last_modified: None,
            tool_version: "0.0.0".into(),
            git_rev: None,
        }
    }

    /// Most tests don't care about `Coverage.phase1_files` independently of
    /// the fixture's own record count, so this convenience wrapper sets
    /// `phase1_file_count` to `records.len()` — tests that specifically
    /// exercise the C1 fix-round-1 independence (`phase1_files` sourced
    /// from the phase-1 manifest, not from the loaded record count) use
    /// [`build_stats_fixture_with_phase1_files`] instead.
    fn build_stats_fixture(records: &[WadRecord]) -> StatsJson {
        build_stats_fixture_with_phase1_files(records, u64::try_from(records.len()).unwrap())
    }

    fn build_stats_fixture_with_phase1_files(
        records: &[WadRecord],
        phase1_file_count: u64,
    ) -> StatsJson {
        build_stats(
            records,
            &[],
            &ArchiveTree::default(),
            None,
            phase1_file_count,
            dummy_provenance(),
        )
    }

    // ---- top_bucket / year_of ----

    #[test]
    fn bucket_and_year_rules() {
        assert_eq!(
            top_bucket("levels/doom2/Ports/megawads/"),
            "levels/doom2/Ports"
        );
        assert_eq!(top_bucket("levels/doom2/a-c/"), "levels/doom2");
        assert_eq!(top_bucket("combos/"), "combos");
        assert_eq!(top_bucket("themes/x/"), "themes");
        assert_eq!(year_of("2019-04-02"), "2019");
        assert_eq!(year_of(""), "unknown");
        assert_eq!(year_of("19-4-2"), "unknown");
        // I1: the real corpus has `Ports/` one level deeper than the common
        // case — `levels/doom2/deathmatch/Ports/` (500 entries) and
        // `levels/doom/deathmatch/Ports/` (8) — top_bucket must still
        // collapse these to the `Ports` bucket, not the vanilla one.
        assert_eq!(
            top_bucket("levels/doom2/deathmatch/Ports/megawads/"),
            "levels/doom2/Ports"
        );
        assert_eq!(
            top_bucket("levels/doom/deathmatch/Ports/"),
            "levels/doom/Ports"
        );
    }

    #[test]
    fn top_bucket_handles_bare_and_deep_levels_paths() {
        // levels/<game> alone (no third segment) stays as-is.
        assert_eq!(top_bucket("levels/hexen/"), "levels/hexen");
        // levels/<game>/Ports with more depth still collapses to 3 segments.
        assert_eq!(
            top_bucket("levels/heretic/Ports/single/"),
            "levels/heretic/Ports"
        );
    }

    // ---- populations ----

    #[test]
    fn populations_respect_status_and_flags() {
        let recs = vec![
            wad_record(1, FetchStatus::Ok, vec![wad_member("A.WAD", 50, 100)]),
            wad_record(
                2,
                FetchStatus::FullDownload,
                vec![wad_member("B.WAD", 10, 200)],
            ),
            wad_record(3, FetchStatus::FetchError, vec![]),
            {
                let mut r = wad_record(4, FetchStatus::Ok, vec![]);
                r.other_members = vec!["SNEAKY.WAD".into()];
                r
            },
        ];
        // C1 fix round 1: `phase1_files` must come from the phase-1
        // manifest's `file_count`, independent of how many records this
        // stats run happened to load — pass a value (10) deliberately
        // distinct from `recs.len()` (4) to prove the two are decoupled.
        let stats = build_stats_fixture_with_phase1_files(&recs, 10);
        assert_eq!(stats.idgames.coverage.phase1_files, 10);
        assert_eq!(stats.idgames.coverage.population_entries, 3); // ids 1, 2, 4
        assert_eq!(stats.idgames.coverage.population_wads, 2);
        assert_eq!(stats.idgames.entries.zip_entries, 3);
        assert_eq!(stats.idgames.entries.zero_wad, 1);
        assert_eq!(stats.idgames.entries.wad_named_other_members, 1);
        assert_eq!(stats.idgames.wad_uncompressed.core.max, 200);
        assert_eq!(
            stats.idgames.coverage.status_counts.get("fetch_error"),
            Some(&1)
        );
        assert_eq!(stats.idgames.coverage.status_counts.get("ok"), Some(&2));
    }

    #[test]
    fn zero_and_multi_wad_shares() {
        let recs = vec![
            wad_record(1, FetchStatus::Ok, vec![]), // zero
            wad_record(
                2,
                FetchStatus::Ok,
                vec![wad_member("A.WAD", 1, 1), wad_member("B.WAD", 1, 1)],
            ), // multi
            wad_record(3, FetchStatus::Ok, vec![wad_member("C.WAD", 1, 1)]), // single
            wad_record(4, FetchStatus::Ok, vec![]), // zero
        ];
        let stats = build_stats_fixture(&recs);
        assert_eq!(stats.idgames.entries.zero_wad, 2);
        assert!((stats.idgames.entries.zero_wad_share - 0.5).abs() < 1e-12);
        assert_eq!(stats.idgames.entries.multi_wad, 1);
        assert!((stats.idgames.entries.multi_wad_share - 0.25).abs() < 1e-12);
    }

    #[test]
    fn entry_wad_total_uncompressed_sums_per_entry() {
        let recs = vec![wad_record(
            1,
            FetchStatus::Ok,
            vec![wad_member("A.WAD", 10, 100), wad_member("B.WAD", 10, 50)],
        )];
        let stats = build_stats_fixture(&recs);
        assert_eq!(stats.idgames.entries.entry_wad_total_uncompressed.max, 150);
        assert_eq!(stats.idgames.entries.entry_wad_total_uncompressed.n, 1);
    }

    #[test]
    fn methods_zip64_and_encrypted_are_tallied() {
        let mut r1 = wad_record(1, FetchStatus::Ok, vec![wad_member("A.WAD", 10, 100)]);
        r1.zip64 = true;
        let mut encrypted = wad_member("B.WAD", 10, 100);
        encrypted.encrypted = true;
        encrypted.method = "stored".into();
        let r2 = wad_record(2, FetchStatus::Ok, vec![encrypted]);
        let stats = build_stats_fixture(&[r1, r2]);
        assert_eq!(stats.idgames.entries.zip64_entries, 1);
        assert_eq!(stats.idgames.entries.encrypted_members, 1);
        assert_eq!(stats.idgames.entries.methods.get("deflate"), Some(&1));
        assert_eq!(stats.idgames.entries.methods.get("stored"), Some(&1));
    }

    // ---- ratios ----

    #[test]
    fn ratio_stats_deflate_only_and_per_entry_and_anomalies() {
        // Entry 1: one deflate member (200/100 = 2.0) and one stored member
        // (50/50 = 1.0) — member_deflate sees only the deflate member;
        // per_entry sees the entry aggregate (250/150).
        let mut stored = wad_member("B.WAD", 50, 50);
        stored.method = "stored".into();
        let r1 = wad_record(
            1,
            FetchStatus::Ok,
            vec![wad_member("A.WAD", 100, 200), stored],
        );
        // Entry 2: one anomalous member (compressed 0, uncompressed > 0) —
        // excluded from every ratio population and counted as an anomaly;
        // the entry itself then has 0 total compressed bytes, so it's
        // excluded from per_entry too.
        let r2 = wad_record(2, FetchStatus::Ok, vec![wad_member("C.WAD", 0, 500)]);
        // Entry 3 (I1 fix round 1 regression): a deflate member with
        // compressed == 0 AND uncompressed == 0 — NOT an anomaly (the
        // anomaly definition requires uncompressed > 0), but still must be
        // excluded from `member_deflate` (a 0/0 ratio is NaN, which breaks
        // JSON round-tripping and `sort_ratio_pairs`'s comparator). Must not
        // bump `zero_compressed_anomalies`, and the entry itself (0 total
        // compressed bytes) must not enter `per_entry` either.
        let r3 = wad_record(3, FetchStatus::Ok, vec![wad_member("Z.WAD", 0, 0)]);
        let stats = build_stats_fixture(&[r1, r2, r3]);

        let ratios = &stats.idgames.entries.ratios;
        assert_eq!(ratios.zero_compressed_anomalies, 1);
        assert_eq!(ratios.member_deflate.n, 1);
        assert!((ratios.member_deflate.max - 2.0).abs() < 1e-12);
        assert!(ratios.member_deflate.min.is_finite());
        assert!(ratios.member_deflate.max.is_finite());
        assert_eq!(ratios.per_entry.n, 1);
        assert!((ratios.per_entry.max - (250.0 / 150.0)).abs() < 1e-9);
    }

    // ---- weighted distribution ----

    #[test]
    fn weighted_distribution_shifts_toward_high_vote_and_excludes_zero_vote() {
        let mut low = wad_record(1, FetchStatus::Ok, vec![wad_member("A.WAD", 1, 10)]);
        low.votes = 1;
        let mut mid = wad_record(2, FetchStatus::Ok, vec![wad_member("B.WAD", 1, 20)]);
        mid.votes = 1;
        let mut high = wad_record(3, FetchStatus::Ok, vec![wad_member("C.WAD", 1, 30)]);
        high.votes = 98;
        let mut zero = wad_record(4, FetchStatus::Ok, vec![wad_member("D.WAD", 1, 999)]);
        zero.votes = 0;

        let stats = build_stats_fixture(&[low, mid, high, zero]);
        let weighted = &stats.idgames.wad_uncompressed.weighted;
        assert_eq!(weighted.zero_vote_members_excluded, 1);
        assert_eq!(weighted.total_votes, 100);
        assert_eq!(weighted.core.p50, 30); // weighted p50 shifts to the high-vote value
        assert_eq!(stats.idgames.wad_uncompressed.core.p50, 20); // unweighted (999 excluded from neither — it's still in `core`)
    }

    // ---- segmentation ----

    #[test]
    fn size_stats_segments_by_bucket_and_year() {
        let mut a = wad_record(1, FetchStatus::Ok, vec![wad_member("A.WAD", 1, 100)]);
        a.dir = "levels/doom2/Ports/megawads/".into();
        a.date = "2019-04-02".into();
        let mut b = wad_record(2, FetchStatus::Ok, vec![wad_member("B.WAD", 1, 10)]);
        b.dir = "levels/doom2/a-c/".into();
        b.date = "2003-06-02".into();

        let stats = build_stats_fixture(&[a, b]);
        let by_bucket = &stats.idgames.wad_uncompressed.by_bucket;
        assert_eq!(by_bucket["levels/doom2/Ports"].max, 100);
        assert_eq!(by_bucket["levels/doom2"].max, 10);
        let by_year = &stats.idgames.wad_uncompressed.by_year;
        assert_eq!(by_year["2019"].max, 100);
        assert_eq!(by_year["2003"].max, 10);
    }

    #[test]
    fn build_stats_segments_unknown_year_non_levels_bucket_and_deep_ports() {
        // I1/T6-M4 (deferred from Task 6): exercise by_year "unknown"
        // (missing/malformed date), a non-`levels` top-level bucket, and a
        // `Ports` subtree below depth 3 — all through `build_stats` (not
        // just `top_bucket`/`year_of` in isolation), asserting the
        // resulting `by_bucket` keys directly.
        let mut deep_ports = wad_record(1, FetchStatus::Ok, vec![wad_member("A.WAD", 1, 100)]);
        deep_ports.dir = "levels/doom2/deathmatch/Ports/".into();
        deep_ports.date = "not-a-date".into(); // malformed → "unknown"
        let mut no_date = wad_record(2, FetchStatus::Ok, vec![wad_member("B.WAD", 1, 50)]);
        no_date.dir = "levels/doom/0-9/".into();
        no_date.date = String::new(); // missing → "unknown"
        let mut combo = wad_record(3, FetchStatus::Ok, vec![wad_member("C.WAD", 1, 200)]);
        combo.dir = "combos/".into();
        combo.date = "2020-01-01".into();

        let stats = build_stats_fixture(&[deep_ports, no_date, combo]);

        let by_bucket = &stats.idgames.wad_uncompressed.by_bucket;
        assert!(
            by_bucket.contains_key("levels/doom2/Ports"),
            "{by_bucket:?}"
        );
        assert!(by_bucket.contains_key("levels/doom"), "{by_bucket:?}");
        assert!(by_bucket.contains_key("combos"), "{by_bucket:?}");
        assert!(
            !by_bucket.contains_key("levels/doom2"),
            "deep-Ports entry must not also land in the vanilla levels/doom2 bucket: {by_bucket:?}"
        );

        let by_year = &stats.idgames.wad_uncompressed.by_year;
        assert!(by_year.contains_key("unknown"), "{by_year:?}");
        assert_eq!(by_year["unknown"].n, 2); // deep_ports + no_date
        assert!(by_year.contains_key("2020"), "{by_year:?}");
    }

    // ---- ls-laR join ----

    #[test]
    fn listing_join_prefers_lslar_and_counts_misses() {
        let mut tree = ArchiveTree::default();
        tree.dirs.insert(
            "levels/doom/0-9/".into(),
            vec![
                crate::lslar::TreeFile {
                    name: "f1.zip".into(),
                    size: 150,
                },
                crate::lslar::TreeFile {
                    name: "f3.zip".into(),
                    size: 999,
                },
            ],
        );
        // f2.zip is deliberately absent from the tree's listing (join miss).

        let mut hit = wad_record(1, FetchStatus::Ok, vec![wad_member("A.WAD", 1, 1)]);
        hit.zip_size = 100; // API says 100; ls-laR says 150 → mismatch, delta 50
        let mut miss = wad_record(2, FetchStatus::Ok, vec![wad_member("B.WAD", 1, 1)]);
        miss.zip_size = 777; // no tree entry → falls back to this API size
        // I4 fix round 1: a join HIT that MATCHES (listing == API size) — the
        // common case (~93% of the real corpus per the review). Without this
        // fixture, a bug that pushed a 0 delta for every match would still
        // pass every assertion below (a 0 blends invisibly into `deltas`) —
        // this fixture instead keeps a matching entry entirely OUT of
        // `deltas`/`mismatched`, so such a bug would show up as an
        // unexpected `mismatched`/`p50_abs_delta` shift.
        let mut matched = wad_record(3, FetchStatus::Ok, vec![wad_member("C.WAD", 1, 1)]);
        matched.zip_size = 999; // API agrees exactly with the ls-laR listing

        let stats = build_stats(
            &[hit, miss, matched],
            &[],
            &tree,
            None,
            3,
            dummy_provenance(),
        );

        assert_eq!(stats.idgames.coverage.listing_misses, 1);
        let delta = &stats.idgames.zip_size_listing.api_delta;
        assert_eq!(delta.entries_compared, 2); // hit + matched (both joined); miss did not
        assert_eq!(delta.mismatched, 1); // only hit disagreed
        assert_eq!(delta.max_abs_delta, 50);
        assert_eq!(delta.p50_abs_delta, 50);
        assert!((delta.max_relative - 50.0 / 150.0).abs() < 1e-12);
        // Population uses 150 (the listing) for the hit, 777 (the API
        // fallback) for the miss, and 999 (listing == API) for the match.
        assert_eq!(stats.idgames.zip_size_listing.core.max, 999);
        assert_eq!(stats.idgames.zip_size_listing.core.min, 150);
    }

    #[test]
    fn zip_size_population_excludes_zero_wad_entries() {
        let mut tree = ArchiveTree::default();
        tree.dirs.insert(
            "levels/doom/0-9/".into(),
            vec![crate::lslar::TreeFile {
                name: "f1.zip".into(),
                size: 999,
            }],
        );
        let zero_wad = wad_record(1, FetchStatus::Ok, vec![]);
        let stats = build_stats(&[zero_wad], &[], &tree, None, 1, dummy_provenance());
        assert_eq!(stats.idgames.zip_size_listing.core.n, 0);
        assert_eq!(stats.idgames.coverage.listing_misses, 0);
    }

    // ---- ledger coverage ----

    #[test]
    fn ledger_kinds_are_tallied() {
        let ledger = vec![
            LedgerEntry {
                path: "levels/a/".into(),
                action: "getcontents".into(),
                kind: LedgerKind::HttpError,
                detail: "d".into(),
                attempts: 1,
            },
            LedgerEntry {
                path: "levels/b/".into(),
                action: "getcontents".into(),
                kind: LedgerKind::HttpError,
                detail: "d".into(),
                attempts: 1,
            },
            LedgerEntry {
                path: "levels/c/".into(),
                action: "getcontents".into(),
                kind: LedgerKind::SuspectPath,
                detail: "d".into(),
                attempts: 1,
            },
        ];
        let stats = build_stats(
            &[],
            &ledger,
            &ArchiveTree::default(),
            None,
            0,
            dummy_provenance(),
        );
        assert_eq!(
            stats.idgames.coverage.ledger_kinds.get("http_error"),
            Some(&2)
        );
        assert_eq!(
            stats.idgames.coverage.ledger_kinds.get("suspect_path"),
            Some(&1)
        );
    }

    // ---- outliers ----

    fn outlier_record(slug: &str, status: FetchStatus, wads: Vec<WadMember>) -> OutlierRecord {
        OutlierRecord {
            slug: slug.into(),
            url: format!("https://example.com/{slug}.zip"),
            zip_size: 1_000_000,
            zip64: false,
            member_count: u64::try_from(wads.len()).unwrap(),
            wads,
            other_members: Vec::new(),
            fetch_status: status,
        }
    }

    #[test]
    fn outliers_stats_analyzed_skipped_and_maxima() {
        let recs = vec![
            outlier_record(
                "zeta",
                FetchStatus::Ok,
                vec![wad_member("A.WAD", 1, 500), wad_member("B.WAD", 1, 100)],
            ),
            outlier_record("alpha", FetchStatus::Ok, vec![wad_member("C.WAD", 1, 50)]),
            outlier_record("skipped-one", FetchStatus::NoRangeSupport, vec![]),
        ];
        let stats = build_stats(
            &[],
            &[],
            &ArchiveTree::default(),
            Some(&recs),
            0,
            dummy_provenance(),
        );
        let outliers = stats.outliers.expect("outliers present");
        assert_eq!(
            outliers
                .analyzed
                .iter()
                .map(|s| s.slug.as_str())
                .collect::<Vec<_>>(),
            vec!["alpha", "zeta"] // sorted by slug
        );
        assert_eq!(outliers.skipped.len(), 1);
        assert_eq!(outliers.skipped[0].slug, "skipped-one");
        assert_eq!(outliers.skipped[0].fetch_status, "no_range_support");
        assert_eq!(outliers.max_zip_size, 1_000_000);
        assert_eq!(outliers.max_member_count, 2);
        assert_eq!(outliers.max_entry_total_uncompressed, 600); // zeta: 500+100
        assert_eq!(outliers.wad_uncompressed.n, 3); // 2 from zeta + 1 from alpha
        assert_eq!(outliers.wad_uncompressed.max, 500);
    }

    #[test]
    fn outliers_none_when_absent_some_empty_when_present() {
        // build_stats itself only sees the already-resolved Option: `None`
        // for "no supplement", `Some(&[])` for "present but zero records".
        // The absent-vs-empty file distinction lives in run_with_paths's
        // I/O layer (see run_with_paths tests below).
        let absent = build_stats(
            &[],
            &[],
            &ArchiveTree::default(),
            None,
            0,
            dummy_provenance(),
        );
        assert!(absent.outliers.is_none());

        let empty = build_stats(
            &[],
            &[],
            &ArchiveTree::default(),
            Some(&[]),
            0,
            dummy_provenance(),
        );
        let outliers = empty
            .outliers
            .expect("Some(&[]) must still produce Some(OutliersStats)");
        assert!(outliers.analyzed.is_empty());
        assert!(outliers.skipped.is_empty());
        assert_eq!(outliers.wad_uncompressed.n, 0);
    }

    // ---- run_with_paths I/O ----

    /// A tiny gzipped ls-laR listing: one `.zip` in `levels/doom/0-9/`.
    /// Matches the format `lslar.rs`'s own tests parse.
    fn ls_lar_gz_fixture() -> Vec<u8> {
        let text = "\
.:
total 1
drwxr-xr-x 3 ftp ftp 4096 Jan  1  2020 levels

./levels:
total 1
drwxr-xr-x 3 ftp ftp 4096 Jan  1  2020 doom

./levels/doom:
total 1
drwxr-xr-x 2 ftp ftp 4096 Jan  1  2020 0-9

./levels/doom/0-9:
total 1
-rw-r--r--  1 ftp ftp 500 Jan  1  2020 f1.zip
";
        let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        std::io::Write::write_all(&mut enc, text.as_bytes()).unwrap();
        enc.finish().unwrap()
    }

    /// Write every required input under `dir`, returning the assembled
    /// [`StatsPaths`]. `dir` doubles as both `out_dir` and the cache root's
    /// parent, mirroring a real (unscoped) layout closely enough for a
    /// self-contained fixture.
    /// The original 2-record, single-dir fixture every pre-existing
    /// `run_with_paths` test uses. `phase1_file_count` (the phase-1
    /// manifest's `file_count`) intentionally matches the record count here
    /// — [`write_fixture_inputs_with_records`] is used directly wherever a
    /// test needs the two to diverge (I3: `--root` scoping; C1: the
    /// `phase1_files` independence check).
    fn write_fixture_inputs(dir: &std::path::Path) -> StatsPaths {
        write_fixture_inputs_with_records(
            dir,
            &[
                wad_record(1, FetchStatus::Ok, vec![wad_member("F1.WAD", 100, 500)]),
                wad_record(2, FetchStatus::Ok, vec![wad_member("F2.WAD", 100, 500)]),
            ],
            2,
        )
    }

    /// Write every required input under `dir` from an arbitrary `records`
    /// list (so tests can exercise multiple `dir`s, e.g. for `--root`
    /// filtering) and an independently-chosen `phase1_file_count` (so tests
    /// can exercise `Coverage::phase1_files`'s manifest-sourced, not
    /// record-count-sourced, value). Returns the assembled [`StatsPaths`].
    fn write_fixture_inputs_with_records(
        dir: &std::path::Path,
        records: &[WadRecord],
        phase1_file_count: u64,
    ) -> StatsPaths {
        let mut wads_jsonl = String::new();
        for record in records {
            wads_jsonl.push_str(&serde_json::to_string(record).unwrap());
            wads_jsonl.push('\n');
        }
        std::fs::write(dir.join("idgames-wads.jsonl"), wads_jsonl).unwrap();

        let entries_total = u64::try_from(records.len()).unwrap();
        // B2: agrees with the records' own zip64 tally by construction — a
        // test that wants to exercise a manifest/record disagreement builds
        // its own `ZipsManifest` directly (see
        // `zip64_statement_flags_manifest_disagreement` in `report.rs`)
        // rather than fighting this fixture's default.
        let zip64_entries =
            u64::try_from(records.iter().filter(|r| r.zip64).count()).unwrap_or(u64::MAX);
        std::fs::write(
            dir.join("wads-manifest.json"),
            serde_json::to_string(&crate::schema::ZipsManifest {
                id: "harvest-zips-1".into(),
                started_at: "2026-08-16T00:00:00Z".into(),
                duration_secs: 1,
                tool_version: tool_version(),
                git_rev: None,
                scoped_root: None,
                limit: None,
                entries_total,
                records_written: entries_total,
                ledger_count: 0,
                cache_hits: 0,
                live_entries: entries_total,
                range_requests: entries_total,
                bytes_transferred: 100,
                full_downloads: 0,
                fallback_bytes: 0,
                zip64_entries,
                status_counts: BTreeMap::new(),
                unaccounted_entries: 0,
                aborted: None,
            })
            .unwrap(),
        )
        .unwrap();
        std::fs::write(
            dir.join("harvest-manifest.json"),
            serde_json::to_string(&crate::schema::HarvestManifest {
                id: "harvest-1".into(),
                started_at: "2026-08-16T00:00:00Z".into(),
                duration_secs: 1,
                api_version: 3,
                tool_version: tool_version(),
                git_rev: None,
                bootstrap: "ls-lar-fresh:infania".into(),
                roots: vec!["levels/".into()],
                scoped_root: None,
                limit: None,
                dir_count: 1,
                file_count: phase1_file_count,
                error_count: 0,
                cache_hits: 0,
                live_api_calls: 1,
                max_file_id: Some(entries_total),
            })
            .unwrap(),
        )
        .unwrap();
        std::fs::write(dir.join("harvest-errors.jsonl"), "").unwrap();

        let cache_dir = dir.join("cache");
        std::fs::create_dir_all(&cache_dir).unwrap();
        std::fs::write(cache_dir.join("ls-laR.gz"), ls_lar_gz_fixture()).unwrap();
        std::fs::write(
            cache_dir.join("ls-laR.meta.json"),
            serde_json::to_string(&LsLarMeta {
                mirror: "infania".into(),
                last_modified: Some("Wed, 12 Aug 2026 06:00:00 GMT".into()),
                fetched_at: "2026-08-16T00:00:00Z".into(),
            })
            .unwrap(),
        )
        .unwrap();

        StatsPaths {
            wads_jsonl: dir.join("idgames-wads.jsonl"),
            zips_manifest: dir.join("wads-manifest.json"),
            phase1_manifest: dir.join("harvest-manifest.json"),
            phase1_ledger: dir.join("harvest-errors.jsonl"),
            lslar_gz: cache_dir.join("ls-laR.gz"),
            lslar_meta: cache_dir.join("ls-laR.meta.json"),
            outliers_jsonl: dir.join("outliers-wads.jsonl"),
            outliers_manifest: dir.join("outliers-manifest.json"),
            out_dir: dir.to_path_buf(),
        }
    }

    #[test]
    fn run_with_paths_writes_stats_and_sweep_with_no_wall_clock() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = write_fixture_inputs(tmp.path());
        run_with_paths(&paths, None, None).unwrap();

        let stats_text = std::fs::read_to_string(tmp.path().join("stats.json")).unwrap();
        assert!(stats_text.ends_with('\n'));
        let stats: serde_json::Value = serde_json::from_str(&stats_text).unwrap();
        assert_eq!(stats["schema_version"], 1);
        assert_eq!(stats["provenance"]["phase1_manifest"], "harvest-1");
        assert_eq!(stats["provenance"]["phase2_manifest"], "harvest-zips-1");
        assert_eq!(stats["provenance"]["bootstrap_mirror"], "infania");
        assert!(stats["outliers"].is_null());
        // §9.3/§7: no wall-clock field anywhere in the document.
        for key in ["started_at", "fetched_at", "duration_secs"] {
            assert!(
                !stats_text.contains(key),
                "wall-clock-shaped key {key:?} found in stats.json"
            );
        }

        let sweep_text = std::fs::read_to_string(tmp.path().join("sweep-corpus.jsonl")).unwrap();
        assert_eq!(sweep_text.lines().count(), 2);
    }

    #[test]
    fn run_with_paths_limit_scopes_sweep_and_status_counts_but_not_phase1_files() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = write_fixture_inputs(tmp.path());
        run_with_paths(&paths, None, Some(1)).unwrap();
        let stats: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(tmp.path().join("stats.json")).unwrap())
                .unwrap();
        // C1 fix round 1: `phase1_files` is the phase-1 manifest's
        // `file_count` (2, from `write_fixture_inputs`) — it must NOT drop
        // to 1 just because `--limit 1` scoped the records this run loaded.
        // The scoped record count is visible as Σ status_counts instead.
        assert_eq!(stats["idgames"]["coverage"]["phase1_files"], 2);
        let status_sum: u64 = stats["idgames"]["coverage"]["status_counts"]
            .as_object()
            .unwrap()
            .values()
            .map(|v| v.as_u64().unwrap())
            .sum();
        assert_eq!(status_sum, 1);
        let sweep_text = std::fs::read_to_string(tmp.path().join("sweep-corpus.jsonl")).unwrap();
        assert_eq!(sweep_text.lines().count(), 1);
    }

    #[test]
    fn run_with_paths_root_filters_by_dir_prefix_not_substring() {
        // I3 fix round 1: exercise `--root` for real, across two dirs, and
        // pin the prefix (not substring) semantics: "levels/doom" must
        // match "levels/doom/..." but NOT "levels/doom2/..." — a naive
        // (non-slash-anchored) prefix check would wrongly include the
        // latter.
        let mut in_scope_a = wad_record(1, FetchStatus::Ok, vec![wad_member("F1.WAD", 100, 500)]);
        in_scope_a.dir = "levels/doom/0-9/".into();
        let mut in_scope_b = wad_record(2, FetchStatus::Ok, vec![wad_member("F2.WAD", 100, 500)]);
        in_scope_b.dir = "levels/doom/a-c/".into();
        let mut out_of_scope = wad_record(3, FetchStatus::Ok, vec![wad_member("F3.WAD", 100, 500)]);
        out_of_scope.dir = "levels/doom2/0-9/".into();

        let tmp = tempfile::tempdir().unwrap();
        let paths = write_fixture_inputs_with_records(
            tmp.path(),
            &[in_scope_a, in_scope_b, out_of_scope],
            3,
        );
        run_with_paths(&paths, Some("levels/doom"), None).unwrap();

        let stats: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(tmp.path().join("stats.json")).unwrap())
                .unwrap();
        let status_sum: u64 = stats["idgames"]["coverage"]["status_counts"]
            .as_object()
            .unwrap()
            .values()
            .map(|v| v.as_u64().unwrap())
            .sum();
        assert_eq!(status_sum, 2); // levels/doom2/... excluded
        // phase1_files stays at the manifest's file_count regardless of --root.
        assert_eq!(stats["idgames"]["coverage"]["phase1_files"], 3);

        let sweep_text = std::fs::read_to_string(tmp.path().join("sweep-corpus.jsonl")).unwrap();
        let sweep_ids: Vec<u64> = sweep_text
            .lines()
            .map(|line| {
                serde_json::from_str::<serde_json::Value>(line).unwrap()["id"]
                    .as_u64()
                    .unwrap()
            })
            .collect();
        assert_eq!(sweep_ids, vec![1, 2]);
    }

    #[test]
    fn run_with_paths_missing_wads_jsonl_names_harvest_zips() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = write_fixture_inputs(tmp.path());
        std::fs::remove_file(&paths.wads_jsonl).unwrap();
        let err = run_with_paths(&paths, None, None).unwrap_err().to_string();
        assert!(err.contains("just harvest-zips"), "{err}");
    }

    #[test]
    fn run_with_paths_missing_zips_manifest_names_harvest_zips() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = write_fixture_inputs(tmp.path());
        std::fs::remove_file(&paths.zips_manifest).unwrap();
        let err = run_with_paths(&paths, None, None).unwrap_err().to_string();
        assert!(err.contains("just harvest-zips"), "{err}");
    }

    #[test]
    fn run_with_paths_missing_phase1_manifest_names_harvest_api() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = write_fixture_inputs(tmp.path());
        std::fs::remove_file(&paths.phase1_manifest).unwrap();
        let err = run_with_paths(&paths, None, None).unwrap_err().to_string();
        assert!(err.contains("just harvest-api"), "{err}");
    }

    #[test]
    fn run_with_paths_missing_ledger_names_harvest_api() {
        // I2 fix round 1: harvest-errors.jsonl is written unconditionally by
        // `xtask harvest-api` (empty when there were zero failures), so its
        // absence means a damaged output dir, not "no failures" — must be a
        // hard error, not a silent `unwrap_or_default()`.
        let tmp = tempfile::tempdir().unwrap();
        let paths = write_fixture_inputs(tmp.path());
        std::fs::remove_file(&paths.phase1_ledger).unwrap();
        let err = run_with_paths(&paths, None, None).unwrap_err().to_string();
        assert!(err.contains("just harvest-api"), "{err}");
    }

    #[test]
    fn run_with_paths_missing_lslar_gz_names_harvest_api() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = write_fixture_inputs(tmp.path());
        std::fs::remove_file(&paths.lslar_gz).unwrap();
        let err = run_with_paths(&paths, None, None).unwrap_err().to_string();
        assert!(err.contains("just harvest-api"), "{err}");
    }

    #[test]
    fn run_with_paths_missing_lslar_meta_names_harvest_api() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = write_fixture_inputs(tmp.path());
        std::fs::remove_file(&paths.lslar_meta).unwrap();
        let err = run_with_paths(&paths, None, None).unwrap_err().to_string();
        assert!(err.contains("just harvest-api"), "{err}");
    }

    fn outliers_manifest_fixture() -> OutliersManifest {
        OutliersManifest {
            id: "harvest-outliers-1".into(),
            started_at: "2026-08-16T00:00:00Z".into(),
            duration_secs: 1,
            tool_version: tool_version(),
            git_rev: None,
            limit: None,
            entries_total: 0,
            records_written: 0,
            ledger_count: 0,
            range_requests: 0,
            bytes_transferred: 0,
            status_counts: BTreeMap::new(),
        }
    }

    #[test]
    fn run_with_paths_absent_outliers_pair_yields_none() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = write_fixture_inputs(tmp.path());
        run_with_paths(&paths, None, None).unwrap();
        let stats: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(tmp.path().join("stats.json")).unwrap())
                .unwrap();
        assert!(stats["outliers"].is_null());
    }

    #[test]
    fn run_with_paths_empty_but_present_outliers_pair_yields_some() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = write_fixture_inputs(tmp.path());
        std::fs::write(&paths.outliers_jsonl, "").unwrap();
        std::fs::write(
            &paths.outliers_manifest,
            serde_json::to_string(&outliers_manifest_fixture()).unwrap(),
        )
        .unwrap();
        run_with_paths(&paths, None, None).unwrap();
        let stats: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(tmp.path().join("stats.json")).unwrap())
                .unwrap();
        assert!(!stats["outliers"].is_null());
        assert_eq!(stats["outliers"]["analyzed"], serde_json::json!([]));
    }

    #[test]
    fn run_with_paths_half_present_outliers_pair_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = write_fixture_inputs(tmp.path());
        // jsonl present, manifest absent.
        std::fs::write(&paths.outliers_jsonl, "").unwrap();
        let err = run_with_paths(&paths, None, None).unwrap_err().to_string();
        assert!(err.contains("just harvest-outliers"), "{err}");
        assert!(err.contains("inconsistent outliers state"), "{err}");
    }

    #[test]
    fn run_with_paths_half_present_outliers_pair_errors_other_direction() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = write_fixture_inputs(tmp.path());
        // manifest present, jsonl absent.
        std::fs::write(
            &paths.outliers_manifest,
            serde_json::to_string(&outliers_manifest_fixture()).unwrap(),
        )
        .unwrap();
        let err = run_with_paths(&paths, None, None).unwrap_err().to_string();
        assert!(err.contains("just harvest-outliers"), "{err}");
    }

    // ---- Task 7: recommendations + report wiring + trio determinism/PII ----

    /// [`wad_record`] with every field a Task 7 diversity fixture needs to
    /// override — `dir`/`filename`/`zip_size` (so records can land in different
    /// ls-laR listing entries with deliberate matches/mismatches/misses),
    /// `other_members`, `votes`, `date`, and `zip64`.
    #[allow(
        clippy::too_many_arguments,
        reason = "test-only fixture builder — one call site per record, explicit fields read \
                  far more clearly here than a builder-pattern would for a handful of one-off records"
    )]
    fn wad_record_full(
        id: u64,
        dir: &str,
        filename: &str,
        fetch_status: FetchStatus,
        zip_size: u64,
        wads: Vec<crate::schema::WadMember>,
        other_members: Vec<String>,
        votes: u64,
        date: &str,
        zip64: bool,
    ) -> WadRecord {
        let mut r = wad_record(id, fetch_status, wads);
        r.dir = dir.to_owned();
        r.filename = filename.to_owned();
        r.zip_size = zip_size;
        r.other_members = other_members;
        r.votes = votes;
        r.date = date.to_owned();
        r.zip64 = zip64;
        r
    }

    /// The record list half of [`write_diverse_fixture_inputs`]: multi-wad,
    /// zero-wad, zip64, `full_download`, zero votes, missing date, a
    /// duplicate id, and a wad-named `other_member`. Split out so neither
    /// function trips clippy's line-count lint.
    #[allow(
        clippy::too_many_lines,
        reason = "a flat list of 9 one-off fixture records, each already split across its own \
                  multi-line call — splitting further would hide the diversity this fixture \
                  exists to enumerate, not clarify it"
    )]
    fn diverse_records() -> Vec<WadRecord> {
        vec![
            wad_record_full(
                1,
                "levels/doom/0-9/",
                "multi.zip",
                FetchStatus::Ok,
                500,
                vec![
                    wad_member("A.WAD", 100, 1_000),
                    wad_member("B.WAD", 50, 500),
                ],
                vec![],
                10,
                "2019-04-02",
                false,
            ),
            wad_record_full(
                2,
                "levels/doom/0-9/",
                "zero.zip",
                FetchStatus::Ok,
                200,
                vec![],
                vec![],
                0,
                "2020-01-01",
                false,
            ),
            wad_record_full(
                3,
                "levels/doom2/Ports/megawads/",
                "zip64.zip",
                FetchStatus::FullDownload,
                700,
                vec![wad_member("C.WAD", 300, 3_000)],
                vec![],
                5,
                "2021-06-01",
                true,
            ),
            wad_record_full(
                4,
                "levels/heretic/0-9/",
                "nodate.zip",
                FetchStatus::Ok,
                800,
                vec![wad_member("D.WAD", 10, 100)],
                vec![],
                1,
                "",
                false,
            ),
            wad_record_full(
                5,
                "levels/hexen/0-9/",
                "sneaky.zip",
                FetchStatus::Ok,
                900,
                vec![wad_member("E.WAD", 10, 100)],
                vec!["SNEAKY.WAD".into()],
                2,
                "2018-03-03",
                false,
            ),
            wad_record_full(
                6,
                "levels/doom/0-9/",
                "mismatch.zip",
                FetchStatus::Ok,
                550, // API says 550; the ls-laR listing below says 600.
                vec![wad_member("F.WAD", 20, 200)],
                vec![],
                0,
                "2017-07-07",
                false,
            ),
            wad_record_full(
                7,
                "levels/doom/0-9/",
                "missing.zip",
                FetchStatus::Ok,
                12_345, // absent from the listing below — a join miss.
                vec![wad_member("G.WAD", 5, 50)],
                vec![],
                0,
                "2016-01-01",
                false,
            ),
            wad_record_full(
                42,
                "levels/doom/a-c/",
                "dup-a.zip",
                FetchStatus::Ok,
                111,
                vec![wad_member("H.WAD", 1, 10)],
                vec![],
                0,
                "2015-01-01",
                false,
            ),
            wad_record_full(
                42, // duplicate id — same value as the record above.
                "levels/doom/a-c/",
                "dup-b.zip",
                FetchStatus::Ok,
                222,
                vec![wad_member("I.WAD", 2, 20)],
                vec![],
                0,
                "2015-01-02",
                false,
            ),
        ]
    }

    /// A maximally diverse record set for the trio's byte-identity and PII
    /// tests (§9.3, ADR-0030 §3): see [`diverse_records`] for the record
    /// shapes (multi-wad, zero-wad, zip64, `full_download`, zero votes,
    /// missing date, a duplicate id, a wad-named `other_member`); this
    /// function additionally builds a matching ls-laR listing with a
    /// deliberate listing mismatch and a listing miss, plus the §6.4
    /// outliers pair (one analyzed, one skipped) and a phase-1 ledger entry
    /// whose free-text-shaped `path`/`detail` fields plant
    /// `"EVIL_AUTHOR"`/`"EVIL_TITLE"`. `build_stats` never reads a ledger
    /// entry past its `kind` (see [`ledger_kind_counts`]), so these two
    /// strings must never surface in any of the three trio outputs —
    /// `trio_has_no_free_text_keys` is the witness.
    fn write_diverse_fixture_inputs(dir: &std::path::Path) -> StatsPaths {
        let records = diverse_records();
        let record_count = u64::try_from(records.len()).unwrap();
        let paths = write_fixture_inputs_with_records(dir, &records, record_count);

        // Overwrite the base fixture's ls-laR cache with a listing matching
        // this diverse set: `multi.zip`/`zip64.zip`/`nodate.zip`/`sneaky.zip`
        // match their record's `zip_size` exactly; `mismatch.zip` disagrees
        // (550 vs 600); `missing.zip` and the two `levels/doom/a-c/` dup
        // records are absent entirely (listing misses).
        let text = "\
.:
total 0

./levels/doom/0-9:
total 2
-rw-r--r--  1 ftp ftp  500 Jan  1  2020 multi.zip
-rw-r--r--  1 ftp ftp  600 Jan  1  2020 mismatch.zip

./levels/doom2/Ports/megawads:
total 1
-rw-r--r--  1 ftp ftp  700 Jan  1  2020 zip64.zip

./levels/heretic/0-9:
total 1
-rw-r--r--  1 ftp ftp  800 Jan  1  2020 nodate.zip

./levels/hexen/0-9:
total 1
-rw-r--r--  1 ftp ftp  900 Jan  1  2020 sneaky.zip
";
        let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        std::io::Write::write_all(&mut enc, text.as_bytes()).unwrap();
        std::fs::write(&paths.lslar_gz, enc.finish().unwrap()).unwrap();

        write_diverse_ledger_and_outliers(&paths);
        paths
    }

    /// The ledger/outliers half of [`write_diverse_fixture_inputs`]: plants
    /// the PII-shaped ledger strings and writes the §6.4 outliers pair.
    /// Split out so neither function trips clippy's line-count lint.
    fn write_diverse_ledger_and_outliers(paths: &StatsPaths) {
        // Phase-1 ledger: a real `HttpError` entry (so `ledger_kinds` is
        // non-empty) whose `path`/`detail` plant PII-shaped free text that
        // `build_stats` never reads past `.kind` — see this fn's doc comment.
        let ledger = [LedgerEntry {
            path: "levels/EVIL_TITLE-not-a-real-archive-path/".into(),
            action: "getcontents".into(),
            kind: LedgerKind::HttpError,
            detail: "uploader EVIL_AUTHOR triggered HTTP 500".into(),
            attempts: 2,
        }];
        let mut ledger_text = String::new();
        for entry in &ledger {
            ledger_text.push_str(&serde_json::to_string(entry).unwrap());
            ledger_text.push('\n');
        }
        std::fs::write(&paths.phase1_ledger, ledger_text).unwrap();

        // §6.4 outliers pair: one analyzed, one skipped (no range support).
        let analyzed = OutlierRecord {
            slug: "curated-megawad".into(),
            url: "https://example.com/curated-megawad.zip".into(),
            zip_size: 2_000_000_000,
            zip64: false,
            member_count: 1,
            wads: vec![wad_member("J.WAD", 900_000, 9_000_000)],
            other_members: vec![],
            fetch_status: FetchStatus::Ok,
        };
        let skipped = OutlierRecord {
            slug: "refused-host".into(),
            url: "https://example.com/refused-host.zip".into(),
            zip_size: 0,
            zip64: false,
            member_count: 0,
            wads: vec![],
            other_members: vec![],
            fetch_status: FetchStatus::NoRangeSupport,
        };
        let mut outliers_text = String::new();
        for rec in [&analyzed, &skipped] {
            outliers_text.push_str(&serde_json::to_string(rec).unwrap());
            outliers_text.push('\n');
        }
        std::fs::write(&paths.outliers_jsonl, outliers_text).unwrap();
        std::fs::write(
            &paths.outliers_manifest,
            serde_json::to_string(&OutliersManifest {
                id: "harvest-outliers-1".into(),
                started_at: "2026-08-16T00:00:00Z".into(),
                duration_secs: 1,
                tool_version: tool_version(),
                git_rev: None,
                limit: None,
                entries_total: 2,
                records_written: 2,
                ledger_count: 0,
                range_requests: 2,
                bytes_transferred: 1_000,
                status_counts: BTreeMap::from([
                    ("ok".to_owned(), 1),
                    ("no_range_support".to_owned(), 1),
                ]),
            })
            .unwrap(),
        )
        .unwrap();
    }

    /// Reads the three §9.3 trio outputs (in a fixed, named order) as raw
    /// bytes — the byte-identity witness compares this across two runs.
    fn read_trio(out_dir: &std::path::Path) -> Vec<(String, Vec<u8>)> {
        ["stats.json", "stats-report.md", "sweep-corpus.jsonl"]
            .into_iter()
            .map(|name| {
                let bytes = std::fs::read(out_dir.join(name))
                    .unwrap_or_else(|e| panic!("reading {name}: {e}"));
                (name.to_owned(), bytes)
            })
            .collect()
    }

    /// Recursively asserts no JSON object key anywhere in `v` is (or
    /// contains, case-insensitively) an ADR-0030 §3 forbidden free-text
    /// field name. Generalizes
    /// [`crate::api::model::tests::assert_no_email_keys`]'s single-field
    /// shape to the full forbidden set the trio must never carry.
    fn assert_no_pii_keys(v: &serde_json::Value) {
        const FORBIDDEN: [&str; 5] = ["title", "author", "description", "email", "textfile"];
        match v {
            serde_json::Value::Object(map) => {
                for (k, val) in map {
                    let lower = k.to_ascii_lowercase();
                    assert!(
                        !FORBIDDEN.iter().any(|f| lower.contains(f)),
                        "PII-shaped key {k:?} present"
                    );
                    assert_no_pii_keys(val);
                }
            }
            serde_json::Value::Array(items) => items.iter().for_each(assert_no_pii_keys),
            _ => {}
        }
    }

    #[test]
    fn trio_is_byte_identical_across_reruns() {
        let dir = tempfile::tempdir().unwrap();
        let paths = write_diverse_fixture_inputs(dir.path());
        run_with_paths(&paths, None, None).unwrap();
        let first = read_trio(&paths.out_dir);
        run_with_paths(&paths, None, None).unwrap();
        let second = read_trio(&paths.out_dir);
        assert_eq!(first, second);
        // Sanity: every output is non-trivial (a bug that made every run
        // write empty files would otherwise pass a byte-identity check
        // vacuously).
        for (name, bytes) in &first {
            assert!(!bytes.is_empty(), "{name} is empty");
        }
    }

    #[test]
    fn trio_has_no_free_text_keys() {
        let dir = tempfile::tempdir().unwrap();
        let paths = write_diverse_fixture_inputs(dir.path());
        run_with_paths(&paths, None, None).unwrap();

        let stats_text = std::fs::read_to_string(paths.out_dir.join("stats.json")).unwrap();
        assert_no_pii_keys(&serde_json::from_str(&stats_text).unwrap());

        let sweep_text = std::fs::read_to_string(paths.out_dir.join("sweep-corpus.jsonl")).unwrap();
        for line in sweep_text.lines() {
            assert_no_pii_keys(&serde_json::from_str(line).unwrap());
        }

        let report_text = std::fs::read_to_string(paths.out_dir.join("stats-report.md")).unwrap();
        for planted in ["EVIL_AUTHOR", "EVIL_TITLE"] {
            assert!(!stats_text.contains(planted), "stats.json leaked {planted}");
            assert!(
                !sweep_text.contains(planted),
                "sweep-corpus.jsonl leaked {planted}"
            );
            assert!(
                !report_text.contains(planted),
                "stats-report.md leaked {planted}"
            );
        }
    }

    #[test]
    fn missing_outliers_is_stated_not_silent() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = write_fixture_inputs(tmp.path()); // no outliers files
        run_with_paths(&paths, None, None).unwrap();

        let report = std::fs::read_to_string(paths.out_dir.join("stats-report.md")).unwrap();
        assert!(report.contains("No outliers analyzed"), "{report}");

        let stats: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(paths.out_dir.join("stats.json")).unwrap(),
        )
        .unwrap();
        assert!(stats["outliers"].is_null());
    }

    #[test]
    fn stats_json_wire_keys_pin_percentile_serde_renames() {
        // Task 6 ledgered minor: M2 — `p99.5`/`p99.9` must round-trip on
        // the actual JSON wire, not just through the Rust struct.
        let tmp = tempfile::tempdir().unwrap();
        let paths = write_fixture_inputs(tmp.path());
        run_with_paths(&paths, None, None).unwrap();
        let text = std::fs::read_to_string(paths.out_dir.join("stats.json")).unwrap();
        assert!(text.contains("\"p99.5\""), "p99.5 wire key missing");
        assert!(text.contains("\"p99.9\""), "p99.9 wire key missing");
    }

    #[test]
    fn run_with_paths_writes_a_nonempty_report_stating_every_recommendation() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = write_fixture_inputs(tmp.path());
        run_with_paths(&paths, None, None).unwrap();
        let report = std::fs::read_to_string(paths.out_dir.join("stats-report.md")).unwrap();
        assert!(report.starts_with("# idgames corpus statistics (phase 3)"));
        for key in [
            "wire_cap_zip",
            "wire_cap_wad",
            "decoded_cap",
            "max_member_count",
            "max_entry_uncompressed_bytes",
            "max_member_compression_ratio",
            "compression_method_allowlist",
            "zip64_statement",
        ] {
            assert!(
                report.contains(key),
                "recommendation {key} missing from report"
            );
        }
        let stats: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(paths.out_dir.join("stats.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(stats["recommendations"].as_array().unwrap().len(), 8);
    }
}
