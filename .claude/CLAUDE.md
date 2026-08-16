# CLAUDE.md

## Project overview

`crustywad` is a Rust workspace providing safe, documented Doom WAD file I/O. It targets the Rust 2024 edition with MSRV 1.94.0 and is dual-licensed under MIT OR Apache-2.0.

**Current state:** safe WAD header and lump-directory reading, typed scaffolding for classic Doom map-record lumps, WAD serialization (`write` feature), zero-copy memory-mapped loading (`mmap` feature), a `cwad` CLI with `info`/`list`/`validate`/`merge`/`diff`/`extract`/`convert`/`build` subcommands, `cargo-fuzz` targets, and Criterion benchmarking infrastructure.

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
    src/main.rs        # `info`/`list`/`validate`/`merge`/`diff`/`extract`/`convert`/`build` subcommands via clap
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
  check_doc_versions.py # Fails if a documented `crustywad = "X.Y.Z"` pin no longer resolves to the crate's actual version; run via `just docs-sync`
anchors.txt            # Anchor strings that must appear verbatim in all three main doc files
tests/
  fixtures/
    fetch_freedoom.py  # Downloads Freedoom WAD fixtures from GitHub releases
    README.md          # Fixture documentation and version configuration
tools/
  Cargo.toml           # Pinned versions of mdbook and mdbook-mermaid; Dependabot watches this
  src/lib.rs           # Empty; makes the package a valid Cargo package for Dependabot resolution
xtask/
  DESIGN.md          # idgames corpus harvest operational spec (ADR-0030; own cargo workspace)
  src/               # harvest tool: phase 1 API enumeration (#405); phases 2-3 pending (#406/#407)
.github/
  codeql/codeql-config.yml   # Advanced CodeQL query config (security-extended + quality)
  workflows/ci.yml            # Main CI pipeline
  workflows/codeql.yml        # CodeQL security analysis
  workflows/release-plz.yml  # Automated release PR workflow
  workflows/bench.yml         # Criterion benchmark trend reporting to GitHub Pages
  workflows/fuzz.yml          # cargo-fuzz targets
  workflows/pages.yml         # mdBook guide deployment to GitHub Pages
  workflows/release.yml # dist: cross-platform cwad binaries + installers (CLI releases)
  workflows/xtask.yml         # path-gated xtask workspace CI (fmt/clippy/test/deny + weekly deny sweep)
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
| Doc drift check (anchors + version pins) | `just docs-sync` |
| Benchmarks | `just bench` (criterion; HTML report in `target/criterion/`) |
| Benchmarks + open report | `just bench-open` |
| Mid-iteration check (skips doctests + rustdoc) | `just ci-fast` |
| Pre-push CI gate (fail-fast) | `just ci` |
| Full CI check (adds `build` + `deny`) | `just ci-full` |

**Always run `just ci` before pushing.** It is the fail-fast pre-push gate — `docs-sync`, `lint` (fmt + clippy), `test`, `doc`, cheapest first, so failures surface in seconds instead of after a long compile. It intentionally omits `build` — a speed tradeoff, not a lost check: `lint`'s clippy pass type-checks every target in normal (non-test) mode, `test` fully builds and links the lib and every test binary, and CI's msrv job still runs a plain `cargo build` on the pinned 1.94.0 toolchain — and `deny` (a dependency-graph audit whose outcome code edits never change). Run **`just ci-full`** — the gate plus `build` and `deny` — before releases and on any branch that touches `Cargo.toml`/`Cargo.lock`. For code-only mid-iteration loops, **`just ci-fast`** drops the doctests and the rustdoc pass (`docs-sync`, `lint`, `cargo test --tests` — unit and integration tests still run; only doctests are skipped) — a meaningful compile saving, but never a pre-push substitute, since doctests are the only check that catches API drift in doc samples.

**Match your local toolchain to CI, and treat CI as the source of truth.** CI's Rust jobs install the current `stable` toolchain via `dtolnay/rust-toolchain` (the `msrv` job pins Rust 1.94.0 instead), so `cargo fmt` and `cargo clippy` outcomes depend on the exact rustfmt/clippy version. A lagging local toolchain can make `just ci` pass locally while CI's `fmt`/`clippy` fail on the same code — run `rustup update stable` before trusting a local pass. A change is not green until `gh pr checks` shows every required check passing on the PR; local results are a fast pre-filter, not the verdict.

Core Rust commands run by CI:

```bash
cargo build --workspace --all-features
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all --check
cargo doc --workspace --all-features --no-deps
```

Freedoom fixture tests require both the feature flag and the env var. **The path
must be absolute.** Cargo runs each test binary with its CWD set to the *package*
root (`crates/crustywad`), not the workspace root, so a workspace-relative path
never resolves — the fixture tests then skip silently and appear to pass:

```bash
just fetch-fixtures                         # default version
just fetch-fixtures version=v0.14.0        # specific version
CRUSTYWAD_FREEDOOM_DIR="$PWD/tests/fixtures/freedoom" cargo test --workspace --all-features
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

- `ParseOptions { strictness: Strictness::Strict | Strictness::Lenient, limits: Limits }` (`limits` bounds UDMF text nesting depth; ignored by binary paths).
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

**Sync rule:** when a feature flag is added, removed, or renamed, update **all four** of:
1. `docs/guide/src/features.md` — detailed docs, usage examples, and the summary table (primary)
2. The summary table below in this file
3. The summary table in `.github/copilot-instructions.md`
4. The summary table in `README.md`

| Feature | Default | Purpose |
|---|---|---|
| `mmap` | no | Enables `Wad::from_path_mapped[_with_options]` for zero-copy memory-mapped loading via `memmap2`; `from_path` always reads into memory regardless of this flag |
| `freedoom-tests` | no | Enables optional integration tests against local Freedoom WADs (supplied via `CRUSTYWAD_FREEDOOM_DIR`; auto-fetchable via `just fetch-fixtures`) |
| `hexen-tests` | no | Enables optional integration tests against a local Hexen IWAD (supplied via `CRUSTYWAD_HEXEN_DIR`; not auto-fetchable) |
| `doom64-tests` | no | Enables optional integration tests against a local Doom 64 IWAD (supplied via `CRUSTYWAD_DOOM64_DIR`; not auto-fetchable) |
| `sweep-tests` | no | Enables an optional sweep test that assembles every map of every WAD in a local collection (supplied via `CRUSTYWAD_SWEEP_DIR`; not auto-fetchable; `just test-sweep`) |
| `guide-doctests` | no | **Internal, CI-only.** Compiles the mdBook guide's Rust code samples as crate doctests (enabled by `--all-features`); not a runtime capability |
| `write` | no | Enables `WadBuilder`, `WriteError`, `WriteOptions`, `WriteWarning`, and `Wad::to_builder()` for WAD serialization |
| `nodebuild` | no | Enables the `map::build` node-lump builders (implies `write`) — `build_blockmap`/`build_reject`/`build_nodes` (the classic BSP pass: `SEGS`/`SSECTORS`/`NODES`), the `add_doom_map_with_nodes` engine-playable one-shot, and their `to_lump_bytes` serializers, for clean-room BLOCKMAP/REJECT/BSP node generation (ADR-0024). Also emits a `ZDoom` non-GL `XNOD`/`ZNOD` extended-node stream via `NodeFormat` (ADR-0025), plus the GL `XGLN`/`XGL2`/`XGL3` streams (and their `Z*` twins with `extended-nodes-zlib`), with `NodeFormat::Gl` auto-selecting the minimal dialect, via `build_gl_nodes`/`BuiltGlNodes` (ADR-0026), and a UDMF one-shot (`add_udmf_map_with_nodes`) that builds a `ZNODES` stream for a UDMF map group. Powers `cwad convert --nodes` and `cwad build --nodes`, including UDMF `ZNODES` output — GL dialects by default, `xnod`/`znod` on explicit request |
| `doom64-gfx` | no | Enables `Doom64Png` decoding of Doom 64's PNG texture/sprite lumps via the `png` crate (indexed pixels + palette rows + `grAb` offsets, capped by `Limits::max_decoded_pixels`) |
| `extended-nodes-zlib` | no | Enables reading the zlib-compressed ZDoom extended node formats (`ZNOD`/`ZGLN`/`ZGL2`/`ZGL3`) by inflating (via the pure-Rust `miniz_oxide`) to their uncompressed twins and decoding through the same parser, bounded by `Limits::max_decoded_node_bytes`. Off by default so the core build stays decompressor-free (ADR-0025 §5, #327). Powers the `Z*` half of the extended-node reader. With `nodebuild` also enabled, also powers the `nodebuild` `ZNOD` and `Z*` GL writers |

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

**Breaking changes must be marked** with `!` (e.g. `feat(map)!: remove RejectLump`) or a
`BREAKING CHANGE:` footer — release-plz derives version bumps from these annotations, and an
unmarked breaking change proposes a semver-violating patch release (caught live on the 0.5.0
prep). `semver_check = true` in `release-plz.toml` runs `cargo-semver-checks` against the
published baseline as a safety net, but correct annotations are the first line of defense.

**The PR title is the only Conventional Commit release-plz ever sees.** PRs land as a single
squash commit whose subject is the PR title and whose body is empty — every branch commit
subject is discarded. So the PR title alone selects the CHANGELOG section and drives the
derived bump; a `feat(map):` commit on a branch merged under a `fix(map):` title files its new
public API under `### Fixed` and never appears under `### Added` (observed on #423 → 0.9.5).

**Title a mixed PR by its highest-impact change** — `!` > `feat` > `fix` > everything else —
**or split it** into one PR per type when the halves are genuinely separable and each earns its
own CHANGELOG line. Splitting is the better choice when both entries carry real information;
titling by highest impact is the default when they do not, or when the halves are
dependency-ordered.

Pre-1.0 this is a **changelog** decision, not a version one: `feat:` and `fix:` both derive a
patch and only `!` bumps the minor, so no title choice can change the computed version (see
[Versioning and Release Policy](../docs/guide/src/versioning.md)). **At 1.0.0 the title becomes
load-bearing** — `feat:` will bump the minor, and a `fix:`-titled PR that adds public API
becomes a real SemVer violation — so the habit is built now, while it costs nothing. Never
hand-force a version to compensate for a title.

The `lefthook.yml` pre-commit hook runs `cargo fmt` and `cargo clippy`, and validates commit messages against the Conventional Commits pattern.

## Project tracking

Work is tracked on the **[Crustywad](https://github.com/users/masriamir/projects/5)** GitHub Project board — the single source of "what to pick up next". Every roadmap issue lives there with three planning dimensions:

- **Status** (workflow stage): `Backlog` → `Ready` → `In progress` → `In review` → `Done`. Pull work from the `Ready` column. Most transitions are **agent-driven** — see [Issue status transitions](#issue-status-transitions-agent-driven) below; only `Done` is set automatically by the board on merge/close.
- **Horizon** (priority bucket): `Now` / `Next` / `Later`. This carries planning intent for issues that have no milestone yet (e.g. the editor epic #18 and its long-horizon spikes); it replaces the former `Short Term` / `Future` milestones.
- **Milestone** (release scope): milestones are **release-scoped and scope-named** (`Audio layer`, `Nodebuilder`, …) — each the set of epics/issues intended to ship together. Names deliberately do **not** encode version numbers: crates version independently (ADR-0011 §3) and `release-plz` derives the actual versions from Conventional Commits **at ship time**, so a version-shaped milestone name is a prediction that cannot reliably be kept (the `v0.7.0` milestone shipped as crustywad 0.6.1 / cli 0.3.1 on 2026-07-19, prompting this policy). At closeout, record the shipped crate versions in the milestone description. Historical milestones `v0.1.0`–`v0.6.0` keep their version-shaped names.

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

**Epics** (the `epic` label, e.g. #17, #18) carry an **aggregate** Status distinct from the per-issue flow above: an epic moves to `In progress` when its **first** sub-issue starts work, and stays there until **all** its sub-issues close (only then does the board automate it to `Done`). GitHub does not propagate a sub-issue's board **Status** field up to the epic (only completion progress rolls up automatically), so I set the epic's Status myself and announce it. An epic's Horizon (`Now`/`Next`/`Later`) still carries its planning intent; sub-issue rollup shows its completion progress.

### Milestone closeout (propose-and-confirm)

A milestone is **complete** when BOTH conditions hold:

1. **All milestone items closed** — the milestone reports `open_issues == 0`. That is GitHub's milestone counter, which covers every assigned item (issues *and* any pull requests), not just issues, and is distinct from the board's `Done` Status field — closing an item typically sets both, but the milestone counter is the signal here.
2. **Shipped** — a `release-plz` PR (`chore: release`) has merged at or after the last milestone item closed. This is tag-agnostic: milestones are scope-named, so no version correspondence is expected — any release PR merged on/after that final close is the ship signal. When closing, add the shipped crate versions to the milestone description.

GitHub never auto-closes milestones. Unlike the agent-driven board Status transitions above, milestone closeout is **propose-and-confirm**: when I notice both conditions hold for an open milestone, I surface it and **ask before closing**. On your approval:

```bash
gh api -X PATCH repos/masriamir/crustywad/milestones/<milestone_number> -f state=closed
```

Closure is reversible (`-f state=open`). To find `<milestone_number>` (and its issue counts), the companion listing recipe — `gh api "repos/masriamir/crustywad/milestones?state=all"` — lives in the `reference-project-board` memory.

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

**Release branches are not used** — `release-plz` automates version bumps, CHANGELOG, and git tags (`crustywad-v*` / `crustywad-cli-v*`) from Conventional Commits on `main`. Merge the `release-plz` release PR to ship.

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
through its comments across as many rounds as needed.

A PR is ready for human review only when **all** automated review threads are resolved **and**
all required CI checks pass (`gh pr checks`) — resolved threads over any red required check do
not make a PR ready. Only then hand off for human review.

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
| `msrv` | build + test on Rust 1.94.0 |
| `docs` | `cargo doc` with `RUSTDOCFLAGS=-D warnings` |
| `coverage` | `cargo llvm-cov` + Codecov upload |
| `security-deny` | `cargo deny check` |
| `docs-sync` | `python3 scripts/check_doc_anchors.py` — verifies anchor strings are present in all three main doc files; and `python3 scripts/check_doc_versions.py` — verifies every documented `crustywad = "X.Y.Z"` pin still resolves to the crate's actual version |

**Runner policy:** hand-authored workflows use `-latest` runner labels (`ubuntu-latest`; `macos-latest`/`windows-latest` in the test matrix) deliberately. GitHub's labeled images mutate weekly in place, so a version label buys no real reproducibility, while pinned labels become scheduled outages when GitHub retires an image (as happened to `ubuntu-20.04` and `macos-12`). Reproducibility comes from `Cargo.lock`, the pinned msrv toolchain, Dependabot-pinned action versions, and the pinned dist/mdbook tool versions instead. The only pinned runners (`ubuntu-22.04`) live in `release.yml`, which is **generated by dist** — never hand-edit it to normalize runner labels (`dist generate` owns that file), and Linux release artifacts target musl, so runner-image variance barely affects artifact compatibility.

CodeQL (`.github/workflows/codeql.yml`) runs on push, PR, and weekly. It uses `security-extended` and `security-and-quality` query suites.

`bench` (`.github/workflows/bench.yml`) runs on push to `main` and `workflow_dispatch`. It is **non-blocking** (never gates merges). On each run it uploads a downloadable Criterion HTML artifact (90-day retention) and commits benchmark trend data to the `gh-pages` branch at `dev/bench/`. The `pages.yml` guide deploy and `bench.yml` share the `gh-pages` concurrency group so they never write to the branch simultaneously.

`release-plz` (`.github/workflows/release-plz.yml`) runs two jobs on push to `main`, authenticated as a **GitHub App** (`RELEASE_PLZ_APP_ID` / `RELEASE_PLZ_APP_PRIVATE_KEY`) so its PRs and tags trigger downstream workflows. `release-pr` opens/updates the release PR; `release` publishes to crates.io via Trusted Publishing (OIDC — no stored `CARGO_REGISTRY_TOKEN`) and pushes tags. It does **not** create GitHub Releases (`git_release_enable = false`). Binary releases of the `cwad` CLI are handled by **dist** (`.github/workflows/release.yml`), which triggers on the `crustywad-cli-v*` tag and creates the GitHub Release with cross-platform binaries and installers. The library crate is excluded from dist (`[package.metadata.dist] dist = false`).

**Documented version pins (minor releases):** the README and the guide show `Cargo.toml` snippets pinned to the **full `X.Y.Z`** (`crustywad = "0.3.0"`) — the form `cargo add` itself writes, and the one that states the minimum patch a reader needs. For a `0.x` crate, Cargo's caret requirement is *minor*-pinned — `"0.2.0"` means `>=0.2.0, <0.3.0` — so the moment a release bumps the **minor** version, every stale snippet stops resolving for readers. `scripts/check_doc_versions.py` (part of `just docs-sync`, and run by the `docs-sync` CI job) enforces this: it reads `[package].version` from `crates/crustywad/Cargo.toml`, discovers every doc that carries a pin (no hardcoded file list), and fails when a pin would no longer fetch the current version.

**Consequence: a release PR that bumps the minor version will fail CI until its doc pins are updated.** That is intentional — the pins silently rotted through the `0.1` → `0.2` bump and nobody noticed (#235). When `release-plz` opens a **minor**-bump release PR, push a commit onto its branch updating the pins in `README.md` and `docs/guide/src/`, then merge.

**Patch bumps need no change:** `0.3.0` → `0.3.1` still resolves against a `"0.3.0"` pin (the caret fetches 0.3.1), so CI stays green and the docs need no commit. The check fires only when a pin genuinely stops working.

**Version bump:** `crates/crustywad-cli/Cargo.toml` pins the `crustywad` path dependency with an explicit version (`crustywad = { path = "../crustywad", version = "X.Y.Z" }`), which Cargo treats as a caret requirement — patch/compatible bumps to `crustywad`'s version need no change here. `cargo-deny` requires this field (`wildcards = "deny"`) but it does not inherit `crustywad`'s version automatically. Crates are versioned independently (ADR-0011 §3, no `version.workspace = true`); update this field only when `crustywad`'s version moves outside the current caret range (e.g. `0.1.z` → `0.2.0`), or `cargo deny check` will fail.

## Roadmap

The original six-item format roadmap (directory reading, map lump parsing with full graph assembly, graphics, textures, audio, write support) has shipped in full. Current direction lives on the project board (see [Project tracking](#project-tracking) above) — active long-horizon epics: ACS support (#242), editor foundations (#18), and idgames corpus tooling (#401).
