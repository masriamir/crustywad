# CLAUDE.md

## Project overview

`crustywad` is a Rust workspace providing safe, documented Doom WAD file I/O. It targets the Rust 2024 edition with MSRV 1.85.0 and is dual-licensed under MIT OR Apache-2.0.

**Current state:** safe WAD header and lump-directory reading, typed scaffolding for classic Doom map-record lumps, WAD serialization (`write` feature), zero-copy memory-mapped loading (`mmap` feature), a `cwad` CLI with `info`/`list`/`validate`/`merge`/`diff`/`extract`/`build` subcommands, `cargo-fuzz` targets, and Criterion benchmarking infrastructure.

## Workspace layout

```
crates/
  crustywad/           # Core library crate
    src/
      lib.rs           # Public API — Wad, WadKind, WadHeader, Lump, ParseOptions, Strictness
      error.rs         # ParseError (thiserror) and ParseWarning types
      map/             # Map-record structs, organized by format
        mod.rs         #   parse_records<T>, MapParseError, shared re-exports
        assemble.rs    #   Map::assemble/assemble_with_options, MapAssembleError
        common.rs      #   records identical across formats (Vertex, Sidedef, Sector, Seg, Subsector, Node, Name8)
        doom.rs        #   Doom/Heretic layout (Thing, Linedef)
        graph.rs       #   assembled Map graph — MapVertex/MapLinedef/MapSidedef/MapSector/MapThing, index newtypes, MapWarning
        group.rs       #   MapGroup — identifying one map's lumps (Wad::map_groups/map_group)
      mmap.rs          # Read-only memmap2-backed file loading (feature = "mmap")
      util.rs          # Crate-internal helpers (trim_nul: NUL-padding trim for 8-byte names)
    benches/
      helpers.rs       # Synthetic WAD builder + Freedoom loader for bench use
      read_ops.rs      # Criterion benchmarks for parsing, lump access, and map records
      write_ops.rs     # Criterion benchmarks for write/build/round-trip (feature = "write")
    tests/
      common/mod.rs    # Shared WAD-building helpers (build_wad, lump_map)
      wad_reader.rs    # Integration tests for the main WAD reader API
      map_records.rs   # Integration tests for typed map-record parsing
      freedoom.rs      # Optional Freedoom fixture tests (feature = "freedoom-tests")
  crustywad-cli/       # CLI binary crate (`cwad`)
    src/main.rs        # `info`/`list`/`validate`/`merge`/`diff`/`extract`/`build` subcommands via clap
    src/cli.rs         # CLI argument types (also included by build.rs for shell completions)
docs/
  design.md            # Goals, data model, read pipeline, feature plan
  adr/                 # Architecture decision records
  diagrams/            # Standalone Mermaid diagram files (included into the guide via {{#include}})
  guide/               # mdBook user guide — source of truth for user-facing docs, deployed to GitHub Pages
    book.toml          # mdBook + mdbook-mermaid configuration
    src/               # Guide source pages (SUMMARY.md, *.md)
scripts/
  check_doc_anchors.py # Living-docs anchor drift detector (ADR-0007); run via `just docs-sync`
anchors.txt            # Anchor strings that must appear verbatim in all three main doc files
tests/
  fixtures/
    fetch_freedoom.py  # Downloads Freedoom WAD fixtures from GitHub releases
    README.md          # Fixture documentation and version configuration
tools/
  Cargo.toml           # Pinned versions of mdbook and mdbook-mermaid; Dependabot watches this
  src/lib.rs           # Empty; makes the package a valid Cargo package for Dependabot resolution
.github/
  codeql/codeql-config.yml   # Advanced CodeQL query config (security-extended + quality)
  workflows/ci.yml            # Main CI pipeline
  workflows/codeql.yml        # CodeQL security analysis
  workflows/release-plz.yml  # Automated release PR workflow
  workflows/bench.yml         # Criterion benchmark trend reporting to GitHub Pages
  workflows/fuzz.yml          # cargo-fuzz targets
  workflows/pages.yml         # mdBook guide deployment to GitHub Pages
  workflows/release.yml # dist: cross-platform cwad binaries + installers (CLI releases)
```

## Development workflow

Install [just](https://github.com/casey/just), then:

| Recipe | Command |
|---|---|
| Build | `just build` |
| Test | `just test` |
| Lint (fmt + clippy) | `just lint` |
| Auto-format | `just fmt` |
| Docs | `just doc` |
| Guide (mdBook) | `just guide` (requires `mdbook` + `mdbook-mermaid`) |
| Coverage | `just cov` (requires `cargo-llvm-cov`) |
| Dependency audit | `just deny` (requires `cargo-deny`) |
| Fetch Freedoom fixtures | `just fetch-fixtures` |
| Anchor drift check | `just docs-sync` |
| Benchmarks | `just bench` (criterion; HTML report in `target/criterion/`) |
| Benchmarks + open report | `just bench-open` |
| Full CI check | `just ci` |

**Always run `just ci` before pushing.** It runs the same checks as GitHub Actions (build, test, clippy, fmt, doc, deny, docs-sync) and catches failures locally before they reach CI.

Core Rust commands run by CI:

```bash
cargo build --workspace --all-features
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all --check
cargo doc --workspace --all-features --no-deps
```

Freedoom fixture tests require both the feature flag and the env var:

```bash
just fetch-fixtures                         # default version
just fetch-fixtures version=v0.14.0        # specific version
CRUSTYWAD_FREEDOOM_DIR=tests/fixtures/freedoom cargo test --workspace --all-features
```

## Code conventions

### Error handling

- All errors in the library crate use `thiserror`-derived enums: `ParseError` and `MapParseError`.
- `ParseWarning` collects non-fatal issues in lenient mode; it derives `thiserror::Error` and implements `Display` with human-readable messages.
- `anyhow` is permitted only in `crustywad-cli`.
- Every public fallible function must have a `# Errors` doc section.

### Documentation

- All documentation uses American English spelling (e.g. "artifacts" not "artefacts", "customization" not "customisation").
- `missing_docs = "deny"` is enforced workspace-wide — every public item must have a doc comment.
- Use `//!` for module-level docs, `///` for item-level docs.
- Include `# Errors` in doc comments for fallible functions, `# Panics` where relevant.
- Crate-level docs live in `lib.rs` as `#![doc = r#"..."]` (inner attribute — note the `!`).

### Safety

- `#![deny(unsafe_code)]` is set in the core library crate. Unsafe code is permitted only in `mmap.rs` (behind `#![allow(unsafe_code)]`) and only to call `memmap2::MmapOptions::map`.
- All parsing and validation code is free of unsafe.

### Lints

- `clippy::all` and `clippy::pedantic` are enabled workspace-wide. All warnings are errors in CI.
- New code must compile with zero warnings locally before pushing.

### Naming

- Doom WAD concepts follow the unofficial spec: `Lump`, `WadKind`, `WadHeader`, `ParseOptions`.
- Map-record types use singular Rust names matching the lump: `Thing`, `Linedef`, `Sidedef`, `Vertex`, `Seg`, `Subsector`, `Node`, `Sector`.
- `snake_case` for items, `UPPER_SNAKE_CASE` for constants.

### Strictness model

- `ParseOptions { strictness: Strictness::Strict | Strictness::Lenient }`.
- Strict mode returns the first `ParseError` encountered.
- Lenient mode attempts best-effort recovery and collects `ParseWarning` values.
- Every new validation must honour both modes.


## Testing practices

### Unit vs integration tests

- **Unit tests** (private helpers only): `#[cfg(test)] mod tests {}` inside the source file.
- **Integration tests** (all public API): `crates/crustywad/tests/`, one file per concern.
- `common/mod.rs` contains shared WAD-building helpers; add shared test utilities there.

### Freedoom fixture tests

- Live in `tests/freedoom.rs`, gated by `#[cfg(feature = "freedoom-tests")]`.
- Fixtures are gitignored — never commit WAD blobs.
- Tests skip gracefully when `CRUSTYWAD_FREEDOOM_DIR` is not set.

### Property-based tests

- Use `proptest` for parser invariants; place in the same file as the regular tests they complement.
- `wad_reader.rs` has an existing proptest for empty-WAD parsing.

## Writing or updating an ADR

Before opening a PR for a new or updated ADR, run through this checklist. The items target the most common review-round causes: claims that diverge from the actual codebase, field descriptions written from memory, and incomplete API surface.

1. **Verify every claim about existing code.** For each statement about a type, function, error variant, or behavior, open the relevant file and confirm it. Common failure modes: calling an implemented function a "stub", citing a validation that only applies to the write path in a read-path description, describing a lenient-mode output as a strict-mode rejection.
2. **Anchor on-disk field descriptions to Rust types.** Derive prose from the struct definition (`[u8; 8]`, `i32 LE`), not from informal memory. Avoid wording like "up to 8 characters" when the field is `[u8; 8]`; use "8 bytes".
3. **Keep read-path and write-path concerns separate.** "Oversized lump name" is a write-side concern (the field is always exactly 8 bytes on disk). Don't let write-side validation examples leak into descriptions of the read-side `ParseOptions` behavior.
4. **Specify complete API signatures for any new type that surfaces values.** If the ADR introduces a warning type, define how callers observe those warnings on success (e.g., `Result<(T, Vec<Warning>), Error>`). An API item mentioned without a return type is underspecified.
5. **Cross-check Context vs Decision.** Re-read both sections back-to-back. Remove any alternative or option from Context that the Decision has already rejected — surviving alternatives mislead implementers.
6. **Verify "new" types and functions don't already exist.** Grep the codebase for any type or function the ADR calls "new" before declaring it new. Extending an existing type (`RawDirectoryEntry` adding `BinWrite`) is different from introducing one.

## Adding a new lump type

1. Add a `binrw`-derived struct to the appropriate `crates/crustywad/src/map/` module — `doom.rs` for a Doom/Heretic-format record, or `common.rs` for a record whose on-disk byte layout is identical across formats — with full doc comments on every field.
2. Ensure the struct uses `#[br(little)]` and implements `BinRead` with `Args<'a> = ()`.
3. Check the Doom WAD spec for correct field types (signed vs unsigned matters — e.g., `Thing.angle` is `u16`, coords are `i16`).
4. Add integration tests in `crates/crustywad/tests/map_records.rs` with a hand-crafted byte sequence and at least one field assertion per field.
5. Add a proptest if the type has meaningful invariants.
6. Update `README.md` if the new type is user-visible.
7. Run `just lint` and `just doc` before committing.

### Parser/assembly hardening checklist (ADR-0016)

Every PR that adds a parse or assembly surface — a new lump type, a new map
format, or a new decode/assembly path — must satisfy **and state in its PR
description** all of:

1. **Bounded allocation.** Memory use is `O(input length)` (record/element counts
   bounded by `input_len / min_record_size`). For structured *text* formats (UDMF),
   allocation is instead explicitly depth-/count-limited via `Limits` (introduced
   with UDMF, #57–#58).
2. **No unbounded recursion.** The path is iterative, or recursive with an explicit
   depth counter that fails cleanly against `Limits::max_depth` rather than risking
   stack overflow.
3. **A `cargo-fuzz` target** exists for the surface, with the no-panic oracle, an
   output-size (`O(input)`) assertion per item 1, and a committed seed corpus — and
   is wired into `.github/workflows/fuzz.yml`.
4. **Both `Strictness` modes** reject or recover from malformed input without
   panicking.

The threat model is denial of service (unexpected panic/abort, OOM, unbounded work,
stack overflow), not memory safety — the core crate is `#![deny(unsafe_code)]`. See
ADR-0016 for the full rationale.

## Feature flags

See [`docs/guide/src/features.md`](../docs/guide/src/features.md) for the full feature flag reference including usage examples, platform notes, and common `cargo` invocations. That file is the single source of truth — it is published via the mdBook guide to GitHub Pages.

**Sync rule:** when a feature flag is added, removed, or renamed, update **all three** of:
1. `docs/guide/src/features.md` — detailed docs, usage examples, and the summary table (primary)
2. The summary table below in this file
3. The summary table in `.github/copilot-instructions.md`

| Feature | Default | Purpose |
|---|---|---|
| `mmap` | no | Enables `Wad::from_path_mapped[_with_options]` for zero-copy memory-mapped loading via `memmap2`; `from_path` always reads into memory regardless of this flag |
| `freedoom-tests` | no | Enables optional integration tests against local Freedoom fixture WADs |
| `write` | no | Enables `WadBuilder`, `WriteError`, `WriteOptions`, `WriteWarning`, and `Wad::to_builder()` for WAD serialization |

## Commit conventions

Follow [Conventional Commits](https://www.conventionalcommits.org/):

| Prefix | When |
|---|---|
| `feat:` | new functionality |
| `fix:` | bug fixes |
| `docs:` | documentation only |
| `test:` | test-only changes |
| `refactor:` | no behavior change |
| `chore:` | build, tooling |
| `ci:` | CI workflow changes |

Scope is encouraged: `feat(map):`, `fix(cli):`, etc.

The `lefthook.yml` pre-commit hook runs `cargo fmt` and `cargo clippy`, and validates commit messages against the Conventional Commits pattern.

## Project tracking

Work is tracked on the **[Crustywad](https://github.com/users/masriamir/projects/5)** GitHub Project board — the single source of "what to pick up next". Every roadmap issue lives there with three planning dimensions:

- **Status** (workflow stage): `Backlog` → `Ready` → `In progress` → `In review` → `Done`. Pull work from the `Ready` column. Most transitions are **agent-driven** — see [Issue status transitions](#issue-status-transitions-agent-driven) below; only `Done` is set automatically by the board on merge/close.
- **Horizon** (priority bucket): `Now` / `Next` / `Later`. This carries planning intent for issues that have no milestone yet (e.g. the editor epic #18 and its long-horizon spikes); it replaces the former `Short Term` / `Future` milestones.
- **Milestone** (release scope): milestones are **release-scoped** (`v0.2.0`, `v0.3.0`, …), each the set of epics/issues intended to ship together. `release-plz` derives the actual version tags from Conventional Commits on `main`; the milestone only groups scope.

**Epics** (the `epic` label, e.g. #17, #18) use GitHub **native sub-issues**, so they show automatic progress rollup — attach each new format/feature issue as a sub-issue of its epic.

Typical flow: pick a `Ready` + `Now` item → begin planning (`In progress`) → branch by its issue number (below) → open a PR (`In review`) → merge closes the issue and sets `Done`.

### Issue status transitions (agent-driven)

I move the board myself as work progresses and **announce each change** in my reply (e.g. "moved #201 → In progress") rather than asking first — board edits are internal and easily reversed. The transitions:

| Transition | Trigger |
|---|---|
| `Backlog → Ready` | The user says they want to start work on an issue |
| `Ready → In progress` | I begin brainstorming or drafting an implementation plan for the issue — **before** any branch or code |
| `In progress → In review` | The PR opens |
| `In review → Done` | PR merges/closes — **board-automated**, not manual |

`In review` holds through the entire Copilot review loop, until human review and merge. Transitions apply only to a tracked issue that is on the board; if an issue exists but isn't on the board, add it first. The `gh project item-edit` recipe (Status field + option IDs) lives in the `reference-project-board` memory; it needs the `read:project,project` scope — if that scope is missing, surface it and ask the user to grant it rather than silently skipping the transition.

## Git branching workflow

All work branches from `main` after a `git pull`. A branch is named `<type>/<slug>`; the slug is descriptive (never a bare issue number) and is prefixed with the issue number when a tracking issue exists.

| Branch type | Template | Example |
|---|---|---|
| Feature | `feature/###-short-desc` (or `feature/short-desc`) | `feature/42-mmap-support` |
| Bugfix | `bugfix/###-short-desc` (or `bugfix/short-desc`) | `bugfix/17-header-parse` |
| Hotfix | `hotfix/###-short-desc` (or `hotfix/short-desc`) | `hotfix/55-oob-read` |
| Docs | `docs/###-short-desc` (or `docs/short-desc`) | `docs/197-project-workflow` |
| Chore | `chore/###-short-desc` (or `chore/short-desc`) | `chore/tidy-ci` |

`###` is the GitHub issue number. It is optional in the pre-push hook but strongly encouraged for `feature`/`bugfix`/`hotfix` branches, which are issue-driven; `docs`/`chore` branches commonly omit it. A descriptive slug is always required — a bare number such as `feature/42` is rejected.

**Release branches are not used** — `release-plz` automates version bumps, CHANGELOG, and git tags (`vX.Y.Z`) from Conventional Commits on `main`. When publishing is enabled, merge the `release-plz` release PR to ship.

The `lefthook.yml` pre-push hook enforces branch naming and will reject pushes from non-conforming branches.

**Workflow:**
1. `git pull origin main`
2. `git checkout -b feature/###-description`
3. Commit with Conventional Commits (`feat(scope): ...`)
4. Run `just ci` before pushing
5. Open a PR against `main`

## Copilot review-comment workflow

PRs are reviewed automatically by `copilot-pull-request-reviewer`. Use the personal
`resolving-bot-pr-reviews` skill (`~/.claude/skills/resolving-bot-pr-reviews/`) to work
through its comments until two consecutive clean checks, then hand off for human review.

Project facts the skill needs:
- Bot login: `copilot-pull-request-reviewer`
- CI command: `just ci` (see Development workflow above)
- Owner/repo: parsed from the `git remote get-url origin` URL
- Timing: skill defaults (60s poll / 5min dwell / 15min stall) apply — no override needed

## CI pipeline

The CI (`.github/workflows/ci.yml`) runs on every push to `main` and all PRs:

| Job | What it checks |
|---|---|
| `fmt` | `cargo fmt --all --check` |
| `clippy` | `cargo clippy --workspace --all-targets --all-features -- -D warnings` |
| `test` | `cargo test --workspace --all-features` on ubuntu, macos, windows (no fixtures fetched; freedoom-tests are skipped unless `CRUSTYWAD_FREEDOOM_DIR` is set) |
| `msrv` | build + test on Rust 1.85.0 |
| `docs` | `cargo doc` with `RUSTDOCFLAGS=-D warnings` |
| `coverage` | `cargo llvm-cov` + Codecov upload |
| `security-deny` | `cargo deny check` |
| `docs-sync` | `python3 scripts/check_doc_anchors.py` — verifies anchor strings are present in all three main doc files |

CodeQL (`.github/workflows/codeql.yml`) runs on push, PR, and weekly. It uses `security-extended` and `security-and-quality` query suites.

`bench` (`.github/workflows/bench.yml`) runs on push to `main` and `workflow_dispatch`. It is **non-blocking** (never gates merges). On each run it uploads a downloadable Criterion HTML artifact (90-day retention) and commits benchmark trend data to the `gh-pages` branch at `dev/bench/`. The `pages.yml` guide deploy and `bench.yml` share the `gh-pages` concurrency group so they never write to the branch simultaneously.

`release-plz` (`.github/workflows/release-plz.yml`) runs two jobs on push to `main`, authenticated as a **GitHub App** (`RELEASE_PLZ_APP_ID` / `RELEASE_PLZ_APP_PRIVATE_KEY`) so its PRs and tags trigger downstream workflows. `release-pr` opens/updates the release PR; `release` publishes to crates.io via Trusted Publishing (OIDC — no stored `CARGO_REGISTRY_TOKEN`) and pushes tags. It does **not** create GitHub Releases (`git_release_enable = false`). Binary releases of the `cwad` CLI are handled by **dist** (`.github/workflows/release.yml`), which triggers on the `crustywad-cli-v*` tag and creates the GitHub Release with cross-platform binaries and installers. The library crate is excluded from dist (`[package.metadata.dist] dist = false`).

**Version bump:** `crates/crustywad-cli/Cargo.toml` pins the `crustywad` path dependency with an explicit version (`crustywad = { path = "../crustywad", version = "X.Y.Z" }`), which Cargo treats as a caret requirement — patch/compatible bumps to `crustywad`'s version need no change here. `cargo-deny` requires this field (`wildcards = "deny"`) but it does not inherit `crustywad`'s version automatically. Crates are versioned independently (ADR-0011 §3, no `version.workspace = true`); update this field only when `crustywad`'s version moves outside the current caret range (e.g. `0.1.z` → `0.2.0`), or `cargo deny check` will fail.

## Roadmap

1. ✅ Directory reading and map-record scaffolding (this PR)
2. 🔜 Map lump parsing (full graph assembly)
3. Graphics
4. Textures
5. Audio
6. ✅ Write support
