//! Enrichment walk over the §5.0 tree; BFS fallback (DESIGN.md §4.2).
//!
//! Primary mode: the worklist comes from the ls-laR bootstrap tree and the
//! API is metadata-only enrichment — one `getcontents` per in-scope
//! directory. BFS discovery via `getcontents` is the explicit fallback
//! when no bootstrap is obtainable; its frontier is checkpointed (§4.6)
//! and visited dirs are replayed through the response cache on resume, so
//! a resumed run re-derives nothing over the network.

use std::collections::{BTreeSet, VecDeque};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::api::client::{ApiCallError, ApiClient, FetchOutcome};
use crate::api::model::{FileRecord, normalize_dir};
use crate::cache::atomic_write;
use crate::lslar::ArchiveTree;
use crate::schema::{LedgerEntry, LedgerKind};
use crate::scope::{ScopeDecision, decide};

/// Anything that can answer a `getcontents` — the real client, or a test
/// fake. `async fn` in a public trait normally trips the `async_fn_in_trait`
/// lint (the returned future carries no auto-trait bounds, e.g. `Send`);
/// this trait is internal-only, always driven as `&mut impl ListingSource`
/// on a single-threaded call chain, so the missing bound is never a
/// problem.
#[allow(async_fn_in_trait)]
pub trait ListingSource {
    /// Fetch one directory's listing.
    ///
    /// # Errors
    /// See [`ApiClient::getcontents`].
    async fn getcontents(&mut self, dir: &str) -> Result<FetchOutcome, ApiCallError>;
}

impl ListingSource for ApiClient {
    async fn getcontents(&mut self, dir: &str) -> Result<FetchOutcome, ApiCallError> {
        ApiClient::getcontents(self, dir).await
    }
}

/// Result of one traversal (either mode).
#[derive(Debug, Default)]
pub struct TraverseOutcome {
    /// Every file record collected.
    pub records: Vec<FileRecord>,
    /// Directories that produced a listing or a ledger entry.
    pub dirs_processed: u64,
    /// Directories whose scrubbed body hash moved on a live refetch
    /// (phase-2 invalidation signal, §4.5).
    pub changed_dirs: u64,
    /// Failures and findings (record, don't skip).
    pub ledger: Vec<LedgerEntry>,
    /// Top-level Triage segments seen in the tree (§4.2: surface loudly).
    pub triage: Vec<String>,
}

/// Derive the enrichment worklist from the bootstrap tree.
///
/// Full mode (`root: None`): in-scope dirs per §4.2, plus the deduped
/// top-level segments of every `Triage` dir. Dev mode (`root: Some`): the
/// normalized root's subtree, scope tables ignored (dev inspects anything).
pub fn worklist_from_tree(tree: &ArchiveTree, root: Option<&str>) -> (Vec<String>, Vec<String>) {
    if let Some(r) = root {
        let prefix = normalize_dir(r);
        let work = tree
            .dirs
            .keys()
            .filter(|d| d.starts_with(&prefix))
            .cloned()
            .collect();
        return (work, Vec::new());
    }
    let mut work = Vec::new();
    let mut triage = BTreeSet::new();
    for dir in tree.dirs.keys() {
        match decide(dir) {
            ScopeDecision::Include => work.push(dir.clone()),
            ScopeDecision::Triage => {
                if let Some(top) = dir.split('/').next() {
                    triage.insert(format!("{top}/"));
                }
            }
            ScopeDecision::Skip => {}
        }
    }
    (work, triage.into_iter().collect())
}

/// Enrich each worklist directory with one `getcontents` (§4.2 primary
/// mode). `tree` enables the §5.0 API-size-vs-listing cross-check.
/// Never returns `Err` and never panics — every failure path ends in a
/// [`LedgerEntry`] (record, don't skip).
pub async fn enrich(
    source: &mut impl ListingSource,
    worklist: &[String],
    tree: Option<&ArchiveTree>,
    limit: Option<u64>,
) -> TraverseOutcome {
    let mut out = TraverseOutcome::default();
    let bar = progress_bar(worklist.len());
    for dir in worklist {
        if at_limit(&out, limit) {
            break;
        }
        process_dir(source, dir, tree, &mut out).await;
        bar.inc(1);
    }
    bar.finish_and_clear();
    truncate_to_limit(&mut out, limit);
    out
}

/// How [`bfs`] decides whether a discovered subdirectory joins the
/// frontier. Tree mode's `--root` handling deliberately ignores the §4.2
/// scope tables ("dev inspects anything"); [`BfsScope::Subtree`] keeps the
/// BFS fallback consistent with that, instead of silently stopping one
/// level into a Skip/Triage dev root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BfsScope {
    /// Full harvest: only §4.2 `Include` directories are followed.
    IncludeTable,
    /// Dev-scoped (`--root`) run: anything under the call's roots is
    /// followed, matching tree-mode `--root` subtree semantics.
    Subtree,
}

/// Checkpointed BFS discovery via `getcontents` (§4.2 fallback mode), used
/// when no ls-laR bootstrap is obtainable. `scope_mode` governs which
/// discovered subdirectories are followed. The frontier (`pending` +
/// `visited`) is written atomically to `checkpoint` after every processed
/// directory and deleted on a completed run. On resume, `visited` dirs are
/// replayed through `source` — in production that means the disk cache
/// answers them, so a resumed run re-derives nothing over the network; the
/// fake source in tests just answers identically to the first pass. The
/// checkpoint file is shared across run modes (dev-scoped vs. full), so a
/// checkpoint written for a different set of roots is ignored rather than
/// adopted — otherwise an interrupted dev run's frontier could silently
/// truncate a full harvest to the dev subtree, or vice versa. Never returns
/// `Err` and never panics — every failure path ends in a [`LedgerEntry`]
/// (record, don't skip).
pub async fn bfs(
    source: &mut impl ListingSource,
    roots: &[String],
    checkpoint: &Path,
    limit: Option<u64>,
    scope_mode: BfsScope,
) -> TraverseOutcome {
    let mut out = TraverseOutcome::default();
    let normalized_roots: Vec<String> = roots.iter().map(|r| normalize_dir(r)).collect();
    let (mut pending, mut visited) = load_checkpoint(checkpoint, &normalized_roots)
        .unwrap_or_else(|| (normalized_roots.clone(), BTreeSet::new()));
    // Replay visited dirs first — cache-fresh in production, so this costs
    // no network and repopulates `records` after an interrupted run. The
    // queue is deduped as it is built: a checkpoint written mid-replay can
    // name a dir in both `visited` and `pending`, and processing it twice
    // would double its records (and trip `--limit` early).
    let mut queue: VecDeque<String> = VecDeque::new();
    let mut enqueued: BTreeSet<String> = BTreeSet::new();
    for dir in visited.iter().cloned().chain(pending.drain(..)) {
        if enqueued.insert(dir.clone()) {
            queue.push_back(dir);
        }
    }

    while !queue.is_empty() {
        if at_limit(&out, limit) {
            break;
        }
        // The limit check above ran while `queue` was still non-empty, so
        // this never panics — and critically, an early `break` leaves the
        // about-to-be-processed dir in `queue`, keeping `queue.is_empty()`
        // an honest completion signal below (a limit stop must not delete
        // the checkpoint out from under unfinished work).
        let dir = queue.pop_front().expect("checked non-empty");
        let discovered = process_dir(source, &dir, None, &mut out).await;
        visited.insert(dir);
        for sub in discovered {
            let sub = normalize_dir(&sub);
            let follow = match scope_mode {
                BfsScope::IncludeTable => decide(&sub) == ScopeDecision::Include,
                BfsScope::Subtree => normalized_roots.iter().any(|r| sub.starts_with(r.as_str())),
            };
            if follow && enqueued.insert(sub.clone()) {
                queue.push_back(sub);
            }
        }
        // `pending` records only genuinely-unvisited work: replay entries
        // still in the queue are already carried by `visited`, and writing
        // them to both sides would recreate the duplicate on resume.
        let still_pending: Vec<&String> = queue.iter().filter(|d| !visited.contains(*d)).collect();
        save_checkpoint(checkpoint, &normalized_roots, &still_pending, &visited);
    }
    truncate_to_limit(&mut out, limit);
    if queue.is_empty() {
        let _ = std::fs::remove_file(checkpoint);
    }
    out
}

/// One directory: fetch, ledger failures, collect records, return
/// discovered subdirectory paths (used by BFS; ignored by `enrich`).
async fn process_dir(
    source: &mut impl ListingSource,
    dir: &str,
    tree: Option<&ArchiveTree>,
    out: &mut TraverseOutcome,
) -> Vec<String> {
    out.dirs_processed += 1;
    let outcome = match source.getcontents(dir).await {
        Ok(o) => o,
        Err(e) => {
            out.ledger.push(ledger_for(dir, &e));
            return Vec::new();
        }
    };
    if outcome.listing.is_suspect() {
        // §4.1: never an empty directory — bad paths answer identically.
        // A suspect response produced no listing, so it must never count
        // as a "changed" dir even if the cache layer's hash comparison
        // says otherwise — check this before the `changed` count, not
        // after.
        out.ledger.push(LedgerEntry {
            path: dir.to_owned(),
            action: "getcontents".into(),
            kind: LedgerKind::SuspectPath,
            detail: "content.file and content.dir both null".into(),
            attempts: 1,
        });
        return Vec::new();
    }
    if outcome.changed == Some(true) {
        out.changed_dirs += 1;
    }
    let (files, dirs) = outcome.listing.into_parts();
    for file in files {
        if let Some(tree) = tree
            && let Some(listed) = tree.size_of(dir, &file.filename)
            && listed != file.size
        {
            out.ledger.push(LedgerEntry {
                path: format!("{dir}{}", file.filename),
                action: "getcontents".into(),
                kind: LedgerKind::SizeMismatch,
                detail: format!("api size {} vs ls-laR size {listed}", file.size),
                attempts: 1,
            });
        }
        out.records.push(file);
    }
    dirs.into_iter().map(|d| d.name).collect()
}

fn ledger_for(dir: &str, e: &ApiCallError) -> LedgerEntry {
    let (kind, detail, attempts) = match e {
        ApiCallError::Http { attempts, detail } => {
            (LedgerKind::HttpError, detail.clone(), *attempts)
        }
        ApiCallError::Api {
            fault_kind,
            message,
        } => (LedgerKind::HttpError, format!("{fault_kind}: {message}"), 1),
        ApiCallError::Shape(msg) => (LedgerKind::ParseError, msg.clone(), 1),
    };
    LedgerEntry {
        path: dir.to_owned(),
        action: "getcontents".into(),
        kind,
        detail,
        attempts,
    }
}

fn at_limit(out: &TraverseOutcome, limit: Option<u64>) -> bool {
    limit.is_some_and(|l| u64::try_from(out.records.len()).unwrap_or(u64::MAX) >= l)
}

fn truncate_to_limit(out: &mut TraverseOutcome, limit: Option<u64>) {
    if let Some(l) = limit {
        out.records
            .truncate(usize::try_from(l).unwrap_or(usize::MAX));
    }
}

fn progress_bar(len: usize) -> indicatif::ProgressBar {
    if len < 2 {
        return indicatif::ProgressBar::hidden();
    }
    let bar = indicatif::ProgressBar::new(u64::try_from(len).unwrap_or(u64::MAX));
    bar.set_style(
        indicatif::ProgressStyle::with_template("{bar:40} {pos}/{len} dirs ({eta} left) {msg}")
            .expect("static template is valid"),
    );
    bar
}

/// On-disk BFS frontier (§4.6): the queue plus everything already
/// processed, so a resumed run knows both what's left and what to replay.
/// `roots` records which call produced this frontier — the file is shared
/// across dev-scoped and full runs, so a mismatch there means this
/// checkpoint belongs to a different run and must not be adopted.
/// `#[serde(default)]` keeps pre-fix checkpoints (written before this field
/// existed) deserializable; they read as `roots: []`, which never matches a
/// real (non-empty) root set and so are correctly treated as mismatched.
#[derive(Serialize, Deserialize)]
struct Checkpoint {
    #[serde(default)]
    roots: Vec<String>,
    pending: Vec<String>,
    visited: Vec<String>,
}

/// Load `path`'s checkpoint, but only if it was written for the same
/// `roots` as this call. A mismatched checkpoint is discarded (logged, not
/// silently ignored) so the walk starts fresh from `roots` — the file
/// itself is left in place to be overwritten by the first new save.
fn load_checkpoint(path: &Path, roots: &[String]) -> Option<(Vec<String>, BTreeSet<String>)> {
    let ckpt: Checkpoint = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
    let stored: BTreeSet<&str> = ckpt.roots.iter().map(String::as_str).collect();
    let current: BTreeSet<&str> = roots.iter().map(String::as_str).collect();
    if stored != current {
        tracing::warn!(
            checkpoint = %path.display(),
            stored_roots = ?ckpt.roots,
            current_roots = ?roots,
            "ignoring BFS checkpoint written for different roots"
        );
        return None;
    }
    Some((ckpt.pending, ckpt.visited.into_iter().collect()))
}

fn save_checkpoint(path: &Path, roots: &[String], pending: &[&String], visited: &BTreeSet<String>) {
    let ckpt = Checkpoint {
        roots: roots.to_vec(),
        pending: pending.iter().map(|s| (*s).clone()).collect(),
        visited: visited.iter().cloned().collect(),
    };
    if let Ok(bytes) = serde_json::to_vec(&ckpt)
        && let Err(e) = atomic_write(path, &bytes)
    {
        tracing::warn!(error = %e, "could not write BFS checkpoint");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::model::ContentListing;
    use std::collections::BTreeMap;

    /// Canned listing source; records every requested dir.
    struct Fake {
        responses: BTreeMap<String, serde_json::Value>,
        calls: Vec<String>,
        fail_with_http: Vec<String>,
        changed_true: Vec<String>,
    }

    impl Fake {
        fn new() -> Self {
            Self {
                responses: BTreeMap::new(),
                calls: Vec::new(),
                fail_with_http: Vec::new(),
                changed_true: Vec::new(),
            }
        }

        fn dir(mut self, path: &str, files: &[(u64, &str, u64)], subdirs: &[&str]) -> Self {
            let files: Vec<serde_json::Value> = files
                .iter()
                .map(|(id, name, size)| {
                    serde_json::json!({
                        "id": id, "dir": path, "filename": name, "size": size,
                        "age": 0, "email": "x@y.z"
                    })
                })
                .collect();
            let dirs: Vec<serde_json::Value> = subdirs
                .iter()
                .map(|d| serde_json::json!({"id": 1, "name": d.trim_end_matches('/')}))
                .collect();
            let files = if files.is_empty() {
                serde_json::Value::Null
            } else {
                serde_json::Value::Array(files)
            };
            let dirs = if dirs.is_empty() {
                serde_json::Value::Null
            } else {
                serde_json::Value::Array(dirs)
            };
            // At least one non-null key unless the caller wants a suspect dir.
            self.responses.insert(
                path.to_owned(),
                serde_json::json!({"file": files, "dir": dirs}),
            );
            self
        }

        fn suspect(mut self, path: &str) -> Self {
            self.responses.insert(
                path.to_owned(),
                serde_json::json!({"file": null, "dir": null}),
            );
            self
        }

        fn failing(mut self, path: &str) -> Self {
            self.fail_with_http.push(path.to_owned());
            self
        }

        /// Report `changed: Some(true)` for this path's response — used to
        /// prove a suspect path never counts as "changed" even when the
        /// cache layer's hash comparison would otherwise say so.
        fn changed(mut self, path: &str) -> Self {
            self.changed_true.push(path.to_owned());
            self
        }
    }

    impl ListingSource for Fake {
        // Sync body in the desugared async-trait form (`impl Future` +
        // `ready`) — clippy 1.98's `unused_async_trait_impl`.
        fn getcontents(
            &mut self,
            dir: &str,
        ) -> impl std::future::Future<Output = Result<FetchOutcome, ApiCallError>> {
            self.calls.push(dir.to_owned());
            if self.fail_with_http.iter().any(|p| p == dir) {
                return std::future::ready(Err(ApiCallError::Http {
                    attempts: 6,
                    detail: "HTTP 500".into(),
                }));
            }
            let body = self
                .responses
                .get(dir)
                .cloned()
                .unwrap_or(serde_json::json!({"file": null, "dir": null}));
            let listing: ContentListing = serde_json::from_value(body).unwrap();
            let changed = Some(self.changed_true.iter().any(|p| p == dir));
            std::future::ready(Ok(FetchOutcome {
                listing,
                from_cache: false,
                changed,
            }))
        }
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
    fn worklist_filters_by_scope_and_collects_triage() {
        let tree = tree_with(&[
            ("", &[]),
            ("levels/doom/0-9/", &[("a.zip", 10)]),
            ("levels/reviews/", &[]),
            ("music/", &[]),
            ("misc/", &[("odd.zip", 5)]),
            ("brandnew/", &[("new.zip", 7)]),
            ("themes/x/", &[]),
        ]);
        let (work, triage) = worklist_from_tree(&tree, None);
        assert_eq!(work, vec!["levels/doom/0-9/", "themes/x/"]);
        assert_eq!(triage, vec!["brandnew/"]);
    }

    #[test]
    fn worklist_root_override_ignores_scope() {
        let tree = tree_with(&[("misc/", &[]), ("misc/old/", &[]), ("levels/doom/", &[])]);
        let (work, triage) = worklist_from_tree(&tree, Some("misc"));
        assert_eq!(work, vec!["misc/", "misc/old/"]);
        assert!(triage.is_empty());
    }

    #[tokio::test]
    async fn enrich_collects_ledgers_and_cross_checks() {
        let mut fake = Fake::new()
            .dir(
                "levels/doom/0-9/",
                &[(1, "a.zip", 10), (2, "b.zip", 999)],
                &[],
            )
            .suspect("levels/doom/a-c/")
            .failing("levels/doom/d-f/");
        let tree = tree_with(&[(
            "levels/doom/0-9/",
            &[("a.zip", 10), ("b.zip", 42)], // b.zip size disagrees with the API's 999
        )]);
        let worklist = vec![
            "levels/doom/0-9/".to_owned(),
            "levels/doom/a-c/".to_owned(),
            "levels/doom/d-f/".to_owned(),
        ];
        let out = enrich(&mut fake, &worklist, Some(&tree), None).await;
        assert_eq!(out.records.len(), 2); // mismatch is recorded, not skipped
        assert_eq!(out.dirs_processed, 3);
        let kinds: Vec<&LedgerKind> = out.ledger.iter().map(|e| &e.kind).collect();
        assert!(kinds.contains(&&LedgerKind::SuspectPath));
        assert!(kinds.contains(&&LedgerKind::HttpError));
        assert!(kinds.contains(&&LedgerKind::SizeMismatch));
        assert_eq!(out.ledger.len(), 3);
    }

    #[tokio::test]
    async fn enrich_respects_limit() {
        let mut fake = Fake::new()
            .dir("a/", &[(1, "1.zip", 1), (2, "2.zip", 1)], &[])
            .dir("b/", &[(3, "3.zip", 1)], &[]);
        let out = enrich(
            &mut fake,
            &["a/".to_owned(), "b/".to_owned()],
            None,
            Some(2),
        )
        .await;
        assert_eq!(out.records.len(), 2);
        assert_eq!(fake.calls, vec!["a/"]); // b/ never requested
    }

    #[tokio::test]
    async fn bfs_discovers_scoped_subtree_and_checkpoints() {
        let tmp = tempfile::tempdir().unwrap();
        let ckpt = tmp.path().join("bfs-frontier.json");
        let mut fake = Fake::new()
            .dir("levels/", &[], &["levels/doom", "levels/reviews"])
            .dir("levels/doom/", &[(1, "a.zip", 5)], &[])
            .dir("levels/reviews/", &[(9, "review.zip", 1)], &[]);
        let out = bfs(
            &mut fake,
            &["levels/".to_owned()],
            &ckpt,
            None,
            BfsScope::IncludeTable,
        )
        .await;
        // levels/reviews/ is Skip-scoped: discovered but never enqueued.
        assert!(!fake.calls.contains(&"levels/reviews/".to_owned()));
        assert_eq!(out.records.len(), 1);
        assert_eq!(out.records[0].id, 1);
        // Completed run cleans its checkpoint.
        assert!(!ckpt.exists());
    }

    #[tokio::test]
    async fn bfs_subtree_mode_follows_non_include_dirs_under_root() {
        // A dev `--root` at a Skip root must traverse its whole subtree
        // (tree-mode `--root` parity), while dirs outside the root stay out.
        let tmp = tempfile::tempdir().unwrap();
        let ckpt = tmp.path().join("bfs-frontier.json");
        let mut fake = Fake::new()
            .dir("misc/", &[(1, "odd.zip", 5)], &["misc/old", "levels/doom"])
            .dir("misc/old/", &[(2, "older.zip", 3)], &[])
            .dir("levels/doom/", &[(9, "a.zip", 1)], &[]);
        let out = bfs(
            &mut fake,
            &["misc/".to_owned()],
            &ckpt,
            None,
            BfsScope::Subtree,
        )
        .await;
        assert!(fake.calls.contains(&"misc/old/".to_owned()));
        assert!(!fake.calls.contains(&"levels/doom/".to_owned()));
        assert_eq!(out.records.len(), 2);
        assert!(!ckpt.exists());
    }

    #[tokio::test]
    async fn bfs_resumes_from_checkpoint() {
        let tmp = tempfile::tempdir().unwrap();
        let ckpt = tmp.path().join("bfs-frontier.json");
        std::fs::write(
            &ckpt,
            serde_json::json!({
                "roots": ["levels/"],
                "pending": ["levels/doom/"],
                "visited": ["levels/"]
            })
            .to_string(),
        )
        .unwrap();
        let mut fake = Fake::new().dir("levels/", &[], &["levels/doom"]).dir(
            "levels/doom/",
            &[(1, "a.zip", 5)],
            &[],
        );
        let out = bfs(
            &mut fake,
            &["levels/".to_owned()],
            &ckpt,
            None,
            BfsScope::IncludeTable,
        )
        .await;
        // Visited dirs are replayed (cache-fresh in production), pending resumed.
        assert!(fake.calls.contains(&"levels/".to_owned()));
        assert!(fake.calls.contains(&"levels/doom/".to_owned()));
        assert_eq!(out.records.len(), 1);
        assert!(!ckpt.exists());
    }

    #[tokio::test]
    async fn bfs_resume_dedups_dirs_present_in_both_pending_and_visited() {
        // A checkpoint written mid-replay can carry a dir on both sides;
        // resuming from it must process the dir exactly once, or its
        // records double and `--limit` trips early.
        let tmp = tempfile::tempdir().unwrap();
        let ckpt = tmp.path().join("bfs-frontier.json");
        std::fs::write(
            &ckpt,
            serde_json::json!({
                "roots": ["levels/"],
                "pending": ["levels/", "levels/doom/"],
                "visited": ["levels/"]
            })
            .to_string(),
        )
        .unwrap();
        let mut fake = Fake::new()
            .dir("levels/", &[(7, "root.zip", 2)], &["levels/doom"])
            .dir("levels/doom/", &[(1, "a.zip", 5)], &[]);
        let out = bfs(
            &mut fake,
            &["levels/".to_owned()],
            &ckpt,
            None,
            BfsScope::IncludeTable,
        )
        .await;
        assert_eq!(
            fake.calls.iter().filter(|c| *c == "levels/").count(),
            1,
            "duplicated across pending and visited must fetch once: {:?}",
            fake.calls
        );
        assert_eq!(out.records.len(), 2);
        assert!(!ckpt.exists());
    }

    #[tokio::test]
    async fn bfs_ignores_checkpoint_written_for_different_roots() {
        let tmp = tempfile::tempdir().unwrap();
        let ckpt = tmp.path().join("bfs-frontier.json");
        // A frontier left behind by a prior run scoped to a different root
        // (e.g. a dev `--root misc` run interrupted mid-walk).
        let stale = Checkpoint {
            roots: vec!["misc/".to_owned()],
            pending: vec!["misc/old/".to_owned()],
            visited: vec!["misc/".to_owned()],
        };
        std::fs::write(&ckpt, serde_json::to_vec(&stale).unwrap()).unwrap();
        let mut fake = Fake::new().dir("levels/", &[(1, "a.zip", 5)], &[]);
        let out = bfs(
            &mut fake,
            &["levels/".to_owned()],
            &ckpt,
            None,
            BfsScope::IncludeTable,
        )
        .await;
        // The stale "misc/" frontier must never be visited or replayed.
        assert!(!fake.calls.contains(&"misc/".to_owned()));
        assert!(!fake.calls.contains(&"misc/old/".to_owned()));
        // The walk starts fresh from the roots given to this call.
        assert!(fake.calls.contains(&"levels/".to_owned()));
        assert_eq!(out.records.len(), 1);
    }

    #[tokio::test]
    async fn bfs_limit_stop_keeps_checkpoint_and_pending_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let ckpt = tmp.path().join("bfs-frontier.json");
        let mut fake = Fake::new()
            .dir("a/", &[(1, "1.zip", 1), (2, "2.zip", 1)], &[])
            .dir("b/", &[(3, "3.zip", 1)], &[]);
        let out = bfs(
            &mut fake,
            &["a/".to_owned(), "b/".to_owned()],
            &ckpt,
            Some(2),
            BfsScope::IncludeTable,
        )
        .await;
        assert_eq!(out.records.len(), 2);
        // The limit tripped before "b/" was dequeued: it must never be
        // requested, and — unlike a completed run — the checkpoint must
        // survive so a resume can pick it back up.
        assert!(!fake.calls.contains(&"b/".to_owned()));
        assert!(ckpt.exists());
        let roots = ["a/".to_owned(), "b/".to_owned()];
        let (pending, _visited) = load_checkpoint(&ckpt, &roots).expect("checkpoint present");
        assert_eq!(pending, vec!["b/".to_owned()]);
    }

    #[tokio::test]
    async fn suspect_dir_with_changed_flag_does_not_count_as_changed() {
        // A suspect response carries no listing at all — even if the cache
        // layer's body-hash comparison says the (null, null) body moved,
        // that must not be surfaced as a "changed" directory.
        let mut fake = Fake::new()
            .suspect("levels/doom/a-c/")
            .changed("levels/doom/a-c/");
        let worklist = vec!["levels/doom/a-c/".to_owned()];
        let out = enrich(&mut fake, &worklist, None, None).await;
        assert_eq!(out.changed_dirs, 0);
        assert_eq!(out.ledger.len(), 1);
    }
}
