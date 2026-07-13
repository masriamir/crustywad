//! Doom 64-format map record types.
//!
//! Doom 64 stores each `MAPxx` as a **nested WAD**: the map lump's bytes are a
//! complete WAD whose sub-lumps hold the records. The records diverge from the
//! classic Doom binary layout in width and field content:
//!
//! - `VERTEXES` are 16.16 fixed-point `i32` pairs (not `i16`).
//! - `THINGS` gain a spawn height (`z`) and a thing ID (`id`).
//! - `LINEDEFS` widen `flags` to `u32`.
//! - `SIDEDEFS` reference textures by `u16` index, not 8-byte name.
//! - `SECTORS` reference flats by `u16` index and carry five colored-lighting
//!   IDs and a new `LIGHTS` palette lump.
//!
//! The BSP lumps `SEGS`, `SSECTORS`, and `NODES` are byte-identical to classic
//! Doom, so they are decoded with [`super::common::Seg`],
//! [`super::common::Subsector`], and [`super::common::Node`] rather than
//! redefined here.
//!
//! This module reads records **raw** (un-normalized): the texture/flat/color
//! `u16` indices are preserved as-is; resolving them needs a texture/graphics
//! layer that does not exist yet. A reader API will be added in a future task.

use binrw::BinRead;

/// Returns `true` if `bytes` look like a Doom 64 map lump: a lump whose content
/// is itself a WAD (leading `IWAD` or `PWAD` magic).
///
/// This is the structural detection signal for Doom 64 maps (ADR-0018): a
/// Doom 64 `MAPxx` lump's bytes begin with WAD magic, whereas a classic
/// Doom/Hexen map marker is a conventionally empty lump carrying no such magic.
/// The check is cheap and does not parse the nested directory.
#[must_use]
pub fn is_doom64_map_lump(bytes: &[u8]) -> bool {
    bytes.len() >= 12 && (bytes[0..4] == *b"IWAD" || bytes[0..4] == *b"PWAD")
}

/// A single `VERTEXES` record (8 bytes): a map vertex in **16.16 fixed-point**.
///
/// Unlike classic Doom's `i16` integer coordinates, Doom 64 stores each axis as
/// a 32-bit fixed-point value with 16 fractional bits: the map-unit value is
/// `raw as f64 / 65536.0` (e.g. raw `20971520` = `320.0`, raw `-65536` = `-1.0`).
/// The raw `i32`s are preserved here without conversion.
#[derive(Debug, Clone, PartialEq, Eq, BinRead)]
#[br(little)]
pub struct Vertex {
    /// X coordinate, 16.16 fixed-point (positive = East). Divide by `65536.0`
    /// for map units.
    pub x: i32,
    /// Y coordinate, 16.16 fixed-point (positive = North). Divide by `65536.0`
    /// for map units.
    pub y: i32,
}

/// A single `THINGS` record (14 bytes).
///
/// Extends the classic Doom thing with a spawn height (`z`) and a thing ID
/// (`id`, the tag referenced by specials/scripts). All fields are signed
/// 16-bit as stored on disk.
#[derive(Debug, Clone, PartialEq, Eq, BinRead)]
#[br(little)]
pub struct Thing {
    /// X coordinate in map units (positive = East).
    pub x: i16,
    /// Y coordinate in map units (positive = North).
    pub y: i16,
    /// Spawn height above the sector floor, in map units.
    pub z: i16,
    /// Facing angle in degrees (`0` = East, increasing counter-clockwise).
    pub angle: i16,
    /// Numeric thing-type identifier (doomednum) selecting the actor class.
    pub type_id: i16,
    /// Bitfield of spawn/behavior flags; stored opaquely (bits not interpreted).
    pub flags: i16,
    /// Thing ID (tag/tid) referenced by specials and scripts; `0` means none.
    pub id: i16,
}

/// A single `LINEDEFS` record (16 bytes).
///
/// Diverges from classic Doom by widening `flags` to `u32`. `sideback ==
/// 0xffff` marks a one-sided linedef (same sentinel meaning as Doom's
/// `left_sidedef`).
#[derive(Debug, Clone, PartialEq, Eq, BinRead)]
#[br(little)]
pub struct Linedef {
    /// Index into `VERTEXES` for the start vertex.
    pub v1: u16,
    /// Index into `VERTEXES` for the end vertex.
    pub v2: u16,
    /// Bitfield of linedef behavior flags (widened to 32 bits in Doom 64).
    /// Stored opaquely; individual bits are not interpreted here.
    pub flags: u32,
    /// Special action type triggered when the linedef is activated; `0` = none.
    pub special: u16,
    /// Sector tag identifying which sectors the special affects.
    pub tag: u16,
    /// Index into `SIDEDEFS` for the right (front) sidedef.
    pub sidefront: u16,
    /// Index into `SIDEDEFS` for the left (back) sidedef, or `0xffff` when the
    /// linedef is one-sided.
    pub sideback: u16,
}

/// A single `SIDEDEFS` record (12 bytes).
///
/// Doom 64 references wall textures by **`u16` index** into the engine's
/// texture table rather than by 8-byte name, so the record is 12 bytes instead
/// of classic Doom's 30.
#[derive(Debug, Clone, PartialEq, Eq, BinRead)]
#[br(little)]
pub struct Sidedef {
    /// Horizontal texture offset in map units.
    pub x_offset: i16,
    /// Vertical texture offset in map units.
    pub y_offset: i16,
    /// Upper-texture index (into the texture table). Preserved raw.
    pub upper: u16,
    /// Lower-texture index. Preserved raw.
    pub lower: u16,
    /// Middle-texture index. Preserved raw.
    pub middle: u16,
    /// Index into `SECTORS` for the sector this sidedef faces.
    pub sector: u16,
}

/// A single `SECTORS` record (24 bytes).
///
/// Doom 64 references floor/ceiling flats by **`u16` index** and adds five
/// colored-lighting IDs (`colors`) that index the map's `LIGHTS` palette.
#[derive(Debug, Clone, PartialEq, Eq, BinRead)]
#[br(little)]
pub struct Sector {
    /// Floor height in map units.
    pub floor_height: i16,
    /// Ceiling height in map units.
    pub ceiling_height: i16,
    /// Floor-flat index (into the texture/flat table). Preserved raw.
    pub floor_tex: u16,
    /// Ceiling-flat index. Preserved raw.
    pub ceiling_tex: u16,
    /// Five colored-lighting IDs indexing this map's `LIGHTS` palette
    /// (Doom 64 colored lighting). Preserved raw; semantics per Doom64 EX.
    pub colors: [u16; 5],
    /// Sector special type; `0` = none.
    pub special: u16,
    /// Sector tag matched by linedef specials.
    pub tag: u16,
    /// Bitfield of sector flags; stored opaquely.
    pub flags: u16,
}

/// A single `LIGHTS` record (6 bytes): one colored-lighting palette entry.
///
/// The `LIGHTS` lump is the color palette that [`Sector::colors`] indexes.
/// Measured from a retail `DOOM64.WAD`: `r`/`g`/`b` are the color channels,
/// `tag` is a small identifier (observed values `0`–`2`), and `unknown` is a
/// 16-bit field whose high byte is always zero in retail data. The exact
/// meaning of `tag`/`unknown` is not yet confirmed (per Doom64 EX); the raw
/// bytes are preserved.
#[derive(Debug, Clone, PartialEq, Eq, BinRead)]
#[br(little)]
pub struct Light {
    /// Red channel (`0`–`255`).
    pub r: u8,
    /// Green channel (`0`–`255`).
    pub g: u8,
    /// Blue channel (`0`–`255`).
    pub b: u8,
    /// Small identifier (observed `0`–`2`); tentative semantics, preserved raw.
    pub tag: u8,
    /// 16-bit trailing field (high byte always `0` in retail data); tentative
    /// semantics, preserved raw.
    pub unknown: u16,
}
