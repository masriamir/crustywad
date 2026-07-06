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

1. Add a `binrw`-derived struct to `crates/crustywad/src/map.rs` with full doc comments on every field.
2. Ensure the struct uses `#[br(little)]` and implements `BinRead` with `Args<'a> = ()`.
3. Check the Doom WAD spec for correct field types (signed vs unsigned matters — e.g., `Thing.angle` is `u16`, coords are `i16`).
4. Add integration tests in `crates/crustywad/tests/map_records.rs` with a hand-crafted byte sequence and at least one field assertion per field.
5. Add a proptest if the type has meaningful invariants.
6. Update `README.md` if the new type is user-visible.
7. Run `just lint` and `just doc` before committing.

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

## Copilot review-comment workflow

PRs are reviewed automatically by `copilot-pull-request-reviewer`. Work through its comments with this loop until a round produces no new comments, then hand off for human review:

1. Fetch all review threads directly via `gh api graphql` with a query of the form `{ repository(owner: "OWNER", name: "REPO") { pullRequest(number: 140) { reviewThreads(first: 100) { nodes { id isResolved path line comments(last: 10) { nodes { databaseId author { login } body } } } pageInfo { hasNextPage endCursor } } } } }` (100 is GitHub's per-page max — follow `pageInfo` if a PR ever has more threads than that; keep the thread `id` in the result, it's needed by `resolveReviewThread` in step 5; use `last: 10` on `comments` so a long thread shows its most recent replies rather than its oldest ones), then filter the returned nodes client-side for `isResolved: false` — the connection itself takes no `resolved` argument. Prefer this over any cached/wrapped PR-comment tool — it has been observed to return stale results.
2. For each thread, verify the comment against actual code and test/CI behavior before acting — Copilot comments are sometimes wrong (e.g. asserting the opposite of documented behavior) or based on a stale diff. Confirm by reading the relevant source, not just the comment text.
3. Fix the underlying issue, or reply explaining why no change is needed if the comment doesn't hold up.
4. Run `just ci` locally before pushing.
5. Push, then reply on the review comment (referencing the fix commit) and resolve the thread via the GraphQL `resolveReviewThread` mutation.
6. Once every thread from the current round is resolved, note the current time as an explicit UTC ISO-8601 timestamp (e.g. via `date -u +%Y-%m-%dT%H:%M:%SZ`), then request a fresh Copilot review (`POST /pulls/{number}/requested_reviewers` with `copilot-pull-request-reviewer[bot]`).
7. Poll every ~60 seconds — do not wait a flat 5 minutes here — confirming via `gh api graphql` with a query of the form `{ repository(owner: "OWNER", name: "REPO") { pullRequest(number: 140) { reviews(last: 20) { nodes { author { login } submittedAt } } } } }` whether a review with `author.login` equal to `copilot-pull-request-reviewer` exists (GraphQL omits the `[bot]` suffix used when requesting the reviewer via REST) with `submittedAt` on or after the request time. Use a generous `last` count (20, not 5) — a small window can push the target review out if several other reviews land afterward. In practice the review has landed within 1-2 minutes each time observed, so 60s polling finds it far sooner than a fixed 5-minute wait would — if no such review is found yet, keep polling rather than checking `reviewThreads`. The step 6 timestamp has whole-second precision, so treat a `submittedAt` equal to (not just strictly after) the request time as a match — a stricter `>` comparison can miss a review submitted in the same second.
8. Once that review is found, immediately fetch `reviewThreads` (the query from step 1) as the first of two required thread checks.
9. Wait a further ~5 minutes from the moment the review was found (this dwell is not shortened by the tighter polling in step 7) and fetch `reviewThreads` again as the second required thread check. Copilot has been observed to post additional comments in a second wave up to ~5 minutes after its review event first appears (e.g. PR #136: a round was marked resolved at 15:30, but two more legitimate comments landed at 15:35) — do not skip or shorten this dwell just because step 7 finished quickly, since the second wave is a function of time since the review landed, not time since the request.
10. If either the step 8 check or the step 9 check surfaces new unresolved threads, work through them (steps 2-5) and repeat from step 6. Only report the PR ready for human review once a single round produces two consecutive clean `reviewThreads` checks — the step 8 check and the step 9 check both showing zero new unresolved threads.

**Branch drift:** if the PR branch has `main` merged into it mid-session (e.g. by an earlier, unrelated commit), a local branch based on an older fetch will diverge and a plain `git push` will be rejected as non-fast-forward. Re-fetch the remote branch and `git rebase` onto its tip before pushing; resolve any conflicts by hand rather than force-pushing over the newer history.

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

`release-plz` (`.github/workflows/release-plz.yml`) creates release PRs on push to `main`. Publishing to crates.io is intentionally disabled until credentials and release policy are ready.

**Version bump:** `crates/crustywad-cli/Cargo.toml` pins the `crustywad` path dependency with an explicit version (`crustywad = { path = "../crustywad", version = "X.Y.Z" }`). `cargo-deny` requires this (`wildcards = "deny"`) but it does not inherit the workspace version automatically. When bumping the workspace `version`, update this field to match or `cargo deny check` will fail.

## Roadmap

1. ✅ Directory reading and map-record scaffolding (this PR)
2. 🔜 Map lump parsing (full graph assembly)
3. Graphics
4. Textures
5. Audio
6. Write support
