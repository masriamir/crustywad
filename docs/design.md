# Design

## Project overview

`crustywad` is a Rust workspace providing safe, documented Doom WAD file I/O. It targets the Rust 2024 edition with MSRV 1.85.0 and is dual-licensed under MIT OR Apache-2.0.

## Goals

- Provide a small, safe Rust library for Doom WAD file I/O.
- Start with reliable header and lump-directory reading.
- Keep the API ready for future write support, async I/O, zero-copy parsing, and optional memory mapping.

## Non-goals

- Full map graph assembly in this milestone.
- Async runtime integration in this milestone.

## Data model

A WAD contains a 12-byte header, lump data blobs, and a directory of 16-byte lump entries. The header stores the byte offset at which the directory begins; in practice the directory sits after all lump data at the end of the file. `crustywad` models the file as owned bytes plus validated metadata for the parsed header and lump directory.

See [Data model](diagrams/data-model.md) for the WAD on-disk layout and public API type relationship diagrams.

## Read pipeline

1. Read the file into owned bytes.
2. Parse the header with `binrw` using little-endian integers.
3. Validate or recover header fields according to `ParseOptions`.
4. Parse the lump directory.
5. Clamp invalid lump ranges in lenient mode and collect warnings.

See [Data flow](diagrams/data-flow.md) for the read pipeline flowchart.

## Strict vs. lenient parsing

Parsing is controlled by `ParseOptions { strictness: Strictness::Strict | Strictness::Lenient }`.

`Strictness::Strict` treats malformed magic, negative counts, out-of-range offsets, oversized lumps, and non-ASCII names as hard errors.

`Strictness::Lenient` keeps parsing when possible, returning a `Wad` plus collected warnings. In lenient mode, invalid directory sizes are truncated to the number of complete entries that fit in the buffer and invalid lump byte ranges are clamped into a safe slice.

See [Data flow](diagrams/data-flow.md) for the strict/lenient mode comparison diagram.

## Map record parsing

`parse_records::<T>` turns raw lump bytes into a typed vector using `binrw`. The generic parameter `T` may be any record-based map lump type (`Thing`, `Linedef`, `Sidedef`, `Vertex`, `Seg`, `Subsector`, `Node`, `Sector`) that implements `BinRead<Args<'_> = ()>`. An empty buffer always yields an empty `Vec`. Otherwise the on-disk record size is derived by parsing the first record and measuring the bytes consumed by `BinRead` — this avoids relying on `size_of::<T>()`, which reflects in-memory layout rather than on-disk size. If zero bytes are consumed or the total length is not an exact multiple of the record size, a `TrailingBytes` error is returned.

See [Data flow](diagrams/data-flow.md) for the map record parsing flowchart.

## Write pipeline

The `write` feature flag adds `WadBuilder`, a standalone type for constructing a WAD from scratch or round-tripping a parsed `Wad` via `Wad::to_builder()`. Callers accumulate lumps with `add_lump`, then call `build()` (strict mode) or `build_with_options()` (strict or lenient, per `WriteOptions`). All name and size validation, plus offset (`filepos`, `infotableofs`) computation, is deferred to `build`/`build_with_options` — callers never supply offsets directly. The output always has the layout `[12-byte header][lump data blobs][16-byte directory entries]`, per ADR-0006.

`WriteOptions { strictness: Strictness }` mirrors `ParseOptions`: strict mode rejects invalid input immediately with a `WriteError`; lenient mode truncates over-length names and non-standard magic values, collecting `WriteWarning`s instead.

See [Data flow](diagrams/data-flow.md) for the write pipeline flowchart and the strict/lenient write mode comparison, and [Data model](diagrams/data-model.md) for how `WadBuilder` and its supporting types relate to `Wad`.

## Feature plan

- `mmap`: enables `Wad::from_path_mapped[_with_options]` for read-only memory-mapped file loading via `memmap2`; `from_path` always reads into memory regardless of this flag.
- `write`: enables `WadBuilder`, `WriteError`, `WriteWarning`, `WriteOptions`, and `Wad::to_builder()` for WAD serialization.
- `freedoom-tests`: optional integration tests that inspect downloaded Freedoom fixtures.
- Future `async`: alternate I/O constructors without changing the in-memory parse model.
- Future zero-copy: borrowed views over validated bytes.

## Milestones

1. Header and directory parsing
2. Map lump record parsing
3. Graphics and patches
4. Texture composition
5. Audio lumps
6. Writing support

## Testing strategy

- Synthetic WAD builders for offline unit and integration tests.
- Optional Freedoom fixture coverage for real-world inputs.
- `proptest` for parser invariants.
- Future fuzzing and criterion benchmarks once the API surface expands.

## Code conventions

### Error handling

All errors in the library crate use `thiserror`-derived enums (`ParseError`, `MapParseError`). `anyhow` is permitted only in `crustywad-cli`.

### Documentation

`missing_docs = "deny"` is enforced workspace-wide — every public item must have a doc comment. All documentation uses American English spelling.

### Safety

`#![deny(unsafe_code)]` is set in the core library crate. Unsafe code is permitted only in `mmap.rs`.

### Lints

`clippy::all` and `clippy::pedantic` are enabled workspace-wide. All warnings are errors in CI.

## Development workflow

Run `just ci` before pushing. It runs the same checks as GitHub Actions (build, test, clippy, fmt, doc, deny, docs-sync) and catches failures locally before they reach CI.

## Commit conventions

Follow [Conventional Commits](https://www.conventionalcommits.org/) (e.g. `feat:`, `fix:`, `docs:`, `chore:`). Scope is encouraged: `feat(map):`, `fix(cli):`, etc.
