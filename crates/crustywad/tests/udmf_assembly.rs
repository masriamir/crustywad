//! Integration tests for UDMF (`TEXTMAP`) map assembly (ADR-0017): the full
//! `TEXTMAP` -> [`Map`] pipeline exercised through the public API, across the
//! scenario matrix from strict/lenient recovery, dangling references,
//! out-of-range fields, and missing/malformed `TEXTMAP` data.

mod common;

use crustywad::map::{Map, MapAssembleError, MapFormat, MapWarning, SidedefIdx, VertexIdx};
use crustywad::{ParseOptions, Wad};

/// A complete one-of-each-block UDMF map: 2 vertices, 1 linedef (one-sided),
/// 1 sidedef, 1 sector, 1 thing.
const FULL_MAP: &str = concat!(
    "namespace = \"doom\";\n",
    "vertex { x = 0.0; y = 0.0; }\n",
    "vertex { x = 64.0; y = 0.0; }\n",
    "linedef { v1 = 0; v2 = 1; sidefront = 0; }\n",
    "sidedef { sector = 0; }\n",
    "sector { texturefloor = \"F\"; textureceiling = \"C\"; }\n",
    "thing { x = 16.0; y = 16.0; type = 1; }\n",
);

#[test]
fn full_one_of_each_block_map_assembles() {
    let bytes = common::build_named_lumps(&[
        ("MAP01", vec![]),
        ("TEXTMAP", FULL_MAP.as_bytes().to_vec()),
        ("ENDMAP", vec![]),
    ]);
    let wad = Wad::from_bytes_with_options(bytes, ParseOptions::default()).unwrap();
    let group = wad.map_group("MAP01").unwrap();
    let map = Map::assemble_with_options(&wad, &group, ParseOptions::default()).unwrap();

    assert_eq!(map.namespace(), Some("doom"));
    assert_eq!(map.format(), MapFormat::Udmf);
    assert_eq!(map.vertices().len(), 2);
    assert_eq!(map.linedefs().len(), 1);
    assert_eq!(map.sidedefs().len(), 1);
    assert_eq!(map.sectors().len(), 1);
    assert_eq!(map.things().len(), 1);

    let l = &map.linedefs()[0];
    assert_eq!(l.start, VertexIdx(0));
    assert_eq!(l.end, VertexIdx(1));
    assert_eq!(l.right, SidedefIdx(0));
    assert_eq!(l.left, None); // one-sided: no `sideback` given
    assert!(map.warnings().is_empty());
}

// A `sideback = 1` linedef with 2 sidedefs resolves to `Some(SidedefIdx(1))`
// (the normal, in-range two-sided path). The 65535-as-a-real-index case (no
// binary `0xffff` sentinel confusion) is already covered by the unit-level
// `resolve_optional_has_no_binary_sentinel` test in `assemble.rs`, which
// guards the constraint this scenario exercises at the byte level.
#[test]
fn two_sided_linedef_resolves_left_sidedef() {
    let text = concat!(
        "namespace = \"doom\";\n",
        "vertex { x = 0.0; y = 0.0; }\n",
        "vertex { x = 64.0; y = 0.0; }\n",
        "linedef { v1 = 0; v2 = 1; sidefront = 0; sideback = 1; }\n",
        "sidedef { sector = 0; }\n",
        "sidedef { sector = 0; }\n",
        "sector { texturefloor = \"F\"; textureceiling = \"C\"; }\n",
        "thing { x = 0.0; y = 0.0; type = 1; }\n",
    );
    let bytes = common::build_named_lumps(&[
        ("MAP01", vec![]),
        ("TEXTMAP", text.as_bytes().to_vec()),
        ("ENDMAP", vec![]),
    ]);
    let wad = Wad::from_bytes_with_options(bytes, ParseOptions::default()).unwrap();
    let group = wad.map_group("MAP01").unwrap();
    let map = Map::assemble_with_options(&wad, &group, ParseOptions::default()).unwrap();

    assert_eq!(map.sidedefs().len(), 2);
    assert_eq!(map.linedefs()[0].left, Some(SidedefIdx(1)));
}

#[test]
fn thing_type_out_of_range_strict_errors_lenient_clamps_and_warns() {
    let text = concat!(
        "namespace = \"doom\";\n",
        "vertex { x = 0.0; y = 0.0; }\n",
        "vertex { x = 64.0; y = 0.0; }\n",
        "linedef { v1 = 0; v2 = 1; sidefront = 0; }\n",
        "sidedef { sector = 0; }\n",
        "sector { texturefloor = \"F\"; textureceiling = \"C\"; }\n",
        "thing { type = 70000; x = 0.0; y = 0.0; }\n",
    );
    let bytes = common::build_named_lumps(&[
        ("MAP01", vec![]),
        ("TEXTMAP", text.as_bytes().to_vec()),
        ("ENDMAP", vec![]),
    ]);
    let wad = Wad::from_bytes_with_options(bytes.clone(), ParseOptions::default()).unwrap();
    let group = wad.map_group("MAP01").unwrap();

    // Strict: out-of-range thing type is fatal.
    let err = Map::assemble_with_options(&wad, &group, ParseOptions::default()).unwrap_err();
    assert!(matches!(err, MapAssembleError::FieldOutOfRange { .. }));

    // Lenient: clamps to u16::MAX and records a warning.
    let wad = Wad::from_bytes_with_options(bytes, ParseOptions::lenient()).unwrap();
    let group = wad.map_group("MAP01").unwrap();
    let map = Map::assemble_with_options(&wad, &group, ParseOptions::lenient()).unwrap();
    assert_eq!(map.things()[0].type_id, u16::MAX);
    assert!(
        map.warnings()
            .iter()
            .any(|w| matches!(w, MapWarning::FieldOutOfRange { .. }))
    );
}

#[test]
fn dangling_linedef_vertex_strict_errors_lenient_warns() {
    // Only 2 vertices exist (indices 0, 1); v1 = 99 is dangling.
    let text = concat!(
        "namespace = \"doom\";\n",
        "vertex { x = 0.0; y = 0.0; }\n",
        "vertex { x = 64.0; y = 0.0; }\n",
        "linedef { v1 = 99; v2 = 1; sidefront = 0; }\n",
        "sidedef { sector = 0; }\n",
        "sector { texturefloor = \"F\"; textureceiling = \"C\"; }\n",
        "thing { x = 0.0; y = 0.0; type = 1; }\n",
    );
    let bytes = common::build_named_lumps(&[
        ("MAP01", vec![]),
        ("TEXTMAP", text.as_bytes().to_vec()),
        ("ENDMAP", vec![]),
    ]);

    // Strict: dangling reference is fatal.
    let wad = Wad::from_bytes_with_options(bytes.clone(), ParseOptions::default()).unwrap();
    let group = wad.map_group("MAP01").unwrap();
    let err = Map::assemble_with_options(&wad, &group, ParseOptions::default()).unwrap_err();
    assert!(matches!(err, MapAssembleError::DanglingReference { .. }));

    // Lenient: clamped and recorded as a warning.
    let wad = Wad::from_bytes_with_options(bytes, ParseOptions::lenient()).unwrap();
    let group = wad.map_group("MAP01").unwrap();
    let map = Map::assemble_with_options(&wad, &group, ParseOptions::lenient()).unwrap();
    assert!(
        map.warnings()
            .iter()
            .any(|w| matches!(w, MapWarning::DanglingReference { .. }))
    );
}

#[test]
fn missing_endmap_strict_errors_lenient_warns() {
    let bytes = common::build_named_lumps(&[
        ("MAP01", vec![]),
        ("TEXTMAP", FULL_MAP.as_bytes().to_vec()),
        // No ENDMAP lump.
    ]);

    let wad = Wad::from_bytes_with_options(bytes.clone(), ParseOptions::default()).unwrap();
    let group = wad.map_group("MAP01").unwrap();
    let err = Map::assemble_with_options(&wad, &group, ParseOptions::default()).unwrap_err();
    assert!(matches!(err, MapAssembleError::UnterminatedUdmf { .. }));

    let wad = Wad::from_bytes_with_options(bytes, ParseOptions::lenient()).unwrap();
    let group = wad.map_group("MAP01").unwrap();
    let map = Map::assemble_with_options(&wad, &group, ParseOptions::lenient()).unwrap();
    assert!(
        map.warnings()
            .iter()
            .any(|w| matches!(w, MapWarning::UnterminatedUdmf { .. }))
    );
}

#[test]
fn non_utf8_textmap_is_a_udmf_error() {
    let bytes = common::build_named_lumps(&[
        ("MAP01", vec![]),
        ("TEXTMAP", vec![0xFF]),
        ("ENDMAP", vec![]),
    ]);
    let wad = Wad::from_bytes_with_options(bytes, ParseOptions::default()).unwrap();
    let group = wad.map_group("MAP01").unwrap();
    let err = Map::assemble_with_options(&wad, &group, ParseOptions::default()).unwrap_err();
    assert!(matches!(err, MapAssembleError::Udmf { .. }));
}

#[test]
fn detect_map_format_recognizes_udmf_textmap_group() {
    use crustywad::map::detect_map_format;

    let bytes = common::build_named_lumps(&[
        ("MAP01", vec![]),
        ("TEXTMAP", FULL_MAP.as_bytes().to_vec()),
        ("ENDMAP", vec![]),
    ]);
    let wad = Wad::from_bytes_with_options(bytes, ParseOptions::default()).unwrap();
    let group = wad.map_group("MAP01").unwrap();
    assert_eq!(detect_map_format(&wad, &group), MapFormat::Udmf);
}

#[test]
fn udmf_thing_flags_reach_the_map_graph() {
    // ADR-0019: UDMF's discrete skill/multiplayer thing booleans are packed
    // into the Doom/Boom-MBF `Thing.flags` layout on assembly, so this value
    // must reach `MapThing.flags` unchanged: skill3 -> bit 1, ambush -> bit 3.
    let text = concat!(
        "namespace = \"doom\";\n",
        "vertex { x = 0; y = 0; }\n",
        "vertex { x = 64; y = 0; }\n",
        "sector { texturefloor = \"FLOOR\"; textureceiling = \"CEIL\"; }\n",
        "sidedef { sector = 0; }\n",
        "linedef { v1 = 0; v2 = 1; sidefront = 0; }\n",
        "thing { x = 32; y = 32; type = 1; skill3 = true; ambush = true; }\n",
    );
    let bytes = common::build_named_lumps(&[
        ("MAP01", vec![]),
        ("TEXTMAP", text.as_bytes().to_vec()),
        ("ENDMAP", vec![]),
    ]);
    let wad = Wad::from_bytes_with_options(bytes, ParseOptions::default()).unwrap();
    let group = wad.map_group("MAP01").unwrap();
    let map = Map::assemble(&wad, &group).unwrap();
    assert_eq!(map.things()[0].flags, 0b0000_1010);
}
