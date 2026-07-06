# ADR-0012: Criterion benchmarking infrastructure, GitHub Pages trend reporting, and CI integration

- **Status:** Accepted
- **Date:** 2026-07-06
- **Deciders:** @masriamir
- **Tracking issues:** https://github.com/masriamir/crustywad/issues/2,
  https://github.com/masriamir/crustywad/issues/3,
  https://github.com/masriamir/crustywad/issues/6

## Context

`crustywad` needs measurable performance baselines for its WAD read and write paths so that
regressions are caught before shipping and optimization work (issue #67) is guided by data
rather than intuition. Two categories of operations are in scope:

1. **Read path** — `Wad::from_bytes*`, `Wad::from_path*`, all lump accessors, and
   `parse_records::<T>` for all eight classic map record types (issues #3, #4).
2. **Write path** — `WadBuilder::build`, `WadBuilder::build_with_options`, and the
   `Wad::to_builder` + `build` round-trip (issue #6). Write support landed in Epic #12;
   the original stub/suppress plan in issue #5 was closed as obsolete.

Three questions need answers before implementation:

1. Which Rust benchmarking library to use?
2. Where and how to persist benchmark results so trend charts survive across CI runs?
3. How to integrate reporting with the existing GitHub Pages setup, which currently deploys
   the mdBook user guide as a GitHub Actions artifact?

## Decision drivers

- Criterion HTML reports must be downloadable as GitHub Actions artifacts on every bench run.
- Trend data must persist across CI runs — per-run summaries alone are insufficient for
  regression detection over time.
- Benchmarks must not block or slow down the main CI pipeline (`ci.yml`).
- The setup must follow established community practices.
- Concurrent writes to GitHub Pages from two independent workflows must be safe.

## Considered options

1. Artifact-based Pages (keep current setup; integrate bench HTML via cross-workflow artifact handoff)
2. `gh-pages` branch (migrate Pages to branch-based deployment; use `github-action-benchmark`)

## Decision outcome

Chosen option: **`gh-pages` branch**, because the `github-action-benchmark` tool — the
de-facto standard for Criterion trend tracking on GitHub Actions — requires a persistent
branch for JSON trend data. Option 1 cannot provide durable trend charts without
reimplementing what that action already does, and cross-workflow artifact handoff is fragile
and error-prone.

### Consequences

- Good, because trend charts are available at a stable URL (`crustywad.dev/dev/bench/`) and
  persist indefinitely.
- Good, because Criterion HTML reports are always downloadable as GitHub Actions artifacts
  (90-day default retention).
- Good, because benchmarks run on a separate, non-blocking workflow; CI merges are
  unaffected.
- Bad, because migrating Pages source requires a one-time manual step in GitHub Settings.
- Neutral, because `pages.yml` is rewritten to push via `JamesIves/github-pages-deploy-action` rather
  than the artifact upload pattern; `pages: write` and `id-token: write` permissions are
  replaced with `contents: write`.

## Pros and cons of the options

### Option 1 — artifact-based Pages

- Good, because no GitHub Settings change is required.
- Bad, because cross-workflow artifact downloads require knowing the bench workflow's
  `run-id`; artifacts expire (max 90 days), leaving Pages without fresh data on expiry.
- Bad, because `github-action-benchmark` requires a `gh-pages` branch — incompatible with
  this model.
- Bad, because running benchmarks inside the Pages build job makes every guide deploy slow.

### Option 2 — `gh-pages` branch

- Good, because this is the established pattern; `github-action-benchmark` was designed for
  exactly this layout.
- Good, because Criterion HTML artifact upload and `gh-pages` trend push are independent
  steps; the artifact succeeds even if the trend push is skipped.
- Good, because the guide workflow becomes simpler (git push replaces artifact orchestration).
- Bad, because a one-time manual Pages source change is required in repository Settings.
- Bad, because two workflows writing to the same branch creates a concurrent write risk —
  mitigated by a GitHub Actions `concurrency` group.

## Detailed design

### Benchmarking library

**Criterion** (`criterion` crate, with the `html_reports` feature) is the industry standard
for Rust micro-benchmarks. It provides statistical analysis, outlier detection, throughput
reporting (`Throughput::Bytes` for MB/s), and self-contained HTML report generation. Its
`--output-format bencher` flag emits libtest-compatible output that `github-action-benchmark`
parses with `tool: cargo`.

### Cargo changes

`criterion` is added as a dev-dependency directly in `crates/crustywad/Cargo.toml`, not the
workspace, since only the core library has bench targets:

```toml
[dev-dependencies]
criterion = { version = "0.7", features = ["html_reports"] }
```

Criterion 0.8.x requires Rust 1.86, one above the workspace MSRV of 1.85.0. Criterion 0.7.0
is the latest MSRV-compatible release and is pinned using `"0.7"`, which accepts future 0.7.x
patch releases automatically.

Two explicit bench targets are declared (`autobenches = false` is already set):

```toml
[[bench]]
name = "read_ops"
harness = false

[[bench]]
name = "write_ops"
harness = false
required-features = ["write"]
```

`harness = false` is required by Criterion. `required-features = ["write"]` ensures
`write_ops` is skipped when the `write` feature is absent. `cargo bench --all-features`
(used by `just bench`) enables it.

**MSRV note.** `cargo test --workspace --all-features` in the `msrv` CI job compiles dev-
dependencies, including Criterion. If the chosen Criterion version's MSRV exceeds 1.85.0,
the `msrv` job in `ci.yml` is updated to run `cargo build --workspace --all-features` only
(dropping `cargo test`), consistent with the precedent established in ADR-0009 for
`cargo-fuzz`. The implementation PR must verify this and update `ci.yml` accordingly.

### Bench file layout

```
crates/crustywad/benches/
  helpers.rs     — synthetic WAD builder + Freedoom loader (standalone; not imported from tests)
  read_ops.rs    — all read-path benchmarks
  write_ops.rs   — all write-path benchmarks (feature = "write")
```

**`helpers.rs`**

| Symbol | Description |
|--------|-------------|
| `fn build_wad(kind: [u8; 4], lumps: &[(&str, &[u8])]) -> Vec<u8>` | Identical layout to the test helper; standalone copy to avoid coupling benches to test internals |
| `fn freedoom_wad_path() -> Option<PathBuf>` | Reads `CRUSTYWAD_FREEDOOM_DIR`; returns `None` when unset |
| `fn small_wad() -> Vec<u8>` | 10 lumps × 256 B, PWAD |
| `fn medium_wad() -> Vec<u8>` | 100 lumps × 4 KiB, PWAD |
| `fn large_wad() -> Vec<u8>` | 1 000 lumps × 16 KiB, PWAD |

**`read_ops.rs`** (closes issue #3)

| Benchmark group | Operations |
|-----------------|------------|
| `parse/from_bytes` | `Wad::from_bytes` strict on small / medium / large; `Throughput::Bytes` |
| `parse/from_bytes_lenient` | `Wad::from_bytes_with_options` lenient on small / medium / large |
| `parse/from_path` | `Wad::from_path` strict on a `tempfile`-backed medium WAD |
| `lump_access` | `Wad::lump(idx)`, `Wad::lump_by_name` (first-match + miss), `Wad::lump_bytes(idx)`, `Wad::lump_data(lump)`, `Wad::lumps()` |
| `map_records` | `parse_records::<T>` for all eight classic types: `Thing`, `Linedef`, `Sidedef`, `Vertex`, `Seg`, `Subsector`, `Node`, `Sector` |
| `freedoom` | `from_bytes` and `lump_by_name` on real Freedoom WAD; skipped if `CRUSTYWAD_FREEDOOM_DIR` is unset |

**`write_ops.rs`** (closes issue #6)

| Benchmark group | Operations |
|-----------------|------------|
| `write/build_strict` | `WadBuilder::build` on small / medium / large; `Throughput::Bytes` |
| `write/build_lenient` | `WadBuilder::build_with_options(lenient)` on small / medium / large |
| `write/roundtrip` | `Wad::from_bytes` → `Wad::to_builder` → `WadBuilder::build` on small / medium / large |
| `write/freedoom_roundtrip` | Same round-trip on real Freedoom WAD; skipped if env var unset |

### Justfile changes

The existing `bench` recipe is updated:

```just
bench:
    cargo bench --all-features
    @echo "Criterion HTML report: target/criterion/report/index.html"

bench-open:
    cargo bench --all-features
    {{ if os() == "macos" { "open" } else if os_family() == "windows" { "explorer" } else { "xdg-open" } }} target/criterion/report/index.html
```

### GitHub Pages migration

**One-time manual prerequisite** (performed by the maintainer before the PR lands):

```bash
# Create an orphan gh-pages branch.
git switch --orphan gh-pages
git commit --allow-empty -m "chore: initialize gh-pages branch"
git push origin gh-pages
git switch main
```

Then in GitHub → Settings → Pages, change the source from **"GitHub Actions"** to
**"Deploy from a branch: `gh-pages`, `/ (root)`"**.

**`pages.yml` rewrite.** The `upload-pages-artifact` / `deploy-pages` steps and their
`pages: write` / `id-token: write` permissions are replaced with a
`JamesIves/github-pages-deploy-action` push step and `contents: write`. A `concurrency` group
serializes concurrent guide and bench pushes:

```yaml
concurrency:
  group: gh-pages
  cancel-in-progress: false   # queue; never cancel a pending Pages deploy
```

The `JamesIves/github-pages-deploy-action` action handles sparse checkout, file copy, and force-push
atomically, preventing partial deploys.

### `bench.yml` workflow

New file `.github/workflows/bench.yml`. Runs on push to `main` and `workflow_dispatch`;
never runs on PRs (benchmark results in shared CI are too noisy to be actionable per-PR).

```yaml
on:
  push:
    branches: [main]
  workflow_dispatch:

concurrency:
  group: gh-pages
  cancel-in-progress: false

jobs:
  bench:
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@v7
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2

      - name: Run benchmarks
        run: |
          cargo bench --all-features -- --output-format bencher 2>&1 | tee bench_output.txt

      - name: Upload Criterion HTML report
        uses: actions/upload-artifact@v4
        with:
          name: criterion-html-report
          path: target/criterion/
          retention-days: 90

      - name: Store benchmark trend data
        uses: benchmark-action/github-action-benchmark@v1
        with:
          tool: cargo
          output-file-path: bench_output.txt
          github-token: ${{ secrets.GITHUB_TOKEN }}
          auto-push: true
          gh-pages-branch: gh-pages
          benchmark-data-dir-path: dev/bench
```

### `cargo-deny` compatibility

`criterion`'s license (Apache-2.0 / MIT dual-licensed) is already on the workspace allow-
list. Any new transitive dependencies introduced by Criterion's dependency graph are reviewed
during the implementation PR as part of the `cargo deny check` step.

## More information

- Criterion book: https://bheisler.github.io/criterion.rs/book/
- `github-action-benchmark`: https://github.com/benchmark-action/github-action-benchmark
- `JamesIves/github-pages-deploy-action`: https://github.com/JamesIves/github-pages-deploy-action
- Related issues: #2 (infra), #3 (read benchmarks), #4 (`lump_by_name`), #6 (write
  benchmarks), #19 (epic), #67 (performance enhancements)
- Related ADR: ADR-0009 (`cargo-fuzz`) — establishes that development tooling may carry
  different MSRV requirements than the published library.
- Revisit the bench workflow trigger schedule if CI minutes become a concern; benchmarks
  could be moved to a weekly schedule while retaining `workflow_dispatch`.
