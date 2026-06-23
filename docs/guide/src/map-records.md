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
use crustywad::{Wad, map};

// Build a minimal IWAD that contains one THINGS lump with a single thing.
let thing_bytes: &[u8] = &[
    100_i16.to_le_bytes()[0], 100_i16.to_le_bytes()[1],  // x = 100
    200_i16.to_le_bytes()[0], 200_i16.to_le_bytes()[1],  // y = 200
    0, 0,                                                  // angle = 0
    1, 0,                                                  // type_id = 1 (player 1 start)
    7, 0,                                                  // flags = 0x0007
];

let things: Vec<map::Thing> = map::parse_records(thing_bytes)?;
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
