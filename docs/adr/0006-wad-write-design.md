# ADR-0006: WAD write design

- **Status:** Proposed
- **Date:** 2026-06-14
- **Deciders:** @masriamir
- **Tracking issue:** https://github.com/masriamir/crustywad/issues/20

## Context

`crustywad` currently provides read-only WAD access. Milestone 6 adds write support. Before
any implementation begins, the design space must be explored and a direction chosen, because
the approach affects public API surface, error model, and how the existing `into_bytes` stub
in `lib.rs` should evolve.

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
Lump names are ASCII, up to 8 bytes, zero-padded to 8 bytes.

### Three approaches considered

**Option A — Full rebuild (always serialize to a fresh `Vec<u8>`).**
Allocate a new buffer, write the header placeholder, append each lump's raw bytes in order,
record the directory offset, append directory entries, then back-patch the header. Offsets
and sizes are computed as lumps are appended; no pre-existing layout is reused.

Pros:
- Simple and correct by construction — directory always follows all lump data.
- Works regardless of whether the `Wad` was originally memory-mapped or owned.
- Easy to round-trip: serialize to bytes then re-parse, and the result always parses cleanly.

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

- **Strict:** reject the first invalid input (bad lump name, oversized lump name, unknown
  `WadKind`, etc.) and return a typed error.
- **Lenient:** warn and clamp (e.g., truncate a name longer than 8 bytes to 8 bytes,
  replace non-ASCII bytes with the Unicode replacement character (U+FFFD)).

Write validation rules are the inverse of parse validation:
- Lump names must be ASCII and at most 8 bytes. Strict mode rejects any violation with a
  `WriteError`. Lenient mode truncates names longer than 8 bytes and emits a `WriteWarning`;
  non-ASCII names are rejected in both modes — there is no unambiguous ASCII-preserving
  sanitization for arbitrary Unicode input.
- Lump data sizes must fit in `i32`; both strict and lenient mode reject oversized lumps
  with a `WriteError` — truncating would silently discard user data with no way to recover
  it.
- Total lump count must fit in `i32`; both strict and lenient mode reject overflow with a
  `WriteError` — the count is stored as `i32` in the header with no fallback representation.
- `WadKind::Unknown` magic: strict mode rejects it, lenient mode writes the raw bytes.

A new `WriteOptions { strictness: Strictness }` (or reuse of `ParseOptions` renamed to
`Options`) should be introduced to avoid coupling write behavior to parse behavior.

### The existing `into_bytes` stub

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
   - Writes directory entries using `BinWrite` on a new `RawDirectoryEntry` type that
     derives both `BinRead` and `BinWrite`.
   - Back-patches `numlumps` and `infotableofs` in the header.
5. `Wad::to_builder() -> WadBuilder` converts a parsed `Wad` into a `WadBuilder` for
   round-tripping and editing (add, remove, reorder lumps).
6. Introduce `WriteError` and `WriteWarning` types following the same `thiserror` pattern
   as `ParseError` / `ParseWarning`.
7. Introduce `WriteOptions { strictness: Strictness }` mirroring `ParseOptions`; pass it
   to `WadBuilder::build_with_options`.
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
- Write validation mirrors parse validation: `WriteOptions::strict()` / `::lenient()`
  follows the same ergonomics as `ParseOptions`.
- Implementation of this ADR requires sub-issues #21–#27 (per the epic tracking issue #12)
  to be opened and tackled after this ADR is accepted.
