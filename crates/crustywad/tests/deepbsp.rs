//! Integration tests for wiring the `DeePBSP` v4 (`xNd4`) decoder into the map
//! assembler's binary `SEGS`/`SSECTORS`/`NODES` gate (ADR-0025 Stage 3, #328).
//!
//! `DeePBSP` is a classic-widened node format detected by an 8-byte `xNd4`
//! signature at the head of the `NODES` lump, ahead of the 4-byte `ZDoom`
//! `EXTENDED_NODE_SIGNATURES` check. It is binary-only (it never appears on the
//! UDMF `ZNODES` path) and adds no new vertices. Each test assembles a whole
//! [`Map`] through the public API and asserts the resulting BSP arenas, in
//! **both** strictness modes.
//!
//! The framing-defect policy under test: a structurally malformed `DeePBSP`
//! lump (misaligned/truncated records) is a hard `Records` error in **both**
//! modes, mirroring the classic path it resembles — unlike the `ZDoom` `X*`/`Z*`
//! readers' whole-BSP lenient degrade.

mod common;

use crustywad::map::{LinedefIdx, Map, MapAssembleError, NodeIdx, VertexIdx};
use crustywad::{ParseOptions, Wad};

// --- Classic binary map-record encoders (a 64x64 square). ---

fn vertex(x: i16, y: i16) -> Vec<u8> {
    [x.to_le_bytes(), y.to_le_bytes()].concat()
}

/// The four corners of a 64x64 square.
fn square_vertexes() -> Vec<u8> {
    [vertex(0, 0), vertex(64, 0), vertex(64, 64), vertex(0, 64)].concat()
}

fn linedef(sv: u16, ev: u16, right: u16, left: u16) -> Vec<u8> {
    // 14 bytes: v1, v2, flags, special, tag, right sidedef, left sidedef.
    [sv, ev, 0, 0, 0, right, left]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect()
}

/// Four one-sided edge linedefs of the square (0->1->2->3->0).
fn square_linedefs() -> Vec<u8> {
    [
        linedef(0, 1, 0, 0xffff),
        linedef(1, 2, 0, 0xffff),
        linedef(2, 3, 0, 0xffff),
        linedef(3, 0, 0, 0xffff),
    ]
    .concat()
}

fn sidedef() -> Vec<u8> {
    // 30 bytes: x/y offset, upper/lower/middle (8 each), sector.
    let mut b = Vec::new();
    b.extend(0i16.to_le_bytes());
    b.extend(0i16.to_le_bytes());
    b.extend(b"-\0\0\0\0\0\0\0");
    b.extend(b"-\0\0\0\0\0\0\0");
    b.extend(b"WALL\0\0\0\0");
    b.extend(0u16.to_le_bytes());
    b
}

fn sector() -> Vec<u8> {
    // 26 bytes.
    let mut b = Vec::new();
    b.extend(0i16.to_le_bytes());
    b.extend(128i16.to_le_bytes());
    b.extend(b"FLOOR\0\0\0");
    b.extend(b"CEIL\0\0\0\0");
    b.extend(160i16.to_le_bytes());
    b.extend(0i16.to_le_bytes());
    b.extend(0i16.to_le_bytes());
    b
}

// --- DeePBSP v4 record encoders. ---

/// The child-index leaf flag (bit 31): set means a subsector leaf.
const NF_SUBSECTOR: u32 = 0x8000_0000;

/// Encodes a `DeePBSP` `SEGS` record (16 bytes, little-endian).
fn seg4(v1: i32, v2: i32, angle: u16, line: u16, side: u16, offset: i16) -> Vec<u8> {
    let mut b = Vec::with_capacity(16);
    b.extend_from_slice(&v1.to_le_bytes());
    b.extend_from_slice(&v2.to_le_bytes());
    b.extend_from_slice(&angle.to_le_bytes());
    b.extend_from_slice(&line.to_le_bytes());
    b.extend_from_slice(&side.to_le_bytes());
    b.extend_from_slice(&offset.to_le_bytes());
    b
}

/// Encodes a `DeePBSP` `SSECTORS` record (6 bytes, packed, little-endian).
fn subsector4(numsegs: u16, firstseg: u32) -> Vec<u8> {
    let mut b = Vec::with_capacity(6);
    b.extend_from_slice(&numsegs.to_le_bytes());
    b.extend_from_slice(&firstseg.to_le_bytes());
    b
}

/// Encodes a `DeePBSP` `NODES` record (32 bytes, little-endian).
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

/// The 8-byte `xNd4\0\0\0\0` signature followed by `node_bytes`.
fn nodes_lump(node_bytes: &[u8]) -> Vec<u8> {
    let mut b = b"xNd4\0\0\0\0".to_vec();
    b.extend_from_slice(node_bytes);
    b
}

/// The square's four `DeePBSP` segs.
fn square_segs4() -> Vec<u8> {
    let mut b = Vec::new();
    b.extend(seg4(0, 1, 0x0000, 0, 0, 0));
    b.extend(seg4(1, 2, 0x4000, 1, 0, 5));
    b.extend(seg4(2, 3, 0x8000, 2, 1, -3));
    b.extend(seg4(3, 0, 0xC000, 3, 0, 0));
    b
}

/// Two subsectors: segs [0,2) and [2,4).
fn square_ssectors4() -> Vec<u8> {
    let mut b = Vec::new();
    b.extend(subsector4(2, 0));
    b.extend(subsector4(2, 2));
    b
}

/// One node whose two children are the square's two subsector leaves.
fn square_nodes4() -> Vec<u8> {
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
    nodes_lump(&node)
}

fn assemble_square_with(
    extra: &[(&str, &[u8])],
    options: ParseOptions,
) -> Result<Map, MapAssembleError> {
    let bytes = common::build_doom_map_wad_with_lumps(
        "E1M1",
        /* things   */ vec![],
        /* linedefs */ square_linedefs(),
        /* sidedefs */ sidedef(),
        /* vertexes */ square_vertexes(),
        /* sectors  */ sector(),
        extra,
    );
    let wad = Wad::from_bytes(bytes).expect("parse PWAD");
    let group = wad.map_group("E1M1").expect("E1M1 group");
    Map::assemble_with_options(&wad, &group, options)
}

#[test]
fn deepbsp_map_decodes_on_the_binary_path() {
    let segs = square_segs4();
    let ssectors = square_ssectors4();
    let nodes = square_nodes4();
    let extra: &[(&str, &[u8])] = &[("SEGS", &segs), ("SSECTORS", &ssectors), ("NODES", &nodes)];

    for options in [ParseOptions::default(), ParseOptions::lenient()] {
        let map = assemble_square_with(extra, options).expect("DeePBSP decodes");
        // DeePBSP adds no new vertices: the arena is the map's four originals.
        assert_eq!(map.vertices().len(), 4, "no added vertices");
        assert_eq!(map.segs().len(), 4);
        assert_eq!(map.subsectors().len(), 2);
        assert_eq!(map.subsectors()[0].segs, 0..2);
        assert_eq!(map.subsectors()[1].segs, 2..4);
        assert_eq!(map.nodes().len(), 1);
        // Root is the last node (crate convention).
        assert_eq!(map.bsp_root(), Some(NodeIdx(0)));
        // Widened seg fields survive: 32-bit vertex indices, real linedefs.
        assert_eq!(map.segs()[2].start, VertexIdx(2));
        assert_eq!(map.segs()[2].end, VertexIdx(3));
        assert_eq!(map.segs()[1].linedef, Some(LinedefIdx(1)));
        assert_eq!(map.segs()[1].offset, 5);
        // DeePBSP has no minisegs: every seg is linedef-backed.
        assert!(map.segs().iter().all(|s| s.linedef.is_some()));
        assert!(map.warnings().is_empty(), "clean map warns nothing");
    }
}

#[test]
fn malformed_deepbsp_segs_is_fatal_in_both_modes() {
    // SEGS one byte short of a whole 16-byte record: a framing defect that is a
    // hard `Records` error in BOTH modes (the adjudicated policy), unlike the
    // ZDoom readers' lenient whole-BSP degrade.
    let mut segs = square_segs4();
    segs.truncate(segs.len() - 1);
    let ssectors = square_ssectors4();
    let nodes = square_nodes4();
    let extra: &[(&str, &[u8])] = &[("SEGS", &segs), ("SSECTORS", &ssectors), ("NODES", &nodes)];

    for options in [ParseOptions::default(), ParseOptions::lenient()] {
        let err = assemble_square_with(extra, options).expect_err("misaligned SEGS is fatal");
        assert!(
            matches!(err, MapAssembleError::Records { lump: "SEGS", .. }),
            "expected Records error for SEGS, got {err:?}"
        );
    }
}

#[test]
fn misaligned_deepbsp_nodes_records_is_fatal_in_both_modes() {
    // A `NODES` lump with the full 8-byte `xNd4` signature (so it routes to the
    // DeePBSP decoder) but a post-signature body that is not a whole multiple of
    // the 32-byte node record is a framing defect: a hard `Records` error in
    // both modes.
    let segs = square_segs4();
    let ssectors = square_ssectors4();
    let bad_nodes = nodes_lump(&[0u8; 10]); // 10 bytes < one 32-byte node record
    let extra: &[(&str, &[u8])] = &[
        ("SEGS", &segs),
        ("SSECTORS", &ssectors),
        ("NODES", &bad_nodes),
    ];

    for options in [ParseOptions::default(), ParseOptions::lenient()] {
        let err = assemble_square_with(extra, options).expect_err("misaligned NODES is fatal");
        assert!(
            matches!(err, MapAssembleError::Records { lump: "NODES", .. }),
            "expected Records error for NODES, got {err:?}"
        );
    }
}

// --- Regression: a non-DeePBSP classic map still decodes classically. ---

/// Encodes a classic `SEGS` record (12 bytes, little-endian).
fn seg_classic(v1: u16, v2: u16, angle: u16, line: u16, side: u16, offset: i16) -> Vec<u8> {
    [v1, v2, angle, line, side]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .chain(offset.to_le_bytes())
        .collect()
}

/// Encodes a classic `SSECTORS` record (4 bytes, little-endian).
fn subsector_classic(seg_count: u16, first_seg: u16) -> Vec<u8> {
    [seg_count.to_le_bytes(), first_seg.to_le_bytes()].concat()
}

/// Encodes a classic `NODES` record (28 bytes, little-endian).
fn node_classic(right_child: u16, left_child: u16) -> Vec<u8> {
    let mut b = Vec::with_capacity(28);
    for v in [0i16, 0, 8, 8] {
        b.extend_from_slice(&v.to_le_bytes());
    }
    // right + left bbox, 8 i16.
    for _ in 0..8 {
        b.extend_from_slice(&16i16.to_le_bytes());
    }
    b.extend_from_slice(&right_child.to_le_bytes());
    b.extend_from_slice(&left_child.to_le_bytes());
    b
}

#[test]
fn classic_map_still_decodes_classically() {
    // A `NODES` lump NOT starting with `xNd4` must fall through to the classic
    // 12/4/28-byte decoders, unchanged by the DeePBSP gate.
    let mut segs = Vec::new();
    segs.extend(seg_classic(0, 1, 0, 0, 0, 0));
    segs.extend(seg_classic(1, 2, 0, 1, 0, 0));
    let mut ssectors = Vec::new();
    ssectors.extend(subsector_classic(2, 0));
    // Bit 15 set -> subsector 0 for both children.
    let nodes = node_classic(0x8000, 0x8000);
    let extra: &[(&str, &[u8])] = &[("SEGS", &segs), ("SSECTORS", &ssectors), ("NODES", &nodes)];

    for options in [ParseOptions::default(), ParseOptions::lenient()] {
        let map = assemble_square_with(extra, options).expect("classic map decodes");
        assert_eq!(map.segs().len(), 2);
        assert_eq!(map.subsectors().len(), 1);
        assert_eq!(map.subsectors()[0].segs, 0..2);
        assert_eq!(map.nodes().len(), 1);
        assert_eq!(map.bsp_root(), Some(NodeIdx(0)));
        assert!(map.warnings().is_empty());
    }
}
