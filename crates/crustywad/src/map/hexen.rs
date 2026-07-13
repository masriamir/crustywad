//! Hexen-format map record types (`Thing`, `Linedef`).
//!
//! Hexen diverges from the classic Doom binary layout ([`super::doom`]) only in
//! these two records — it extends `THINGS` (10 → 20 bytes) and `LINEDEFS`
//! (14 → 16 bytes) with a thing ID, spawn height, and the ZDoom-style
//! `special` + `args` action model. Every other map record (`VERTEXES`,
//! `SIDEDEFS`, `SECTORS`, `SEGS`, `SSECTORS`, `NODES`) is byte-identical to Doom
//! and is decoded from [`super::common`].

use binrw::BinRead;

/// A single record from a Hexen `THINGS` lump (20 bytes).
///
/// Extends the Doom [`Thing`](super::doom::Thing) with a thing ID (`tid`), a
/// spawn height (`z`), and the Hexen action model (`special` + `args`).
#[derive(Debug, Clone, PartialEq, Eq, BinRead)]
#[br(little)]
pub struct Thing {
    /// Thing ID (tag) used by ACS scripts and specials to reference this thing;
    /// `0` means no ID. Stored as an unsigned 16-bit identifier.
    pub tid: u16,
    /// X coordinate in map units (positive = East).
    pub x: i16,
    /// Y coordinate in map units (positive = North).
    pub y: i16,
    /// Spawn height above the sector floor, in map units.
    pub z: i16,
    /// Facing angle in degrees (`0`–`359`), `0` = East, increasing counter-clockwise.
    pub angle: u16,
    /// Numeric thing-type identifier (doomednum) selecting the actor class.
    pub type_id: u16,
    /// Bitfield of Hexen spawn/behavior flags (skill levels, `dormant`, the
    /// fighter/cleric/mage class filters, and the *positive* single/co-op/
    /// deathmatch bits `0x0100`/`0x0200`/`0x0400`). Stored opaquely here — the
    /// raw record keeps the on-disk word verbatim. Map assembly translates it
    /// into the graph's single Doom/Boom-MBF layout
    /// ([`MapThing::flags`](super::MapThing::flags)).
    pub flags: u16,
    /// Activation special (action number, `0`–`255`) run when the thing is used
    /// or triggered; `0` means none.
    pub special: u8,
    /// The five arguments passed to `special`, per-special semantics.
    pub args: [u8; 5],
}

/// A single record from a Hexen `LINEDEFS` lump (16 bytes).
///
/// Replaces the Doom [`Linedef`](super::doom::Linedef)'s `special: u16` +
/// `sector_tag: u16` with the Hexen action model: a one-byte `special` and five
/// `args`. There is no dedicated tag field — a special's target tag, when it has
/// one, is carried in `args`.
#[derive(Debug, Clone, PartialEq, Eq, BinRead)]
#[br(little)]
pub struct Linedef {
    /// Index into `VERTEXES` for the start (first) vertex.
    pub start_vertex: u16,
    /// Index into `VERTEXES` for the end (second) vertex.
    pub end_vertex: u16,
    /// Bitfield of Hexen linedef flags (including the activation-type bits).
    /// Stored opaquely; individual bits are not interpreted here.
    pub flags: u16,
    /// Activation special (action number, `0`–`255`); `0` means none.
    pub special: u8,
    /// The five arguments passed to `special`, per-special semantics.
    pub args: [u8; 5],
    /// Index into `SIDEDEFS` for the right (front) sidedef.
    pub right_sidedef: u16,
    /// Index into `SIDEDEFS` for the left (back) sidedef, or `0xffff` when the
    /// linedef is one-sided (same sentinel as Doom).
    pub left_sidedef: u16,
}
