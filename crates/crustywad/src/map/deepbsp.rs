//! Decoding `DeePBSP` v4 (`xNd4`) BSP node lumps (ADR-0025 Stage 3, #328).
//!
//! `DeePBSP` v4 is a **classic-widened** node format, not a ZDoom extended
//! variant: it keeps the classic three separate lumps `SEGS`/`SSECTORS`/`NODES`
//! but widens the on-disk records so vertex, seg, and child indices are 32-bit.
//! Its seg semantics are the classic ones — `angle`/`offset`/`side` are stored
//! on disk (no derivation), there are **no minisegs** (so every seg has a
//! backing linedef), and it adds **no new vertices** (the map's existing
//! `VERTEXES` lump is used unchanged). The lumps are uncompressed, so this
//! decoder is always-on core (no feature flag).
//!
//! Byte layout source-verified against gzdoom's `mapseg4_t`/`mapsubsector4_t`/
//! `mapnode4_t` structs (`src/doomdata.h`) and their loader
//! (`src/maploader/maploader.cpp`); the format and staging are recorded in
//! [ADR-0025](https://github.com/masriamir/crustywad/blob/main/docs/adr/0025-extended-node-formats.md)
//! (§1–§3 and the Stage 3 amendment). Only the detection signature and the
//! record widths differ from classic; the
//! cross-reference normalization reuses the classic discipline from
//! [`assemble`](crate::map::assemble): [`resolve_required`] for strict-error /
//! lenient-clamp handling of out-of-range references, and the same whole-BSP
//! lenient degrade posture as `normalize_bsp_or_degrade`.

use binrw::BinRead;

use crate::Strictness;
use crate::map::assemble::{MapAssembleError, resolve_required};
use crate::map::graph::{
    LinedefIdx, MapNode, MapSeg, MapSubsector, MapWarning, NodeChild, NodeIdx, SubsectorIdx,
    VertexIdx,
};
use crate::map::{MapParseError, parse_records};

/// The length of the `xNd4\0\0\0\0` signature that heads the `NODES` lump.
/// Node records begin at this offset (`NF_LUMPOFFSET = 8`, gzdoom
/// `doomdata.h`).
const NODES_SIGNATURE_LEN: usize = 8;

/// The 8-byte `DeePBSP` v4 signature that heads a `DeePBSP` `NODES` lump
/// (gzdoom `doomdata.h`). The assembly gate ([`crate::map::assemble`]) routes a
/// `NODES` lump starting with these bytes to [`decode_deepbsp`], ahead of the
/// 4-byte `ZDoom` `EXTENDED_NODE_SIGNATURES` check.
pub(crate) const DEEPBSP_SIGNATURE: [u8; NODES_SIGNATURE_LEN] = *b"xNd4\0\0\0\0";

/// Returns `true` when `nodes` begins with the 8-byte [`DEEPBSP_SIGNATURE`],
/// identifying it as a `DeePBSP` v4 `NODES` lump.
pub(crate) fn is_deepbsp(nodes: &[u8]) -> bool {
    nodes.starts_with(&DEEPBSP_SIGNATURE)
}

/// The child-index leaf flag: when bit 31 is set the remaining 31 bits are a
/// subsector index, otherwise the whole value is a node index
/// (`NF_SUBSECTOR = 0x80000000`, gzdoom `doomdata.h`; maploader.cpp:1210-1212).
/// This is bit **31**, widened from the classic bit-15 `0x8000` flag.
const NF_SUBSECTOR: u32 = 0x8000_0000;

/// A single record from a `DeePBSP` v4 `SEGS` lump — **16 bytes**, little-endian
/// (gzdoom `mapseg4_t`, `doomdata.h:279-290`). Widened from the classic 12-byte
/// `mapseg_t` only in its two vertex indices (`v1`/`v2` become `i32`); the
/// trailing four fields keep classic widths and semantics.
#[derive(Debug, Clone, PartialEq, Eq, BinRead)]
#[br(little)]
pub(crate) struct Seg4 {
    /// Index into the map's `VERTEXES` lump for this seg's start vertex
    /// (widened to `i32` from the classic `u16`).
    pub v1: i32,
    /// Index into the map's `VERTEXES` lump for this seg's end vertex
    /// (widened to `i32` from the classic `u16`).
    pub v2: i32,
    /// The seg's raw 16-bit binary angle (BAM). gzdoom types this field as a
    /// signed `short` but uses it unsigned; the crate's classic
    /// [`Seg`](crate::map::common::Seg) likewise stores the raw BAM as `u16`,
    /// so this decoder keeps the byte-identical `u16` view (offset 8).
    pub angle: u16,
    /// Index into the map's `LINEDEFS` lump for the linedef this seg was cut
    /// from. `DeePBSP` has no minisegs, so this is always a real linedef
    /// (offset 10).
    pub linedef: u16,
    /// Which side of the parent linedef this seg faces: `0` = right/front,
    /// `1` = left/back. gzdoom types this as a signed `short`; the value is a
    /// small selector, and the crate's classic seg stores the direction as
    /// `u16`, so this decoder keeps the byte-identical `u16` view (offset 12).
    pub side: u16,
    /// Distance along the parent linedef from its start vertex to the start of
    /// this seg, in map units. Signed (negative after BSP splitting), classic
    /// width (offset 14).
    pub offset: i16,
}

/// A single record from a `DeePBSP` v4 `SSECTORS` lump — **6 bytes, packed**,
/// little-endian (gzdoom `mapsubsector4_t`, `doomdata.h:256-262`). Widened from
/// the classic 4-byte `mapsubsector_t` only in `firstseg` (now `u32`).
///
/// gzdoom declares the struct `#pragma pack(1)`, so its on-disk size is 6 bytes
/// (`u16` + `u32`), **not** the 8 bytes natural alignment would give. `binrw`
/// inserts no padding, so this struct advances exactly 6 bytes and
/// [`parse_records`] derives a 6-byte stride (proven by the `subsector4_stride`
/// test).
#[derive(Debug, Clone, PartialEq, Eq, BinRead)]
#[br(little)]
pub(crate) struct Subsector4 {
    /// Number of segs that make up this subsector's boundary (offset 0).
    pub numsegs: u16,
    /// Index into the `SEGS` lump of the first seg in this subsector; the run
    /// is `firstseg..firstseg + numsegs`. Widened to `u32` from the classic
    /// `u16` (offset 2).
    pub firstseg: u32,
}

/// A single record from a `DeePBSP` v4 `NODES` lump — **32 bytes**,
/// little-endian, beginning at offset 8 after the `xNd4\0\0\0\0` signature
/// (gzdoom `mapnode4_t`, `doomdata.h:315-329`). Widened from the classic
/// 28-byte `mapnode_t` only in its two child indices (now `u32`).
#[derive(Debug, Clone, PartialEq, Eq, BinRead)]
#[br(little)]
pub(crate) struct Node4 {
    /// X coordinate of the partition line's start point, in map units
    /// (offset 0).
    pub x: i16,
    /// Y coordinate of the partition line's start point, in map units
    /// (offset 2).
    pub y: i16,
    /// Horizontal extent of the partition line (delta X), in map units
    /// (offset 4).
    pub dx: i16,
    /// Vertical extent of the partition line (delta Y), in map units
    /// (offset 6).
    pub dy: i16,
    /// Axis-aligned bounding box for the **right** child, as
    /// `[top, bottom, left, right]` in map units (offset 8; classic
    /// `BOXTOP=0, BOXBOTTOM=1, BOXLEFT=2, BOXRIGHT=3` order).
    pub right_bbox: [i16; 4],
    /// Axis-aligned bounding box for the **left** child, same `[top, bottom,
    /// left, right]` order (offset 16).
    pub left_bbox: [i16; 4],
    /// Right (front) child index. If bit 31 (`0x80000000`) is set the low 31
    /// bits are a subsector index, otherwise the whole value is a node index
    /// (offset 24). Widened to `u32` from the classic `u16` bit-15 encoding.
    pub right_child: u32,
    /// Left (back) child index, same bit-31 encoding as `right_child`
    /// (offset 28).
    pub left_child: u32,
}

/// Decodes `DeePBSP` v4 (`xNd4`) BSP lumps into the graph's `(segs, subsectors,
/// nodes)` arenas.
///
/// `nodes_bytes` **includes** the leading 8-byte `xNd4\0\0\0\0` signature;
/// `segs_bytes` and `ssectors_bytes` carry no signature and are read from byte
/// 0. A `NODES` lump of exactly 8 bytes (signature only) is the valid degenerate
/// single-leaf case and yields zero nodes.
///
/// Mirrors the classic BSP normalization ([`assemble`](crate::map::assemble)):
/// out-of-range vertex/linedef/child references take the shared
/// [`resolve_required`] discipline (strict error, lenient clamp-to-0 + warn),
/// and an unrecoverable reference into an empty arena degrades the whole BSP to
/// empty arenas plus a single warning in lenient mode (the same posture as
/// `normalize_bsp_or_degrade`). A structurally malformed lump — a `NODES` lump
/// shorter than its 8-byte signature, or any of the three lumps whose length is
/// not a whole multiple of its record size — is a fatal
/// [`MapAssembleError::Records`] in **both** strictness modes, exactly as the
/// classic record decoders (`decode_required`/`decode_optional`) treat a
/// malformed record stream.
///
/// # Errors
///
/// - [`MapAssembleError::Records`] — a `NODES` lump shorter than 8 bytes, or a
///   lump length that is not a whole multiple of its record size (16 for
///   `SEGS`, 6 for `SSECTORS`, 32 for post-signature `NODES`).
/// - [`MapAssembleError::DanglingReference`] (strict mode) — a seg vertex or
///   linedef index, a subsector seg run, or a node child index out of range.
///   Lenient mode recovers each with a [`MapWarning::DanglingReference`].
#[allow(clippy::type_complexity)]
pub(crate) fn decode_deepbsp(
    segs_bytes: &[u8],
    ssectors_bytes: &[u8],
    nodes_bytes: &[u8],
    vertex_count: usize,
    linedef_count: usize,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<(Vec<MapSeg>, Vec<MapSubsector>, Vec<MapNode>), MapAssembleError> {
    // The NODES lump must be able to hold at least its 8-byte signature.
    // Detection guarantees this in the real pipeline; guard defensively so a
    // direct caller cannot slice past the end. A truncated signature is a
    // malformed record stream (fatal in both modes, like the classic path).
    if nodes_bytes.len() < NODES_SIGNATURE_LEN {
        return Err(MapAssembleError::Records {
            lump: "NODES",
            source: MapParseError::TrailingBytes { offset: 0 },
        });
    }

    let raw_segs =
        parse_records::<Seg4>(segs_bytes).map_err(|source| MapAssembleError::Records {
            lump: "SEGS",
            source,
        })?;
    let raw_subsectors = parse_records::<Subsector4>(ssectors_bytes).map_err(|source| {
        MapAssembleError::Records {
            lump: "SSECTORS",
            source,
        }
    })?;
    // Node records begin at offset 8, after the signature; count = (len-8)/32.
    let raw_nodes =
        parse_records::<Node4>(&nodes_bytes[NODES_SIGNATURE_LEN..]).map_err(|source| {
            MapAssembleError::Records {
                lump: "NODES",
                source,
            }
        })?;

    // Whole-BSP lenient degrade: a reference into an empty arena has nothing to
    // clamp to (BSP data is optional), so lenient assembly must not fail on it.
    // Discard any per-element warnings for the now-dropped arenas and surface a
    // single warning — the same posture as `normalize_bsp_or_degrade`.
    let watermark = warnings.len();
    match normalize_deepbsp(
        &raw_segs,
        &raw_subsectors,
        &raw_nodes,
        vertex_count,
        linedef_count,
        strictness,
        warnings,
    ) {
        Err(MapAssembleError::DanglingReference {
            referent,
            index,
            from,
            count,
        }) if strictness == Strictness::Lenient => {
            warnings.truncate(watermark);
            warnings.push(MapWarning::DanglingReference {
                referent,
                index,
                from,
                count,
            });
            Ok((Vec::new(), Vec::new(), Vec::new()))
        }
        other => other,
    }
}

/// Resolves one `DeePBSP` v4 node child (`right_child`/`left_child`).
///
/// Bit 31 (`0x80000000`, [`NF_SUBSECTOR`]) set selects a subsector leaf (the
/// low 31 bits index into `subsector_count`); clear selects an internal node
/// (the whole value indexes into `node_count`). This is the bit-31/`u32`
/// analog of the classic bit-15/`u16` `resolve_node_child`; both share the
/// [`resolve_required`] range-check discipline. A `DeePBSP`-local variant (rather
/// than a generalized classic helper) keeps the classic hot path untouched.
fn resolve_deepbsp_child(
    raw: u32,
    node_count: usize,
    subsector_count: usize,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<NodeChild, MapAssembleError> {
    if raw & NF_SUBSECTOR == 0 {
        // Node index: `raw <= 0x7FFF_FFFF` here, so it fits `i32` losslessly.
        let index = i32::try_from(raw).unwrap_or(i32::MAX);
        Ok(NodeChild::Node(NodeIdx(resolve_required(
            index, node_count, "node", "node", strictness, warnings,
        )?)))
    } else {
        // Subsector index: mask off bit 31; the remaining value fits `i32`.
        let index = i32::try_from(raw & !NF_SUBSECTOR).unwrap_or(i32::MAX);
        Ok(NodeChild::Subsector(SubsectorIdx(resolve_required(
            index,
            subsector_count,
            "subsector",
            "node",
            strictness,
            warnings,
        )?)))
    }
}

/// Normalizes decoded `DeePBSP` v4 records into the graph arenas, validating every
/// cross-reference (mirror of the classic `normalize_bsp`). Iterative: the BSP
/// tree is stored, not walked, so no crafted input can recurse.
#[allow(clippy::type_complexity)]
fn normalize_deepbsp(
    raw_segs: &[Seg4],
    raw_subsectors: &[Subsector4],
    raw_nodes: &[Node4],
    vertex_count: usize,
    linedef_count: usize,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<(Vec<MapSeg>, Vec<MapSubsector>, Vec<MapNode>), MapAssembleError> {
    let mut segs = Vec::with_capacity(raw_segs.len());
    for sg in raw_segs {
        segs.push(MapSeg {
            start: VertexIdx(resolve_required(
                sg.v1,
                vertex_count,
                "vertex",
                "seg",
                strictness,
                warnings,
            )?),
            end: VertexIdx(resolve_required(
                sg.v2,
                vertex_count,
                "vertex",
                "seg",
                strictness,
                warnings,
            )?),
            angle: sg.angle,
            // `DeePBSP` has no minisegs: every seg has a backing linedef.
            linedef: Some(LinedefIdx(resolve_required(
                i32::from(sg.linedef),
                linedef_count,
                "linedef",
                "seg",
                strictness,
                warnings,
            )?)),
            direction: sg.side,
            offset: i32::from(sg.offset),
        });
    }

    let mut subsectors = Vec::with_capacity(raw_subsectors.len());
    for ss in raw_subsectors {
        let first = ss.firstseg as usize;
        // `checked_add` guards a pathological `firstseg` near `usize::MAX` on
        // 32-bit targets; overflow is treated as an out-of-range run.
        let end = first.checked_add(usize::from(ss.numsegs));
        let range = match end {
            Some(end) if end <= segs.len() && first <= segs.len() => first..end,
            _ => {
                let reported = end.unwrap_or(usize::MAX);
                match strictness {
                    Strictness::Strict => {
                        return Err(MapAssembleError::DanglingReference {
                            referent: "seg",
                            index: i32::try_from(reported).unwrap_or(i32::MAX),
                            from: "subsector",
                            count: segs.len(),
                        });
                    }
                    Strictness::Lenient => {
                        warnings.push(MapWarning::DanglingReference {
                            referent: "seg",
                            index: i32::try_from(reported).unwrap_or(i32::MAX),
                            from: "subsector",
                            count: segs.len(),
                        });
                        first.min(segs.len())..segs.len()
                    }
                }
            }
        };
        subsectors.push(MapSubsector {
            segs: range,
            leafs: 0..0,
        });
    }

    let node_count = raw_nodes.len();
    let subsector_count = subsectors.len();
    let mut nodes = Vec::with_capacity(node_count);
    for nd in raw_nodes {
        let right = resolve_deepbsp_child(
            nd.right_child,
            node_count,
            subsector_count,
            strictness,
            warnings,
        )?;
        let left = resolve_deepbsp_child(
            nd.left_child,
            node_count,
            subsector_count,
            strictness,
            warnings,
        )?;
        nodes.push(MapNode {
            x: i32::from(nd.x),
            y: i32::from(nd.y),
            dx: i32::from(nd.dx),
            dy: i32::from(nd.dy),
            right_bbox: nd.right_bbox.map(i32::from),
            left_bbox: nd.left_bbox.map(i32::from),
            right,
            left,
        });
    }

    Ok((segs, subsectors, nodes))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Encodes a `Seg4` (16 bytes, little-endian).
    fn seg4(v1: i32, v2: i32, angle: u16, linedef: u16, side: u16, offset: i16) -> Vec<u8> {
        let mut b = Vec::with_capacity(16);
        b.extend_from_slice(&v1.to_le_bytes());
        b.extend_from_slice(&v2.to_le_bytes());
        b.extend_from_slice(&angle.to_le_bytes());
        b.extend_from_slice(&linedef.to_le_bytes());
        b.extend_from_slice(&side.to_le_bytes());
        b.extend_from_slice(&offset.to_le_bytes());
        b
    }

    /// Encodes a `Subsector4` (6 bytes, packed, little-endian).
    fn subsector4(numsegs: u16, firstseg: u32) -> Vec<u8> {
        let mut b = Vec::with_capacity(6);
        b.extend_from_slice(&numsegs.to_le_bytes());
        b.extend_from_slice(&firstseg.to_le_bytes());
        b
    }

    /// Encodes a `Node4` (32 bytes, little-endian).
    #[allow(clippy::too_many_arguments)]
    fn node4(
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
        for v in right_bbox.iter().chain(left_bbox.iter()) {
            b.extend_from_slice(&v.to_le_bytes());
        }
        b.extend_from_slice(&right_child.to_le_bytes());
        b.extend_from_slice(&left_child.to_le_bytes());
        b
    }

    /// The 8-byte `xNd4\0\0\0\0` NODES signature followed by `node_bytes`.
    fn nodes_lump(node_bytes: &[u8]) -> Vec<u8> {
        let mut b = b"xNd4\0\0\0\0".to_vec();
        b.extend_from_slice(node_bytes);
        b
    }

    /// A valid 4-vertex square: 4 segs, 2 single-seg-ish subsectors, 1 node
    /// whose two children are subsector leaves. Asserts every decoded field.
    #[test]
    fn valid_square_map() {
        // 4 segs forming the square's edges (vertices 0..4, linedefs 0..4).
        let mut segs = Vec::new();
        segs.extend(seg4(0, 1, 0x0000, 0, 0, 0));
        segs.extend(seg4(1, 2, 0x4000, 1, 0, 5));
        segs.extend(seg4(2, 3, 0x8000, 2, 1, -3));
        segs.extend(seg4(3, 0, 0xC000, 3, 0, 0));

        // Two subsectors: segs [0,2) and [2,4).
        let mut ssectors = Vec::new();
        ssectors.extend(subsector4(2, 0));
        ssectors.extend(subsector4(2, 2));

        // One node whose children are the two subsector leaves (bit 31 set).
        let node = node4(
            16,
            32,
            8,
            -8,
            [64, -64, -64, 64],
            [63, -63, -62, 61],
            NF_SUBSECTOR,     // right -> subsector 0
            NF_SUBSECTOR | 1, // left  -> subsector 1
        );
        let nodes = nodes_lump(&node);

        let mut warnings = Vec::new();
        let (segs, subsectors, nodes) = decode_deepbsp(
            &segs,
            &ssectors,
            &nodes,
            4, // vertex_count
            4, // linedef_count
            Strictness::Strict,
            &mut warnings,
        )
        .expect("valid square decodes");
        assert!(warnings.is_empty());

        // Segs: every field.
        assert_eq!(segs.len(), 4);
        assert_eq!(segs[0].start, VertexIdx(0));
        assert_eq!(segs[0].end, VertexIdx(1));
        assert_eq!(segs[0].angle, 0x0000);
        assert_eq!(segs[0].linedef, Some(LinedefIdx(0)));
        assert_eq!(segs[0].direction, 0);
        assert_eq!(segs[0].offset, 0);
        assert_eq!(segs[1].angle, 0x4000);
        assert_eq!(segs[1].linedef, Some(LinedefIdx(1)));
        assert_eq!(segs[1].offset, 5);
        assert_eq!(segs[2].start, VertexIdx(2));
        assert_eq!(segs[2].end, VertexIdx(3));
        assert_eq!(segs[2].angle, 0x8000);
        assert_eq!(segs[2].direction, 1);
        assert_eq!(segs[2].offset, -3);
        assert_eq!(segs[3].end, VertexIdx(0));
        assert_eq!(segs[3].angle, 0xC000);

        // Subsectors: seg runs.
        assert_eq!(subsectors.len(), 2);
        assert_eq!(subsectors[0].segs, 0..2);
        assert_eq!(subsectors[0].leafs, 0..0);
        assert_eq!(subsectors[1].segs, 2..4);

        // Node: partition line, bboxes, and bit-31 children.
        assert_eq!(nodes.len(), 1);
        let n = &nodes[0];
        assert_eq!((n.x, n.y, n.dx, n.dy), (16, 32, 8, -8));
        assert_eq!(n.right_bbox, [64, -64, -64, 64]);
        assert_eq!(n.left_bbox, [63, -63, -62, 61]);
        assert_eq!(n.right, NodeChild::Subsector(SubsectorIdx(0)));
        assert_eq!(n.left, NodeChild::Subsector(SubsectorIdx(1)));
    }

    /// The degenerate single-leaf case: one subsector, zero nodes (an 8-byte
    /// signature-only NODES lump). Valid — must not be rejected.
    #[test]
    fn degenerate_single_leaf() {
        let mut segs = Vec::new();
        segs.extend(seg4(0, 1, 0, 0, 0, 0));
        let ssectors = subsector4(1, 0);
        let nodes = nodes_lump(&[]); // signature only -> zero nodes

        let mut warnings = Vec::new();
        let (segs, subsectors, nodes) = decode_deepbsp(
            &segs,
            &ssectors,
            &nodes,
            2,
            1,
            Strictness::Strict,
            &mut warnings,
        )
        .expect("degenerate single-leaf decodes");
        assert!(warnings.is_empty());
        assert_eq!(segs.len(), 1);
        assert_eq!(subsectors.len(), 1);
        assert_eq!(subsectors[0].segs, 0..1);
        assert!(nodes.is_empty());
    }

    /// `Subsector4` is 6 bytes packed, not 8: an 18-byte lump is 3 records.
    #[test]
    fn subsector4_stride() {
        let mut lump = Vec::new();
        lump.extend(subsector4(1, 10));
        lump.extend(subsector4(2, 20));
        lump.extend(subsector4(3, 30));
        assert_eq!(lump.len(), 18);

        let recs = parse_records::<Subsector4>(&lump).expect("3 records");
        assert_eq!(recs.len(), 3);
        assert_eq!(
            recs[0],
            Subsector4 {
                numsegs: 1,
                firstseg: 10
            }
        );
        assert_eq!(
            recs[1],
            Subsector4 {
                numsegs: 2,
                firstseg: 20
            }
        );
        assert_eq!(
            recs[2],
            Subsector4 {
                numsegs: 3,
                firstseg: 30
            }
        );
    }

    /// NODES = signature(8) + 1 node(32) = 40 bytes decodes to exactly 1 node
    /// (records begin at offset 8).
    #[test]
    fn node_offset_after_signature() {
        let segs = seg4(0, 1, 0, 0, 0, 0);
        let ssectors = subsector4(1, 0);
        // One node with a single-subsector map: both children -> subsector 0.
        let node = node4(
            0,
            0,
            1,
            1,
            [1, -1, -1, 1],
            [1, -1, -1, 1],
            NF_SUBSECTOR,
            NF_SUBSECTOR,
        );
        let nodes = nodes_lump(&node);
        assert_eq!(nodes.len(), 40);

        let mut warnings = Vec::new();
        let (_, _, nodes) = decode_deepbsp(
            &segs,
            &ssectors,
            &nodes,
            2,
            1,
            Strictness::Strict,
            &mut warnings,
        )
        .expect("decodes");
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].right, NodeChild::Subsector(SubsectorIdx(0)));
        assert_eq!(nodes[0].left, NodeChild::Subsector(SubsectorIdx(0)));
    }

    /// A node child that is an internal node index (bit 31 clear) resolves to a
    /// `NodeChild::Node`.
    #[test]
    fn node_child_internal_node() {
        let segs = seg4(0, 1, 0, 0, 0, 0);
        let ssectors = subsector4(1, 0);
        // Node 0's right child points at node 1 (index 1, bit 31 clear);
        // node 1's children are the single subsector.
        let mut node_bytes = Vec::new();
        node_bytes.extend(node4(0, 0, 1, 1, [0; 4], [0; 4], 1, NF_SUBSECTOR));
        node_bytes.extend(node4(
            0,
            0,
            1,
            1,
            [0; 4],
            [0; 4],
            NF_SUBSECTOR,
            NF_SUBSECTOR,
        ));
        let nodes = nodes_lump(&node_bytes);

        let mut warnings = Vec::new();
        let (_, _, nodes) = decode_deepbsp(
            &segs,
            &ssectors,
            &nodes,
            2,
            1,
            Strictness::Strict,
            &mut warnings,
        )
        .expect("decodes");
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].right, NodeChild::Node(NodeIdx(1)));
        assert_eq!(nodes[0].left, NodeChild::Subsector(SubsectorIdx(0)));
    }

    /// (a) A NODES lump shorter than its 8-byte signature is a fatal `Records`
    /// error in both modes (a malformed record stream, like the classic path).
    #[test]
    fn malformed_nodes_too_short() {
        let segs = seg4(0, 1, 0, 0, 0, 0);
        let ssectors = subsector4(1, 0);
        let short_nodes = b"xNd4\0\0\0"; // 7 bytes

        for strictness in [Strictness::Strict, Strictness::Lenient] {
            let mut warnings = Vec::new();
            let err = decode_deepbsp(
                &segs,
                &ssectors,
                short_nodes,
                2,
                1,
                strictness,
                &mut warnings,
            )
            .expect_err("too-short NODES is fatal");
            assert!(matches!(
                err,
                MapAssembleError::Records { lump: "NODES", .. }
            ));
        }
    }

    /// (b) A seg vertex index out of range: strict errors, lenient clamps + warns.
    #[test]
    fn seg_vertex_out_of_range() {
        // v1 = 9 but only 4 vertices exist.
        let segs = seg4(9, 1, 0, 0, 0, 0);
        let ssectors = subsector4(1, 0);
        let nodes = nodes_lump(&[]);

        let mut w = Vec::new();
        let err = decode_deepbsp(&segs, &ssectors, &nodes, 4, 1, Strictness::Strict, &mut w)
            .expect_err("strict rejects");
        assert!(matches!(
            err,
            MapAssembleError::DanglingReference {
                referent: "vertex",
                index: 9,
                from: "seg",
                count: 4
            }
        ));

        let mut w = Vec::new();
        let (segs, _, _) =
            decode_deepbsp(&segs, &ssectors, &nodes, 4, 1, Strictness::Lenient, &mut w)
                .expect("lenient recovers");
        assert_eq!(segs[0].start, VertexIdx(0)); // clamped to 0
        assert_eq!(w.len(), 1);
        assert!(matches!(
            w[0],
            MapWarning::DanglingReference {
                referent: "vertex",
                index: 9,
                ..
            }
        ));
    }

    /// (c) A seg linedef index out of range: strict errors, lenient clamps + warns.
    #[test]
    fn seg_linedef_out_of_range() {
        let segs = seg4(0, 1, 0, 7, 0, 0); // linedef 7, only 2 exist
        let ssectors = subsector4(1, 0);
        let nodes = nodes_lump(&[]);

        let mut w = Vec::new();
        let err = decode_deepbsp(&segs, &ssectors, &nodes, 2, 2, Strictness::Strict, &mut w)
            .expect_err("strict rejects");
        assert!(matches!(
            err,
            MapAssembleError::DanglingReference {
                referent: "linedef",
                index: 7,
                from: "seg",
                count: 2
            }
        ));

        let mut w = Vec::new();
        let (segs, _, _) =
            decode_deepbsp(&segs, &ssectors, &nodes, 2, 2, Strictness::Lenient, &mut w)
                .expect("lenient recovers");
        assert_eq!(segs[0].linedef, Some(LinedefIdx(0)));
        assert_eq!(w.len(), 1);
    }

    /// (d) A subsector seg run past `segs.len()`: strict errors, lenient truncates.
    #[test]
    fn subsector_run_past_segs() {
        let segs = seg4(0, 1, 0, 0, 0, 0); // 1 seg
        let ssectors = subsector4(5, 0); // claims 5 segs from index 0
        let nodes = nodes_lump(&[]);

        let mut w = Vec::new();
        let err = decode_deepbsp(&segs, &ssectors, &nodes, 2, 1, Strictness::Strict, &mut w)
            .expect_err("strict rejects");
        assert!(matches!(
            err,
            MapAssembleError::DanglingReference {
                referent: "seg",
                from: "subsector",
                count: 1,
                ..
            }
        ));

        let mut w = Vec::new();
        let (_, subsectors, _) =
            decode_deepbsp(&segs, &ssectors, &nodes, 2, 1, Strictness::Lenient, &mut w)
                .expect("lenient recovers");
        assert_eq!(subsectors[0].segs, 0..1); // truncated to the arena
        assert_eq!(w.len(), 1);
    }

    /// (e) A node child index out of range (bit 31 clear, node index too big):
    /// strict errors, lenient clamps + warns.
    #[test]
    fn node_child_out_of_range() {
        let segs = seg4(0, 1, 0, 0, 0, 0);
        let ssectors = subsector4(1, 0);
        // Single node; right child references node index 9 (only 1 node).
        let node = node4(0, 0, 1, 1, [0; 4], [0; 4], 9, NF_SUBSECTOR);
        let nodes = nodes_lump(&node);

        let mut w = Vec::new();
        let err = decode_deepbsp(&segs, &ssectors, &nodes, 2, 1, Strictness::Strict, &mut w)
            .expect_err("strict rejects");
        assert!(matches!(
            err,
            MapAssembleError::DanglingReference {
                referent: "node",
                index: 9,
                from: "node",
                count: 1
            }
        ));

        let mut w = Vec::new();
        let (_, _, nodes) =
            decode_deepbsp(&segs, &ssectors, &nodes, 2, 1, Strictness::Lenient, &mut w)
                .expect("lenient recovers");
        assert_eq!(nodes[0].right, NodeChild::Node(NodeIdx(0))); // clamped
        assert_eq!(w.len(), 1);
    }

    /// (f) A record-size-misaligned lump (SEGS length not a multiple of 16) is a
    /// fatal `Records` error in both modes.
    #[test]
    fn misaligned_segs_lump() {
        let mut segs = seg4(0, 1, 0, 0, 0, 0);
        segs.truncate(15); // one byte short of a whole record
        let ssectors = subsector4(1, 0);
        let nodes = nodes_lump(&[]);

        for strictness in [Strictness::Strict, Strictness::Lenient] {
            let mut w = Vec::new();
            let err = decode_deepbsp(&segs, &ssectors, &nodes, 2, 1, strictness, &mut w)
                .expect_err("misaligned SEGS is fatal");
            assert!(matches!(
                err,
                MapAssembleError::Records { lump: "SEGS", .. }
            ));
        }
    }

    /// Whole-BSP lenient degrade: a node child referencing an empty subsector
    /// arena has nothing to clamp to, so lenient returns empty arenas + one
    /// warning (the `normalize_bsp_or_degrade` posture); strict errors.
    #[test]
    fn empty_arena_degrades_lenient() {
        let segs = seg4(0, 1, 0, 0, 0, 0);
        // No subsectors at all, but a node references a subsector leaf.
        let ssectors: &[u8] = &[];
        let node = node4(0, 0, 1, 1, [0; 4], [0; 4], NF_SUBSECTOR, NF_SUBSECTOR);
        let nodes = nodes_lump(&node);

        // Strict: hard error (count == 0).
        let mut w = Vec::new();
        decode_deepbsp(&segs, ssectors, &nodes, 2, 1, Strictness::Strict, &mut w)
            .expect_err("strict rejects empty subsector arena");

        // Lenient: whole-BSP degrade to empty arenas + exactly one warning.
        let mut w = Vec::new();
        let (segs, subsectors, nodes) =
            decode_deepbsp(&segs, ssectors, &nodes, 2, 1, Strictness::Lenient, &mut w)
                .expect("lenient degrades");
        assert!(segs.is_empty());
        assert!(subsectors.is_empty());
        assert!(nodes.is_empty());
        assert_eq!(w.len(), 1);
        assert!(matches!(w[0], MapWarning::DanglingReference { .. }));
    }

    /// (g) A record-size-misaligned lump (SSECTORS length not a multiple of 6)
    /// is a fatal `Records` error in both modes.
    #[test]
    fn misaligned_ssectors_lump() {
        let segs = seg4(0, 1, 0, 0, 0, 0);
        let mut ssectors = subsector4(1, 0);
        ssectors.push(0); // one byte extra -> 7 bytes, not a multiple of 6
        let nodes = nodes_lump(&[]);

        for strictness in [Strictness::Strict, Strictness::Lenient] {
            let mut w = Vec::new();
            let err = decode_deepbsp(&segs, &ssectors, &nodes, 2, 1, strictness, &mut w)
                .expect_err("misaligned SSECTORS is fatal");
            assert!(matches!(
                err,
                MapAssembleError::Records {
                    lump: "SSECTORS",
                    ..
                }
            ));
        }
    }

    /// (h) A seg `v2` (end) vertex index out of range: strict errors, lenient
    /// clamps + warns. `v1` stays in range so this exercises the `v2` resolve
    /// distinctly from the existing `seg_vertex_out_of_range` (`v1`) case.
    #[test]
    fn seg_v2_vertex_out_of_range() {
        // v1 = 0 (valid), v2 = 9 but only 4 vertices exist.
        let segs = seg4(0, 9, 0, 0, 0, 0);
        let ssectors = subsector4(1, 0);
        let nodes = nodes_lump(&[]);

        let mut w = Vec::new();
        let err = decode_deepbsp(&segs, &ssectors, &nodes, 4, 1, Strictness::Strict, &mut w)
            .expect_err("strict rejects");
        assert!(matches!(
            err,
            MapAssembleError::DanglingReference {
                referent: "vertex",
                index: 9,
                from: "seg",
                count: 4
            }
        ));

        let mut w = Vec::new();
        let (segs, _, _) =
            decode_deepbsp(&segs, &ssectors, &nodes, 4, 1, Strictness::Lenient, &mut w)
                .expect("lenient recovers");
        assert_eq!(segs[0].start, VertexIdx(0)); // v1 unaffected
        assert_eq!(segs[0].end, VertexIdx(0)); // v2 clamped to 0
        assert_eq!(w.len(), 1);
        assert!(matches!(
            w[0],
            MapWarning::DanglingReference {
                referent: "vertex",
                index: 9,
                ..
            }
        ));
    }

    /// (i) A node's LEFT child index out of range, with a valid RIGHT child:
    /// strict errors, lenient clamps + warns. Exercises the left-child resolve
    /// distinctly from `node_child_out_of_range` (which faults the right child).
    #[test]
    fn node_left_child_out_of_range() {
        let segs = seg4(0, 1, 0, 0, 0, 0);
        let ssectors = subsector4(1, 0);
        // Single node; right child is a valid subsector leaf, left references
        // node index 9 (only 1 node exists, bit 31 clear).
        let node = node4(0, 0, 1, 1, [0; 4], [0; 4], NF_SUBSECTOR, 9);
        let nodes = nodes_lump(&node);

        let mut w = Vec::new();
        let err = decode_deepbsp(&segs, &ssectors, &nodes, 2, 1, Strictness::Strict, &mut w)
            .expect_err("strict rejects");
        assert!(matches!(
            err,
            MapAssembleError::DanglingReference {
                referent: "node",
                index: 9,
                from: "node",
                count: 1
            }
        ));

        let mut w = Vec::new();
        let (_, _, nodes) =
            decode_deepbsp(&segs, &ssectors, &nodes, 2, 1, Strictness::Lenient, &mut w)
                .expect("lenient recovers");
        assert_eq!(nodes[0].right, NodeChild::Subsector(SubsectorIdx(0)));
        assert_eq!(nodes[0].left, NodeChild::Node(NodeIdx(0))); // clamped
        assert_eq!(w.len(), 1);
    }
}
