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
| [`guide-doctests`](#guide-doctests) | no | **Internal, CI-only.** Compiles this guide's Rust code samples as crate doctests (enabled by `--all-features`); not a runtime capability |
| [`write`](#write) | no | WAD serialization — `WadBuilder`, `WriteError`, `WriteOptions`, `WriteWarning` |
| [`nodebuild`](#nodebuild) | no | Clean-room node-lump builders (enables `write`) — `map::build`, `build_blockmap`, `build_reject`, `build_nodes` (the classic BSP pass: `SEGS`/`SSECTORS`/`NODES`), the `add_doom_map_with_nodes` engine-playable one-shot, and the `to_lump_bytes` serializers; also emits the `XNOD`/`ZNOD` non-GL stream via `NodeFormat`, plus the GL `XGLN`/`XGL2`/`XGL3` streams (and their `Z*` twins with `extended-nodes-zlib`), with `NodeFormat::Gl` auto-selecting the minimal dialect, via `build_gl_nodes` (ADR-0025, ADR-0026); powers `cwad convert --nodes` |
| [`doom64-gfx`](#doom64-gfx) | no | Doom 64 PNG texture/sprite decoding via `png` — `Doom64Png`, capped by `Limits::max_decoded_pixels` |
| [`extended-nodes-zlib`](#extended-nodes-zlib) | no | Decode the zlib-compressed ZDoom extended node formats (`ZNOD`/`ZGLN`/`ZGL2`/`ZGL3`) via `miniz_oxide`, bounded by `Limits::max_decoded_node_bytes`; with `nodebuild` also enabled, also powers the `nodebuild` `ZNOD` and `Z*` GL writers |

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
crustywad = { version = "0.9.0", features = ["mmap"] }
```

```rust,no_run
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

## `guide-doctests`

**Enables:** compiling this guide's own Rust code samples as crate doctests
(`crates/crustywad/src/guide_doctests.rs`)

**Adds dependency:** none

Internal, CI-only. The harness pulls each guide page into the crate via
`#[doc = include_str!(...)]` so that `cargo test --doc --all-features` compiles
(and runs, where not `no_run`) every ` ```rust ` block the guide presents as
real code — catching API drift in a sample before it ships. It is **not a
runtime capability**; a library consumer never needs it.

The module is gated `cfg(all(doctest, feature = "guide-doctests",
has_guide_sources))`. `build.rs` sets `has_guide_sources` only when the
repo-level `docs/guide/src/` files exist, so enabling the feature outside the
source workspace (e.g. on the packaged crate, where those files are absent) is a
graceful no-op rather than a missing-file compile error. CI runs it via the
existing `cargo test --workspace --all-features`; `just guide-test` runs it
locally.

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
crustywad = { version = "0.9.0", features = ["write"] }
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
`build_blockmap`, `build_reject`, `build_nodes` (the classic BSP pass),
`add_doom_map_with_nodes` (the engine-playable one-shot), and the
`nodebuild`-gated `to_lump_bytes` serializers on the read-side lump types (`MapBlockmap`,
`MapReject`, and `BuiltNodes`) — plus `BuiltNodes::to_extended_lump_bytes`, which serializes an
`XNOD`/`ZNOD` `ZDoom` extended-node stream instead of the classic three-lump layout

**Adds dependency:** none — implies `write`

Clean-room BLOCKMAP, REJECT, and classic BSP (`SEGS`/`SSECTORS`/`NODES`) generation from an
assembled `Map` (ADR-0024) — together the full set of node lumps a vanilla engine needs. It
fulfills the revisit condition `add_doom_map` left open: that path deliberately emits
zero-length `SEGS`/`SSECTORS`/`NODES`/`REJECT`/`BLOCKMAP` with an always-on
`DoomWriteWarning::NodesNotBuilt`, whereas the `nodebuild` builders produce those lumps for
real. Coordinate narrowing is shared with the write path (ADR-0024 §3), so a builder
operates on exactly the `i16` geometry the engine reads.

`build_reject` returns the correctly-sized all-zeros `REJECT` (`ceil(sectors² / 8)` bytes) —
an all-clear table pre-rejects no line of sight, which is always engine-correct and is what
`zdbsp` itself emits. `build_blockmap` builds the packed 128-unit-grid `BLOCKMAP`
(deduplicated blocklists, strict/lenient offset-ceiling policy per ADR-0024 §5).
`build_nodes` is the classic BSP pass: it partitions the map on seg lines into a
deterministic `SEGS`/`SSECTORS`/`NODES` tree (`BuiltNodes`), narrowing through the same
write-path pass. It is validated against the full retail collection — 551 classic maps build
clean, save for the mixed-sector fan (two sectors meeting at a bare corner vertex, which no
seg line can separate): strict `build_nodes` rejects such a map, and lenient accepts the leaf
with a `NodeBuildWarning::MixedSectorSubsector` — the exact engine-tolerated output the retail
masters themselves ship (ADR-0024 §7 amendment, 2026-07-19).

The `add_doom_map_with_nodes` one-shot bundles all three builders (plus the five
data lumps) into a single call that adds a complete, engine-playable map group to
a `WadBuilder` — the same path `cwad convert --to doom --nodes` runs. See the
[Building nodes](building-nodes.md) guide page for when you need built nodes, the
tolerated mixed-sector fan, and when GL/extended nodes still call for an external
tool.

`NodeBuildOptions::format` (a `NodeFormat`, ADR-0025 §Amendment #323) selects the on-disk node
encoding `build_nodes`/`add_doom_map_with_nodes` target: `NodeFormat::Classic` (the default,
unchanged from above) writes the vanilla `SEGS`/`SSECTORS`/`NODES` lumps; `NodeFormat::Xnod` (or,
with `extended-nodes-zlib`, `NodeFormat::Znod`) instead serializes a single `ZDoom` non-GL
extended-node stream in `NODES` via `BuiltNodes::to_extended_lump_bytes`, leaving `SEGS`/
`SSECTORS` empty. The extended formats widen the subsector/node/seg/vertex ceilings from the
vanilla 15/16-bit limits to a 31-bit structural cap, so a past-vanilla map can serialize — though
a seg's linedef reference stays a 16-bit field in the non-GL `XNOD`/`ZNOD` streams, so a map with
more than 65,536 linedefs is unrepresentable *there*. The GL formats lift that in stages:
`build_gl_nodes` (and `add_doom_map_with_nodes`) emit an `XGLN`, `XGL2`, or `XGL3` stream (or
their zlib twins `ZGLN`/`ZGL2`/`ZGL3`) via `BuiltGlNodes::to_extended_lump_bytes`, carried in
`SSECTORS`; `XGLN` keeps a 16-bit seg linedef like the non-GL streams, `XGL2` widens it to `u32`,
and `XGL3` additionally allows fractional or out-of-`i16`-range node partitions.
`NodeFormat::Gl`/`Zgl` auto-select the minimal dialect that fits the map, so callers who don't
need a specific dialect can request `Gl` and get the smallest stream that round-trips it.
`cwad`'s CLI does not yet expose GL emission (#366).

### Usage

```toml
# Cargo.toml
crustywad = { version = "0.9.0", features = ["nodebuild"] }
```

Or with `cargo add`:

```sh
cargo add crustywad --features nodebuild
```

```rust
use crustywad::map::build::{NodeBuildOptions, build_blockmap, build_nodes, build_reject};
use crustywad::map::write_doom_map;
use crustywad::{WadBuilder, WadKind, WriteOptions};

# fn run(map: &crustywad::map::Map) -> Result<(), Box<dyn std::error::Error>> {
let reject = build_reject(map); // infallible: ceil(sectors² / 8) all-zero bytes
let (blockmap, _warnings) = build_blockmap(map, &NodeBuildOptions::strict())?;

// The classic BSP pass: SEGS/SSECTORS/NODES. Lenient tolerates the mixed-sector
// fan the retail masters ship (ADR-0024 §7 amendment); strict rejects it.
let (nodes, _warnings) = build_nodes(map, &NodeBuildOptions::lenient())?;
let node_lumps = nodes.to_lump_bytes()?;

// The five data lumps. When the BSP pass splits segs it creates new vertices;
// `split_vertexes` MUST be appended to VERTEXES or the segs' vertex indices
// (which address the map's vertices followed by the split ones) dangle.
let (mut data, _warnings) = write_doom_map(map, &WriteOptions::strict())?;
data.vertexes.extend_from_slice(&node_lumps.split_vertexes);

let mut builder = WadBuilder::new(WadKind::Pwad);
builder
    .add_lump("MAP01", b"")
    .add_lump("THINGS", data.things)
    .add_lump("LINEDEFS", data.linedefs)
    .add_lump("SIDEDEFS", data.sidedefs)
    .add_lump("VERTEXES", data.vertexes) // map vertices + split vertices
    .add_lump("SEGS", node_lumps.segs)
    .add_lump("SSECTORS", node_lumps.ssectors)
    .add_lump("NODES", node_lumps.nodes)
    .add_lump("SECTORS", data.sectors)
    .add_lump("REJECT", reject.to_lump_bytes())
    .add_lump("BLOCKMAP", blockmap.to_lump_bytes()?);
# let _ = builder;
# Ok(())
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
crustywad = { version = "0.9.0", features = ["doom64-gfx"] }
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

## `extended-nodes-zlib`

**Enables:** reading the zlib-compressed ZDoom extended node formats
(`ZNOD`/`ZGLN`/`ZGL2`/`ZGL3`) — the compressed twins of the uncompressed
`XNOD`/`XGLN`/`XGL2`/`XGL3` dialects — by inflating each to its uncompressed body and
decoding it through the same parser

**Adds dependency:** [`miniz_oxide`](https://crates.io/crates/miniz_oxide)

ZDoom's node builders (ZDBSP, GDBSP) can write the extended node data either raw (`X*`, read
unconditionally since ADR-0025 §4, #326) or zlib-compressed (`Z*`). A compressed lump is
`[4-byte plaintext tag][zlib RFC1950 stream]`; with this feature on, the assembler skips the
tag, inflates the remaining bytes, and feeds the result to the *same* decoder its uncompressed
twin uses — so a `ZNOD` lump yields BSP arenas byte-identical to the `XNOD` twin's. The
inflater is the pure-Rust [`miniz_oxide`](https://crates.io/crates/miniz_oxide) (no C
dependency), used through its length-limited entry point so the decompressor stops at the cap
rather than materializing an unbounded buffer from a malicious "zip bomb". Off by default so
the core build pulls in no decompressor. This covers both the binary `NODES`/`SSECTORS` seam
and the UDMF `ZNODES` lump.

With the feature **off**, a recognized `Z*` signature keeps the extended-encoding gate: strict
mode returns `MapAssembleError::UnsupportedNodeEncoding`, lenient mode skips the BSP arenas and
records a warning — the geometry still assembles.

This feature is unrelated to two other node formats that decode as **always-on core** (no
feature flag, since neither needs a decompressor): `DeePBSP` v4 (`xNd4`) and classic GL node
lumps (`GL_VERT`/`GL_SEGS`/`GL_SSECT`/`GL_NODES`) — see
[Classic GL nodes](map-records.md#classic-gl-nodes) in the map-records guide.

With `nodebuild` also enabled, this feature gates the write side too: `NodeFormat::Znod`
(ADR-0025 §Amendment #323) and its GL twins `NodeFormat::Zgln`/`Zgl2`/`Zgl3`/`Zgl` (ADR-0026
#364, #365) only exist as variants when `extended-nodes-zlib` is on. It powers both the `ZNOD`
and `Z*` GL writers: `BuiltNodes::to_extended_lump_bytes(_, compressed: true)` compresses the
`XNOD` body, and `BuiltGlNodes::to_extended_lump_bytes(_, format)` compresses the selected GL
dialect's body for `Zgln`/`Zgl2`/`Zgl3` (and the auto `Zgl`), each with
`miniz_oxide::deflate::compress_to_vec_zlib`
before prepending the matching four-byte tag. Requesting compressed output without this feature
returns `NodeBuildError::CompressionUnavailable` rather than panicking.

### Usage

```toml
# Cargo.toml
crustywad = { version = "0.9.0", features = ["extended-nodes-zlib"] }
```

Or with `cargo add`:

```sh
cargo add crustywad --features extended-nodes-zlib
```

Decoding is transparent — the compressed lump is inflated and decoded during normal map
assembly:

```rust
use crustywad::map::Map;
use crustywad::{ParseOptions, Wad};

# fn run(wad_bytes: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
let wad = Wad::from_bytes(wad_bytes)?;
let group = wad.map_group("MAP01").expect("MAP01");
// With `extended-nodes-zlib`, a compressed `ZNOD`/`ZGL*` node lump inflates
// and decodes into the map's BSP arenas exactly as an uncompressed `X*` lump.
let map = Map::assemble_with_options(&wad, &group, ParseOptions::strict())?;
let _ = (map.segs(), map.subsectors(), map.nodes());
# Ok(())
# }
```

### Strictness and limits

The inflated output of a single compressed node lump is bounded by
[`Limits::max_decoded_node_bytes`](https://docs.rs/crustywad/latest/crustywad/struct.Limits.html#structfield.max_decoded_node_bytes)
(default `1 << 26`, 64 MiB), enforced *during* inflation via `miniz_oxide`'s length-limited
inflater — the decoder never allocates a buffer larger than the cap (ADR-0016 §1). Exceeding it
is `MapAssembleError::ExtendedNode { reason: DecodedSizeExceeded, .. }` in strict mode, or a
whole-BSP degrade-to-empty with one warning in lenient mode; an un-inflatable stream is
`CorruptStream` under the same strict/lenient split. All other structural faults in the inflated
body follow the same contract as the uncompressed decoder.

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
| Build with `extended-nodes-zlib` | `cargo build -p crustywad --features extended-nodes-zlib` |
| Test with `extended-nodes-zlib` | `cargo test -p crustywad --features extended-nodes-zlib` |
| Full CI check | `just ci` |

See the [`justfile`](https://github.com/masriamir/crustywad/blob/main/justfile) for
available `just` recipes including feature-specific aliases.
