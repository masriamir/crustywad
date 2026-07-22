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
//! This module currently provides version detection ([`detect_gl_version`]),
//! the `GL_VERT` decoder ([`decode_gl_vertices`]), and the `GL_SEGS` decoder
//! ([`decode_gl_segs`], which applies the GL/normal vertex high-bit split);
//! decoders for the remaining lumps are added by later tasks in the classic-GL
//! read effort (#324).

use crate::Strictness;
use crate::map::MapParseError;
use crate::map::assemble::{MapAssembleError, resolve_required};
use crate::map::graph::{
    GlSeg, GlSegIdx, GlVertex, GlVertexIdx, GlVertexRef, LinedefIdx, MapWarning, VertexIdx,
};

/// A decodable classic GL node format version.
///
/// glBSP produced several incompatible on-disk layouts over its lifetime.
/// Only V2, V3, and V5 are decodable here — V1 and V4 are refused (see
/// [`detect_gl_version`]).
///
/// Not yet constructed outside tests: the per-version decoders that match on
/// this enum land in later tasks (#324), so `dead_code` is explicitly allowed
/// here until those call sites land.
#[allow(dead_code)]
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
/// Not yet called outside tests: assembly wiring (`Map::assemble`) lands in a
/// later task (#324), so `dead_code` is explicitly allowed here until that
/// call site lands.
#[allow(dead_code)]
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
/// Not yet called outside tests: assembly wiring lands in a later task
/// (#324), so `dead_code` is explicitly allowed here until that call site
/// lands.
#[allow(dead_code)]
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
#[allow(dead_code)] // Wired into assembly in a later task (#324).
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
/// Not yet called outside tests: assembly wiring lands in a later task (#324),
/// so `dead_code` is explicitly allowed here until that call site lands.
#[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::{GlVersion, decode_gl_segs, decode_gl_vertices, detect_gl_version};
    use crate::Strictness;
    use crate::map::MapParseError;
    use crate::map::assemble::MapAssembleError;
    use crate::map::graph::{
        GlSegIdx, GlVertexIdx, GlVertexRef, LinedefIdx, MapWarning, VertexIdx,
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
}
