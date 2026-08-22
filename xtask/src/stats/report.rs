//! §8 constant recommendations and the `data/stats-report.md` renderer
//! (DESIGN.md §8, §9.3). Everything here is a pure function of an already
//! -built [`StatsJson`]/[`IdgamesStats`]/[`OutliersStats`] tree — no I/O, no
//! recomputation of anything `stats::build_stats` already computed. That
//! split is what makes the trio (`stats.json`, `stats-report.md`,
//! `sweep-corpus.jsonl`) byte-identical across reruns of unchanged inputs
//! (§9.3): every number here is read, never recalculated from raw records.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::schema::{
    Coverage, Distribution, HistogramBucket, IdgamesStats, OutliersStats, RatioDistribution,
    Recommendation, StatsJson, StatsProvenance,
};

const KIB: u64 = 1024;
const MIB: u64 = KIB * 1024;
const GIB: u64 = MIB * 1024;
const TIB: u64 = GIB * 1024;

/// Smallest power of two ≥ `x` (§8 formula primitive): `0` and `1` both map
/// to `1`. Saturates to [`u64::MAX`] for `x > 2^62` instead of calling
/// [`u64::next_power_of_two`] directly. The guard is deliberately more
/// conservative than that method's *actual* panic boundary: it only panics
/// for `x > 2^63` (its correct answer would be `2^64`, which doesn't fit
/// `u64`) — an `x` in `(2^62, 2^63]` would round-trip through
/// `next_power_of_two` just fine. `2^62` is used anyway as a round,
/// easy-to-state cutoff, trading away that legal-but-unneeded sliver of
/// domain rather than hugging the true boundary exactly — unreachable for
/// any real corpus statistic either way (nothing here is within
/// light-years of 2^62 bytes), but a documented saturation beats a latent
/// panic regardless of exactly where the line is drawn.
#[must_use]
pub(crate) fn pow2_ceil(x: u64) -> u64 {
    if x <= 1 {
        return 1;
    }
    if x > (1u64 << 62) {
        return u64::MAX;
    }
    x.next_power_of_two()
}

/// Human-readable byte count (§8): the exact integer, with an IEC unit
/// label appended only when `n` lands exactly on a KiB/MiB/GiB/TiB boundary
/// (e.g. `"2147483648 (2 GiB)"`); otherwise the bare number. Deterministic
/// and lossless — this never rounds, so it can't hide precision the way a
/// "2.0 GiB"-style formatter would.
#[must_use]
pub(crate) fn fmt_bytes(n: u64) -> String {
    if n != 0 && n.is_multiple_of(TIB) {
        format!("{n} ({} TiB)", n / TIB)
    } else if n != 0 && n.is_multiple_of(GIB) {
        format!("{n} ({} GiB)", n / GIB)
    } else if n != 0 && n.is_multiple_of(MIB) {
        format!("{n} ({} MiB)", n / MIB)
    } else if n != 0 && n.is_multiple_of(KIB) {
        format!("{n} ({} KiB)", n / KIB)
    } else {
        n.to_string()
    }
}

/// Outliers value for a "both populations" recommendation row: `None` when
/// the §6.4 supplement is absent or present-but-empty (§9.3/spec §6:
/// "outliers: none analyzed" is stated explicitly rather than silently
/// falling back), `Some` otherwise.
fn outlier_value(
    outliers: Option<&OutliersStats>,
    pick: impl Fn(&OutliersStats) -> u64,
) -> Option<u64> {
    outliers.filter(|o| !o.analyzed.is_empty()).map(pick)
}

/// Builds one "`pow2_ceil(multiplier × max(idgames, outliers))`"-shaped §8
/// row (`wire_cap_zip`, `wire_cap_wad`, `max_member_count`,
/// `max_entry_uncompressed_bytes`): cites the idgames statistic always, the
/// outliers statistic when the §6.4 supplement has analyzed entries (or
/// states "outliers: none analyzed" otherwise), and — when the outliers
/// value is what actually moved the recommendation above the idgames-only
/// number — additionally cites what the idgames-only recommendation would
/// have been, so that number is never lost even though [`Recommendation`]
/// carries no separate idgames-only column.
#[allow(
    clippy::too_many_arguments,
    reason = "one row-builder used by 4 call sites; splitting the args into a struct would not make any call site clearer"
)]
fn combined_row(
    key: &str,
    formula: &str,
    idgames_label: &str,
    idgames_val: u64,
    outliers_label: &str,
    outliers_val: Option<u64>,
    multiplier: u64,
    fmt: impl Fn(u64) -> String,
) -> Recommendation {
    let raw_max = outliers_val.map_or(idgames_val, |o| idgames_val.max(o));
    let value = pow2_ceil(raw_max.saturating_mul(multiplier));
    let mut source = format!("{idgames_label} = {idgames_val}");
    match outliers_val {
        Some(o) => {
            let _ = write!(source, "; {outliers_label} = {o}");
            if o > idgames_val {
                let idgames_only = pow2_ceil(idgames_val.saturating_mul(multiplier));
                let _ = write!(
                    source,
                    "; idgames-only recommendation would be {}",
                    fmt(idgames_only)
                );
            }
        }
        None => source.push_str("; outliers: none analyzed"),
    }
    Recommendation {
        key: key.to_owned(),
        recommended: fmt(value),
        value: Some(value),
        formula: formula.to_owned(),
        source,
    }
}

/// The §8.1/§8.3 constant recommendations, one row per constant, in the
/// fixed order the report renders them (spec §6 table). Every row's
/// `formula`/`source` is non-empty by construction; `render_report` renders
/// this list verbatim — it never recomputes. `manifest_zip64_entries` is
/// [`crate::schema::ZipsManifest::zip64_entries`] (§B2): the
/// `zip64_statement` row cross-checks the record-derived count it would
/// otherwise report alone against this independently-tallied one, so a
/// divergence between the two can't go unnoticed. `scoped` is `true` when
/// the run is `--root`/`--limit` filtered (#442): the record population no
/// longer spans the manifest's full-run tally, so the cross-check is not
/// applicable and the row states the skip instead of a spurious disagreement.
#[must_use]
pub(crate) fn recommendations(
    idgames: &IdgamesStats,
    outliers: Option<&OutliersStats>,
    manifest_zip64_entries: u64,
    scoped: bool,
) -> Vec<Recommendation> {
    let wire_cap_zip = combined_row(
        "wire_cap_zip",
        "pow2_ceil(max(idgames p99.5 listing zip size, outliers max zip size))",
        "idgames p99.5 zip_size_listing",
        idgames.zip_size_listing.core.p99_5,
        "outliers max zip_size",
        outlier_value(outliers, |o| o.max_zip_size),
        1,
        fmt_bytes,
    );

    let wire_cap_wad = combined_row(
        "wire_cap_wad",
        "pow2_ceil(max(idgames p99.5 wad uncompressed, outliers max wad uncompressed))",
        "idgames p99.5 wad_uncompressed",
        idgames.wad_uncompressed.core.p99_5,
        "outliers max wad_uncompressed",
        outlier_value(outliers, |o| o.wad_uncompressed.max),
        1,
        fmt_bytes,
    );

    // §8.1: "identical regardless of path, because it bounds the same
    // downstream work" — decoded_cap is wire_cap_wad's number under a
    // different name, not an independently computed statistic.
    let decoded_cap = Recommendation {
        key: "decoded_cap".to_owned(),
        recommended: wire_cap_wad.recommended.clone(),
        value: wire_cap_wad.value,
        formula: "same as wire_cap_wad (bounds the same downstream work, §8.1)".to_owned(),
        source: format!("mirrors wire_cap_wad: {}", wire_cap_wad.source),
    };

    let max_member_count = combined_row(
        "max_member_count",
        "pow2_ceil(2 × max(idgames max member_count, outliers max member_count))",
        "idgames max member_count",
        idgames.entries.member_count.max,
        "outliers max member_count",
        outlier_value(outliers, |o| o.max_member_count),
        2,
        |v| v.to_string(),
    );

    let mut max_entry_uncompressed_bytes = combined_row(
        "max_entry_uncompressed_bytes",
        "pow2_ceil(2 × max(idgames max entry_wad_total_uncompressed, outliers max_entry_total_uncompressed))",
        "idgames max entry_wad_total_uncompressed",
        idgames.entries.entry_wad_total_uncompressed.max,
        "outliers max_entry_total_uncompressed",
        outlier_value(outliers, |o| o.max_entry_total_uncompressed),
        2,
        fmt_bytes,
    );
    // Binding review ruling: the envelope-total caveat (§6/§6.3 — phase 2
    // retains no non-wad member sizes) must appear in this row's source,
    // not just the report's method notes.
    max_entry_uncompressed_bytes
        .source
        .push_str(" (wad-member totals only — phase 2 retains no non-wad member sizes)");

    let max_member_compression_ratio = ratio_recommendation(idgames);
    let compression_method_allowlist = method_allowlist_recommendation(idgames);
    let zip64_statement = zip64_recommendation(idgames, manifest_zip64_entries, scoped);

    vec![
        wire_cap_zip,
        wire_cap_wad,
        decoded_cap,
        max_member_count,
        max_entry_uncompressed_bytes,
        max_member_compression_ratio,
        compression_method_allowlist,
        zip64_statement,
    ]
}

/// `max_member_compression_ratio` (§6.3/§8.3): `10 × ceil(2 × observed max
/// deflate ratio / 10)` — the next multiple of 10 above 2× the observed
/// maximum. When no `deflate` member was ever observed (`n == 0`, so there
/// is no maximum to double), the ratio recommendation falls back to a fixed
/// `40` — an arbitrary documented default, stated honestly as such in the
/// source rather than dressed up as a derived bound (there is no "classic
/// ~20:1 worst-case deflate ratio": DEFLATE's actual worst-case expansion
/// is ~1032:1, so `40` isn't 2× any real engineering constant — it's just a
/// round number with nothing to derive it from), and not silently computed
/// from an empty population either (which would otherwise yield a
/// nonsensical `0`).
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the ceil'd (2×max_ratio/10) quotient is a small positive multiplier (real deflate \
              ratios never approach u64::MAX/10); truncation/sign-loss are both unreachable here"
)]
fn ratio_recommendation(idgames: &IdgamesStats) -> Recommendation {
    let pop = &idgames.entries.ratios.member_deflate;
    let (value, source) = if pop.n == 0 {
        (
            40,
            "no deflate members observed — arbitrary documented default; no observation to \
             derive from (idgames population only)"
                .to_owned(),
        )
    } else {
        let multiple = ((2.0 * pop.max) / 10.0).ceil() as u64;
        (
            multiple * 10,
            format!(
                "idgames max member_deflate ratio = {:.2} (n = {}) (idgames population only)",
                pop.max, pop.n
            ),
        )
    };
    Recommendation {
        key: "max_member_compression_ratio".to_owned(),
        recommended: value.to_string(),
        value: Some(value),
        formula: "10 × ceil(2 × observed max deflate ratio / 10)".to_owned(),
        source,
    }
}

/// `compression_method_allowlist` (§8.3): always recommends `stored,
/// deflate` — the source lists every observed method with its count so the
/// assertion is checkable, and any method beyond the allowlist gets an
/// `OBSERVED UNEXPECTED METHOD` marker appended so the report can't bury it
/// in a long list.
fn method_allowlist_recommendation(idgames: &IdgamesStats) -> Recommendation {
    let methods = &idgames.entries.methods;
    let mut source = if methods.is_empty() {
        "no members observed".to_owned()
    } else {
        methods
            .iter()
            .map(|(method, count)| format!("{method} = {count}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    if methods.keys().any(|m| m != "stored" && m != "deflate") {
        source.push_str(" — OBSERVED UNEXPECTED METHOD");
    }
    // Review fix I2: these counts come from `wads[]` only — non-`.wad`
    // archive members reach Phase 2 as bare names with no method recorded
    // (§5.6 `other_members`) — so this is a `.wad`-members-only census, not
    // a whole-archive one, even though §8.3's allowlist governs every
    // member. Same treatment the envelope-totals caveat already gets on
    // `max_entry_uncompressed_bytes`.
    source.push_str(" (.wad members only — non-wad archive members carry no recorded method)");
    // I3(c): translate the raw method-id census into what adopting the
    // allowlist would actually cost — which members get rejected, and how
    // many.
    if let Some(cost) = allowlist_cost(methods) {
        let _ = write!(source, "; {cost}");
    }
    source.push_str(" (idgames population only)");
    Recommendation {
        key: "compression_method_allowlist".to_owned(),
        recommended: "stored, deflate".to_owned(),
        value: None,
        formula: "assertion: report every observed method with its count; recommend stored + \
                  deflate; flag any method beyond the allowlist"
            .to_owned(),
        source,
    }
}

/// §8.3 allowlist-adoption cost: how many `.wad` members observed in
/// `methods` (already computed by `stats::build_stats` — this only sums and
/// maps what's already there, per the module doc's "never recomputes"
/// caveat) would be rejected by adopting `stored + deflate` alone, broken
/// down by translated method name. `None` when every observed method is
/// already `stored`/`deflate` — there is nothing to cost out.
#[allow(
    clippy::cast_precision_loss,
    reason = "reporting a rejected-member share as f64 for the report; corpus member counts are \
              well within f64's exact integer range in practice, matching the module's existing \
              coverage-share precedent"
)]
fn allowlist_cost(methods: &BTreeMap<String, u64>) -> Option<String> {
    let rejected: Vec<(String, u64)> = methods
        .iter()
        .filter(|(m, _)| m.as_str() != "stored" && m.as_str() != "deflate")
        .map(|(m, &count)| (translate_method_label(m), count))
        .collect();
    if rejected.is_empty() {
        return None;
    }
    let total: u64 = methods.values().sum();
    let rejected_total: u64 = rejected.iter().map(|(_, count)| count).sum();
    let share = if total == 0 {
        0.0
    } else {
        (rejected_total as f64 / total as f64) * 100.0
    };
    let detail = rejected
        .iter()
        .map(|(name, count)| format!("{name}={count}"))
        .collect::<Vec<_>>()
        .join(", ");
    Some(format!(
        "adopting stored+deflate would reject {rejected_total} members ({share:.1}%): {detail}"
    ))
}

/// Translate one `methods` census label into a human APPNOTE method name
/// for the §8.3 cost line. `zips::inspect`'s method-labeling helper renders
/// an unsupported compression method id `N` as `"unsupported(N)"`
/// (lowercased `Debug` of `zip::CompressionMethod::Unsupported(N)`) — this
/// maps the three ids the real corpus has actually produced (`1` Shrink,
/// `6` Implode, `9` Deflate64) to `"Name(N)"`, any other id to the generic
/// `"method N"`, and passes through any other label (a future method-label
/// shape) unchanged.
fn translate_method_label(label: &str) -> String {
    let Some(id) = label
        .strip_prefix("unsupported(")
        .and_then(|s| s.strip_suffix(')'))
        .and_then(|s| s.parse::<u16>().ok())
    else {
        return label.to_owned();
    };
    match id {
        1 => format!("Shrink({id})"),
        6 => format!("Implode({id})"),
        9 => format!("Deflate64({id})"),
        _ => format!("method {id}"),
    }
}

/// `zip64_statement` (§5.3/§6.3/§9.3): a count, never a bare "handled" claim
/// — `0` states the absence explicitly rather than staying silent, per
/// §9.3's "≥1 resolved or absence stated" acceptance rule. `manifest_zip64`
/// is [`crate::schema::ZipsManifest::zip64_entries`] (§B2): an
/// independently-tallied count from the same phase-2 run, cross-checked
/// against the record-derived `n` here so the two can't silently diverge —
/// the source states agreement explicitly, or flags a disagreement loudly
/// rather than trusting the record-derived count alone.
fn zip64_recommendation(
    idgames: &IdgamesStats,
    manifest_zip64: u64,
    scoped: bool,
) -> Recommendation {
    let n = idgames.entries.zip64_entries;
    let agreement = if scoped {
        // #442: under `--root`/`--limit` the record population is
        // deliberately narrower than the manifest's full-run tally, so a
        // count mismatch is expected, not a finding — state the skip
        // instead of a spurious DISAGREES.
        "scoped run: manifest cross-check skipped (records are --root/--limit filtered)".to_owned()
    } else if n == manifest_zip64 {
        "manifest agrees".to_owned()
    } else {
        format!("MANIFEST DISAGREES: {manifest_zip64}")
    };
    Recommendation {
        key: "zip64_statement".to_owned(),
        recommended: "zip64 handled".to_owned(),
        value: None,
        formula: "count of records with zip64: true".to_owned(),
        source: format!(
            "{n} zip64 entries observed (records); {agreement} (idgames population only)"
        ),
    }
}

// ---------------------------------------------------------------------
// Markdown rendering. Every function below only *formats* fields already
// present on `StatsJson` — none of them compute a new *corpus* statistic
// (nothing here re-reads a raw record or re-derives a population). A
// couple of functions do compute a presentation-level value from fields
// already on `StatsJson`: `render_coverage`'s phase-2-coverage percentage
// (a ratio of two already-computed counts) and `allowlist_cost`'s
// rejected-member-share/method-cost sum (a sum and a ratio over the
// already-computed `methods` census). Both are arithmetic over numbers
// `stats::build_stats` already produced, not a new measurement of the
// corpus, so the "never recomputes" invariant still holds in the sense
// that matters: no function here can disagree with `stats::build_stats`
// about what the corpus contains.
// ---------------------------------------------------------------------

/// Full `data/stats-report.md` content (§6.5/§8/§9.3). Fixed section order:
/// title, provenance, method notes, §6.1 core distribution, §6.2
/// segmentations, §6.3 decision-driving counts, §6.4 outliers, §8
/// recommendations.
#[must_use]
pub(crate) fn render_report(stats: &StatsJson) -> String {
    let mut out = String::from("# idgames corpus statistics (phase 3)\n\n");
    out.push_str("## Provenance\n\n");
    render_provenance(&mut out, &stats.provenance, stats.schema_version);
    out.push_str("\n## Method notes\n\n");
    render_method_notes(&mut out, &stats.idgames);
    out.push('\n');
    render_core_section(&mut out, &stats.idgames);
    out.push('\n');
    render_segmentations(&mut out, &stats.idgames);
    out.push('\n');
    render_decision_counts(&mut out, &stats.idgames);
    out.push('\n');
    render_outliers(&mut out, stats.outliers.as_ref(), &stats.idgames);
    out.push('\n');
    render_recommendations(&mut out, &stats.recommendations);
    out
}

fn render_provenance(out: &mut String, p: &StatsProvenance, schema_version: u32) {
    out.push_str("| field | value |\n| --- | --- |\n");
    let _ = writeln!(out, "| schema_version | {schema_version} |");
    let _ = writeln!(out, "| phase1_manifest | {} |", p.phase1_manifest);
    let _ = writeln!(out, "| phase2_manifest | {} |", p.phase2_manifest);
    let _ = writeln!(
        out,
        "| outliers_manifest | {} |",
        p.outliers_manifest.as_deref().unwrap_or("none")
    );
    let _ = writeln!(out, "| bootstrap_mirror | {} |", p.bootstrap_mirror);
    let _ = writeln!(
        out,
        "| bootstrap_last_modified | {} |",
        p.bootstrap_last_modified.as_deref().unwrap_or("unknown")
    );
    let _ = writeln!(out, "| tool_version | {} |", p.tool_version);
    let _ = writeln!(
        out,
        "| git_rev | {} |",
        p.git_rev.as_deref().unwrap_or("unknown")
    );
}

fn render_method_notes(out: &mut String, idgames: &IdgamesStats) {
    out.push_str(
        "Percentiles use the nearest-rank method (§6.1): for percentile `p` in tenths-of-a-\
         percent (`p50` = 500 … `p99.9` = 999), rank `R = ceil(p10 * n / 1000)`, 1-indexed into \
         the ascending-sorted population; `min`/`max` are that population's first/last elements. \
         `stddev` is the **population** standard deviation (not sample), from exact integer sums.\n\n",
    );
    out.push_str(
        "`member_count` (§5.6) counts *distinct* central-directory entry names — a duplicate \
         member name under-counts by design (this is what the `zip` crate itself exposes, not \
         the raw on-disk central-directory record count). The caveat applies everywhere a member \
         count appears below, including the outliers section and the recommendations table.\n\n",
    );
    let _ = write!(
        out,
        "Zip-size statistics use the ls-laR mirror listing size, not the idgames API `size` \
         field (§6.3's locked decision): the listing is what a wire transfer actually carries. \
         {} WAD-bearing entries had no listing match and fell back to the API `size` \
         (`listing_misses`). The `api_delta` block below quantifies how far the API's `size` \
         would have misled a wire-cap decision, computed **among the entries where the listing \
         disagreed with the API** — not across the whole corpus.\n\n",
        idgames.coverage.listing_misses
    );
    out.push_str(
        "Envelope byte totals (`entry_wad_total_uncompressed` below, and the \
         `max_entry_uncompressed_bytes` recommendation) cover **`.wad` members only** — Phase 2 \
         retains no size data for non-wad archive members.\n\n",
    );
    out.push_str(
        "Compression-method counts (`### Compression methods observed` below, and the \
         `compression_method_allowlist` recommendation) are likewise a **`.wad`-members-only** \
         census, not a whole-archive one — non-`.wad` archive members reach Phase 2 as bare \
         names with no recorded method (§5.6 `other_members`). §8.3's allowlist governs every \
         member of an uploaded zip, so this count under-represents the true method population.\n",
    );
}

fn render_core_section(out: &mut String, idgames: &IdgamesStats) {
    out.push_str("## §6.1 Core distribution — WAD uncompressed size\n\n");
    push_distribution_table(out, &[("wad_uncompressed", &idgames.wad_uncompressed.core)]);
    out.push('\n');
    push_histogram_table(out, &idgames.wad_uncompressed.histogram);
}

fn render_segmentations(out: &mut String, idgames: &IdgamesStats) {
    let weighted = &idgames.wad_uncompressed.weighted;
    out.push_str("## §6.2 Segmentations\n\n### Vote-weighted vs. unweighted\n\n");
    push_distribution_table(
        out,
        &[
            ("unweighted", &idgames.wad_uncompressed.core),
            ("vote-weighted", &weighted.core),
        ],
    );
    let _ = write!(
        out,
        "\n`total_votes` = {} (summed once per vote-weighted `.wad` **member**, not an entry \
         count). `zero_vote_members_excluded` = {} (a count of `.wad` **members** whose parent \
         record had `votes == 0`, not a count of entries).\n\n",
        weighted.total_votes, weighted.zero_vote_members_excluded
    );
    out.push_str("### By top-level bucket\n\n");
    push_segmented_table(out, &idgames.wad_uncompressed.by_bucket);
    out.push_str("\n### By year\n\n");
    push_segmented_table(out, &idgames.wad_uncompressed.by_year);
}

#[allow(
    clippy::cast_precision_loss,
    reason = "reporting a share as f64 for the report; entry counts are well within f64's exact \
              integer range in practice, matching stats::mod's existing ratio_f64 precedent"
)]
fn render_decision_counts(out: &mut String, idgames: &IdgamesStats) {
    let entries = &idgames.entries;
    let coverage = &idgames.coverage;

    out.push_str("## §6.3 Decision-driving counts\n\n### Zip size (wire-cap population)\n\n");
    push_distribution_table(out, &[("zip_size_listing", &idgames.zip_size_listing.core)]);
    out.push('\n');
    push_histogram_table(out, &idgames.zip_size_listing.histogram);

    let delta = &idgames.zip_size_listing.api_delta;
    let size_mismatch_ledgered = coverage
        .ledger_kinds
        .get("size_mismatch")
        .copied()
        .unwrap_or(0);
    let _ = write!(
        out,
        "\n**API-vs-listing sanity check** (§6.3): {} entries compared, {} mismatched. Among \
         mismatches: max_abs_delta = {}, p50_abs_delta = {}, p99_abs_delta = {}, max_relative = \
         {:.4}.\n\n\
         Note: entries whose API size disagrees with the mirror listing are largely absent from \
         this population by construction — phase 2's Content-Range guard refuses them (they land \
         in `fetch_error`; see the Coverage section's `ledger_kinds` below). The phase-1 ledger \
         recorded {size_mismatch_ledgered} `size_mismatch` findings — the truer measure of \
         API-size unreliability than the `mismatched` count above.\n\n",
        delta.entries_compared,
        delta.mismatched,
        delta.max_abs_delta,
        delta.p50_abs_delta,
        delta.p99_abs_delta,
        delta.max_relative
    );

    out.push_str("### Entry-level counts\n\n");
    let _ = write!(
        out,
        "- zip_entries: {}\n\
         - zero_wad: {} ({:.2}% of entries)\n\
         - multi_wad: {} ({:.2}% of entries)\n\
         - zip64_entries: {}\n\
         - encrypted_members: {}\n\
         - wad_named_other_members (diagnostic only — never counted as a WAD): {}\n\n",
        entries.zip_entries,
        entries.zero_wad,
        entries.zero_wad_share * 100.0,
        entries.multi_wad,
        entries.multi_wad_share * 100.0,
        entries.zip64_entries,
        entries.encrypted_members,
        entries.wad_named_other_members
    );
    push_distribution_table(
        out,
        &[
            ("member_count (distinct CD names)", &entries.member_count),
            (
                "entry_wad_total_uncompressed (.wad members only)",
                &entries.entry_wad_total_uncompressed,
            ),
        ],
    );

    out.push_str("\n### Compression ratios\n\n");
    push_ratio_table(
        out,
        &[
            ("member_deflate", &entries.ratios.member_deflate),
            ("per_entry", &entries.ratios.per_entry),
        ],
    );
    let _ = write!(
        out,
        "\nzero_compressed_anomalies (excluded from both ratio populations above): {}\n\n",
        entries.ratios.zero_compressed_anomalies
    );

    out.push_str("### Compression methods observed\n\n");
    push_map_table(out, "method", "count", &entries.methods);

    out.push_str("\n### Coverage\n\n");
    render_coverage(out, coverage);
}

#[allow(
    clippy::cast_precision_loss,
    reason = "reporting a coverage share as f64 for the report; corpus entry counts are well \
              within f64's exact integer range in practice"
)]
fn render_coverage(out: &mut String, coverage: &Coverage) {
    let loaded: u64 = coverage.status_counts.values().sum();
    let share = if coverage.phase1_files == 0 {
        0.0
    } else {
        (loaded as f64 / coverage.phase1_files as f64) * 100.0
    };
    let _ = write!(
        out,
        "`phase1_files` (corpus denominator, from `harvest-manifest.json`): {}\n\n\
         loaded records (Σ status_counts): {} ({share:.1}% phase-2 coverage of phase1_files)\n\n",
        coverage.phase1_files, loaded
    );
    push_map_table(out, "status", "count", &coverage.status_counts);
    out.push('\n');
    push_map_table(out, "ledger_kind", "count", &coverage.ledger_kinds);
    let _ = write!(
        out,
        "\n`listing_misses`: {} (WAD-bearing population entries absent from the ls-laR listing; \
         fell back to the API `zip_size`)\n\n\
         `population_entries`: {}\n\n\
         `population_wads`: {}\n",
        coverage.listing_misses, coverage.population_entries, coverage.population_wads
    );
}

/// The exact phrase both the "outliers file absent" and "outliers file
/// present but empty" cases lead with — the
/// `missing_outliers_is_stated_not_silent` acceptance case (§9.3) greps for
/// this substring.
const NO_OUTLIERS_ANALYZED: &str = "No outliers analyzed";

fn render_outliers(out: &mut String, outliers: Option<&OutliersStats>, idgames: &IdgamesStats) {
    out.push_str("## §6.4 Modern-outliers supplement\n\n");
    let Some(outliers) = outliers else {
        out.push_str(NO_OUTLIERS_ANALYZED);
        out.push_str(" — data/outliers-wads.jsonl absent; run `just harvest-outliers`.\n");
        return;
    };

    // The statement must appear whenever `analyzed` is empty, independent
    // of `skipped` — review fix I1: a plausible live outcome is "every
    // curated host failed" (every entry lands in `skipped`, none in
    // `analyzed`), and the section that owns this fact must state it
    // itself rather than leaving only the recommendations table's
    // "outliers: none analyzed" phrase to imply it.
    if outliers.analyzed.is_empty() {
        out.push_str(NO_OUTLIERS_ANALYZED);
        if outliers.skipped.is_empty() {
            out.push_str(" — data/outliers-wads.jsonl is present but empty.\n");
            return;
        }
        out.push_str(" — every curated entry was skipped; see the table below.\n\n");
    } else {
        out.push_str("### Analyzed\n\n");
        out.push_str(
            "| slug | zip_size | member_count | wad_count | max_wad_uncompressed | \
             total_wad_uncompressed |\n| --- | --- | --- | --- | --- | --- |\n",
        );
        for a in &outliers.analyzed {
            let _ = writeln!(
                out,
                "| {} | {} | {} | {} | {} | {} |",
                a.slug,
                a.zip_size,
                a.member_count,
                a.wad_count,
                a.max_wad_uncompressed,
                a.total_wad_uncompressed
            );
        }
        out.push('\n');
        push_distribution_table(
            out,
            &[("outliers wad_uncompressed", &outliers.wad_uncompressed)],
        );
        let _ = write!(
            out,
            "\nmax_zip_size: {}\n\nmax_member_count (distinct CD names): {}\n\n\
             max_entry_total_uncompressed (.wad members only): {}\n\n",
            outliers.max_zip_size, outliers.max_member_count, outliers.max_entry_total_uncompressed
        );
    }

    if !outliers.skipped.is_empty() {
        out.push_str("### Skipped — hosts that did not permit analysis\n\n");
        out.push_str("| slug | fetch_status |\n| --- | --- |\n");
        for skip in &outliers.skipped {
            let _ = writeln!(out, "| {} | {} |", skip.slug, skip.fetch_status);
        }
        out.push('\n');
    }

    // I3(a): when the supplement has analyzed entries but none of them
    // actually exceeded the idgames-only p99.5 wad size, *and* at least one
    // curated outlier was refused outright (skipped), the size-tail anchors
    // this supplement exists to capture never landed — the §8
    // recommendations above rest on the idgames population alone despite
    // the supplement's presence. That must be stated here, not left for a
    // reader to infer by cross-referencing the recommendations table.
    if !outliers.analyzed.is_empty()
        && !outliers.skipped.is_empty()
        && outliers
            .analyzed
            .iter()
            .all(|a| a.max_wad_uncompressed <= idgames.wad_uncompressed.core.p99_5)
    {
        out.push_str(
            "The §6.4 size-tail anchors were all refused by their hosts this run — the upper \
             tail remains truncated by the idgames upload cap, and the recommendations above \
             rest on the idgames population alone.\n\n",
        );
    }
}

fn render_recommendations(out: &mut String, recommendations: &[Recommendation]) {
    out.push_str("## §8 Constant recommendations\n\n");
    out.push_str("| key | recommended | formula | source |\n| --- | --- | --- | --- |\n");
    for r in recommendations {
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} |",
            r.key, r.recommended, r.formula, r.source
        );
    }
}

fn push_distribution_table(out: &mut String, rows: &[(&str, &Distribution)]) {
    out.push_str(
        "| population | n | min | p50 | p75 | p90 | p95 | p99 | p99.5 | p99.9 | max | mean | \
         stddev |\n| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | \
         --- |\n",
    );
    for (name, d) in rows {
        let _ = writeln!(
            out,
            "| {name} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {:.2} | {:.2} |",
            d.n,
            d.min,
            d.p50,
            d.p75,
            d.p90,
            d.p95,
            d.p99,
            d.p99_5,
            d.p99_9,
            d.max,
            d.mean,
            d.stddev
        );
    }
}

fn push_segmented_table(out: &mut String, segments: &BTreeMap<String, Distribution>) {
    if segments.is_empty() {
        out.push_str("(no segments)\n");
        return;
    }
    let rows: Vec<(&str, &Distribution)> = segments.iter().map(|(k, v)| (k.as_str(), v)).collect();
    push_distribution_table(out, &rows);
}

fn push_ratio_table(out: &mut String, rows: &[(&str, &RatioDistribution)]) {
    out.push_str("| population | n | min | p50 | p90 | p99 | max |\n| --- | --- | --- | --- | --- | --- | --- |\n");
    for (name, d) in rows {
        let _ = writeln!(
            out,
            "| {name} | {} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} |",
            d.n, d.min, d.p50, d.p90, d.p99, d.max
        );
    }
}

fn push_histogram_table(out: &mut String, buckets: &[HistogramBucket]) {
    let max_count = buckets.iter().map(|b| b.count).max().unwrap_or(0);
    out.push_str("| bucket | count | bar |\n| --- | --- | --- |\n");
    for b in buckets {
        let _ = writeln!(
            out,
            "| {} | {} | {} |",
            b.label,
            b.count,
            "#".repeat(bar_len(b.count, max_count))
        );
    }
}

/// `#`-bar length for a histogram row: proportional to `count / max_count`,
/// scaled so the largest bucket renders exactly 40 characters. `0` when
/// `max_count` is `0` (an empty histogram — no bucket to scale against).
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "bar length is a small (<=40) rendering value derived from corpus counts well \
              within f64's exact integer range; the round-trip through f64 for proportional \
              scaling is the whole point of this helper"
)]
fn bar_len(count: u64, max_count: u64) -> usize {
    if max_count == 0 {
        return 0;
    }
    ((count as f64 / max_count as f64) * 40.0).round() as usize
}

fn push_map_table(
    out: &mut String,
    key_label: &str,
    value_label: &str,
    map: &BTreeMap<String, u64>,
) {
    let _ = writeln!(out, "| {key_label} | {value_label} |\n| --- | --- |");
    if map.is_empty() {
        out.push_str("| (none) | 0 |\n");
        return;
    }
    for (k, v) in map {
        let _ = writeln!(out, "| {k} | {v} |");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{
        ApiDelta, Coverage, EntryStats, OutlierSummary, RatioStats, SizeStats,
        WeightedDistribution, ZipSizeStats,
    };

    fn distribution(n: u64, min: u64, max: u64, p99_5: u64) -> Distribution {
        Distribution {
            n,
            min,
            p50: min,
            p75: min,
            p90: min,
            p95: min,
            p99: min,
            p99_5,
            p99_9: max,
            max,
            mean: 0.0,
            stddev: 0.0,
        }
    }

    /// A plausible, fully-populated `IdgamesStats` for recommendation/report
    /// tests: zip p99.5 = 40 MiB, wad p99.5 = 20 MiB (both round numbers so
    /// `pow2_ceil` boundaries are easy to hand-verify), one deflate/one
    /// stored method, zero zip64.
    fn idgames_fixture() -> IdgamesStats {
        let zip_p99_5 = 41_943_040; // 40 MiB
        let wad_p99_5 = 20_971_520; // 20 MiB
        IdgamesStats {
            coverage: Coverage {
                phase1_files: 100,
                status_counts: BTreeMap::from([
                    ("ok".to_owned(), 90),
                    ("fetch_error".to_owned(), 10),
                ]),
                ledger_kinds: BTreeMap::new(),
                listing_misses: 3,
                population_entries: 90,
                population_wads: 95,
            },
            wad_uncompressed: SizeStats {
                core: distribution(95, 100, 30_000_000, wad_p99_5),
                histogram: vec![
                    HistogramBucket {
                        label: "2^6-2^7".into(),
                        count: 1,
                    },
                    HistogramBucket {
                        label: "2^24-2^25".into(),
                        count: 20,
                    },
                ],
                weighted: WeightedDistribution {
                    core: distribution(92, 100, 30_000_000, wad_p99_5),
                    total_votes: 500,
                    zero_vote_members_excluded: 3,
                },
                by_bucket: BTreeMap::from([(
                    "levels/doom".to_owned(),
                    distribution(10, 100, 900, 900),
                )]),
                by_year: BTreeMap::from([("2019".to_owned(), distribution(10, 100, 900, 900))]),
            },
            zip_size_listing: ZipSizeStats {
                core: distribution(90, 1_000, 60_000_000, zip_p99_5),
                histogram: vec![HistogramBucket {
                    label: "2^20-2^21".into(),
                    count: 40,
                }],
                api_delta: ApiDelta {
                    entries_compared: 90,
                    mismatched: 5,
                    max_abs_delta: 10,
                    p50_abs_delta: 2,
                    p99_abs_delta: 9,
                    max_relative: 0.01,
                },
            },
            entries: EntryStats {
                zip_entries: 90,
                zero_wad: 2,
                zero_wad_share: 0.022,
                multi_wad: 5,
                multi_wad_share: 0.055,
                member_count: distribution(90, 1, 12, 10),
                entry_wad_total_uncompressed: distribution(90, 100, 5_000_000, 4_000_000),
                ratios: RatioStats {
                    member_deflate: RatioDistribution {
                        n: 80,
                        min: 1.1,
                        p50: 3.0,
                        p90: 6.0,
                        p99: 8.0,
                        max: 9.5,
                    },
                    per_entry: RatioDistribution {
                        n: 90,
                        min: 1.0,
                        p50: 2.5,
                        p90: 5.0,
                        p99: 7.0,
                        max: 8.0,
                    },
                    zero_compressed_anomalies: 1,
                },
                methods: BTreeMap::from([("deflate".to_owned(), 80), ("stored".to_owned(), 15)]),
                zip64_entries: 0,
                encrypted_members: 0,
                wad_named_other_members: 1,
            },
        }
    }

    fn outliers_fixture(zip_size: u64) -> OutliersStats {
        OutliersStats {
            analyzed: vec![OutlierSummary {
                slug: "big-one".into(),
                zip_size,
                member_count: 3,
                wad_count: 1,
                max_wad_uncompressed: 900_000_000,
                total_wad_uncompressed: 900_000_000,
            }],
            skipped: vec![crate::schema::OutlierSkip {
                slug: "refused-one".into(),
                fetch_status: "no_range_support".into(),
            }],
            wad_uncompressed: distribution(1, 900_000_000, 900_000_000, 900_000_000),
            max_zip_size: zip_size,
            max_member_count: 3,
            max_entry_total_uncompressed: 900_000_000,
        }
    }

    // ---- pow2_ceil ----

    #[test]
    fn pow2_ceil_boundaries() {
        assert_eq!(pow2_ceil(0), 1);
        assert_eq!(pow2_ceil(1), 1);
        assert_eq!(pow2_ceil(2), 2);
        assert_eq!(pow2_ceil(3), 4);
        assert_eq!(pow2_ceil(4096), 4096);
        assert_eq!(pow2_ceil(4097), 8192);
    }

    #[test]
    fn pow2_ceil_saturates_above_two_pow_62() {
        assert_eq!(pow2_ceil(1u64 << 62), 1u64 << 62); // exactly on the boundary: not saturated
        assert_eq!(pow2_ceil((1u64 << 62) + 1), u64::MAX);
        assert_eq!(pow2_ceil(u64::MAX), u64::MAX);
    }

    // ---- fmt_bytes ----

    #[test]
    fn fmt_bytes_boundaries_and_bare_numbers() {
        assert_eq!(fmt_bytes(2_147_483_648), "2147483648 (2 GiB)");
        assert_eq!(fmt_bytes(1024), "1024 (1 KiB)");
        assert_eq!(fmt_bytes(1_048_576), "1048576 (1 MiB)");
        assert_eq!(fmt_bytes(1_099_511_627_776), "1099511627776 (1 TiB)");
        assert_eq!(fmt_bytes(0), "0");
        assert_eq!(fmt_bytes(1023), "1023");
        assert_eq!(fmt_bytes(41_943_040), "41943040 (40 MiB)");
    }

    // ---- recommendations ----

    #[test]
    fn recommendations_cover_every_s8_constant() {
        let recs = recommendations(&idgames_fixture(), None, 0, false);
        let keys: Vec<&str> = recs.iter().map(|r| r.key.as_str()).collect();
        assert_eq!(
            keys,
            [
                "wire_cap_zip",
                "wire_cap_wad",
                "decoded_cap",
                "max_member_count",
                "max_entry_uncompressed_bytes",
                "max_member_compression_ratio",
                "compression_method_allowlist",
                "zip64_statement",
            ]
        );
        for r in &recs {
            assert!(!r.formula.is_empty() && !r.source.is_empty(), "{r:?}");
        }
    }

    #[test]
    fn outliers_absent_notes_none_analyzed_in_source() {
        let recs = recommendations(&idgames_fixture(), None, 0, false);
        for key in [
            "wire_cap_zip",
            "wire_cap_wad",
            "max_member_count",
            "max_entry_uncompressed_bytes",
        ] {
            let row = recs.iter().find(|r| r.key == key).unwrap();
            assert!(
                row.source.contains("outliers: none analyzed"),
                "{key}: {}",
                row.source
            );
        }
    }

    #[test]
    fn outliers_move_the_number_and_are_cited() {
        // idgames p99.5 zip = 40 MiB (41_943_040); outlier max zip = 1.5 GiB
        // (1_610_612_736) → pow2_ceil(1.5 GiB) = 2 GiB. The source cites both
        // populations, and the idgames-only recommendation (64 MiB) survives
        // in the source text even though outliers won.
        let idgames = idgames_fixture();
        let outliers = outliers_fixture(1_610_612_736);
        let recs = recommendations(&idgames, Some(&outliers), 0, false);
        let row = recs.iter().find(|r| r.key == "wire_cap_zip").unwrap();
        assert_eq!(row.value, Some(2_147_483_648));
        assert_eq!(row.recommended, "2147483648 (2 GiB)");
        assert!(row.source.contains("41943040"), "{}", row.source);
        assert!(row.source.contains("1610612736"), "{}", row.source);
        assert!(row.source.contains("idgames-only"), "{}", row.source);
        assert!(row.source.contains("67108864"), "{}", row.source); // pow2_ceil(40 MiB) = 64 MiB
    }

    #[test]
    fn outliers_present_but_smaller_than_idgames_does_not_claim_a_move() {
        // Outlier max zip below idgames p99.5 → idgames alone still wins;
        // both numbers are still cited, but no "idgames-only" clause (there
        // was nothing for outliers to move).
        let idgames = idgames_fixture();
        let outliers = outliers_fixture(1_000_000); // well under 40 MiB
        let recs = recommendations(&idgames, Some(&outliers), 0, false);
        let row = recs.iter().find(|r| r.key == "wire_cap_zip").unwrap();
        assert_eq!(row.value, Some(pow2_ceil(41_943_040)));
        assert!(row.source.contains("1000000"), "{}", row.source);
        assert!(!row.source.contains("idgames-only"), "{}", row.source);
    }

    #[test]
    fn decoded_cap_mirrors_wire_cap_wad_value() {
        let recs = recommendations(&idgames_fixture(), None, 0, false);
        let wire_cap_wad = recs.iter().find(|r| r.key == "wire_cap_wad").unwrap();
        let decoded_cap = recs.iter().find(|r| r.key == "decoded_cap").unwrap();
        assert_eq!(wire_cap_wad.value, decoded_cap.value);
        assert_eq!(wire_cap_wad.recommended, decoded_cap.recommended);
        assert_ne!(wire_cap_wad.formula, decoded_cap.formula);
    }

    #[test]
    fn max_member_count_is_not_byte_formatted() {
        let recs = recommendations(&idgames_fixture(), None, 0, false);
        let row = recs.iter().find(|r| r.key == "max_member_count").unwrap();
        // max = 12, pow2_ceil(2 * 12) = pow2_ceil(24) = 32 — plain number,
        // never an IEC byte label.
        assert_eq!(row.recommended, "32");
        assert_eq!(row.value, Some(32));
    }

    #[test]
    fn max_entry_uncompressed_bytes_source_states_wad_only_caveat() {
        let recs = recommendations(&idgames_fixture(), None, 0, false);
        let row = recs
            .iter()
            .find(|r| r.key == "max_entry_uncompressed_bytes")
            .unwrap();
        assert!(
            row.source.contains("non-wad member sizes"),
            "{}",
            row.source
        );
    }

    #[test]
    fn ratio_recommendation_falls_back_when_no_deflate_members_observed() {
        let mut idgames = idgames_fixture();
        idgames.entries.ratios.member_deflate = RatioDistribution {
            n: 0,
            min: 0.0,
            p50: 0.0,
            p90: 0.0,
            p99: 0.0,
            max: 0.0,
        };
        let recs = recommendations(&idgames, None, 0, false);
        let row = recs
            .iter()
            .find(|r| r.key == "max_member_compression_ratio")
            .unwrap();
        assert_eq!(row.recommended, "40");
        assert_eq!(row.value, Some(40));
        assert!(
            row.source.contains("no deflate members observed"),
            "{}",
            row.source
        );
    }

    #[test]
    fn ratio_recommendation_is_next_multiple_of_ten_above_double_observed() {
        let mut idgames = idgames_fixture();
        idgames.entries.ratios.member_deflate = RatioDistribution {
            n: 10,
            min: 1.0,
            p50: 5.0,
            p90: 10.0,
            p99: 17.0,
            max: 17.3,
        };
        let recs = recommendations(&idgames, None, 0, false);
        let row = recs
            .iter()
            .find(|r| r.key == "max_member_compression_ratio")
            .unwrap();
        // 2 * 17.3 = 34.6 -> ceil(34.6 / 10) = 4 -> 4 * 10 = 40.
        assert_eq!(row.value, Some(40));
    }

    #[test]
    fn compression_method_allowlist_flags_unexpected_method() {
        let mut idgames = idgames_fixture();
        idgames.entries.methods.insert("bzip2".to_owned(), 2);
        let recs = recommendations(&idgames, None, 0, false);
        let row = recs
            .iter()
            .find(|r| r.key == "compression_method_allowlist")
            .unwrap();
        assert_eq!(row.recommended, "stored, deflate");
        assert_eq!(row.value, None);
        assert!(
            row.source.contains("OBSERVED UNEXPECTED METHOD"),
            "{}",
            row.source
        );
        assert!(row.source.contains("bzip2 = 2"), "{}", row.source);
    }

    #[test]
    fn compression_method_allowlist_clean_when_only_stored_and_deflate() {
        let recs = recommendations(&idgames_fixture(), None, 0, false);
        let row = recs
            .iter()
            .find(|r| r.key == "compression_method_allowlist")
            .unwrap();
        assert!(!row.source.contains("UNEXPECTED"), "{}", row.source);
    }

    #[test]
    fn compression_method_allowlist_source_states_wad_only_caveat() {
        // Review fix I2: method counts come from `wads[]` only — non-`.wad`
        // archive members reach Phase 2 as bare names with no method
        // recorded, so this is a `.wad`-members-only census, not a
        // whole-archive one, even though §8.3's allowlist governs every
        // member. The row's source must say so (same treatment as
        // `max_entry_uncompressed_bytes`'s wad-only caveat).
        let recs = recommendations(&idgames_fixture(), None, 0, false);
        let row = recs
            .iter()
            .find(|r| r.key == "compression_method_allowlist")
            .unwrap();
        assert!(row.source.contains(".wad members only"), "{}", row.source);
    }

    #[test]
    fn compression_method_allowlist_translates_unsupported_method_ids_and_cost() {
        // I3(c): "unsupported(1)"/"unsupported(6)"/"unsupported(9)" are the
        // lowercased-Debug labels `zips::inspect::method_label` gives APPNOTE
        // method ids Shrink/Implode/Deflate64 — the allowlist row must
        // translate them into names and state what adopting stored+deflate
        // would actually cost, not just list the raw ids.
        let mut idgames = idgames_fixture(); // methods: deflate=80, stored=15 (95 total)
        idgames
            .entries
            .methods
            .insert("unsupported(1)".to_owned(), 4);
        idgames
            .entries
            .methods
            .insert("unsupported(6)".to_owned(), 215);
        idgames
            .entries
            .methods
            .insert("unsupported(9)".to_owned(), 1);
        let recs = recommendations(&idgames, None, 0, false);
        let row = recs
            .iter()
            .find(|r| r.key == "compression_method_allowlist")
            .unwrap();
        // 4 + 215 + 1 = 220 rejected members out of 95 + 220 = 315 total.
        assert!(
            row.source.contains("would reject 220 members"),
            "{}",
            row.source
        );
        assert!(row.source.contains("Shrink(1)=4"), "{}", row.source);
        assert!(row.source.contains("Implode(6)=215"), "{}", row.source);
        assert!(row.source.contains("Deflate64(9)=1"), "{}", row.source);
    }

    #[test]
    fn compression_method_allowlist_unknown_method_id_falls_back_to_generic_name() {
        let mut idgames = idgames_fixture();
        idgames
            .entries
            .methods
            .insert("unsupported(99)".to_owned(), 3);
        let recs = recommendations(&idgames, None, 0, false);
        let row = recs
            .iter()
            .find(|r| r.key == "compression_method_allowlist")
            .unwrap();
        assert!(row.source.contains("method 99=3"), "{}", row.source);
    }

    #[test]
    fn ratio_and_allowlist_and_zip64_sources_state_idgames_population_only() {
        // I3(b): rows 6-8 silently exclude the outliers population (unlike
        // the four `combined_row`-based rows above them) — each source must
        // say so explicitly.
        let recs = recommendations(&idgames_fixture(), None, 0, false);
        for key in [
            "max_member_compression_ratio",
            "compression_method_allowlist",
            "zip64_statement",
        ] {
            let row = recs.iter().find(|r| r.key == key).unwrap();
            assert!(
                row.source.contains("(idgames population only)"),
                "{key}: {}",
                row.source
            );
        }
    }

    #[test]
    fn zip64_statement_states_absence_explicitly() {
        // manifest_zip64_entries = 0 agrees with idgames_fixture()'s
        // zip64_entries = 0.
        let recs = recommendations(&idgames_fixture(), None, 0, false);
        let row = recs.iter().find(|r| r.key == "zip64_statement").unwrap();
        assert_eq!(
            row.source,
            "0 zip64 entries observed (records); manifest agrees (idgames population only)"
        );
        assert_eq!(row.recommended, "zip64 handled");
    }

    #[test]
    fn zip64_statement_counts_when_present() {
        let mut idgames = idgames_fixture();
        idgames.entries.zip64_entries = 4;
        let recs = recommendations(&idgames, None, 4, false); // manifest agrees
        let row = recs.iter().find(|r| r.key == "zip64_statement").unwrap();
        assert_eq!(
            row.source,
            "4 zip64 entries observed (records); manifest agrees (idgames population only)"
        );
    }

    #[test]
    fn zip64_statement_flags_manifest_disagreement() {
        // B2: the record-derived zip64 count must be cross-checked against
        // wads-manifest.json's independently-tallied `zip64_entries` — a
        // disagreement between the two must render visibly, not be
        // silently trusted away in favor of the record-derived count.
        let mut idgames = idgames_fixture();
        idgames.entries.zip64_entries = 4;
        let recs = recommendations(&idgames, None, 7, false); // manifest disagrees: 7 != 4
        let row = recs.iter().find(|r| r.key == "zip64_statement").unwrap();
        assert!(
            row.source.contains("4 zip64 entries observed"),
            "{}",
            row.source
        );
        assert!(
            row.source.contains("MANIFEST DISAGREES: 7"),
            "{}",
            row.source
        );
    }

    #[test]
    fn zip64_cross_check_states_agreement_disagreement_and_scoped_skip() {
        // `idgames_fixture()` is the module's existing builder
        // (report.rs:866). Read it first: if its `entries.zip64_entries`
        // isn't 2, adjust this test's literals to whatever it carries —
        // the three cases only need equal / unequal / scoped-unequal.
        let mut idgames = idgames_fixture();
        idgames.entries.zip64_entries = 2;
        // Unscoped + equal: agreement, verbatim current wording.
        let r = zip64_recommendation(&idgames, 2, false);
        assert!(r.source.contains("manifest agrees"), "{}", r.source);
        // Unscoped + unequal: the loud flag, verbatim current wording.
        let r = zip64_recommendation(&idgames, 5, false);
        assert!(r.source.contains("MANIFEST DISAGREES: 5"), "{}", r.source);
        // Scoped: the cross-check is not applicable — never DISAGREES,
        // even though the counts differ (#442: a --root/--limit-filtered
        // record population is expected to diverge from the manifest's
        // full-run tally).
        let r = zip64_recommendation(&idgames, 5, true);
        assert!(
            r.source
                .contains("scoped run: manifest cross-check skipped"),
            "{}",
            r.source
        );
        assert!(!r.source.contains("DISAGREES"), "{}", r.source);
    }

    // ---- render_report ----

    fn stats_fixture(outliers: Option<OutliersStats>) -> StatsJson {
        let idgames = idgames_fixture();
        let recs = recommendations(&idgames, outliers.as_ref(), 0, false);
        StatsJson {
            schema_version: crate::schema::STATS_SCHEMA_VERSION,
            provenance: StatsProvenance {
                phase1_manifest: "harvest-1".into(),
                phase2_manifest: "harvest-zips-1".into(),
                outliers_manifest: outliers.as_ref().map(|_| "harvest-outliers-1".to_owned()),
                bootstrap_mirror: "infania".into(),
                bootstrap_last_modified: Some("Wed, 12 Aug 2026 06:00:00 GMT".into()),
                tool_version: "0.0.0".into(),
                git_rev: Some("abc1234".into()),
            },
            idgames,
            outliers,
            recommendations: recs,
        }
    }

    #[test]
    fn render_report_title_and_section_order() {
        let stats = stats_fixture(None);
        let report = render_report(&stats);
        assert!(report.starts_with("# idgames corpus statistics (phase 3)\n"));
        let headings = [
            "## Provenance",
            "## Method notes",
            "## §6.1",
            "## §6.2",
            "## §6.3",
            "## §6.4",
            "## §8",
        ];
        let positions: Vec<usize> = headings
            .iter()
            .map(|h| {
                report
                    .find(h)
                    .unwrap_or_else(|| panic!("missing section {h}"))
            })
            .collect();
        assert!(
            positions.windows(2).all(|w| w[0] < w[1]),
            "sections out of order: {positions:?}"
        );
        assert!(report.contains(NO_OUTLIERS_ANALYZED));
        assert!(report.contains("wire_cap_zip"));
    }

    #[test]
    fn render_report_method_notes_state_wad_only_method_census_caveat() {
        // Review fix I2: the method notes must carry the same caveat as the
        // `compression_method_allowlist` recommendation row.
        let report = render_report(&stats_fixture(None));
        assert!(
            report.contains("Compression-method counts") && report.contains(".wad`-members-only"),
            "{report}"
        );
    }

    #[test]
    fn render_report_api_delta_explains_structural_zero_via_ledger_when_absent() {
        // Review fix (Task 8 live-smoke finding): `mismatched` reads 0 not
        // because the API size is accurate, but because phase 2's
        // Content-Range guard already excluded every disagreeing entry
        // upstream (they land in `fetch_error`, ledgered as
        // `size_mismatch`). `idgames_fixture()`'s `ledger_kinds` is empty —
        // the sentence must still render, deterministically, with `0`.
        let report = render_report(&stats_fixture(None));
        assert!(
            report.contains("recorded 0 `size_mismatch` findings"),
            "{report}"
        );
        assert!(
            report.contains("Content-Range guard refuses them"),
            "{report}"
        );
    }

    #[test]
    fn render_report_api_delta_cites_size_mismatch_count_when_present() {
        // Real-harvest shape (Task 8 live smoke): 1099 `size_mismatch`
        // ledger findings alongside a structurally-zero `mismatched` count
        // in the same report.
        let mut stats = stats_fixture(None);
        stats
            .idgames
            .coverage
            .ledger_kinds
            .insert("size_mismatch".to_owned(), 1099);
        let report = render_report(&stats);
        assert!(
            report.contains("recorded 1099 `size_mismatch` findings"),
            "{report}"
        );
    }

    #[test]
    fn render_report_never_recomputes_it_only_formats() {
        // Mutate `zip64_entries` *after* `recommendations` was already
        // computed from the original value (see `stats_fixture`) — the
        // §6.3 decision-counts bullet reads the mutated field directly and
        // must reflect it, even though it now disagrees with the
        // recommendations table's `zip64_statement` row (still citing the
        // stale value). That divergence is exactly the proof: the renderer
        // formats whatever is in the struct, field by field, and performs
        // no independent computation of its own.
        let mut stats = stats_fixture(None);
        stats.idgames.entries.zip64_entries = 999;
        let report = render_report(&stats);
        assert!(report.contains("zip64_entries: 999"), "{report}");
    }

    #[test]
    fn render_report_shows_outliers_analyzed_and_skipped() {
        let stats = stats_fixture(Some(outliers_fixture(500_000_000)));
        let report = render_report(&stats);
        assert!(report.contains("big-one"));
        assert!(report.contains("refused-one"));
        assert!(report.contains("no_range_support"));
        assert!(!report.contains(NO_OUTLIERS_ANALYZED));
        // outliers_fixture()'s analyzed max_wad_uncompressed (900_000_000)
        // exceeds idgames_fixture()'s wad p99.5 (20 MiB) — the size-tail
        // anchor DID land, so the I3(a) truncation sentence must not render.
        assert!(
            !report.contains("size-tail anchors were all refused"),
            "{report}"
        );
    }

    #[test]
    fn render_report_states_size_tail_truncated_when_analyzed_never_exceeds_idgames_p99_5() {
        // I3(a): analyzed entries exist (so the supplement isn't silent),
        // at least one curated entry was refused by its host (skipped is
        // non-empty), and none of what *did* get analyzed exceeded the
        // idgames-only p99.5 wad size (20 MiB in idgames_fixture()) — the
        // size-tail this supplement exists to capture never actually
        // landed, and the report must say so.
        let outliers = OutliersStats {
            analyzed: vec![OutlierSummary {
                slug: "small-one".into(),
                zip_size: 1_000_000,
                member_count: 1,
                wad_count: 1,
                max_wad_uncompressed: 1_000_000, // well under 20 MiB
                total_wad_uncompressed: 1_000_000,
            }],
            skipped: vec![crate::schema::OutlierSkip {
                slug: "refused-one".into(),
                fetch_status: "no_range_support".into(),
            }],
            wad_uncompressed: distribution(1, 1_000_000, 1_000_000, 1_000_000),
            max_zip_size: 1_000_000,
            max_member_count: 1,
            max_entry_total_uncompressed: 1_000_000,
        };
        let stats = stats_fixture(Some(outliers));
        let report = render_report(&stats);
        assert!(
            report.contains("The §6.4 size-tail anchors were all refused by their hosts this run"),
            "{report}"
        );
        assert!(
            report.contains("recommendations above rest on the idgames population alone"),
            "{report}"
        );
    }

    #[test]
    fn render_report_present_but_empty_outliers_states_it() {
        let empty = OutliersStats {
            analyzed: vec![],
            skipped: vec![],
            wad_uncompressed: distribution(0, 0, 0, 0),
            max_zip_size: 0,
            max_member_count: 0,
            max_entry_total_uncompressed: 0,
        };
        let stats = stats_fixture(Some(empty));
        let report = render_report(&stats);
        assert!(report.contains(NO_OUTLIERS_ANALYZED));
        assert!(report.contains("present but empty"));
    }

    #[test]
    fn render_report_all_skipped_outliers_states_none_analyzed_and_shows_skip_table() {
        // Review fix I1: every curated host failed (a plausible live
        // outcome) — `analyzed` is empty but `skipped` is not. The section
        // must state "No outliers analyzed" itself (not just imply it via
        // the recommendations table) *and* still render the skip table.
        let all_skipped = OutliersStats {
            analyzed: vec![],
            skipped: vec![
                crate::schema::OutlierSkip {
                    slug: "refused-one".into(),
                    fetch_status: "no_range_support".into(),
                },
                crate::schema::OutlierSkip {
                    slug: "refused-two".into(),
                    fetch_status: "fetch_error".into(),
                },
            ],
            wad_uncompressed: distribution(0, 0, 0, 0),
            max_zip_size: 0,
            max_member_count: 0,
            max_entry_total_uncompressed: 0,
        };
        let stats = stats_fixture(Some(all_skipped));
        let report = render_report(&stats);
        assert!(report.contains(NO_OUTLIERS_ANALYZED), "{report}");
        assert!(report.contains("refused-one"), "{report}");
        assert!(report.contains("refused-two"), "{report}");
        assert!(report.contains("no_range_support"), "{report}");
        assert!(report.contains("fetch_error"), "{report}");
        assert!(report.contains("### Skipped"), "{report}");
        assert!(!report.contains("### Analyzed"), "{report}");
    }
}
