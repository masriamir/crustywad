#!/usr/bin/env bash
#
# create-issues.sh — bulk-create the crustywad planning backlog.
#
# This mirrors docs/issue-backlog.md. It creates:
#   * missing labels and milestones
#   * every sub-issue under the existing epics #12-#18
#   * a new "Benchmarking & performance" epic, re-parenting existing #2-#6
#   * parent/child sub-issue links
#   * best-effort "blocked by" dependencies (spike -> implementation siblings)
#
# Requirements: GitHub CLI (`gh`) authenticated with repo write access.
#
# Usage:
#   ./scripts/create-issues.sh            # create everything
#   DRY_RUN=1 ./scripts/create-issues.sh  # print what would be created
#
# WARNING: running this more than once creates DUPLICATES. It is intended to
# be run exactly once. Do NOT also approve the chat-staged issue drafts, or
# you will get duplicates.

set -euo pipefail

REPO="${REPO:-masriamir/crustywad}"
ASSIGNEE="${ASSIGNEE:-masriamir}"
DRY_RUN="${DRY_RUN:-0}"

declare -A NUM   # slug -> created issue number

# --------------------------------------------------------------------------
# Helpers
# --------------------------------------------------------------------------
require() { command -v "$1" >/dev/null 2>&1 || { echo "error: '$1' not found" >&2; exit 1; }; }

run() {
  if [[ "$DRY_RUN" == "1" ]]; then echo "DRY: $*"; else "$@"; fi
}

create_label() {
  local name="$1" color="$2" desc="$3"
  if [[ "$DRY_RUN" == "1" ]]; then echo "DRY: label $name"; return 0; fi
  gh label create "$name" --repo "$REPO" --color "$color" --description "$desc" --force >/dev/null
  echo "label: $name"
}

create_milestone() {
  local title="$1"
  if [[ "$DRY_RUN" == "1" ]]; then echo "DRY: milestone $title"; return 0; fi
  # Idempotent: ignore "already_exists" errors.
  gh api --method POST "repos/$REPO/milestones" -f title="$title" >/dev/null 2>&1 \
    && echo "milestone: $title" \
    || echo "milestone: $title (exists)"
}

# create_issue <slug> <parent#|BENCH> <labels> <milestone> <title> <body>
create_issue() {
  local slug="$1" parent="$2" labels="$3" milestone="$4" title="$5" body="$6"
  [[ "$parent" == "BENCH" ]] && parent="${NUM[b-epic]}"

  if [[ "$DRY_RUN" == "1" ]]; then
    echo "DRY: issue '$title' [$labels] (ms:$milestone) -> parent #$parent"
    NUM[$slug]=0
    return 0
  fi

  local url num
  url=$(gh issue create --repo "$REPO" \
        --title "$title" \
        --body "$(printf '%b' "$body")" \
        --label "$labels" \
        --assignee "$ASSIGNEE" \
        --milestone "$milestone")
  num=$(printf '%s\n' "$url" | tail -n1 | sed -n 's#.*/issues/##p')
  NUM[$slug]=$num
  echo "created #$num: $title"
  link_sub "$parent" "$num"
}

link_sub() {
  local parent="$1" child="$2" cid
  [[ "$DRY_RUN" == "1" ]] && { echo "DRY: link #$child under #$parent"; return 0; }
  cid=$(gh api "repos/$REPO/issues/$child" --jq '.id' 2>/dev/null) \
    || { echo "  WARN: cannot fetch id for #$child"; return 0; }
  if gh api --method POST "repos/$REPO/issues/$parent/sub_issues" -F sub_issue_id="$cid" >/dev/null 2>&1; then
    echo "  #$child -> sub-issue of #$parent"
  else
    echo "  WARN: could not link #$child under #$parent (add manually in the UI)"
  fi
}

resolve() { local k="$1"; [[ "$k" =~ ^[0-9]+$ ]] && echo "$k" || echo "${NUM[$k]:-}"; }

add_blocked_by() {
  local issue blocker iid bid
  issue=$(resolve "$1"); blocker=$(resolve "$2")
  [[ -z "$issue" || -z "$blocker" ]] && { echo "  WARN: unresolved dep $1 <- $2"; return 0; }
  [[ "$DRY_RUN" == "1" ]] && { echo "DRY: #$issue blocked-by #$blocker"; return 0; }
  bid=$(gh api "repos/$REPO/issues/$blocker" --jq '.id' 2>/dev/null) || return 0
  if gh api --method POST "repos/$REPO/issues/$issue/dependencies/blocked_by" -F issue_id="$bid" >/dev/null 2>&1; then
    echo "  #$issue blocked by #$blocker"
  else
    echo "  WARN: dependency #$issue<-#$blocker not set (dependencies API may be unavailable; note is in the body)"
  fi
}

# --------------------------------------------------------------------------
# Preflight
# --------------------------------------------------------------------------
require gh
echo "Repo: $REPO   Assignee: @$ASSIGNEE   DRY_RUN=$DRY_RUN"
if [[ "$DRY_RUN" != "1" ]]; then
  read -r -p "This will create ~50 issues. Continue? [y/N] " ans
  [[ "$ans" == "y" || "$ans" == "Y" ]] || { echo "aborted"; exit 1; }
fi

# --------------------------------------------------------------------------
# 1. Labels
# --------------------------------------------------------------------------
create_label epic           6f42c1 "Tracking epic"
create_label spike          d4c5f9 "Time-boxed research producing an ADR"
create_label cli            1d76db "cwad CLI work"
create_label testing        0e8a16 "Tests, coverage, fuzzing"
create_label release        fbca04 "Packaging, CI/CD, releases"
create_label format-support c2e0c6 "New WAD/map formats"
create_label performance    d93f0b "Benchmarking and optimization"
create_label future         bfdadc "Long-horizon / aspirational"

# --------------------------------------------------------------------------
# 2. Milestones
# --------------------------------------------------------------------------
create_milestone "v1.0"
create_milestone "Short Term"
create_milestone "Future"

# --------------------------------------------------------------------------
# 3. New Benchmarking & performance epic (created first so children can attach)
# --------------------------------------------------------------------------
if [[ "$DRY_RUN" == "1" ]]; then
  echo "DRY: epic 'Benchmarking & performance'"; NUM[b-epic]=0
else
  url=$(gh issue create --repo "$REPO" \
    --title "[EPIC] Benchmarking & performance (Short Term)" \
    --label "epic,performance,enhancement" \
    --assignee "$ASSIGNEE" \
    --milestone "Short Term" \
    --body "$(printf '%b' "Groups benchmarking work and the performance-optimization goal from todo.md (Short Term #3).\n\nAdopted sub-issues: #2, #3, #4, #5, #6.\nRefs: crates/crustywad/src/lib.rs (parse_bytes L414-510, lump_by_name L360-364), justfile (bench L30-31).")")
  NUM[b-epic]=$(printf '%s\n' "$url" | tail -n1 | sed -n 's#.*/issues/##p')
  echo "created epic #${NUM[b-epic]}: Benchmarking & performance"
fi

# --------------------------------------------------------------------------
# 4. Sub-issues  (slug | parent | labels | milestone | title | body)
# --------------------------------------------------------------------------

# ---- Epic #12: write support ----
create_issue w-spike     12 "spike,enhancement"      "v1.0" \
  "Spike: WAD write design & ADR" \
  "Design in-place modification vs. full rebuild, offset/directory recomputation, and a buffer/writer abstraction.\n\n**Deliverable:** accepted ADR under docs/adr/. **Blocks all other write sub-issues.**\n\nRefs: crates/crustywad/src/lib.rs (WadData L141-156, into_bytes L405-411, parse_bytes L414-510); docs/design.md (milestone 6)."
create_issue w-hdr       12 "enhancement"            "v1.0" \
  "Write: header + lump-directory serialization" \
  "Serialize WadHeader and the 16-byte directory entries.\nBlocked by the write design spike.\n\nRefs: crates/crustywad/src/lib.rs (RawHeader L186-192, RawDirectoryEntry L194-200)."
create_issue w-payload   12 "enhancement"            "v1.0" \
  "Write: lump payload serialization + offset/size recompute" \
  "Write lump payloads and recompute filepos/size + infotableofs.\nBlocked by the write design spike.\n\nRefs: crates/crustywad/src/lib.rs (Lump L113-139, lump_bytes/lump_data L366-391)."
create_issue w-validate  12 "enhancement"            "v1.0" \
  "Write: strict/lenient validation on write" \
  "Honour the Strictness model when writing.\nBlocked by the write design spike.\n\nRefs: crates/crustywad/src/lib.rs (Strictness L57-64, validate_entry L554-622)."
create_issue w-roundtrip 12 "testing,enhancement"    "v1.0" \
  "Write: round-trip read->write->read property tests" \
  "Property-based round-trip invariants.\nBlocked by the write design spike.\n\nRefs: crates/crustywad/tests/wad_reader.rs."
create_issue w-cli       12 "cli,enhancement"        "v1.0" \
  "Write: CLI write wiring for cwad" \
  "Expose writing through cwad.\nBlocked by the write design spike.\n\nRefs: crates/crustywad-cli/src/main.rs (L22-70)."
create_issue w-tests     12 "testing,enhancement"    "v1.0" \
  "Write: exhaustive test coverage for write paths" \
  "Edge cases: empty WADs, huge lumps, IWAD vs PWAD, overflow.\nBlocked by the write design spike."
create_issue w-bench     12 "performance,enhancement" "v1.0" \
  "Write: unblock & link write-benchmark stubs (#5, #6)" \
  "Once write support lands, unblock #5/#6.\nBlocked by the write design spike."

# ---- Epic #13: documentation ----
create_issue d-api      13 "documentation"      "v1.0" \
  "Docs: API reference coverage for lib.rs" \
  "Round out rustdoc across the public surface (missing_docs already denied).\n\nRefs: crates/crustywad/src/lib.rs; Cargo.toml (L23-24)."
create_issue d-cli      13 "documentation,cli"  "v1.0" \
  "Docs: CLI usage documentation + man page" \
  "Document subcommands, options, examples; generate a man page.\n\nRefs: crates/crustywad-cli/src/main.rs."
create_issue d-diagrams 13 "documentation"      "v1.0" \
  "Docs: architecture & data-flow diagrams" \
  "Add mermaid diagrams of the read/write pipeline.\n\nRefs: docs/design.md."
create_issue d-guide    13 "documentation"      "v1.0" \
  "Docs: user guide & examples via GitHub Pages" \
  "Publish guides/examples to GitHub Pages."
create_issue d-spike    13 "spike,documentation" "v1.0" \
  "Spike: living-docs automation & ADR" \
  "Keep .github/copilot-instructions.md, CLAUDE.md, and docs/ in sync automatically.\n\n**Deliverable:** ADR. **Blocks the living-docs task.**\n\nRefs: .github/copilot-instructions.md."
create_issue d-living   13 "documentation"      "v1.0" \
  "Docs: keep docs/ current as living documents" \
  "Implements the living-docs spike outcome.\nBlocked by the living-docs spike."

# ---- Epic #14: CLI ----
create_issue c-spike    14 "spike,cli" "v1.0" \
  "Spike: CLI UX & architecture & ADR" \
  "Subcommand structure, arg parsing, output formats, packaging implications.\n\n**Deliverable:** ADR. **Blocks all cwad commands below.**\n\nRefs: crates/crustywad-cli/src/main.rs."
create_issue c-validate 14 "cli"       "v1.0" \
  "CLI: cwad validate" "Check WAD correctness. Blocked by the CLI UX spike."
create_issue c-info     14 "cli"       "v1.0" \
  "CLI: cwad info (expand)" "Display richer WAD metadata. Blocked by the CLI UX spike.\n\nRefs: crates/crustywad-cli/src/main.rs (Command::Info L24-28, L44-52)."
create_issue c-diff     14 "cli"       "v1.0" \
  "CLI: cwad diff" "Compare two WAD files. Blocked by the CLI UX spike."
create_issue c-merge    14 "cli"       "v1.0" \
  "CLI: cwad merge" "Combine multiple WADs. Blocked by the CLI UX spike."
create_issue c-extract  14 "cli"       "v1.0" \
  "CLI: cwad extract" "Extract specific lumps. Blocked by the CLI UX spike.\n\nRefs: crates/crustywad/src/lib.rs (lump_bytes L366-371)."
create_issue c-hardening 14 "cli,testing" "v1.0" \
  "CLI: hardening test suite & invalid-file regressions" \
  "Comprehensive CLI tests + edge cases. Blocked by the CLI UX spike."

# ---- Epic #15: testing ----
create_issue t-cov        15 "testing"       "v1.0" \
  "Testing: raise coverage to >=90%" \
  "Gate with cargo-llvm-cov.\n\nRefs: justfile (cov L17-18)."
create_issue t-corpus     15 "testing"       "v1.0" \
  "Testing: malformed & large WAD corpus" \
  "Grow synthetic + real fixtures.\n\nRefs: crates/crustywad/tests/common/mod.rs."
create_issue t-e2e        15 "testing"       "v1.0" \
  "Testing: end-to-end read->modify->write tests" \
  "Full workflow coverage. **Blocked by Epic #12 (write support).**"
create_issue t-fuzz-spike 15 "spike,testing" "v1.0" \
  "Spike: cargo-fuzz harness & ADR" \
  "Design a fuzzing harness.\n\n**Deliverable:** ADR. **Blocks the fuzz implementation.**\n\nRefs: justfile (fuzz L27-28)."
create_issue t-fuzz       15 "testing"       "v1.0" \
  "Testing: implement cargo-fuzz targets" \
  "Implements the fuzz spike. Blocked by the fuzz spike."
create_issue t-prop-spike 15 "spike,testing" "v1.0" \
  "Spike: proptest strategy & ADR" \
  "Define parser-invariant properties.\n\n**Deliverable:** ADR. **Blocks the proptest implementation.**"
create_issue t-prop       15 "testing"       "v1.0" \
  "Testing: implement proptest invariant tests" \
  "Implements the proptest spike. Blocked by the proptest spike.\n\nRefs: crates/crustywad/src/map.rs (parse_records L196-225)."

# ---- Epic #16: release ----
create_issue r-spike     16 "spike,release"        "v1.0" \
  "Spike: publish workflow & ADR" \
  "Workspace publish ordering, changelog generation, version strategy.\n\n**Deliverable:** ADR. **Blocks all release sub-issues.**\n\nRefs: .github/workflows/release-plz.yml; Cargo.toml (L1-29)."
create_issue r-docsrs    16 "release,documentation" "v1.0" \
  "Release: docs.rs configuration" \
  "All features, cross-crate linking, badges. Blocked by the publish spike."
create_issue r-changelog 16 "release"             "v1.0" \
  "Release: changelog automation" \
  "Automate changelog generation. Blocked by the publish spike."
create_issue r-policy    16 "release,documentation" "v1.0" \
  "Release: SemVer, MSRV & release policy doc" \
  "Document versioning policy. Blocked by the publish spike.\n\nRefs: Cargo.toml (rust-version L8)."
create_issue r-artifacts 16 "release"             "v1.0" \
  "Release: cross-platform binary artifacts" \
  "Build/publish cwad binaries. Blocked by the publish spike.\n\nRefs: .github/workflows/ci.yml."

# ---- Epic #17: formats ----
create_issue f-spike        17 "spike,format-support" "Short Term" \
  "Spike: format-landscape review & ADR" \
  "Collect specs, review compat, propose an extension strategy.\n\n**Deliverable:** ADR. **Blocks Doom64/Hexen/Heretic.**\n\nRefs: crates/crustywad/src/lib.rs (WadKind L46-55)."
create_issue f-doom64       17 "format-support" "Short Term" \
  "Formats: Doom 64 support" "Blocked by the format-landscape spike."
create_issue f-hexen        17 "format-support" "Short Term" \
  "Formats: Hexen support" "Blocked by the format-landscape spike.\n\nRefs: crates/crustywad/src/map.rs (Thing L33-46; Hexen extends THINGS)."
create_issue f-heretic      17 "format-support" "Short Term" \
  "Formats: Heretic support" "Blocked by the format-landscape spike."
create_issue f-udmf-spike   17 "spike,format-support" "Short Term" \
  "Spike: UDMF design/representation & ADR" \
  "Model UDMF and its relationship to classic formats.\n\n**Deliverable:** ADR. **Blocks UDMF read/convert/write.**"
create_issue f-udmf-read    17 "format-support" "Short Term" \
  "Formats: UDMF read support" "Blocked by the UDMF spike."
create_issue f-udmf-convert 17 "format-support" "Short Term" \
  "Formats: UDMF <-> classic WAD conversion" "Blocked by the UDMF spike."
create_issue f-udmf-write   17 "format-support" "Short Term" \
  "Formats: UDMF write support" "Blocked by the UDMF spike and **Epic #12 (write support)**."

# ---- Epic #18: future GUI/editor ----
create_issue g-gui-spike    18 "spike,future" "Future" \
  "Spike: GUI framework evaluation & ADR" \
  "Evaluate egui, Tauri, Qt6, cross-platform options.\n\n**Deliverable:** ADR. **Blocks viewer/manager/preview.**"
create_issue g-render-spike 18 "spike,future" "Future" \
  "Spike: map-renderer design & ADR" \
  "2D/3D/interactive rendering design.\n\n**Deliverable:** ADR. **Blocks the visual map viewer.**"
create_issue g-editor-spike 18 "spike,future" "Future" \
  "Spike: editor architecture + versioned WADs & ADR" \
  "UDB-in-Rust architecture + git-like WAD version control.\n\n**Deliverable:** ADR. **Blocks resource manager & live preview.**"
create_issue g-viewer       18 "future" "Future" \
  "Future: visual map viewer core" \
  "Blocked by the GUI-framework and map-renderer spikes.\n\nRefs: crates/crustywad/src/map.rs."
create_issue g-manager      18 "future" "Future" \
  "Future: lump/resource manager" \
  "Blocked by the GUI-framework and editor-architecture spikes."
create_issue g-preview      18 "future" "Future" \
  "Future: live preview / test integration" \
  "Blocked by the GUI-framework and editor-architecture spikes."

# ---- Benchmarking epic: new optimization sub-issue ----
create_issue b-optimize  BENCH "performance,enhancement" "Short Term" \
  "Perf: implement enhancements identified by benchmarking" \
  "Optimize parsing algorithm and memory usage once benchmarks identify hot spots.\n\nRefs: crates/crustywad/src/lib.rs (parse_bytes L414-510); justfile (bench L30-31)."

# --------------------------------------------------------------------------
# 5. Re-parent existing benchmark issues under the new epic
# --------------------------------------------------------------------------
for n in 2 3 4 5 6; do link_sub "${NUM[b-epic]}" "$n"; done

# --------------------------------------------------------------------------
# 6. Blocked-by dependencies (best effort; bodies also note these in prose)
# --------------------------------------------------------------------------
# Write epic: spike blocks siblings
for s in w-hdr w-payload w-validate w-roundtrip w-cli w-tests w-bench; do add_blocked_by "$s" w-spike; done
# Docs
add_blocked_by d-living d-spike
# CLI: spike blocks commands
for s in c-validate c-info c-diff c-merge c-extract c-hardening; do add_blocked_by "$s" c-spike; done
# Testing
add_blocked_by t-fuzz t-fuzz-spike
add_blocked_by t-prop t-prop-spike
add_blocked_by t-e2e 12
# Release
for s in r-docsrs r-changelog r-policy r-artifacts; do add_blocked_by "$s" r-spike; done
# Formats
for s in f-doom64 f-hexen f-heretic; do add_blocked_by "$s" f-spike; done
for s in f-udmf-read f-udmf-convert f-udmf-write; do add_blocked_by "$s" f-udmf-spike; done
add_blocked_by f-udmf-write 12
# Future
add_blocked_by g-viewer  g-gui-spike
add_blocked_by g-viewer  g-render-spike
add_blocked_by g-manager g-gui-spike
add_blocked_by g-manager g-editor-spike
add_blocked_by g-preview g-gui-spike
add_blocked_by g-preview g-editor-spike
# Benchmarking / write
add_blocked_by 5 12
add_blocked_by 6 12
add_blocked_by 6 5

echo "Done. Review: https://github.com/$REPO/issues"
