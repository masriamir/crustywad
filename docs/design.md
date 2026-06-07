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

## Read pipeline

1. Read the file into owned bytes.
2. Parse the header with `binrw` using little-endian integers.
3. Validate or recover header fields according to `ParseOptions`.
4. Parse the lump directory.
5. Clamp invalid lump ranges in lenient mode and collect warnings.

## Strict vs. lenient parsing

`Strictness::Strict` treats malformed magic, negative counts, out-of-range offsets, oversized lumps, and non-ASCII names as hard errors.

`Strictness::Lenient` keeps parsing when possible, returning a `Wad` plus collected warnings. In lenient mode, invalid directory sizes are truncated to the number of complete entries that fit in the buffer and invalid lump byte ranges are clamped into a safe slice.

## Feature plan

- `mmap`: reserved module and feature flag for future memory-mapped I/O.
- `freedoom-tests`: optional integration tests that inspect downloaded FreeDoom fixtures.
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
- Optional FreeDoom fixture coverage for real-world inputs.
- `proptest` for parser invariants.
- Future fuzzing and criterion benchmarks once the API surface expands.
