//! Integration tests for classic GL-node reading during map assembly (#324).
//!
//! These exercise the public API end to end: `Wad::from_bytes` →
//! `Map::assemble`/`assemble_with_options` → the `gl_*` accessors. A Doom-format
//! map carrying a `GL_<name>` group must populate the additive GL arenas without
//! disturbing the vanilla `SEGS`/`SSECTORS`/`NODES` graph; a refused version (V4)
//! must leave the GL arenas empty (warning in Lenient, error in Strict); and a
//! map with no GL group must leave the arenas empty with no GL warnings.

mod common;

use common::build_doom_map_wad_with_lumps;
use crustywad::map::{Map, MapWarning};
use crustywad::{ParseOptions, Strictness, Wad};

// --- Byte-layout builders for the hand-crafted fixtures ------------------------

/// One classic 4-byte `VERTEXES` record (`i16 x, i16 y`).
fn vertex(x: i16, y: i16) -> Vec<u8> {
    [x.to_le_bytes(), y.to_le_bytes()].concat()
}

/// One classic 14-byte Doom `LINEDEFS` record.
fn linedef(start: u16, end: u16, right: u16, left: u16) -> Vec<u8> {
    [start, end, 0, 0, 0, right, left]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect()
}

/// One classic 30-byte `SIDEDEFS` record facing `sector`.
fn sidedef(sector: u16) -> Vec<u8> {
    let mut b = vec![0u8; 4]; // x_offset, y_offset
    b.extend_from_slice(b"-\0\0\0\0\0\0\0"); // upper
    b.extend_from_slice(b"-\0\0\0\0\0\0\0"); // lower
    b.extend_from_slice(b"WALL\0\0\0\0"); // middle
    b.extend_from_slice(&sector.to_le_bytes());
    b
}

/// One classic 26-byte `SECTORS` record.
fn sector() -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(&0i16.to_le_bytes()); // floor
    b.extend_from_slice(&128i16.to_le_bytes()); // ceiling
    b.extend_from_slice(b"FLOOR\0\0\0"); // floor flat
    b.extend_from_slice(b"CEIL\0\0\0\0"); // ceiling flat
    b.extend_from_slice(&160i16.to_le_bytes()); // light
    b.extend_from_slice(&0i16.to_le_bytes()); // special
    b.extend_from_slice(&0i16.to_le_bytes()); // tag
    b
}

/// One classic 12-byte vanilla `SEGS` record.
fn vanilla_seg(start: u16, end: u16, ld: u16) -> Vec<u8> {
    [start, end, 0, ld, 0]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .chain(0i16.to_le_bytes())
        .collect()
}

/// One classic 4-byte vanilla `SSECTORS` record (`u16 count, u16 first`).
fn vanilla_ssector(count: u16, first: u16) -> Vec<u8> {
    [count.to_le_bytes(), first.to_le_bytes()].concat()
}

/// One classic 28-byte `NODES`/`GL_NODES` (V2/V3) record with zeroed geometry
/// and both children pointing at subsector `ssec`.
fn node_28(right_ssec: u16, left_ssec: u16) -> Vec<u8> {
    let mut b = vec![0u8; 8 + 8 + 8]; // x,y,dx,dy (4×i16) + right_bbox[4] + left_bbox[4]
    b.extend_from_slice(&(0x8000 | right_ssec).to_le_bytes());
    b.extend_from_slice(&(0x8000 | left_ssec).to_le_bytes());
    b
}

/// One 8-byte `GL_VERT` record: 16.16 fixed-point `(x, y)`.
fn gl_vert(x: f64, y: f64) -> Vec<u8> {
    #[allow(clippy::cast_possible_truncation)]
    let (xf, yf) = ((x * 65536.0) as i32, (y * 65536.0) as i32);
    [xf.to_le_bytes(), yf.to_le_bytes()].concat()
}

/// One 10-byte V2 `GL_SEGS` record (`u16 v1, v2, linedef, side, partner`).
fn gl_seg_v2(v1: u16, v2: u16, linedef: u16, side: u16, partner: u16) -> Vec<u8> {
    [v1, v2, linedef, side, partner]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect()
}

/// One 4-byte V2 `GL_SSECT` record (`u16 count, u16 first`).
fn gl_ssect_v2(count: u16, first: u16) -> Vec<u8> {
    [count.to_le_bytes(), first.to_le_bytes()].concat()
}

// --- Fixture assembly ----------------------------------------------------------

/// The raw bytes of the shared Doom map body: 3 real vertices, 1 linedef, and a
/// single-node vanilla BSP (`SEGS`/`SSECTORS`/`NODES`), so the additive-GL
/// assertions can check the vanilla graph is left untouched.
struct BaseMap {
    things: Vec<u8>,
    linedefs: Vec<u8>,
    sidedefs: Vec<u8>,
    vertexes: Vec<u8>,
    sectors: Vec<u8>,
    segs: Vec<u8>,
    ssectors: Vec<u8>,
    nodes: Vec<u8>,
}

fn base_map() -> BaseMap {
    BaseMap {
        things: Vec::new(),
        linedefs: linedef(0, 1, 0, 0xFFFF),
        sidedefs: sidedef(0),
        vertexes: [vertex(0, 0), vertex(64, 0), vertex(64, 64)].concat(),
        sectors: sector(),
        segs: vanilla_seg(0, 1, 0),
        ssectors: vanilla_ssector(1, 0),
        nodes: node_28(0, 0),
    }
}

/// The four raw `GL_*` data lumps of one classic GL node group.
struct GlLumps {
    vert: Vec<u8>,
    segs: Vec<u8>,
    ssect: Vec<u8>,
    nodes: Vec<u8>,
}

/// A minimal valid V2 GL group's four lumps.
///
/// `GL_VERT` = `gNd2` + 1 GL vertex; `GL_SEGS` = 2 V2 segs (one references the
/// GL vertex via the `0x8000` high bit and links a partner); `GL_SSECT` = 1
/// subsector spanning both segs; `GL_NODES` = 1 node whose children are the
/// single GL subsector.
fn v2_gl_lumps() -> GlLumps {
    let mut vert = b"gNd2".to_vec();
    vert.extend(gl_vert(128.0, 64.0));

    // Seg 0: normal vertex 0 -> GL vertex 0 (0x8000), on linedef 0, partner 1.
    // Seg 1: GL vertex 0 -> normal vertex 1, miniseg (linedef 0xFFFF), partner 0.
    let segs = [
        gl_seg_v2(0, 0x8000, 0, 0, 1),
        gl_seg_v2(0x8000, 1, 0xFFFF, 1, 0),
    ]
    .concat();

    GlLumps {
        vert,
        segs,
        ssect: gl_ssect_v2(2, 0), // 2 segs starting at 0
        nodes: node_28(0, 0),     // both children = GL subsector 0
    }
}

/// Builds a Doom-format WAD for `name` with the vanilla BSP present and,
/// optionally, a `GL_<name>` group whose four lumps are `gl`.
fn build_wad_bytes(name: &str, gl: Option<&GlLumps>) -> Vec<u8> {
    let map = base_map();

    let marker = format!("GL_{name}");
    let mut extra: Vec<(&str, &[u8])> = vec![
        ("SEGS", &map.segs),
        ("SSECTORS", &map.ssectors),
        ("NODES", &map.nodes),
    ];
    if let Some(gl) = gl {
        extra.push((marker.as_str(), &[]));
        extra.push(("GL_VERT", &gl.vert));
        extra.push(("GL_SEGS", &gl.segs));
        extra.push(("GL_SSECT", &gl.ssect));
        extra.push(("GL_NODES", &gl.nodes));
    }

    build_doom_map_wad_with_lumps(
        name,
        map.things.clone(),
        map.linedefs.clone(),
        map.sidedefs.clone(),
        map.vertexes.clone(),
        map.sectors.clone(),
        &extra,
    )
}

fn assemble(
    bytes: &[u8],
    name: &str,
    strictness: Strictness,
) -> Result<Map, crustywad::map::MapAssembleError> {
    let wad = Wad::from_bytes(bytes).expect("valid wad");
    let group = wad.map_group(name).expect("map group");
    Map::assemble_with_options(
        &wad,
        &group,
        ParseOptions {
            strictness,
            ..ParseOptions::default()
        },
    )
}

// --- Tests ---------------------------------------------------------------------

#[test]
fn v2_gl_group_populates_additive_arenas() {
    let bytes = build_wad_bytes("MAP01", Some(&v2_gl_lumps()));
    let map = assemble(&bytes, "MAP01", Strictness::Strict).expect("assembles");

    // GL arenas populated.
    assert_eq!(map.gl_vertices().len(), 1, "one GL vertex");
    assert_eq!(map.gl_segs().len(), 2, "two GL segs");
    assert_eq!(map.gl_subsectors().len(), 1, "one GL subsector");
    assert_eq!(map.gl_nodes().len(), 1, "one GL node");

    // Vanilla BSP untouched — GL is additive, not a replacement.
    assert_eq!(map.segs().len(), 1, "vanilla segs unchanged");
    assert_eq!(map.subsectors().len(), 1, "vanilla subsectors unchanged");
    assert_eq!(map.nodes().len(), 1, "vanilla nodes unchanged");

    // No GL warnings on a clean strict assembly.
    assert!(
        !map.warnings().iter().any(|w| matches!(
            w,
            MapWarning::GlNodesRefused { .. } | MapWarning::GlNodesDegraded
        )),
        "no GL warnings expected"
    );
}

#[test]
fn v4_gl_group_refused_lenient_warns_and_keeps_vanilla() {
    let mut gl = v2_gl_lumps();
    gl.vert = b"gNd4".to_vec(); // refuse: V4 GL_VERT magic
    let bytes = build_wad_bytes("MAP01", Some(&gl));

    let map = assemble(&bytes, "MAP01", Strictness::Lenient).expect("lenient assembles");

    // GL arenas empty on refusal.
    assert!(map.gl_vertices().is_empty());
    assert!(map.gl_segs().is_empty());
    assert!(map.gl_subsectors().is_empty());
    assert!(map.gl_nodes().is_empty());

    // Vanilla map intact.
    assert_eq!(map.segs().len(), 1);
    assert_eq!(map.nodes().len(), 1);

    // A refusal warning is surfaced.
    assert!(
        map.warnings()
            .iter()
            .any(|w| matches!(w, MapWarning::GlNodesRefused { version: 4 })),
        "expected GlNodesRefused {{ version: 4 }}, got {:?}",
        map.warnings()
    );
}

#[test]
fn v4_gl_group_refused_strict_errors() {
    let mut gl = v2_gl_lumps();
    gl.vert = b"gNd4".to_vec();
    let bytes = build_wad_bytes("MAP01", Some(&gl));

    let err = assemble(&bytes, "MAP01", Strictness::Strict).expect_err("strict refuses V4");
    assert!(
        matches!(
            err,
            crustywad::map::MapAssembleError::UnsupportedGlNodeVersion { .. }
        ),
        "expected UnsupportedGlNodeVersion, got {err:?}"
    );
}

#[test]
fn no_gl_group_leaves_arenas_empty() {
    let bytes = build_wad_bytes("MAP01", None);
    let map = assemble(&bytes, "MAP01", Strictness::Strict).expect("assembles");

    assert!(map.gl_vertices().is_empty());
    assert!(map.gl_segs().is_empty());
    assert!(map.gl_subsectors().is_empty());
    assert!(map.gl_nodes().is_empty());

    // Vanilla map intact.
    assert_eq!(map.segs().len(), 1);
    assert_eq!(map.nodes().len(), 1);

    // No GL warnings.
    assert!(
        !map.warnings().iter().any(|w| matches!(
            w,
            MapWarning::GlNodesRefused { .. } | MapWarning::GlNodesDegraded
        )),
        "no GL warnings expected"
    );
}

/// `Map::assemble_with_gl_source(.., gl_wad: None, ..)` must be byte-identical
/// to `Map::assemble_with_options` (#342 Task 2 — the new `gl_wad` parameter
/// is threaded through but not yet consulted; Task 3 wires the `.gwa`
/// lookup/precedence). Reuses the same in-WAD V2 GL group fixture as
/// `v2_gl_group_populates_additive_arenas` above.
#[test]
fn assemble_with_gl_source_none_matches_assemble_with_options() {
    let bytes = build_wad_bytes("MAP01", Some(&v2_gl_lumps()));
    let wad = Wad::from_bytes(bytes).expect("valid wad");
    let group = wad.map_group("MAP01").expect("map group");

    let a = Map::assemble_with_options(&wad, &group, ParseOptions::strict()).expect("assembles");
    let b = Map::assemble_with_gl_source(&wad, &group, None, ParseOptions::strict())
        .expect("assembles");

    assert_eq!(a.gl_segs(), b.gl_segs());
    assert_eq!(a.gl_nodes().len(), b.gl_nodes().len());
    assert_eq!(a.nodes().len(), b.nodes().len());
}
