# Map Record Parsing

Doom maps are stored as a group of sequentially named lumps. After the marker lump
(e.g. `E1M1`) come the parsed map data lumps. The table below covers the record lumps
that `crustywad` decodes; classic Doom maps also include additional lumps such as
`REJECT` and `BLOCKMAP` after `SECTORS`. Unlike the flat record lumps below, `REJECT`
and `BLOCKMAP` decode into typed, queryable structures (`MapReject` sector-visibility
lookups, `MapBlockmap` per-block linedef lists) during map assembly — see
[REJECT and BLOCKMAP](#reject-and-blockmap) below.

| Lump | Record type | Record size |
|---|---|---|
| `THINGS` | `Thing` | 10 bytes |
| `LINEDEFS` | `Linedef` | 14 bytes |
| `SIDEDEFS` | `Sidedef` | 30 bytes |
| `VERTEXES` | `Vertex` | 4 bytes |
| `SEGS` | `Seg` | 12 bytes |
| `SSECTORS` | `Subsector` | 4 bytes |
| `NODES` | `Node` | 28 bytes |
| `SECTORS` | `Sector` | 26 bytes |

## Parsing records

`crustywad::map::parse_records::<T>` decodes a byte slice into a `Vec<T>`.
All record types implement `BinRead` with little-endian byte order.

```rust
use crustywad::map;

// Parse a raw THINGS byte slice containing a single thing.
let thing_bytes: &[u8] = &[
    100_i16.to_le_bytes()[0], 100_i16.to_le_bytes()[1],  // x = 100
    200_i16.to_le_bytes()[0], 200_i16.to_le_bytes()[1],  // y = 200
    0, 0,                                                  // angle = 0
    1, 0,                                                  // type_id = 1 (player 1 start)
    7, 0,                                                  // flags = 0x0007
];

let things: Vec<map::doom::Thing> = map::parse_records(thing_bytes)?;
let t = &things[0];
println!("Player 1 start at ({}, {}), angle {}", t.x, t.y, t.angle);
# Ok::<(), crustywad::map::MapParseError>(())
```

## Available record types

### Thing

```rust,ignore
pub struct Thing {
    pub x: i16,        // X coordinate in map units
    pub y: i16,        // Y coordinate in map units
    pub angle: u16,    // Facing angle in degrees (0-359, counter-clockwise from east)
    pub type_id: u16,  // Editor number / thing type
    pub flags: u16,    // Doom thing flags
}
```

### Linedef

```rust,ignore
pub struct Linedef {
    pub start_vertex: u16,   // Start vertex index
    pub end_vertex: u16,     // End vertex index
    pub flags: u16,
    pub special_type: u16,   // Special action
    pub sector_tag: u16,
    pub right_sidedef: u16,  // Right sidedef index
    pub left_sidedef: u16,   // 0xffff when absent
}
```

### Sidedef

```rust,ignore
pub struct Sidedef {
    pub x_offset: i16,
    pub y_offset: i16,
    pub upper_texture: Name8,   // 8-byte NUL-padded name
    pub lower_texture: Name8,
    pub middle_texture: Name8,
    pub sector: u16,
}
```

### Vertex

```rust,ignore
pub struct Vertex {
    pub x: i16,
    pub y: i16,
}
```

### Sector

```rust,ignore
pub struct Sector {
    pub floor_height: i16,
    pub ceiling_height: i16,
    pub floor_texture: Name8,
    pub ceiling_texture: Name8,
    pub light_level: i16,
    pub special_type: i16,
    pub tag: i16,
}
```

See `crustywad::map` in the API docs for the full definitions of `Seg`,
`Subsector`, and `Node`.

## Error handling

`parse_records` returns `MapParseError`:

- `MapParseError::TrailingBytes` — the lump length is not an exact multiple of the record
  size (e.g. a `THINGS` lump whose byte count is not divisible by 10).
- `MapParseError::Binrw` — `binrw` failed to decode a record from the byte stream.

Both variants implement `std::error::Error` and display a human-readable message.

## Assembling a map graph

The record types above are flat and unresolved — a `Linedef`'s `start_vertex` is just a
`u16` index. `crustywad::map` also assembles those flat records into a normalized `Map`
graph, resolving cross-references between vertices, sidedefs, and sectors so callers don't
have to index arenas by hand.

> **Multi-format assembly.** `Map::assemble` detects the map format from its lumps: the marker
> lump is checked first under the Doom 64 dual condition — a `MAPxx` name **and** nested
> `IWAD`/`PWAD` magic in its bytes, the same rule grouping applies (see
> [Doom 64 maps](#doom-64-maps) below); otherwise a `TEXTMAP` lump marks a UDMF map, a
> `BEHAVIOR` lump marks a Hexen map, and anything else is treated as the classic Doom binary
> layout. The assembled `Map` carries its format via `map.format()`, which returns
> `MapFormat::Doom` for classic Doom/Doom II/Heretic maps (which share the same binary record
> layout and differ only in map-marker naming, e.g. `MAP01` vs `E1M1`), `MapFormat::Hexen` for
> Hexen maps, `MapFormat::Udmf` for UDMF (`TEXTMAP`) maps, or `MapFormat::Doom64` for Doom 64
> maps. UDMF maps can also be written back out with `write_udmf` / `add_udmf_map` (the `write`
> feature).

### Finding a map's lumps

A WAD stores maps as a marker lump (e.g. `E1M1`, `MAP01`) followed by a run of data lumps
(`THINGS`, `LINEDEFS`, `SIDEDEFS`, `VERTEXES`, `SECTORS`, and friends). `Wad::map_groups`
and `Wad::map_group` locate these runs and return one `MapGroup` per map:

```rust,ignore
pub struct MapGroup {
    pub marker_index: usize,   // directory index of the marker lump
    pub name: String,          // the map's name, e.g. "E1M1"
    pub data_indices: Vec<usize>,  // directory indices of the map's data lumps, in order
}
```

```rust
use crustywad::Wad;

# let wad = Wad::from_bytes(b"PWAD\x00\x00\x00\x00\x0c\x00\x00\x00".to_vec()).unwrap();
// All maps in the WAD.
for group in wad.map_groups() {
    println!("found map {}", group.name);
}

// A single named map.
if let Some(group) = wad.map_group("E1M1") {
    println!("E1M1 has {} data lumps", group.data_indices.len());
}
```

### Directory sections

Besides map groups, a WAD's lump directory brackets other kinds of content between
marker lumps (typically zero-size; recognized by name): `F_START`/`F_END` for flats,
`S_START`/`S_END` for sprites,
`P_START`/`P_END` for patches, and Doom 64's `T_START`/`T_END` (world textures) and
`DS_START`/`DS_END` (digital sounds), each with nested numbered sub-namespaces
(`F1_`/`F2_`/`P1_`/`P2_`/...) and Boom's doubled-letter aliases (`FF_`, `PP_`, `SS_`).
`Wad::sections` / `Wad::sections_with_options` scan a **single** WAD's directory and
return a `SectionTable` of `Section`s, each carrying its `SectionKind`, the directory
range of its marker pair, its content lumps, and any nested sub-sections:

```rust
use crustywad::{SectionKind, Wad};

# let wad = Wad::from_bytes(b"PWAD\x00\x00\x00\x00\x0c\x00\x00\x00".to_vec()).unwrap();
let table = wad.sections()?;
for flats in table.of_kind(SectionKind::Flats) {
    println!("flats section spans lumps {:?}", flats.lumps);
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

Both reference engines locate a section's extent by unguarded subtraction of two
independently looked-up marker positions, with no check for a missing, inverted, or
duplicated marker (ADR-0022 §2) — this API replaces that anti-pattern with a validated
scan: `Wad::sections` (strict) returns the first `SectionError` on a malformed marker
layout (an unpaired start/end, a duplicate or nested pair, or cross-kind interleaving),
while `Wad::sections_with_options` under
`ParseOptions::lenient()` never errors — it recovers a best-effort `SectionTable` and
records each anomaly as a `SectionWarning` instead. A balanced numbered pair with no
enclosing parent of its kind (e.g. a bare `P3_START..P3_END`, as shipped by SVE.wad) is
**not** an anomaly — engines model no parent/child relationship between markers, so it is
read as a first-class top-level section in both modes. As with map groups, section
scanning is scoped to one WAD's directory; multi-WAD load-order overlay is out of
scope here (tracked on the editor epic's future lump/resource manager, #65).

### Assembling a `Map`

`Map::assemble` builds a graph from a `MapGroup`'s `THINGS`, `LINEDEFS`, `SIDEDEFS`,
`VERTEXES`, and `SECTORS` lumps, decoding the flat records and validating every
cross-reference between them:

```rust
use crustywad::Wad;
use crustywad::map::Map;

# let wad = Wad::from_bytes(b"PWAD\x00\x00\x00\x00\x0c\x00\x00\x00".to_vec()).unwrap();
if let Some(group) = wad.map_group("E1M1") {
    let map = Map::assemble(&wad, &group)?;

    for linedef in map.linedefs() {
        let (start, end) = map.linedef_vertices(linedef);
        if let Some(right) = map.linedef_right(linedef) {
            println!(
                "line ({}, {}) -> ({}, {}), front sector floor {}",
                start.x, start.y, end.x, end.y,
                map.sidedef_sector(right).floor_height
            );
        }
    }
}
# Ok::<(), crustywad::map::MapAssembleError>(())
```

`Map` exposes each normalized arena — `vertices()`, `linedefs()`, `sidedefs()`,
`sectors()`, `things()` — plus infallible resolvers that follow indices between them:

| Resolver | Follows |
|---|---|
| `map.linedef_vertices(linedef)` | `(start, end)` vertex pair |
| `map.linedef_right(linedef)` | right (front) sidedef, or `None` |
| `map.linedef_left(linedef)` | left (back) sidedef, or `None` |
| `map.sidedef_sector(sidedef)` | the sidedef's sector |

The resolvers are total for elements obtained from this map's own accessors
(`map.linedefs()`, `map.sidedefs()`, …): they never panic or return an out-of-range
index, because assembly validated every cross-reference before `Map` was constructed.
(Because `MapLinedef`/`MapSidedef` have public index fields, passing a hand-constructed
value with an out-of-range index can still panic.)

### Texture references

`MapSidedef`'s `upper`/`lower`/`middle` fields and `MapSector`'s `floor_flat`/`ceiling_flat` field
are a `TextureRef`, not a bare string: `TextureRef::Name(String)` for a name (Doom/Hexen's 8-byte
lump name, a UDMF string, or a Doom 64 texture/flat hash resolved against a `Textures` section —
see [Doom 64 maps](#doom-64-maps) below), or `TextureRef::Index(u16)` for a Doom 64 texture/flat
table hash that couldn't be resolved. Classic Doom, Hexen, and UDMF maps always produce `Name`.
`TextureRef::as_name()` returns `Some(&str)` for `Name` and `None` for `Index`, and `TextureRef`
implements `PartialEq<&str>` against the name, so a Doom/Hexen/UDMF texture can be compared
directly against a string literal:

```rust
use crustywad::Wad;
use crustywad::map::Map;

# let wad = Wad::from_bytes(b"PWAD\x00\x00\x00\x00\x0c\x00\x00\x00".to_vec()).unwrap();
if let Some(group) = wad.map_group("E1M1") {
    let map = Map::assemble(&wad, &group)?;
    for sector in map.sectors() {
        if sector.floor_flat == "LAVA1" {
            println!("lava sector, ceiling flat: {:?}", sector.ceiling_flat.as_name());
        }
    }
}
# Ok::<(), crustywad::map::MapAssembleError>(())
```

### One-sided (and sideless) lines

On disk, either of a `Linedef`'s sidedef fields may hold the sentinel value `0xffff`,
meaning "no sidedef on this side". A `left_sidedef` of `0xffff` is the everyday case — a
one-sided line, such as an outer wall. A `right_sidedef` of `0xffff` is rare but
engine-sanctioned (vanilla guards both fields identically): retail maps use it for
invisible blocking lines with no render surfaces at all. Assembly translates the sentinel
into `Option<SidedefIdx>` on both fields — `MapLinedef.left` and `MapLinedef.right` are
each `None` when their side is absent — and `map.linedef_left(linedef)` /
`map.linedef_right(linedef)` mirror this by returning `Option` rather than an error.

### Extended thing and linedef fields

Hexen maps extend the classic Doom binary record layout with additional fields on things and
linedefs. When assembled, a `MapThing` includes `id` (thing ID for cross-references), `height`
(vertical position), and `special` (a `Special` carrying the action number and its five
`args`). A `MapLinedef` likewise has a `special: Special`, plus an `id` — a UDMF/ZDoom line
identifier that is `0` for Doom and Hexen maps (reserved for UDMF). `Special` is shared
across formats: for a Doom *linedef*, its target sector tag lives in `special.args[0]`;
Hexen and UDMF populate the full `args`.

On Doom maps the thing fields (`id`, `height`, and the thing `special`) are all zero, while a
linedef's `special` still holds its classic action number and sector tag (the latter in
`special.args[0]`). Hexen maps additionally populate the thing fields with real values. Use
`map.format()` to decide how to interpret them:

```rust
use crustywad::map::{Map, MapFormat};

# let wad = crustywad::Wad::from_bytes(b"PWAD\x00\x00\x00\x00\x0c\x00\x00\x00".to_vec())?;
if let Some(group) = wad.map_group("MAP01") {
    let map = Map::assemble(&wad, &group)?;

    for thing in map.things() {
        if map.format() == MapFormat::Hexen {
            println!("Hexen thing ID: {}, height: {}", thing.id, thing.height);
        }
    }

    for linedef in map.linedefs() {
        if map.format() == MapFormat::Hexen {
            println!(
                "Hexen line special: {}, args: {:?}",
                linedef.special.special, linedef.special.args
            );
        }
    }
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

### Strict vs. lenient assembly

`Map::assemble(wad, group)` is a convenience wrapper that always uses strict mode.
`Map::assemble_with_options(wad, group, options)` takes a `ParseOptions` and honors its
`strictness`, the same as the raw `Wad` and `parse_records` APIs:

- **Strict** (`Map::assemble`, or `assemble_with_options` with `Strictness::Strict`): the
  first out-of-range cross-reference (e.g. a linedef's vertex index past the end of
  `VERTEXES`) aborts assembly with `MapAssembleError::DanglingReference`. A missing
  required lump or an undecodable record lump also aborts, in both modes, with
  `MapAssembleError::MissingLump` or `MapAssembleError::Records`.
- **Lenient** (`assemble_with_options` with `Strictness::Lenient`): an out-of-range
  cross-reference is clamped to a valid fallback index instead of failing, and a
  `MapWarning::DanglingReference` is recorded. Structural failures (missing lump,
  undecodable records, or a required target arena that is empty) still return
  `MapAssembleError` even in lenient mode.

```rust
use crustywad::map::Map;
use crustywad::{ParseOptions, Wad};

# let wad = Wad::from_bytes(b"PWAD\x00\x00\x00\x00\x0c\x00\x00\x00".to_vec()).unwrap();
if let Some(group) = wad.map_group("E1M1") {
    let map = Map::assemble_with_options(&wad, &group, ParseOptions::lenient())?;
    for warning in map.warnings() {
        eprintln!("{warning}");
    }
}
# Ok::<(), crustywad::map::MapAssembleError>(())
```

`map.warnings()` returns the `MapWarning`s collected during a lenient assembly (empty for
a clean map, and always empty after a strict `Map::assemble`, since strict mode returns an
error instead of recording a warning).

### Doom 64 maps

Doom 64 stores each map as a **nested WAD**: the `MAPxx` marker lump's bytes are themselves a
complete WAD (leading `IWAD`/`PWAD` magic), whose sub-lumps hold the map's records, rather than a
marker followed by a run of sibling data lumps in the outer directory. `Wad::map_groups` /
`Wad::map_group` recognize a Doom 64 map only when **both** signals hold: the marker's name
matches `MAPxx` (`MAP` plus two ASCII digits) *and* its lump bytes carry the nested WAD magic — so
an ordinary empty classic `MAPxx` marker is never misread as Doom 64. A Doom 64 `MapGroup`'s
`data_indices` is always empty, since its data lives inside the marker lump rather than the flat
directory. `map.format()` reports `MapFormat::Doom64` for these maps, and both `Map::assemble` and
`Map::assemble_with_options` assemble them into the same `Map` graph as every other format, in
both strictness modes.

Doom 64 adds per-map colored lighting. `Map::lights()` returns the map's full light table built
the way the engine builds it (mirroring Doom64 EX's `P_LoadLights`): 256 implicit grayscale
entries (`r = g = b = index`, `tag = 0`), followed by the map's `LIGHTS` lump records starting at
index 256. `MapSector.colors` is `Some([LightIdx; 5])` for a Doom 64 sector — five references into
`Map::lights()` — and `None` for every other format. The five slots are carried **positionally**:
Doom 64's own format headers don't name them, so `crustywad` doesn't invent slot meanings either.
`MapSector.light` (the classic scalar light level) is always `0` for a Doom 64 sector, since the
format has no such field; `MapSector.flags` carries the sector's raw Doom 64 flag bits (opaque,
uninterpreted).

Doom 64's sidedef/sector texture and flat fields carry a `u16` hash on disk rather than a name.
When the containing WAD has a `Textures` section (a `T_START`/`T_END`-delimited run — see
[Directory sections](#directory-sections) above), assembly resolves every hash to
the matching texture/flat name in `Textures`, first-match-in-disk-order on a collision, and the
field becomes `TextureRef::Name` like every other format. A miss against a present section is a
strict `MapAssembleError::UnresolvedTextureHash` / lenient `MapWarning::UnresolvedTextureHash`
(keeping `TextureRef::Index`); a WAD with no `Textures` section at all keeps every field as
`TextureRef::Index` silently, since a bare nested-map WAD (no textures alongside it) is a
legitimate input.

A Doom 64-sourced `Map` can be serialized back out (`write_doom_map`/`write_udmf`, the `write`
feature) once its texture references resolve to names. The one remaining unrepresentable piece is
colored lighting (`MapSector.colors`, described above): strict mode refuses with
`UnrepresentableField` (`block: "sector", field: "colors"`), lenient mode drops the colors and
records one `ColoredLightingDropped` warning per map, then converts. A leftover unresolved
`TextureRef::Index` (no `Textures` section, or an unresolved hash kept under lenient assembly)
still hits a defensive `UnresolvedTextureIndex` writer error in both modes.

Doom 64 also decodes the `LEAFS` lump — its render leaves — onto the graph. `Map::leafs()` is a
per-subsector arena of `MapLeaf { vertex: VertexIdx, seg: Option<SegIdx> }`, and each
`MapSubsector::leafs` range selects that subsector's slice, mirroring the existing `segs` range
below. The on-disk seg field's `-1` sentinel becomes `seg: None` ("no seg": the edge is implicit
geometry). The lump's record count must equal the map's subsector count — the engine treats a
mismatch as fatal, and this reader mirrors it: strict mode rejects with
`MapAssembleError::LeafCountMismatch`, lenient mode discards the whole `LEAFS` arena and records
one warning, the same whole-arena degrade policy as the BSP data below. `Map::leafs()` and every
`MapSubsector::leafs` range are empty for every source format other than Doom 64.

Doom 64 also decodes the `MACROS` lump — its scripted action sequences — onto the graph.
`Map::macros()` returns the decoded macros as a slice, in lump order, each `MapMacro { actions: Vec<MapMacroAction> }`
holding `MapMacroAction { id, tag, special }` entries; the engine's loader reads one more action
than the macro's on-disk count states (`count + 1`), and this decode preserves that read exactly.
Decoding stops at the data: `crustywad` builds no interpreter or execution machinery for these
scripts, since running them is the ACS epic's job (#248), not a WAD-reading concern. `Map::macros()`
is empty for every source format other than Doom 64.

### BSP data

Beyond the geometry arenas, `Map` also exposes the engine-built BSP (Binary Space Partitioning)
tree: `map.segs()`, `map.subsectors()`, and `map.nodes()`, normalized from the `SEGS`, `SSECTORS`,
and `NODES` lumps. These are populated for classic Doom/Heretic, Hexen, and Doom 64 maps alike —
Doom 64's BSP records share the classic on-disk layout, so they normalize through the same code
path. A UDMF map's BSP data, *when present*, lives in its own `ZNODES` lump instead, carrying the
same ZDoom extended/GL node encoding described below — see
[Extended node encodings](#extended-node-encodings). Like the classic BSP lumps, it is optional: a
UDMF map with no `ZNODES` lump simply has empty `segs()`/`subsectors()`/`nodes()`.

`map.bsp_root()` returns the index of the tree's root node — `Some(NodeIdx)` if `map.nodes()` is
non-empty, `None` otherwise. By convention the root is the **last** node in the arena, matching
Chocolate Doom's `R_RenderPlayerView`, which starts traversal at `R_RenderBSPNode(numnodes - 1)`.

Like `SEGS`/`SSECTORS`/`NODES` themselves, these three arenas are optional: many editable PWADs
ship without built nodes, so their absence is not an assembly error — `map.segs()`,
`map.subsectors()`, and `map.nodes()` are simply empty, and `map.bsp_root()` returns `None`. A map
produced by converting another format to Doom (`add_doom_map`) ships zero-length
`SEGS`/`SSECTORS`/`NODES` placeholder lumps — real BSP data requires an external nodebuilder pass
— so re-assembling that output also yields empty arenas.

A `MapNode`'s `right`/`left` fields are `NodeChild`, not a bare index: `NodeChild::Node(NodeIdx)`
for an internal child, or `NodeChild::Subsector(SubsectorIdx)` for a leaf. Assembly decodes the
on-disk child word's bit 15 once — set selects a subsector, clear selects a node — so callers
match on `NodeChild` instead of re-checking the bit themselves:

```rust
use crustywad::Wad;
use crustywad::map::{Map, NodeChild};

# let wad = Wad::from_bytes(b"PWAD\x00\x00\x00\x00\x0c\x00\x00\x00".to_vec()).unwrap();
if let Some(group) = wad.map_group("MAP01") {
    let map = Map::assemble(&wad, &group)?;
    if let Some(root) = map.bsp_root() {
        match map.nodes()[root.0].right {
            NodeChild::Node(i) => println!("right child is node {}", i.0),
            NodeChild::Subsector(i) => println!("right child is subsector {}", i.0),
        }
    }
}
# Ok::<(), crustywad::map::MapAssembleError>(())
```

#### Extended node encodings

A `NODES` or `SSECTORS` lump (or, for UDMF, a `ZNODES` lump) can instead carry an extended/GL
node encoding — the ZDBSP family: `XNOD`, `ZNOD`, `XGLN`, `ZGLN`, `XGL2`, `XGL3`, `ZGL2`, `ZGL3` —
identified by a 4-byte signature at the head of the lump. `crustywad`'s classic-path BSP
normalizer never attempts to decode these as fixed-size classic records — doing so would
misread the signature bytes as garbage geometry.

The four **uncompressed** dialects — `XNOD` (non-GL) and the GL layouts `XGLN`, `XGL2`, `XGL3` —
now decode transparently into the same `map.segs()`, `map.subsectors()`, and `map.nodes()`
arenas as the classic encoding, on both the binary `NODES`/`SSECTORS` path and the UDMF
`ZNODES` path. There is nothing extra to opt into: assembly detects the signature and decodes
the stream in place, in both `Strictness` modes. One difference from a classic-decoded map is
worth knowing: a GL dialect's segs can include **minisegs** — synthetic segs that run along a
BSP partition line rather than following a linedef — so `MapSeg::linedef` is `Option<LinedefIdx>`
(`None` for a miniseg) rather than always `Some`.

The four compressed **`Z*`** dialects (`ZNOD`, `ZGLN`, `ZGL2`, `ZGL3` — zlib-wrapped twins of the
`X*` streams above) are still **gated**, not parsed: detecting one of those signatures gates the
whole BSP normalization step — in strict mode assembly fails with
`MapAssembleError::UnsupportedNodeEncoding`; in lenient mode assembly leaves `map.segs()`,
`map.subsectors()`, and `map.nodes()` empty and records one `MapWarning::UnsupportedNodeEncoding`
for the gated lump (a map's extended stream lives in a single lump, so assembly stops at the
first signature it finds and warns once). DeePBSP's `xNd4`
is not yet detected as an extended encoding at all: a lump beginning with that tag falls through
to the classic record decoder rather than tripping this gate, pending a later stage
([#328](https://github.com/masriamir/crustywad/issues/328)). Reading the remaining `Z*`
encodings is tracked as [#199](https://github.com/masriamir/crustywad/issues/199); see ADR-0025
for the staged design.

The same whole-BSP posture applies when BSP data is internally unrecoverable in lenient mode:
a reference that cannot be clamped (for example, a node child pointing into an absent
`SSECTORS` arena) drops all three arenas, records the dangling reference as a warning, and the
rest of the map still assembles. BSP data is optional (ADR-0015 §5), so it never fails a
lenient assembly.

### REJECT and BLOCKMAP

Like the BSP lumps above, `REJECT` and `BLOCKMAP` decode into typed, queryable structures during
map assembly rather than staying raw bytes: `map.reject()` returns `Option<&MapReject>` and
`map.blockmap()` returns `Option<&MapBlockmap>`, `None` when the map carries no (or an empty)
lump of that kind — an editable PWAD with no built REJECT/BLOCKMAP table is as normal as one with
no built nodes.

`MapReject` is a row-major sector-visibility bit matrix, `sector_count × sector_count` bits,
LSB-first within each byte (layout verified against Chocolate Doom's `P_LoadReject` /
`P_CheckSight`):

```rust
use crustywad::Wad;
use crustywad::map::{Map, SectorIdx};

# let wad = Wad::from_bytes(b"PWAD\x00\x00\x00\x00\x0c\x00\x00\x00".to_vec()).unwrap();
if let Some(group) = wad.map_group("E1M1") {
    let map = Map::assemble(&wad, &group)?;
    if let Some(reject) = map.reject() {
        for i in 0..reject.sector_count() {
            let sector = SectorIdx(i);
            if reject.is_rejected(sector, sector) == Some(true) {
                println!("sector {i} pre-rejects sight to itself");
            }
        }
    }
}
# Ok::<(), crustywad::map::MapAssembleError>(())
```

`MapBlockmap` is a grid of 128-map-unit blocks, each holding the linedefs that cross it (layout
verified against Chocolate Doom's `P_LoadBlockMap` / `P_BlockLinesIterator`). `map.blockmap()`
exposes `origin()`, `columns()`/`rows()`, `block(col, row)` (grid-indexed lookup), and
`block_at(x, y)` (map-space coordinate lookup, `None` outside the grid or for non-finite
coordinates):

```rust
use crustywad::Wad;
use crustywad::map::Map;

# let wad = Wad::from_bytes(b"PWAD\x00\x00\x00\x00\x0c\x00\x00\x00".to_vec()).unwrap();
if let Some(group) = wad.map_group("E1M1") {
    let map = Map::assemble(&wad, &group)?;
    if let Some(blockmap) = map.blockmap() {
        if let Some(linedefs) = blockmap.block_at(0.0, 0.0) {
            println!("{} linedefs cross the block at the origin", linedefs.len());
        }
    }
}
# Ok::<(), crustywad::map::MapAssembleError>(())
```

Internally `MapBlockmap` stores the lump's words once and each block holds a validated range
into them, so offset aliasing (ZDBSP-style whole-list sharing) and tail sharing (ZokumBSP-style
partial-list sharing) cost no extra memory (ADR-0016 §1).

Both types honor the same strict/lenient policy as the rest of assembly: an undersized `REJECT`
table is a strict error (`MapAssembleError::UndersizedReject`) or a lenient warning with the
missing bits treated as "not rejected" (`MapWarning::UndersizedReject`); a malformed `BLOCKMAP`
header, an out-of-lump block offset, an unterminated block list, or a block list referencing a
nonexistent linedef are each a strict error
(`MapAssembleError::MalformedBlockmap` / `BlockmapBlockOffset` / `UnterminatedBlockmapList` /
`DanglingReference`) or a lenient recovery with a matching `MapWarning` — discarding the whole
table, truncating the list, or emptying the one affected block, respectively. An empty `REJECT`
or `BLOCKMAP` lump (as `crustywad`'s own writer emits, ADR-0019 §4) is read back as simply
absent, in both modes, with no warning.
