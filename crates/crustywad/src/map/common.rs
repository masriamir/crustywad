//! Map record types whose on-disk byte layout is identical across the classic
//! Doom, Heretic, and Hexen formats. Format-specific records (`Thing`,
//! `Linedef`) live in the per-format modules such as `super::doom`.

use binrw::BinRead;

/// A fixed-width 8-byte Doom name field, as used for texture and flat names.
///
/// Doom stores texture and flat names in 8-byte fields padded with `\0` bytes.
/// This wrapper preserves the raw bytes for round-trip fidelity while
/// [`as_str_lossy`][Name8::as_str_lossy] provides a human-readable view.
#[cfg_attr(feature = "write", derive(binrw::BinWrite))]
#[derive(Debug, Clone, PartialEq, Eq, BinRead)]
pub struct Name8(
    /// The raw 8-byte encoded name, NUL-padded on the right.
    pub [u8; 8],
);

impl Name8 {
    /// Returns the name as a `String` with trailing `\0` padding removed.
    ///
    /// Decoding uses `String::from_utf8_lossy`: any invalid UTF-8 byte sequence
    /// is replaced with U+FFFD, while valid UTF-8 (including pure ASCII) is
    /// preserved.  The result is at most 8 characters long.
    #[must_use]
    pub fn as_str_lossy(&self) -> String {
        String::from_utf8_lossy(crate::util::trim_nul(&self.0)).into_owned()
    }
}

/// A single record from the `SIDEDEFS` lump, associating wall textures with a
/// linedef face.
///
/// Each sidedef belongs to one face of a linedef and references up to
/// three textures: upper (above a portal opening), lower (below a portal
/// opening), and middle (the main solid wall texture).  For one-sided walls
/// only `middle_texture` is relevant.  Texture names that begin with `"-"`
/// indicate *no texture* (the engine skips rendering that surface).
///
/// The `SIDEDEFS` lump contains `N` records of exactly 30 bytes each.  Use
/// [`parse_records::<Sidedef>`](crate::map::parse_records) to decode the whole lump at once.
#[cfg_attr(feature = "write", derive(binrw::BinWrite))]
#[cfg_attr(feature = "write", bw(little))]
#[derive(Debug, Clone, PartialEq, Eq, BinRead)]
#[br(little)]
pub struct Sidedef {
    /// Horizontal texture offset in texels, applied to all three textures.
    ///
    /// Positive values shift the texture to the right (East); negative values
    /// shift it to the left.
    pub x_offset: i16,
    /// Vertical texture offset in texels, applied to all three textures.
    ///
    /// Positive values shift the texture downward; negative values shift it
    /// upward.
    pub y_offset: i16,
    /// Upper texture name (rendered above a portal's opening, if any).
    ///
    /// Used when the ceiling of the back sector is lower than the ceiling of
    /// the front sector.  A value of `"-\0\0\0\0\0\0\0"` (or any name
    /// starting with `"-"`) means no texture is drawn.
    pub upper_texture: Name8,
    /// Lower texture name (rendered below a portal's opening, if any).
    ///
    /// Used when the floor of the back sector is higher than the floor of the
    /// front sector.
    pub lower_texture: Name8,
    /// Middle texture name (the main wall surface).
    ///
    /// For one-sided walls this is the solid wall texture.  For two-sided
    /// linedefs a middle texture creates a partially-transparent overlay (e.g.
    /// bars or chains) within the portal opening.
    pub middle_texture: Name8,
    /// Index into the `SECTORS` lump for the sector this sidedef faces into.
    pub sector: u16,
}

/// A single record from the `VERTEXES` lump, representing a 2D map point.
///
/// Vertices are the endpoints of linedefs and the control points for
/// [`Seg`]s.  All map geometry is built from them.  The `VERTEXES` lump
/// contains `N` records of exactly 4 bytes each.
#[cfg_attr(feature = "write", derive(binrw::BinWrite))]
#[cfg_attr(feature = "write", bw(little))]
#[derive(Debug, Clone, PartialEq, Eq, BinRead)]
#[br(little)]
pub struct Vertex {
    /// X coordinate in map units (positive = East).
    pub x: i16,
    /// Y coordinate in map units (positive = North).
    pub y: i16,
}

/// A single record from the `SEGS` lump, representing a wall segment produced
/// by the BSP builder.
///
/// Segs are the BSP-split fragments of linedefs used by the Doom renderer.
/// The BSP builder may split a linedef into multiple segs when a partition line
/// crosses it.  Each seg references its parent linedef and which side of that
/// linedef it belongs to.
///
/// The `SEGS` lump contains `N` records of exactly 12 bytes each.  Use
/// [`parse_records::<Seg>`](crate::map::parse_records) to decode the whole lump at once.
#[derive(Debug, Clone, PartialEq, Eq, BinRead)]
#[br(little)]
pub struct Seg {
    /// Index into the `VERTEXES` lump for the start vertex of this seg.
    pub start_vertex: u16,
    /// Index into the `VERTEXES` lump for the end vertex of this seg.
    pub end_vertex: u16,
    /// Angle of the seg expressed as a binary angle (`BAMS` unit).
    ///
    /// The full circle maps to `[0, 65536)`: `0x4000` = 90°, `0x8000` = 180°,
    /// `0xC000` = 270°.  This is the direction from `start_vertex` to
    /// `end_vertex`.
    pub angle: u16,
    /// Index into the `LINEDEFS` lump for the linedef this seg is part of.
    pub linedef: u16,
    /// Which side of the parent linedef this seg faces.
    ///
    /// `0` means this seg faces the *right* (front) sidedef of the linedef;
    /// `1` means it faces the *left* (back) sidedef.
    pub direction: u16,
    /// Distance along the parent linedef from its start vertex to the start of
    /// this seg, in map units.
    ///
    /// Negative when the seg starts before the linedef's own start vertex (can
    /// happen after BSP splitting).  Used by the renderer to align textures
    /// across split linedefs.
    pub offset: i16,
}

/// A single record from the `SSECTORS` lump (sub-sectors), one leaf node of
/// the BSP tree.
///
/// A sub-sector is a convex region of the map bounded by segs.  The BSP tree
/// leaf nodes reference sub-sectors, and the renderer draws them in
/// back-to-front order to achieve correct visibility.  Each sub-sector owns a
/// contiguous run of [`Seg`] entries in the `SEGS` lump.
///
/// The `SSECTORS` lump contains `N` records of exactly 4 bytes each.  Use
/// [`parse_records::<Subsector>`](crate::map::parse_records) to decode the whole lump at once.
#[derive(Debug, Clone, PartialEq, Eq, BinRead)]
#[br(little)]
pub struct Subsector {
    /// Number of segs that make up this sub-sector's boundary.
    pub seg_count: u16,
    /// Index into the `SEGS` lump of the first seg belonging to this
    /// sub-sector.  The remaining segs are at indices
    /// `first_seg..first_seg + seg_count`.
    pub first_seg: u16,
}

/// A single record from the `NODES` lump, one internal node of the BSP tree.
///
/// The BSP (*Binary Space Partitioning*) tree is pre-computed by the level
/// builder and stored in the `NODES` lump.  Each node describes a partition
/// line that splits the map into two half-planes (left and right children).
/// The children are either further `Node` entries (internal nodes) or
/// [`Subsector`] entries (leaves), distinguished by the high bit of the child
/// index: if bit 15 is set, the remaining 15 bits are a sub-sector index;
/// otherwise they are a node index.
///
/// The `NODES` lump contains `N` records of exactly 28 bytes each.  Use
/// [`parse_records::<Node>`](crate::map::parse_records) to decode the whole lump at once.
#[derive(Debug, Clone, PartialEq, Eq, BinRead)]
#[br(little)]
pub struct Node {
    /// X coordinate of the partition line's start point, in map units.
    pub x: i16,
    /// Y coordinate of the partition line's start point, in map units.
    pub y: i16,
    /// Horizontal extent of the partition line (delta X from start to end).
    pub dx: i16,
    /// Vertical extent of the partition line (delta Y from start to end).
    pub dy: i16,
    /// Axis-aligned bounding box for the right child, as `[top, bottom, left,
    /// right]` in map units.
    ///
    /// All coordinates are in the map's 2D space; `top > bottom` and
    /// `right > left`.
    pub right_bbox: [i16; 4],
    /// Axis-aligned bounding box for the left child, as `[top, bottom, left,
    /// right]` in map units.
    pub left_bbox: [i16; 4],
    /// Child index for the right (front) half-plane.
    ///
    /// If bit 15 (`0x8000`) is set, the remaining 15 bits are a
    /// [`Subsector`] index; otherwise they are a [`Node`] index.
    pub right_child: u16,
    /// Child index for the left (back) half-plane.
    ///
    /// Same encoding as `right_child`.
    pub left_child: u16,
}

/// A single record from the `SECTORS` lump, describing an enclosed floor/
/// ceiling region.
///
/// A sector is the fundamental volume unit of a Doom map.  Every point on the
/// floor belongs to exactly one sector, which defines the floor and ceiling
/// heights, floor and ceiling textures (called *flats*), ambient light level,
/// and any special environmental effects (damaging floor, blinking lights,
/// etc.).
///
/// The `SECTORS` lump contains `N` records of exactly 26 bytes each.  Use
/// [`parse_records::<Sector>`](crate::map::parse_records) to decode the whole lump at once.
#[cfg_attr(feature = "write", derive(binrw::BinWrite))]
#[cfg_attr(feature = "write", bw(little))]
#[derive(Debug, Clone, PartialEq, Eq, BinRead)]
#[br(little)]
pub struct Sector {
    /// Floor height in map units.
    ///
    /// The player stands on the floor.  Heights are relative to a common
    /// origin; negative values are valid (e.g. a pit floor at -64).
    pub floor_height: i16,
    /// Ceiling height in map units.
    ///
    /// Must be greater than or equal to `floor_height` for a non-sky sector to
    /// be accessible to the player.
    pub ceiling_height: i16,
    /// Name of the floor flat (texture drawn on the floor).
    ///
    /// Flats are 64×64 pixel textures stored in the `F_START` / `F_END` lump
    /// namespace.  The special name `"F_SKY1"` makes the floor render as sky
    /// (rare but valid).
    pub floor_texture: Name8,
    /// Name of the ceiling flat (texture drawn on the ceiling).
    ///
    /// The special name `"F_SKY1"` makes the ceiling render as an outdoor sky.
    pub ceiling_texture: Name8,
    /// Ambient light level in the range `0` (total darkness) to `255`
    /// (full brightness).
    ///
    /// Stored as `i16` in the WAD; values outside `0..=255` are treated as
    /// clamped by the engine.
    pub light_level: i16,
    /// Special effect type for this sector.
    ///
    /// `0` means no special.  Non-zero values activate effects such as
    /// flickering lights (`1`), blinking lights (`2`), damaging floor (`5`,
    /// `7`, `16`), and exit triggers (`11`).
    pub special_type: i16,
    /// Sector tag, matching the `sector_tag` of linedefs that trigger
    /// actions on this sector.
    ///
    /// `0` means untagged (no linedef will trigger actions on this sector via
    /// the tag mechanism).
    pub tag: i16,
}

/// Placeholder for the `REJECT` lump (not yet parsed).
///
/// The `REJECT` lump is a bit matrix used by the engine to quickly determine
/// whether a monster in one sector can potentially see the player in another.
/// It exists as a performance optimization for the AI line-of-sight check.
/// Full parsing is deferred to a future milestone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RejectLump;

/// Placeholder for the `BLOCKMAP` lump (not yet parsed).
///
/// The `BLOCKMAP` lump is a spatial index that divides the map into a grid of
/// 128×128 map-unit cells and records which linedefs cross each cell.  The
/// engine uses it to accelerate collision detection.  Full parsing is deferred
/// to a future milestone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BlockmapLump;
