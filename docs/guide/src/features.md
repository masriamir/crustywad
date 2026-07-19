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
| [`sweep-tests`](#sweep-tests) | no | Sweep test that assembles every map of every WAD in a local collection (not auto-fetchable) |
| [`write`](#write) | no | WAD serialization — `WadBuilder`, `WriteError`, `WriteOptions`, `WriteWarning` |
| [`nodebuild`](#nodebuild) | no | Clean-room node-lump builders (enables `write`) — `map::build`, `build_reject`, `MapReject::to_lump_bytes` |
| [`doom64-gfx`](#doom64-gfx) | no | Doom 64 PNG texture/sprite decoding via `png` — `Doom64Png`, capped by `Limits::max_decoded_pixels` |

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
crustywad = { version = "0.6.0", features = ["mmap"] }
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

## `sweep-tests`

**Enables:** the integration test in `crates/crustywad/tests/sweep.rs`

### Purpose

Gates the retail-WAD sweep: for every WAD file in a caller-supplied directory, it parses
the container strictly, assembles **every** map group in **both** strictness modes
(reading Doom 64 nested-WAD maps through `read_doom64_map`), and asserts zero errors and
zero warnings throughout — no allowlist. It is the regression net for the map read path
against real retail data. Retail WADs are **not freely redistributable** — no fetch
script, no committed fixture; supply your own collection locally.

### Running the tests

Point `CRUSTYWAD_SWEEP_DIR` at a directory of WAD files. **Use an absolute path** —
cargo runs the test binary with its CWD at the package root (`crates/crustywad`), so a
relative path resolves against that directory rather than the workspace root and can miss
(or accidentally hit the wrong) collection, leaving only a stderr skip note:

```bash
CRUSTYWAD_SWEEP_DIR=/path/to/wads \
  cargo test -p crustywad --features sweep-tests --test sweep
```

Or use the `just` recipe, which defaults to the repository's gitignored `RETAIL/`
directory as an absolute path (an explicit `dir=` override should also be absolute):

```bash
just test-sweep              # sweeps ./RETAIL
just test-sweep dir=/path/to/wads
```

The test skips gracefully when `CRUSTYWAD_SWEEP_DIR` is unset or contains no WAD files.

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
crustywad = { version = "0.6.0", features = ["write"] }
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

## `nodebuild`

**Enables:** the `map::build` module — `NodeBuildOptions`, `NodeBuildError`, `NodeBuildWarning`,
`build_reject`, and `nodebuild`-gated `to_lump_bytes` serializers on the read-side lump types
(`MapReject` first; `MapBlockmap` and the classic BSP builders follow, ADR-0024 §9)

**Adds dependency:** none — implies `write`

Clean-room BLOCKMAP, REJECT, and (in later stages) classic BSP generation, turning an
assembled `Map` into engine-playable node lumps (ADR-0024). It fulfills the revisit condition
`add_doom_map` left open: that path deliberately emits zero-length `SEGS`/`SSECTORS`/`NODES`/
`REJECT`/`BLOCKMAP` with an always-on `DoomWriteWarning::NodesNotBuilt`, whereas the
`nodebuild` builders produce those lumps for real. Coordinate narrowing is shared with the
write path (ADR-0024 §3), so a builder operates on exactly the `i16` geometry the engine reads.

Stage 1 ships `build_reject`, which returns the correctly-sized all-zeros `REJECT`
(`ceil(sectors² / 8)` bytes). An all-clear table pre-rejects no line of sight, which is always
engine-correct — it is what `zdbsp` itself emits.

### Usage

```toml
# Cargo.toml
crustywad = { version = "0.6.0", features = ["nodebuild"] }
```

Or with `cargo add`:

```sh
cargo add crustywad --features nodebuild
```

```rust,ignore
use crustywad::map::build::build_reject;

# fn run(map: &crustywad::map::Map) {
let reject = build_reject(map);
let bytes = reject.to_lump_bytes(); // ceil(sectors² / 8) all-zero bytes
# let _ = bytes;
# }
```

---

## `doom64-gfx`

**Enables:** `Doom64Png` decoding of Doom 64's PNG texture/sprite lumps via the `png` crate
(indexed pixels + palette rows + `grAb` offsets, capped by `Limits::max_decoded_pixels`)

**Adds dependency:** [`png`](https://crates.io/crates/png)

Doom 64's PC port stores its texture and sprite lumps as standard palette-indexed PNG files
rather than the classic picture format (ADR-0022 §5) — a different lump family from the rest
of `crustywad::gfx`, decoded separately behind this feature rather than unconditionally in
the core crate. `Doom64Png::decode` parses the indexed pixel data, the embedded `PLTE` (up to
16 rows of 16 colors serving runtime palette variants), optional per-index `tRNS` alpha, and
sprite draw offsets from a private `grAb` chunk (a big-endian `i32` pair, the `ZDoom`
convention). The declared `width × height` is checked against
[`Limits::max_decoded_pixels`](https://docs.rs/crustywad/latest/crustywad/struct.Limits.html#structfield.max_decoded_pixels)
— and a 65535-per-side cap — before any pixel buffer is allocated, fired in **both**
strictness modes (the same DoS-cap exception `TextureSet::compose`'s composite limit uses).

### Usage

```toml
# Cargo.toml
crustywad = { version = "0.6.0", features = ["doom64-gfx"] }
```

Or with `cargo add`:

```sh
cargo add crustywad --features doom64-gfx
```

```rust
use crustywad::gfx::Doom64Png;
use crustywad::ParseOptions;

# fn run(png_bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
let img = Doom64Png::decode(png_bytes, &ParseOptions::strict())?;

// Tier-2 view: palette indices plus a coverage mask.
let indexed = img.to_indexed();

// Full-color view: the PNG's own PLTE/tRNS, not `indexed`'s palette + boolean
// mask — Doom 64 PNGs carry per-index alpha that a boolean mask can't represent.
let rgba = img.to_rgba();
let _ = (indexed, rgba);
# Ok(())
# }
```

### Strictness and limits

`Doom64Png::decode` follows the same `ParseOptions::strict()`/`ParseOptions::lenient()`
contract as the rest of `crustywad::gfx`: strict mode returns the first `GfxError`
encountered; lenient mode recovers with a best-effort value and records the matching
`GfxWarning`. `Limits::max_decoded_pixels` (default `1 << 24`) bounds the pixel buffer a
single `decode` call allocates and is enforced in both modes, ahead of any allocation — see
the [Graphics](graphics.md#doom-64-graphics) guide page for how this fits alongside the rest
of `crustywad::gfx`.

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
| Sweep a local WAD collection | `CRUSTYWAD_SWEEP_DIR=… cargo test -p crustywad --features sweep-tests` |
| Build with `write` | `cargo build -p crustywad --features write` |
| Test with `write` | `cargo test -p crustywad --features write` |
| Build with `nodebuild` | `cargo build -p crustywad --features nodebuild` |
| Test with `nodebuild` | `cargo test -p crustywad --features nodebuild` |
| Build with `doom64-gfx` | `cargo build -p crustywad --features doom64-gfx` |
| Test with `doom64-gfx` | `cargo test -p crustywad --features doom64-gfx` |
| Full CI check | `just ci` |

See the [`justfile`](https://github.com/masriamir/crustywad/blob/main/justfile) for
available `just` recipes including feature-specific aliases.
