# ADR-0006: WAD write design

- **Status:** Proposed
- **Date:** 2026-06-14
- **Deciders:** @masriamir
- **Tracking issue:** https://github.com/masriamir/crustywad/issues/20

## Context

`crustywad` currently provides read-only WAD access. Milestone 6 adds write support. Before
any implementation begins, the design space must be explored and a direction chosen, because
the approach affects public API surface, error model, and how `Wad::into_bytes` in `lib.rs`
relates to the new serialization entry point.

### WAD on-disk layout recap

The WAD format tracks the directory via `infotableofs`, so the directory can appear
at any offset in the file. The proposed writer will always emit three regions in this
order:

```
[ 12-byte header ][ lump data blobs... ][ 16-byte directory entries... ]
```

The header stores:
- magic (`IWAD` or `PWAD`, 4 bytes)
- `numlumps` (i32 LE)
- `infotableofs` — byte offset of the directory (i32 LE)

Each directory entry is 16 bytes: `filepos` (i32), `size` (i32), `name` ([u8; 8], zero-padded).
The name field is an 8-byte array; WAD names are conventionally ASCII characters (per the
Doom spec), and `crustywad` enforces this — rejecting non-ASCII in strict mode and decoding
lossily in lenient mode.

### Three approaches considered

**Option A — Full rebuild (always serialize to a fresh `Vec<u8>`).**
Allocate a new buffer, write the header placeholder, append each lump's raw bytes in order,
record the directory offset, append directory entries, then back-patch the header. Offsets
and sizes are computed as lumps are appended; no pre-existing layout is reused.

Pros:
- Simple and correct by construction — directory always follows all lump data.
- Works regardless of whether the `Wad` was originally memory-mapped or owned.
- Structurally self-consistent: the bytes produced by `build()` can always be re-parsed
  under `ParseOptions` compatible with the `WriteOptions` used — e.g., a strict build
  always re-parses under strict settings.

Cons:
- O(total WAD size) allocation for any edit, even a one-byte change.
- Memory-mapped source data must be copied into the output buffer.

**Option B — In-place patch (modify byte ranges in the backing buffer for same-size changes).**
For edits that do not change a lump's size, overwrite the relevant byte slice directly in
the backing `Vec<u8>` and update the directory entry in-place. For size-changing edits, fall
back to a full rebuild.

Pros:
- Zero additional allocation for metadata-only or same-size lump edits.

Cons:
- Mmap-backed `WadData::Mapped` is read-only; in-place mutation requires converting to
  `WadData::Owned` first, negating the zero-copy benefit.
- Correctness requires careful bookkeeping of offset validity after every mutation.
- Two code paths (in-place and rebuild) increase implementation and testing surface.
- The "fall back to full rebuild" logic is invisible to callers and hard to test in isolation.

**Option C — Builder pattern (a separate `WadBuilder` / `WadWriter` type).**
Introduce a standalone `WadBuilder` that accepts a `WadKind` and a sequence of `(name,
bytes)` pairs, validates them, and serializes to a `Vec<u8>` on `build()`. The existing
`Wad` type grows a `to_builder()` conversion to allow round-tripping.

Pros:
- Clean separation between the parsed/read model (`Wad`) and the construction model
  (`WadBuilder`).
- Ergonomic for creation use cases that have no existing WAD to start from.
- No mutation of the parsed `Wad` struct; the immutable read model stays simple.
- Public API surface of `Wad` stays stable; write path is opt-in.

Cons:
- A `Wad` round-trip requires an intermediate `WadBuilder` step.
- Adds a new public type and conversion methods to the public API.

### `binrw` write support

`binrw` provides `BinWrite` / `BinWriterExt` symmetric to `BinRead`. The library already
depends on `binrw` for reading. Using `BinWrite` for the header and directory records
eliminates hand-rolled little-endian serialization and keeps format definitions co-located
with their read counterparts.

### `ParseOptions` / `Strictness` for write validation

The existing `ParseOptions { strictness }` type encodes two modes:

- **Strict:** reject the first invalid input (non-standard magic, non-ASCII lump names,
  out-of-bounds lump data ranges, negative counts) and return a typed error.
- **Lenient:** warn and recover (e.g., clamp out-of-bounds lump data ranges, decode
  non-ASCII lump names lossily via `String::from_utf8_lossy`, preserve unknown magic as
  `WadKind::Unknown`).

Write validation ensures caller-provided inputs are representable in the WAD format:
- Lump names must be ASCII, free of embedded NUL bytes, and at most 8 bytes. Strict mode
  rejects any violation with a `WriteError`. Lenient mode truncates names longer than 8 bytes
  and emits a `WriteWarning`; non-ASCII and NUL-containing names are rejected in both modes.
  Embedded NULs are forbidden because `decode_name` terminates at the first `\0` — a name
  written with an interior NUL would be silently shortened on re-parse, breaking round-trip
  invariants.
- Lump data sizes must fit in `i32`; both strict and lenient mode reject oversized lumps
  with a `WriteError` — truncating would silently discard user data with no way to recover
  it.
- Total lump count must fit in `i32`; both strict and lenient mode reject overflow with a
  `WriteError` — the count is stored as `i32` in the header with no fallback representation.
- Computed offsets (`filepos` per lump, `infotableofs` in the header) and the total output
  length must fit in `i32`; both modes reject a build where any computed offset would
  overflow — wrapping would produce an unreadable directory with no safe recovery path.
- `WadKind::Unknown` magic: strict mode rejects it, lenient mode writes the raw bytes.

A new `WriteOptions { strictness: Strictness }` should be introduced to avoid coupling
write behavior to parse behavior.

### `Wad::into_bytes`

`Wad::into_bytes(self) -> Vec<u8>` currently returns the original backing bytes unmodified —
it is a raw buffer extractor, not a serializer. This is useful for extracting owned bytes
from a loaded WAD and should be kept as-is. It must not be confused with or repurposed as
the write serialization path; the serialization entry point should have a distinct name such
as `WadBuilder::build() -> Result<Vec<u8>, WriteError>`.

## Decision

**Adopt Option C (builder pattern) as the primary write API, with `BinWrite` for record
serialization.**

Concretely:

1. Introduce a `WadBuilder` struct in a new `write` module (gated behind a `write` feature
   flag, off by default, to keep the default API surface read-only and reduce compile time
   for read-only users).
2. `WadBuilder::new(kind: WadKind)` starts an empty builder.
3. `WadBuilder::add_lump(name: &str, data: impl Into<Vec<u8>>) -> Result<&mut Self, WriteError>`
   validates the name and stores the lump.
4. `WadBuilder::build() -> Result<Vec<u8>, WriteError>` serializes the complete WAD:
   - Writes a placeholder 12-byte header.
   - Appends each lump's data in insertion order, tracking `filepos` per lump.
   - Writes directory entries using `BinWrite` on the existing `RawDirectoryEntry` type,
     extended to derive both `BinRead` and `BinWrite`.
   - Back-patches `numlumps` and `infotableofs` in the header.
5. `Wad::to_builder() -> WadBuilder` converts a parsed `Wad` into a `WadBuilder` for
   round-tripping and editing (add, remove, reorder lumps).
6. Introduce `WriteError` and `WriteWarning` types following the same `thiserror` pattern
   as `ParseError` / `ParseWarning`.
7. Introduce `WriteOptions { strictness: Strictness }` with the same API shape as
   `ParseOptions` (`strictness` field, `strict()` / `lenient()` constructors).
   `WadBuilder::build_with_options(opts: WriteOptions) -> Result<(Vec<u8>, Vec<WriteWarning>), WriteError>`
   is the lenient-mode entry point; `WadBuilder::build()` is a strict-mode convenience
   wrapper that returns `Result<Vec<u8>, WriteError>` (no warnings possible in strict mode).
8. Keep `Wad::into_bytes` as a raw buffer extractor (no behavior change).
9. Offset and size fields (`filepos`, `size` in directory entries; `infotableofs`,
   `numlumps` in the header) are always recomputed by the builder — callers never supply
   offsets directly.

`BinWrite` is chosen over hand-rolled serialization because:
- `binrw` is already a dependency.
- `BinWrite` keeps format layout in one place alongside `BinRead` on the same struct.
- Little-endian attribute `#[bw(little)]` mirrors `#[br(little)]` on `RawHeader` and
  `RawDirectoryEntry`, reducing the chance of endianness bugs.

Option A (full rebuild without a builder type) was not chosen because it provides no
ergonomic affordance for creation from scratch and conflates the mutation API with the
serialization API. Option B (in-place patch) was not chosen because the complexity of
managing two code paths outweighs the memory savings, especially given that mmap-backed
`Wad` instances must be converted to owned before any mutation.

## Consequences

- A new `write` feature flag and `write` module are added to the `crustywad` crate.
- `WadBuilder`, `WriteError`, `WriteWarning`, and `WriteOptions` are new public API surface.
- `RawDirectoryEntry` and `RawHeader` in `lib.rs` must derive `BinWrite` in addition to
  `BinRead`; this is a non-breaking internal change.
- `Wad::into_bytes` is unchanged; it remains a raw buffer extractor.
- The `Wad::to_builder()` method creates an owned copy of all lump data, so memory usage
  doubles during a round-trip. This is acceptable for an offline editing tool; a future
  zero-copy edit path can be added without changing the public API.
- `WriteOptions::strict()` / `::lenient()` follow the same API ergonomics as `ParseOptions`,
  making the write API consistent and familiar for existing users of the library.
- Implementation of this ADR requires sub-issues #21–#27 (per the epic tracking issue #12)
  to be opened and tackled after this ADR is accepted.
