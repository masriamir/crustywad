# crustywad

[![CI](https://github.com/masriamir/crustywad/actions/workflows/ci.yml/badge.svg)](https://github.com/masriamir/crustywad/actions/workflows/ci.yml)
[![CodeQL](https://github.com/masriamir/crustywad/actions/workflows/codeql.yml/badge.svg)](https://github.com/masriamir/crustywad/actions/workflows/codeql.yml)
[![Coverage](https://codecov.io/gh/masriamir/crustywad/graph/badge.svg)](https://codecov.io/gh/masriamir/crustywad)
[![Benchmarks](https://img.shields.io/badge/benchmarks-Criterion-informational)](https://crustywad.dev/dev/bench/)
[![dependency status](https://deps.rs/repo/github/masriamir/crustywad/status.svg)](https://deps.rs/repo/github/masriamir/crustywad)
[![docs.rs](https://img.shields.io/docsrs/crustywad)](https://docs.rs/crustywad)
[![crates.io](https://img.shields.io/crates/v/crustywad.svg)](https://crates.io/crates/crustywad)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)
[![MSRV](https://img.shields.io/badge/MSRV-1.94.0-blue)](https://www.rust-lang.org)

Performant, safe, typed Doom WAD file I/O in Rust.

A Doom WAD is a container format that stores a header plus a directory of named "lumps" containing maps, graphics, audio, and other game data. The [Doom Wiki](https://doomwiki.org/wiki/WAD) is a good starting point for the unofficial format specification.

**Full guide, API usage, and CLI reference: [crustywad.dev](https://crustywad.dev)**

## Status

`crustywad` provides safe, documented reading of WAD headers, lump directories, and typed map-record lumps; zero-copy memory-mapped WAD loading via the `mmap` feature; writing of WAD headers, lump directories, and raw lump data via `WadBuilder`; [UDMF ↔ Doom map format conversion](https://crustywad.dev/converting-maps.html); [classic graphics decode](https://crustywad.dev/graphics.html): pictures (patches/sprites), flats, PLAYPAL/COLORMAP, and TEXTUREx/PNAMES composition, with indexed + RGBA8 views; content-first audio detection with typed DMX and PC-speaker sound-effect decode, plus WAV/MIDI container parsing of the Doom 64 remaster's `DS`/`DM` sections, surfaced through audio-aware `cwad extract` (DMX lumps wrapped as WAV, MUS lumps optionally converted to MIDI) and `cwad info`/`list` audio annotations; and a `cwad` CLI for inspecting, validating, merging, diffing, extracting, converting, and building WAD files. Correctness and performance are validated via `cargo-fuzz` targets and Criterion benchmarks.

Integration tests for each layer live in `crates/crustywad/tests/`:
- `wad_reader.rs` — WAD header and directory parsing
- `map_records.rs` — typed map-record decoding (`Thing`, `Linedef`, `Sector`, etc.)
- `write.rs` — WAD write support (`WadBuilder`)
- `e2e.rs` — end-to-end read → modify → write → verify pipeline
- `malformed_wads.rs` — synthetic corpus of malformed and large WAD inputs
- `audio.rs` — classic audio detection and sound-effect decode (`AudioKind`, `DmxSound`, `PcSpeakerSound`)
- `freedoom.rs` — optional tests against real Freedoom WAD fixtures

## Installation

Library:

```toml
[dependencies]
crustywad = "0.8.0"
```

Enable optional features as needed (see [Feature flags](#feature-flags) below):

```toml
[dependencies]
crustywad = { version = "0.8.0", features = ["write", "mmap"] }
```

CLI (`cwad`) — any of:

```bash
# From source via cargo
cargo install crustywad-cli

# Prebuilt binary via cargo-binstall (falls back to source if unavailable)
cargo binstall crustywad-cli

# Prebuilt binary via the platform installer script (from the GitHub Release)
# Linux/macOS:
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/masriamir/crustywad/releases/latest/download/crustywad-cli-installer.sh | sh
```

Windows PowerShell:

```powershell
irm https://github.com/masriamir/crustywad/releases/latest/download/crustywad-cli-installer.ps1 | iex
```

> **Note:** The prebuilt-binary options (`cargo binstall` and the installer scripts) require a release produced by the binary-release pipeline, available from **v0.1.1 onward**. For **v0.1.0**, install from source with `cargo install crustywad-cli`.

## Workspace layout

- `crates/crustywad` — core library for safe WAD reading and writing.
- `crates/crustywad-cli` — `cwad`, a CLI for inspecting, validating, merging, diffing, extracting, converting, and building WAD files.
- `docs/` — design notes, ADRs, and the [mdBook user guide](https://crustywad.dev).
- `.github/` — CI, release automation, issue templates, and repository policy files.

## Quickstart

### Library

```rust
use crustywad::Wad;

let bytes = [
    b'I', b'W', b'A', b'D',
    0, 0, 0, 0,
    12, 0, 0, 0,
];

let wad = Wad::from_bytes(bytes)?;
assert_eq!(wad.lump_count(), 0);
# Ok::<(), crustywad::ParseError>(())
```

See [Reading WAD Files](https://crustywad.dev/reading-wads.html) and [Writing WAD Files](https://crustywad.dev/writing-wads.html) in the guide for the full API, or `crates/crustywad/examples/` for runnable examples (`cargo run -p crustywad --example read_wad`).

### Assembling a map

`crustywad::map` can go beyond raw lump records and assemble a normalized, index-addressed
`Map` graph for a single map, resolving vertex/sidedef/sector cross-references along the way:

```rust
use crustywad::Wad;
use crustywad::map::Map;

let wad = Wad::from_path("DOOM1.WAD")?;

if let Some(group) = wad.map_group("E1M1") {
    let map = Map::assemble(&wad, &group)?;

    for linedef in map.linedefs() {
        let (start, end) = map.linedef_vertices(linedef);
        // `linedef_right` is `Option`: a rare line may have no front side.
        let Some(right) = map.linedef_right(linedef) else {
            continue;
        };
        let front_sector = map.sidedef_sector(right);

        match map.linedef_left(linedef) {
            Some(left) => {
                let back_sector = map.sidedef_sector(left);
                println!(
                    "two-sided line ({:.0},{:.0})-({:.0},{:.0}): front floor {}, back floor {}",
                    start.x, start.y, end.x, end.y,
                    front_sector.floor_height, back_sector.floor_height
                );
            }
            None => println!(
                "one-sided line ({:.0},{:.0})-({:.0},{:.0})",
                start.x, start.y, end.x, end.y
            ),
        }
    }
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

`Wad::map_groups()` / `Wad::map_group(name)` locate a map's marker lump and its associated data
lumps within the flat directory; `Map::assemble` (strict) and `Map::assemble_with_options`
(honors `ParseOptions::strictness`) build the graph. See
[Map Record Parsing](https://crustywad.dev/map-records.html) in the guide for the full API,
including lenient-mode dangling-reference handling and the one-sided-line sentinel.
Marker-delimited directory sections (`F_START`/`S_START`/… incl. nested sub-namespaces and
Boom aliases) enumerate via `Wad::sections()`.

Doom, **Doom II**, **Heretic**, **Hexen**, and **Doom 64** maps all assemble into the `Map` graph.
Doom/Doom II/Heretic share the classic binary record layout (differing only in map-marker naming,
e.g. `MAP01` vs `E1M1`); Hexen is detected via the `BEHAVIOR` lump and assembles with format-tagged
extended fields. **Doom 64** stores each map as a nested WAD inside its `MAPxx` marker lump
(detected by name **and** magic) and assembles into the same graph — its sidedef/sector
texture and flat fields carry a `u16` hash on disk that assembly resolves to a `TextureRef::Name`
whenever the outer WAD carries a `Textures` section (first-match-in-disk-order on a collision;
`TextureRef::Index` otherwise, or on an unresolved hash under lenient assembly). Its per-sector
colored lighting becomes `MapSector.colors` indexing `Map::lights()`, the map's combined light
table. A Doom 64-sourced `Map` writes back out (`write_doom_map` / `write_udmf`) once its texture
references resolve to names; the remaining unrepresentable colored lighting follows the same
three-tier policy as every other conversion (strict refuses, lenient drops it and warns).
`REJECT` and `BLOCKMAP` decode into typed, queryable structures (`MapReject` sector-visibility lookups, `MapBlockmap` per-block linedef lists) during map assembly, and Doom 64's `LEAFS` render leaves decode into a per-subsector `MapLeaf` arena. `MACROS` scripts decode read-only into `MapMacro` action lists.
Beyond the classic `SEGS`/`SSECTORS`/`NODES` BSP, assembly reads the ZDoom uncompressed extended node formats (`XNOD`/`XGLN`/`XGL2`/`XGL3`, and their zlib-compressed `Z*` twins behind the `extended-nodes-zlib` feature) and DeePBSP v4 (`xNd4`) into the same `segs()`/`subsectors()`/`nodes()` arenas, plus **classic GL nodes** (`GL_<mapname>` groups: V2/V3/V5; V1/V4 refused) into separate, additive `gl_vertices()`/`gl_segs()`/`gl_subsectors()`/`gl_nodes()` arenas — in-WAD groups only, `.gwa` sibling files not yet supported.
UDMF (`TEXTMAP`) maps are read into the `Map` graph and can be written back out with
`write_udmf` / `add_udmf_map` (the `write` feature). The same `Map` graph converts a UDMF
map to the classic Doom binary lumps with `write_doom_map` / `add_doom_map` — see
[Converting maps](https://crustywad.dev/converting-maps.html) in the guide for the
round-trip envelope and the three-tier data-loss policy.

### CLI

```text
cargo run -p crustywad-cli -- info path/to/file.wad
cargo run -p crustywad-cli -- list path/to/file.wad
```

`cwad` also has `validate`, `merge`, `diff`, `extract`, `convert`, and `build` subcommands — `convert --to doom --nodes` additionally builds engine-playable node lumps (`SEGS`/`SSECTORS`/`NODES`/`REJECT`/`BLOCKMAP`). See [CLI Usage](https://crustywad.dev/cli.html) in the guide for the full reference.

## Feature flags

| Feature | Default | Description |
| --- | --- | --- |
| `mmap` | no | Enables `Wad::from_path_mapped` and `Wad::from_path_mapped_with_options` for memory-mapped file loading via `memmap2` — no heap copy on load. `Wad::from_path` always reads into memory regardless of this flag. |
| `write` | no | Enables `WadBuilder`, `WriteError`, `WriteOptions`, `WriteWarning`, and `Wad::to_builder()` for WAD serialization. |
| `nodebuild` | no | Enables the `map::build` node-lump builders (implies `write`) — `build_blockmap`/`build_reject`/`build_nodes` (the classic BSP pass: `SEGS`/`SSECTORS`/`NODES`), the `add_doom_map_with_nodes` engine-playable one-shot, and their `to_lump_bytes` serializers, for clean-room BLOCKMAP/REJECT/BSP node generation (ADR-0024). Powers `cwad convert --nodes`. |
| `doom64-gfx` | no | Enables `Doom64Png` decoding of Doom 64's PNG texture/sprite lumps via the `png` crate (indexed pixels + palette rows + `grAb` offsets, capped by `Limits::max_decoded_pixels`). |
| `extended-nodes-zlib` | no | Enables reading the zlib-compressed ZDoom extended node formats (`ZNOD`/`ZGLN`/`ZGL2`/`ZGL3`) by inflating (via the pure-Rust `miniz_oxide`) to their uncompressed twins and decoding through the same parser, bounded by `Limits::max_decoded_node_bytes`. Off by default so the core build stays decompressor-free (ADR-0025 §5). |
| `freedoom-tests` | no | Enables optional integration tests against local Freedoom WADs, supplied via `CRUSTYWAD_FREEDOOM_DIR` (auto-fetchable via `just fetch-fixtures`). |
| `hexen-tests` | no | Enables optional integration tests against a local Hexen IWAD, supplied via `CRUSTYWAD_HEXEN_DIR` (not auto-fetchable). |
| `doom64-tests` | no | Enables optional integration tests against a local Doom 64 IWAD, supplied via `CRUSTYWAD_DOOM64_DIR` (not auto-fetchable). |
| `sweep-tests` | no | Enables an optional sweep test that assembles every map of every WAD in a local collection, supplied via `CRUSTYWAD_SWEEP_DIR` (not auto-fetchable). |
| `guide-doctests` | no | Internal, CI-only: compiles the guide's Rust code samples as crate doctests (enabled by `--all-features`); not a runtime capability. |

## Development

Install [`just`](https://github.com/casey/just) and run:

```text
just build
just test
just lint
just doc
just fetch-fixtures
just ci
```

`just cov` uses `cargo-llvm-cov`, and the Codecov upload in CI may require a `CODECOV_TOKEN` repository secret depending on repository visibility and Codecov settings.

### Freedoom fixtures

Optional integration tests parse real Freedoom WAD files. The Freedoom version to download is configurable:

```bash
# Default version (v0.13.0)
just fetch-fixtures

# Override via argument
just fetch-fixtures version=v0.14.0

# Override via environment variable
FREEDOOM_VERSION=v0.14.0 just fetch-fixtures
```

Enable the optional fixture coverage with `just test-freedoom`, or by passing `--features freedoom-tests` (or `--all-features`) **and** setting `CRUSTYWAD_FREEDOOM_DIR` to an **absolute** path (e.g. `CRUSTYWAD_FREEDOOM_DIR="$PWD/tests/fixtures/freedoom"`). A relative path does not resolve — cargo runs the test binary from the package root — and the fixture tests skip silently instead of failing.

## MSRV

The minimum supported Rust version is **1.94.0**. The project follows a rolling **N-3** MSRV policy — MSRV tracks _(latest stable Rust minor at release time) − 3_, guaranteeing builds on the last four stable Rust releases (~6 months). An MSRV raise is a minor version bump. See the [versioning guide](https://masriamir.github.io/crustywad/versioning.html#msrv-policy) for details.

## Roadmap

1. ✅ Directory reading
2. 🔜 Map lump parsing
3. Graphics
4. Textures
5. Audio
6. ✅ Write support

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for setup, hooks, fixture handling, and release notes.

## License

Licensed under either of:

- [MIT License](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

at your option.
