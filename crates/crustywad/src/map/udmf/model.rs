//! Typed, un-normalized UDMF document model (the intermediate produced by
//! [`parse_udmf`][crate::map::udmf::parse_udmf]).
//!
//! Each struct carries the standardized fields that map assembly normalizes
//! into [`Map`][crate::map::Map], plus an `extras` list retaining every other
//! assignment for lossless round-trip (ADR-0027). All structs are
//! `#[non_exhaustive]` so future work (a full-fidelity map editor) can add
//! fields without a breaking change.

/// A UDMF assignment value.
///
/// Mirrors the lexer's four value token shapes; a parsed value is always one
/// of these (the parser rejects any other token in value position), and a
/// retained `Float` is always finite (the lexer rejects
/// overflow-to-infinity literals).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum UdmfValue {
    /// A `true`/`false` literal.
    Bool(bool),
    /// An integer literal (decimal or hexadecimal).
    Int(i64),
    /// A floating-point literal (always finite).
    Float(f64),
    /// A double-quoted string literal, escapes resolved.
    Str(String),
}

/// A retained `name = value;` assignment (ADR-0027).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct UdmfAssignment {
    /// The field name, folded to ASCII lowercase (UDMF identifiers are
    /// case-insensitive).
    pub name: String,
    /// The assigned value.
    pub value: UdmfValue,
}

/// A retained block whose header identifier is not one of the five
/// standardized kinds (e.g. a port-specific block), in declaration order
/// within [`UdmfMap::unknown_blocks`].
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct UdmfUnknownBlock {
    /// The block header identifier, folded to ASCII lowercase.
    pub name: String,
    /// The block's assignments: first-assignment order, last-assignment
    /// value on duplicate names (UDMF's last-wins semantics).
    pub fields: Vec<UdmfAssignment>,
}

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
    /// Blocks whose header is not one of the five standardized kinds, in
    /// declaration order (ADR-0027).
    pub unknown_blocks: Vec<UdmfUnknownBlock>,
    /// Global assignments other than `namespace`: first-assignment order,
    /// last-assignment value on duplicates (ADR-0027).
    pub global_extras: Vec<UdmfAssignment>,
}

/// A UDMF `vertex` block.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct UdmfVertex {
    /// The vertex X coordinate (required; no default).
    pub x: f64,
    /// The vertex Y coordinate (required; no default).
    pub y: f64,
    /// Assignments not losslessly held by a typed field (port extensions,
    /// `comment`, `user_*`, …), retained for round-trip: first-assignment
    /// order, last-assignment value on duplicates (ADR-0027).
    pub extras: Vec<UdmfAssignment>,
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
    /// Assignments not losslessly held by a typed field (port extensions,
    /// `comment`, `user_*`, …), retained for round-trip: first-assignment
    /// order, last-assignment value on duplicates (ADR-0027).
    pub extras: Vec<UdmfAssignment>,
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
    /// Assignments not losslessly held by a typed field (port extensions,
    /// `comment`, `user_*`, …), retained for round-trip: first-assignment
    /// order, last-assignment value on duplicates (ADR-0027).
    pub extras: Vec<UdmfAssignment>,
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
    /// Assignments not losslessly held by a typed field (port extensions,
    /// `comment`, `user_*`, …), retained for round-trip: first-assignment
    /// order, last-assignment value on duplicates (ADR-0027).
    pub extras: Vec<UdmfAssignment>,
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
    /// `standing`) are dropped. See [`extras`](Self::extras) for the
    /// round-trip source of truth.
    pub flags: u32,
    /// Assignments not losslessly held by a typed field, retained for
    /// round-trip — **including** the 10 recognized booleans
    /// (`skill1`–`skill5`, `ambush`, `single`, `dm`, `coop`, `friend`),
    /// which are dual-stored here because their [`flags`](Self::flags) fold
    /// is not invertible (ADR-0027). `extras` is the round-trip source of
    /// truth; `flags` is derived.
    pub extras: Vec<UdmfAssignment>,
}
