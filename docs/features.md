# Feature Flags

`crustywad` uses Cargo feature flags to keep the default dependency footprint small while
allowing callers to opt in to additional capabilities.

## Summary

| Feature | Default | Purpose |
|---|---|---|
| [`mmap`](#mmap) | no | Memory-mapped file loading via `memmap2` |
| [`freedoom-tests`](#freedoom-tests) | no | Integration tests against local Freedoom WAD fixtures |

---

## `mmap`

**Enables:** `Wad::from_path_mapped` and `Wad::from_path_mapped_with_options`

**Adds dependency:** [`memmap2`](https://crates.io/crates/memmap2)

Memory-maps the WAD file instead of reading it into a `Vec<u8>`. On large WADs this avoids
a heap allocation equal to the file size and lets the OS page in only the bytes that are
actually accessed. The tradeoff is a small amount of `unsafe` code in `mmap.rs` (the only
`unsafe` in the library crate) to call `memmap2::MmapOptions::map`.

`Wad::from_path` (the non-mapped variant) always reads the whole file into memory regardless
of whether this feature is enabled.

### Usage

```toml
# Cargo.toml
crustywad = { version = "0.1", features = ["mmap"] }
```

```rust
use crustywad::{Wad, ParseOptions};

// Zero-copy load from disk:
let wad = Wad::from_path_mapped("doom.wad")?;

// Zero-copy load with options:
let wad = Wad::from_path_mapped_with_options("doom.wad", ParseOptions::lenient())?;
```

### Platform notes

`memmap2` is supported on all tier-1 Rust targets (Linux, macOS, Windows). Memory-mapped
files are read-only; there is no risk of accidentally writing to the underlying file.

---

## `freedoom-tests`

**Enables:** integration tests in `crates/crustywad/tests/freedoom.rs`

**Adds dependency:** none (test-only fixture files on disk)

Gates optional tests that parse real [Freedoom](https://freedoom.github.io/) WAD files.
These tests require:

1. The Freedoom fixture files to be downloaded locally (see below).
2. The `CRUSTYWAD_FREEDOOM_DIR` environment variable to point at the directory containing them.

If the environment variable is not set the tests skip gracefully — they do not fail.

### Fetching fixtures

```sh
# Default version (configured in tests/fixtures/fetch_freedoom.py):
just fetch-fixtures

# Specific Freedoom release:
just fetch-fixtures version=v0.14.0
```

### Running the tests

```sh
CRUSTYWAD_FREEDOOM_DIR=tests/fixtures/freedoom \
  cargo test --workspace --all-features
```

Or with `just`:

```sh
just test-freedoom
```

### CI

The `freedoom-tests` feature is intentionally **not** enabled in CI by default because the
fixture WADs are not committed to the repository (they are gitignored). They run only when
`CRUSTYWAD_FREEDOOM_DIR` is set, which CI does not do.

---

## Common `cargo` invocations

| Goal | Command |
|---|---|
| Build with all features | `cargo build --workspace --all-features` |
| Build with `mmap` only | `cargo build -p crustywad --features mmap` |
| Test with all features | `cargo test --workspace --all-features` |
| Test with `mmap` only | `cargo test -p crustywad --features mmap` |
| Test with Freedoom fixtures | `CRUSTYWAD_FREEDOOM_DIR=… cargo test --workspace --all-features` |
| Full CI check | `just ci` |

See the [`justfile`](../justfile) for available `just` recipes including feature-specific
aliases.
