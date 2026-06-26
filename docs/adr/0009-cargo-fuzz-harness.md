# ADR-0009: cargo-fuzz harness for WAD parser

- **Status:** Proposed
- **Date:** 2026-06-14
- **Deciders:** @masriamir
- **Tracking issue:** https://github.com/masriamir/crustywad/issues/44

## Context

`crustywad` accepts untrusted input via `Wad::from_bytes` and
`Wad::from_bytes_with_options` (both take `impl Into<Vec<u8>>` and take
ownership of the data) and `parse_records::<T>` (which borrows `&[u8]`). The
`from_bytes`
variants decode the 12-byte header and walk the variable-length directory,
validating and clamping offsets and ranges; they do not read lump payload bytes.
`parse_records::<T>` reads lump payload bytes and uses `binrw` to deserialize
typed map records. Malformed or adversarial input across either path could
trigger unexpected panics or — should future milestones introduce `unsafe` for
SIMD or direct I/O — undefined behavior. Coverage-guided fuzzing is the most
effective way to discover such
cases before they reach production.

Three mature Rust fuzzing engines exist, each with a different toolchain
requirement:

| Engine | Crate | Toolchain | Sanitizer support |
|---|---|---|---|
| libFuzzer | `cargo-fuzz` | **nightly only** | ASan, UBSan, MSan |
| AFL++ | `cargo-afl` | stable or nightly | ASan, UBSan |
| Honggfuzz | `honggfuzz-rs` | stable or nightly | ASan |

The critical constraint is the workspace MSRV of **Rust 1.85.0 (stable)**. The
library crate must continue to compile, test, and lint on stable 1.85.0. However,
the fuzzing infrastructure is an *out-of-tree development tool*, not a build
dependency. This distinction is key: just as benchmarks may depend on `criterion`
which does not compile on MSRV stable, a fuzz crate can depend on nightly-only
capabilities without changing the MSRV of the published library.

cargo-fuzz stores its targets under a top-level `fuzz/` directory. The `fuzz/`
directory is a separate Cargo package with its own `[workspace]` declaration,
which keeps it out of the root `[workspace]` in `Cargo.toml`. The fuzz package
is therefore invisible to `cargo build --workspace`, `cargo test --workspace`,
and `cargo deny check`, and does not affect the published crate.

## Decision

### 1. Fuzzing engine: cargo-fuzz (libFuzzer, nightly)

We choose **cargo-fuzz** for the following reasons:

- libFuzzer is the de-facto standard in the Rust security ecosystem and is
  required for Google OSS-Fuzz, which is a viable future integration path.
- The fuzz package is an isolated Cargo package under `fuzz/` (with its own
  `[workspace]` declaration). It carries its own `rust-toolchain.toml` pinning a
  nightly channel (e.g. `channel = "nightly"`). Because `cargo fuzz run` always
  builds from within the `fuzz/` subtree, this toolchain file is picked up
  automatically. Alternatively, `cargo +nightly fuzz run <target>` can be used
  from the repo root without relying on the toolchain file. The main workspace
  remains on stable 1.85.0.
- `cargo fuzz run` is invoked only by developers who have explicitly installed the
  nightly toolchain. It is never part of `just ci`, `cargo test --workspace`, or
  any step run during a normal build.
- AFL++ (`cargo-afl`) would preserve stable compatibility but adds an external C
  process dependency and requires a custom build step to instrument the binary.
  The added complexity is not justified by the stability benefit given the clean
  workspace isolation available with cargo-fuzz.
- honggfuzz-rs requires the honggfuzz binary to be installed separately and offers
  fewer sanitizer combinations than libFuzzer.

The `justfile` `fuzz` recipe will be updated to print a human-readable notice
explaining the nightly requirement and the `cargo fuzz run <target>` invocation,
replacing the existing placeholder message.

### 2. Fuzz targets

Three targets are defined under `fuzz/fuzz_targets/`:

| Target file | Entry point | Goal |
|---|---|---|
| `fuzz_wad_strict.rs` | `Wad::from_bytes(data.to_vec())` | No panic on arbitrary bytes; `Err(ParseError)` is always acceptable |
| `fuzz_wad_lenient.rs` | `Wad::from_bytes_with_options(data.to_vec(), ParseOptions::lenient())` | Same goal; lenient path exercises warning accumulation and best-effort clamping |
| `fuzz_parse_records_thing.rs` | `parse_records::<Thing>(data)` | No panic; `Err(MapParseError)` acceptable; exercises `binrw` little-endian fixed-record parsing |

The oracle in every harness is:

```rust
let _ = black_box(/* call */);
// If we reach here without a panic, the harness passes.
```

`ParseError` and `MapParseError` returns are explicitly allowed — they represent
correct rejection of invalid input, not bugs. Panics, assertion failures, and
any kind of UB (caught by sanitizers) are failures.

`fuzz_wad_lenient.rs` also asserts that `wad.warnings().len()` does not exceed
`wad.lump_count() * 5 + 5` — a loose upper bound derived from the five per-lump
warning sites (`NegativeValue` for `filepos`, `NegativeValue` for `size`,
`NonAsciiName`, `Overflow` for the lump range, and `OutOfBounds` for lump data)
plus up to five header/directory-level warnings emitted before the per-lump loop
(`InvalidMagic`, `NegativeValue` for `numlumps`, `NegativeValue` for
`infotableofs`, `Overflow` for directory length, and `OutOfBounds` for directory
range) — to guard against unbounded warning vector growth.

Additional targets for `parse_records::<Linedef>`, `Sidedef`, `Vertex`, and so
on can be added later without any structural change.

### 3. CI integration

Fuzzing runs are **not** added to `ci.yml`. Continuous fuzzing requires hours to
days of wall-clock time and produces its own corpus of interesting inputs; gating
PRs on a fuzzing run would be impractical.

Instead, a separate workflow `.github/workflows/fuzz.yml` is introduced with the
following properties:

- Triggered by `workflow_dispatch` (manual) and `schedule` (weekly, off-peak).
- Runs only on Linux (`ubuntu-latest`) to keep nightly toolchain maintenance to
  one platform.
- Installs the nightly toolchain via `dtolnay/rust-toolchain@nightly` and
  `cargo install cargo-fuzz`.
- Runs each target with a 60-second wall-clock limit:
  `cargo fuzz run <target> -- -max_total_time=60`.
- Uploads any crash artifacts (files written to `fuzz/artifacts/<target>/`) as
  GitHub Actions workflow artifacts on failure.
- Does **not** block `main` merges; failures produce a GitHub Actions notification
  but do not fail required status checks.

This keeps fuzzing separate from the normal quality gate while ensuring regressions
are detected within a week without requiring external infrastructure.

OSS-Fuzz integration is deferred to a later milestone. If it is adopted, the same
`fuzz/` workspace and target files are directly reusable — OSS-Fuzz supports
cargo-fuzz out of the box.

### 4. Corpus management

Seed inputs are committed under `fuzz/corpus/<target>/`:

- `fuzz/corpus/fuzz_wad_strict/` — a minimal valid IWAD, a minimal valid PWAD, a
  zero-byte file (triggers `ParseError::Header`), and a 12-byte file containing
  only a complete WAD header that claims one lump but provides no directory bytes
  (triggers `ParseError::Directory`).
- `fuzz/corpus/fuzz_wad_lenient/` — same four seeds; the lenient path diverges
  at the strictness branch inside `parse_bytes`.
- `fuzz/corpus/fuzz_parse_records_thing/` — an empty slice, a single valid
  `Thing` record (10 bytes), and a 9-byte slice (triggers `TrailingBytes`).

Interesting inputs discovered during a fuzzing run (inputs that increase
coverage but do not crash) are written by libFuzzer to the corpus directory
automatically. Those files are **gitignored** via an entry in `.gitignore`:

```
fuzz/corpus/*/[0-9a-f][0-9a-f]*
```

This pattern excludes the hex-named auto-generated files that libFuzzer creates
while preserving the human-authored seed files, which use descriptive names
(`minimal_iwad`, `truncated_header`, etc.).

Crash artifacts (`fuzz/artifacts/`) are fully gitignored. If a crash is found,
the developer reproduces it with `cargo fuzz run <target> fuzz/artifacts/<target>/<file>`,
writes a regression test in the appropriate integration test file, and removes
the artifact.

## Consequences

- **MSRV is unaffected.** The fuzz package is a separate Cargo package with its
  own `[workspace]` declaration and `rust-toolchain.toml`. Running
  `cargo build --workspace` or `just ci` from the repo root never touches the
  fuzz crate.
- **cargo-deny is unaffected.** The fuzz package is not a member of the root
  workspace, so `cargo deny check` does not scan its dependencies. The
  `cargo-fuzz` and `libfuzzer-sys` crates do not appear in the workspace
  dependency graph.
- **libFuzzer requires nightly.** Any developer who wants to run fuzz targets
  locally must install a nightly toolchain (`rustup toolchain install nightly`).
  This is a one-time setup step. The CONTRIBUTING guide and the `justfile`
  notice should document this clearly.
- **The weekly CI schedule means a regression may go undetected for up to seven
  days.** This is acceptable for a design-spike stage. If future milestones
  introduce write support or complex graph assembly, the schedule can be tightened
  or OSS-Fuzz can provide continuous coverage.
- **The three initial targets cover the two primary public parse entry points and
  one map-record type.** Adding more `parse_records::<T>` targets is mechanical
  and can be done incrementally as new map record types are stabilized.
- **Corpus seed files are small** (< 100 bytes each) and committed, so new
  contributors get a working starting point without downloading fixtures.
