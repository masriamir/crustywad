# Map Record Parsing

Doom maps are stored as a group of sequentially named lumps. After the marker lump
(e.g. `E1M1`) come the parsed map data lumps. The table below covers the record lumps
that `crustywad` decodes; classic Doom maps also include additional lumps such as
`REJECT` and `BLOCKMAP` after `SECTORS`:

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

```rust
pub struct Thing {
    pub x: i16,        // X coordinate in map units
    pub y: i16,        // Y coordinate in map units
    pub angle: u16,    // Facing angle in degrees (0-359, counter-clockwise from east)
    pub type_id: u16,  // Editor number / thing type
    pub flags: u16,    // Doom thing flags
}
```

### Linedef

```rust
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

```rust
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

```rust
pub struct Vertex {
    pub x: i16,
    pub y: i16,
}
```

### Sector

```rust
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

> **Multi-format assembly.** `Map::assemble` detects the binary map format from its lumps: a
> `BEHAVIOR` lump marks a Hexen map, otherwise it is treated as the classic Doom binary layout.
> The assembled `Map` carries its format via `map.format()`, which returns `MapFormat::Doom`
> for classic Doom/Doom II/Heretic maps (which share the same binary record layout and differ
> only in map-marker naming, e.g. `MAP01` vs `E1M1`), or `MapFormat::Hexen` for Hexen maps.
> A `TEXTMAP` (UDMF) map is not yet supported — assembly returns
> `MapAssembleError::UnsupportedFormat` for it until UDMF support lands (Epic #17).

### Finding a map's lumps

A WAD stores maps as a marker lump (e.g. `E1M1`, `MAP01`) followed by a run of data lumps
(`THINGS`, `LINEDEFS`, `SIDEDEFS`, `VERTEXES`, `SECTORS`, and friends). `Wad::map_groups`
and `Wad::map_group` locate these runs and return one `MapGroup` per map:

```rust
pub struct MapGroup {
    pub marker_index: usize,   // directory index of the marker lump
    pub name: String,          // the map's name, e.g. "E1M1"
    pub data_indices: Vec<usize>,  // directory indices of the map's data lumps, in order
}
```

```rust
use crustywad::Wad;

# let wad = Wad::from_bytes(Vec::<u8>::new()).unwrap();
// All maps in the WAD.
for group in wad.map_groups() {
    println!("found map {}", group.name);
}

// A single named map.
if let Some(group) = wad.map_group("E1M1") {
    println!("E1M1 has {} data lumps", group.data_indices.len());
}
```

### Assembling a `Map`

`Map::assemble` builds a graph from a `MapGroup`'s `THINGS`, `LINEDEFS`, `SIDEDEFS`,
`VERTEXES`, and `SECTORS` lumps, decoding the flat records and validating every
cross-reference between them:

```rust
use crustywad::Wad;
use crustywad::map::Map;

# let wad = Wad::from_bytes(Vec::<u8>::new()).unwrap();
if let Some(group) = wad.map_group("E1M1") {
    let map = Map::assemble(&wad, &group)?;

    for linedef in map.linedefs() {
        let (start, end) = map.linedef_vertices(linedef);
        let right = map.linedef_right(linedef);
        println!(
            "line ({}, {}) -> ({}, {}), front sector floor {}",
            start.x, start.y, end.x, end.y,
            map.sidedef_sector(right).floor_height
        );
    }
}
# Ok::<(), crustywad::map::MapAssembleError>(())
```

`Map` exposes each normalized arena — `vertices()`, `linedefs()`, `sidedefs()`,
`sectors()`, `things()` — plus infallible resolvers that follow indices between them:

| Resolver | Follows |
|---|---|
| `map.linedef_vertices(linedef)` | `(start, end)` vertex pair |
| `map.linedef_right(linedef)` | right (front) sidedef |
| `map.linedef_left(linedef)` | left (back) sidedef, or `None` |
| `map.sidedef_sector(sidedef)` | the sidedef's sector |

The resolvers are total for elements obtained from this map's own accessors
(`map.linedefs()`, `map.sidedefs()`, …): they never panic or return an out-of-range
index, because assembly validated every cross-reference before `Map` was constructed.
(Because `MapLinedef`/`MapSidedef` have public index fields, passing a hand-constructed
value with an out-of-range index can still panic.)

### One-sided lines

On disk, a `Linedef`'s `left_sidedef` field uses the sentinel value `0xffff` to mean "no
back sidedef" (a one-sided line, such as an outer wall). Assembly translates that sentinel
into `MapLinedef.left: Option<SidedefIdx>` — `None` for one-sided lines, `Some(idx)` for
two-sided lines. `map.linedef_left(linedef)` mirrors this: it returns `None` for a
one-sided line rather than an error.

### Hexen-specific fields

Hexen maps extend the classic Doom binary record layout with additional fields on things and
linedefs. When assembled, a `MapThing` includes Hexen-specific fields: `tid` (thing ID for
cross-references), `z` (vertical position), and the Hexen-only `special`/`args`. A `MapLinedef`'s
`special` field is a `LineSpecial`, which for Hexen maps carries the one-byte `special` action
number and its `args[5]` (action arguments); Doom maps use `LineSpecial`'s `tag` instead.

For Doom maps, these fields are always zero; for Hexen maps, they carry the actual Hexen
action semantics. You can check the map's format via `map.format()` to determine whether
to interpret these fields:

```rust
use crustywad::map::{Map, MapFormat};

# let wad = crustywad::Wad::from_bytes(Vec::<u8>::new())?;
# let group = wad.map_group("MAP01").unwrap();
let map = Map::assemble(&wad, &group)?;

for thing in map.things() {
    if map.format() == MapFormat::Hexen {
        println!("Hexen thing ID: {}, Z: {}", thing.tid, thing.z);
    }
}

for linedef in map.linedefs() {
    if map.format() == MapFormat::Hexen {
        println!("Hexen line special: {}, args: {:?}", linedef.special.special, linedef.special.args);
    }
}
# Ok::<(), crustywad::map::MapAssembleError>(())
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

# let wad = Wad::from_bytes(Vec::<u8>::new()).unwrap();
# let group = wad.map_group("E1M1").unwrap();
let map = Map::assemble_with_options(&wad, &group, ParseOptions::lenient())?;
for warning in map.warnings() {
    eprintln!("{warning}");
}
# Ok::<(), crustywad::map::MapAssembleError>(())
```

`map.warnings()` returns the `MapWarning`s collected during a lenient assembly (empty for
a clean map, and always empty after a strict `Map::assemble`, since strict mode returns an
error instead of recording a warning).
