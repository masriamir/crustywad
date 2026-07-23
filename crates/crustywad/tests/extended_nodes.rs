//! Integration tests for wiring the `ZDoom` extended-node decoder into the map
//! assembler at its two seams: the binary `NODES`/`SSECTORS` gate dispatch and
//! the UDMF `ZNODES` routing. Covers the uncompressed `X*` dialects (ADR-0025,
//! #326) and, under the `extended-nodes-zlib` feature, their zlib-compressed
//! `Z*` twins (ADR-0025 §5, #327); with the feature off a `Z*` signature keeps
//! #199's extended-encoding gate. Each test assembles a whole [`Map`] through
//! the public API and asserts the resulting BSP arenas, in **both** strictness
//! modes.
//!
//! The extended streams are hand-built here to be consistent with the enclosing
//! map (the stream's `origVerts` matches the map's vertex count, and every seg's
//! linedef index is in range), so these exercise the happy decode path — not the
//! header-mismatch error path.

mod common;

#[cfg(feature = "nodebuild")]
use crustywad::map::build::{BuiltNodes, NodeBuildOptions, NodeFormat, build_nodes};
use crustywad::map::{ExtendedNodeError, Map, MapAssembleError, MapFormat, MapWarning};
#[cfg(feature = "nodebuild")]
use crustywad::map::{
    LinedefIdx, MapNode, MapSeg, MapSubsector, MapVertex, NodeChild, SubsectorIdx, VertexIdx,
};
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

/// Builds an on-disk compressed `Z*` lump from a full uncompressed `X*` stream:
/// strip the 4-byte `X*` tag, zlib-compress the tag-less body, and prepend the
/// plaintext `Z*` tag — the `[tag][zlib stream]` layout the decoder expects.
#[cfg(feature = "extended-nodes-zlib")]
fn zlib_lump(z_tag: [u8; 4], x_full: &[u8]) -> Vec<u8> {
    let mut lump = z_tag.to_vec();
    lump.extend(miniz_oxide::deflate::compress_to_vec_zlib(&x_full[4..], 6));
    lump
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

// --- Binary path: Z* — gated without the feature, decoded with it. ---

#[cfg(not(feature = "extended-nodes-zlib"))]
#[test]
fn zgl3_ssectors_lump_still_gates_on_the_binary_path() {
    // Without `extended-nodes-zlib`, a recognized `Z*` signature keeps #199's
    // extended-encoding gate. A minimal (tag-only) ZGL3 blob suffices: the gate
    // fires on the signature alone, before any inflate is attempted.
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

#[cfg(feature = "extended-nodes-zlib")]
#[test]
fn zgl3_ssectors_lump_decodes_on_the_binary_path_with_the_feature() {
    // With `extended-nodes-zlib`, a compressed ZGL3 lump inflates to its XGL3
    // twin's body and yields the same arenas the uncompressed twin decodes to.
    let z_lump = zlib_lump(*b"ZGL3", &xgl3_stream(*b"XGL3"));
    for options in [ParseOptions::default(), ParseOptions::lenient()] {
        let map = assemble_square_with(&[("SSECTORS", &z_lump)], options)
            .expect("compressed ZGL3 decodes");
        assert_eq!(map.vertices().len(), 4, "no added vertices");
        assert_eq!(map.segs().len(), 4);
        assert_eq!(map.subsectors().len(), 2);
        assert_eq!(map.nodes().len(), 1);
        assert_eq!(map.segs()[1].linedef, None, "miniseg survives inflate");
        assert!(map.warnings().is_empty());
    }
}

#[cfg(feature = "extended-nodes-zlib")]
#[test]
fn corrupt_zgl3_ssectors_lump_errors_through_the_binary_dispatch() {
    // A ZGL3 tag over a body that is not a zlib stream (the raw uncompressed
    // XGL3 bytes): with the feature on, the gate now routes to the compressed
    // decoder, which fails to inflate — an ExtendedNode/CorruptStream fault
    // (proving the dispatch reaches the decoder), not the old gate error.
    let blob = xgl3_stream(*b"ZGL3");
    let err = assemble_square_with(&[("SSECTORS", &blob)], ParseOptions::default())
        .expect_err("un-inflatable Z* stream is fatal in strict mode");
    assert!(matches!(
        err,
        MapAssembleError::ExtendedNode {
            dialect: "ZGL3",
            reason: ExtendedNodeError::CorruptStream,
        }
    ));

    // Lenient: the whole BSP degrades to empty arenas with a single warning.
    let map = assemble_square_with(&[("SSECTORS", &blob)], ParseOptions::lenient())
        .expect("un-inflatable Z* degrades, not fatal, in lenient mode");
    assert!(map.segs().is_empty());
    assert_eq!(map.vertices().len(), 4, "geometry survives the degrade");
    assert_eq!(map.warnings().len(), 1);
    assert!(matches!(
        map.warnings()[0],
        MapWarning::ExtendedNode {
            dialect: "ZGL3",
            reason: ExtendedNodeError::CorruptStream,
        }
    ));
}

// --- Binary path: a structurally malformed X* propagates through the gate. ---

#[test]
fn malformed_xnod_nodes_lump_strict_error_propagates_through_the_binary_dispatch() {
    // Every other binary-path test above either decodes cleanly or hits the
    // still-gated-Z* branch, neither of which ever calls
    // `decode_extended_nodes` in a way that returns `Err` — this exercises
    // that error-propagation arm of the gate dispatch itself: a tag-only
    // XNOD stream is truncated mid vertex-header, so `decode_extended_nodes`
    // fails structurally (not with a cross-reference error) and, in strict
    // mode, that `Err` propagates straight through the assembler.
    let truncated: &[u8] = b"XNOD";
    let err = assemble_square_with(&[("NODES", truncated)], ParseOptions::default())
        .expect_err("truncated XNOD stream is fatal in strict mode");
    assert!(matches!(
        err,
        MapAssembleError::ExtendedNode {
            reason: ExtendedNodeError::Truncated { .. },
            ..
        }
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

#[cfg(not(feature = "extended-nodes-zlib"))]
#[test]
fn udmf_znodes_zgl3_gates() {
    let blob = xgl3_stream(*b"ZGL3");

    // Strict: without the feature the UDMF path gates too (it previously never did).
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

#[cfg(feature = "extended-nodes-zlib")]
#[test]
fn udmf_znodes_unrecognized_signature_gates_even_with_the_feature() {
    // A `ZNODES` lump whose head is not one of the 8 known X*/Z* signatures
    // classifies to `None`, falling through to the `_` gate arm even with
    // `extended-nodes-zlib` on — distinct from `udmf_znodes_zgl3_gates`
    // above, which only gates a *recognized* `Z*` signature when the feature
    // is off.
    let blob = b"xNd4\x00\x00\x00\x00".to_vec();

    let err = assemble_udmf_square_with_znodes(&blob, ParseOptions::default())
        .expect_err("unrecognized signature gates in strict mode");
    assert!(matches!(
        err,
        MapAssembleError::UnsupportedNodeEncoding {
            lump: "ZNODES",
            signature,
        } if &signature == b"xNd4"
    ));

    // Lenient: empty BSP arenas, one warning, UDMF geometry intact.
    let map = assemble_udmf_square_with_znodes(&blob, ParseOptions::lenient())
        .expect("unrecognized signature is skipped, not fatal, in lenient mode");
    assert!(map.segs().is_empty());
    assert!(map.subsectors().is_empty());
    assert!(map.nodes().is_empty());
    assert_eq!(map.vertices().len(), 4, "geometry survives the gate");
    assert_eq!(map.linedefs().len(), 4);
    assert_eq!(map.warnings().len(), 1);
    assert!(matches!(
        map.warnings()[0],
        MapWarning::UnsupportedNodeEncoding { lump: "ZNODES" }
    ));
}

#[cfg(feature = "extended-nodes-zlib")]
#[test]
fn udmf_znodes_zgl3_decodes_with_the_feature() {
    // With the feature, a compressed ZGL3 `ZNODES` lump inflates and decodes to
    // the same arenas the uncompressed twin yields on the UDMF path.
    let z_lump = zlib_lump(*b"ZGL3", &xgl3_stream(*b"XGL3"));
    for options in [ParseOptions::default(), ParseOptions::lenient()] {
        let map = assemble_udmf_square_with_znodes(&z_lump, options)
            .expect("UDMF compressed ZGL3 decodes");
        assert_eq!(map.format(), MapFormat::Udmf);
        assert_eq!(map.vertices().len(), 4);
        assert_eq!(map.segs().len(), 4);
        assert_eq!(map.subsectors().len(), 2);
        assert_eq!(map.nodes().len(), 1);
        assert_eq!(map.segs()[1].linedef, None, "miniseg carries through UDMF");
        assert!(map.warnings().is_empty());
    }
}

#[cfg(feature = "extended-nodes-zlib")]
#[test]
fn corrupt_zgl3_znodes_lump_errors_through_the_udmf_dispatch() {
    // Mirrors `corrupt_zgl3_ssectors_lump_errors_through_the_binary_dispatch`
    // for the UDMF `ZNODES` seam: an un-inflatable `Z*` stream must propagate
    // through the UDMF decode arm's own error path too, not just the binary
    // path's.
    let blob = xgl3_stream(*b"ZGL3");
    let err = assemble_udmf_square_with_znodes(&blob, ParseOptions::default())
        .expect_err("un-inflatable Z* stream is fatal in strict mode");
    assert!(matches!(
        err,
        MapAssembleError::ExtendedNode {
            dialect: "ZGL3",
            reason: ExtendedNodeError::CorruptStream,
        }
    ));

    // Lenient: the whole BSP degrades to empty arenas with a single warning.
    let map = assemble_udmf_square_with_znodes(&blob, ParseOptions::lenient())
        .expect("un-inflatable Z* degrades, not fatal, in lenient mode");
    assert!(map.segs().is_empty());
    assert_eq!(map.vertices().len(), 4, "geometry survives the degrade");
    assert_eq!(map.warnings().len(), 1);
    assert!(matches!(
        map.warnings()[0],
        MapWarning::ExtendedNode {
            dialect: "ZGL3",
            reason: ExtendedNodeError::CorruptStream,
        }
    ));
}

// --- Write path: `BuiltNodes::to_extended_lump_bytes` golden bytes,
// round-trips through the reader, the full `build_nodes` chain, ZNOD, and a
// build->write->read proptest (#323 Task 5). ---

/// The `BuiltNodes` equivalent of `xnod_stream()`: 1 split vertex (centre), 1
/// subsector of 2 segs, 1 node with both children subsector 0. Angle/offset are
/// arbitrary — XNOD stores neither.
#[cfg(feature = "nodebuild")]
fn xnod_built() -> BuiltNodes {
    // `BuiltNodes` is `#[non_exhaustive]`, so an integration test (a separate
    // crate) must go through the public `BuiltNodes::new` constructor rather
    // than a struct literal.
    BuiltNodes::new(
        vec![MapVertex { x: 32.0, y: 32.0 }],
        vec![
            MapSeg {
                start: VertexIdx(0),
                end: VertexIdx(1),
                angle: 0,
                linedef: Some(LinedefIdx(0)),
                direction: 0,
                offset: 0,
            },
            MapSeg {
                start: VertexIdx(1),
                end: VertexIdx(4),
                angle: 0,
                linedef: Some(LinedefIdx(1)),
                direction: 0,
                offset: 0,
            },
        ],
        vec![MapSubsector {
            segs: 0..2,
            leafs: 0..0,
        }],
        vec![MapNode {
            x: 32,
            y: 0,
            dx: 0,
            dy: 64,
            right_bbox: [64, 0, 0, 64],
            left_bbox: [64, 0, 0, 64],
            right: NodeChild::Subsector(SubsectorIdx(0)),
            left: NodeChild::Subsector(SubsectorIdx(0)),
        }],
    )
}

#[cfg(feature = "nodebuild")]
#[test]
fn to_extended_lump_bytes_matches_the_xnod_reader_fixture() {
    let bytes = xnod_built()
        .to_extended_lump_bytes(4, false)
        .expect("serializes");
    assert_eq!(
        bytes,
        xnod_stream(),
        "writer output is byte-identical to the read fixture"
    );
}

#[cfg(feature = "nodebuild")]
#[test]
fn xnod_writer_output_round_trips_through_the_reader() {
    let built = xnod_built();
    let stream = built.to_extended_lump_bytes(4, false).expect("serializes");
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        let map = assemble_square_with(&[("NODES", &stream)], options).expect("XNOD decodes");
        // Stored fields survive (angle/offset are re-derived on read, excluded).
        assert_eq!(map.vertices().len(), 5, "4 orig + 1 split");
        assert_eq!(map.segs().len(), 2);
        assert_eq!(map.segs()[0].start, VertexIdx(0));
        assert_eq!(map.segs()[0].end, VertexIdx(1));
        assert_eq!(map.segs()[0].linedef, Some(LinedefIdx(0)));
        assert_eq!(map.segs()[0].direction, 0);
        assert_eq!(map.segs()[1].end, VertexIdx(4));
        assert_eq!(map.subsectors().len(), 1);
        assert_eq!(map.subsectors()[0].segs, 0..2);
        assert_eq!(map.nodes().len(), 1);
        assert_eq!(
            (
                map.nodes()[0].x,
                map.nodes()[0].y,
                map.nodes()[0].dx,
                map.nodes()[0].dy
            ),
            (32, 0, 0, 64)
        );
        assert_eq!(map.nodes()[0].right_bbox, [64, 0, 0, 64]);
        assert!(matches!(
            map.nodes()[0].right,
            NodeChild::Subsector(SubsectorIdx(0))
        ));
    }
}

#[cfg(feature = "nodebuild")]
#[test]
fn build_nodes_xnod_round_trips_the_square() {
    let map = assemble_square_with(&[], ParseOptions::strict()).expect("square assembles");
    let mut opts = NodeBuildOptions::strict();
    opts.format = NodeFormat::Xnod;
    let (built, _warnings) = build_nodes(&map, &opts).expect("builds");
    let stream = built
        .to_extended_lump_bytes(map.vertices().len(), false)
        .expect("serializes");

    let read_back =
        assemble_square_with(&[("NODES", &stream)], ParseOptions::strict()).expect("reads");
    assert_eq!(read_back.segs().len(), built.segs.len());
    assert_eq!(read_back.subsectors().len(), built.subsectors.len());
    assert_eq!(read_back.nodes().len(), built.nodes.len());
    for (r, b) in read_back.segs().iter().zip(&built.segs) {
        assert_eq!(r.start, b.start);
        assert_eq!(r.end, b.end);
        assert_eq!(r.linedef, b.linedef);
        assert_eq!(r.direction, b.direction);
        // angle/offset are re-derived on read — excluded (Global Constraint 3).
    }

    // The chosen node format changes no in-bounds `BuiltNodes` output — the
    // classic and XNOD builds agree on the same square.
    let classic_opts = NodeBuildOptions::strict();
    let (classic_built, _classic_warnings) = build_nodes(&map, &classic_opts).expect("builds");
    assert_eq!(
        classic_built, built,
        "NodeFormat only changes serialization, not the built BSP"
    );
}

#[cfg(all(feature = "nodebuild", feature = "extended-nodes-zlib"))]
#[test]
fn to_extended_lump_bytes_znod_matches_the_zlib_fixture() {
    let znod = xnod_built()
        .to_extended_lump_bytes(4, true)
        .expect("compresses");
    assert_eq!(
        znod,
        zlib_lump(*b"ZNOD", &xnod_stream()),
        "ZNOD is byte-identical to the fixture"
    );
}

#[cfg(all(feature = "nodebuild", feature = "extended-nodes-zlib"))]
#[test]
fn znod_writer_output_round_trips_through_the_reader() {
    let stream = xnod_built()
        .to_extended_lump_bytes(4, true)
        .expect("compresses");
    let map =
        assemble_square_with(&[("NODES", &stream)], ParseOptions::strict()).expect("ZNOD decodes");
    assert_eq!(map.vertices().len(), 5);
    assert_eq!(map.segs().len(), 2);
    assert_eq!(map.subsectors().len(), 1);
    assert_eq!(map.nodes().len(), 1);
}

#[cfg(feature = "nodebuild")]
proptest::proptest! {
    #[test]
    fn xnod_seg_and_subsector_arenas_survive_round_trip(
        // up to 2 split verts (whole coords in a small window) and up to 4
        // subsectors of 1..=3 segs each; seg indices resolve against 4 orig
        // verts + the split verts; linedef indices are 0..4.
        split_coords in proptest::collection::vec((-64i16..=64, -64i16..=64), 0..=2),
        runs in proptest::collection::vec(1usize..=3, 1..=4),
        seed in proptest::array::uniform32(0u8..),
    ) {
        let combined = 4 + split_coords.len();
        let split_vertices: Vec<MapVertex> = split_coords.iter()
            .map(|&(x, y)| MapVertex { x: f64::from(x), y: f64::from(y) }).collect();
        // Build segs run by run, deterministically from `seed` (no Date/rand).
        let mut segs = Vec::new();
        let mut subsectors = Vec::new();
        let mut k = 0usize;
        for &len in &runs {
            let start = segs.len();
            for _ in 0..len {
                let v1 = usize::from(seed[k % 32]) % combined; k += 1;
                let v2 = usize::from(seed[k % 32]) % combined; k += 1;
                let line = usize::from(seed[k % 32]) % 4; k += 1;
                let dir = u16::from(seed[k % 32] & 1); k += 1;
                segs.push(MapSeg { start: VertexIdx(v1), end: VertexIdx(v2), angle: 0, linedef: Some(LinedefIdx(line)), direction: dir, offset: 0 });
            }
            subsectors.push(MapSubsector { segs: start..segs.len(), leafs: 0..0 });
        }
        let built = BuiltNodes::new(split_vertices, segs, subsectors, Vec::new());
        let stream = built.to_extended_lump_bytes(4, false).expect("serializes");
        let map = assemble_square_with(&[("NODES", &stream)], ParseOptions::strict()).expect("decodes");

        proptest::prop_assert_eq!(map.segs().len(), built.segs.len());
        proptest::prop_assert_eq!(map.subsectors().len(), built.subsectors.len());
        for (r, b) in map.segs().iter().zip(&built.segs) {
            proptest::prop_assert_eq!(r.start, b.start);
            proptest::prop_assert_eq!(r.end, b.end);
            proptest::prop_assert_eq!(r.linedef, b.linedef);
            proptest::prop_assert_eq!(r.direction, b.direction);
        }
        for (r, b) in map.subsectors().iter().zip(&built.subsectors) {
            proptest::prop_assert_eq!(r.segs.clone(), b.segs.clone());
        }
    }
}
