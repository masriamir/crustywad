//! Doom map lump record types.
//!
//! A Doom map is stored as a group of lumps whose names follow a well-known
//! sequence (e.g. `E1M1`, `THINGS`, `LINEDEFS`, …).  This module provides
//! [`binrw`]-derived structs that map one-to-one onto the on-disk layout of
//! each record type, plus the [`parse_records`] helper that reads a raw lump
//! byte slice into a `Vec` of the appropriate type.
//!
//! These types intentionally stop at the individual record level for now.  A
//! future milestone will assemble them into a richer `Map` graph once the
//! crate has stable building blocks for the raw lump data.
//!
//! All structs use little-endian byte order as specified by the Doom WAD
//! format.  Field types follow the unofficial Doom spec:
//! coordinates and offsets are `i16`; indices, angles, and flags are `u16`;
//! light levels and special types may be either depending on the lump.

use std::io::Cursor;

use binrw::{BinRead, BinReaderExt};
use thiserror::Error;

/// A fixed-width 8-byte Doom name field, as used for texture and flat names.
///
/// Doom stores texture and flat names in 8-byte fields padded with `\0` bytes.
/// This wrapper preserves the raw bytes for round-trip fidelity while
/// [`as_str_lossy`][Name8::as_str_lossy] provides a human-readable view.
#[derive(Debug, Clone, PartialEq, Eq, BinRead)]
pub struct Name8(
    /// The raw 8-byte encoded name, NUL-padded on the right.
    pub [u8; 8],
);

impl Name8 {
    /// Returns the name as a `String` with trailing `\0` padding removed.
    ///
    /// Non-ASCII bytes (which should not appear in well-formed WADs) are
    /// replaced with U+FFFD rather than returning an error.  The result is
    /// at most 8 characters long.
    #[must_use]
    pub fn as_str_lossy(&self) -> String {
        let end = self
            .0
            .iter()
            .position(|&byte| byte == 0)
            .unwrap_or(self.0.len());
        String::from_utf8_lossy(&self.0[..end]).into_owned()
    }
}

/// A single record from the `THINGS` lump, describing one map object.
///
/// Things include player start positions, monsters, items, decorations, and
/// teleport destinations.  Each thing has a position in the map, a facing
/// direction, a numeric type identifier that determines what it looks like and
/// how it behaves, and a set of flags that control which difficulty levels and
/// game modes it appears in.
///
/// The `THINGS` lump contains `N` records of exactly 10 bytes each, where `N`
/// is `lump_size / 10`.  Use [`parse_records::<Thing>`] to decode the whole
/// lump at once.
///
/// See the [Doom Wiki — THINGS](https://doomwiki.org/wiki/Things) for the
/// authoritative field descriptions and type-ID tables.
#[derive(Debug, Clone, PartialEq, Eq, BinRead)]
#[br(little)]
pub struct Thing {
    /// X coordinate in map units (horizontal axis, positive = East).
    ///
    /// Doom uses a 2D overhead coordinate system.  The X axis points East and
    /// the Y axis points North.  Map units have no fixed real-world scale, but
    /// 64 units is roughly the width of a standard door.
    pub x: i16,
    /// Y coordinate in map units (vertical axis, positive = North).
    pub y: i16,
    /// Facing angle in degrees (`0`–`359`), where `0` = East and the value
    /// increases counter-clockwise (`90` = North, `180` = West, `270` = South).
    ///
    /// Stored as `u16` in the WAD.  Values are typically multiples of 45° in
    /// Doom's original maps, though the engine accepts any value.
    pub angle: u16,
    /// Numeric thing-type identifier (also called *`DoomEd` number*).
    ///
    /// Each non-zero value corresponds to a specific actor class defined in the
    /// engine (e.g. `1` = Player 1 Start, `3001` = Imp, `2001` = Shotgun).
    /// Type `0` is not normally used.  Refer to the Doom Wiki's actor tables
    /// for a complete listing.
    pub type_id: u16,
    /// Bitfield of spawn-condition flags.
    ///
    /// Common flags (Doom/Doom II format):
    /// - bit 0 (`0x0001`) — appears on skill 1 and 2 (I'm Too Young to Die /
    ///   Hey Not Too Rough)
    /// - bit 1 (`0x0002`) — appears on skill 3 (Hurt Me Plenty)
    /// - bit 2 (`0x0004`) — appears on skill 4 and 5 (Ultra-Violence / Nightmare)
    /// - bit 3 (`0x0008`) — deaf / ambush (does not wake on sound, only sight)
    /// - bit 4 (`0x0010`) — multiplayer-only (single player spawns are skipped)
    pub flags: u16,
}

/// A single record from the `LINEDEFS` lump, connecting two vertices.
///
/// Linedefs form the walls and boundaries of a Doom map.  Every linedef
/// connects two [`Vertex`] entries by index and must have at least one
/// [`Sidedef`] (the right side, facing the player as they walk from
/// `start_vertex` to `end_vertex`).  Two-sided linedefs have both a right and
/// a left sidedef and are used to create transparent barriers, windows, and
/// height changes between adjacent [`Sector`]s.
///
/// The `LINEDEFS` lump contains `N` records of exactly 14 bytes each.  Use
/// [`parse_records::<Linedef>`] to decode the whole lump at once.
#[derive(Debug, Clone, PartialEq, Eq, BinRead)]
#[br(little)]
pub struct Linedef {
    /// Index into the `VERTEXES` lump for the start (first) vertex.
    pub start_vertex: u16,
    /// Index into the `VERTEXES` lump for the end (second) vertex.
    pub end_vertex: u16,
    /// Bitfield of linedef behavior flags.
    ///
    /// Common flags:
    /// - bit 0 (`0x0001`) — impassable (blocks player and monsters)
    /// - bit 1 (`0x0002`) — blocks monsters
    /// - bit 2 (`0x0004`) — two-sided (has left sidedef, allows sight through)
    /// - bit 3 (`0x0008`) — upper texture is unpegged (anchored at ceiling)
    /// - bit 4 (`0x0010`) — lower texture is unpegged (anchored at floor)
    /// - bit 5 (`0x0020`) — secret (shown as solid on automap)
    /// - bit 6 (`0x0040`) — blocks sound propagation
    /// - bit 7 (`0x0080`) — never shown on automap
    /// - bit 8 (`0x0100`) — always shown on automap
    pub flags: u16,
    /// Special action type triggered when the linedef is activated.
    ///
    /// `0` means no special.  Non-zero values refer to the Doom engine's
    /// built-in action table (e.g. `1` = manual door open/close, `48` =
    /// scrolling wall texture).
    pub special_type: u16,
    /// Sector tag used to identify which sectors a special action affects.
    ///
    /// When `special_type` is non-zero, the engine looks for [`Sector`]
    /// entries whose `tag` field matches this value and applies the action to
    /// them.
    pub sector_tag: u16,
    /// Index into the `SIDEDEFS` lump for the right (front) sidedef.
    ///
    /// The right sidedef faces the player walking from `start_vertex` to
    /// `end_vertex`.  Every linedef has a right sidedef.
    pub right_sidedef: u16,
    /// Index into the `SIDEDEFS` lump for the left (back) sidedef, or
    /// `0xffff` when the linedef is one-sided.
    ///
    /// One-sided linedefs (solid walls) store `0xffff` here.  Two-sided
    /// linedefs store the index of the sidedef visible from the opposite
    /// direction.
    pub left_sidedef: u16,
}

/// A single record from the `SIDEDEFS` lump, associating wall textures with a
/// linedef face.
///
/// Each sidedef belongs to one face of a [`Linedef`] and references up to
/// three textures: upper (above a portal opening), lower (below a portal
/// opening), and middle (the main solid wall texture).  For one-sided walls
/// only `middle_texture` is relevant.  Texture names that begin with `"-"`
/// indicate *no texture* (the engine skips rendering that surface).
///
/// The `SIDEDEFS` lump contains `N` records of exactly 30 bytes each.  Use
/// [`parse_records::<Sidedef>`] to decode the whole lump at once.
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
/// Vertices are the endpoints of [`Linedef`]s and the control points for
/// [`Seg`]s.  All map geometry is built from them.  The `VERTEXES` lump
/// contains `N` records of exactly 4 bytes each.
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
/// Segs are the BSP-split fragments of [`Linedef`]s used by the Doom renderer.
/// The BSP builder may split a linedef into multiple segs when a partition line
/// crosses it.  Each seg references its parent linedef and which side of that
/// linedef it belongs to.
///
/// The `SEGS` lump contains `N` records of exactly 12 bytes each.  Use
/// [`parse_records::<Seg>`] to decode the whole lump at once.
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
/// [`parse_records::<Subsector>`] to decode the whole lump at once.
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
/// [`parse_records::<Node>`] to decode the whole lump at once.
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
/// [`parse_records::<Sector>`] to decode the whole lump at once.
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
    /// Sector tag, matching the `sector_tag` of [`Linedef`]s that trigger
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

/// Errors returned when decoding typed map records from a lump byte slice.
#[derive(Debug, Error)]
pub enum MapParseError {
    /// The lump byte slice length is not an exact multiple of the record size.
    ///
    /// This indicates a corrupt or truncated lump.  This check runs before any
    /// records are decoded: `offset` is the byte position where the trailing
    /// partial record begins, equal to
    /// `(lump_len / size_of::<T>()) * size_of::<T>()`.
    #[error("record stream ended mid-record at byte offset {offset}")]
    TrailingBytes {
        /// The byte offset of the start of the trailing partial record
        /// (i.e. the first byte that does not belong to a complete record).
        offset: u64,
    },
    /// `binrw` failed to decode a record from the byte stream.
    ///
    /// This typically indicates that the lump data is corrupted or does not
    /// actually contain the expected record type.
    #[error("failed to parse map records: {0}")]
    Binrw(#[from] binrw::Error),
}

/// Parses a raw lump byte slice into a `Vec` of the requested map record type.
///
/// This is the primary entry point for decoding map lumps.  Pass the raw bytes
/// from a lump (obtained via [`Wad::lump_bytes`][crate::Wad::lump_bytes] or
/// [`Wad::lump_data`][crate::Wad::lump_data]) and specify the target type:
///
/// ```rust,no_run
/// use crustywad::map::{Thing, parse_records};
///
/// # let raw_lump_bytes: &[u8] = &[];
/// let things: Vec<Thing> = parse_records(raw_lump_bytes)?;
/// # Ok::<(), crustywad::map::MapParseError>(())
/// ```
///
/// The function reads the entire slice as a sequence of fixed-size little-endian
/// records.  All record types in this module implement [`BinRead`] with the
/// correct on-disk field layout.
///
/// # Errors
///
/// - [`MapParseError::TrailingBytes`] — the slice length is not a whole
///   multiple of `size_of::<T>()`.  The lump is likely truncated or contains
///   the wrong record type.
/// - [`MapParseError::Binrw`] — `binrw` encountered an error decoding a
///   record. This usually means the bytes are corrupt.
pub fn parse_records<T>(bytes: &[u8]) -> Result<Vec<T>, MapParseError>
where
    T: for<'a> BinRead<Args<'a> = ()>,
{
    let record_size = std::mem::size_of::<T>();
    if record_size == 0 {
        // ZST records have no binary representation. An empty buffer produces
        // zero records; any non-empty buffer has unresolvable trailing bytes.
        return if bytes.is_empty() {
            Ok(Vec::new())
        } else {
            Err(MapParseError::TrailingBytes { offset: 0 })
        };
    }
    if bytes.len() % record_size != 0 {
        return Err(MapParseError::TrailingBytes {
            offset: (bytes.len() / record_size * record_size) as u64,
        });
    }

    let mut cursor = Cursor::new(bytes);
    let mut records = Vec::with_capacity(bytes.len() / record_size);
    let bytes_len = bytes.len() as u64;

    while cursor.position() < bytes_len {
        records.push(cursor.read_le()?);
    }

    Ok(records)
}
