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

### When to use mmap

Memory-mapped loading is useful for large WADs when you only need to access a subset of
lumps. The OS maps the file into the address space without copying all bytes into heap memory
upfront — pages are faulted in on demand.

For small WADs or when you will access most lumps, `Wad::from_path` (which reads into a
`Vec<u8>`) is equally fast and has simpler lifetime semantics.

### Platform notes

`memmap2` is supported on all tier-1 Rust targets (Linux, macOS, Windows). Memory-mapped
files are read-only; there is no risk of accidentally writing to the underlying file.

**Warning:** the WAD file must not be truncated or replaced by another process while the
`Wad` is alive. On Unix, truncation from another process triggers a `SIGBUS` on the next
lump data access, which will abort the process. On Windows the mapping prevents truncation
but concurrent writes by another process may expose inconsistent data. Use `Wad::from_path`
if the file may be modified externally while in use.

---

## `freedoom-tests`

**Enables:** integration tests in `crates/crustywad/tests/freedoom.rs`

**Adds dependency:** none (test-only fixture files on disk)

Gates optional tests that parse real [Freedoom](https://freedoom.github.io/) WAD files.
Tests skip gracefully when `CRUSTYWAD_FREEDOOM_DIR` is not set or when the expected WAD files
are not present in that directory — they do not fail.

### Fetching fixtures

```sh
# Default version (configured in tests/fixtures/fetch_freedoom.py):
just fetch-fixtures

# Specific Freedoom release:
just fetch-fixtures version=v0.13.0
```

### Running the tests

```sh
# Using just — defaults CRUSTYWAD_FREEDOOM_DIR to tests/fixtures/freedoom:
just test-freedoom

# Override the fixture directory:
just test-freedoom dir=/path/to/freedoom

# Or run cargo directly:
CRUSTYWAD_FREEDOOM_DIR=tests/fixtures/freedoom \
  cargo test -p crustywad --features freedoom-tests
```

### CI

CI runs `cargo test --workspace --all-features`, which enables the `freedoom-tests` feature
flag. The tests skip gracefully when `CRUSTYWAD_FREEDOOM_DIR` is not set — and CI never sets
it because the fixture WADs are gitignored and not downloaded in the standard CI pipeline.

---

## Common `cargo` invocations

| Goal | Command |
|---|---|
| Build with all features | `cargo build --workspace --all-features` |
| Build with `mmap` only | `cargo build -p crustywad --features mmap` |
| Test with all features | `cargo test --workspace --all-features` |
| Test with `mmap` only | `cargo test -p crustywad --features mmap` |
| Test with Freedoom fixtures | `CRUSTYWAD_FREEDOOM_DIR=… cargo test -p crustywad --features freedoom-tests` |
| Full CI check | `just ci` |

See the [`justfile`](https://github.com/masriamir/crustywad/blob/main/justfile) for
available `just` recipes including feature-specific aliases.
