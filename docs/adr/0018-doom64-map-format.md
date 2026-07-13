# ADR-0018: Doom 64 map format — nested-WAD structure and raw-record reading

- **Status:** Accepted
- **Date:** 2026-07-12
- **Deciders:** @masriamir
- **Tracking issue:** https://github.com/masriamir/crustywad/issues/54

## Context and problem statement

ADR-0014 (multi-format map support) named Doom 64 "the most divergent target and
the largest unknown," deferring its record layouts and detection strategy to #54.
This ADR settles them, grounded in the **measured structure of a real retail
`DOOM64.WAD`** (an IWAD, 1668 lumps, 40 maps `MAP01`–`MAP40`), inspected with
crustywad's own reader (`Wad::from_bytes`) rather than secondary sources.

The inspection **corrected two assumptions** that earlier doc-based research had
recorded:

### Finding 1 — Doom 64 maps are nested WADs, not flat marker+sibling lumps

Unlike a classic Doom map (a conventionally empty marker lump followed by sibling
`THINGS`, `LINEDEFS`, … lumps at the top level), **each Doom 64 `MAPxx` is a single
lump whose bytes are themselves a complete WAD**. `MAP01` is one 112 800-byte lump
beginning with the ASCII magic `IWAD`, a little-endian `i32` lump count (`14`),
and a directory offset. Its bytes decode to a self-contained WAD whose own directory
lists 14 sub-lumps — the first of which is an *inner* empty `MAP01` marker, distinct
from the outer `MAP01` lump that contains it:

```
MAP01(marker, 0B)  THINGS  LINEDEFS  SIDEDEFS  VERTEXES  SEGS  SSECTORS
NODES  SECTORS  REJECT  BLOCKMAP  LEAFS  LIGHTS  MACROS
```

So reading a Doom 64 map is: take the `MAPxx` lump's bytes → parse them **as a
WAD** (`Wad::from_bytes`) → read the sub-lump records. Most map lumps carry `IWAD`
magic; at least one (`MAP40`) carries `PWAD` magic — both are valid `WadKind`s the
existing reader already accepts. This is the classic N64 lineage layout (as used
by Doom64 EX / WadGen), evidenced by the `LEAFS`/`LIGHTS`/`MACROS` sub-lumps and
top-level `DEMO1..4LMP` lumps; it is **not** the 2020 KEX re-release repackaging.

### Finding 2 — Doom 64 maps are structurally auto-detectable

Earlier research assumed Doom 64 shared Doom's lump names with **no** auto-detect
signal, requiring a caller hint. The nested-WAD structure is itself the signal: a
Doom 64 map is a `MAPxx` lump whose content begins with `IWAD`/`PWAD` magic, whereas
a classic Doom/Hexen map marker carries no such magic (it is a conventionally empty
lump) and is recognized instead by a following data lump. Detection keys on the
presence of the nested-WAD magic, not on any marker size check. No caller hint is
needed.

### Finding 3 — measured record layouts

All sizes are exact divisors of the real MAP01 sub-lump byte counts; field
breakdowns are read from actual records (little-endian throughout):

| Sub-lump | Record | Fields (measured) |
|---|---|---|
| `VERTEXES` | 8 B | `x: i32`, `y: i32` — **16.16 fixed-point** (raw `20971520` = `320.0`) |
| `THINGS` | 14 B | `x, y, z, angle, type, flags, id` — all `i16` (`z`/`id` are Doom 64 additions; `id` = the tag/tid) |
| `LINEDEFS` | 16 B | `v1: u16, v2: u16, flags: u32, special: u16, tag: u16, sidefront: u16, sideback: u16` — **flags widened to `u32`**; `sideback == 0xffff` is the one-sided sentinel |
| `SIDEDEFS` | 12 B | `x_offset: i16, y_offset: i16, upper: u16, lower: u16, middle: u16, sector: u16` — **textures are `u16` indices**, not 8-byte names |
| `SECTORS` | 24 B | `floor_height: i16, ceiling_height: i16, floor_tex: u16, ceiling_tex: u16, colors: [u16; 5], special: u16, tag: u16, flags: u16` — **flats are `u16` indices** + **5 colored-lighting IDs** |
| `SEGS` | 12 B | `v1: u16, v2: u16, angle: u16, linedef: u16, side: u16, offset: i16` — matches `map::common::Seg` field-for-field (`offset` is signed; goes negative after BSP splitting) |
| `SSECTORS` | 4 B | `seg_count: u16, first_seg: u16` |
| `NODES` | 28 B | classic Doom node: `x, y, dx, dy: i16`, two `[i16; 4]` bounding boxes, two `u16` children (`0x8000` = subsector flag) |
| `LIGHTS` | 6 B | `r, g, b: u8` + 3 further bytes — the colored-lighting palette the sector color IDs reference |
| `LEAFS` | variable | render leaves (subsector edge lists) |
| `MACROS` | variable | compiled script bytecode (Doom 64's ACS/BEHAVIOR analog) |
| `REJECT`, `BLOCKMAP` | variable | as in classic Doom |

### Current code this ADR must not contradict

- `Wad::from_bytes[_with_options]` parses arbitrary WAD bytes and already accepts
  both `IWAD` and `PWAD` magic (`WadKind`). It can parse a `MAPxx` lump's bytes
  directly — verified against the real file.
- `map::common` holds records whose byte layout is identical across formats
  (`Vertex`, `Sidedef`, `Sector`, `Seg`, `Subsector`, `Node`, `Name8`);
  `map::doom` and `map::hexen` hold the format-specific `Thing`/`Linedef`.
  Doom 64's `VERTEXES`/`THINGS`/`LINEDEFS`/`SIDEDEFS`/`SECTORS` each differ (in
  width or field layout) from every existing struct and need new `map::doom64`
  types. Its **BSP lumps `SEGS`/`SSECTORS`/`NODES`, however, share the classic
  Doom byte layout** already modeled by `map::common::{Seg, Subsector, Node}`
  (verified field-for-field against the real file), so Doom 64 **reuses** those
  and defines only the differing records in its own module.
- `parse_records::<T>` reads a lump byte slice into `Vec<T>` for any
  `T: BinRead<Args<'_> = ()>`; it derives on-disk record size from the first
  record's consumed bytes. It applies unchanged to Doom 64 sub-lump slices.
- `detect_map_format(wad, group) -> MapFormat` classifies a `MapGroup` (a marker
  plus its sibling data-lump run). A Doom 64 map is a single lump, not a
  marker+run, so it does **not** fit the `MapGroup` model — this ADR keeps Doom 64
  out of that machinery (see Decision).
- `resolve_left`'s `0xffff` one-sided sentinel (`assemble.rs`) matches the Doom 64
  `sideback` sentinel — relevant only if/when Doom 64 is later normalized into the
  `Map` graph (deferred; see Scope).

## Decision drivers

- **Ground truth over guesses.** Layouts are taken from the real file; anything
  not yet byte-verified is called out, not invented.
- **Avoid the "largest unknown" scope trap** ADR-0014 flagged: Doom 64's `u16`
  texture/flat **indices** and colored-lighting IDs cannot be modeled meaningfully
  in the `Map` graph without a texture/graphics layer that does not exist yet.
- **Consistency with the hardening policy** (ADR-0016): the reader must be
  O(input), non-recursive, no-panic in both strictness modes, and fuzzed.
- **Reuse the existing WAD reader** for the nested WAD rather than a bespoke
  container parser.

## Considered options

1. **Standalone `map::doom64` raw-record module, no `Map`-graph integration
   (chosen).** New record structs + a helper to read a `MAPxx` lump's inner WAD
   and parse its sub-lumps. No `MapFormat::Doom64`, no `detect_map_format` branch,
   no normalization into `Map`.
2. **Raw records + detection integration.** As (1) plus a `MapFormat::Doom64`
   variant and a `detect_map_format`/`MapGroup` branch, while `Map::assemble`
   still refuses Doom 64. Adds format-API surface and couples the flat
   `MapGroup` model to a nested-WAD map it cannot represent, for a format that
   still cannot assemble.
3. **Full integration.** Records **and** normalization into the `Map` graph now,
   storing raw texture/flat/color indices in new graph fields. Forces graph
   changes for indices no consumer can yet resolve — the exact "largest unknown"
   scope risk ADR-0014 warned against.

## Decision outcome

**Chosen: Option 1 — a standalone `map::doom64` raw-record module.**

### 1. Module and record types

A new module `crates/crustywad/src/map/doom64.rs`, mirroring `map::doom` /
`map::hexen`, with `binrw`-derived, little-endian structs matching Finding 3.
Every field is documented and anchored to its measured on-disk width:

```rust
pub struct Vertex { pub x: i32, pub y: i32 }              // 16.16 fixed-point
pub struct Thing { pub x: i16, pub y: i16, pub z: i16, pub angle: i16,
    pub type_id: i16, pub flags: i16, pub id: i16 }        // 14 B
pub struct Linedef { pub v1: u16, pub v2: u16, pub flags: u32, pub special: u16,
    pub tag: u16, pub sidefront: u16, pub sideback: u16 }  // 16 B
pub struct Sidedef { pub x_offset: i16, pub y_offset: i16, pub upper: u16,
    pub lower: u16, pub middle: u16, pub sector: u16 }     // 12 B; textures are indices
pub struct Sector { pub floor_height: i16, pub ceiling_height: i16,
    pub floor_tex: u16, pub ceiling_tex: u16, pub colors: [u16; 5],
    pub special: u16, pub tag: u16, pub flags: u16 }       // 24 B; 5 color IDs
pub struct Light { pub r: u8, pub g: u8, pub b: u8, /* + 3 bytes, field-typed in #54 */ }
```

Doom 64's BSP lumps are **not** redefined: `SEGS` (12 B), `SSECTORS` (4 B), and
`NODES` (28 B) match the classic Doom byte layout field-for-field, so the reader
uses the existing `map::common::{Seg, Subsector, Node}` for them.

`REJECT`, `BLOCKMAP`, `LEAFS`, and `MACROS` are read as **raw byte slices**
(no record decoding) in this pass — `LEAFS` (render leaves) and `MACROS`
(compiled scripts) are recognition-only, mirroring how Hexen's `BEHAVIOR` was
recognized but not decoded.

**The `Light` trailing 3 bytes, and the exact semantics of `Sector.colors` /
texture indices, are the only field-level items still to be pinned in the #54
spec** (against the measured `LIGHTS` records and Doom64 EX source). They do not
block this ADR's decisions.

### 2. Reading a Doom 64 map (nested WAD)

A `MAPxx` lump's bytes are parsed with the **existing** `Wad::from_bytes`; its
sub-lumps are read with the **existing** `parse_records::<doom64::T>`. The #54
API surface is a small helper layer — exact signatures are the #54 spec's job,
but the shape is: given a `MAPxx` lump's bytes, produce the parsed record vectors
(and raw slices for the undecoded lumps). No new WAD-container code is written.

### 3. Detection — a standalone helper, not `detect_map_format`

Doom 64 detection is a free helper (e.g. `is_doom64_map_lump(bytes: &[u8]) -> bool`
checking for leading `IWAD`/`PWAD` magic on a non-empty map lump), **kept out of
`detect_map_format`/`MapFormat`/`MapGroup`**, which model the flat marker+run
maps and cannot represent a nested-WAD map. Adding `MapFormat::Doom64` is deferred
to whenever Doom 64 is normalized into the `Map` graph.

### 4. Scope — raw records only; no `Map`-graph normalization

Per the prior decision and Option 1: #54 delivers record structs, nested-WAD
reading, detection, tests, fuzz, and docs — but **no** normalization into the
`Map` graph, **no** `MapFormat::Doom64`, and **no** `assemble` path. Doom 64's
`u16` texture/flat indices and colored-lighting IDs are stored raw; giving them
meaning needs a texture/graphics layer (a later milestone), which a future issue
will build on top of these raw records.

## Consequences

- Doom 64 map records become readable and testable now, without waiting on the
  texture/graphics layer, and without contorting the `Map` graph.
- `map::doom64` is self-contained and add-only; nothing about the existing Doom /
  Hexen / UDMF paths changes.
- The nested-WAD read reuses `Wad::from_bytes` + `parse_records`, so the Doom 64
  path inherits the crate's existing hardening (O(input) records, no-panic
  parsing) with no new container parser to audit.
- A consumer wanting a normalized Doom 64 `Map` (with resolved textures/colors)
  is explicitly a future issue; this ADR does not promise it.
- Because detection is a standalone helper, callers read Doom 64 maps through a
  dedicated `map::doom64` entrypoint rather than the generic `Map::assemble`.

## Pros and cons of the options

### Option 1 — standalone raw-record module (chosen)
- **Good:** smallest surface; no graph/format coupling; matches "raw records
  only"; reuses the WAD reader; unblocks reading immediately.
- **Bad:** Doom 64 is not reachable via the generic `Map::assemble` API; a second
  read entrypoint exists until a future normalization issue.

### Option 2 — raw records + detection integration
- **Good:** surfaces `MapFormat::Doom64` in the format API now.
- **Bad:** couples the flat `MapGroup`/`detect_map_format` model to a nested-WAD
  map it cannot represent; ships a `MapFormat` variant that cannot assemble.

### Option 3 — full integration
- **Good:** one unified `Map::assemble` path for all formats.
- **Bad:** forces `Map`-graph changes to carry texture/flat/color **indices** no
  consumer can resolve yet — the "largest unknown" scope risk; large and
  speculative.

## More information

- **Source:** measured from a real retail `DOOM64.WAD` (N64 lineage IWAD) via
  `cwad info`/`list`/`extract` and direct byte inspection of the extracted MAP01
  nested WAD. The file is not redistributable and is not committed (root WAD blobs
  are gitignored per #226); only structural facts are recorded here.
- **Hardening (ADR-0016) obligations for #54:** a `cargo-fuzz` target over the
  Doom 64 map-reading path (arbitrary bytes → nested-WAD parse → record parse)
  with the no-panic + O(input) oracles and a seed corpus; both strictness modes
  non-panicking. `#![deny(unsafe_code)]` holds (no `unsafe`).
- **Deferred / future issues:** normalizing Doom 64 into the `Map` graph
  (needs the texture/flat index tables and the `LIGHTS` colored-lighting palette);
  decoding `LEAFS` render leaves; decoding/executing `MACROS` scripts (the ACS
  strand tracked separately); the 2020 KEX re-release's differing container.
- Relates to ADR-0014 (format axis, Doom 64 non-goals), ADR-0015 (`Map` graph
  this pass does **not** touch), ADR-0016 (hardening), ADR-0013 (lump-by-name
  lookup, reused for the nested WAD's sub-lumps).
