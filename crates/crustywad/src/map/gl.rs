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
//! This module currently provides only version detection
//! ([`detect_gl_version`]); decoders for the individual lumps are added by
//! later tasks in the classic-GL read effort (#324).

use crate::map::MapParseError;
use crate::map::assemble::MapAssembleError;
use crate::map::graph::GlVertex;

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

#[cfg(test)]
mod tests {
    use super::{GlVersion, decode_gl_vertices, detect_gl_version};
    use crate::map::MapParseError;
    use crate::map::assemble::MapAssembleError;

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
}
