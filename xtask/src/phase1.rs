//! Phase-1 orchestrator: bootstrap, probe, enrichment, outputs (DESIGN.md §4).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::Context as _;
use chrono::Utc;

use crate::api::client::{ApiCallError, ApiClient};
use crate::api::model::{FileRecord, normalize_dir};
use crate::api::traverse::{self, TraverseOutcome};
use crate::cache::ApiCache;
use crate::lslar::ArchiveTree;
use crate::mirror::{self, BootstrapSource};
use crate::schema::{self, HarvestManifest, LedgerEntry, LedgerKind, manifest_id, read_manifest};
use crate::scope;

/// Run phase 1 (`xtask harvest-api`). `root`/`limit` are the §4.6 dev
/// flags; when either is set the run is "scoped" and writes to
/// `data/dev/`.
///
/// # Errors
/// Environmental failures only (directories, output writes, client/HTTP
/// setup). API/mirror failures are ledgered, not returned.
pub fn run(root: Option<&str>, limit: Option<usize>) -> anyhow::Result<()> {
    let runtime = tokio::runtime::Runtime::new().context("building tokio runtime")?;
    let limit = limit.map(|l| u64::try_from(l).unwrap_or(u64::MAX));
    runtime.block_on(run_async(root, limit))
}

/// Compile-time absolute path to `xtask/data` — the tool works from any cwd.
pub(crate) fn data_root() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/data"))
}

/// Output directory for this run's mode: `data/dev/` when scoped
/// (`--root`/`--limit`), `data/` for a full run — so a scoped run never
/// clobbers a full harvest's outputs.
pub(crate) fn output_dir(scoped: bool) -> PathBuf {
    if scoped {
        data_root().join("dev")
    } else {
        data_root()
    }
}

/// Directories whose cache entries must be dropped so the traversal
/// refetches them live: in-scope tree dirs whose `.zip` `(name, size)`
/// set differs from the prior run's records. Names are compared ASCII
/// case-insensitively on both sides, matching the `.zip` detection, so an
/// API-vs-mirror case difference never spuriously invalidates.
/// `latestfiles` records carry no `dir` (observed live — see DESIGN §4.5
/// correction), so mirror-side tree drift is the
/// addition/deletion/replacement signal.
pub(crate) fn dirs_to_invalidate_from_tree(
    tree: &ArchiveTree,
    prior: &[FileRecord],
) -> BTreeSet<String> {
    let mut baseline: std::collections::BTreeMap<&str, BTreeSet<(String, u64)>> =
        std::collections::BTreeMap::new();
    for rec in prior {
        // Mirror the tree side's zip filter: a rare non-`.zip` archive
        // entry (DESIGN §5.5) must not keep the sets permanently unequal
        // and re-invalidate its directory on every run.
        if !rec.filename.to_ascii_lowercase().ends_with(".zip") {
            continue;
        }
        baseline
            .entry(rec.dir.as_str())
            .or_default()
            .insert((rec.filename.to_ascii_lowercase(), rec.size));
    }
    let mut changed = BTreeSet::new();
    for (dir, files) in &tree.dirs {
        if scope::decide(dir) != scope::ScopeDecision::Include {
            continue;
        }
        let tree_zips: BTreeSet<(String, u64)> = files
            .iter()
            .filter(|f| f.name.to_ascii_lowercase().ends_with(".zip"))
            .map(|f| (f.name.to_ascii_lowercase(), f.size))
            .collect();
        // Compare against the borrowed baseline set — no per-dir clone.
        let drifted = match baseline.get(dir.as_str()) {
            Some(prior_zips) => tree_zips != *prior_zips,
            None => !tree_zips.is_empty(),
        };
        if drifted {
            changed.insert(dir.clone());
        }
    }
    changed
}

async fn run_async(root: Option<&str>, limit: Option<u64>) -> anyhow::Result<()> {
    let started_at = Utc::now();
    let scoped = root.is_some() || limit.is_some();
    let out_dir = output_dir(scoped);
    let cache_dir = data_root().join("cache");
    std::fs::create_dir_all(&out_dir).with_context(|| format!("creating {}", out_dir.display()))?;
    let cache = ApiCache::new(cache_dir.join("api"), chrono::Duration::days(7))?;
    let mut client = ApiClient::new(cache)?;
    let mut extra_ledger: Vec<LedgerEntry> = Vec::new();

    // §4.5 cheap rerun (as corrected): a latestfiles probe on full reruns
    // only; additions/deletions/replacements are picked up via the
    // bootstrap tree diff below — there is no walk-back.
    let manifest_path = out_dir.join("harvest-manifest.json");
    // `None` = no baseline file (first run) — skip invalidation entirely.
    // `Some(records)` = a baseline exists; if damaged lines left it empty,
    // the diff degrades to broad invalidation of every populated in-scope
    // dir, exactly as `read_files_jsonl` documents.
    let prior_records: Option<Vec<FileRecord>> = if scoped {
        None
    } else {
        schema::read_files_jsonl(&out_dir.join("idgames-files.jsonl"))
    };
    let probe_max = if scoped {
        None
    } else {
        probe_rerun(&mut client, &manifest_path, &mut extra_ledger).await
    };

    // §5.0 bootstrap, then enrichment — or §4.2 BFS fallback.
    let http = mirror::build_http()?;
    let (tree, source) = mirror::fetch_ls_lar(&http, &cache_dir).await;

    // Fresh mirror content + a baseline → invalidate exactly the dirs
    // whose zip sets moved, so the traversal refetches them live. A 304 or
    // stale-cache tree cannot carry changes; skip the diff there.
    if !scoped
        && matches!(source, BootstrapSource::Fresh { .. })
        && let Some(prior) = &prior_records
        && let Some(tree) = &tree
    {
        invalidate_stale_dirs(client.cache(), tree, prior);
    }

    let (mut outcome, roots) =
        traverse_or_fallback(&mut client, tree.as_ref(), &source, root, limit, &cache_dir).await;
    outcome.ledger.append(&mut extra_ledger);
    warn_untriaged_roots(&outcome.triage, tree.as_ref());

    // Outputs (§4.7). Manifest is written LAST so a crash mid-write never
    // leaves a manifest describing outputs that don't exist.
    let ledger_path = out_dir.join("harvest-errors.jsonl");
    let max_record_id = outcome.records.iter().map(|r| r.id).max();
    let file_count =
        schema::write_files_jsonl(&out_dir.join("idgames-files.jsonl"), outcome.records)?;
    let error_count = schema::write_ledger(&ledger_path, outcome.ledger)?;
    let stats = client.stats();
    let duration = (Utc::now() - started_at).num_seconds().max(0);
    let manifest = HarvestManifest {
        id: manifest_id(&started_at),
        started_at: started_at.to_rfc3339(),
        duration_secs: u64::try_from(duration).unwrap_or(0),
        // 0 = unknown: no response envelope (live or cached) was parsed
        // this run. Defaulting to the spike-verified 3 would fake
        // certainty on a run where the API was never actually observed.
        api_version: client.observed_api_version().unwrap_or(0),
        tool_version: schema::tool_version(),
        git_rev: schema::git_rev(),
        bootstrap: source.label(),
        roots,
        scoped_root: root.map(str::to_owned),
        limit,
        dir_count: outcome.dirs_processed,
        file_count,
        error_count,
        cache_hits: stats.cache_hits,
        live_api_calls: stats.live_calls,
        max_file_id: max_record_id.max(probe_max),
    };
    schema::write_manifest(&manifest_path, &manifest)?;

    tracing::info!(
        files = file_count,
        dirs = manifest.dir_count,
        errors = error_count,
        cache_hits = stats.cache_hits,
        live_calls = stats.live_calls,
        bootstrap = manifest.bootstrap.as_str(),
        "harvest-api complete"
    );
    if error_count > 0 {
        tracing::warn!(
            ledger = %ledger_path.display(),
            "run finished with ledgered failures"
        );
    }

    // Environmental-failure escape hatch (§9.3): a run that has neither a
    // bootstrap tree nor a single BFS-discovered record collected nothing
    // at all — every root was unreachable. `justfile`'s `harvest` recipe
    // chains phases purely on exit code, so this must not exit 0 (an empty
    // manifest would otherwise look like a legitimate zero-file harvest to
    // phase 2). A partial BFS harvest or an empty-but-bootstrapped tree are
    // both real outcomes, not failures — see `is_total_failure`.
    if is_total_failure(&source, file_count) {
        anyhow::bail!(
            "no bootstrap and no records — every root unreachable; see {}",
            ledger_path.display()
        );
    }
    Ok(())
}

/// True only when the run collected literally nothing: no ls-laR bootstrap
/// (`BootstrapSource::Unavailable`) AND zero file records. A partial BFS
/// harvest (`Unavailable` bootstrap but `file_count > 0`) is a legitimate
/// `Ok` — BFS did its job under a real degradation. An empty-but-bootstrapped
/// tree (`Fresh`/`NotModified`/`StaleCache` with `file_count == 0`) is a data
/// fact about the corpus, not an environmental failure.
pub(crate) fn is_total_failure(source: &BootstrapSource, file_count: u64) -> bool {
    matches!(source, BootstrapSource::Unavailable) && file_count == 0
}

/// One `latestfiles(1)` probe (§4.5, as corrected) — logs movement against
/// the prior manifest's `max_file_id` and returns the probed max id. A
/// missing prior manifest means there is nothing to compare against, so no
/// call is made at all. A probe failure is pushed to `extra_ledger` and the
/// run continues (§9.3: this is the warm rerun's one API request).
async fn probe_rerun(
    client: &mut ApiClient,
    manifest_path: &Path,
    extra_ledger: &mut Vec<LedgerEntry>,
) -> Option<u64> {
    let prior_manifest = read_manifest(manifest_path)?;
    match client.latestfiles(1).await {
        Ok(latest) => {
            let probe_max = latest.first().map(|r| r.id);
            match (probe_max, prior_manifest.max_file_id) {
                (Some(p), Some(k)) if p > k => {
                    tracing::info!(probe_max = p, known_max = k, "additions since last harvest");
                }
                (Some(p), Some(k)) => {
                    tracing::info!(
                        probe_max = p,
                        known_max = k,
                        "no additions since last harvest"
                    );
                }
                (Some(p), None) => {
                    // An older/damaged manifest without a max id: there is
                    // no baseline, so claiming "no additions" would mislead.
                    tracing::info!(probe_max = p, "probe ran without a prior max-id baseline");
                }
                _ => {}
            }
            probe_max
        }
        Err(e) => {
            extra_ledger.push(latestfiles_ledger(&e));
            None
        }
    }
}

/// Drop `getcontents` cache entries for tree dirs whose `.zip` set moved
/// since the prior baseline, so the traversal refetches them live (§4.5
/// correction: a mirror-side diff replaces the impossible `latestfiles`
/// walk-back).
fn invalidate_stale_dirs(cache: &ApiCache, tree: &ArchiveTree, prior: &[FileRecord]) {
    let stale = dirs_to_invalidate_from_tree(tree, prior);
    for dir in &stale {
        if let Err(e) = cache.invalidate("getcontents", dir) {
            tracing::warn!(dir, error = %e, "cache invalidation failed");
        }
    }
    if stale.is_empty() {
        tracing::info!("fresh ls-laR shows no zip-set drift (mirror may lag the API)");
    } else {
        tracing::info!(
            dirs = stale.len(),
            "invalidated directories with zip-set drift"
        );
    }
}

/// Enrich from the bootstrap tree when available (§4.2 primary mode);
/// otherwise fall back to checkpointed BFS discovery from the include
/// roots. Returns the outcome plus the traversal roots for the manifest.
async fn traverse_or_fallback(
    client: &mut ApiClient,
    tree: Option<&ArchiveTree>,
    source: &BootstrapSource,
    root: Option<&str>,
    limit: Option<u64>,
    cache_dir: &Path,
) -> (TraverseOutcome, Vec<String>) {
    let Some(tree) = tree else {
        debug_assert_eq!(*source, BootstrapSource::Unavailable);
        tracing::warn!("no ls-laR bootstrap available — falling back to BFS discovery");
        let roots = scoped_or_bfs_roots(root);
        let ckpt = cache_dir.join("bfs-frontier.json");
        // Dev `--root` runs follow the whole subtree, matching tree-mode
        // `--root` semantics (dev inspects anything, §4.6).
        let scope_mode = if root.is_some() {
            traverse::BfsScope::Subtree
        } else {
            traverse::BfsScope::IncludeTable
        };
        let outcome = traverse::bfs(client, &roots, &ckpt, limit, scope_mode).await;
        return (outcome, roots);
    };
    let (worklist, triage) = traverse::worklist_from_tree(tree, root);
    let roots = scoped_or_bfs_roots(root);
    tracing::info!(dirs = worklist.len(), "enriching from ls-laR tree");
    let mut outcome = traverse::enrich(client, &worklist, Some(tree), limit).await;
    outcome.triage = triage;
    (outcome, roots)
}

/// Full-run roots (§4.2 include set) or the single dev-scoped `--root`.
fn scoped_or_bfs_roots(root: Option<&str>) -> Vec<String> {
    match root {
        Some(r) => vec![normalize_dir(r)],
        None => scope::BFS_ROOTS.iter().map(|r| (*r).to_owned()).collect(),
    }
}

/// Surface every untriaged top-level segment loudly (§4.2: inspect and
/// record the include/skip call before the first full run).
fn warn_untriaged_roots(triage: &[String], tree: Option<&ArchiveTree>) {
    for seg in triage {
        let zips = tree.map_or(0, |t| t.zip_count(seg));
        tracing::warn!(
            root = seg.as_str(),
            zips,
            "untriaged root skipped — record the include/skip call in xtask/DESIGN.md §4.2"
        );
    }
}

fn latestfiles_ledger(e: &ApiCallError) -> LedgerEntry {
    LedgerEntry {
        path: String::new(),
        action: "latestfiles".into(),
        kind: LedgerKind::HttpError,
        detail: e.to_string(),
        attempts: match e {
            ApiCallError::Http { attempts, .. } => *attempts,
            ApiCallError::Api { .. } | ApiCallError::Shape(_) => 1,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::model::FileRecord;

    fn rec_named(id: u64, dir: &str, filename: &str, size: u64) -> FileRecord {
        serde_json::from_value(serde_json::json!({
            "id": id, "dir": dir, "filename": filename, "size": size, "age": 0
        }))
        .unwrap()
    }

    fn tree_with(entries: &[(&str, &[(&str, u64)])]) -> crate::lslar::ArchiveTree {
        let mut tree = crate::lslar::ArchiveTree::default();
        for (dir, files) in entries {
            tree.dirs.insert(
                (*dir).to_owned(),
                files
                    .iter()
                    .map(|(n, s)| crate::lslar::TreeFile {
                        name: (*n).to_owned(),
                        size: *s,
                    })
                    .collect(),
            );
        }
        tree
    }

    #[test]
    fn tree_diff_flags_added_removed_and_resized_zips() {
        let tree = tree_with(&[
            (
                "levels/doom/0-9/",
                &[("a.zip", 10), ("b.zip", 20), ("a.txt", 1)],
            ),
            ("levels/doom/a-c/", &[("c.zip", 30)]),
            ("levels/doom/d-f/", &[("d.zip", 40)]),
            ("levels/doom/g-i/", &[("e.zip", 55)]),
        ]);
        let prior = vec![
            rec_named(1, "levels/doom/0-9/", "a.zip", 10), // b.zip added → dir flagged
            rec_named(2, "levels/doom/a-c/", "c.zip", 30), // unchanged → not flagged
            rec_named(3, "levels/doom/d-f/", "d.zip", 40),
            rec_named(4, "levels/doom/d-f/", "gone.zip", 5), // removed → flagged
            rec_named(5, "levels/doom/g-i/", "e.zip", 50),   // size moved → flagged
        ];
        let dirs = dirs_to_invalidate_from_tree(&tree, &prior);
        assert_eq!(
            dirs.into_iter().collect::<Vec<_>>(),
            vec!["levels/doom/0-9/", "levels/doom/d-f/", "levels/doom/g-i/"]
        );
    }

    #[test]
    fn tree_diff_ignores_txt_case_and_out_of_scope() {
        let tree = tree_with(&[
            // .TXT changes and case-different zip names must not flag.
            ("levels/doom/0-9/", &[("A.ZIP", 10), ("new.txt", 2)]),
            // Out-of-scope root with zips: excluded even though no baseline.
            ("music/", &[("tune.zip", 99)]),
            ("misc/", &[("odd.zip", 7)]),
        ]);
        let prior = vec![rec_named(1, "levels/doom/0-9/", "A.ZIP", 10)];
        let dirs = dirs_to_invalidate_from_tree(&tree, &prior);
        assert!(dirs.is_empty(), "{dirs:?}");
    }

    #[test]
    fn tree_diff_name_compare_is_case_insensitive() {
        // A mirror/API case divergence on the same physical file must not
        // spuriously invalidate the directory.
        let tree = tree_with(&[("levels/doom/0-9/", &[("A.ZIP", 10)])]);
        let prior = vec![rec_named(1, "levels/doom/0-9/", "a.zip", 10)];
        assert!(dirs_to_invalidate_from_tree(&tree, &prior).is_empty());
        // A size change still flags through the case normalization.
        let prior = vec![rec_named(1, "levels/doom/0-9/", "a.zip", 11)];
        assert_eq!(
            dirs_to_invalidate_from_tree(&tree, &prior)
                .into_iter()
                .collect::<Vec<_>>(),
            vec!["levels/doom/0-9/"]
        );
    }

    #[test]
    fn tree_diff_baseline_ignores_non_zip_records() {
        // A rare non-.zip archive entry (DESIGN §5.5) in the baseline must
        // not keep the sets permanently unequal — that would re-invalidate
        // its directory on every run and break the §9.3 warm-rerun budget.
        let tree = tree_with(&[("levels/doom/0-9/", &[("a.zip", 10), ("a.txt", 2)])]);
        let prior = vec![
            rec_named(1, "levels/doom/0-9/", "a.zip", 10),
            rec_named(2, "levels/doom/0-9/", "oldstuff.exe", 77),
        ];
        assert!(dirs_to_invalidate_from_tree(&tree, &prior).is_empty());
    }

    #[test]
    fn tree_diff_with_empty_baseline_flags_populated_scoped_dirs() {
        let tree = tree_with(&[("levels/doom/0-9/", &[("a.zip", 10)])]);
        let dirs = dirs_to_invalidate_from_tree(&tree, &[]);
        assert_eq!(
            dirs.into_iter().collect::<Vec<_>>(),
            vec!["levels/doom/0-9/"]
        );
    }

    #[test]
    fn data_root_is_inside_the_xtask_workspace() {
        let root = data_root();
        assert!(root.ends_with("data"));
        assert!(root.parent().unwrap().join("Cargo.toml").exists());
    }

    #[test]
    fn scoped_runs_use_the_dev_output_dir() {
        assert!(output_dir(true).ends_with("data/dev"));
        assert!(output_dir(false).ends_with("data"));
    }

    #[test]
    fn total_failure_is_only_unavailable_bootstrap_with_zero_records() {
        assert!(is_total_failure(&BootstrapSource::Unavailable, 0));
        // Partial BFS harvest — BFS did its job under a real degradation.
        assert!(!is_total_failure(&BootstrapSource::Unavailable, 3));
        // Empty-but-bootstrapped tree — a data fact, not a failure.
        assert!(!is_total_failure(
            &BootstrapSource::Fresh {
                mirror: "infania".into()
            },
            0
        ));
        assert!(!is_total_failure(&BootstrapSource::StaleCache, 0));
    }
}
