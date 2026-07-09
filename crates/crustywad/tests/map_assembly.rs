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
use crustywad::map::{Map, MapAssembleError, MapFormat};

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
fn thing(type_id: u16) -> Vec<u8> {
    // 10 bytes: x, y (i16), angle, type_id, flags (u16)
    let mut b = Vec::new();
    b.extend(32i16.to_le_bytes());
    b.extend(48i16.to_le_bytes());
    b.extend(90u16.to_le_bytes());
    b.extend(type_id.to_le_bytes());
    b.extend(7u16.to_le_bytes());
    b
}

// A fully populated map: one thing and a *two-sided* linedef, exercising
// `normalize_things`, the `resolve_left` in-range (Some) branch, and every
// `Map` accessor (name/format/vertices/linedefs/sidedefs/sectors/things).
#[test]
fn assembles_two_sided_map_and_exposes_all_accessors() {
    let bytes = common::build_doom_map_wad(
        "E1M1",
        thing(3001),                            // one thing (an imp)
        linedef(0, 1, 0, 1),                    // two-sided: right=0, left=1
        [sidedef(0), sidedef(0)].concat(),      // two sidedefs
        [vertex(0, 0), vertex(64, 0)].concat(), // two vertices
        sector(),                               // one sector
    );
    let wad = Wad::from_bytes(bytes).unwrap();
    let group = wad.map_group("E1M1").unwrap();
    let map = Map::assemble(&wad, &group).unwrap();

    assert_eq!(map.name(), "E1M1");
    assert_eq!(map.format(), MapFormat::Doom);
    assert_eq!(map.vertices().len(), 2);
    assert_eq!(map.linedefs().len(), 1);
    assert_eq!(map.sidedefs().len(), 2);
    assert_eq!(map.sectors().len(), 1);
    assert_eq!(map.things().len(), 1);
    assert_eq!(map.things()[0].type_id, 3001);
    assert_eq!(map.sectors()[0].ceiling_height, 128);
    assert!(map.warnings().is_empty());

    // Two-sided line: `left` resolves to `Some`, exercising the resolver's
    // in-range branch.
    let l = &map.linedefs()[0];
    let left = map
        .linedef_left(l)
        .expect("two-sided line has a left sidedef");
    assert_eq!(map.sidedef_sector(left).floor_flat, "FLOOR");
}

// Assembly must refuse a non-Doom map rather than silently mis-decoding its
// lumps as Doom records (whose byte lengths can coincidentally align).
#[test]
fn refuses_non_doom_formats() {
    // UDMF: a TEXTMAP lump holds text, not Doom binary records.
    let udmf = common::build_named_lumps(&[
        ("MAP01", vec![]),
        ("TEXTMAP", b"namespace = \"zdoom\";".to_vec()),
    ]);
    let wad = Wad::from_bytes(udmf).unwrap();
    let group = wad.map_group("MAP01").unwrap();
    let err = Map::assemble(&wad, &group).unwrap_err();
    assert!(matches!(
        err,
        MapAssembleError::UnsupportedFormat { lump: "TEXTMAP" }
    ));
    assert!(err.to_string().contains("TEXTMAP"));

    // Hexen: a BEHAVIOR lump alongside otherwise Doom-shaped lumps.
    let hexen = common::build_named_lumps(&[
        ("MAP02", vec![]),
        ("THINGS", vec![0; 10]),
        ("LINEDEFS", vec![0; 16]), // Hexen linedefs are 16 bytes, not 14
        ("SIDEDEFS", vec![0; 30]),
        ("VERTEXES", vec![0; 4]),
        ("SECTORS", vec![0; 26]),
        ("BEHAVIOR", vec![0; 8]),
    ]);
    let wad = Wad::from_bytes(hexen).unwrap();
    let group = wad.map_group("MAP02").unwrap();
    assert!(matches!(
        Map::assemble(&wad, &group).unwrap_err(),
        MapAssembleError::UnsupportedFormat { lump: "BEHAVIOR" }
    ));
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

#[test]
fn lenient_clamps_dangling_and_warns() {
    let bytes = common::build_doom_map_wad(
        "E1M1",
        vec![],
        linedef(0, 9, 0, 0xffff), // vertex 9 out of range (only 2 exist)
        sidedef(0),
        [vertex(0, 0), vertex(64, 0)].concat(),
        sector(),
    );
    let wad = crustywad::Wad::from_bytes(bytes).unwrap();
    let group = wad.map_group("E1M1").unwrap();
    let map = Map::assemble_with_options(&wad, &group, ParseOptions::lenient()).unwrap();
    assert_eq!(map.linedefs()[0].end, crustywad::map::VertexIdx(0)); // clamped
    assert_eq!(map.warnings().len(), 1);
}

#[test]
fn lenient_out_of_range_left_becomes_none() {
    let bytes = common::build_doom_map_wad(
        "E1M1",
        vec![],
        linedef(0, 1, 0, 5), // left=5 non-sentinel, only 1 sidedef
        sidedef(0),
        [vertex(0, 0), vertex(64, 0)].concat(),
        sector(),
    );
    let wad = crustywad::Wad::from_bytes(bytes).unwrap();
    let group = wad.map_group("E1M1").unwrap();
    let map = Map::assemble_with_options(&wad, &group, ParseOptions::lenient()).unwrap();
    assert!(map.linedefs()[0].left.is_none());
    assert_eq!(map.warnings().len(), 1);
}

#[test]
fn empty_required_arena_errors_even_in_lenient() {
    // SIDEDEFS present but zero-length → sidedef arena empty; a linedef needs a right side.
    let bytes = common::build_doom_map_wad(
        "E1M1",
        vec![],
        linedef(0, 1, 0, 0xffff),
        /*sidedefs*/ vec![],
        [vertex(0, 0), vertex(64, 0)].concat(),
        sector(),
    );
    let wad = crustywad::Wad::from_bytes(bytes).unwrap();
    let group = wad.map_group("E1M1").unwrap();
    let err = Map::assemble_with_options(&wad, &group, ParseOptions::lenient()).unwrap_err();
    assert!(matches!(
        err,
        MapAssembleError::DanglingReference { count: 0, .. }
    ));
}

#[test]
fn assemble_is_strict_by_default() {
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
    assert!(Map::assemble(&wad, &group).is_err());
}

#[test]
fn strict_rejects_dangling_left_sidedef() {
    // left=5 is a non-sentinel out-of-range left sidedef; only 1 sidedef exists.
    let bytes = common::build_doom_map_wad(
        "E1M1",
        vec![],
        linedef(0, 1, 0, 5),
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
            referent: "sidedef",
            index: 5,
            count: 1,
            ..
        }
    ));
}

#[test]
fn records_error_on_undecodable_linedefs_lump() {
    // Valid VERTEXES/SECTORS/SIDEDEFS, but a LINEDEFS lump whose length (13
    // bytes) is not a multiple of the 14-byte record size.
    let bytes = common::build_named_lumps(&[
        ("E1M1", vec![]),
        ("VERTEXES", [vertex(0, 0), vertex(64, 0)].concat()),
        ("SECTORS", sector()),
        ("SIDEDEFS", sidedef(0)),
        ("LINEDEFS", vec![0; 13]),
        ("THINGS", vec![]),
    ]);
    let wad = crustywad::Wad::from_bytes(bytes).unwrap();
    let group = wad.map_group("E1M1").unwrap();
    let err = Map::assemble_with_options(&wad, &group, ParseOptions::default()).unwrap_err();
    assert!(matches!(
        err,
        MapAssembleError::Records {
            lump: "LINEDEFS",
            ..
        }
    ));
}

#[test]
fn hand_built_map_group_with_out_of_range_data_index_does_not_panic() {
    use crustywad::map::MapGroup;

    let bytes = common::build_doom_map_wad(
        "E1M1",
        vec![],
        linedef(0, 1, 0, 0xffff),
        sidedef(0),
        [vertex(0, 0), vertex(64, 0)].concat(),
        sector(),
    );
    let wad = crustywad::Wad::from_bytes(bytes).unwrap();
    let group = wad.map_group("E1M1").unwrap();

    // Replace the real data indices with a single out-of-range index, so the
    // required-lump search (Fix 1's `.get(i)`) must skip it rather than
    // panic, and every required lump ends up "not present".
    let bad_group = MapGroup {
        marker_index: group.marker_index,
        name: group.name.clone(),
        data_indices: vec![wad.lump_count()], // one past the end: out of range
    };

    let err = Map::assemble(&wad, &bad_group).unwrap_err();
    assert!(matches!(err, MapAssembleError::MissingLump { .. }));
}
