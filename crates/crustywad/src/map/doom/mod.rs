//! Doom-format map record types (`Thing`, `Linedef`). Heretic reuses these
//! same on-disk layouts. Records shared with other formats live in
//! [`super::common`].

use binrw::BinRead;

#[cfg(feature = "write")]
mod write;

#[cfg(feature = "write")]
pub use write::{DoomMapLumps, DoomWriteError, DoomWriteWarning, add_doom_map, write_doom_map};

// The `write` module is private to `map::doom`, so the `nodebuild` builders
// (in `map::build`) cannot reach its `pub(crate)` coordinate narrower by path.
// Re-export the two items they share (ADR-0024 §3), gated so nothing is unused
// when the feature is off.
#[cfg(feature = "nodebuild")]
pub(crate) use write::{Narrower, narrow_vertices};

/// A single record from the `THINGS` lump, describing one map object.
///
/// Things include player start positions, monsters, items, decorations, and
/// teleport destinations.  Each thing has a position in the map, a facing
/// direction, a numeric type identifier that determines what it looks like and
/// how it behaves, and a set of flags that control which difficulty levels and
/// game modes it appears in.
///
/// The `THINGS` lump contains `N` records of exactly 10 bytes each, where `N`
/// is `lump_size / 10`.  Use [`parse_records::<Thing>`](crate::map::parse_records) to decode the whole
/// lump at once.
///
/// See the [Doom Wiki — THINGS](https://doomwiki.org/wiki/Things) for the
/// authoritative field descriptions and type-ID tables.
#[cfg_attr(feature = "write", derive(binrw::BinWrite))]
#[cfg_attr(feature = "write", bw(little))]
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
/// connects two [`Vertex`](crate::map::Vertex) entries by index and must have at least one
/// [`Sidedef`](crate::map::Sidedef) (the right side, facing the player as they walk from
/// `start_vertex` to `end_vertex`).  Two-sided linedefs have both a right and
/// a left sidedef and are used to create transparent barriers, windows, and
/// height changes between adjacent [`Sector`](crate::map::Sector)s.
///
/// The `LINEDEFS` lump contains `N` records of exactly 14 bytes each.  Use
/// [`parse_records::<Linedef>`](crate::map::parse_records) to decode the whole lump at once.
#[cfg_attr(feature = "write", derive(binrw::BinWrite))]
#[cfg_attr(feature = "write", bw(little))]
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
    /// When `special_type` is non-zero, the engine looks for [`Sector`](crate::map::Sector)
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
