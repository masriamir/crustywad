//! Decoding classic GL node lumps (`GL_VERT`/`GL_SEGS`/`GL_SSECT`/`GL_NODES`,
//! #324).
//!
//! Classic GL nodes are glBSP's precursor to ZDoom's extended node formats
//! (`docs/adr/0025-extended-node-formats.md`): a `GL_<mapname>` marker lump
//! followed by four data lumps holding extra vertices, minisegs, subsectors,
//! and BSP nodes computed for faster in-engine rendering. Several incompatible
//! on-disk versions exist, identified by a magic signature at the head of
//! `GL_VERT` (and, for the V2/V3 split, `GL_SEGS`).
//!
//! This module decodes a complete GL node group. Version detection
//! ([`detect_gl_version`]) classifies the group from the `GL_VERT`/`GL_SEGS`
//! magics, then four lump decoders read the records: `GL_VERT`
//! ([`decode_gl_vertices`]), `GL_SEGS` ([`decode_gl_segs`], which applies the
//! GL/normal vertex high-bit split and resolves partner segs), `GL_SSECT`
//! ([`decode_gl_subsectors`], which validates seg runs), and `GL_NODES`
//! ([`decode_gl_nodes`], which resolves BSP-child references). The orchestrator
//! [`decode_gl_group`] ties them together — detecting the version, refusing
//! V1/V4, stripping the V3-only `gNd3` header, decoding in dependency order,
//! and degrading the whole group cleanly on a structural fault.
//!
//! [`crate::map::group::gl_group_for`] locates a map's `GL_<mapname>` group in
//! the WAD directory (in-WAD only — a `.gwa` sibling file is a deferred
//! follow-up), and [`Map::assemble_with_options`](crate::map::graph::Map::assemble_with_options)
//! decodes it via [`decode_gl_group`] and stores the resulting [`DecodedGl`]
//! arenas on the map graph as `Map::gl_vertices()`/`gl_segs()`/`gl_subsectors()`/
//! `gl_nodes()` — additive to, and independent from, the vanilla
//! `SEGS`/`SSECTORS`/`NODES` BSP. This is unconditional core (no feature flag)
//! on the binary Doom/Hexen assembly path; UDMF and Doom 64 maps never reach
//! it and always report empty GL arenas. See the ADR-0025 amendment for the
//! settled design decisions (separate arenas, `.gwa` deferral, bit masks,
//! hardening).

use crate::Strictness;
use crate::map::assemble::{MapAssembleError, resolve_required};
use crate::map::common::Node as ClassicNode;
use crate::map::graph::{
    GlNode, GlNodeChild, GlNodeIdx, GlSeg, GlSegIdx, GlSubsector, GlSubsectorIdx, GlVertex,
    GlVertexIdx, GlVertexRef, LinedefIdx, MapWarning, VertexIdx,
};
use crate::map::{MapParseError, parse_records};

/// A decodable classic GL node format version.
///
/// glBSP produced several incompatible on-disk layouts over its lifetime.
/// Only V2, V3, and V5 are decodable here — V1 and V4 are refused (see
/// [`detect_gl_version`]).
///
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GlVersion {
    /// glBSP V2: `GL_VERT` begins with `gNd2`, `GL_SEGS` does not begin with
    /// `gNd3`.
    V2,
    /// glBSP V3: `GL_VERT` begins with `gNd2` (the same magic as V2), but
    /// `GL_SEGS` begins with `gNd3` — the documented V3 quirk of carrying its
    /// version marker on the segs lump instead of the verts lump.
    V3,
    /// glBSP V5: `GL_VERT` begins with `gNd5`.
    V5,
}

/// Detects the classic GL node format version from the magic signatures at
/// the head of `GL_VERT` and (for the V2/V3 split) `GL_SEGS`.
///
/// Only called after a GL group has been confirmed present (all four
/// `GL_*` lumps located), so there is no "not a GL group" case to represent —
/// every input is classified as one of the three decodable versions or a
/// refused version number.
///
/// # Errors
///
/// Returns `Err(version)` with the refused glBSP version number when the
/// format is not decodable:
///
/// - `Err(1)` — no recognized magic at the head of `gl_vert` (including a
///   slice shorter than 4 bytes). A `GL_VERT` lump without a recognized magic
///   is classic V1 (raw `i16` vertices, no signature), which this crate does
///   not decode.
/// - `Err(4)` — `gl_vert` begins with `gNd4` (glBSP V4). V4 dropped partner
///   seg information needed to rebuild subsector winding and is refused by
///   gzdoom for the same reason.
///
/// Consumed by [`decode_gl_group`], the group orchestrator.
pub(crate) fn detect_gl_version(gl_vert: &[u8], gl_segs: &[u8]) -> Result<GlVersion, u8> {
    match gl_vert.get(0..4) {
        Some(b"gNd5") => Ok(GlVersion::V5),
        Some(b"gNd4") => Err(4),
        Some(b"gNd2") => {
            if gl_segs.get(0..4) == Some(b"gNd3") {
                Ok(GlVersion::V3)
            } else {
                Ok(GlVersion::V2)
            }
        }
        _ => Err(1),
    }
}

/// Decodes a `GL_VERT` lump (V2/V3/V5 layout) into world-coordinate
/// [`GlVertex`] values.
///
/// The lump begins with a 4-byte magic signature (already identified by
/// [`detect_gl_version`]) followed by a run of vertices, each two
/// little-endian `i32` 16.16 fixed-point coordinates (`x`, then `y`). Each
/// raw coordinate widens losslessly to `f64` world units via `raw as f64 /
/// 65536.0`, mirroring [`MapVertex`](crate::map::graph::MapVertex)'s `i16`
/// widening.
///
/// Bounded and panic-safe: iterates `bytes.get(4..).unwrap_or(&[])` in fixed
/// 8-byte chunks (`chunks_exact`), so memory use is `O(bytes.len())` with no
/// capacity taken from an untrusted count.
///
/// # Errors
///
/// Returns [`MapAssembleError::Records`] (fatal in **both** strictness
/// modes, matching the framing-defect posture of the classic BSP decoders in
/// [`deepbsp`](crate::map::deepbsp)) when:
///
/// - `bytes` is shorter than the 4-byte magic (no room for it), or
/// - the byte count remaining after the magic is not an exact multiple of 8
///   (a partial trailing vertex).
///
/// Consumed by [`decode_gl_group`], the group orchestrator.
pub(crate) fn decode_gl_vertices(bytes: &[u8]) -> Result<Vec<GlVertex>, MapAssembleError> {
    if bytes.len() < 4 {
        return Err(MapAssembleError::Records {
            lump: "GL_VERT",
            source: MapParseError::TrailingBytes { offset: 0 },
        });
    }

    let rest = bytes.get(4..).unwrap_or(&[]);
    if !rest.len().is_multiple_of(8) {
        return Err(MapAssembleError::Records {
            lump: "GL_VERT",
            source: MapParseError::TrailingBytes {
                offset: (4 + (rest.len() / 8) * 8) as u64,
            },
        });
    }

    Ok(rest
        .chunks_exact(8)
        .map(|chunk| {
            let x_raw = i32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            let y_raw = i32::from_le_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]);
            GlVertex {
                x: f64::from(x_raw) / 65536.0,
                y: f64::from(y_raw) / 65536.0,
            }
        })
        .collect())
}

/// Splits a raw `GL_SEGS` vertex reference into a [`GlVertexRef`], applying the
/// version's GL-vertex high-bit convention.
///
/// The high bit(s) of the on-disk index select which arena the remaining bits
/// index into:
///
/// - **V2:** bit `0x8000` set → a `GL_VERT` vertex, index is `raw & 0x7FFF`.
/// - **V3/V5:** either of the top two bits (`0xC000_0000`) set → a `GL_VERT`
///   vertex, index is `raw & 0x3FFF_FFFF` (gzdoom `checkGLVertex3`).
///
/// The extracted index is used **directly** as the 0-based index into
/// `gl_vertices` — unlike gzdoom, which adds a `firstglvertex` offset because it
/// stores GL and normal vertices in one combined array; this crate keeps them in
/// separate arenas, so no offset is applied. A clear flag selects the normal
/// `VERTEXES` arena with the raw index unchanged.
///
/// # Errors
///
/// Propagates [`resolve_required`]'s [`MapAssembleError::DanglingReference`] when
/// the (masked) index is out of range in strict mode.
fn split_vertex(
    raw: u32,
    ver: GlVersion,
    normal_count: usize,
    gl_count: usize,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<GlVertexRef, MapAssembleError> {
    let (is_gl, index) = match ver {
        GlVersion::V2 => {
            const GL_FLAG_V2: u32 = 0x8000; // VERIFIED gzdoom glnodes.cpp
            const GL_MASK_V2: u32 = 0x7FFF; // VERIFIED gzdoom glnodes.cpp
            if raw & GL_FLAG_V2 != 0 {
                (true, raw & GL_MASK_V2)
            } else {
                (false, raw)
            }
        }
        // V3 and V5 GL_SEGS use the identical 16-byte record and the two-top-bit
        // flag (gzdoom `checkGLVertex3`).
        GlVersion::V3 | GlVersion::V5 => {
            const GL_FLAG_V3: u32 = 0xC000_0000; // VERIFIED gzdoom glnodes.cpp checkGLVertex3
            const GL_MASK_V3: u32 = 0x3FFF_FFFF; // VERIFIED gzdoom glnodes.cpp checkGLVertex3
            if raw & GL_FLAG_V3 != 0 {
                (true, raw & GL_MASK_V3)
            } else {
                (false, raw)
            }
        }
    };
    // The masked index always fits a non-negative `i32` (V2: <= 0x7FFF; V3/V5:
    // <= 0x3FFF_FFFF), so this conversion never truncates.
    let signed = i32::try_from(index).unwrap_or(i32::MAX);
    if is_gl {
        Ok(GlVertexRef::Gl(GlVertexIdx(resolve_required(
            signed,
            gl_count,
            "gl vertex",
            "gl seg",
            strictness,
            warnings,
        )?)))
    } else {
        Ok(GlVertexRef::Normal(VertexIdx(resolve_required(
            signed,
            normal_count,
            "vertex",
            "gl seg",
            strictness,
            warnings,
        )?)))
    }
}

/// Reads one raw `GL_SEGS` record's fields as `(v1, v2, linedef, side, partner)`.
///
/// `v1`/`v2` are the raw endpoint words (widened to `u32`, high-bit flag intact
/// for [`split_vertex`]); `side` is the low byte of the 2-byte side field (`0` or
/// `1`); `partner` is `None` for the version's one-sided sentinel (V2 `0xFFFF`,
/// V3/V5 `0xFFFF_FFFF`) and otherwise the raw partner index widened to `u32`.
///
/// `c` must be exactly `record_size` bytes (10 for V2, 16 for V3/V5), as
/// guaranteed by the `chunks_exact` caller.
fn read_seg_record(c: &[u8], ver: GlVersion) -> (u32, u32, u16, u8, Option<u32>) {
    match ver {
        GlVersion::V2 => {
            // VERIFIED gzdoom glnodes.cpp: u16 v1, u16 v2, u16 linedef, u16 side, u16 partner.
            let v1 = u32::from(u16::from_le_bytes([c[0], c[1]]));
            let v2 = u32::from(u16::from_le_bytes([c[2], c[3]]));
            let linedef = u16::from_le_bytes([c[4], c[5]]);
            let side = c[6]; // low byte of the 2-byte side field (value is 0 or 1)
            let partner_raw = u16::from_le_bytes([c[8], c[9]]);
            // VERIFIED gzdoom glnodes.cpp: V2 partner sentinel is 0xFFFF.
            let partner = (partner_raw != 0xFFFF).then_some(u32::from(partner_raw));
            (v1, v2, linedef, side, partner)
        }
        GlVersion::V3 | GlVersion::V5 => {
            // VERIFIED gzdoom glnodes.cpp: i32 v1, i32 v2, u16 linedef, u16 side, i32 partner.
            let v1 = u32::from_le_bytes([c[0], c[1], c[2], c[3]]);
            let v2 = u32::from_le_bytes([c[4], c[5], c[6], c[7]]);
            let linedef = u16::from_le_bytes([c[8], c[9]]);
            let side = c[10]; // low byte of the 2-byte side field (value is 0 or 1)
            let partner_raw = u32::from_le_bytes([c[12], c[13], c[14], c[15]]);
            // VERIFIED gzdoom glnodes.cpp: V3/V5 partner sentinel is 0xFFFF_FFFF.
            let partner = (partner_raw != 0xFFFF_FFFF).then_some(partner_raw);
            (v1, v2, linedef, side, partner)
        }
    }
}

/// Decodes a `GL_SEGS` lump (V2, V3, or V5 layout) into [`GlSeg`] records.
///
/// Record widths and sentinels are version-dependent (all little-endian):
///
/// - **V2:** 10-byte records — `u16 v1, u16 v2, u16 linedef, u16 side,
///   u16 partner`; the `linedef`/`partner` "none" sentinel is `0xFFFF`.
/// - **V3/V5:** 16-byte records (byte-identical, one decode path) — `i32 v1,
///   i32 v2, u16 linedef, u16 side, i32 partner`; the `linedef` sentinel is
///   `0xFFFF` and the `partner` sentinel is `0xFFFF_FFFF`.
///
/// The `v1`/`v2` endpoints carry the GL-vertex high-bit convention decoded by
/// [`split_vertex`]. `linedef == 0xFFFF` marks a GL miniseg (`linedef: None`).
/// `side` is `0` (right/front) or `1` (left/back). `partner` references another
/// seg **in this same lump**, so it is resolved in a second pass once the total
/// seg count is known: in range → `Some`, out of range → strict error / lenient
/// `None` + a [`MapWarning::DanglingReference`], mirroring `resolve_optional`.
///
/// Bounded and panic-safe: iterates fixed-size chunks (`chunks_exact`), so
/// memory use is `O(bytes.len())` with no capacity taken from an untrusted
/// count.
///
/// # Errors
///
/// Returns [`MapAssembleError::Records`] (fatal in **both** strictness modes,
/// matching [`decode_gl_vertices`]) when `bytes.len()` is not an exact multiple
/// of the version's record width (a partial trailing record). Propagates
/// [`MapAssembleError::DanglingReference`] from a vertex, linedef, or partner
/// reference that is out of range in strict mode.
///
/// Consumed by [`decode_gl_group`], the group orchestrator.
pub(crate) fn decode_gl_segs(
    bytes: &[u8],
    ver: GlVersion,
    normal_vert_count: usize,
    gl_vert_count: usize,
    linedef_count: usize,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<Vec<GlSeg>, MapAssembleError> {
    // VERIFIED gzdoom glnodes.cpp: V2 seg = 10 bytes, V3/V5 seg = 16 bytes.
    let record_size = match ver {
        GlVersion::V2 => 10,
        GlVersion::V3 | GlVersion::V5 => 16,
    };

    if !bytes.len().is_multiple_of(record_size) {
        return Err(MapAssembleError::Records {
            lump: "GL_SEGS",
            source: MapParseError::TrailingBytes {
                offset: (bytes.len() / record_size * record_size) as u64,
            },
        });
    }

    // Pass 1: decode endpoints, linedef, and side; capture each seg's raw
    // partner value (`None` = sentinel) for pass-2 resolution.
    let mut segs: Vec<GlSeg> = Vec::with_capacity(bytes.len() / record_size);
    let mut raw_partners: Vec<Option<u32>> = Vec::with_capacity(bytes.len() / record_size);

    for c in bytes.chunks_exact(record_size) {
        let (v1, v2, linedef, side, partner) = read_seg_record(c, ver);

        let start = split_vertex(
            v1,
            ver,
            normal_vert_count,
            gl_vert_count,
            strictness,
            warnings,
        )?;
        let end = split_vertex(
            v2,
            ver,
            normal_vert_count,
            gl_vert_count,
            strictness,
            warnings,
        )?;
        // VERIFIED gzdoom glnodes.cpp: linedef sentinel is 0xFFFF (GL miniseg).
        let linedef = if linedef == 0xFFFF {
            None
        } else {
            Some(LinedefIdx(resolve_required(
                i32::from(linedef),
                linedef_count,
                "linedef",
                "gl seg",
                strictness,
                warnings,
            )?))
        };

        segs.push(GlSeg {
            start,
            end,
            linedef,
            side,
            partner: None,
        });
        raw_partners.push(partner);
    }

    // Pass 2: resolve partner references against the now-known seg count using
    // optional semantics (mirroring `resolve_optional`): in range → `Some`,
    // out of range → strict error / lenient `None` + a dangling-reference warning.
    let seg_count = segs.len();
    for (seg, raw) in segs.iter_mut().zip(raw_partners) {
        let Some(raw) = raw else { continue };
        let index = i32::try_from(raw).unwrap_or(i32::MAX);
        if let Ok(u) = usize::try_from(index)
            && u < seg_count
        {
            seg.partner = Some(GlSegIdx(u));
            continue;
        }
        match strictness {
            Strictness::Strict => {
                return Err(MapAssembleError::DanglingReference {
                    referent: "gl seg",
                    index,
                    from: "gl seg",
                    count: seg_count,
                });
            }
            Strictness::Lenient => {
                warnings.push(MapWarning::DanglingReference {
                    referent: "gl seg",
                    index,
                    from: "gl seg",
                    count: seg_count,
                });
                seg.partner = None;
            }
        }
    }

    Ok(segs)
}

/// Decodes a `GL_SSECT` lump (V2, V3, or V5 layout) into [`GlSubsector`] runs.
///
/// Each subsector names a contiguous run of `GL_SEGS` as `first..first + count`.
/// Record widths and field types are version-dependent (all little-endian):
///
/// - **V2:** 4-byte records — `u16 count, u16 first`.
/// - **V3/V5:** 8-byte records (byte-identical, one decode path) — `i32 count,
///   i32 first` (gzdoom `gl3_mapsubsector_t`; values are non-negative).
///
/// The lump passed here holds **pure records** — V3's 4-byte `gNd3` header is
/// stripped by the caller before this function is reached (mirroring
/// [`decode_gl_segs`]); this decoder never sees a magic.
///
/// Each run is range-checked against `seg_count` using the same semantics as the
/// classic subsector normalizer (`normalize_bsp` in
/// [`assemble`](crate::map::assemble)): in range → `first..first + count`; out of
/// range → strict error / lenient clamp (`first.min(seg_count)..seg_count`) plus a
/// [`MapWarning::DanglingReference`].
///
/// Bounded and panic-safe: iterates fixed-size chunks (`chunks_exact`), so memory
/// use is `O(bytes.len())` with no capacity taken from an untrusted count.
///
/// # Errors
///
/// Returns [`MapAssembleError::Records`] (fatal in **both** strictness modes,
/// matching [`decode_gl_segs`]) when `bytes.len()` is not an exact multiple of the
/// version's record width (a partial trailing record). Returns
/// [`MapAssembleError::DanglingReference`] from an out-of-range seg run in strict
/// mode.
///
/// Consumed by [`decode_gl_group`], the group orchestrator.
pub(crate) fn decode_gl_subsectors(
    bytes: &[u8],
    ver: GlVersion,
    seg_count: usize,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<Vec<GlSubsector>, MapAssembleError> {
    // VERIFIED gzdoom glnodes.cpp: V2 subsector = 4 bytes (u16,u16),
    // V3/V5 subsector = 8 bytes (i32,i32).
    let record_size = match ver {
        GlVersion::V2 => 4,
        GlVersion::V3 | GlVersion::V5 => 8,
    };

    if !bytes.len().is_multiple_of(record_size) {
        return Err(MapAssembleError::Records {
            lump: "GL_SSECT",
            source: MapParseError::TrailingBytes {
                offset: (bytes.len() / record_size * record_size) as u64,
            },
        });
    }

    // `seg_count` never exceeds `isize::MAX`, so it fits `i64` losslessly; the
    // whole range check runs in `i64` so a malformed negative V3/V5 field is
    // treated as out of range rather than wrapping through `usize`.
    let seg_count_i = i64::try_from(seg_count).unwrap_or(i64::MAX);
    let mut subsectors = Vec::with_capacity(bytes.len() / record_size);
    for c in bytes.chunks_exact(record_size) {
        let (count, first): (i64, i64) = match ver {
            GlVersion::V2 => (
                i64::from(u16::from_le_bytes([c[0], c[1]])),
                i64::from(u16::from_le_bytes([c[2], c[3]])),
            ),
            GlVersion::V3 | GlVersion::V5 => (
                i64::from(i32::from_le_bytes([c[0], c[1], c[2], c[3]])),
                i64::from(i32::from_le_bytes([c[4], c[5], c[6], c[7]])),
            ),
        };
        let end = first + count; // first, count <= i32::MAX, so the sum fits i64.
        let range = if first >= 0 && count >= 0 && end <= seg_count_i {
            // Both bounds are non-negative and <= seg_count <= isize::MAX here.
            usize::try_from(first).unwrap_or(0)..usize::try_from(end).unwrap_or(0)
        } else {
            // Mirror the classic subsector normalizer's out-of-range handling.
            let index = i32::try_from(end).unwrap_or(i32::MAX);
            match strictness {
                Strictness::Strict => {
                    return Err(MapAssembleError::DanglingReference {
                        referent: "gl seg",
                        index,
                        from: "gl subsector",
                        count: seg_count,
                    });
                }
                Strictness::Lenient => {
                    warnings.push(MapWarning::DanglingReference {
                        referent: "gl seg",
                        index,
                        from: "gl subsector",
                        count: seg_count,
                    });
                    let clamped_first = usize::try_from(first.clamp(0, seg_count_i)).unwrap_or(0);
                    clamped_first..seg_count
                }
            }
        };
        subsectors.push(GlSubsector { segs: range });
    }

    Ok(subsectors)
}

/// Resolves one `GL_NODES` child word into a [`GlNodeChild`].
///
/// `flag`/`mask` carry the version's subsector-leaf convention: `flag` set →
/// a subsector leaf whose index is `raw & mask`; `flag` clear → an interior node
/// whose index is the whole `raw`. Both branches share [`resolve_required`]'s
/// range-check discipline, mirroring `resolve_node_child` /
/// `resolve_deepbsp_child` in [`assemble`](crate::map::assemble) /
/// [`deepbsp`](crate::map::deepbsp).
fn resolve_gl_node_child(
    raw: u32,
    flag: u32,
    mask: u32,
    node_count: usize,
    subsector_count: usize,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<GlNodeChild, MapAssembleError> {
    if raw & flag == 0 {
        // Node index: `raw` has the flag bit clear, so it fits `i32` losslessly
        // for every width used here (u16 always; u32 <= 0x7FFF_FFFF).
        let index = i32::try_from(raw).unwrap_or(i32::MAX);
        Ok(GlNodeChild::Node(GlNodeIdx(resolve_required(
            index, node_count, "gl node", "gl node", strictness, warnings,
        )?)))
    } else {
        let index = i32::try_from(raw & mask).unwrap_or(i32::MAX);
        Ok(GlNodeChild::Subsector(GlSubsectorIdx(resolve_required(
            index,
            subsector_count,
            "gl subsector",
            "gl node",
            strictness,
            warnings,
        )?)))
    }
}

/// Assembles one [`GlNode`] from decoded partition/bbox fields and raw child
/// words, resolving both children against the node/subsector counts.
///
/// Field, bbox (`[top, bottom, left, right]`), and child (right/front before
/// left/back) ordering mirror the classic `common::Node` → `MapNode` mapping in
/// `normalize_bsp`. `flag`/`mask` are the version's child convention.
#[allow(clippy::too_many_arguments)]
fn build_gl_node(
    x: i16,
    y: i16,
    dx: i16,
    dy: i16,
    right_bbox: [i16; 4],
    left_bbox: [i16; 4],
    right_raw: u32,
    left_raw: u32,
    flag: u32,
    mask: u32,
    node_count: usize,
    subsector_count: usize,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<GlNode, MapAssembleError> {
    let right = resolve_gl_node_child(
        right_raw,
        flag,
        mask,
        node_count,
        subsector_count,
        strictness,
        warnings,
    )?;
    let left = resolve_gl_node_child(
        left_raw,
        flag,
        mask,
        node_count,
        subsector_count,
        strictness,
        warnings,
    )?;
    Ok(GlNode {
        x: i32::from(x),
        y: i32::from(y),
        dx: i32::from(dx),
        dy: i32::from(dy),
        right_bbox: right_bbox.map(i32::from),
        left_bbox: left_bbox.map(i32::from),
        right,
        left,
    })
}

/// Decodes a `GL_NODES` lump (V2, V3, or V5 layout) into [`GlNode`] records.
///
/// The partition line (`x`, `y`, `dx`, `dy`), the two child bounding boxes
/// (`[top, bottom, left, right]`), and the right-then-left child order all mirror
/// [`MapNode`](crate::map::graph::MapNode). Record widths and the child-index
/// convention are version-dependent (all little-endian):
///
/// - **V2/V3:** 28-byte records, **byte-identical to the classic Doom `NODES`
///   record** — decoded by reusing [`parse_records`] over
///   [`common::Node`](crate::map::common::Node). Children are `u16`; bit `0x8000`
///   set selects a subsector leaf (index `child & 0x7FFF`).
/// - **V5:** 32-byte records (gzdoom `gl5_mapnode_t`) — `i16 x,y,dx,dy`, two
///   `i16[4]` bboxes, then `u32 right_child, u32 left_child`; bit `0x8000_0000`
///   set selects a subsector leaf (index `child & 0x7FFF_FFFF`).
///
/// Children are resolved against the self-count of node records in this lump and
/// the supplied `subsector_count`, using the same [`resolve_required`] discipline
/// as `resolve_node_child`: in range → the typed child; out of range → strict
/// error / lenient clamp-to-0 plus a [`MapWarning::DanglingReference`].
///
/// Bounded and panic-safe: V2/V3 goes through [`parse_records`] and V5 iterates
/// fixed-size chunks (`chunks_exact`), so memory use is `O(bytes.len())` with no
/// capacity taken from an untrusted count.
///
/// # Errors
///
/// Returns [`MapAssembleError::Records`] (fatal in **both** strictness modes,
/// matching [`decode_gl_segs`]) when `bytes.len()` is not an exact multiple of the
/// version's record width (a partial trailing record). Returns
/// [`MapAssembleError::DanglingReference`] from an out-of-range child in strict
/// mode.
///
/// Consumed by [`decode_gl_group`], the group orchestrator.
pub(crate) fn decode_gl_nodes(
    bytes: &[u8],
    ver: GlVersion,
    subsector_count: usize,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<Vec<GlNode>, MapAssembleError> {
    match ver {
        // V2/V3 GL_NODES is byte-identical to the classic 28-byte NODES record,
        // so reuse `common::Node` rather than re-deriving the layout.
        GlVersion::V2 | GlVersion::V3 => {
            // VERIFIED gzdoom glnodes.cpp: V2/V3 child flag NF_SUBSECTOR = 0x8000,
            // index mask = 0x7FFF.
            const FLAG_16: u32 = 0x8000;
            const MASK_16: u32 = 0x7FFF;
            let raw: Vec<ClassicNode> =
                parse_records(bytes).map_err(|source| MapAssembleError::Records {
                    lump: "GL_NODES",
                    source,
                })?;
            let node_count = raw.len();
            let mut nodes = Vec::with_capacity(node_count);
            for nd in raw {
                nodes.push(build_gl_node(
                    nd.x,
                    nd.y,
                    nd.dx,
                    nd.dy,
                    nd.right_bbox,
                    nd.left_bbox,
                    u32::from(nd.right_child),
                    u32::from(nd.left_child),
                    FLAG_16,
                    MASK_16,
                    node_count,
                    subsector_count,
                    strictness,
                    warnings,
                )?);
            }
            Ok(nodes)
        }
        GlVersion::V5 => {
            // VERIFIED gzdoom glnodes.cpp gl5_mapnode_t: 32-byte record.
            const RECORD_SIZE: usize = 32;
            // VERIFIED gzdoom glnodes.cpp: V5 child flag GL5_NF_SUBSECTOR =
            // 0x8000_0000, index mask = 0x7FFF_FFFF.
            const FLAG_32: u32 = 0x8000_0000;
            const MASK_32: u32 = 0x7FFF_FFFF;
            if !bytes.len().is_multiple_of(RECORD_SIZE) {
                return Err(MapAssembleError::Records {
                    lump: "GL_NODES",
                    source: MapParseError::TrailingBytes {
                        offset: (bytes.len() / RECORD_SIZE * RECORD_SIZE) as u64,
                    },
                });
            }
            let node_count = bytes.len() / RECORD_SIZE;
            let rd_i16 = |c: &[u8], off: usize| i16::from_le_bytes([c[off], c[off + 1]]);
            let mut nodes = Vec::with_capacity(node_count);
            for c in bytes.chunks_exact(RECORD_SIZE) {
                let x = rd_i16(c, 0);
                let y = rd_i16(c, 2);
                let dx = rd_i16(c, 4);
                let dy = rd_i16(c, 6);
                let right_bbox = [rd_i16(c, 8), rd_i16(c, 10), rd_i16(c, 12), rd_i16(c, 14)];
                let left_bbox = [rd_i16(c, 16), rd_i16(c, 18), rd_i16(c, 20), rd_i16(c, 22)];
                let right_raw = u32::from_le_bytes([c[24], c[25], c[26], c[27]]);
                let left_raw = u32::from_le_bytes([c[28], c[29], c[30], c[31]]);
                nodes.push(build_gl_node(
                    x,
                    y,
                    dx,
                    dy,
                    right_bbox,
                    left_bbox,
                    right_raw,
                    left_raw,
                    FLAG_32,
                    MASK_32,
                    node_count,
                    subsector_count,
                    strictness,
                    warnings,
                )?);
            }
            Ok(nodes)
        }
    }
}

/// The four decoded classic GL node arenas for one map, produced by
/// [`decode_gl_group`].
///
/// Each field is the fully decoded, cross-referenced form of one `GL_*` lump.
/// A refused version or a lenient degrade yields the empty (`default`) value,
/// so callers observe "no GL data" identically to a group that was never
/// present.
///
/// Stored on the assembled [`Map`](crate::map::graph::Map) by
/// [`assemble_with_options`](crate::map::graph::Map::assemble_with_options),
/// which decodes the group via [`decode_gl_group`] (#324).
#[derive(Debug, Default)]
pub(crate) struct DecodedGl {
    /// Decoded `GL_VERT` vertices (extra glBSP vertices in world units).
    pub vertices: Vec<GlVertex>,
    /// Decoded `GL_SEGS` records (minisegs plus partner links).
    pub segs: Vec<GlSeg>,
    /// Decoded `GL_SSECT` subsectors (contiguous seg runs).
    pub subsectors: Vec<GlSubsector>,
    /// Decoded `GL_NODES` BSP nodes.
    pub nodes: Vec<GlNode>,
}

/// Decodes the four GL data lumps into their record-form fallible steps.
///
/// Split out from [`decode_gl_group`] so the whole sequence is a single
/// fallible unit the orchestrator can match on for the lenient degrade — a
/// structural fault in any step must roll the whole group back, which a chain
/// of `?` inside the public function could not express.
#[allow(clippy::too_many_arguments)]
fn decode_gl_group_records(
    vert: &[u8],
    segs_rec: &[u8],
    ssect_rec: &[u8],
    nodes: &[u8],
    ver: GlVersion,
    normal_vert_count: usize,
    linedef_count: usize,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<DecodedGl, MapAssembleError> {
    let vertices = decode_gl_vertices(vert)?;
    let segs = decode_gl_segs(
        segs_rec,
        ver,
        normal_vert_count,
        vertices.len(),
        linedef_count,
        strictness,
        warnings,
    )?;
    let subsectors = decode_gl_subsectors(ssect_rec, ver, segs.len(), strictness, warnings)?;
    let decoded_nodes = decode_gl_nodes(nodes, ver, subsectors.len(), strictness, warnings)?;
    Ok(DecodedGl {
        vertices,
        segs,
        subsectors,
        nodes: decoded_nodes,
    })
}

/// Orchestrates decoding one complete classic GL node group (`GL_VERT`,
/// `GL_SEGS`, `GL_SSECT`, `GL_NODES`) into the [`DecodedGl`] arenas.
///
/// Ties together [`detect_gl_version`] and the four lump decoders, wiring each
/// stage's output count into the next (GL-vertex count into segs, seg count into
/// subsectors, subsector count into nodes) and resolving partner segs against the
/// final seg count. The caller supplies the map's `normal_vert_count` (the
/// classic `VERTEXES` arena) and `linedef_count` for cross-lump reference checks.
///
/// # Strictness posture
///
/// Two distinct fault classes are handled here; both mirror the classic-BSP and
/// `DeePBSP` decoders ([`assemble`](crate::map::assemble) /
/// [`deepbsp`](crate::map::deepbsp)):
///
/// - **Refused version (V1/V4).** [`detect_gl_version`] rejects the group. In
///   **Strict** this is a hard [`MapAssembleError::UnsupportedGlNodeVersion`]
///   carrying the first four `GL_VERT` bytes as `magic` (zero-padded when the
///   lump is shorter than four bytes). In **Lenient** it is recovered: a single
///   [`MapWarning::GlNodesRefused`] is pushed and empty arenas are returned, so
///   the caller sees "no GL data" exactly as for an absent group. This
///   Strict/Lenient split is decided here.
/// - **Structural / cross-reference fault** (a dangling vertex, seg, subsector,
///   linedef, or child reference). In **Strict** the first such
///   [`MapAssembleError::DanglingReference`] propagates. In **Lenient**, when a
///   reference cannot be recovered by clamping (a reference into an *empty*
///   arena), the whole group degrades: `warnings` is rolled back to the
///   watermark captured on entry, a single [`MapWarning::GlNodesDegraded`] is
///   pushed, and empty arenas are returned — the same whole-BSP degrade posture
///   as `normalize_bsp_or_degrade`, so a partially broken GL BSP does not
///   surface a pile of per-element diagnostics.
///
/// **Framing defects stay hard errors in both modes**: a bad lump length, or a
/// V3 `GL_SEGS`/`GL_SSECT` lump too short to hold its `gNd3` header, returns
/// [`MapAssembleError::Records`] regardless of strictness (matching the
/// framing-defect posture of every GL lump decoder).
///
/// # V3 header stripping
///
/// The lump decoders are header-agnostic (pure records). For **V3 only**,
/// `GL_SEGS` and `GL_SSECT` carry a 4-byte `gNd3` version marker, which is
/// stripped here before decoding; `GL_VERT` strips its own magic and `GL_NODES`
/// never carries one.
///
/// # Errors
///
/// Returns [`MapAssembleError::UnsupportedGlNodeVersion`] (Strict refusal),
/// [`MapAssembleError::Records`] (framing defect, both modes), or
/// [`MapAssembleError::DanglingReference`] (Strict structural fault). See the
/// strictness posture above.
///
/// Called by [`assemble_with_options`](crate::map::graph::Map::assemble_with_options)
/// on the binary-map path, which stores the resulting [`DecodedGl`] arenas on
/// the assembled map (#324).
#[allow(clippy::too_many_arguments)]
pub(crate) fn decode_gl_group(
    vert: &[u8],
    segs: &[u8],
    ssect: &[u8],
    nodes: &[u8],
    normal_vert_count: usize,
    linedef_count: usize,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<DecodedGl, MapAssembleError> {
    // Step 1: detect the version, deciding the Strict/Lenient refusal split here.
    let ver = match detect_gl_version(vert, segs) {
        Ok(ver) => ver,
        Err(version) => {
            return match strictness {
                Strictness::Strict => {
                    // First four GL_VERT bytes, zero-padded when the lump is short.
                    let mut magic = [0u8; 4];
                    let n = vert.len().min(4);
                    magic[..n].copy_from_slice(&vert[..n]);
                    Err(MapAssembleError::UnsupportedGlNodeVersion { magic })
                }
                Strictness::Lenient => {
                    warnings.push(MapWarning::GlNodesRefused { version });
                    Ok(DecodedGl::default())
                }
            };
        }
    };

    // Step 2: strip the V3-only `gNd3` header from GL_SEGS/GL_SSECT before the
    // header-agnostic decoders see them. A lump too short to hold its header is a
    // framing defect -> a hard error in both modes (propagated by `?`).
    let (segs_rec, ssect_rec) = if ver == GlVersion::V3 {
        // Defensively unreachable: `detect_gl_version` only returns `V3` when
        // `segs[0..4] == b"gNd3"`, which already requires `segs.len() >= 4` — so
        // this `None` arm can never fire for a properly-detected V3 group. Kept
        // (and the `?` kept) as defense in depth rather than an `unwrap`; there is
        // no dedicated test for this arm because it is unreachable, unlike the
        // `ssect` arm below, which `detect_gl_version` never inspects and so can
        // genuinely be too short (see `v3_group_short_ssect_lump_is_framing_error`).
        let segs_rec = segs.get(4..).ok_or(MapAssembleError::Records {
            lump: "GL_SEGS",
            source: MapParseError::TrailingBytes { offset: 0 },
        })?;
        let ssect_rec = ssect.get(4..).ok_or(MapAssembleError::Records {
            lump: "GL_SSECT",
            source: MapParseError::TrailingBytes { offset: 0 },
        })?;
        (segs_rec, ssect_rec)
    } else {
        (segs, ssect)
    };

    // Steps 3-4: decode in dependency order, degrading the whole group in Lenient
    // on an unrecoverable structural fault (mirrors `normalize_bsp_or_degrade` /
    // the DeePBSP inline degrade). Framing errors fall through as hard errors.
    let watermark = warnings.len();
    match decode_gl_group_records(
        vert,
        segs_rec,
        ssect_rec,
        nodes,
        ver,
        normal_vert_count,
        linedef_count,
        strictness,
        warnings,
    ) {
        Err(MapAssembleError::DanglingReference { .. }) if strictness == Strictness::Lenient => {
            warnings.truncate(watermark);
            warnings.push(MapWarning::GlNodesDegraded);
            Ok(DecodedGl::default())
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        GlVersion, decode_gl_group, decode_gl_nodes, decode_gl_segs, decode_gl_subsectors,
        decode_gl_vertices, detect_gl_version,
    };
    use crate::Strictness;
    use crate::map::MapParseError;
    use crate::map::assemble::MapAssembleError;
    use crate::map::graph::{
        GlNodeChild, GlNodeIdx, GlSegIdx, GlSubsectorIdx, GlVertexIdx, GlVertexRef, LinedefIdx,
        MapWarning, VertexIdx,
    };

    #[test]
    fn detects_v5() {
        assert_eq!(detect_gl_version(b"gNd5....", b"").unwrap(), GlVersion::V5);
    }

    #[test]
    fn detects_v3_from_gnd3_on_segs() {
        assert_eq!(
            detect_gl_version(b"gNd2....", b"gNd3....").unwrap(),
            GlVersion::V3
        );
    }

    #[test]
    fn detects_v2_when_segs_lack_gnd3() {
        assert_eq!(
            detect_gl_version(b"gNd2....", b"....").unwrap(),
            GlVersion::V2
        );
    }

    #[test]
    fn refuses_v4() {
        assert_eq!(detect_gl_version(b"gNd4....", b""), Err(4));
    }

    #[test]
    fn refuses_v1_on_unknown_magic() {
        assert_eq!(detect_gl_version(b"\0\0\0\0..", b""), Err(1));
    }

    #[test]
    fn refuses_v1_on_short_gl_vert_without_panicking() {
        assert_eq!(detect_gl_version(b"gN", b""), Err(1));
    }

    #[test]
    fn decodes_gl_vert_fixed_point() {
        let mut b = b"gNd2".to_vec();
        b.extend_from_slice(&98_304i32.to_le_bytes());
        b.extend_from_slice(&(-131_072i32).to_le_bytes());
        let v = decode_gl_vertices(&b).unwrap();
        assert_eq!(v.len(), 1);
        assert!((v[0].x - 1.5).abs() < 1e-9);
        assert!((v[0].y + 2.0).abs() < 1e-9);
    }

    #[test]
    fn rejects_bad_length_after_magic() {
        // Magic plus 7 trailing bytes: not a whole multiple of 8.
        let mut b = b"gNd2".to_vec();
        b.extend_from_slice(&[0u8; 7]);
        let err = decode_gl_vertices(&b).unwrap_err();
        assert!(matches!(
            err,
            MapAssembleError::Records {
                lump: "GL_VERT",
                source: MapParseError::TrailingBytes { offset: 4 },
            }
        ));
    }

    #[test]
    fn rejects_too_short_for_magic_without_panicking() {
        let err = decode_gl_vertices(b"gN").unwrap_err();
        assert!(matches!(
            err,
            MapAssembleError::Records {
                lump: "GL_VERT",
                source: MapParseError::TrailingBytes { offset: 0 },
            }
        ));
    }

    #[test]
    fn decodes_empty_vertex_run() {
        let v = decode_gl_vertices(b"gNd2").unwrap();
        assert_eq!(v.len(), 0);
    }

    // ---- GL_SEGS ----

    /// Builds a single 10-byte V2 `GL_SEGS` record.
    fn v2_seg(v1: u16, v2: u16, linedef: u16, side: u16, partner: u16) -> Vec<u8> {
        let mut b = Vec::with_capacity(10);
        b.extend_from_slice(&v1.to_le_bytes());
        b.extend_from_slice(&v2.to_le_bytes());
        b.extend_from_slice(&linedef.to_le_bytes());
        b.extend_from_slice(&side.to_le_bytes());
        b.extend_from_slice(&partner.to_le_bytes());
        b
    }

    /// Builds a single 16-byte V3/V5 `GL_SEGS` record.
    fn v5_seg(v1: u32, v2: u32, linedef: u16, side: u16, partner: u32) -> Vec<u8> {
        let mut b = Vec::with_capacity(16);
        b.extend_from_slice(&v1.to_le_bytes());
        b.extend_from_slice(&v2.to_le_bytes());
        b.extend_from_slice(&linedef.to_le_bytes());
        b.extend_from_slice(&side.to_le_bytes());
        b.extend_from_slice(&partner.to_le_bytes());
        b
    }

    #[test]
    fn v2_seg_normal_and_gl_vertex_split() {
        // start = normal vtx 2 (0x0002), end = GL vtx 0 (0x8000),
        // linedef 0xFFFF (miniseg), side 0, partner 0xFFFF (one-sided).
        let bytes = v2_seg(0x0002, 0x8000, 0xFFFF, 0x0000, 0xFFFF);
        let mut w = Vec::new();
        let segs =
            decode_gl_segs(&bytes, GlVersion::V2, 3, 1, 5, Strictness::Strict, &mut w).unwrap();
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].start, GlVertexRef::Normal(VertexIdx(2)));
        assert_eq!(segs[0].end, GlVertexRef::Gl(GlVertexIdx(0)));
        assert_eq!(segs[0].linedef, None);
        assert_eq!(segs[0].side, 0);
        assert_eq!(segs[0].partner, None);
        assert!(w.is_empty());
    }

    #[test]
    fn v5_seg_top_two_bit_flag_and_normal() {
        // seg0: start GL flag via top bit (0xC000_0005 -> Gl 5), end plain (3 -> Normal 3),
        //       linedef 0 -> Some(0), side 1, partner 0xFFFF_FFFF -> None.
        // seg1: start GL flag via bit30 only (0x4000_0000 -> Gl 0), end plain (1 -> Normal 1),
        //       linedef 0xFFFF -> None, side 0, partner 0xFFFF_FFFF -> None.
        let mut bytes = v5_seg(0xC000_0005, 0x0000_0003, 0x0000, 0x0001, 0xFFFF_FFFF);
        bytes.extend(v5_seg(
            0x4000_0000,
            0x0000_0001,
            0xFFFF,
            0x0000,
            0xFFFF_FFFF,
        ));
        let mut w = Vec::new();
        let segs =
            decode_gl_segs(&bytes, GlVersion::V5, 4, 6, 1, Strictness::Strict, &mut w).unwrap();
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].start, GlVertexRef::Gl(GlVertexIdx(5)));
        assert_eq!(segs[0].end, GlVertexRef::Normal(VertexIdx(3)));
        assert_eq!(segs[0].linedef, Some(LinedefIdx(0)));
        assert_eq!(segs[0].side, 1);
        assert_eq!(segs[0].partner, None);
        assert_eq!(segs[1].start, GlVertexRef::Gl(GlVertexIdx(0)));
        assert_eq!(segs[1].end, GlVertexRef::Normal(VertexIdx(1)));
        assert_eq!(segs[1].linedef, None);
        assert!(w.is_empty());
    }

    #[test]
    fn v2_in_range_partner_resolves() {
        // seg0 partners seg1 (in range, count == 2); seg1 is one-sided.
        let mut bytes = v2_seg(0x0000, 0x0001, 0x0000, 0x0000, 0x0001);
        bytes.extend(v2_seg(0x0001, 0x0000, 0x0000, 0x0001, 0xFFFF));
        let mut w = Vec::new();
        let segs =
            decode_gl_segs(&bytes, GlVersion::V2, 2, 0, 1, Strictness::Strict, &mut w).unwrap();
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].partner, Some(GlSegIdx(1)));
        assert_eq!(segs[1].partner, None);
        assert!(w.is_empty());
    }

    #[test]
    fn v2_out_of_range_partner_strict_errors_lenient_warns() {
        // Single seg whose partner (1) points past the only seg (count == 1).
        let bytes = v2_seg(0x0000, 0x0001, 0x0000, 0x0000, 0x0001);
        let mut w = Vec::new();
        let err =
            decode_gl_segs(&bytes, GlVersion::V2, 2, 0, 1, Strictness::Strict, &mut w).unwrap_err();
        assert!(matches!(
            err,
            MapAssembleError::DanglingReference {
                referent: "gl seg",
                ..
            }
        ));

        let mut w = Vec::new();
        let segs =
            decode_gl_segs(&bytes, GlVersion::V2, 2, 0, 1, Strictness::Lenient, &mut w).unwrap();
        assert_eq!(segs[0].partner, None);
        assert!(matches!(
            w.as_slice(),
            [MapWarning::DanglingReference {
                referent: "gl seg",
                ..
            }]
        ));
    }

    #[test]
    fn bad_length_lump_is_framing_error() {
        // 15 bytes: not a whole multiple of the 10-byte V2 record.
        let bytes = vec![0u8; 15];
        let err = decode_gl_segs(
            &bytes,
            GlVersion::V2,
            4,
            4,
            4,
            Strictness::Strict,
            &mut Vec::new(),
        )
        .unwrap_err();
        assert!(matches!(
            err,
            MapAssembleError::Records {
                lump: "GL_SEGS",
                source: MapParseError::TrailingBytes { offset: 10 },
            }
        ));
        // Lenient mode rejects the same framing defect.
        let err = decode_gl_segs(
            &bytes,
            GlVersion::V2,
            4,
            4,
            4,
            Strictness::Lenient,
            &mut Vec::new(),
        )
        .unwrap_err();
        assert!(matches!(
            err,
            MapAssembleError::Records {
                lump: "GL_SEGS",
                source: MapParseError::TrailingBytes { offset: 10 },
            }
        ));
    }

    #[test]
    fn dangling_vertex_strict_errors_lenient_clamps() {
        // start references normal vtx 9, but only 1 normal vertex exists.
        let bytes = v2_seg(0x0009, 0x0000, 0xFFFF, 0x0000, 0xFFFF);
        let err = decode_gl_segs(
            &bytes,
            GlVersion::V2,
            1,
            1,
            1,
            Strictness::Strict,
            &mut Vec::new(),
        )
        .unwrap_err();
        assert!(matches!(
            err,
            MapAssembleError::DanglingReference {
                referent: "vertex",
                ..
            }
        ));

        let mut w = Vec::new();
        let segs =
            decode_gl_segs(&bytes, GlVersion::V2, 1, 1, 1, Strictness::Lenient, &mut w).unwrap();
        assert_eq!(segs[0].start, GlVertexRef::Normal(VertexIdx(0)));
        assert!(matches!(
            w.as_slice(),
            [MapWarning::DanglingReference {
                referent: "vertex",
                ..
            }]
        ));
    }

    // ---- GL_SSECT ----

    /// Builds a single 4-byte V2 `GL_SSECT` record (`u16 count, u16 first`).
    fn v2_ssect(count: u16, first: u16) -> Vec<u8> {
        let mut b = Vec::with_capacity(4);
        b.extend_from_slice(&count.to_le_bytes());
        b.extend_from_slice(&first.to_le_bytes());
        b
    }

    /// Builds a single 8-byte V3/V5 `GL_SSECT` record (`i32 count, i32 first`).
    fn v3_ssect(count: i32, first: i32) -> Vec<u8> {
        let mut b = Vec::with_capacity(8);
        b.extend_from_slice(&count.to_le_bytes());
        b.extend_from_slice(&first.to_le_bytes());
        b
    }

    #[test]
    fn v2_ssect_run_resolves() {
        let bytes = v2_ssect(3, 0);
        let mut w = Vec::new();
        let ss =
            decode_gl_subsectors(&bytes, GlVersion::V2, 3, Strictness::Strict, &mut w).unwrap();
        assert_eq!(ss.len(), 1);
        assert_eq!(ss[0].segs, 0..3);
        assert!(w.is_empty());
    }

    #[test]
    fn v3_and_v5_ssect_are_byte_identical() {
        // 8-byte record; first=2, count=4 -> 2..6 against seg_count 6.
        let bytes = v3_ssect(4, 2);
        for ver in [GlVersion::V3, GlVersion::V5] {
            let mut w = Vec::new();
            let ss = decode_gl_subsectors(&bytes, ver, 6, Strictness::Strict, &mut w).unwrap();
            assert_eq!(ss.len(), 1);
            assert_eq!(ss[0].segs, 2..6);
            assert!(w.is_empty());
        }
    }

    #[test]
    fn ssect_bad_length_is_framing_error() {
        // 6 bytes: not a whole multiple of the 4-byte V2 record.
        let bytes = vec![0u8; 6];
        for strictness in [Strictness::Strict, Strictness::Lenient] {
            let err = decode_gl_subsectors(&bytes, GlVersion::V2, 4, strictness, &mut Vec::new())
                .unwrap_err();
            assert!(matches!(
                err,
                MapAssembleError::Records {
                    lump: "GL_SSECT",
                    source: MapParseError::TrailingBytes { offset: 4 },
                }
            ));
        }
    }

    #[test]
    fn ssect_out_of_range_run_strict_errors_lenient_clamps() {
        // count 5 from first 0 overruns the 2 available segs.
        let bytes = v2_ssect(5, 0);
        let err = decode_gl_subsectors(
            &bytes,
            GlVersion::V2,
            2,
            Strictness::Strict,
            &mut Vec::new(),
        )
        .unwrap_err();
        assert!(matches!(
            err,
            MapAssembleError::DanglingReference {
                referent: "gl seg",
                from: "gl subsector",
                ..
            }
        ));

        let mut w = Vec::new();
        let ss =
            decode_gl_subsectors(&bytes, GlVersion::V2, 2, Strictness::Lenient, &mut w).unwrap();
        // Lenient clamp mirrors the classic normalizer: first.min(seg_count)..seg_count.
        assert_eq!(ss[0].segs, 0..2);
        assert!(matches!(
            w.as_slice(),
            [MapWarning::DanglingReference {
                referent: "gl seg",
                from: "gl subsector",
                ..
            }]
        ));
    }

    // ---- GL_NODES ----

    /// Builds a single 28-byte V2/V3 `GL_NODES` record (byte-identical to the
    /// classic Doom `NODES` record; `u16` children).
    #[allow(clippy::too_many_arguments)]
    fn v2_node(
        x: i16,
        y: i16,
        dx: i16,
        dy: i16,
        right_bbox: [i16; 4],
        left_bbox: [i16; 4],
        right_child: u16,
        left_child: u16,
    ) -> Vec<u8> {
        let mut b = Vec::with_capacity(28);
        for v in [x, y, dx, dy] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        for v in right_bbox.into_iter().chain(left_bbox) {
            b.extend_from_slice(&v.to_le_bytes());
        }
        b.extend_from_slice(&right_child.to_le_bytes());
        b.extend_from_slice(&left_child.to_le_bytes());
        b
    }

    /// Builds a single 32-byte V5 `GL_NODES` record (`u32` children).
    #[allow(clippy::too_many_arguments)]
    fn v5_node(
        x: i16,
        y: i16,
        dx: i16,
        dy: i16,
        right_bbox: [i16; 4],
        left_bbox: [i16; 4],
        right_child: u32,
        left_child: u32,
    ) -> Vec<u8> {
        let mut b = Vec::with_capacity(32);
        for v in [x, y, dx, dy] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        for v in right_bbox.into_iter().chain(left_bbox) {
            b.extend_from_slice(&v.to_le_bytes());
        }
        b.extend_from_slice(&right_child.to_le_bytes());
        b.extend_from_slice(&left_child.to_le_bytes());
        b
    }

    #[test]
    fn v5_node_children_and_fields() {
        // node0 under test; nodes 1 and 2 are filler pointing at subsector 0.
        let mut bytes = v5_node(
            1,
            -2,
            3,
            -4,
            [10, -20, -30, 40],
            [50, 60, 70, 80],
            0x8000_0001, // VERIFIED leaf: bit31 set -> Subsector(1)
            0x0000_0002, // interior: bit31 clear -> Node(2)
        );
        bytes.extend(v5_node(
            0,
            0,
            0,
            0,
            [0; 4],
            [0; 4],
            0x8000_0000,
            0x8000_0000,
        ));
        bytes.extend(v5_node(
            0,
            0,
            0,
            0,
            [0; 4],
            [0; 4],
            0x8000_0000,
            0x8000_0000,
        ));
        let mut w = Vec::new();
        // subsector_count 2, node self-count 3.
        let nodes = decode_gl_nodes(&bytes, GlVersion::V5, 2, Strictness::Strict, &mut w).unwrap();
        assert_eq!(nodes.len(), 3);
        assert_eq!(nodes[0].x, 1);
        assert_eq!(nodes[0].y, -2);
        assert_eq!(nodes[0].dx, 3);
        assert_eq!(nodes[0].dy, -4);
        assert_eq!(nodes[0].right_bbox, [10, -20, -30, 40]);
        assert_eq!(nodes[0].left_bbox, [50, 60, 70, 80]);
        assert_eq!(nodes[0].right, GlNodeChild::Subsector(GlSubsectorIdx(1)));
        assert_eq!(nodes[0].left, GlNodeChild::Node(GlNodeIdx(2)));
        assert!(w.is_empty());
    }

    #[test]
    fn v2_and_v3_node_children_and_fields() {
        // right leaf (0x8000 -> Subsector 0), left interior (0x0000 -> Node 0).
        let bytes = v2_node(
            5,
            6,
            -7,
            8,
            [100, -100, -50, 50],
            [1, 2, 3, 4],
            0x8000, // VERIFIED leaf: bit15 set -> Subsector(0)
            0x0000, // interior: bit15 clear -> Node(0)
        );
        for ver in [GlVersion::V2, GlVersion::V3] {
            let mut w = Vec::new();
            let nodes = decode_gl_nodes(&bytes, ver, 1, Strictness::Strict, &mut w).unwrap();
            assert_eq!(nodes.len(), 1);
            assert_eq!(nodes[0].x, 5);
            assert_eq!(nodes[0].y, 6);
            assert_eq!(nodes[0].dx, -7);
            assert_eq!(nodes[0].dy, 8);
            assert_eq!(nodes[0].right_bbox, [100, -100, -50, 50]);
            assert_eq!(nodes[0].left_bbox, [1, 2, 3, 4]);
            assert_eq!(nodes[0].right, GlNodeChild::Subsector(GlSubsectorIdx(0)));
            assert_eq!(nodes[0].left, GlNodeChild::Node(GlNodeIdx(0)));
            assert!(w.is_empty());
        }
    }

    #[test]
    fn node_bad_length_is_framing_error() {
        // 15 bytes: not a whole multiple of the 28-byte V2 record.
        let bytes = vec![0u8; 15];
        for strictness in [Strictness::Strict, Strictness::Lenient] {
            let err =
                decode_gl_nodes(&bytes, GlVersion::V2, 1, strictness, &mut Vec::new()).unwrap_err();
            assert!(matches!(
                err,
                MapAssembleError::Records {
                    lump: "GL_NODES",
                    source: MapParseError::TrailingBytes { offset: 0 },
                }
            ));
        }
        // V5's 32-byte record: 40 bytes leaves a 8-byte partial tail.
        let bytes = vec![0u8; 40];
        let err = decode_gl_nodes(
            &bytes,
            GlVersion::V5,
            1,
            Strictness::Strict,
            &mut Vec::new(),
        )
        .unwrap_err();
        assert!(matches!(
            err,
            MapAssembleError::Records {
                lump: "GL_NODES",
                source: MapParseError::TrailingBytes { offset: 32 },
            }
        ));
    }

    #[test]
    fn node_dangling_child_strict_errors_lenient_clamps() {
        // left_child Node(5) overruns the single node in the lump.
        let bytes = v2_node(0, 0, 0, 0, [0; 4], [0; 4], 0x8000, 0x0005);
        let err = decode_gl_nodes(
            &bytes,
            GlVersion::V2,
            1,
            Strictness::Strict,
            &mut Vec::new(),
        )
        .unwrap_err();
        assert!(matches!(
            err,
            MapAssembleError::DanglingReference {
                referent: "gl node",
                from: "gl node",
                ..
            }
        ));

        let mut w = Vec::new();
        let nodes = decode_gl_nodes(&bytes, GlVersion::V2, 1, Strictness::Lenient, &mut w).unwrap();
        // Lenient resolve_required clamps to index 0.
        assert_eq!(nodes[0].left, GlNodeChild::Node(GlNodeIdx(0)));
        assert!(matches!(
            w.as_slice(),
            [MapWarning::DanglingReference {
                referent: "gl node",
                from: "gl node",
                ..
            }]
        ));
    }

    // ---- decode_gl_group orchestration ----

    #[test]
    fn v2_group_orchestrates_full_decode() {
        // GL_VERT: gNd2 + 1 GL vertex.
        let mut vert = b"gNd2".to_vec();
        vert.extend_from_slice(&0i32.to_le_bytes());
        vert.extend_from_slice(&0i32.to_le_bytes());
        // GL_SEGS: two segs (normal 0 <-> GL 0) that partner each other.
        let mut segs = v2_seg(0x0000, 0x8000, 0x0000, 0x0000, 0x0001);
        segs.extend(v2_seg(0x8000, 0x0000, 0x0000, 0x0001, 0x0000));
        // GL_SSECT: one run covering both segs.
        let ssect = v2_ssect(2, 0);
        // GL_NODES: one node whose children both name the single subsector.
        let nodes = v2_node(
            1,
            2,
            3,
            4,
            [10, 20, 30, 40],
            [50, 60, 70, 80],
            0x8000,
            0x8000,
        );

        let mut w = Vec::new();
        let gl = decode_gl_group(
            &vert,
            &segs,
            &ssect,
            &nodes,
            1,
            1,
            Strictness::Strict,
            &mut w,
        )
        .unwrap();
        assert_eq!(gl.vertices.len(), 1);
        assert_eq!(gl.segs.len(), 2);
        assert_eq!(gl.segs[0].start, GlVertexRef::Normal(VertexIdx(0)));
        assert_eq!(gl.segs[0].end, GlVertexRef::Gl(GlVertexIdx(0)));
        assert_eq!(gl.segs[0].partner, Some(GlSegIdx(1)));
        assert_eq!(gl.segs[1].partner, Some(GlSegIdx(0)));
        assert_eq!(gl.subsectors.len(), 1);
        assert_eq!(gl.subsectors[0].segs, 0..2);
        assert_eq!(gl.nodes.len(), 1);
        assert_eq!(gl.nodes[0].right, GlNodeChild::Subsector(GlSubsectorIdx(0)));
        assert_eq!(gl.nodes[0].left, GlNodeChild::Subsector(GlSubsectorIdx(0)));
        assert!(w.is_empty());
    }

    #[test]
    fn v3_group_strips_gnd3_header_on_segs_and_ssect() {
        // GL_VERT: gNd2 + 1 GL vertex (V3 shares V2's vert magic).
        let mut vert = b"gNd2".to_vec();
        vert.extend_from_slice(&0i32.to_le_bytes());
        vert.extend_from_slice(&0i32.to_le_bytes());
        // GL_SEGS: gNd3 header + one 16-byte V3 seg (GL 0 -> normal 0, miniseg).
        let mut segs = b"gNd3".to_vec();
        segs.extend(v5_seg(
            0xC000_0000,
            0x0000_0000,
            0xFFFF,
            0x0000,
            0xFFFF_FFFF,
        ));
        // GL_SSECT: gNd3 header + one 8-byte V3 run.
        let mut ssect = b"gNd3".to_vec();
        ssect.extend(v3_ssect(1, 0));
        // GL_NODES: never carries a header; one 28-byte node -> subsector 0.
        let nodes = v2_node(0, 0, 0, 0, [0; 4], [0; 4], 0x8000, 0x8000);

        let mut w = Vec::new();
        let gl = decode_gl_group(
            &vert,
            &segs,
            &ssect,
            &nodes,
            1,
            1,
            Strictness::Strict,
            &mut w,
        )
        .unwrap();
        assert_eq!(gl.vertices.len(), 1);
        // Header stripping proven: the 16-byte record decodes as one seg, not a
        // framing error from the leading 4 magic bytes.
        assert_eq!(gl.segs.len(), 1);
        assert_eq!(gl.segs[0].start, GlVertexRef::Gl(GlVertexIdx(0)));
        assert_eq!(gl.segs[0].end, GlVertexRef::Normal(VertexIdx(0)));
        assert_eq!(gl.subsectors.len(), 1);
        assert_eq!(gl.subsectors[0].segs, 0..1);
        assert_eq!(gl.nodes.len(), 1);
        assert!(w.is_empty());
    }

    #[test]
    fn v3_group_short_ssect_lump_is_framing_error() {
        // GL_VERT: gNd2 + 1 GL vertex (V3 shares V2's vert magic).
        let mut vert = b"gNd2".to_vec();
        vert.extend_from_slice(&0i32.to_le_bytes());
        vert.extend_from_slice(&0i32.to_le_bytes());
        // GL_SEGS: exactly the 4-byte gNd3 header, nothing else. This alone is
        // enough for `detect_gl_version` to classify the group as V3 (it only
        // inspects `segs[0..4]`), so `segs.get(4..)` yields `Some(&[])`, not
        // `None` — the segs framing-error arm stays unreachable here, as expected.
        let segs = b"gNd3".to_vec();
        // GL_SSECT: shorter than the 4-byte gNd3 header a V3 group requires.
        // `detect_gl_version` never inspects GL_SSECT, so this is the one lump
        // that can genuinely trip the post-detection framing check.
        let ssect = b"gN".to_vec();
        let nodes: Vec<u8> = Vec::new();

        for strictness in [Strictness::Strict, Strictness::Lenient] {
            let mut w = Vec::new();
            let err = decode_gl_group(&vert, &segs, &ssect, &nodes, 1, 1, strictness, &mut w)
                .unwrap_err();
            assert!(matches!(
                err,
                MapAssembleError::Records {
                    lump: "GL_SSECT",
                    ..
                }
            ));
        }
    }

    #[test]
    fn v4_group_refused_strict_errors_lenient_warns() {
        let mut vert = b"gNd4".to_vec();
        vert.extend_from_slice(&[0u8; 8]);

        let err = decode_gl_group(
            &vert,
            &[],
            &[],
            &[],
            0,
            0,
            Strictness::Strict,
            &mut Vec::new(),
        )
        .unwrap_err();
        assert!(matches!(
            err,
            MapAssembleError::UnsupportedGlNodeVersion { magic } if &magic == b"gNd4"
        ));

        let mut w = Vec::new();
        let gl = decode_gl_group(&vert, &[], &[], &[], 0, 0, Strictness::Lenient, &mut w).unwrap();
        assert!(gl.vertices.is_empty());
        assert!(gl.segs.is_empty());
        assert!(gl.subsectors.is_empty());
        assert!(gl.nodes.is_empty());
        assert!(matches!(
            w.as_slice(),
            [MapWarning::GlNodesRefused { version: 4 }]
        ));
    }

    #[test]
    fn short_vert_refusal_pads_magic() {
        // gl_vert shorter than 4 bytes: refused as V1, magic zero-padded.
        let err = decode_gl_group(
            b"gN",
            &[],
            &[],
            &[],
            0,
            0,
            Strictness::Strict,
            &mut Vec::new(),
        )
        .unwrap_err();
        assert!(matches!(
            err,
            MapAssembleError::UnsupportedGlNodeVersion { magic } if magic == [b'g', b'N', 0, 0]
        ));
    }

    #[test]
    fn dangling_reference_degrades_in_lenient_errors_in_strict() {
        // GL_VERT holds zero GL vertices, so any GL-vertex reference lands in an
        // empty arena that cannot be clamped -> a hard DanglingReference in both
        // modes. seg v1 (normal 5) is a recoverable clamp; seg v2 (GL 0) is the
        // fatal empty-arena reference.
        let vert = b"gNd2".to_vec();
        let segs = v2_seg(0x0005, 0x8000, 0xFFFF, 0x0000, 0xFFFF);

        // Strict: the first dangling reference propagates.
        let err = decode_gl_group(
            &vert,
            &segs,
            &[],
            &[],
            1,
            1,
            Strictness::Strict,
            &mut Vec::new(),
        )
        .unwrap_err();
        assert!(matches!(err, MapAssembleError::DanglingReference { .. }));

        // Lenient: the whole group degrades -- empty arenas, a single
        // GlNodesDegraded, and the intermediate clamp warning rolled back.
        let mut w = Vec::new();
        let gl =
            decode_gl_group(&vert, &segs, &[], &[], 1, 1, Strictness::Lenient, &mut w).unwrap();
        assert!(gl.vertices.is_empty());
        assert!(gl.segs.is_empty());
        assert!(gl.subsectors.is_empty());
        assert!(gl.nodes.is_empty());
        assert!(matches!(w.as_slice(), [MapWarning::GlNodesDegraded]));
    }
}
