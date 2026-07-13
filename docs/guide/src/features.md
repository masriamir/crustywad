# Feature Flags

`crustywad` uses Cargo feature flags to keep the default dependency footprint small while
allowing callers to opt in to additional capabilities.

## Summary

| Feature | Default | Purpose |
|---|---|---|
| [`mmap`](#mmap) | no | Memory-mapped file loading via `memmap2` |
| [`freedoom-tests`](#freedoom-tests) | no | Integration tests against local Freedoom WAD fixtures (auto-fetchable) |
| [`hexen-tests`](#hexen-tests) | no | Integration tests against a local Hexen IWAD (not auto-fetchable) |
| [`doom64-tests`](#doom64-tests) | no | Integration tests against a local Doom 64 IWAD (not auto-fetchable) |
| [`write`](#write) | no | WAD serialization — `WadBuilder`, `WriteError`, `WriteOptions`, `WriteWarning` |

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
crustywad = { version = "0.3.0", features = ["mmap"] }
```

```rust
use crustywad::{Wad, ParseOptions};

// Zero-copy load from disk:
let _wad = Wad::from_path_mapped("doom.wad")?;

// Zero-copy load with options:
let _wad = Wad::from_path_mapped_with_options("doom.wad", ParseOptions::lenient())?;
# Ok::<(), crustywad::ParseError>(())
```

### When to use mmap

Memory-mapped loading is useful for large WADs when you only need to access a subset of
lumps. The OS maps the file into the address space without copying all bytes into heap memory
upfront — pages are faulted in on demand.

For small WADs or when you will access most lumps, `Wad::from_path` (which reads into a
`Vec<u8>`) is equally fast and has simpler lifetime semantics.

The `parse/from_path` benchmark group measures both variants side by side. See the
[Performance](performance.md) page for live throughput data and how to run the benchmarks
locally.

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
just fetch-fixtures version=v0.14.0
```

### Running the tests

```sh
# Using just — defaults CRUSTYWAD_FREEDOOM_DIR to an absolute path under the repo root:
just test-freedoom

# Override the fixture directory:
just test-freedoom dir=/path/to/freedoom

# Or run cargo directly. The path must be ABSOLUTE: cargo sets the test binary's
# working directory to the package root (crates/crustywad), so a relative path
# never resolves and the fixture tests skip silently.
CRUSTYWAD_FREEDOOM_DIR="$PWD/tests/fixtures/freedoom" \
  cargo test -p crustywad --features freedoom-tests
```

### CI

CI runs `cargo test --workspace --all-features`, which enables the `freedoom-tests` feature
flag. The tests skip gracefully when `CRUSTYWAD_FREEDOOM_DIR` is not set — and CI never sets
it because the fixture WADs are gitignored and not downloaded in the standard CI pipeline.

---

## `hexen-tests`

**Enables:** integration tests in `crates/crustywad/tests/hexen.rs`

### Purpose

Gates an optional smoke test that parses a real Hexen IWAD. Unlike Freedoom, Hexen's IWAD
is **not freely redistributable**, so there is no fetch script and no committed fixture —
supply your own copy locally.

### Running the tests

Point `CRUSTYWAD_HEXEN_DIR` at a directory containing `hexen.wad`:

```bash
CRUSTYWAD_HEXEN_DIR=/path/to/hexen \
  cargo test -p crustywad --features hexen-tests
```

The test skips gracefully when `CRUSTYWAD_HEXEN_DIR` is unset or the file is missing.

---

## `doom64-tests`

**Enables:** integration tests in `crates/crustywad/tests/doom64.rs`

### Purpose

Gates an optional smoke test that parses a real Doom 64 IWAD. Like Hexen, the Doom 64 IWAD
is **not freely redistributable** — no fetch script, no committed fixture; supply your own
copy locally.

### Running the tests

Point `CRUSTYWAD_DOOM64_DIR` at a directory containing `doom64.wad`:

```bash
CRUSTYWAD_DOOM64_DIR=/path/to/doom64 \
  cargo test -p crustywad --features doom64-tests
```

The test skips gracefully when `CRUSTYWAD_DOOM64_DIR` is unset or the file is missing.

---

## `write`

**Enables:** `WadBuilder`, `WriteError`, `WriteWarning`, `WriteOptions`, and `Wad::to_builder`

**Adds dependency:** none (uses `binrw` already in the dependency tree)

Adds WAD serialization support. `WadBuilder` accumulates lumps and serializes them to a
`Vec<u8>` in the canonical Doom WAD layout:
`[12-byte header][lump data blobs][16-byte directory entries]`.

### Usage

```toml
# Cargo.toml
crustywad = { version = "0.3.0", features = ["write"] }
```

```rust
use crustywad::{WadBuilder, WadKind};

// Build a new PWAD from scratch:
let bytes = WadBuilder::new(WadKind::Pwad)
    .add_lump("MAP01", b"data")
    .build()
    .unwrap();

assert!(crustywad::Wad::from_bytes(bytes).is_ok());
```

### Round-tripping a parsed WAD

```rust
use crustywad::{Wad, WadBuilder, WadKind};

# let mut source = Vec::new();
# source.extend_from_slice(b"PWAD");
# source.extend_from_slice(&0_i32.to_le_bytes());
# source.extend_from_slice(&12_i32.to_le_bytes());
let wad = Wad::from_bytes(source).unwrap();
let rebuilt = wad.to_builder().build().unwrap();
```

### Validation and error handling

`WadBuilder::build` uses strict mode by default. Use `build_with_options` with
`WriteOptions::lenient()` to collect recoverable issues as `WriteWarning` values instead:

- Names with NUL bytes or non-ASCII bytes always error in both modes.
- Names longer than 8 bytes: strict mode returns `WriteError::NameTooLong`; lenient mode
  truncates and emits `WriteWarning::NameTruncated`.
- `WadKind::Unknown` magic: strict mode returns `WriteError::UnknownMagicStrict`; lenient
  mode writes the raw 4-byte magic.

---

## Common `cargo` invocations

| Goal | Command |
|---|---|
| Build with all features | `cargo build --workspace --all-features` |
| Build with `mmap` only | `cargo build -p crustywad --features mmap` |
| Test with all features | `cargo test --workspace --all-features` |
| Test with `mmap` only | `cargo test -p crustywad --features mmap` |
| Test with Freedoom fixtures | `CRUSTYWAD_FREEDOOM_DIR=… cargo test -p crustywad --features freedoom-tests` |
| Test with Hexen fixture | `CRUSTYWAD_HEXEN_DIR=… cargo test -p crustywad --features hexen-tests` |
| Test with Doom 64 fixture | `CRUSTYWAD_DOOM64_DIR=… cargo test -p crustywad --features doom64-tests` |
| Build with `write` | `cargo build -p crustywad --features write` |
| Test with `write` | `cargo test -p crustywad --features write` |
| Full CI check | `just ci` |

See the [`justfile`](https://github.com/masriamir/crustywad/blob/main/justfile) for
available `just` recipes including feature-specific aliases.
