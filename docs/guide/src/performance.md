# Performance

`crustywad` tracks performance via [Criterion](https://bheisler.github.io/criterion.rs/book/)
micro-benchmarks that run in CI on every push to `main`. Results are published as interactive
trend charts so regressions are visible before they reach a release.

---

## Live benchmark charts

**[crustywad.dev/dev/bench/](https://crustywad.dev/dev/bench/)**

The chart page shows throughput and latency trends over time for every benchmark group. Each
data point corresponds to a push to `main`.

### Benchmark groups

| Group | What is measured |
|---|---|
| `parse/from_bytes_strict` | `Wad::from_bytes` — strict mode — on small (10 × 256 B), medium (100 × 4 KiB), and large (1 000 × 16 KiB) synthetic WADs |
| `parse/from_bytes_lenient` | `Wad::from_bytes_with_options` — lenient mode — on small (10 × 256 B), medium (100 × 4 KiB), and large (1 000 × 16 KiB) synthetic WADs |
| `parse/from_path` | `Wad::from_path` and `Wad::from_path_with_options` on a tempfile-backed medium WAD; mmap variants when the `mmap` feature is enabled |
| `lump_access` | `lump(idx)`, `lump_by_name` (first-match and worst-case last-match), `lump_bytes`, `lump_data`, `lumps().iter().count()`, `clone`, `into_bytes` |
| `map_records` | `parse_records::<T>` for all eight classic map-record types (`Thing`, `Linedef`, `Sidedef`, `Vertex`, `Seg`, `Subsector`, `Node`, `Sector`) against 1 000 records each |
| `write/build_strict` | `WadBuilder::build` — strict mode — on small / medium / large synthetic WADs |
| `write/build_lenient` | `WadBuilder::build_with_options(&WriteOptions::lenient())` on the same sizes |
| `write/build_from_scratch` | `WadBuilder` populated entirely at runtime (10 or 100 lumps) |
| `write/roundtrip` | `Wad::from_bytes` → `Wad::to_builder` → `WadBuilder::build` end-to-end |
| `freedoom` | `from_bytes`, `lump_by_name` (hit and miss), and `roundtrip` on real Freedoom WAD files; skipped when `CRUSTYWAD_FREEDOOM_DIR` is not set |

Throughput groups report **MB/s** via `Throughput::Bytes` so results scale naturally with
input size. Latency groups report **ns/iter**.

---

## Running benchmarks locally

### Prerequisites

Benchmarks are part of the standard workspace and require a stable Rust toolchain. The
`just bench` / `just bench-open` recipes also require
[`just`](https://github.com/casey/just); if you prefer not to install it, use the
equivalent `cargo` command directly:

```sh
cargo bench --all-features --benches
```

### Quick run

```sh
# Run all benchmarks and print the path to the HTML report:
just bench

# Run benchmarks and open the HTML report in the default browser:
just bench-open
```

`just bench-open` uses `open` on macOS, `xdg-open` on Linux, and `explorer` on Windows.

The Criterion HTML report is written to:

```
target/criterion/report/index.html
```

Open it to see per-benchmark violin plots, regression detection, and historical comparisons
between the last two runs on your machine.

### Running a specific group

Pass a filter after `--` to run only matching benchmarks:

```sh
# Only the lump_access group:
cargo bench --all-features --benches -- lump_access

# Only the write/roundtrip benchmarks:
cargo bench --all-features --benches -- "write/roundtrip"
```

### Freedoom real-world benchmarks

The `freedoom` group is skipped by default. To enable it, fetch the fixtures first and point
the environment variable at the directory:

```sh
just fetch-fixtures                         # downloads freedoom1.wad / freedoom2.wad
CRUSTYWAD_FREEDOOM_DIR=tests/fixtures/freedoom just bench
```

---

## CI benchmark workflow

The `bench.yml` workflow runs on every push to `main` and on `workflow_dispatch`. It is
**non-blocking** — `fail-on-alert: false` ensures benchmark regressions never fail the run
or block a merge.

Each run:

1. Compiles and runs all Criterion bench targets with `--output-format bencher`.
2. Uploads the Criterion HTML report as a downloadable GitHub Actions artifact
   (90-day retention) named `criterion-html-report`.
3. Appends trend data to the `gh-pages` branch at `dev/bench/` via
   [`github-action-benchmark`](https://github.com/benchmark-action/github-action-benchmark),
   which powers the chart page at `crustywad.dev/dev/bench/`.

The bench workflow and the guide deploy workflow share a `concurrency: group: gh-pages` so
they never write to the branch simultaneously.

To trigger a benchmark run manually without pushing to `main`:

```sh
gh workflow run bench.yml
```
