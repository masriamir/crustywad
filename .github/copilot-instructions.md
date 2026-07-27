# GitHub Copilot Instructions for `crustywad`

## Project overview

`crustywad` is a Rust workspace providing safe, documented Doom WAD file I/O. It targets the Rust 2024 edition with MSRV 1.94.0 and is dual-licensed under MIT OR Apache-2.0.

The repository currently implements:
- Safe WAD header and lump-directory reading, plus typed scaffolding for classic Doom map-record lumps (`THINGS`, `LINEDEFS`, `SIDEDEFS`, `VERTEXES`, `SEGS`, `SSECTORS`, `NODES`, `SECTORS`)
- WAD serialization (`WadBuilder`, behind the `write` feature)
- Zero-copy memory-mapped loading (behind the `mmap` feature)
- A `cwad` CLI binary with `info`, `list`, `validate`, `merge`, `diff`, `extract`, `convert`, and `build` subcommands
- `cargo-fuzz` targets and Criterion benchmarking infrastructure

## Workspace layout

```
crates/
  crustywad/           # Core library crate
    src/
      lib.rs           # Public API, WAD parsing logic
      error.rs         # ParseError and ParseWarning types
      map.rs           # Typed map-record structs and parse_records<T>
      mmap.rs          # Optional memory-mapped I/O (feature = "mmap")
    tests/
      common/mod.rs    # Shared WAD-building helpers for integration tests
      freedoom.rs      # Optional Freedoom fixture tests (feature = "freedoom-tests")
      map_records.rs   # Integration tests for typed map-record parsing
      wad_reader.rs    # Integration tests for the main WAD reader API
  crustywad-cli/       # CLI binary crate (`cwad`)
    src/main.rs
    src/cli.rs         # CLI argument types (info/list/validate/merge/diff/extract/convert/build)
docs/
  design.md            # Goals, data model, read pipeline, feature plan
  adr/                 # Architecture decision records
  diagrams/            # Standalone Mermaid diagram files (source for the guide via {{#include}})
  guide/               # mdBook user guide — deployed to GitHub Pages; single source of truth for user-facing docs
    book.toml          # mdBook + mdbook-mermaid configuration
    src/               # Guide source pages (SUMMARY.md, *.md including features.md)
scripts/
  check_doc_anchors.py # Living-docs anchor drift detector (ADR-0007); run via `just docs-sync`
  check_doc_versions.py # Fails if a documented `crustywad = "X.Y.Z"` pin no longer resolves to the crate's actual version; run via `just docs-sync`
anchors.txt            # Anchor strings that must appear verbatim in all three main doc files
tests/
  fixtures/
    fetch_freedoom.py  # Script to download Freedoom WAD fixtures
    README.md          # Fixture documentation
tools/
  Cargo.toml           # Pinned versions of mdbook and mdbook-mermaid; Dependabot watches this
  src/lib.rs           # Empty; makes the package a valid Cargo package for Dependabot resolution
.github/
  codeql/
    codeql-config.yml  # Advanced CodeQL query configuration
  workflows/
    ci.yml             # Main CI pipeline
    codeql.yml         # CodeQL security analysis workflow
    release-plz.yml    # Automated release workflow
    bench.yml          # Criterion benchmark trend reporting to GitHub Pages
    fuzz.yml           # cargo-fuzz targets
    pages.yml          # mdBook guide deployment to GitHub Pages
    release.yml        # dist: cross-platform cwad binaries + installers (CLI releases)
```

## Development workflow

Install [just](https://github.com/casey/just) then run these recipes:

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
| Full CI check | `just ci` |

**Always run `just ci` before pushing.** It runs the same checks as GitHub Actions (build, test, clippy, fmt, doc, deny, docs-sync) and catches failures locally before they reach CI.

**Match your local toolchain to CI.** CI's Rust jobs install the current `stable` toolchain via `dtolnay/rust-toolchain` (the `msrv` job pins Rust 1.94.0 instead), so `cargo fmt` and `cargo clippy` outcomes depend on the exact rustfmt/clippy version — with a stale local toolchain these checks can pass locally yet fail in CI on the same code. Run `rustup update stable` before trusting a local pass, and treat `gh pr checks` (all required checks green on the PR) as the source of truth, not local results.

Core Rust commands run by CI:

```bash
cargo build --workspace --all-features
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all --check
cargo doc --workspace --all-features --no-deps
```

## Code conventions

### Error handling
- All errors in the library crate use `thiserror`-derived enums (`ParseError`, `MapParseError`).
- `anyhow` is permitted only in the CLI binary (`crustywad-cli`).
- Every public function that can fail returns `Result<_, ErrorType>` with full doc comments on the `# Errors` section.

### Documentation
- All documentation uses American English spelling (e.g. "artifacts" not "artefacts", "customization" not "customisation").
- `missing_docs = "deny"` is enforced workspace-wide — every public item must have a doc comment.
- Use `///` for item-level docs and `//!` for module-level docs.
- Include `# Errors` sections in doc comments for fallible functions.
- Include `# Panics` sections if a function can panic.

### Safety
- `#![deny(unsafe_code)]` is set in the core library crate.
- Unsafe code is permitted only in `mmap.rs` (behind `#![allow(unsafe_code)]`), solely to call `memmap2::MmapOptions::map`. All parsing and validation logic must remain free of `unsafe`.

### Lints
- All Clippy warnings from `clippy::all` and `clippy::pedantic` are enabled workspace-wide.
- New code must compile with zero warnings.

### Naming
- Doom WAD concepts follow the names in the WAD specification: `Lump`, `WadKind`, `WadHeader`, `ParseOptions`, etc.
- Map-record types use singular Rust names matching the lump: `Thing`, `Linedef`, `Sidedef`, `Vertex`, `Seg`, `Subsector`, `Node`, `Sector`.
- Use `snake_case` for Rust items and `UPPER_SNAKE_CASE` for constants.

### Strictness model
- Parsing is controlled by `ParseOptions { strictness: Strictness::Strict | Strictness::Lenient, limits: Limits }` (`limits` bounds UDMF text nesting depth; ignored by binary paths).
- Strict mode returns the first `ParseError` it encounters.
- Lenient mode attempts best-effort recovery and collects `ParseWarning` values.
- New validation logic must honour both modes.

## Testing practices

### Unit vs integration tests
- **Unit tests** (for private helpers): place in a `#[cfg(test)] mod tests {}` block within the source file, only when the tested code is not accessible from outside the module.
- **Integration tests** (for public API, including typed map records): place in `crates/crustywad/tests/`, one file per concern. These have access only to the public API.
- The `common/` module in the test directory contains shared WAD-building helpers; add shared test utilities there.

### Freedoom fixture tests
- Optional integration tests that parse real WAD files live in `tests/freedoom.rs`, gated by `#[cfg(feature = "freedoom-tests")]`.
- Fixtures are fetched from the Freedoom GitHub release archive using `just fetch-fixtures`.
- The Freedoom version is configurable:
  - CLI: `just fetch-fixtures version=v0.14.0`
  - Environment variable: `FREEDOOM_VERSION=v0.14.0 just fetch-fixtures`
  - Direct invocation: `python3 tests/fixtures/fetch_freedoom.py --version v0.14.0`
- Fixture WADs are gitignored. Do not commit them.
- Fixture tests skip gracefully when `CRUSTYWAD_FREEDOOM_DIR` is not set.

### Property-based tests
- Use `proptest` (already a dev-dependency) for property-based tests.
- Place them in the same file as the regular tests they complement.

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
2. Ensure the struct implements `BinRead` with `#[br(little)]`.
3. Add integration tests in `crates/crustywad/tests/map_records.rs`.
4. Update `README.md` if the new lump type is user-visible.
5. Run `just lint` and `just doc` before committing.

## Feature flags

The full feature flag reference (usage examples, platform notes, cargo invocations) lives in
[`docs/guide/src/features.md`](../docs/guide/src/features.md), which is published via the
mdBook guide to GitHub Pages. That file is the single source of truth.

**Sync rule:** when a feature flag is added, removed, or renamed, update **all four** of:
1. `docs/guide/src/features.md` — detailed docs, usage examples, and the summary table
2. The summary table in `.claude/CLAUDE.md` (Feature flags section)
3. The summary table below in this file
4. The summary table in `README.md`

| Feature | Default | Purpose |
|---|---|---|
| `mmap` | no | Enables `Wad::from_path_mapped[_with_options]` for zero-copy memory-mapped loading via `memmap2`; `from_path` always reads into memory regardless of this flag |
| `freedoom-tests` | no | Enables optional integration tests against local Freedoom WADs (via `CRUSTYWAD_FREEDOOM_DIR`; auto-fetchable via `just fetch-fixtures`) |
| `hexen-tests` | no | Enables optional integration tests against a local Hexen IWAD (via `CRUSTYWAD_HEXEN_DIR`; not auto-fetchable) |
| `doom64-tests` | no | Enables optional integration tests against a local Doom 64 IWAD (via `CRUSTYWAD_DOOM64_DIR`; not auto-fetchable) |
| `sweep-tests` | no | Enables an optional sweep test assembling every map of every WAD in a local collection (via `CRUSTYWAD_SWEEP_DIR`; not auto-fetchable) |
| `guide-doctests` | no | Internal, CI-only: compiles the guide's Rust code samples as crate doctests (enabled by `--all-features`); not a runtime capability |
| `write` | no | Enables `WadBuilder`, `WriteError`, `WriteOptions`, `WriteWarning`, and `Wad::to_builder()` for WAD serialization |
| `nodebuild` | no | Enables the `map::build` node-lump builders (implies `write`) — `build_blockmap`/`build_reject`/`build_nodes` (the classic BSP pass: `SEGS`/`SSECTORS`/`NODES`), the `add_doom_map_with_nodes` engine-playable one-shot, and their `to_lump_bytes` serializers, for clean-room BLOCKMAP/REJECT/BSP node generation (ADR-0024). Also emits a `ZDoom` non-GL `XNOD`/`ZNOD` extended-node stream via `NodeFormat` (ADR-0025), plus the GL `XGL3` stream (and `ZGL3` with `extended-nodes-zlib`) via `build_gl_nodes`/`BuiltGlNodes` (ADR-0026). Powers `cwad convert --nodes` |
| `doom64-gfx` | no | Enables `Doom64Png` decoding of Doom 64's PNG texture/sprite lumps via the `png` crate (indexed pixels + palette rows + `grAb` offsets, capped by `Limits::max_decoded_pixels`) |
| `extended-nodes-zlib` | no | Enables reading the zlib-compressed ZDoom extended node formats (`ZNOD`/`ZGLN`/`ZGL2`/`ZGL3`) by inflating (pure-Rust `miniz_oxide`) to their uncompressed twins, bounded by `Limits::max_decoded_node_bytes`; off by default so the core build stays decompressor-free (ADR-0025 §5). With `nodebuild` also enabled, also powers the `nodebuild` `ZNOD` and `ZGL3` writers |

## Commit conventions

Follow [Conventional Commits](https://www.conventionalcommits.org/):
- `feat:` — new functionality
- `fix:` — bug fixes
- `docs:` — documentation changes only
- `test:` — test-only changes
- `refactor:` — no behaviour change
- `chore:` — build, tooling, CI
- `ci:` — CI workflow changes

Scope can be added when useful: `feat(map):`, `fix(cli):`, etc.

The `lefthook.yml` pre-commit hook runs `cargo fmt` and `cargo clippy`, and validates commit messages against the Conventional Commits pattern.

## Git branching workflow

All feature and bugfix work branches from `main` after a `git pull`. Branches are tied to GitHub issue numbers.

| Branch type | Template | Example |
|---|---|---|
| Feature | `feature/###` or `feature/###-short-desc` | `feature/42-mmap-support` |
| Bugfix | `bugfix/###` or `bugfix/###-short-desc` | `bugfix/17-header-parse` |
| Hotfix | `hotfix/###` or `hotfix/###-short-desc` | `hotfix/55-oob-read` |

`###` is the GitHub issue number. A short slug after the number is optional but encouraged for readability.

**Release branches are not used** — `release-plz` automates version bumps, CHANGELOG, and git tags (`vX.Y.Z`) from Conventional Commits on `main`. When publishing is enabled, merge the `release-plz` release PR to ship.

The `lefthook.yml` pre-push hook enforces branch naming and will reject pushes from non-conforming branches.

**Workflow:**
1. `git pull origin main`
2. `git checkout -b feature/###-description`
3. Commit with Conventional Commits (`feat(scope): ...`)
4. Run `just ci` before pushing
5. Open a PR against `main`

## CI and security

The main CI pipeline (`.github/workflows/ci.yml`) runs:
- `fmt` — formatting check
- `clippy` — lint check
- `test` — test matrix on ubuntu, macos, windows
- `msrv` — MSRV build + test (Rust 1.94.0)
- `docs` — doc build with `-D warnings`
- `coverage` — llvm-cov upload to Codecov
- `security-deny` — `cargo deny check`
- `docs-sync` — anchor drift check via `python3 scripts/check_doc_anchors.py`, plus documented-version-pin drift check via `python3 scripts/check_doc_versions.py`

CodeQL static analysis (`.github/workflows/codeql.yml`) runs on push, pull request, and weekly on a schedule. It uses the advanced configuration in `.github/codeql/codeql-config.yml` which enables the `security-extended` and `security-and-quality` query suites.

`release-plz` (`.github/workflows/release-plz.yml`) runs two jobs on push to `main`, authenticated as a **GitHub App** (`RELEASE_PLZ_APP_ID` / `RELEASE_PLZ_APP_PRIVATE_KEY`) so its PRs and tags trigger downstream workflows. `release-pr` opens/updates the release PR; `release` publishes to crates.io via Trusted Publishing (OIDC — no stored `CARGO_REGISTRY_TOKEN`) and pushes tags. It does **not** create GitHub Releases (`git_release_enable = false`). Binary releases of the `cwad` CLI are handled by **dist** (`.github/workflows/release.yml`), which triggers on the `crustywad-cli-v*` tag and creates the GitHub Release with cross-platform binaries and installers. The library crate is excluded from dist (`[package.metadata.dist] dist = false`).

**Version bump:** `crates/crustywad-cli/Cargo.toml` declares the `crustywad` path dependency with an explicit `version` field (`crustywad = { path = "../crustywad", version = "X.Y.Z" }`), which Cargo treats as a caret requirement — patch/compatible bumps to `crustywad`'s version need no change here. `cargo-deny` enforces `wildcards = "deny"` and requires this field, but it does not inherit `crustywad`'s version automatically. Crates are versioned independently (ADR-0011 §3, no `version.workspace = true`); update this field only when `crustywad`'s version moves outside the current caret range (e.g. `0.1.z` → `0.2.0`), or the `security-deny` CI job will fail.

## Roadmap context

The current milestone (1) covers WAD directory reading and map-record scaffolding. Future milestones are:

1. ✅ Directory reading
2. 🔜 Map lump parsing (full graph assembly)
3. Graphics
4. Textures
5. Audio
6. ✅ Write support

When implementing future milestones, keep `unsafe` code confined to `mmap.rs` and ensure every public item remains documented.
