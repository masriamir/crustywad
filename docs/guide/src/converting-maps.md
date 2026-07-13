# Converting maps

`crustywad::map` can convert an assembled [`Map`](map-records.md) between the
UDMF text format and the classic Doom binary format, in both directions. Both
directions are behind the `write` feature:

```toml
crustywad = { version = "0.1", features = ["write"] }
```

Conversion is `read → Map → write`: there is no direct format-to-format path.
A UDMF field that the `Map` graph does not model is already lost at *read*
time (see [Map Record Parsing](map-records.md)); conversion only polices loss
that is visible in the graph. See [ADR-0019](https://github.com/masriamir/crustywad/blob/main/docs/adr/0019-map-format-conversion.md)
for the full decision record this page summarizes.

## Doom → UDMF

`write_udmf()` and `add_udmf_map()` (covered in [Writing WAD Files](writing-wads.md#writing-udmf-maps))
accept a `Map` assembled from *any* source format, including a classic Doom
map:

```rust
use crustywad::{Wad, WadBuilder, WadKind, WriteOptions};
use crustywad::map::{Map, add_udmf_map, write_udmf};

# // A minimal classic Doom map: one linedef, one sector, one thing.
# let vertexes = [0i16, 0, 64, 0].iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<u8>>();
# let mut linedefs = Vec::new();
# linedefs.extend_from_slice(&0u16.to_le_bytes());
# linedefs.extend_from_slice(&1u16.to_le_bytes());
# linedefs.extend_from_slice(&1u16.to_le_bytes());
# linedefs.extend_from_slice(&0u16.to_le_bytes());
# linedefs.extend_from_slice(&0u16.to_le_bytes());
# linedefs.extend_from_slice(&0u16.to_le_bytes());
# linedefs.extend_from_slice(&0xffffu16.to_le_bytes());
# let mut sidedefs = Vec::new();
# sidedefs.extend_from_slice(&0i16.to_le_bytes());
# sidedefs.extend_from_slice(&0i16.to_le_bytes());
# sidedefs.extend_from_slice(b"-\0\0\0\0\0\0\0");
# sidedefs.extend_from_slice(b"-\0\0\0\0\0\0\0");
# sidedefs.extend_from_slice(b"STARTAN3");
# sidedefs.extend_from_slice(&0u16.to_le_bytes());
# let mut sectors = Vec::new();
# sectors.extend_from_slice(&0i16.to_le_bytes());
# sectors.extend_from_slice(&128i16.to_le_bytes());
# sectors.extend_from_slice(b"FLOOR4_8");
# sectors.extend_from_slice(b"CEIL3_5\0");
# sectors.extend_from_slice(&160i16.to_le_bytes());
# sectors.extend_from_slice(&0i16.to_le_bytes());
# sectors.extend_from_slice(&0i16.to_le_bytes());
# let things = vec![0u8; 10];
# let mut src = WadBuilder::new(WadKind::Pwad);
# src.add_lump("MAP01", b"");
# src.add_lump("THINGS", things);
# src.add_lump("LINEDEFS", linedefs);
# src.add_lump("SIDEDEFS", sidedefs);
# src.add_lump("VERTEXES", vertexes);
# src.add_lump("SECTORS", sectors);
# let wad = Wad::from_bytes(src.build()?)?;
# let group = wad.map_group("MAP01").unwrap();
let map: Map = Map::assemble(&wad, &group)?;

let (textmap, _warnings) = write_udmf(&map, &WriteOptions::strict())?;
assert!(textmap.starts_with("namespace"));

let mut builder = WadBuilder::new(WadKind::Pwad);
add_udmf_map(&mut builder, "MAP01", &map, &WriteOptions::strict())?;
let bytes = builder.build()?;
# assert!(!bytes.is_empty());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## UDMF → Doom

`write_doom_map()` serializes an assembled `Map` into the five classic Doom
map data lumps (`THINGS`, `LINEDEFS`, `SIDEDEFS`, `VERTEXES`, `SECTORS`);
`add_doom_map()` adds a complete map group to a `WadBuilder`. Both are
available with the `write` feature:

```rust
use crustywad::{Wad, WadBuilder, WadKind, WriteOptions};
use crustywad::map::{Map, add_doom_map, write_doom_map};

# let textmap = concat!(
#     "namespace = \"doom\";\n",
#     "vertex { x = 0; y = 0; }\n",
#     "vertex { x = 64; y = 0; }\n",
#     "sector { texturefloor = \"FLOOR4_8\"; textureceiling = \"CEIL3_5\"; }\n",
#     "sidedef { sector = 0; }\n",
#     "linedef { v1 = 0; v2 = 1; sidefront = 0; }\n",
#     "thing { x = 32; y = 32; type = 1; skill1 = true; skill2 = true; skill3 = true; }\n",
# );
# let mut src = WadBuilder::new(WadKind::Pwad);
# src.add_lump("MAP01", b"");
# src.add_lump("TEXTMAP", textmap.as_bytes().to_vec());
# src.add_lump("ENDMAP", b"");
# let wad = Wad::from_bytes(src.build()?)?;
# let group = wad.map_group("MAP01").unwrap();
let map: Map = Map::assemble(&wad, &group)?;

// Serialize to the five Doom binary map lumps:
let (lumps, warnings) = write_doom_map(&map, &WriteOptions::strict())?;
assert!(!lumps.vertexes.is_empty());
// Nodes are never built (see below): this warning is always present.
assert!(warnings.contains(&crustywad::map::DoomWriteWarning::NodesNotBuilt));

// Or add a complete map group to a builder:
let mut builder = WadBuilder::new(WadKind::Pwad);
add_doom_map(&mut builder, "MAP01", &map, &WriteOptions::strict())?;
let bytes = builder.build()?;
# assert!(!bytes.is_empty());
# Ok::<(), Box<dyn std::error::Error>>(())
```

> **Converted output is not engine-playable.** `add_doom_map` writes
> **zero-length `SEGS`, `SSECTORS`, `NODES`, `REJECT`, and `BLOCKMAP`**
> lumps — the canonical Doom lump run editors and nodebuilders expect to
> find, but with no node data in them. Every call returns
> `DoomWriteWarning::NodesNotBuilt`, in **both** strictness modes: it is
> a property of the output, not a defect strictness can fix. The result is
> **editor- and nodebuilder-ready, not engine-playable** — run an external
> nodebuilder (`zdbsp`, `bsp`, …) over it before loading it in a source
> port. crustywad has no nodebuilder; building nodes is tracked separately
> (issue #199).

## Round-tripping: not symmetric

> Doom → UDMF → Doom is a **byte-identical round-trip** for `VERTEXES`,
> `LINEDEFS`, `SIDEDEFS`, and `SECTORS`, and for `THINGS` within an envelope.
> **UDMF → Doom → UDMF is *not* reversible.** No option, flag, or mode makes
> it reversible.

**Doom → UDMF → Doom** reproduces the four geometry lumps exactly, and
`THINGS` too, provided the map stays inside the envelope where UDMF has a
representation for every Doom bit:

- Linedef flag bits 0–8 (the nine standard bits) round-trip; a bit ≥ 9 (e.g.
  Boom's `passuse`, `0x200`) has no UDMF boolean and is dropped.
- Thing flag bits 0–7 (skill 1–5, ambush, multiplayer-only, and the Boom/MBF
  dm/co-op/friend bits) round-trip; a bit ≥ 8 has no UDMF boolean and is
  dropped.
- A thing `angle` in `0..360` round-trips exactly; an angle ≥ 360 comes back
  as `angle % 360`. This is a **semantic no-op**, not data loss: Doom's
  `P_SpawnMapThing` computes the spawn facing as `ANG45 * (angle / 45)` with
  integer division, so `360` and `0` produce the identical facing. This case
  is not hypothetical — 226 things across 10 Freedoom maps store a literal
  `angle = 360`.

**UDMF → Doom → UDMF is one-way.** Converting a UDMF map to Doom and back
does not reproduce the original UDMF map: `f64` coordinates are rounded to
`i16` map units, and fields Doom has no slot for (tier 3 below) are dropped
permanently. There is no lossless UDMF → UDMF path through this API; a
caller needing full text-level fidelity should keep the parsed `UdmfMap`
intermediate instead of round-tripping through `Map`.

## Strict vs. lenient conversion

`write_doom_map()` / `add_doom_map()` share the crate's usual
[`WriteOptions`](writing-wads.md#strict-vs-lenient-write-validation)
strict/lenient contract. **Strict mode refuses any data loss** — a typical
ZDoom-namespace UDMF map, with linedef `args` or thing `height`/`id`/`special`
set, will **fail** strict conversion to Doom, naming the first offending
field. This is the intended design: `write_doom_map(&map, &WriteOptions::strict())`
returning `Ok` is exactly the answer to "does this map fit in the Doom
format?" `WriteOptions::lenient()` is the single-flag acknowledgment that the
loss is acceptable — it recovers a best-effort value for every lossy field
and reports each recovery as a `DoomWriteWarning`.

The Doom binary format is strictly narrower than the `Map` graph, so
narrowing it loses data in three tiers (from ADR-0019):

### Tier 1 — structurally impossible: errors in both modes

Doom's `u16` indices cannot address an arena beyond their range; there is no
honest recovery, so this errors in **both** strictness modes.

| Arena | Maximum | Why |
|---|---|---|
| vertices | 65,536 | indices `0..=65,535` |
| sectors | 65,536 | indices `0..=65,535` |
| sidedefs | 65,535 | `0xffff` is the "no sidedef" sentinel |

Reported as `DoomWriteError::TooManyElements { kind, count, max }`.

### Tier 2 — value loss: strict errors, lenient recovers and warns

| Loss | Lenient recovery |
|---|---|
| Fractional `f64` coordinate (vertex `x`/`y`, thing `x`/`y`) | round to nearest `i16` (half away from zero) |
| Coordinate outside `i16` range | clamp to `i16::MIN`/`i16::MAX` |
| Linedef `special` outside `u16`; `args[0]` (the sector tag) outside `u16` | clamp |
| Sidedef `x_offset` / `y_offset` outside `i16` | clamp |
| Sector `floor_height` / `ceiling_height` / `light` / `special` / `tag` outside `i16` | clamp |
| Thing `flags` with any bit above 15 set | truncate to `u16` |
| Texture/flat name longer than 8 bytes | truncate to 8 bytes |
| Non-finite (`NaN`/infinite) coordinate | strict errors, lenient substitutes `0` |

**Name fidelity has a caveat.** A texture/flat name round-trips byte-for-byte
**only if it is valid UTF-8 and NUL-clean** — valid UTF-8 up to its first NUL,
with nothing but NUL padding after it. Every name in practice is ASCII, so this
holds for real maps, but the exceptions are real and are not warned about:

- Doom's on-disk name field is a raw `[u8; 8]`, and `map::common::Name8` keeps
  those bytes verbatim — but the `Map` graph does **not**. `MapSidedef` and
  `MapSector` store `String`, filled on read via `Name8::as_str_lossy`, which
  trims at the first NUL and decodes with `String::from_utf8_lossy`.
- A name containing **invalid UTF-8** is therefore normalized on *read*:
  `b"\x81OCK\0\0\0\0"` becomes `"\u{FFFD}OCK"` in the graph and is written back
  as `EF BF BD 4F 43 4B 00 00` — different bytes, no warning. An 8-byte
  all-invalid name expands to a 24-byte replacement-character string and then
  **fails** as `DoomWriteError::NameTooLong` in strict mode.
- **Bytes after the NUL terminator** (which real IWADs do contain) are dropped
  on read for the same reason.

Only a name longer than 8 bytes is *conversion* loss; the two cases above are
read-time normalization, and no `WriteOptions` mode changes them.

### Tier 3 — no slot in the Doom format: strict errors, lenient drops and warns

A Doom linedef carries only `special_type` plus one sector tag; a Doom thing
carries no special, no tid, and no height. A nonzero value in any of the
following has nowhere to go:

- linedef `args[1..=4]` (nonzero)
- linedef `id`
- thing `special` and `args[0..=4]` (nonzero)
- thing `height` (nonzero)
- thing `id` (the tid)

This is exactly why a ZDoom-namespace UDMF map typically fails strict
conversion — "this map is not expressible in Doom format" is the correct
answer, and `WriteOptions::lenient()` is how a caller accepts that.

## Error handling

`write_doom_map()` and `add_doom_map()` return
`Result<(DoomMapLumps, Vec<DoomWriteWarning>), DoomWriteError>` and
`Result<Vec<DoomWriteWarning>, DoomWriteError>` respectively — the warnings
vector always contains at least `DoomWriteWarning::NodesNotBuilt`, in both
strictness modes. Each strict-mode `DoomWriteError` variant has exactly one
lenient-mode `DoomWriteWarning` counterpart, so the two modes read as a
single decision table rather than two separate implementations.

See [Map Record Parsing](map-records.md) for the `Map` graph types these APIs
consume, and [Writing WAD Files](writing-wads.md) for the general
`WadBuilder` / `WriteOptions` contract.
