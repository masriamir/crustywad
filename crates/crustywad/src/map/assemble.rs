//! Assembling normalized [`Map`]s from a WAD's flat records (ADR-0015 §3–5).

use crate::map::graph::{
    Map, MapFormat, MapLinedef, MapSector, MapSidedef, MapThing, MapVertex, MapWarning, SectorIdx,
    SidedefIdx, Special, TextureRef, VertexIdx,
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
    /// The map group is in a format assembly does not yet decode into a
    /// [`Map`] — currently a Doom 64 group (a `MAPxx` marker carrying nested
    /// `IWAD`/`PWAD` magic, ADR-0021 §1; assembly tracked in Epic #17). Doom,
    /// Hexen, and UDMF maps assemble normally today.
    #[error(
        "unsupported map format: found a {lump} lump; assembly does not support this format yet"
    )]
    UnsupportedFormat {
        /// The format-specific marker lump detected.
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

/// Resolves a binary linedef's sidedef reference (either side) against the
/// `0xffff` sentinel.
///
/// `0xffff` (65535) is the on-disk Doom/Hexen "no side" marker for **both**
/// sidedef fields — vanilla engines guard `sidenum[0]` and `sidenum[1]`
/// identically (`!= -1`; Chocolate Doom/Hexen `P_LoadLineDefs`), so a front of
/// `0xffff` is a valid frontless line, not a defect (ADR-0020). The sentinel
/// maps to `None` in both strictness modes with no warning. Any other value
/// outside `0..count` errors (strict) or becomes `None` + a warning (lenient);
/// a negative index (reachable only via the widened signed parameter) is
/// simply out of range.
///
/// This helper is for the **binary** (Doom/Hexen) normalizers only. The UDMF
/// normalizer must **not** call it: per ADR-0017 §2/§3 it range-checks UDMF
/// sidedef indices directly via [`resolve_optional`] — with `sideback` already
/// normalized to `Option<i32>` (`-1` → `None` in the parser) and `sidefront` a
/// required raw integer — so a valid UDMF sidedef index of 65535 is never
/// mistaken for the binary sentinel. (`raw` is `i32` only to reuse the shared
/// range-check plumbing; binary `u16` fields widen into it losslessly.)
fn resolve_binary_side(
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
/// both normalizers share it. `0xffff` in either sidedef field yields `None`
/// for that side (no back side / no front side; ADR-0020).
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
) -> Result<(VertexIdx, VertexIdx, Option<SidedefIdx>, Option<SidedefIdx>), MapAssembleError> {
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
    let right = resolve_binary_side(
        i32::from(right_sidedef),
        sidedef_count,
        "linedef",
        strictness,
        warnings,
    )?
    .map(SidedefIdx);
    let left = resolve_binary_side(
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
            floor_flat: TextureRef::Name(s.floor_texture.as_str_lossy()),
            ceiling_flat: TextureRef::Name(s.ceiling_texture.as_str_lossy()),
            light: i32::from(s.light_level),
            special: i32::from(s.special_type),
            tag: i32::from(s.tag),
            colors: None,
            flags: 0,
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
            upper: TextureRef::Name(sd.upper_texture.as_str_lossy()),
            lower: TextureRef::Name(sd.lower_texture.as_str_lossy()),
            middle: TextureRef::Name(sd.middle_texture.as_str_lossy()),
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

/// Translates a raw Hexen `THINGS` flag word into the graph's single
/// Doom/Boom-MBF thing-flag layout ([`MapThing::flags`], ADR-0019 §2).
///
/// Hexen's on-disk bits are *not* Doom's: its game-mode bits are **positive**
/// ("appears in X") and live at `0x0100`/`0x0200`/`0x0400`, where Doom's are
/// **negative** ("not in X") at bits 4/5/6; and Hexen spends bits 4–7 on
/// `dormant` plus the fighter/cleric/mage class filters. Translating here keeps
/// [`MapThing::flags`] meaning exactly one thing for every source format, so the
/// writers ([`write_udmf`](crate::map::write_udmf),
/// [`write_doom_map`](crate::map::write_doom_map)) can interpret it uniformly.
///
/// | Hexen (on disk) | Normalized |
/// |---|---|
/// | skill 1&2 / 3 / 4&5 (bits 0–2), ambush (bit 3) | copied unchanged |
/// | appears in single-player (`0x0100`) | bit 4 — *not* in single-player (inverted) |
/// | appears in deathmatch (`0x0400`) | bit 5 — *not* in deathmatch (inverted) |
/// | appears in co-op (`0x0200`) | bit 6 — *not* in co-op (inverted) |
/// | dormant (`0x0010`), class filters (`0x0020`/`0x0040`/`0x0080`) | dropped — no Doom equivalent |
/// | — | bit 7 (friend, MBF) is always `0`; Hexen has no equivalent |
///
/// Dropping the dormant and class bits is silent and unwarned, consistent with
/// how ADR-0017/ADR-0019 treat every other unmappable per-format boolean.
fn normalize_hexen_thing_flags(flags: u16) -> u32 {
    /// Hexen "appears in single-player games".
    const HEXEN_SINGLE: u16 = 0x0100;
    /// Hexen "appears in cooperative games".
    const HEXEN_COOP: u16 = 0x0200;
    /// Hexen "appears in deathmatch games".
    const HEXEN_DEATHMATCH: u16 = 0x0400;

    // Skills (bits 0-2) and ambush (bit 3) share Doom's meaning and position.
    let mut out = u32::from(flags & 0x000F);
    if flags & HEXEN_SINGLE == 0 {
        out |= 0x0010; // not in single-player
    }
    if flags & HEXEN_DEATHMATCH == 0 {
        out |= 0x0020; // not in deathmatch
    }
    if flags & HEXEN_COOP == 0 {
        out |= 0x0040; // not in co-op
    }
    out
}

/// Translates Doom 64 on-disk thing flags into the graph's normalized
/// Doom/Boom layout (ADR-0019 §2, ADR-0021 §2).
///
/// Verified against Doom64 EX `doomdef.h`: `MTF_EASY`/`MTF_NORMAL`/`MTF_HARD`/
/// `MTF_AMBUSH`/`MTF_MULTI` (1/2/4/8/16) already sit on the normalized bit
/// positions 0–4 (Doom 64's difficulty bits are positive per-skill spawn
/// flags, matching the normalized meaning); `MTF_NODEATHMATCH` (1024) maps to
/// bit 5 and `MTF_NONETGAME` (2048) to bit 6 (co-op). The Doom 64-only bits —
/// `MTF_SPAWN`/`MTF_ONTOUCH`/`MTF_ONDEATH`/`MTF_SECRET`/`MTF_NOINFIGHTING`/
/// `MTF_NIGHTMARE` — have no normalized slot and drop, exactly as Hexen's
/// dormant/class bits do. Bit 7 (friendly) is never set — Doom 64 has no such
/// flag. The raw word remains available via `Doom64Map`.
// Called by the Doom 64 assemble arm (next task).
#[allow(dead_code)]
fn normalize_doom64_thing_flags(raw: i16) -> u32 {
    #[allow(clippy::cast_sign_loss)] // bit reinterpretation is intended
    let raw = raw as u16;
    let mut flags = u32::from(raw & 0b1_1111); // EASY|NORMAL|HARD|AMBUSH|MULTI
    if raw & 1024 != 0 {
        flags |= 0b10_0000; // not-in-deathmatch
    }
    if raw & 2048 != 0 {
        flags |= 0b100_0000; // not-in-co-op (Doom 64 "standard netgame")
    }
    flags
}

/// Widens raw Hexen `THINGS` records into normalized [`MapThing`]s, translating
/// the Hexen flag word into the graph's Doom/Boom-MBF layout (see
/// [`normalize_hexen_thing_flags`]).
fn normalize_things_hexen(raw: &[hexen::Thing]) -> Vec<MapThing> {
    raw.iter()
        .map(|t| MapThing {
            x: f64::from(t.x),
            y: f64::from(t.y),
            angle: t.angle,
            type_id: t.type_id,
            flags: normalize_hexen_thing_flags(t.flags),
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
fn normalize_udmf_vertices(raw: &[crate::map::udmf::UdmfVertex]) -> Vec<MapVertex> {
    raw.iter().map(|v| MapVertex { x: v.x, y: v.y }).collect()
}

/// Widens raw UDMF `SECTOR` records into normalized [`MapSector`]s.
fn normalize_udmf_sectors(raw: &[crate::map::udmf::UdmfSector]) -> Vec<MapSector> {
    raw.iter()
        .map(|s| MapSector {
            floor_height: s.heightfloor,
            ceiling_height: s.heightceiling,
            floor_flat: TextureRef::Name(s.texturefloor.clone()),
            ceiling_flat: TextureRef::Name(s.textureceiling.clone()),
            light: s.lightlevel,
            special: s.special,
            tag: s.id,
            colors: None,
            flags: 0,
        })
        .collect()
}

/// Widens raw UDMF `SIDEDEF` records into normalized [`MapSidedef`]s, validating
/// each sidedef's sector cross-reference.
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
            upper: TextureRef::Name(sd.texturetop.clone()),
            lower: TextureRef::Name(sd.texturebottom.clone()),
            middle: TextureRef::Name(sd.texturemiddle.clone()),
        });
    }
    Ok(out)
}

/// Widens raw UDMF `LINEDEF` records into normalized [`MapLinedef`]s, validating
/// each linedef's vertex and sidedef cross-references. Does not use the binary
/// `0xffff` sentinel for one-sided; instead routes `sideback: None` to `left: None`
/// and validates real `Some(idx)` values via [`resolve_optional`] (ADR-0017 §2).
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
        // `sidefront` is required by the UDMF parser (spec: no valid default);
        // a dangling or negative value here resolves like any optional sidedef
        // reference — strict error, lenient `None` + warning (ADR-0020 §3) —
        // rather than clamping to index 0, which fabricated a reference not
        // present in the source.
        let right = resolve_optional(ld.sidefront, sidedef_count, "linedef", strictness, warnings)?
            .map(SidedefIdx);
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
/// `type_id` to `u16`, wrapping `angle` modulo 360, and carrying the packed
/// Doom/Boom-MBF thing flags through (ADR-0019, amending ADR-0017 §1).
fn normalize_udmf_things(
    raw: &[crate::map::udmf::UdmfThing],
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<Vec<MapThing>, MapAssembleError> {
    let mut out = Vec::with_capacity(raw.len());
    for t in raw {
        let type_id = coerce_u16(t.type_id, "thing.type", "thing", strictness, warnings)?;
        // `rem_euclid(360)` yields 0..=359 for any i32, which always fits u16;
        // the conversion is infallible by construction.
        let angle = u16::try_from(t.angle.rem_euclid(360))
            .expect("rem_euclid(360) is in 0..=359, which always fits u16");
        out.push(MapThing {
            x: t.x,
            y: t.y,
            angle,
            type_id,
            flags: t.flags,
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
        let mut warnings = Vec::new();

        match crate::map::detect_map_format(wad, group) {
            MapFormat::Udmf => assemble_udmf(wad, group, options, warnings),
            format => {
                let s = options.strictness;

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
                    MapFormat::Udmf => unreachable!("Udmf is handled by the outer match arm"),
                    // Doom64 assembly is not implemented yet (Task 5 adds the
                    // real arm). This IS reachable: detection keys on the
                    // marker's nested IWAD/PWAD magic alone, while Doom64
                    // grouping also requires the MAPxx name — so a
                    // classic-named marker whose bytes carry nested magic
                    // groups classically (with data lumps that can decode
                    // above), yet detects as Doom64 and lands here.
                    MapFormat::Doom64 => {
                        return Err(MapAssembleError::UnsupportedFormat { lump: "MAP" });
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
                    lights: vec![],
                    warnings,
                })
            }
        }
    }
}

/// Assembles a UDMF (`TEXTMAP`) map group into a [`Map`] (ADR-0017 §3).
fn assemble_udmf(
    wad: &Wad,
    group: &MapGroup,
    options: ParseOptions,
    mut warnings: Vec<MapWarning>,
) -> Result<Map, MapAssembleError> {
    let s = options.strictness;

    if !crate::map::group::group_has_lump(wad, group, "ENDMAP") {
        match s {
            Strictness::Strict => {
                return Err(MapAssembleError::UnterminatedUdmf {
                    name: group.name.clone(),
                });
            }
            Strictness::Lenient => warnings.push(MapWarning::UnterminatedUdmf {
                name: group.name.clone(),
            }),
        }
    }

    let bytes = lump_bytes(wad, group, "TEXTMAP")
        .ok_or(MapAssembleError::MissingLump { lump: "TEXTMAP" })?;
    let text = crate::map::udmf::decode_textmap(bytes)
        .map_err(|source| MapAssembleError::Udmf { source })?;
    let udmf = crate::map::udmf::parse_udmf(text, options.limits)
        .map_err(|source| MapAssembleError::Udmf { source })?;

    let vertices = normalize_udmf_vertices(&udmf.vertices);
    let sectors = normalize_udmf_sectors(&udmf.sectors);
    let sidedefs = normalize_udmf_sidedefs(&udmf.sidedefs, sectors.len(), s, &mut warnings)?;
    let linedefs = normalize_udmf_linedefs(
        &udmf.linedefs,
        vertices.len(),
        sidedefs.len(),
        s,
        &mut warnings,
    )?;
    let things = normalize_udmf_things(&udmf.things, s, &mut warnings)?;

    Ok(Map {
        name: group.name.clone(),
        format: MapFormat::Udmf,
        namespace: Some(udmf.namespace),
        vertices,
        linedefs,
        sidedefs,
        sectors,
        things,
        lights: vec![],
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        Map, MapAssembleError, normalize_doom64_thing_flags, normalize_hexen_thing_flags,
        normalize_udmf_linedefs, normalize_udmf_sidedefs, normalize_udmf_things,
        normalize_udmf_vertices, resolve_binary_side, resolve_optional, resolve_required,
    };
    use crate::map::graph::{MapFormat, SidedefIdx, VertexIdx};
    use crate::map::udmf::{UdmfLinedef, UdmfSidedef, UdmfThing};
    use crate::{ParseOptions, Strictness, map::MapWarning};
    use proptest::prelude::*;

    fn encode_i32(value: usize) -> [u8; 4] {
        i32::try_from(value)
            .expect("test fixture values should fit within i32")
            .to_le_bytes()
    }

    /// Builds minimal PWAD bytes from `(name, data)` lump pairs, mirroring the
    /// on-disk layout used by `tests/common/mod.rs::build_wad` and
    /// `group.rs`'s test helper of the same name: a 12-byte header (`PWAD`,
    /// lump count, directory offset), lump payloads, then 16-byte directory
    /// entries (`filepos`, `size`, 8-byte name).
    fn build_pwad(lumps: &[(&str, &[u8])]) -> Vec<u8> {
        let mut payload = Vec::new();
        let mut directory = Vec::new();
        let directory_offset = 12 + lumps.iter().map(|(_, bytes)| bytes.len()).sum::<usize>();

        for (name, bytes) in lumps {
            let filepos = 12 + payload.len();
            payload.extend_from_slice(bytes);
            directory.extend_from_slice(&encode_i32(filepos));
            directory.extend_from_slice(&encode_i32(bytes.len()));
            let mut encoded = [0_u8; 8];
            for (slot, byte) in name.as_bytes().iter().take(8).enumerate() {
                encoded[slot] = *byte;
            }
            directory.extend_from_slice(&encoded);
        }

        let mut wad = Vec::new();
        wad.extend_from_slice(b"PWAD");
        wad.extend_from_slice(&encode_i32(lumps.len()));
        wad.extend_from_slice(&encode_i32(directory_offset));
        wad.extend_from_slice(&payload);
        wad.extend_from_slice(&directory);
        wad
    }

    #[test]
    fn assembles_a_minimal_udmf_map() {
        let text = concat!(
            "namespace = \"doom\";\n",
            "vertex { x = 0.0; y = 0.0; }\n",
            "vertex { x = 64.0; y = 0.0; }\n",
            "linedef { v1 = 0; v2 = 1; sidefront = 0; }\n",
            "sidedef { sector = 0; }\n",
            "sector { texturefloor = \"F\"; textureceiling = \"C\"; }\n",
            "thing { x = 0.0; y = 0.0; type = 1; }\n",
        );
        let wad = crate::Wad::from_bytes(build_pwad(&[
            ("MAP01", b"" as &[u8]),
            ("TEXTMAP", text.as_bytes()),
            ("ENDMAP", b""),
        ]))
        .unwrap();
        let g = crate::map::group::map_group(&wad, "MAP01").unwrap();
        assert_eq!(crate::map::detect_map_format(&wad, &g), MapFormat::Udmf);
        let map = Map::assemble_with_options(&wad, &g, ParseOptions::default()).unwrap();
        assert_eq!(map.namespace(), Some("doom"));
        assert_eq!(map.format(), MapFormat::Udmf);
        assert_eq!(map.vertices().len(), 2);
        assert_eq!(map.linedefs().len(), 1);
        assert_eq!(map.linedefs()[0].left, None);
    }

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
    fn resolve_binary_side_sentinel_and_negative() {
        let mut warnings = Vec::new();
        // The binary 0xffff one-sided sentinel resolves to `None`.
        assert_eq!(
            resolve_binary_side(0xffff, 4, "linedef", Strictness::Strict, &mut warnings).unwrap(),
            None
        );
        assert!(warnings.is_empty());
        // A negative index (not the sentinel) is out of range → lenient `None` + warning.
        assert_eq!(
            resolve_binary_side(-2, 4, "linedef", Strictness::Lenient, &mut warnings).unwrap(),
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
        assert_eq!(out[0].right, Some(SidedefIdx(0)));
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
            flags: 0,
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
            flags: 0,
        }];
        assert!(normalize_udmf_things(&bad, Strictness::Strict, &mut w).is_err());
        let out = normalize_udmf_things(&bad, Strictness::Lenient, &mut w).unwrap();
        assert_eq!(out[0].type_id, u16::MAX);
        assert!(
            w.iter()
                .any(|x| matches!(x, MapWarning::FieldOutOfRange { .. }))
        );
    }

    #[test]
    fn normalize_udmf_sidedef_dangling_sector_strict_errors() {
        // Strict-mode error propagation on a UDMF sidedef's out-of-range sector.
        let mut w = Vec::new();
        let sides = [UdmfSidedef {
            offsetx: 0,
            offsety: 0,
            texturetop: "-".to_owned(),
            texturebottom: "-".to_owned(),
            texturemiddle: "-".to_owned(),
            sector: 99,
        }];
        let err = normalize_udmf_sidedefs(&sides, 1, Strictness::Strict, &mut w).unwrap_err();
        assert!(matches!(err, MapAssembleError::DanglingReference { .. }));
    }

    #[test]
    fn normalize_udmf_linedef_dangling_end_vertex_strict_errors() {
        // Strict-mode error on the second (end/`v2`) vertex reference — `v1` is
        // valid so resolution reaches `v2`.
        let mut w = Vec::new();
        let lines = [UdmfLinedef {
            v1: 0,
            v2: 99,
            sidefront: 0,
            sideback: None,
            id: 0,
            special: 0,
            args: [0; 5],
            flags: 0,
        }];
        let err = normalize_udmf_linedefs(&lines, 2, 1, Strictness::Strict, &mut w).unwrap_err();
        assert!(matches!(err, MapAssembleError::DanglingReference { .. }));
    }

    #[test]
    fn normalize_udmf_linedef_dangling_sidefront_strict_errors() {
        // Strict-mode error on the `sidefront` (right sidedef) reference — the
        // vertices resolve, so resolution reaches `sidefront`.
        let mut w = Vec::new();
        let lines = [UdmfLinedef {
            v1: 0,
            v2: 1,
            sidefront: 99,
            sideback: None,
            id: 0,
            special: 0,
            args: [0; 5],
            flags: 0,
        }];
        let err = normalize_udmf_linedefs(&lines, 2, 1, Strictness::Strict, &mut w).unwrap_err();
        assert!(matches!(err, MapAssembleError::DanglingReference { .. }));
    }

    /// A Hexen thing present in all three game modes (`0x0100` single |
    /// `0x0200` co-op | `0x0400` deathmatch) must normalize to Doom's *negative*
    /// bits 4/5/6 all **clear** — the graph says "not excluded from any mode".
    #[test]
    fn hexen_thing_in_all_game_modes_clears_the_negative_bits() {
        let normalized = normalize_hexen_thing_flags(0x0100 | 0x0200 | 0x0400);
        assert_eq!(normalized & 0x0070, 0, "bits 4/5/6 must all be clear");
        assert_eq!(normalized, 0x0000);
    }

    /// The converse: a Hexen thing naming no game mode appears nowhere, which in
    /// Doom's negative encoding is bits 4/5/6 all **set**.
    #[test]
    fn hexen_thing_in_no_game_mode_sets_the_negative_bits() {
        let normalized = normalize_hexen_thing_flags(0x0000);
        assert_eq!(normalized & 0x0070, 0x0070, "bits 4/5/6 must all be set");
        assert_eq!(normalized, 0x0070);
    }

    /// Each Hexen game-mode bit maps to its own Doom bit, inverted. Note the
    /// crossover: Hexen orders the bits single/co-op/deathmatch, Doom orders
    /// them single/deathmatch/co-op, so co-op and deathmatch swap positions.
    #[test]
    fn hexen_game_mode_bits_invert_into_their_doom_positions() {
        // Single-player only: DM (bit 5) and co-op (bit 6) excluded, SP not.
        assert_eq!(normalize_hexen_thing_flags(0x0100), 0x0060);
        // Co-op only (Hexen 0x0200) -> Doom bit 6 clear, bits 4 and 5 set.
        assert_eq!(normalize_hexen_thing_flags(0x0200), 0x0030);
        // Deathmatch only (Hexen 0x0400) -> Doom bit 5 clear, bits 4 and 6 set.
        assert_eq!(normalize_hexen_thing_flags(0x0400), 0x0050);
    }

    /// Skills (bits 0–2) and ambush (bit 3) share Doom's meaning *and* position,
    /// so they survive verbatim.
    #[test]
    fn hexen_skill_and_ambush_bits_are_preserved() {
        // All skills + ambush, no game modes: low nibble kept, bits 4/5/6 set.
        assert_eq!(normalize_hexen_thing_flags(0x000F), 0x007F);
        // Skill 3 only, in every game mode.
        assert_eq!(normalize_hexen_thing_flags(0x0002 | 0x0700), 0x0002);
    }

    /// `dormant` and the fighter/cleric/mage class filters have no Doom bit and
    /// are dropped — crucially, they must not leak into Doom's bits 4–7, which
    /// they collide with on disk.
    #[test]
    fn hexen_dormant_and_class_bits_are_dropped() {
        // dormant | fighter | cleric | mage, in all three game modes: every one
        // of those bits is unmappable, so nothing but 0 survives.
        let raw = 0x0010 | 0x0020 | 0x0040 | 0x0080 | 0x0100 | 0x0200 | 0x0400;
        assert_eq!(normalize_hexen_thing_flags(raw), 0x0000);
        // Bit 7 (friend, MBF) has no Hexen source and is never set — not even by
        // Hexen's `mage` bit, which occupies that same on-disk position.
        assert_eq!(normalize_hexen_thing_flags(0x0080) & 0x0080, 0);
    }

    #[test]
    fn doom64_thing_flags_translate_to_the_normalized_layout() {
        // Verified against Doom64 EX doomdef.h (ADR-0021 §2): EASY/NORMAL/HARD/
        // AMBUSH/MULTI (1/2/4/8/16) are value-identical to normalized bits 0-4;
        // NODEATHMATCH (1024) -> bit 5; NONETGAME (2048) -> bit 6 (co-op);
        // SPAWN/ONTOUCH/ONDEATH/SECRET/NOINFIGHTING/NIGHTMARE drop.
        assert_eq!(normalize_doom64_thing_flags(0), 0);
        assert_eq!(normalize_doom64_thing_flags(1 | 2 | 4), 0b111);
        assert_eq!(normalize_doom64_thing_flags(8), 0b1000);
        assert_eq!(normalize_doom64_thing_flags(16), 0b1_0000);
        assert_eq!(normalize_doom64_thing_flags(1024), 0b10_0000);
        assert_eq!(normalize_doom64_thing_flags(2048), 0b100_0000);
        // Doom 64-only bits drop; friend (bit 7) is never set.
        assert_eq!(
            normalize_doom64_thing_flags(32 | 64 | 128 | 256 | 512 | 4096),
            0
        );
    }

    proptest! {
        // Arbitrary UTF-8 text, wrapped as a TEXTMAP: whenever it happens to
        // parse as UDMF, normalization must neither panic nor create more
        // vertices than could possibly have been parsed from the input — the
        // O(input) allocation invariant (ADR-0016 item 1) applied to the UDMF
        // assembly surface.
        #[test]
        fn udmf_assembly_never_panics_and_is_bounded(text in ".*") {
            if let Ok(map) = crate::map::udmf::parse_udmf(&text, crate::Limits::default()) {
                // Normalizing cannot create more elements than were parsed.
                let mut w = Vec::new();
                let v = normalize_udmf_vertices(&map.vertices);
                prop_assert!(v.len() <= text.len());
                let _ = normalize_udmf_things(&map.things, Strictness::Lenient, &mut w);
            }
        }
    }
}
