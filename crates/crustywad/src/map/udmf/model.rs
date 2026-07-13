//! Typed, un-normalized UDMF document model (the intermediate produced by
//! [`parse_udmf`][crate::map::udmf::parse_udmf]).
//!
//! Each struct carries only the standardized fields that map assembly (PR B)
//! normalizes into [`Map`][crate::map::Map]; every other recognized UDMF field
//! is parsed for syntactic validity and dropped. All structs are
//! `#[non_exhaustive]` so future work (a full-fidelity map editor) can add
//! fields without a breaking change.

/// A parsed UDMF text map, before cross-reference resolution.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct UdmfMap {
    /// The `namespace` declaration (e.g. `"doom"`, `"zdoom"`).
    pub namespace: String,
    /// The map's vertices, in declaration order.
    pub vertices: Vec<UdmfVertex>,
    /// The map's linedefs, in declaration order.
    pub linedefs: Vec<UdmfLinedef>,
    /// The map's sidedefs, in declaration order.
    pub sidedefs: Vec<UdmfSidedef>,
    /// The map's sectors, in declaration order.
    pub sectors: Vec<UdmfSector>,
    /// The map's things, in declaration order.
    pub things: Vec<UdmfThing>,
}

/// A UDMF `vertex` block.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct UdmfVertex {
    /// The vertex X coordinate (required; no default).
    pub x: f64,
    /// The vertex Y coordinate (required; no default).
    pub y: f64,
}

/// A UDMF `linedef` block.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct UdmfLinedef {
    /// The start vertex index (required; no default).
    pub v1: i32,
    /// The end vertex index (required; no default).
    pub v2: i32,
    /// The front sidedef index (required; no default).
    pub sidefront: i32,
    /// The back sidedef index, or `None` if the UDMF default -1 is present.
    pub sideback: Option<i32>,
    /// The linedef ID (UDMF default -1).
    pub id: i32,
    /// The special type (UDMF default 0).
    pub special: i32,
    /// The special arguments; all default to 0.
    pub args: [i32; 5],
    /// The Doom-mapped linedef flags packed into bits 0–8 of a `u32`.
    pub flags: u32,
}

/// A UDMF `sidedef` block.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct UdmfSidedef {
    /// The X offset (UDMF default 0).
    pub offsetx: i32,
    /// The Y offset (UDMF default 0).
    pub offsety: i32,
    /// The upper texture name (UDMF default "-").
    pub texturetop: String,
    /// The lower texture name (UDMF default "-").
    pub texturebottom: String,
    /// The middle texture name (UDMF default "-").
    pub texturemiddle: String,
    /// The sector index (required; no default).
    pub sector: i32,
}

/// A UDMF `sector` block.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct UdmfSector {
    /// The floor height (UDMF default 0).
    pub heightfloor: i32,
    /// The ceiling height (UDMF default 0).
    pub heightceiling: i32,
    /// The floor texture name (required; no default).
    pub texturefloor: String,
    /// The ceiling texture name (required; no default).
    pub textureceiling: String,
    /// The light level (UDMF default 160).
    pub lightlevel: i32,
    /// The special type (UDMF default 0).
    pub special: i32,
    /// The sector ID/tag (UDMF default 0).
    pub id: i32,
}

/// A UDMF `thing` block.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct UdmfThing {
    /// The X coordinate (required; no default).
    pub x: f64,
    /// The Y coordinate (required; no default).
    pub y: f64,
    /// The height above the sector floor (UDMF default 0).
    pub height: f64,
    /// The angle in degrees (UDMF default 0; raw, not normalized).
    pub angle: i32,
    /// The thing type (required; no default).
    pub type_id: i32,
    /// The thing ID (UDMF default 0).
    pub id: i32,
    /// The special type (UDMF default 0).
    pub special: i32,
    /// The special arguments; all default to 0.
    pub args: [i32; 5],
    /// The Doom/Boom-MBF-mapped thing flags, packed into bits 0–7 (ADR-0019).
    ///
    /// `skill1 | skill2` → bit 0, `skill3` → bit 1, `skill4 | skill5` → bit 2,
    /// `ambush` → bit 3, `!single` → bit 4, `!dm` → bit 5, `!coop` → bit 6,
    /// `friend` → bit 7. The skill pairs are OR-folded because Doom has one bit
    /// per pair; UDMF booleans with no Doom bit (`class1`–`class3`, `dormant`,
    /// `standing`) are dropped.
    pub flags: u32,
}
