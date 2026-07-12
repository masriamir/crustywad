//! Assembling normalized [`Map`]s from a WAD's flat records (ADR-0015 §3–5).

use crate::map::graph::{
    Map, MapFormat, MapLinedef, MapSector, MapSidedef, MapThing, MapVertex, MapWarning, SectorIdx,
    SidedefIdx, Special, VertexIdx,
};
use crate::map::{MapGroup, MapParseError, common, doom, hexen, parse_records};
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
        /// The out-of-range index value that was encountered (signed, since UDMF
        /// indices may be negative).
        index: i32,
        /// The name of the element kind the dangling reference was found on (e.g. `"linedef"`).
        from: &'static str,
        /// The number of elements actually available in the referenced arena.
        count: usize,
    },
    /// The map group is in a text-based format that assembly does not yet decode
    /// as binary records — currently only UDMF (`TEXTMAP`, tracked in Epic #17).
    /// Binary Doom and Hexen maps assemble normally; a `TEXTMAP` lump marks a
    /// format whose records would otherwise silently mis-decode, so assembly
    /// refuses it up front.
    #[error(
        "unsupported map format: found a {lump} lump; assembly does not support this text-based format yet"
    )]
    UnsupportedFormat {
        /// The format-specific marker lump detected (currently always `"TEXTMAP"`).
        lump: &'static str,
    },
    /// The `TEXTMAP` text failed to decode or parse as UDMF.
    #[error("failed to parse UDMF text map: {source}")]
    Udmf {
        /// The underlying UDMF parse error.
        #[source]
        source: crate::map::udmf::UdmfParseError,
    },
    /// A UDMF map (`TEXTMAP` present) had no `ENDMAP` terminator lump (strict mode).
    #[error("UDMF map '{name}' has no ENDMAP terminator lump")]
    UnterminatedUdmf {
        /// The map's marker name.
        name: String,
    },
    /// A field value was outside its target field's representable range (strict mode).
    #[error("{field} value {value} on {from} is out of range")]
    FieldOutOfRange {
        /// The UDMF field name.
        field: &'static str,
        /// The element kind.
        from: &'static str,
        /// The offending value.
        value: i32,
    },
}

/// Finds the bytes of the data lump named `lump` within `group`.
fn lump_bytes<'w>(wad: &'w Wad, group: &MapGroup, lump: &str) -> Option<&'w [u8]> {
    group
        .data_indices
        .iter()
        .copied()
        .find(|&i| wad.lumps().get(i).is_some_and(|l| l.name() == lump))
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
///
/// `index` is `i32` so UDMF's signed, wider indices share this validator with the
/// binary formats (whose non-negative `u16` indices widen losslessly); a negative
/// index is treated as out of range, taking the same dangling-reference path. The
/// raw signed `index` is preserved in the diagnostic (error/warning).
fn resolve_required(
    index: i32,
    count: usize,
    referent: &'static str,
    from: &'static str,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<usize, MapAssembleError> {
    if count == 0 {
        return Err(MapAssembleError::DanglingReference {
            referent,
            index,
            from,
            count: 0,
        });
    }
    // A negative index (or one past `count`) is out of range.
    if let Ok(idx) = usize::try_from(index) {
        if idx < count {
            return Ok(idx);
        }
    }
    match strictness {
        Strictness::Strict => Err(MapAssembleError::DanglingReference {
            referent,
            index,
            from,
            count,
        }),
        Strictness::Lenient => {
            warnings.push(MapWarning::DanglingReference {
                referent,
                index,
                from,
                count,
            });
            Ok(0) // clamp to a valid in-range fallback
        }
    }
}

/// Range-checks an **optional** sidedef reference with no binary sentinel.
///
/// Used by the UDMF normalizer, which supplies `sideback` already stripped of
/// its `-1` one-sided sentinel (the parser mapped `-1 -> None`), so a real index
/// — including `65535` — is validated normally rather than treated as the binary
/// `0xffff` "no back side" marker. In range -> `Some(idx)`; otherwise strict
/// error / lenient `None` + `DanglingReference` warning.
fn resolve_optional(
    idx: i32,
    count: usize,
    from: &'static str,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<Option<usize>, MapAssembleError> {
    if let Ok(u) = usize::try_from(idx) {
        if u < count {
            return Ok(Some(u));
        }
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

/// Resolves a linedef's **left** sidedef against the binary `0xffff` sentinel.
///
/// `0xffff` (65535) is the on-disk Doom/Hexen "no back side" marker: `sideback`
/// is a `u16` there, so that value means one-sided and maps to `None`. Any other
/// value outside `0..count` errors (strict) or becomes `None` + a warning
/// (lenient); a negative index (reachable only via the widened signed parameter)
/// is simply out of range.
///
/// `raw` is `i32` so binary and UDMF callers can share this validator, but the
/// UDMF normalizer (#58b) must **not** route a raw sidedef index through this
/// `0xffff` sentinel. Per ADR-0017 §2/§3 it receives `sideback` already
/// normalized to `Option<i32>` (`-1` → `None` in the parser), maps `None`
/// straight to `left: None`, and range-checks a real `Some(idx)` — so a valid
/// UDMF sidedef index of 65535 is never mistaken for the one-sided sentinel.
fn resolve_left(
    raw: i32,
    count: usize,
    from: &'static str,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<Option<usize>, MapAssembleError> {
    if raw == 0xffff {
        return Ok(None);
    }
    resolve_optional(raw, count, from, strictness, warnings)
}

/// Resolves a linedef's four cross-references (start/end vertex, right/left
/// sidedef) — the resolution is identical for the Doom and Hexen layouts, so
/// both normalizers share it. `left_sidedef == 0xffff` yields `None` (one-sided).
// The four raw fields plus the two arena counts, strictness, and the warnings
// sink are each independently meaningful (not a natural struct); grouping them
// would only relocate, not reduce, the parameter count.
#[allow(clippy::too_many_arguments)]
fn resolve_linedef_refs(
    start_vertex: u16,
    end_vertex: u16,
    right_sidedef: u16,
    left_sidedef: u16,
    vertex_count: usize,
    sidedef_count: usize,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<(VertexIdx, VertexIdx, SidedefIdx, Option<SidedefIdx>), MapAssembleError> {
    let start = VertexIdx(resolve_required(
        i32::from(start_vertex),
        vertex_count,
        "vertex",
        "linedef",
        strictness,
        warnings,
    )?);
    let end = VertexIdx(resolve_required(
        i32::from(end_vertex),
        vertex_count,
        "vertex",
        "linedef",
        strictness,
        warnings,
    )?);
    let right = SidedefIdx(resolve_required(
        i32::from(right_sidedef),
        sidedef_count,
        "sidedef",
        "linedef",
        strictness,
        warnings,
    )?);
    let left = resolve_left(
        i32::from(left_sidedef),
        sidedef_count,
        "linedef",
        strictness,
        warnings,
    )?
    .map(SidedefIdx);
    Ok((start, end, right, left))
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
            id: 0,
            height: 0.0,
            special: Special {
                special: 0,
                args: [0; 5],
            },
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
            i32::from(sd.sector),
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
        let (start, end, right, left) = resolve_linedef_refs(
            ld.start_vertex,
            ld.end_vertex,
            ld.right_sidedef,
            ld.left_sidedef,
            vertex_count,
            sidedef_count,
            strictness,
            warnings,
        )?;
        linedefs.push(MapLinedef {
            start,
            end,
            right,
            left,
            flags: u32::from(ld.flags),
            special: Special {
                special: i32::from(ld.special_type),
                args: [i32::from(ld.sector_tag), 0, 0, 0, 0],
            },
            id: 0,
        });
    }
    Ok(linedefs)
}

/// Widens raw Hexen `THINGS` records into normalized [`MapThing`]s.
fn normalize_things_hexen(raw: &[hexen::Thing]) -> Vec<MapThing> {
    raw.iter()
        .map(|t| MapThing {
            x: f64::from(t.x),
            y: f64::from(t.y),
            angle: t.angle,
            type_id: t.type_id,
            flags: u32::from(t.flags),
            id: i32::from(t.tid),
            height: f64::from(t.z),
            special: Special {
                special: i32::from(t.special),
                args: t.args.map(i32::from),
            },
        })
        .collect()
}

/// Widens raw Hexen `LINEDEFS` records into normalized [`MapLinedef`]s, validating
/// each linedef's vertex and sidedef cross-references (via [`resolve_linedef_refs`]).
fn normalize_linedefs_hexen(
    raw: &[hexen::Linedef],
    vertex_count: usize,
    sidedef_count: usize,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<Vec<MapLinedef>, MapAssembleError> {
    let mut linedefs = Vec::with_capacity(raw.len());
    for ld in raw {
        let (start, end, right, left) = resolve_linedef_refs(
            ld.start_vertex,
            ld.end_vertex,
            ld.right_sidedef,
            ld.left_sidedef,
            vertex_count,
            sidedef_count,
            strictness,
            warnings,
        )?;
        linedefs.push(MapLinedef {
            start,
            end,
            right,
            left,
            flags: u32::from(ld.flags),
            special: Special {
                special: i32::from(ld.special),
                args: ld.args.map(i32::from),
            },
            id: 0,
        });
    }
    Ok(linedefs)
}

/// Narrows an `i32` UDMF value into `u16`: strict rejects out-of-range;
/// lenient clamps to `u16` bounds and records a [`MapWarning::FieldOutOfRange`].
#[allow(dead_code)]
fn coerce_u16(
    value: i32,
    field: &'static str,
    from: &'static str,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<u16, MapAssembleError> {
    if let Ok(v) = u16::try_from(value) {
        return Ok(v);
    }
    match strictness {
        Strictness::Strict => Err(MapAssembleError::FieldOutOfRange { field, from, value }),
        Strictness::Lenient => {
            warnings.push(MapWarning::FieldOutOfRange { field, from, value });
            Ok(if value < 0 { 0 } else { u16::MAX })
        }
    }
}

/// Widens raw UDMF `VERTEX` records into normalized [`MapVertex`]es.
#[allow(dead_code)]
fn normalize_udmf_vertices(raw: &[crate::map::udmf::UdmfVertex]) -> Vec<MapVertex> {
    raw.iter().map(|v| MapVertex { x: v.x, y: v.y }).collect()
}

/// Widens raw UDMF `SECTOR` records into normalized [`MapSector`]s.
#[allow(dead_code)]
fn normalize_udmf_sectors(raw: &[crate::map::udmf::UdmfSector]) -> Vec<MapSector> {
    raw.iter()
        .map(|s| MapSector {
            floor_height: s.heightfloor,
            ceiling_height: s.heightceiling,
            floor_flat: s.texturefloor.clone(),
            ceiling_flat: s.textureceiling.clone(),
            light: s.lightlevel,
            special: s.special,
            tag: s.id,
        })
        .collect()
}

/// Widens raw UDMF `SIDEDEF` records into normalized [`MapSidedef`]s, validating
/// each sidedef's sector cross-reference.
#[allow(dead_code)]
fn normalize_udmf_sidedefs(
    raw: &[crate::map::udmf::UdmfSidedef],
    sector_count: usize,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<Vec<MapSidedef>, MapAssembleError> {
    let mut out = Vec::with_capacity(raw.len());
    for sd in raw {
        let sector = SectorIdx(resolve_required(
            sd.sector,
            sector_count,
            "sector",
            "sidedef",
            strictness,
            warnings,
        )?);
        out.push(MapSidedef {
            sector,
            x_offset: sd.offsetx,
            y_offset: sd.offsety,
            upper: sd.texturetop.clone(),
            lower: sd.texturebottom.clone(),
            middle: sd.texturemiddle.clone(),
        });
    }
    Ok(out)
}

/// Widens raw UDMF `LINEDEF` records into normalized [`MapLinedef`]s, validating
/// each linedef's vertex and sidedef cross-references. Does not use the binary
/// `0xffff` sentinel for one-sided; instead routes `sideback: None` to `left: None`
/// and validates real `Some(idx)` values via [`resolve_optional`] (ADR-0017 §2).
#[allow(dead_code)]
fn normalize_udmf_linedefs(
    raw: &[crate::map::udmf::UdmfLinedef],
    vertex_count: usize,
    sidedef_count: usize,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<Vec<MapLinedef>, MapAssembleError> {
    let mut out = Vec::with_capacity(raw.len());
    for ld in raw {
        let start = VertexIdx(resolve_required(
            ld.v1,
            vertex_count,
            "vertex",
            "linedef",
            strictness,
            warnings,
        )?);
        let end = VertexIdx(resolve_required(
            ld.v2,
            vertex_count,
            "vertex",
            "linedef",
            strictness,
            warnings,
        )?);
        let right = SidedefIdx(resolve_required(
            ld.sidefront,
            sidedef_count,
            "sidedef",
            "linedef",
            strictness,
            warnings,
        )?);
        let left = match ld.sideback {
            None => None,
            Some(idx) => resolve_optional(idx, sidedef_count, "linedef", strictness, warnings)?
                .map(SidedefIdx),
        };
        out.push(MapLinedef {
            start,
            end,
            right,
            left,
            flags: ld.flags,
            special: Special {
                special: ld.special,
                args: ld.args,
            },
            id: ld.id,
        });
    }
    Ok(out)
}

/// Widens raw UDMF `THING` records into normalized [`MapThing`]s, coercing
/// `type_id` to `u16` and wrapping `angle` modulo 360.
#[allow(dead_code)]
fn normalize_udmf_things(
    raw: &[crate::map::udmf::UdmfThing],
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<Vec<MapThing>, MapAssembleError> {
    let mut out = Vec::with_capacity(raw.len());
    for t in raw {
        let type_id = coerce_u16(t.type_id, "thing.type", "thing", strictness, warnings)?;
        // rem_euclid(360) yields 0..=359, which always fits u16.
        let angle = u16::try_from(t.angle.rem_euclid(360)).unwrap_or(0);
        out.push(MapThing {
            x: t.x,
            y: t.y,
            angle,
            type_id,
            flags: 0, // UDMF thing flags are not modeled in Map yet (ADR-0017 §1).
            id: t.id,
            height: t.height,
            special: Special {
                special: t.special,
                args: t.args,
            },
        });
    }
    Ok(out)
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

        // UDMF (TEXTMAP) is a text format with no binary assembler yet (#58);
        // refuse it explicitly rather than mis-decode it as binary records.
        if lump_bytes(wad, group, "TEXTMAP").is_some() {
            return Err(MapAssembleError::UnsupportedFormat { lump: "TEXTMAP" });
        }
        let format = crate::map::detect_map_format(wad, group); // Doom | Hexen

        // Records shared by both binary formats.
        let raw_verts = decode_required::<common::Vertex>(wad, group, "VERTEXES")?;
        let raw_sectors = decode_required::<common::Sector>(wad, group, "SECTORS")?;
        let raw_sides = decode_required::<common::Sidedef>(wad, group, "SIDEDEFS")?;

        let vertices = normalize_vertices(&raw_verts);
        let sectors = normalize_sectors(&raw_sectors);
        let sidedefs = normalize_sidedefs(&raw_sides, sectors.len(), s, &mut warnings)?;

        // Format-specific THINGS/LINEDEFS.
        let (things, linedefs) = match format {
            MapFormat::Doom => {
                let raw_lines = decode_required::<doom::Linedef>(wad, group, "LINEDEFS")?;
                let raw_things = decode_required::<doom::Thing>(wad, group, "THINGS")?;
                let linedefs = normalize_linedefs(
                    &raw_lines,
                    vertices.len(),
                    sidedefs.len(),
                    s,
                    &mut warnings,
                )?;
                (normalize_things(&raw_things), linedefs)
            }
            MapFormat::Hexen => {
                let raw_lines = decode_required::<hexen::Linedef>(wad, group, "LINEDEFS")?;
                let raw_things = decode_required::<hexen::Thing>(wad, group, "THINGS")?;
                let linedefs = normalize_linedefs_hexen(
                    &raw_lines,
                    vertices.len(),
                    sidedefs.len(),
                    s,
                    &mut warnings,
                )?;
                (normalize_things_hexen(&raw_things), linedefs)
            }
        };

        Ok(Map {
            name: group.name.clone(),
            format,
            namespace: None,
            vertices,
            linedefs,
            sidedefs,
            sectors,
            things,
            warnings,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        normalize_udmf_linedefs, normalize_udmf_things, resolve_left, resolve_optional,
        resolve_required,
    };
    use crate::map::graph::{SidedefIdx, VertexIdx};
    use crate::map::udmf::{UdmfLinedef, UdmfThing};
    use crate::{Strictness, map::MapWarning};

    #[test]
    fn resolve_required_negative_index_is_out_of_range() {
        let mut warnings = Vec::new();
        // Strict: a negative (UDMF-style) index is a dangling reference.
        assert!(
            resolve_required(
                -1,
                4,
                "vertex",
                "linedef",
                Strictness::Strict,
                &mut warnings
            )
            .is_err()
        );
        assert!(warnings.is_empty());
        // Lenient: clamps to 0 and records a warning.
        let idx = resolve_required(
            -1,
            4,
            "vertex",
            "linedef",
            Strictness::Lenient,
            &mut warnings,
        )
        .expect("lenient recovers");
        assert_eq!(idx, 0);
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn resolve_left_sentinel_and_negative() {
        let mut warnings = Vec::new();
        // The binary 0xffff one-sided sentinel resolves to `None`.
        assert_eq!(
            resolve_left(0xffff, 4, "linedef", Strictness::Strict, &mut warnings).unwrap(),
            None
        );
        assert!(warnings.is_empty());
        // A negative index (not the sentinel) is out of range → lenient `None` + warning.
        assert_eq!(
            resolve_left(-2, 4, "linedef", Strictness::Lenient, &mut warnings).unwrap(),
            None
        );
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn resolve_optional_has_no_binary_sentinel() {
        let mut w = Vec::new();
        // 65535 is a VALID index when count is large enough (no 0xffff sentinel).
        assert_eq!(
            resolve_optional(0xffff, 70000, "linedef", Strictness::Strict, &mut w).unwrap(),
            Some(0xffff)
        );
        assert!(w.is_empty());
        // Out of range -> lenient None + warning.
        assert_eq!(
            resolve_optional(5, 4, "linedef", Strictness::Lenient, &mut w).unwrap(),
            None
        );
        assert_eq!(w.len(), 1);
    }

    #[test]
    fn normalize_udmf_linedef_sideback_none_and_valid_65535() {
        let mut w = Vec::new();
        // sideback None -> left None; a valid Some(1) with 2 sidedefs -> Some(1).
        let lines = [UdmfLinedef {
            v1: 0,
            v2: 1,
            sidefront: 0,
            sideback: Some(1),
            id: 7,
            special: 80,
            args: [1, 2, 0, 0, 0],
            flags: 0b101,
        }];
        let out = normalize_udmf_linedefs(&lines, 2, 2, Strictness::Strict, &mut w).unwrap();
        assert_eq!(out[0].start, VertexIdx(0));
        assert_eq!(out[0].end, VertexIdx(1));
        assert_eq!(out[0].right, SidedefIdx(0));
        assert_eq!(out[0].left, Some(SidedefIdx(1)));
        assert_eq!(out[0].id, 7);
        assert_eq!(out[0].flags, 0b101);
        assert_eq!(out[0].special.special, 80);
        assert_eq!(out[0].special.args, [1, 2, 0, 0, 0]);

        let one_sided = [UdmfLinedef {
            v1: 0,
            v2: 1,
            sidefront: 0,
            sideback: None,
            id: -1,
            special: 0,
            args: [0; 5],
            flags: 0,
        }];
        let out2 = normalize_udmf_linedefs(&one_sided, 2, 2, Strictness::Strict, &mut w).unwrap();
        assert_eq!(out2[0].left, None);
    }

    #[test]
    fn normalize_udmf_thing_narrows_type_and_wraps_angle() {
        let mut w = Vec::new();
        let things = [UdmfThing {
            x: 1.0,
            y: 2.0,
            height: 3.0,
            angle: 450,
            type_id: 1,
            id: 5,
            special: 0,
            args: [0; 5],
        }];
        let out = normalize_udmf_things(&things, Strictness::Strict, &mut w).unwrap();
        assert_eq!((out[0].x, out[0].y, out[0].height), (1.0, 2.0, 3.0));
        assert_eq!(out[0].angle, 90); // 450 rem_euclid 360
        assert_eq!(out[0].type_id, 1);
        assert_eq!(out[0].id, 5);
        assert_eq!(out[0].flags, 0);
    }

    #[test]
    fn thing_type_overflow_strict_errors_lenient_clamps() {
        let mut w = Vec::new();
        let bad = [UdmfThing {
            x: 0.0,
            y: 0.0,
            height: 0.0,
            angle: 0,
            type_id: 70000,
            id: 0,
            special: 0,
            args: [0; 5],
        }];
        assert!(normalize_udmf_things(&bad, Strictness::Strict, &mut w).is_err());
        let out = normalize_udmf_things(&bad, Strictness::Lenient, &mut w).unwrap();
        assert_eq!(out[0].type_id, u16::MAX);
        assert!(
            w.iter()
                .any(|x| matches!(x, MapWarning::FieldOutOfRange { .. }))
        );
    }
}
