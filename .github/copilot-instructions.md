# GitHub Copilot Instructions for `crustywad`

## Project overview

`crustywad` is a Rust workspace providing safe, documented Doom WAD file I/O. It targets the Rust 2024 edition with MSRV 1.85.0 and is dual-licensed under MIT OR Apache-2.0.

The repository currently implements milestone 1:
- Safe WAD header and lump-directory reading
- Typed scaffolding for classic Doom map-record lumps (`THINGS`, `LINEDEFS`, `SIDEDEFS`, `VERTEXES`, `SEGS`, `SSECTORS`, `NODES`, `SECTORS`)
- A small CLI binary for dogfooding the parser

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
      freedoom.rs      # Optional FreeDoom fixture tests (feature = "freedoom-tests")
      map_records.rs   # Integration tests for typed map-record parsing
      wad_reader.rs    # Integration tests for the main WAD reader API
  crustywad-cli/       # CLI binary crate (`cwad`)
    src/main.rs
docs/                  # Design doc, ADRs
tests/
  fixtures/
    fetch_freedoom.py  # Script to download FreeDoom WAD fixtures
    README.md          # Fixture documentation
.github/
  codeql/
    codeql-config.yml  # Advanced CodeQL query configuration
  workflows/
    ci.yml             # Main CI pipeline
    codeql.yml         # CodeQL security analysis workflow
    release-plz.yml    # Automated release workflow
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
| Coverage | `just cov` (requires `cargo-llvm-cov`) |
| Dependency audit | `just deny` (requires `cargo-deny`) |
| Fetch FreeDoom fixtures | `just fetch-fixtures` |
| Full CI check | `just ci` |

Exact commands used in CI:

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
- Parsing is controlled by `ParseOptions { strictness: Strictness::Strict | Strictness::Lenient }`.
- Strict mode returns the first `ParseError` it encounters.
- Lenient mode attempts best-effort recovery and collects `ParseWarning` values.
- New validation logic must honour both modes.

## Testing practices

### Unit vs integration tests
- **Unit tests** (for private helpers): place in a `#[cfg(test)] mod tests {}` block within the source file, only when the tested code is not accessible from outside the module.
- **Integration tests** (for public API, including typed map records): place in `crates/crustywad/tests/`, one file per concern. These have access only to the public API.
- The `common/` module in the test directory contains shared WAD-building helpers; add shared test utilities there.

### FreeDoom fixture tests
- Optional integration tests that parse real WAD files live in `tests/freedoom.rs`, gated by `#[cfg(feature = "freedoom-tests")]`.
- Fixtures are fetched from the FreeDoom GitHub release archive using `just fetch-fixtures`.
- The FreeDoom version is configurable:
  - CLI: `just fetch-fixtures version=v0.14.0`
  - Environment variable: `FREEDOOM_VERSION=v0.14.0 just fetch-fixtures`
  - Direct invocation: `python tests/fixtures/fetch_freedoom.py --version v0.14.0`
- Fixture WADs are gitignored. Do not commit them.
- Fixture tests skip gracefully when `CRUSTYWAD_FREEDOOM_DIR` is not set.

### Property-based tests
- Use `proptest` (already a dev-dependency) for property-based tests.
- Place them in the same file as the regular tests they complement.

## Adding a new lump type

1. Add a `binrw`-derived struct to `crates/crustywad/src/map.rs` with full doc comments on every field.
2. Ensure the struct implements `BinRead` with `#[br(little)]`.
3. Add integration tests in `crates/crustywad/tests/map_records.rs`.
4. Update `README.md` if the new lump type is user-visible.
5. Run `just lint` and `just doc` before committing.

## Feature flags

| Feature | Default | Purpose |
|---|---|---|
| `mmap` | no | Enables `Wad::from_path_mapped[_with_options]` for zero-copy memory-mapped loading via `memmap2`; `from_path` always reads into memory regardless of this flag |
| `freedoom-tests` | no | Enables optional FreeDoom integration tests |

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

## CI and security

The main CI pipeline (`.github/workflows/ci.yml`) runs:
- `fmt` — formatting check
- `clippy` — lint check
- `test` — test matrix on ubuntu, macos, windows
- `msrv` — MSRV build + test (Rust 1.85.0)
- `docs` — doc build with `-D warnings`
- `coverage` — llvm-cov upload to Codecov
- `security-deny` — `cargo deny check`

CodeQL static analysis (`.github/workflows/codeql.yml`) runs on push, pull request, and weekly on a schedule. It uses the advanced configuration in `.github/codeql/codeql-config.yml` which enables the `security-extended` and `security-and-quality` query suites.

**Version bump:** `crates/crustywad-cli/Cargo.toml` declares the `crustywad` path dependency with an explicit `version` field (`crustywad = { path = "../crustywad", version = "X.Y.Z" }`). `cargo-deny` enforces `wildcards = "deny"` and requires this, but it does not inherit the workspace version automatically. When bumping the workspace `version`, update this field to match or the `security-deny` CI job will fail.

## Roadmap context

The current milestone (1) covers WAD directory reading and map-record scaffolding. Future milestones are:

1. ✅ Directory reading
2. 🔜 Map lump parsing (full graph assembly)
3. Graphics
4. Textures
5. Audio
6. Write support

When implementing future milestones, keep `unsafe` code confined to `mmap.rs` and ensure every public item remains documented.
