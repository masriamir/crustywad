//! Phase-3 `harvest-outliers` orchestrator: curated non-idgames megawad
//! analysis (DESIGN.md §6.4, spec §8).
//!
//! idgames enforces its own upload limits, so the corpus's upper tail is
//! truncated by the archive's own cap — the largest modern megawads
//! (Cacoward-tier releases) live elsewhere. `xtask/outliers.toml` is a
//! small, hand-curated, *committed* list of direct download URLs for those
//! releases; this module fetches and inspects each one with the same
//! central-directory-only machinery `zips::inspect::inspect_zip` gives
//! phase 2, over [`crate::zips::url_source::UrlRanges`] (a single-URL
//! [`crate::zips::inspect::RangeSource`], no mirror pool, no failover) —
//! and writes a record for every entry, whether or not the fetch succeeded
//! ("record, don't skip", matching phase 1/2's discipline).
//!
//! **No full-download fallback** (spec §2.2, a locked decision): a host
//! that refuses range requests gets a `no_range_support` ledger entry and
//! stays that way — outliers are large by design, so falling back to a
//! full download would blow the politeness budget this module is built
//! to respect in the first place.
//!
//! **Politeness** (ADR-0030 §4 spirit): strictly sequential — one entry at
//! a time, never a pool — with a full second of spacing between requests.
//! The curated list is small (n ≈ 8), so there's no resumability store: a
//! rerun simply refetches every central directory (a few hundred KiB
//! total across the whole list), matching spec §8's "no resumability
//! store" call.
//!
//! **Status set** is [`FetchStatus`] minus the mirror-pool/fallback-only
//! variants: `Ok`, `NoRangeSupport`, `ZipParseError`, `FetchError`. A `404`
//! maps to `FetchError` here, not `Mirror404All` — that variant names a
//! *pool* fact (both mirrors independently 404ing), which has no meaning
//! for a single URL (spec §8).

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use anyhow::Context as _;
use chrono::Utc;

use crate::schema::{self, FetchStatus, LedgerEntry, LedgerKind, OutlierRecord, OutliersManifest};
use crate::zips::inspect::{self, FetchFailure, InspectError, Inspection};
use crate::zips::range_reader::TransferCounters;
use crate::zips::url_source::UrlRanges;

/// Spacing between entries (ADR-0030 §4 politeness spirit, spec §8).
const ENTRY_SPACING: Duration = Duration::from_secs(1);

/// One curated outlier entry from `xtask/outliers.toml`'s `[[outlier]]`
/// table (§6.4).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct OutlierSpec {
    /// Our own kebab-case identifier — never author-supplied free text
    /// (ADR-0030 §3).
    pub slug: String,
    /// Direct zip download URL.
    pub url: String,
    /// One-line human rationale for why the entry is on the list. Read only
    /// for [`parse_outliers_toml`]'s non-empty validation — never copied
    /// into any output (the ADR-0030 §3 free-text ban) and never surfaced
    /// past parse time, so it's documentary for readers of the committed
    /// TOML file, not a value the orchestrator otherwise consumes.
    pub note: String,
}

/// The `[[outlier]]` array-of-tables shape `xtask/outliers.toml` parses
/// into.
#[derive(Debug, serde::Deserialize)]
struct OutliersToml {
    outlier: Vec<OutlierSpec>,
}

/// Parse and validate `xtask/outliers.toml`: every entry needs a non-empty
/// `slug`/`url`/`note`, and no two entries may share a `slug` (it's the
/// output key `write_outliers_jsonl` sorts/dedupes by, per
/// [`crate::schema::write_outliers_jsonl`]).
///
/// # Errors
/// Malformed TOML, an empty required field, or a duplicate `slug`.
pub fn parse_outliers_toml(text: &str) -> anyhow::Result<Vec<OutlierSpec>> {
    let parsed: OutliersToml = toml::from_str(text).context("parsing outliers.toml")?;
    let mut seen = std::collections::BTreeSet::new();
    for spec in &parsed.outlier {
        anyhow::ensure!(!spec.slug.is_empty(), "outlier entry has an empty slug");
        anyhow::ensure!(
            !spec.url.is_empty(),
            "outlier entry {:?} has an empty url",
            spec.slug
        );
        anyhow::ensure!(
            !spec.note.is_empty(),
            "outlier entry {:?} has an empty note",
            spec.slug
        );
        anyhow::ensure!(
            seen.insert(spec.slug.clone()),
            "duplicate slug {:?} in outliers.toml",
            spec.slug
        );
    }
    Ok(parsed.outlier)
}

/// Map one entry's fetch/inspect outcome onto (§5.6 status, optional
/// ledger line). Pure — the orchestrator's per-entry decision logic, kept
/// separate from the network loop so it's unit-testable without a live
/// server (task-5 brief). [`Self`]-less free function: called both for a
/// successful `discover_size` followed by `inspect_zip`, and for a
/// `discover_size` failure alone (wrapped as `Err(InspectError::Fetch(_))`
/// by the caller — see [`fetch_and_inspect`]), so both stages share one
/// mapping.
pub(crate) fn entry_outcome(
    slug: &str,
    result: &Result<Inspection, InspectError>,
) -> (FetchStatus, Option<LedgerEntry>) {
    match result {
        Ok(_) => (FetchStatus::Ok, None),
        Err(InspectError::Fetch(FetchFailure::RangeUnsupported)) => (
            FetchStatus::NoRangeSupport,
            Some(ledger_line(
                slug,
                LedgerKind::HttpError,
                "host refuses range requests".to_owned(),
            )),
        ),
        // No mirror pool here, so there's no `Mirror404All` equivalent —
        // a single host 404ing is a plain fetch error (module doc).
        Err(InspectError::Fetch(FetchFailure::NotFound)) => (
            FetchStatus::FetchError,
            Some(ledger_line(
                slug,
                LedgerKind::HttpError,
                "404 not found".to_owned(),
            )),
        ),
        Err(InspectError::Fetch(FetchFailure::Http(detail))) => (
            FetchStatus::FetchError,
            Some(ledger_line(slug, LedgerKind::HttpError, detail.clone())),
        ),
        Err(e @ (InspectError::CdTooLarge { .. } | InspectError::TooChatty { .. })) => (
            FetchStatus::ZipParseError,
            Some(ledger_line(slug, LedgerKind::ParseError, e.to_string())),
        ),
        Err(InspectError::Parse(detail)) => (
            FetchStatus::ZipParseError,
            Some(ledger_line(slug, LedgerKind::ParseError, detail.clone())),
        ),
    }
}

/// Build one `outliers-errors.jsonl` line — action is always
/// `"harvest-outliers"`, `path` is the slug (there's no archive-tree path
/// for a curated outlier).
fn ledger_line(slug: &str, kind: LedgerKind, detail: String) -> LedgerEntry {
    LedgerEntry {
        path: slug.to_owned(),
        action: "harvest-outliers".into(),
        kind,
        detail,
        attempts: 1,
    }
}

/// Build the §6.4 record (and, via [`entry_outcome`], any ledger line) for
/// one entry. Pure — the orchestrator's other per-entry decision, tested
/// directly (task-5 brief: "record construction ... must be pure and
/// unit-tested"). `zip_size` is `0` when [`fetch_and_inspect`]'s
/// `discover_size` stage itself failed — no size is known for that entry
/// (record, don't skip: it still gets a record, just an empty one beyond
/// its status).
pub(crate) fn build_record(
    spec: &OutlierSpec,
    zip_size: u64,
    result: &Result<Inspection, InspectError>,
) -> (OutlierRecord, Option<LedgerEntry>) {
    let (fetch_status, ledger) = entry_outcome(&spec.slug, result);
    let (zip64, member_count, wads, other_members) = match result {
        Ok(inspection) => (
            inspection.zip64,
            inspection.member_count,
            inspection.wads.clone(),
            inspection.other_members.clone(),
        ),
        Err(_) => (false, 0, Vec::new(), Vec::new()),
    };
    let record = OutlierRecord {
        slug: spec.slug.clone(),
        url: spec.url.clone(),
        zip_size,
        zip64,
        member_count,
        wads,
        other_members,
        fetch_status,
    };
    (record, ledger)
}

/// Record count per `fetch_status` wire value (mirrors
/// `zips::status_counts`'s convention).
fn status_counts(records: &[OutlierRecord]) -> std::collections::BTreeMap<String, u64> {
    let mut counts = std::collections::BTreeMap::new();
    for record in records {
        let label = serde_json::to_value(record.fetch_status)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_default();
        *counts.entry(label).or_insert(0_u64) += 1;
    }
    counts
}

/// HEAD-then-ranged-inspect one entry over a fresh [`UrlRanges`]. Returns
/// the discovered size (`0` when discovery itself failed — no size is
/// known) and the inspection result; [`build_record`] turns this into the
/// record/ledger pair. An unparseable `url` (defensive — the curated list
/// is expected to hold only valid URLs, but "never panic or skip" applies
/// here same as phase 1/2) is reported the same way a transport failure
/// would be, via [`InspectError::Fetch`], so it flows through the exact
/// same [`entry_outcome`] mapping as every other failure — no separate
/// status is needed for it.
async fn fetch_and_inspect(
    client: &reqwest::Client,
    spec: &OutlierSpec,
    counters: Arc<TransferCounters>,
) -> (u64, Result<Inspection, InspectError>) {
    let url = match reqwest::Url::parse(&spec.url) {
        Ok(url) => url,
        Err(e) => {
            return (
                0,
                Err(InspectError::Fetch(FetchFailure::Http(format!(
                    "invalid url: {e}"
                )))),
            );
        }
    };
    let mut source = UrlRanges::new(client.clone(), url, counters);
    match source.discover_size().await {
        Ok(size) => (size, inspect::inspect_zip(&mut source, size).await),
        Err(f) => (0, Err(InspectError::Fetch(f))),
    }
}

/// Run `xtask harvest-outliers`. `--root` doesn't apply — there is no
/// archive-tree path to scope by for a curated URL list — so it's rejected
/// rather than silently ignored (task-5 brief). `--limit` truncates the
/// (already-parsed) worklist to its first N entries and, like phase 1/2,
/// switches outputs to `data/dev/` (`scoped = limit.is_some()`).
///
/// # Errors
/// `--root` was given; `xtask/outliers.toml` is missing/malformed/invalid
/// (see [`parse_outliers_toml`]); or an environmental failure (directories,
/// HTTP client setup, output writes).
pub fn run(root: Option<&str>, limit: Option<usize>) -> anyhow::Result<()> {
    if root.is_some() {
        anyhow::bail!("--root does not apply to harvest-outliers");
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("building tokio runtime")?;
    runtime.block_on(run_async(limit))
}

/// Shape the parsed TOML entries into this run's worklist: `--limit` (when
/// given) truncates to its first N entries, in file order. No `--root`
/// scoping — there's no archive-tree path to filter a curated URL list by,
/// which is exactly why [`run`] rejects `--root` outright rather than
/// silently ignoring it. Pure and unit-tested without network, mirroring
/// `zips::worklist`'s shaping precedent (`zips/mod.rs`).
fn worklist(mut specs: Vec<OutlierSpec>, limit: Option<usize>) -> Vec<OutlierSpec> {
    if let Some(limit) = limit {
        specs.truncate(limit);
    }
    specs
}

async fn run_async(limit: Option<usize>) -> anyhow::Result<()> {
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("outliers.toml");
    let text = std::fs::read_to_string(&toml_path)
        .with_context(|| format!("reading {}", toml_path.display()))?;
    let specs = worklist(parse_outliers_toml(&text)?, limit);
    let entries_total = u64::try_from(specs.len()).unwrap_or(u64::MAX);

    let scoped = limit.is_some();
    let out_dir = crate::phase1::output_dir(scoped);
    std::fs::create_dir_all(&out_dir).with_context(|| format!("creating {}", out_dir.display()))?;

    let client = crate::mirror::build_zips_http()?;
    let counters = Arc::new(TransferCounters::new());
    let started_at = Utc::now();

    let mut records = Vec::with_capacity(specs.len());
    let mut ledger = Vec::new();
    for (i, spec) in specs.iter().enumerate() {
        if i > 0 {
            tokio::time::sleep(ENTRY_SPACING).await;
        }
        let (zip_size, result) = fetch_and_inspect(&client, spec, Arc::clone(&counters)).await;
        let (record, ledger_entry) = build_record(spec, zip_size, &result);
        records.push(record);
        if let Some(entry) = ledger_entry {
            ledger.push(entry);
        }
    }

    let status_counts = status_counts(&records);
    let records_written =
        schema::write_outliers_jsonl(&out_dir.join("outliers-wads.jsonl"), records)?;
    let ledger_count = schema::write_ledger(&out_dir.join("outliers-errors.jsonl"), ledger)?;

    let duration = (Utc::now() - started_at).num_seconds().max(0);
    let manifest = OutliersManifest {
        id: format!("harvest-outliers-{}", started_at.format("%Y%m%dT%H%M%SZ")),
        started_at: started_at.to_rfc3339(),
        duration_secs: u64::try_from(duration).unwrap_or(0),
        tool_version: schema::tool_version(),
        git_rev: schema::git_rev(),
        limit: limit.map(|l| u64::try_from(l).unwrap_or(u64::MAX)),
        entries_total,
        records_written,
        ledger_count,
        range_requests: counters.requests.load(Ordering::Relaxed),
        bytes_transferred: counters.bytes.load(Ordering::Relaxed),
        status_counts,
    };
    schema::write_outliers_manifest(&out_dir.join("outliers-manifest.json"), &manifest)?;

    tracing::info!(
        records = manifest.records_written,
        ledger = manifest.ledger_count,
        "harvest-outliers complete"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zips::inspect::{FetchFailure, Inspection};

    #[test]
    fn toml_parses_and_validates() {
        let good = r#"
            [[outlier]]
            slug = "example-wad"
            url = "https://example.com/files/example.zip"
            note = "size-tail case"
        "#;
        let specs = parse_outliers_toml(good).unwrap();
        assert_eq!(specs[0].slug, "example-wad");
        let dup = format!("{good}{good}");
        assert!(
            parse_outliers_toml(&dup)
                .unwrap_err()
                .to_string()
                .contains("duplicate slug")
        );
        assert!(
            parse_outliers_toml("[[outlier]]\nslug = \"\"\nurl = \"x\"\nnote = \"n\"").is_err()
        );
    }

    #[test]
    fn toml_rejects_empty_url_and_note() {
        assert!(
            parse_outliers_toml("[[outlier]]\nslug = \"s\"\nurl = \"\"\nnote = \"n\"").is_err()
        );
        assert!(
            parse_outliers_toml("[[outlier]]\nslug = \"s\"\nurl = \"https://x\"\nnote = \"\"")
                .is_err()
        );
    }

    #[test]
    fn the_committed_starter_toml_parses_and_validates() {
        let text = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("outliers.toml"),
        )
        .expect("xtask/outliers.toml must exist");
        let specs = parse_outliers_toml(&text).expect("starter outliers.toml must be valid");
        assert!(!specs.is_empty());
    }

    #[test]
    fn root_flag_is_rejected() {
        let err = run(Some("levels/doom/"), None).unwrap_err();
        assert!(err.to_string().contains("--root does not apply"));
    }

    fn specs(n: usize) -> Vec<OutlierSpec> {
        (0..n)
            .map(|i| OutlierSpec {
                slug: format!("s{i}"),
                url: format!("https://example.com/{i}.zip"),
                note: "n".to_owned(),
            })
            .collect()
    }

    #[test]
    fn worklist_truncates_to_the_limit_in_file_order() {
        // limit smaller than the list: keeps the first N, in order.
        let limited = worklist(specs(5), Some(2));
        assert_eq!(
            limited.iter().map(|s| s.slug.clone()).collect::<Vec<_>>(),
            vec!["s0".to_owned(), "s1".to_owned()]
        );
    }

    #[test]
    fn worklist_with_no_limit_keeps_every_entry() {
        let all = worklist(specs(3), None);
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn worklist_limit_larger_than_the_list_keeps_every_entry() {
        let all = worklist(specs(3), Some(100));
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn worklist_limit_zero_empties_the_list() {
        let none = worklist(specs(3), Some(0));
        assert!(none.is_empty());
    }

    #[test]
    fn scoped_output_dir_matches_the_limit_flag() {
        // Mirrors phase1::tests::scoped_runs_use_the_dev_output_dir: pins
        // run_async's `scoped = limit.is_some()` decision, the same way
        // phase 1/2 pin theirs — a scoped (`--limit`) run must never write
        // into the same output dir as a full run.
        let limit: Option<usize> = Some(3);
        assert!(crate::phase1::output_dir(limit.is_some()).ends_with("data/dev"));
        let no_limit: Option<usize> = None;
        assert!(crate::phase1::output_dir(no_limit.is_some()).ends_with("data"));
    }

    #[test]
    fn outcomes_map_to_statuses_and_ledger() {
        let (s, l) = entry_outcome(
            "x",
            &Err(InspectError::Fetch(FetchFailure::RangeUnsupported)),
        );
        assert!(matches!(s, FetchStatus::NoRangeSupport));
        assert!(l.is_some());
        let (s, _) = entry_outcome("x", &Err(InspectError::Fetch(FetchFailure::NotFound)));
        assert!(matches!(s, FetchStatus::FetchError)); // no mirror pool → mirror_404_all does not apply
        let (s, _) = entry_outcome("x", &Err(InspectError::Parse("bad".into())));
        assert!(matches!(s, FetchStatus::ZipParseError));
    }

    #[test]
    fn outcomes_cover_every_fetch_failure_and_inspect_error_variant() {
        let (s, l) = entry_outcome(
            "x",
            &Err(InspectError::Fetch(FetchFailure::Http("timeout".into()))),
        );
        assert!(matches!(s, FetchStatus::FetchError));
        assert_eq!(l.unwrap().detail, "timeout");

        let (s, l) = entry_outcome(
            "x",
            &Err(InspectError::CdTooLarge {
                needed: 999_999_999,
            }),
        );
        assert!(matches!(s, FetchStatus::ZipParseError));
        assert!(matches!(l.unwrap().kind, LedgerKind::ParseError));

        let (s, l) = entry_outcome("x", &Err(InspectError::TooChatty { rounds: 12 }));
        assert!(matches!(s, FetchStatus::ZipParseError));
        assert!(matches!(l.unwrap().kind, LedgerKind::ParseError));

        let inspection = Inspection {
            zip64: false,
            member_count: 0,
            wads: Vec::new(),
            other_members: Vec::new(),
        };
        let (s, l) = entry_outcome("x", &Ok(inspection));
        assert!(matches!(s, FetchStatus::Ok));
        assert!(l.is_none());
    }

    #[test]
    fn no_range_support_ledger_line_carries_the_slug_and_action() {
        let (_, l) = entry_outcome(
            "blade-of-agony",
            &Err(InspectError::Fetch(FetchFailure::RangeUnsupported)),
        );
        let l = l.unwrap();
        assert_eq!(l.path, "blade-of-agony");
        assert_eq!(l.action, "harvest-outliers");
        assert!(matches!(l.kind, LedgerKind::HttpError));
    }

    fn spec(slug: &str) -> OutlierSpec {
        OutlierSpec {
            slug: slug.to_owned(),
            url: format!("https://example.com/{slug}.zip"),
            note: "test entry".to_owned(),
        }
    }

    #[test]
    fn build_record_on_success_carries_inspection_fields_and_no_ledger() {
        let inspection = Inspection {
            zip64: true,
            member_count: 2,
            wads: vec![crate::schema::WadMember {
                name: "MAP01.WAD".into(),
                compressed: 100,
                uncompressed: 400,
                method: "deflate".into(),
                encrypted: false,
            }],
            other_members: vec!["readme.txt".into()],
        };
        let (record, ledger) = build_record(&spec("golden-souls-2"), 12_345, &Ok(inspection));
        assert_eq!(record.slug, "golden-souls-2");
        assert_eq!(record.zip_size, 12_345);
        assert!(record.zip64);
        assert_eq!(record.member_count, 2);
        assert_eq!(record.wads.len(), 1);
        assert_eq!(record.other_members, vec!["readme.txt".to_owned()]);
        assert!(matches!(record.fetch_status, FetchStatus::Ok));
        assert!(ledger.is_none());
    }

    #[test]
    fn build_record_on_discovery_failure_is_zero_sized_but_still_a_record() {
        // "record, don't skip": a failed HEAD probe (zip_size 0, since
        // discover_size never learned a size) still yields a full record.
        let (record, ledger) = build_record(
            &spec("total-chaos"),
            0,
            &Err(InspectError::Fetch(FetchFailure::RangeUnsupported)),
        );
        assert_eq!(record.slug, "total-chaos");
        assert_eq!(record.zip_size, 0);
        assert!(!record.zip64);
        assert_eq!(record.member_count, 0);
        assert!(record.wads.is_empty());
        assert!(record.other_members.is_empty());
        assert!(matches!(record.fetch_status, FetchStatus::NoRangeSupport));
        assert!(ledger.is_some());
    }

    #[test]
    fn build_record_on_late_parse_failure_keeps_the_discovered_size() {
        // discover_size succeeded (zip_size known) but inspect_zip failed —
        // the known size is still worth recording.
        let (record, ledger) = build_record(
            &spec("eviternity-ii"),
            555_000_000,
            &Err(InspectError::Parse("bad magic".into())),
        );
        assert_eq!(record.zip_size, 555_000_000);
        assert!(matches!(record.fetch_status, FetchStatus::ZipParseError));
        assert!(ledger.is_some());
    }

    #[test]
    fn status_counts_tallies_by_wire_label() {
        let (ok, _) = build_record(
            &spec("a"),
            10,
            &Ok(Inspection {
                zip64: false,
                member_count: 0,
                wads: Vec::new(),
                other_members: Vec::new(),
            }),
        );
        let (failed, _) = build_record(
            &spec("b"),
            0,
            &Err(InspectError::Fetch(FetchFailure::RangeUnsupported)),
        );
        let counts = status_counts(&[ok, failed]);
        assert_eq!(counts.get("ok"), Some(&1));
        assert_eq!(counts.get("no_range_support"), Some(&1));
    }
}
