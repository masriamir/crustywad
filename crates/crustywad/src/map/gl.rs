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

#[cfg(test)]
mod tests {
    use super::{GlVersion, detect_gl_version};

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
}
