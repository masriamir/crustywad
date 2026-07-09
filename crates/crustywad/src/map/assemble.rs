//! Assembling normalized [`Map`]s from a WAD's flat records (ADR-0015 §3–5).

use crate::map::graph::{
    LineSpecial, Map, MapFormat, MapLinedef, MapSector, MapSidedef, MapThing, MapVertex,
    MapWarning, SectorIdx, SidedefIdx, VertexIdx,
};
use crate::map::{MapGroup, MapParseError, common, doom, parse_records};
use crate::{ParseOptions, Strictness, Wad};

/// Fatal errors from [`Map::assemble_with_options`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum MapAssembleError {
    /// A required lump (e.g. `VERTEXES`, `SECTORS`) was not present in the
    /// map's data lumps.
    #[error("map group is missing required lump {lump}")]
    MissingLump {
        /// The name of the missing required lump.
        lump: &'static str,
    },
    /// A required lump's bytes failed to decode into fixed-size records.
    #[error("failed to decode {lump} records: {source}")]
    Records {
        /// The name of the lump whose records failed to decode.
        lump: &'static str,
        /// The underlying decode error.
        #[source]
        source: MapParseError,
    },
    /// A cross-reference (e.g. a linedef's vertex index) pointed outside the
    /// bounds of the referenced arena, and strict mode rejected it.
    #[error("{referent} index {index} referenced from {from} is out of range ({count} available)")]
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

/// Finds the bytes of the data lump named `lump` within `group`.
fn lump_bytes<'w>(wad: &'w Wad, group: &MapGroup, lump: &str) -> Option<&'w [u8]> {
    group
        .data_indices
        .iter()
        .copied()
        .find(|&i| wad.lumps()[i].name() == lump)
        .and_then(|i| wad.lump_bytes(i))
}

/// Decodes a required record lump, mapping absence/decoding failure to errors.
fn decode_required<T>(
    wad: &Wad,
    group: &MapGroup,
    lump: &'static str,
) -> Result<Vec<T>, MapAssembleError>
where
    T: for<'a> binrw::BinRead<Args<'a> = ()>,
{
    let bytes = lump_bytes(wad, group, lump).ok_or(MapAssembleError::MissingLump { lump })?;
    parse_records::<T>(bytes).map_err(|source| MapAssembleError::Records { lump, source })
}

/// Resolves a **required** reference. Empty target arena is always fatal.
fn resolve_required(
    index: u16,
    count: usize,
    referent: &'static str,
    from: &'static str,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<usize, MapAssembleError> {
    let idx = index as usize;
    if count == 0 {
        return Err(MapAssembleError::DanglingReference {
            referent,
            index: idx,
            from,
            count: 0,
        });
    }
    if idx < count {
        return Ok(idx);
    }
    match strictness {
        Strictness::Strict => Err(MapAssembleError::DanglingReference {
            referent,
            index: idx,
            from,
            count,
        }),
        Strictness::Lenient => {
            warnings.push(MapWarning::DanglingReference {
                referent,
                index: idx,
                from,
                count,
            });
            Ok(0) // clamp to a valid in-range fallback
        }
    }
}

/// Resolves a linedef's **left** sidedef: `0xffff` == one-sided (`None`);
/// any other out-of-range value errors (strict) or becomes `None` + warning (lenient).
fn resolve_left(
    raw: u16,
    count: usize,
    from: &'static str,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<Option<usize>, MapAssembleError> {
    if raw == 0xffff {
        return Ok(None);
    }
    let idx = raw as usize;
    if idx < count {
        return Ok(Some(idx));
    }
    match strictness {
        Strictness::Strict => Err(MapAssembleError::DanglingReference {
            referent: "sidedef",
            index: idx,
            from,
            count,
        }),
        Strictness::Lenient => {
            warnings.push(MapWarning::DanglingReference {
                referent: "sidedef",
                index: idx,
                from,
                count,
            });
            Ok(None)
        }
    }
}

/// Widens raw `VERTEXES` records into normalized [`MapVertex`]es.
fn normalize_vertices(raw: &[common::Vertex]) -> Vec<MapVertex> {
    raw.iter()
        .map(|v| MapVertex {
            x: f64::from(v.x),
            y: f64::from(v.y),
        })
        .collect()
}

/// Widens raw `SECTORS` records into normalized [`MapSector`]s.
fn normalize_sectors(raw: &[common::Sector]) -> Vec<MapSector> {
    raw.iter()
        .map(|s| MapSector {
            floor_height: i32::from(s.floor_height),
            ceiling_height: i32::from(s.ceiling_height),
            floor_flat: s.floor_texture.as_str_lossy(),
            ceiling_flat: s.ceiling_texture.as_str_lossy(),
            light: i32::from(s.light_level),
            special: i32::from(s.special_type),
            tag: i32::from(s.tag),
        })
        .collect()
}

/// Widens raw `THINGS` records into normalized [`MapThing`]s.
fn normalize_things(raw: &[doom::Thing]) -> Vec<MapThing> {
    raw.iter()
        .map(|t| MapThing {
            x: f64::from(t.x),
            y: f64::from(t.y),
            angle: t.angle,
            type_id: t.type_id,
            flags: u32::from(t.flags),
        })
        .collect()
}

/// Widens raw `SIDEDEFS` records into normalized [`MapSidedef`]s, validating
/// each sidedef's sector cross-reference.
fn normalize_sidedefs(
    raw: &[common::Sidedef],
    sector_count: usize,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<Vec<MapSidedef>, MapAssembleError> {
    let mut sidedefs = Vec::with_capacity(raw.len());
    for sd in raw {
        let sector = SectorIdx(resolve_required(
            sd.sector,
            sector_count,
            "sector",
            "sidedef",
            strictness,
            warnings,
        )?);
        sidedefs.push(MapSidedef {
            sector,
            x_offset: i32::from(sd.x_offset),
            y_offset: i32::from(sd.y_offset),
            upper: sd.upper_texture.as_str_lossy(),
            lower: sd.lower_texture.as_str_lossy(),
            middle: sd.middle_texture.as_str_lossy(),
        });
    }
    Ok(sidedefs)
}

/// Widens raw `LINEDEFS` records into normalized [`MapLinedef`]s, validating
/// each linedef's vertex and sidedef cross-references.
fn normalize_linedefs(
    raw: &[doom::Linedef],
    vertex_count: usize,
    sidedef_count: usize,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<Vec<MapLinedef>, MapAssembleError> {
    let mut linedefs = Vec::with_capacity(raw.len());
    for ld in raw {
        let start = VertexIdx(resolve_required(
            ld.start_vertex,
            vertex_count,
            "vertex",
            "linedef",
            strictness,
            warnings,
        )?);
        let end = VertexIdx(resolve_required(
            ld.end_vertex,
            vertex_count,
            "vertex",
            "linedef",
            strictness,
            warnings,
        )?);
        let right = SidedefIdx(resolve_required(
            ld.right_sidedef,
            sidedef_count,
            "sidedef",
            "linedef",
            strictness,
            warnings,
        )?);
        let left = resolve_left(
            ld.left_sidedef,
            sidedef_count,
            "linedef",
            strictness,
            warnings,
        )?
        .map(SidedefIdx);
        linedefs.push(MapLinedef {
            start,
            end,
            right,
            left,
            flags: u32::from(ld.flags),
            special: LineSpecial {
                special: ld.special_type,
                tag: ld.sector_tag,
            },
        });
    }
    Ok(linedefs)
}

impl Map {
    /// Assembles a map from a WAD and one of its groups, using strict rules.
    ///
    /// This is a convenience wrapper over [`Map::assemble_with_options`] with
    /// [`ParseOptions::default()`], which is strict: the first out-of-range
    /// cross-reference or structural failure aborts assembly.
    ///
    /// # Errors
    /// Returns [`MapAssembleError`] if a required lump is missing, a record lump
    /// fails to decode, or any cross-reference is out of range.
    pub fn assemble(wad: &Wad, group: &MapGroup) -> Result<Map, MapAssembleError> {
        Map::assemble_with_options(wad, group, ParseOptions::default())
    }

    /// Assembles a map under explicit options (ADR-0015 §3).
    ///
    /// # Errors
    /// Returns [`MapAssembleError`] if a required lump is missing, a record lump
    /// fails to decode, or (in strict mode) a cross-reference is out of range.
    /// In lenient mode only structural failures (missing lump, undecodable
    /// records, an empty *required* target arena) return an error.
    pub fn assemble_with_options(
        wad: &Wad,
        group: &MapGroup,
        options: ParseOptions,
    ) -> Result<Map, MapAssembleError> {
        let s = options.strictness;
        let mut warnings = Vec::new();

        let raw_verts = decode_required::<common::Vertex>(wad, group, "VERTEXES")?;
        let raw_sectors = decode_required::<common::Sector>(wad, group, "SECTORS")?;
        let raw_sides = decode_required::<common::Sidedef>(wad, group, "SIDEDEFS")?;
        let raw_lines = decode_required::<doom::Linedef>(wad, group, "LINEDEFS")?;
        let raw_things = decode_required::<doom::Thing>(wad, group, "THINGS")?;

        let vertices = normalize_vertices(&raw_verts);
        let sectors = normalize_sectors(&raw_sectors);
        let things = normalize_things(&raw_things);
        let sidedefs = normalize_sidedefs(&raw_sides, sectors.len(), s, &mut warnings)?;
        let linedefs =
            normalize_linedefs(&raw_lines, vertices.len(), sidedefs.len(), s, &mut warnings)?;

        Ok(Map {
            name: group.name.clone(),
            format: MapFormat::Doom,
            vertices,
            linedefs,
            sidedefs,
            sectors,
            things,
            warnings,
        })
    }
}
