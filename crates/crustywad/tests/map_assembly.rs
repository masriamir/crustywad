//! Integration tests for positional map-group detection and strict Doom map
//! assembly.

mod common;
use crustywad::Wad;
use proptest::prelude::*;

#[test]
fn detects_two_maps_and_their_data_runs() {
    // E1M1 + full data run, then MAP01 + a shorter run, plus a trailing non-map lump.
    let bytes = common::build_named_lumps(&[
        ("E1M1", vec![]),
        ("THINGS", vec![0; 10]),
        ("LINEDEFS", vec![0; 14]),
        ("SIDEDEFS", vec![0; 30]),
        ("VERTEXES", vec![0; 4]),
        ("SECTORS", vec![0; 26]),
        ("MAP01", vec![]),
        ("VERTEXES", vec![0; 4]),
        ("LINEDEFS", vec![0; 14]),
        ("SIDEDEFS", vec![0; 30]),
        ("SECTORS", vec![0; 26]),
        ("THINGS", vec![0; 10]),
        ("PLAYPAL", vec![0; 768]), // not a map lump, not preceded by a marker
    ]);
    let wad = Wad::from_bytes(bytes).expect("parse");
    let groups = wad.map_groups();
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].name, "E1M1");
    assert_eq!(groups[0].marker_index, 0);
    assert_eq!(groups[0].data_indices, vec![1, 2, 3, 4, 5]);
    assert_eq!(groups[1].name, "MAP01");
    assert_eq!(groups[1].data_indices.len(), 5);
    assert!(wad.map_group("MAP01").is_some());
    assert!(wad.map_group("NOPE").is_none());
}

proptest! {
    // map_groups must never panic, and every data_indices entry must be a
    // valid, strictly-increasing directory index, for arbitrary small WADs.
    #[test]
    fn map_groups_never_panics_and_indices_are_valid(bytes in common::arb_valid_wad()) {
        let result = Wad::from_bytes(bytes);
        prop_assert!(result.is_ok(), "arb_valid_wad() must produce parseable bytes: {:?}", result.err());
        let wad = result.unwrap();
        let groups = std::hint::black_box(wad.map_groups());
        for group in &groups {
            prop_assert!(group.marker_index < wad.lump_count());
            let mut prev = group.marker_index;
            for &index in &group.data_indices {
                prop_assert!(index < wad.lump_count(), "data index {index} out of range");
                prop_assert!(index > prev, "data indices must be strictly increasing");
                prev = index;
            }
        }
    }
}

use crustywad::ParseOptions;
use crustywad::map::{Map, MapAssembleError};

// Little-endian record encoders kept local to the test.
fn le_u16(v: u16) -> [u8; 2] {
    v.to_le_bytes()
}
fn vertex(x: i16, y: i16) -> Vec<u8> {
    [x.to_le_bytes(), y.to_le_bytes()].concat()
}
fn sector() -> Vec<u8> {
    // 26 bytes: 2+2+8+8+2+2+2
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
fn sidedef(sec: u16) -> Vec<u8> {
    // 30 bytes: 2+2+8+8+8+2
    let mut b = Vec::new();
    b.extend(0i16.to_le_bytes());
    b.extend(0i16.to_le_bytes());
    b.extend(b"-\0\0\0\0\0\0\0");
    b.extend(b"-\0\0\0\0\0\0\0");
    b.extend(b"WALL\0\0\0\0");
    b.extend(le_u16(sec));
    b
}
fn linedef(sv: u16, ev: u16, right: u16, left: u16) -> Vec<u8> {
    // 14 bytes
    [
        le_u16(sv),
        le_u16(ev),
        le_u16(0),
        le_u16(0),
        le_u16(0),
        le_u16(right),
        le_u16(left),
    ]
    .concat()
}

#[test]
fn assembles_valid_doom_map_strict() {
    let bytes = common::build_doom_map_wad(
        "E1M1",
        /*things*/ vec![],
        /*linedefs*/ linedef(0, 1, 0, 0xffff),
        /*sidedefs*/ sidedef(0),
        /*vertexes*/ [vertex(0, 0), vertex(64, 0)].concat(),
        /*sectors*/ sector(),
    );
    let wad = crustywad::Wad::from_bytes(bytes).unwrap();
    let group = wad.map_group("E1M1").unwrap();
    let map = Map::assemble_with_options(&wad, &group, ParseOptions::default()).unwrap();
    assert_eq!(map.vertices().len(), 2);
    assert_eq!(map.linedefs().len(), 1);
    let l = &map.linedefs()[0];
    assert!(l.left.is_none()); // 0xffff sentinel
    assert_eq!(map.linedef_right(l).middle, "WALL");
    assert!(map.warnings().is_empty());
}

#[test]
fn strict_rejects_dangling_vertex() {
    let bytes = common::build_doom_map_wad(
        "E1M1",
        vec![],
        linedef(0, 9, 0, 0xffff),
        sidedef(0),
        [vertex(0, 0), vertex(64, 0)].concat(),
        sector(),
    );
    let wad = crustywad::Wad::from_bytes(bytes).unwrap();
    let group = wad.map_group("E1M1").unwrap();
    let err = Map::assemble_with_options(&wad, &group, ParseOptions::default()).unwrap_err();
    assert!(matches!(
        err,
        MapAssembleError::DanglingReference {
            referent: "vertex",
            index: 9,
            count: 2,
            ..
        }
    ));
}

#[test]
fn strict_errors_on_missing_required_lump() {
    // A marker followed by only VERTEXES — SIDEDEFS/SECTORS/LINEDEFS/THINGS absent.
    let bytes = common::build_named_lumps(&[("E1M1", vec![]), ("VERTEXES", vertex(0, 0))]);
    let wad = crustywad::Wad::from_bytes(bytes).unwrap();
    let group = wad.map_group("E1M1").unwrap();
    let err = Map::assemble_with_options(&wad, &group, ParseOptions::default()).unwrap_err();
    assert!(matches!(err, MapAssembleError::MissingLump { .. }));
}
