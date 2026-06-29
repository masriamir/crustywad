# v1.0 EPIC Parallel Implementation Design

**Date:** 2026-06-29  
**Scope:** All actionable v1.0 EPICs (#12, #13, #14, #15, #16)  
**Strategy:** Audit-then-Wave (Option A)

---

## Overview

24 open v1.0 issues across 5 EPICs. All prerequisite spikes (ADR-0006 through ADR-0011) are complete. Implementation proceeds in three stages:

1. **Audit** — one read-only agent reviews all 20 v1.0 issues against current code and updates GitHub
2. **Wave 1** — 13 parallel agents, one per independently-actionable issue
3. **Wave 2** — 8 agents for write-dependent work, launched after `#21` merges into `main`

Each agent works in its own git worktree on a `feature/###-description` branch and opens a PR.

---

## Stage 1: Audit

**Agent count:** 1 (read-only, no code changes)

**Issues to audit (20):**

| Epic | Issues |
|---|---|
| #12 Write support | #21, #22, #23, #24, #25, #26 |
| #13 Documentation | #29, #33 |
| #14 CLI expansion | #35, #36, #37, #38, #39, #40 |
| #15 Testing | #43, #45, #47 |
| #16 Release engineering | #49, #50, #51, #52 |

**Audit checklist per issue:**
- Verify stale line number references against current `src/` files
- Remove "blocked by spike" language where the spike is now closed
- Check if work is already partially or fully done by recent commits (close with comment if so)
- Verify acceptance criteria align with the relevant ADR

**Output:** Updated GitHub issue bodies. Resolved issues closed. No PRs, no code edits.

---

## Stage 2: Wave 1 (13 Parallel Agents)

Launches immediately after audit completes. Wave 1 completion gate: all agents run independently; Wave 2 triggers once **#21 merges into `main`**.

### Write core

| Issue | Branch | Scope |
|---|---|---|
| #21 | `feature/21-write-header-directory` | Serialize `WadHeader` + 16-byte directory entries using `binrw` `BinWrite`; scaffold `WadBuilder` per ADR-0006 |

### CLI commands (read-only)

| Issue | Branch | Scope |
|---|---|---|
| #35 | `feature/35-cwad-validate` | `cwad validate` — WAD correctness check |
| #36 | `feature/36-cwad-info-expand` | Expand `cwad info` with lump count, map names, IWAD/PWAD |
| #37 | `feature/37-cwad-diff` | `cwad diff` — lump-by-lump comparison of two WADs |
| #39 | `feature/39-cwad-extract` | `cwad extract` — extract lumps to disk |

### Testing

| Issue | Branch | Scope |
|---|---|---|
| #45 | `feature/45-cargo-fuzz` | Fuzz targets per ADR-0009 |
| #47 | `feature/47-proptest` | Proptest invariant tests per ADR-0010 |

### Documentation

| Issue | Branch | Scope |
|---|---|---|
| #29 | `feature/29-cli-docs-man-page` | CLI usage docs + man page via `clap` |
| #33 | `feature/33-living-docs` | Living-docs automation per ADR-0007 |

### Release engineering

| Issue | Branch | Scope |
|---|---|---|
| #49 | `feature/49-docsrs-config` | docs.rs configuration — all features, cross-crate linking, badges |
| #50 | `feature/50-changelog-automation` | Verify/complete `release-plz` changelog setup |
| #51 | `feature/51-semver-msrv-policy` | SemVer, MSRV & release policy documentation |
| #52 | `feature/52-binary-artifacts` | Cross-platform binary artifact CI workflow |

---

## Stage 3: Wave 2 (8 Agents)

Launches after #21 merges. The write chain (#22→#23) is sequential; #24, #25, #26 and #38 run in parallel once #23 merges.

### Write chain (sequential)

| Issue | Branch | Prerequisite | Scope |
|---|---|---|---|
| #22 | `feature/22-write-lump-payload` | #21 merged | Lump payload serialization; recompute `filepos`/`size`/`infotableofs` |
| #23 | `feature/23-write-validation` | #22 merged | Strict/lenient validation on write, honouring `ParseOptions` |

### After #23 merges (parallel)

| Issue | Branch | Scope |
|---|---|---|
| #24 | `feature/24-write-roundtrip-tests` | Round-trip `read→write→read` proptest invariants |
| #25 | `feature/25-write-cli-wiring` | Wire write through `cwad` CLI |
| #26 | `feature/26-write-exhaustive-tests` | Exhaustive write test coverage |
| #38 | `feature/38-cwad-merge` | `cwad merge` — combine multiple WADs |
| #43 | `feature/43-e2e-tests` | End-to-end `read→modify→write` integration tests |

### After all CLI commands merge (#35–#39 from Wave 1 + #38 from Wave 2)

| Issue | Branch | Prerequisite | Scope |
|---|---|---|---|
| #40 | `feature/40-cli-hardening` | #35–#39 and #38 merged | Comprehensive CLI test suite + invalid-file regressions across all subcommands |

---

## Constraints

- All agents run `just ci` before pushing (matches CI exactly)
- Commit messages follow Conventional Commits (`feat(scope):`, `fix(scope):`, etc.)
- No co-author lines in commits
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` must pass before any PR opens
- `missing_docs = "deny"` — every new public item needs a doc comment with `# Errors` where applicable
- American English spelling throughout

---

## Issue Dependencies (summary)

```
Audit
  └─> Wave 1 (all parallel)
        #21 ──merges──> Wave 2 starts
                          #22 ──merges──> #23 ──merges──> #24, #25, #26, #38, #43 (parallel)
        #35–#39 (Wave 1) + #38 (Wave 2) ──all merge──> #40
```
