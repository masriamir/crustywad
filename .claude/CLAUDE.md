# CLAUDE.md

## Project overview

`crustywad` is a Rust workspace providing safe, documented Doom WAD file I/O. It targets the Rust 2024 edition with MSRV 1.85.0 and is dual-licensed under MIT OR Apache-2.0.

**Current milestone (1):** safe WAD header and lump-directory reading, typed scaffolding for classic Doom map-record lumps, and a small CLI binary for dogfooding.

## Workspace layout

```
crates/
  crustywad/           # Core library crate
    src/
      lib.rs           # Public API — Wad, WadKind, WadHeader, Lump, ParseOptions, Strictness
      error.rs         # ParseError (thiserror) and ParseWarning types
      map.rs           # Typed map-record structs and parse_records<T>
      mmap.rs          # Read-only memmap2-backed file loading (feature = "mmap")
    tests/
      common/mod.rs    # Shared WAD-building helpers (build_wad, lump_map)
      wad_reader.rs    # Integration tests for the main WAD reader API
      map_records.rs   # Integration tests for typed map-record parsing
      freedoom.rs      # Optional Freedoom fixture tests (feature = "freedoom-tests")
  crustywad-cli/       # CLI binary crate (`cwad`)
    src/main.rs        # `info` and `list` subcommands via clap
docs/
  design.md            # Goals, data model, read pipeline, feature plan
  adr/                 # Architecture decision records
tests/
  fixtures/
    fetch_freedoom.py  # Downloads Freedoom WAD fixtures from GitHub releases
    README.md          # Fixture documentation and version configuration
.github/
  codeql/codeql-config.yml   # Advanced CodeQL query config (security-extended + quality)
  workflows/ci.yml            # Main CI pipeline
  workflows/codeql.yml        # CodeQL security analysis
  workflows/release-plz.yml  # Automated release PR workflow
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
| Coverage | `just cov` (requires `cargo-llvm-cov`) |
| Dependency audit | `just deny` (requires `cargo-deny`) |
| Fetch Freedoom fixtures | `just fetch-fixtures` |
| Full CI check | `just ci` |

**Always run `just ci` before pushing.** It runs the same checks as GitHub Actions (build, test, clippy, fmt, doc) and catches failures locally before they reach CI.

Exact CI commands:

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

1. Add a `binrw`-derived struct to `crates/crustywad/src/map.rs` with full doc comments on every field.
2. Ensure the struct uses `#[br(little)]` and implements `BinRead` with `Args<'a> = ()`.
3. Check the Doom WAD spec for correct field types (signed vs unsigned matters — e.g., `Thing.angle` is `u16`, coords are `i16`).
4. Add integration tests in `crates/crustywad/tests/map_records.rs` with a hand-crafted byte sequence and at least one field assertion per field.
5. Add a proptest if the type has meaningful invariants.
6. Update `README.md` if the new type is user-visible.
7. Run `just lint` and `just doc` before committing.

## Feature flags

See [`docs/features.md`](../docs/features.md) for the full feature flag reference including usage examples, platform notes, and common `cargo` invocations.

| Feature | Default | Purpose |
|---|---|---|
| `mmap` | no | Enables `Wad::from_path_mapped[_with_options]` for zero-copy memory-mapped loading via `memmap2`; `from_path` always reads into memory regardless of this flag |
| `freedoom-tests` | no | Enables optional integration tests against local Freedoom fixture WADs |

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

CodeQL (`.github/workflows/codeql.yml`) runs on push, PR, and weekly. It uses `security-extended` and `security-and-quality` query suites.

`release-plz` (`.github/workflows/release-plz.yml`) creates release PRs on push to `main`. Publishing to crates.io is intentionally disabled until credentials and release policy are ready.

**Version bump:** `crates/crustywad-cli/Cargo.toml` pins the `crustywad` path dependency with an explicit version (`crustywad = { path = "../crustywad", version = "X.Y.Z" }`). `cargo-deny` requires this (`wildcards = "deny"`) but it does not inherit the workspace version automatically. When bumping the workspace `version`, update this field to match or `cargo deny check` will fail.

## Roadmap

1. ✅ Directory reading and map-record scaffolding (this PR)
2. 🔜 Map lump parsing (full graph assembly)
3. Graphics
4. Textures
5. Audio
6. Write support
