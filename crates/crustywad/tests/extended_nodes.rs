//! Integration tests for wiring the uncompressed `ZDoom` extended-node decoder
//! (ADR-0025, #326) into the map assembler at its two seams: the binary
//! `NODES`/`SSECTORS` gate dispatch and the UDMF `ZNODES` routing. Each test
//! assembles a whole [`Map`] through the public API and asserts the resulting
//! BSP arenas, in **both** strictness modes.
//!
//! The extended streams are hand-built here to be consistent with the enclosing
//! map (the stream's `origVerts` matches the map's vertex count, and every seg's
//! linedef index is in range), so these exercise the happy decode path — not the
//! header-mismatch error path.

mod common;

use crustywad::map::{Map, MapAssembleError, MapFormat, MapWarning};
use crustywad::{ParseOptions, Wad};

// --- A chainable little-endian byte-stream builder for the node streams. ---

#[derive(Default)]
struct Buf(Vec<u8>);

impl Buf {
    fn tag(mut self, t: [u8; 4]) -> Self {
        self.0.extend_from_slice(&t);
        self
    }
    fn u32(mut self, v: u32) -> Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    fn i32(mut self, v: i32) -> Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    fn i16(mut self, v: i16) -> Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    fn u16(mut self, v: u16) -> Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    fn u8(mut self, v: u8) -> Self {
        self.0.push(v);
        self
    }
    fn build(self) -> Vec<u8> {
        self.0
    }
}

// --- Classic binary map-record encoders (a 64x64 square). ---

fn vertex(x: i16, y: i16) -> Vec<u8> {
    [x.to_le_bytes(), y.to_le_bytes()].concat()
}

/// The four corners of a 64x64 square, matching the stream's `origVerts = 4`.
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

/// Four one-sided edge linedefs of the square (0->1->2->3->0), so seg linedef
/// indices 0..=3 are all in range.
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

// --- Extended-node stream fixtures (consistent with the square map). ---

/// An `XNOD` stream over the square: 4 original vertices, 1 node-builder-added
/// vertex (the centre), 1 subsector of 2 explicit-`v2` segs, and 1 node whose
/// two children are that subsector. Seg 1's `v2` is the added centre vertex, so
/// decoding it exercises the combined (existing + new) vertex arena.
fn xnod_stream() -> Vec<u8> {
    Buf::default()
        .tag(*b"XNOD")
        .u32(4) // origVerts
        .u32(1) // newVerts
        // new vertex 4: centre (32, 32) in 16.16 fixed-point
        .i32(32 * 65536)
        .i32(32 * 65536)
        .u32(1) // numSubsectors
        .u32(2) // ss0 segCount
        .u32(2) // numSegs
        // seg0: v1=0, v2=1, line=0, side=0
        .u32(0)
        .u32(1)
        .u16(0)
        .u8(0)
        // seg1: v1=1, v2=4 (the new centre vertex), line=1, side=0
        .u32(1)
        .u32(4)
        .u16(1)
        .u8(0)
        .u32(1) // numNodes
        // i16 partition + 8x i16 bbox
        .i16(32)
        .i16(0)
        .i16(0)
        .i16(64)
        .i16(64)
        .i16(0)
        .i16(0)
        .i16(64)
        .i16(64)
        .i16(0)
        .i16(0)
        .i16(64)
        // children: both subsector 0 (bit 31 set)
        .u32(0x8000_0000)
        .u32(0x8000_0000)
        .build()
}

/// An `XGL3` stream over the square: 4 original vertices, no added vertices, 2
/// subsectors (2 segs each), 1 node with children subsector 0 and 1. Seg 1 is a
/// miniseg (its 32-bit linedef field is the `0xFFFF_FFFF` sentinel).
fn xgl3_stream(tag: [u8; 4]) -> Vec<u8> {
    let mut b = Buf::default()
        .tag(tag)
        .u32(4) // origVerts
        .u32(0) // newVerts
        .u32(2) // numSubsectors
        .u32(2) // ss0 segCount
        .u32(2) // ss1 segCount
        .u32(4); // numSegs
    // GL segs: u32 v1, u32 partner, u32 line, u8 side. v2 is implicit.
    let lines = [0u32, 0xFFFF_FFFF /* miniseg */, 2, 3];
    for (i, line) in lines.iter().enumerate() {
        b = b
            .u32(u32::try_from(i).unwrap()) // v1
            .u32(0xFFFF_FFFF) // partner = none
            .u32(*line) // linedef (0xFFFFFFFF = miniseg)
            .u8(0); // side
    }
    b.u32(1) // numNodes
        // i32 16.16 fixed-point partition
        .i32(32 * 65536)
        .i32(0)
        .i32(0)
        .i32(64 * 65536)
        // right bbox (i16 x4)
        .i16(64)
        .i16(0)
        .i16(0)
        .i16(64)
        // left bbox (i16 x4)
        .i16(64)
        .i16(0)
        .i16(0)
        .i16(64)
        // children: subsector 0 and subsector 1
        .u32(0x8000_0000)
        .u32(0x8000_0001)
        .build()
}

// --- Binary path: positive decodes. ---

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
fn xnod_nodes_lump_decodes_on_the_binary_path() {
    let stream = xnod_stream();
    for options in [ParseOptions::default(), ParseOptions::lenient()] {
        let map = assemble_square_with(&[("NODES", &stream)], options).expect("XNOD decodes");
        // One node-builder vertex was appended to the four originals.
        assert_eq!(map.vertices().len(), 5, "vertex arena grew by newVerts");
        assert_eq!(map.segs().len(), 2);
        assert_eq!(map.subsectors().len(), 1);
        assert_eq!(map.subsectors()[0].segs, 0..2);
        assert_eq!(map.nodes().len(), 1);
        // Root is the last node (crate convention).
        assert_eq!(map.bsp_root(), Some(crustywad::map::NodeIdx(0)));
        // seg1's explicit v2 is the appended centre vertex (index 4).
        assert_eq!(map.segs()[1].end, crustywad::map::VertexIdx(4));
        // XNOD has no minisegs: every seg is linedef-backed.
        assert!(map.segs().iter().all(|s| s.linedef.is_some()));
        assert!(map.warnings().is_empty(), "clean stream warns nothing");
    }
}

#[test]
fn xgl3_ssectors_lump_decodes_with_miniseg_on_the_binary_path() {
    let stream = xgl3_stream(*b"XGL3");
    for options in [ParseOptions::default(), ParseOptions::lenient()] {
        let map = assemble_square_with(&[("SSECTORS", &stream)], options).expect("XGL3 decodes");
        assert_eq!(map.vertices().len(), 4, "no added vertices");
        assert_eq!(map.segs().len(), 4);
        assert_eq!(map.subsectors().len(), 2);
        assert_eq!(map.nodes().len(), 1);
        assert_eq!(map.bsp_root(), Some(crustywad::map::NodeIdx(0)));
        // Seg 1 is the miniseg: no backing linedef, zero offset.
        assert_eq!(map.segs()[1].linedef, None);
        assert_eq!(map.segs()[1].offset, 0);
        // Its linedef-backed neighbours still resolve.
        assert_eq!(map.segs()[0].linedef, Some(crustywad::map::LinedefIdx(0)));
        assert_eq!(map.segs()[3].linedef, Some(crustywad::map::LinedefIdx(3)));
        assert!(map.warnings().is_empty());
    }
}

// --- Binary path: a still-gated Z* keeps the extended-encoding gate. ---

#[test]
fn zgl3_ssectors_lump_still_gates_on_the_binary_path() {
    // A minimal (tag-only) ZGL3 blob: recognized, but this build cannot decode it.
    let blob = xgl3_stream(*b"ZGL3");

    // Strict: the gate rejects with UnsupportedNodeEncoding.
    let err = assemble_square_with(&[("SSECTORS", &blob)], ParseOptions::default())
        .expect_err("Z* gates in strict mode");
    assert!(matches!(
        err,
        MapAssembleError::UnsupportedNodeEncoding {
            lump: "SSECTORS",
            signature,
        } if &signature == b"ZGL3"
    ));

    // Lenient: BSP arenas left empty, one warning, geometry intact.
    let map = assemble_square_with(&[("SSECTORS", &blob)], ParseOptions::lenient())
        .expect("Z* is skipped, not fatal, in lenient mode");
    assert!(map.segs().is_empty());
    assert!(map.subsectors().is_empty());
    assert!(map.nodes().is_empty());
    assert_eq!(map.vertices().len(), 4, "geometry survives the gate");
    assert_eq!(map.linedefs().len(), 4);
    assert_eq!(map.warnings().len(), 1);
    assert!(matches!(
        map.warnings()[0],
        MapWarning::UnsupportedNodeEncoding { lump: "SSECTORS" }
    ));
}

// --- UDMF path: ZNODES routing (positive decode + gate). ---

/// A UDMF square: 4 vertices, 4 one-sided linedefs, 1 sidedef, 1 sector —
/// consistent with the extended streams' `origVerts = 4` and linedef indices.
const UDMF_SQUARE: &str = concat!(
    "namespace = \"zdoom\";\n",
    "vertex { x = 0.0; y = 0.0; }\n",
    "vertex { x = 64.0; y = 0.0; }\n",
    "vertex { x = 64.0; y = 64.0; }\n",
    "vertex { x = 0.0; y = 64.0; }\n",
    "linedef { v1 = 0; v2 = 1; sidefront = 0; }\n",
    "linedef { v1 = 1; v2 = 2; sidefront = 0; }\n",
    "linedef { v1 = 2; v2 = 3; sidefront = 0; }\n",
    "linedef { v1 = 3; v2 = 0; sidefront = 0; }\n",
    "sidedef { sector = 0; }\n",
    "sector { texturefloor = \"F\"; textureceiling = \"C\"; }\n",
);

fn assemble_udmf_square_with_znodes(
    znodes: &[u8],
    options: ParseOptions,
) -> Result<Map, MapAssembleError> {
    let bytes = common::build_named_lumps(&[
        ("MAP01", vec![]),
        ("TEXTMAP", UDMF_SQUARE.as_bytes().to_vec()),
        ("ZNODES", znodes.to_vec()),
        ("ENDMAP", vec![]),
    ]);
    let wad = Wad::from_bytes(bytes).expect("parse PWAD");
    let group = wad.map_group("MAP01").expect("MAP01 group");
    Map::assemble_with_options(&wad, &group, options)
}

#[test]
fn udmf_znodes_xgl3_decodes() {
    let stream = xgl3_stream(*b"XGL3");
    for options in [ParseOptions::default(), ParseOptions::lenient()] {
        let map = assemble_udmf_square_with_znodes(&stream, options).expect("UDMF XGL3 decodes");
        assert_eq!(map.format(), MapFormat::Udmf);
        assert_eq!(map.vertices().len(), 4);
        assert_eq!(map.segs().len(), 4);
        assert_eq!(map.subsectors().len(), 2);
        assert_eq!(map.nodes().len(), 1);
        assert_eq!(map.bsp_root(), Some(crustywad::map::NodeIdx(0)));
        assert_eq!(map.segs()[1].linedef, None, "miniseg carries through UDMF");
        assert!(map.warnings().is_empty());
    }
}

#[test]
fn udmf_znodes_zgl3_gates() {
    let blob = xgl3_stream(*b"ZGL3");

    // Strict: the UDMF path now gates too (it previously never did).
    let err = assemble_udmf_square_with_znodes(&blob, ParseOptions::default())
        .expect_err("Z* gates on the UDMF path in strict mode");
    assert!(matches!(
        err,
        MapAssembleError::UnsupportedNodeEncoding {
            lump: "ZNODES",
            signature,
        } if &signature == b"ZGL3"
    ));

    // Lenient: empty BSP arenas, one warning, UDMF geometry intact.
    let map = assemble_udmf_square_with_znodes(&blob, ParseOptions::lenient())
        .expect("Z* is skipped, not fatal, on the lenient UDMF path");
    assert!(map.segs().is_empty());
    assert!(map.subsectors().is_empty());
    assert!(map.nodes().is_empty());
    assert_eq!(map.vertices().len(), 4);
    assert_eq!(map.linedefs().len(), 4);
    assert_eq!(map.warnings().len(), 1);
    assert!(matches!(
        map.warnings()[0],
        MapWarning::UnsupportedNodeEncoding { lump: "ZNODES" }
    ));
}
