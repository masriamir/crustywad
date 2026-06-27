# Design

## Goals

- Provide a small, safe Rust library for Doom WAD file I/O.
- Start with reliable header and lump-directory reading.
- Keep the API ready for future write support, async I/O, zero-copy parsing, and optional memory mapping.

## Non-goals

- Full map graph assembly in this milestone.
- Write support in this milestone.
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

`Strictness::Strict` treats malformed magic, negative counts, out-of-range offsets, oversized lumps, and non-ASCII names as hard errors.

`Strictness::Lenient` keeps parsing when possible, returning a `Wad` plus collected warnings. In lenient mode, invalid directory sizes are truncated to the number of complete entries that fit in the buffer and invalid lump byte ranges are clamped into a safe slice.

See [Data flow](diagrams/data-flow.md) for the strict/lenient mode comparison diagram.

## Map record parsing

`parse_records::<T>` turns raw lump bytes into a typed vector using `binrw`. The generic parameter `T` may be any map record type (`Thing`, `Linedef`, `Sidedef`, `Vertex`, `Seg`, `Subsector`, `Node`, `Sector`) that implements `BinRead<Args<'_> = ()>`. Zero-sized types (`size_of::<T>() == 0`) are handled as a special case before the modulo check: an empty buffer yields an empty `Vec`, and a non-empty buffer is an unconditional `TrailingBytes` error. For all other types, records are read sequentially until the cursor reaches the end of the slice.

See [Data flow](diagrams/data-flow.md) for the map record parsing flowchart.

## Feature plan

- `mmap`: enables `Wad::from_path_mapped[_with_options]` for read-only memory-mapped file loading via `memmap2`; `from_path` always reads into memory regardless of this flag.
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
