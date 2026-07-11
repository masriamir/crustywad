//! The assembled, index-addressed map graph (ADR-0015 §2).

/// The source format a [`Map`] was assembled from. Only [`Doom`][MapFormat::Doom]
/// is implemented today; Hexen/UDMF (Epic #17) reuse this same model.
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

/// A normalized map vertex; coordinates are `f64` so binary `i16` widens
/// losslessly and future UDMF floats fit natively.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapVertex {
    /// The vertex's X coordinate, in map units.
    pub x: f64,
    /// The vertex's Y coordinate, in map units.
    pub y: f64,
}

/// A normalized line special: the classic Doom `special_type` + `sector_tag`.
/// Hexen's `special` + `args[5]` fold in here in Epic #17 (fields added then).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineSpecial {
    /// The linedef special/action number.
    pub special: u16,
    /// The sector tag this special applies to.
    pub tag: u16,
}

/// A normalized linedef, referencing its endpoints, sidedefs, and special by
/// index into the owning [`Map`]'s arenas.
#[derive(Debug, Clone, PartialEq)]
pub struct MapLinedef {
    /// The index of the linedef's start vertex.
    pub start: VertexIdx,
    /// The index of the linedef's end vertex.
    pub end: VertexIdx,
    /// The index of the linedef's right (front) sidedef.
    pub right: SidedefIdx,
    /// The index of the linedef's left (back) sidedef. `None` == one-sided
    /// (the `0xffff` sentinel).
    pub left: Option<SidedefIdx>,
    /// The linedef's bit flags (blocking, two-sided, secret, etc.).
    pub flags: u32,
    /// The linedef's special/tag pair.
    pub special: LineSpecial,
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
    /// The upper texture name, or empty if none.
    pub upper: String,
    /// The lower texture name, or empty if none.
    pub lower: String,
    /// The middle texture name, or empty if none.
    pub middle: String,
}

/// A normalized sector.
#[derive(Debug, Clone, PartialEq)]
pub struct MapSector {
    /// The floor height, in map units.
    pub floor_height: i32,
    /// The ceiling height, in map units.
    pub ceiling_height: i32,
    /// The floor flat (texture) name.
    pub floor_flat: String,
    /// The ceiling flat (texture) name.
    pub ceiling_flat: String,
    // Doom stores sector special/tag as i16; widen losslessly to i32
    // (avoids an i16->u16 sign-loss cast that clippy::pedantic rejects).
    /// The light level, in the range `0..=255` on disk, widened to `i32`.
    pub light: i32,
    /// The sector special number.
    pub special: i32,
    /// The sector tag.
    pub tag: i32,
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
    /// The thing's bit flags (skill levels, deaf, multiplayer-only, etc.).
    pub flags: u32,
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
        /// The out-of-range index value that was encountered.
        index: usize,
        /// The name of the element kind the dangling reference was found on (e.g. `"linedef"`).
        from: &'static str,
        /// The number of elements actually available in the referenced arena.
        count: usize,
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
    pub(crate) vertices: Vec<MapVertex>,
    pub(crate) linedefs: Vec<MapLinedef>,
    pub(crate) sidedefs: Vec<MapSidedef>,
    pub(crate) sectors: Vec<MapSector>,
    pub(crate) things: Vec<MapThing>,
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

    /// Resolves a linedef's start/end vertices. Total for elements produced by
    /// this map's own assembly; a linedef carrying an out-of-range index (e.g.
    /// hand-constructed, since `MapLinedef`'s fields are public) may panic.
    #[must_use]
    pub fn linedef_vertices(&self, l: &MapLinedef) -> (&MapVertex, &MapVertex) {
        (&self.vertices[l.start.0], &self.vertices[l.end.0])
    }

    /// Resolves a linedef's right (front) sidedef. Total for elements produced
    /// by this map's own assembly; a linedef carrying an out-of-range index
    /// (e.g. hand-constructed, since `MapLinedef`'s fields are public) may panic.
    #[must_use]
    pub fn linedef_right(&self, l: &MapLinedef) -> &MapSidedef {
        &self.sidedefs[l.right.0]
    }

    /// Resolves a linedef's left (back) sidedef, if two-sided. Total for
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
            vertices: vec![MapVertex { x: 0.0, y: 0.0 }, MapVertex { x: 64.0, y: 0.0 }],
            sidedefs: vec![MapSidedef {
                sector: SectorIdx(0),
                x_offset: 0,
                y_offset: 0,
                upper: String::new(),
                lower: String::new(),
                middle: "WALL".into(),
            }],
            sectors: vec![MapSector {
                floor_height: 0,
                ceiling_height: 128,
                floor_flat: "FLOOR".into(),
                ceiling_flat: "CEIL".into(),
                light: 160,
                special: 0,
                tag: 0,
            }],
            linedefs: vec![MapLinedef {
                start: VertexIdx(0),
                end: VertexIdx(1),
                right: SidedefIdx(0),
                left: None,
                flags: 1,
                special: LineSpecial { special: 0, tag: 0 },
            }],
            things: vec![],
            warnings: vec![],
        }
    }

    #[test]
    fn resolvers_follow_indices() {
        let m = tiny_map();
        let l = &m.linedefs()[0];
        let (a, b) = m.linedef_vertices(l);
        assert_eq!((a.x, b.x), (0.0, 64.0));
        assert_eq!(m.linedef_right(l).middle, "WALL");
        assert!(m.linedef_left(l).is_none());
        assert_eq!(m.sidedef_sector(m.linedef_right(l)).ceiling_height, 128);
    }
}
