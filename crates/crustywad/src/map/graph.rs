//! The assembled, index-addressed map graph (ADR-0015 §2).

/// The source format a [`Map`] was assembled from. [`Doom`][MapFormat::Doom],
/// [`Hexen`][MapFormat::Hexen], and [`Udmf`][MapFormat::Udmf] are assembled
/// today; Doom64 (Epic #17) reuses this same model but isn't implemented yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MapFormat {
    /// The classic Doom binary layout — also used, unchanged, by **Doom II** and
    /// **Heretic** (which differ only in map-marker naming, not record format).
    Doom,
    /// The Hexen binary layout — extends `THINGS`/`LINEDEFS` (see
    /// [`map::hexen`][crate::map::hexen]); detected by the presence of a
    /// `BEHAVIOR` lump.
    Hexen,
    /// The Universal Doom Map Format (UDMF) text layout (see
    /// [`map::udmf`][crate::map::udmf]); detected by the presence of a
    /// `TEXTMAP` lump.
    Udmf,
}

/// A zero-based index into [`Map::vertices`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VertexIdx(pub usize);

/// A zero-based index into [`Map::sidedefs`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SidedefIdx(pub usize);

/// A zero-based index into [`Map::sectors`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SectorIdx(pub usize);

/// A zero-based index into [`Map::linedefs`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LinedefIdx(pub usize);

/// A zero-based index into [`Map::lights`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LightIdx(pub usize);

/// A normalized map vertex; coordinates are `f64` so binary `i16` widens
/// losslessly and future UDMF floats fit natively.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapVertex {
    /// The vertex's X coordinate, in map units.
    pub x: f64,
    /// The vertex's Y coordinate, in map units.
    pub y: f64,
}

/// A normalized action special: the `special` number plus its five `args`.
///
/// Shared by [`MapLinedef`] and [`MapThing`] — Doom, Hexen, and UDMF all use this
/// `special` + `args` shape. For a Doom *linedef*, the sector tag is carried in `args[0]`
/// (Doom things have no special, so a Doom thing's `Special` is all zero).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Special {
    /// The action special number.
    pub special: i32,
    /// The special's five arguments; `args[0]` carries Doom's sector tag.
    pub args: [i32; 5],
}

/// A normalized linedef, referencing its endpoints, sidedefs, and special by
/// index into the owning [`Map`]'s arenas.
#[derive(Debug, Clone, PartialEq)]
pub struct MapLinedef {
    /// The index of the linedef's start vertex.
    pub start: VertexIdx,
    /// The index of the linedef's end vertex.
    pub end: VertexIdx,
    /// The index of the linedef's right (front) sidedef; `None` == no front
    /// side. Sources: the binary `0xffff` sentinel (vanilla-sanctioned and
    /// rare, e.g. an invisible blocking line; ADR-0020) or lenient-mode
    /// recovery of a dangling reference.
    pub right: Option<SidedefIdx>,
    /// The index of the linedef's left (back) sidedef; `None` == no back
    /// side (a one-sided wall in the common case, though a line can also be
    /// fully sideless). Sources: the binary `0xffff` sentinel, an omitted
    /// UDMF `sideback`, or lenient-mode recovery of a dangling reference.
    pub left: Option<SidedefIdx>,
    /// The linedef's bit flags (blocking, two-sided, secret, etc.).
    pub flags: u32,
    /// The linedef's action special and arguments.
    pub special: Special,
    /// The linedef's identifier (the UDMF/ZDoom line id); `0` for Doom/Hexen maps.
    pub id: i32,
}

/// The [`MapLinedef::id`] value that means "this linedef has no id", which is
/// **source-dependent**: UDMF's spec default is `-1`, while a Doom/Hexen map's
/// linedefs are assembled with `0` (the graph convention). Assembly copies each
/// source's sentinel into `MapLinedef.id` verbatim, so any consumer asking
/// "does this linedef carry a real id?" must know the format.
///
/// Both writers depend on this rule and must agree on it — `map::udmf::write`
/// omits the sentinel so a Doom line is not written as a genuine UDMF `id = 0`,
/// and `map::doom::write` treats it as the *absence* of an id rather than
/// tier-3 data loss. Defining it once is what keeps a Doom → UDMF → Doom
/// round-trip from resurrecting a sentinel as data.
#[cfg(feature = "write")]
#[must_use]
pub(crate) fn linedef_id_unset(format: MapFormat) -> i32 {
    if format == MapFormat::Udmf { -1 } else { 0 }
}

/// A texture or flat reference in the assembled graph (ADR-0021 §3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextureRef {
    /// A texture name (a Doom/Hexen 8-byte lump name, or a UDMF string).
    Name(String),
    /// A Doom 64 texture/flat table index — resolvable to a texture identity
    /// once the texture layer (v0.5.0, #156/#157) exists.
    Index(u16),
}

impl TextureRef {
    /// The texture name, or `None` for a Doom 64 [`TextureRef::Index`].
    #[must_use]
    pub fn as_name(&self) -> Option<&str> {
        match self {
            TextureRef::Name(name) => Some(name),
            TextureRef::Index(_) => None,
        }
    }
}

impl PartialEq<&str> for TextureRef {
    fn eq(&self, other: &&str) -> bool {
        self.as_name() == Some(*other)
    }
}

/// A normalized sidedef, referencing its sector by index into the owning
/// [`Map`]'s sector arena.
#[derive(Debug, Clone, PartialEq)]
pub struct MapSidedef {
    /// The index of the sector this sidedef faces.
    pub sector: SectorIdx,
    /// The horizontal texture offset, in map units.
    pub x_offset: i32,
    /// The vertical texture offset, in map units.
    pub y_offset: i32,
    /// The upper texture, or an empty name if none. A Doom 64 map's [`TextureRef::Index`]
    /// has no name until the texture layer (v0.5.0) can resolve it.
    pub upper: TextureRef,
    /// The lower texture, or an empty name if none. A Doom 64 map's [`TextureRef::Index`]
    /// has no name until the texture layer (v0.5.0) can resolve it.
    pub lower: TextureRef,
    /// The middle texture, or an empty name if none. A Doom 64 map's [`TextureRef::Index`]
    /// has no name until the texture layer (v0.5.0) can resolve it.
    pub middle: TextureRef,
}

/// A normalized Doom 64 colored-lighting palette entry (ADR-0021 §4).
///
/// The raw record's trailing `unknown` field (tentative semantics) is not
/// normalized; a consumer needing it reads [`Doom64Map`][crate::map::Doom64Map].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapLight {
    /// Red channel (`0`–`255`).
    pub r: u8,
    /// Green channel (`0`–`255`).
    pub g: u8,
    /// Blue channel (`0`–`255`).
    pub b: u8,
    /// Small identifier (observed `0`–`2` in retail data; tentative semantics).
    pub tag: u8,
}

/// A normalized sector.
#[derive(Debug, Clone, PartialEq)]
pub struct MapSector {
    /// The floor height, in map units.
    pub floor_height: i32,
    /// The ceiling height, in map units.
    pub ceiling_height: i32,
    /// The floor flat (texture). A Doom 64 map's [`TextureRef::Index`] has no
    /// name until the texture layer (v0.5.0) can resolve it.
    pub floor_flat: TextureRef,
    /// The ceiling flat (texture). A Doom 64 map's [`TextureRef::Index`] has no
    /// name until the texture layer (v0.5.0) can resolve it.
    pub ceiling_flat: TextureRef,
    // Doom stores sector special/tag as i16; widen losslessly to i32
    // (avoids an i16->u16 sign-loss cast that clippy::pedantic rejects).
    /// The light level, in the range `0..=255` on disk, widened to `i32`.
    pub light: i32,
    /// The sector special number.
    pub special: i32,
    /// The sector tag.
    pub tag: i32,
    /// Doom 64 colored lighting: five references into [`Map::lights`], carried
    /// positionally — Doom64 EX's map-format headers do not name the slots
    /// (ADR-0021 §4). `None` for every other format.
    pub colors: Option<[LightIdx; 5]>,
    /// The sector's raw Doom 64 flag bits (`Sector.flags`, stored opaquely);
    /// `0` for every other format (mirrors `MapLinedef.flags`).
    pub flags: u32,
}

/// A normalized map thing (monster, item, player start, etc.).
#[derive(Debug, Clone, PartialEq)]
pub struct MapThing {
    /// The thing's X coordinate, in map units.
    pub x: f64,
    /// The thing's Y coordinate, in map units.
    pub y: f64,
    /// The thing's facing angle, in degrees.
    pub angle: u16,
    /// The thing's doomednum, identifying its type.
    pub type_id: u16,
    /// The thing's bit flags in the Doom/Boom-MBF layout — skill 1–2 (bit 0),
    /// skill 3 (bit 1), skill 4–5 (bit 2), ambush (bit 3), not-in-single-player
    /// (bit 4), not-in-deathmatch (bit 5), not-in-co-op (bit 6), friendly
    /// (bit 7). Note that bits 4–6 are *negative*: a clear bit means the thing
    /// **does** appear in that game mode.
    ///
    /// This layout is the graph's single contract — every source format is
    /// normalized into it on read (ADR-0019 §2), so the field means the same
    /// thing regardless of where the map came from:
    ///
    /// - **Doom/Heretic**: the on-disk word, used as-is.
    /// - **Hexen**: translated on assembly. Hexen's game-mode bits are positive
    ///   and live elsewhere (`0x0100` single-player, `0x0200` co-op, `0x0400`
    ///   deathmatch), so they are inverted into bits 4/5/6; Hexen's `dormant`
    ///   (`0x0010`) and fighter/cleric/mage class filters
    ///   (`0x0020`/`0x0040`/`0x0080`) have no Doom equivalent and are dropped.
    ///   Bit 7 (friend) is always `0` — Hexen has no such flag.
    /// - **UDMF**: the discrete booleans (`skill1`…`skill5`, `ambush`, `single`,
    ///   `dm`, `coop`, `friend`) are packed into this layout; the skill pairs
    ///   OR-fold into one bit each and `single`/`dm`/`coop` are inverted.
    pub flags: u32,
    /// The thing's identification tag (Hexen/UDMF tid); `0` for Doom maps and untagged things.
    pub id: i32,
    /// The thing's spawn height above the floor, in map units; `0.0` for Doom maps.
    pub height: f64,
    /// The thing's activation special; `Special { special: 0, args: [0; 5] }` for Doom maps.
    pub special: Special,
}

/// A non-fatal issue recorded during lenient map assembly.
///
/// Produced by [`Map::assemble_with_options`] in [`Strictness::Lenient`] mode
/// and collected on the resulting map; retrieve them via [`Map::warnings`].
///
/// [`Strictness::Lenient`]: crate::Strictness::Lenient
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum MapWarning {
    /// A reference from one arena to another (e.g. a linedef's vertex index)
    /// pointed past the end of the referenced arena and was recovered during
    /// lenient assembly — clamped to an in-range index for a required
    /// reference, or dropped to `None` for an optional left sidedef.
    #[error(
        "{referent} index {index} referenced from {from} is out of range ({count} available); recovered during lenient assembly"
    )]
    DanglingReference {
        /// The name of the arena the out-of-range index referred to (e.g. `"vertex"`).
        referent: &'static str,
        /// The out-of-range index value that was encountered (signed, since UDMF
        /// indices may be negative).
        index: i32,
        /// The name of the element kind the dangling reference was found on (e.g. `"linedef"`).
        from: &'static str,
        /// The number of elements actually available in the referenced arena.
        count: usize,
    },
    /// A field value was outside its target field's representable range and was
    /// clamped during lenient assembly.
    #[error("{field} value {value} on {from} is out of range; clamped during lenient assembly")]
    FieldOutOfRange {
        /// The UDMF field name (e.g. `"thing.type"`).
        field: &'static str,
        /// The element kind the field was on (e.g. `"thing"`).
        from: &'static str,
        /// The offending value.
        value: i32,
    },
    /// A UDMF map lacked its `ENDMAP` terminator and was recovered best-effort.
    #[error("UDMF map '{name}' has no ENDMAP terminator; recovered during lenient assembly")]
    UnterminatedUdmf {
        /// The map's marker name.
        name: String,
    },
}

/// An assembled Doom map graph: normalized elements addressed by index,
/// with infallible resolvers for following references between arenas.
///
/// `Map` is constructed by map assembly ([`Map::assemble`] /
/// [`Map::assemble_with_options`]); its arena fields are crate-private so only
/// assembly builds one directly.
#[derive(Debug, Clone)]
pub struct Map {
    pub(crate) name: String,
    pub(crate) format: MapFormat,
    pub(crate) namespace: Option<String>,
    pub(crate) vertices: Vec<MapVertex>,
    pub(crate) linedefs: Vec<MapLinedef>,
    pub(crate) sidedefs: Vec<MapSidedef>,
    pub(crate) sectors: Vec<MapSector>,
    pub(crate) things: Vec<MapThing>,
    pub(crate) lights: Vec<MapLight>,
    pub(crate) warnings: Vec<MapWarning>,
}

impl Map {
    /// Returns the map's lump name (e.g. `"E1M1"`).
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the source format this map was assembled from.
    #[must_use]
    pub fn format(&self) -> MapFormat {
        self.format
    }

    /// Returns the map's UDMF `namespace` declaration (e.g. `"doom"`), or `None`
    /// for binary-format maps.
    #[must_use]
    pub fn namespace(&self) -> Option<&str> {
        self.namespace.as_deref()
    }

    /// Returns the map's vertex arena.
    #[must_use]
    pub fn vertices(&self) -> &[MapVertex] {
        &self.vertices
    }

    /// Returns the map's linedef arena.
    #[must_use]
    pub fn linedefs(&self) -> &[MapLinedef] {
        &self.linedefs
    }

    /// Returns the map's sidedef arena.
    #[must_use]
    pub fn sidedefs(&self) -> &[MapSidedef] {
        &self.sidedefs
    }

    /// Returns the map's sector arena.
    #[must_use]
    pub fn sectors(&self) -> &[MapSector] {
        &self.sectors
    }

    /// Returns the map's thing arena.
    #[must_use]
    pub fn things(&self) -> &[MapThing] {
        &self.things
    }

    /// Returns the non-fatal warnings collected during assembly.
    #[must_use]
    pub fn warnings(&self) -> &[MapWarning] {
        &self.warnings
    }

    /// The map's colored-lighting palette; empty for non-Doom 64 maps.
    #[must_use]
    pub fn lights(&self) -> &[MapLight] {
        &self.lights
    }

    /// Resolves a linedef's start/end vertices. Total for elements produced by
    /// this map's own assembly; a linedef carrying an out-of-range index (e.g.
    /// hand-constructed, since `MapLinedef`'s fields are public) may panic.
    #[must_use]
    pub fn linedef_vertices(&self, l: &MapLinedef) -> (&MapVertex, &MapVertex) {
        (&self.vertices[l.start.0], &self.vertices[l.end.0])
    }

    /// Resolves a linedef's right (front) sidedef, if present (`None` == no
    /// front side; see [`MapLinedef::right`] for its sources). Total for
    /// elements produced by this map's own assembly; a linedef carrying an
    /// out-of-range index (e.g. hand-constructed, since `MapLinedef`'s fields
    /// are public) may panic.
    #[must_use]
    pub fn linedef_right(&self, l: &MapLinedef) -> Option<&MapSidedef> {
        l.right.map(|i| &self.sidedefs[i.0])
    }

    /// Resolves a linedef's left (back) sidedef, if present (`None` == no
    /// back side; see [`MapLinedef::left`] for its sources). Total for
    /// elements produced by this map's own assembly; a linedef carrying an
    /// out-of-range index (e.g. hand-constructed, since `MapLinedef`'s fields
    /// are public) may panic.
    #[must_use]
    pub fn linedef_left(&self, l: &MapLinedef) -> Option<&MapSidedef> {
        l.left.map(|i| &self.sidedefs[i.0])
    }

    /// Resolves a sidedef's sector. Total for elements produced by this map's
    /// own assembly; a sidedef carrying an out-of-range index (e.g.
    /// hand-constructed, since `MapSidedef`'s fields are public) may panic.
    #[must_use]
    pub fn sidedef_sector(&self, s: &MapSidedef) -> &MapSector {
        &self.sectors[s.sector.0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_map() -> Map {
        Map {
            name: "E1M1".into(),
            format: MapFormat::Doom,
            namespace: None,
            vertices: vec![MapVertex { x: 0.0, y: 0.0 }, MapVertex { x: 64.0, y: 0.0 }],
            sidedefs: vec![MapSidedef {
                sector: SectorIdx(0),
                x_offset: 0,
                y_offset: 0,
                upper: TextureRef::Name(String::new()),
                lower: TextureRef::Name(String::new()),
                middle: TextureRef::Name("WALL".into()),
            }],
            sectors: vec![MapSector {
                floor_height: 0,
                ceiling_height: 128,
                floor_flat: TextureRef::Name("FLOOR".into()),
                ceiling_flat: TextureRef::Name("CEIL".into()),
                light: 160,
                special: 0,
                tag: 0,
                colors: None,
                flags: 0,
            }],
            linedefs: vec![MapLinedef {
                start: VertexIdx(0),
                end: VertexIdx(1),
                right: Some(SidedefIdx(0)),
                left: None,
                flags: 1,
                special: Special {
                    special: 0,
                    args: [0; 5],
                },
                id: 0,
            }],
            things: vec![],
            lights: vec![],
            warnings: vec![],
        }
    }

    #[test]
    fn resolvers_follow_indices() {
        let m = tiny_map();
        let l = &m.linedefs()[0];
        let (a, b) = m.linedef_vertices(l);
        assert_eq!((a.x, b.x), (0.0, 64.0));
        let right = m.linedef_right(l).expect("fronted line");
        assert_eq!(right.middle, "WALL");
        assert!(m.linedef_left(l).is_none());
        assert_eq!(m.sidedef_sector(right).ceiling_height, 128);
    }

    #[test]
    fn classic_map_has_no_lights_and_no_colors() {
        let m = tiny_map();
        assert!(m.lights().is_empty());
        assert_eq!(m.sectors()[0].colors, None);
        assert_eq!(m.sectors()[0].flags, 0);
    }
}
