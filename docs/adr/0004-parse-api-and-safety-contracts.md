# 0004. Parse API and safety contracts for `Wad` and map records

- **Status:** Accepted
- Date: 2026-06-06

## Context

Three related design choices came up when reviewing the initial milestone 1
implementation. Each has non-obvious trade-offs that future contributors might
revisit.

### 1. `from_bytes` accepts `impl Into<Vec<u8>>`

`Wad::from_bytes` needs to own the raw bytes for the lifetime of the `Wad`
(so that `lump_bytes` and `lump_data` can return slices into them without
additional allocation). The initial implementation took `impl AsRef<[u8]>` and
called `.to_vec()`, making a copy even when the caller passed an owned
`Vec<u8>`. `from_bytes_with_options` already used `impl Into<Vec<u8>>`, so the
two entry points had inconsistent performance contracts.

### 2. `lump_data` panics rather than returning `Option`

`Wad::lump_data(&Lump)` slices `self.bytes` directly. `Lump` is `Clone`, so a
caller could pass a lump cloned from a different, larger WAD, causing an
out-of-bounds slice panic. The alternative is to return `Option<&[u8]>`, but
that changes the API shape and forces every call site to unwrap.

### 3. `parse_records` uses `mem::size_of` to detect trailing bytes

When a byte slice length is not a multiple of the record size, the original
implementation's post-loop check (`cursor.position() != bytes_len`) was
unreachable: `binrw` would fail with an opaque `Binrw` error before the loop
exited cleanly. `MapParseError::TrailingBytes` was therefore never produced.

## Decision

**`from_bytes` uses `impl Into<Vec<u8>>`.**  
This unifies the API with `from_bytes_with_options`, avoids a needless copy for
owned buffers, and still accepts `&[u8]` and fixed-size arrays via the standard
`From` impls.

**`lump_data` panics with an assertion, not an `Option` return.**  
Lumps are only created by the parser and are tied to a specific `Wad` instance.
Passing a lump from one WAD to another is a programming error, not a runtime
condition — it should surface loudly. The assertion includes the byte ranges in
its message so the mistake is immediately diagnosable. `lump_bytes(index)`
already provides the safe, index-checked path for callers who need fallibility.

**`parse_records` pre-checks `bytes.len() % mem::size_of::<T>()`.**  
For the record types in this crate (all `binrw`-derived structs with only
primitive integer and fixed-size byte-array fields), `mem::size_of::<T>()` equals
the binary serialized size because Rust's layout produces no padding for
uniformly-aligned fields. This pre-check makes `TrailingBytes` reachable and
gives callers a specific error variant to match. If a future record type adds
`binrw` padding attributes that cause the serialized size to diverge from
`mem::size_of`, this check will need revisiting.

## Consequences

- Callers of `from_bytes` who passed a `Vec<u8>` now get a zero-copy path
  automatically; callers who passed a `&[u8]` continue to pay one allocation.
- `lump_data` is not foolproof against cross-WAD misuse, but the assertion
  message makes such bugs easy to diagnose. A future zero-copy or borrowed-Wad
  API could tie `Lump` to its parent by lifetime, eliminating the issue entirely.
- `parse_records` has an implicit assumption documented here: serialized record
  size == `mem::size_of::<T>()`. Violating this (e.g. via `#[br(pad_after)]`)
  would cause the pre-check to use the wrong stride.
