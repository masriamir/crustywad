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

// UDMF assembly (TEXTMAP -> Map) is exercised end-to-end in
// `assemble::tests::assembles_a_minimal_udmf_map`. Here, strict assembly of a
// TEXTMAP group missing its ENDMAP terminator must fail with
// `UnterminatedUdmf` rather than silently mis-decoding the text lump as Doom
// binary records.
#[test]
fn strict_rejects_udmf_missing_endmap() {
    // UDMF: a TEXTMAP lump holds text, not Doom binary records; no ENDMAP.
    let udmf = common::build_named_lumps(&[
        ("MAP01", vec![]),
        ("TEXTMAP", b"namespace = \"zdoom\";".to_vec()),
    ]);
    let wad = Wad::from_bytes(udmf).unwrap();
    let group = wad.map_group("MAP01").unwrap();
    let err = Map::assemble(&wad, &group).unwrap_err();
    assert!(matches!(
        err,
        MapAssembleError::UnterminatedUdmf { ref name } if name == "MAP01"
    ));
    assert!(err.to_string().contains("ENDMAP"));
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
    assert_eq!(map.linedef_right(l).expect("fronted line").middle, "WALL");
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
    // VERTEXES present but zero-length → vertex arena empty; a linedef's
    // endpoints are required references, so this is fatal in both modes.
    let bytes = common::build_doom_map_wad(
        "E1M1",
        vec![],
        linedef(0, 1, 0, 0xffff),
        sidedef(0),
        /*vertexes*/ vec![],
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

// The sidedef arena is no longer a *required* target (ADR-0020): a linedef can
// legitimately have no sides, so an empty SIDEDEFS arena plus a dangling
// non-sentinel reference recovers in lenient mode (`right: None` + warning)
// instead of aborting. Strict mode still rejects the dangling reference.
#[test]
fn empty_sidedef_arena_recovers_in_lenient() {
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

    let err = Map::assemble_with_options(&wad, &group, ParseOptions::default()).unwrap_err();
    assert!(matches!(
        err,
        MapAssembleError::DanglingReference { count: 0, .. }
    ));

    let map = Map::assemble_with_options(&wad, &group, ParseOptions::lenient()).unwrap();
    assert!(map.linedefs()[0].right.is_none());
    assert_eq!(map.warnings().len(), 1);
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

// The MAP08 shape from the retail hexen_ex.wad: a linedef whose front AND back
// sidedef fields are both the on-disk 0xffff "no side" sentinel (an invisible,
// impassable line). Vanilla engines (Chocolate Doom/Hexen `P_LoadLineDefs`)
// treat both fields symmetrically -- `!= -1` guards -- so this is valid data,
// not a defect: it must assemble cleanly in BOTH modes with no warning
// (ADR-0020).
#[test]
fn front_0xffff_sentinel_assembles_as_none_in_both_modes() {
    let bytes = common::build_doom_map_wad(
        "E1M1",
        vec![],
        linedef(0, 1, 0xffff, 0xffff),
        sidedef(0),
        [vertex(0, 0), vertex(64, 0)].concat(),
        sector(),
    );
    let wad = crustywad::Wad::from_bytes(bytes).unwrap();
    let group = wad.map_group("E1M1").unwrap();
    for options in [ParseOptions::default(), ParseOptions::lenient()] {
        let map = Map::assemble_with_options(&wad, &group, options).unwrap();
        let l = &map.linedefs()[0];
        assert_eq!(l.right, None);
        assert_eq!(l.left, None);
        assert!(map.linedef_right(l).is_none());
        assert!(map.linedef_left(l).is_none());
        assert!(
            map.warnings().is_empty(),
            "sentinel is valid data, not a recovered defect: {:?}",
            map.warnings()
        );
    }
}

// A non-sentinel out-of-range front sidedef is still a dangling reference:
// lenient recovers to `None` + a warning (no more clamp-to-0, which fabricated
// a reference not present in the file -- ADR-0020 S3).
#[test]
fn lenient_out_of_range_front_becomes_none() {
    let bytes = common::build_doom_map_wad(
        "E1M1",
        vec![],
        linedef(0, 1, 5, 0xffff), // right=5 non-sentinel, only 1 sidedef
        sidedef(0),
        [vertex(0, 0), vertex(64, 0)].concat(),
        sector(),
    );
    let wad = crustywad::Wad::from_bytes(bytes).unwrap();
    let group = wad.map_group("E1M1").unwrap();
    let map = Map::assemble_with_options(&wad, &group, ParseOptions::lenient()).unwrap();
    assert!(map.linedefs()[0].right.is_none());
    assert_eq!(map.warnings().len(), 1);
}

// Strict mode still rejects a non-sentinel dangling front sidedef.
#[test]
fn strict_rejects_dangling_front_sidedef() {
    let bytes = common::build_doom_map_wad(
        "E1M1",
        vec![],
        linedef(0, 1, 5, 0xffff),
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

// Heretic maps use the *identical* Doom binary record layout, so they assemble
// through the Doom path. crustywad decodes the records and preserves their
// Heretic-specific values verbatim without interpreting them (thing type-ids and
// linedef specials carry different meanings in Heretic, but the same byte layout).
#[test]
fn heretic_map_assembles_via_doom_path() {
    // 2005 is a Heretic thing type (the Ethereal Crossbow) — the same numeric id
    // means the Chainsaw in Doom; crustywad keeps the raw id, meaning is the
    // caller's concern.
    const HERETIC_THING_TYPE: u16 = 2005;
    let bytes = common::build_doom_map_wad(
        "E1M1", // Heretic uses ExMy markers, like Doom
        thing(HERETIC_THING_TYPE),
        linedef(0, 1, 0, 0xffff),
        sidedef(0),
        [vertex(0, 0), vertex(64, 0)].concat(),
        sector(),
    );
    let wad = Wad::from_bytes(bytes).unwrap();
    let group = wad.map_group("E1M1").unwrap();
    let map = Map::assemble(&wad, &group).unwrap();

    assert_eq!(map.format(), MapFormat::Doom);
    assert_eq!(map.things().len(), 1);
    assert_eq!(map.things()[0].type_id, HERETIC_THING_TYPE);
    assert!(map.warnings().is_empty());
}

// Doom II maps are byte-identical to Doom, differing only in `MAPxx` marker
// naming (vs Doom's `ExMy`), which `map_groups()` handles positionally.
#[test]
fn doom2_map_assembles_via_doom_path() {
    let bytes = common::build_doom_map_wad(
        "MAP01",     // Doom II marker naming
        thing(3004), // a former human (shared Doom/Doom II thing type)
        linedef(0, 1, 0, 0xffff),
        sidedef(0),
        [vertex(0, 0), vertex(64, 0)].concat(),
        sector(),
    );
    let wad = Wad::from_bytes(bytes).unwrap();
    let group = wad.map_group("MAP01").expect("MAPxx marker detected");
    assert_eq!(group.name, "MAP01");
    let map = Map::assemble(&wad, &group).unwrap();
    assert_eq!(map.format(), MapFormat::Doom);
    assert_eq!(map.things().len(), 1);
}

#[test]
fn detects_doom_format_without_behavior() {
    use crustywad::map::{MapFormat, detect_map_format};
    // A marker followed by a VERTEXES data lump — a Doom-format group, no BEHAVIOR.
    let bytes = common::build_named_lumps(&[("MAP01", Vec::new()), ("VERTEXES", vec![0u8; 4])]);
    let wad = crustywad::Wad::from_bytes(bytes).expect("parses");
    let group = wad.map_group("MAP01").expect("group");
    assert_eq!(detect_map_format(&wad, &group), MapFormat::Doom);
}

// `t.height` is widened from an `i16` (24), an exactly f64-representable integer, so
// strict float equality is safe here — not a precision-sensitive comparison.
#[allow(clippy::float_cmp)]
#[test]
fn assembles_hexen_map_with_superset_fields() {
    use crustywad::map::{Map, MapFormat};
    let bytes = common::hexen_sample_map_bytes();
    let wad = crustywad::Wad::from_bytes(bytes).expect("parses");
    let group = wad.map_group("MAP01").expect("group");
    let map = Map::assemble(&wad, &group).expect("assembles");

    assert_eq!(map.format(), MapFormat::Hexen);
    let t = &map.things()[0];
    assert_eq!(t.id, 7);
    assert_eq!(t.height, 24.0);
    assert_eq!(t.special.special, 80);
    assert_eq!(t.special.args, [1, 2, 3, 4, 5]);
    let l = &map.linedefs()[0];
    assert_eq!(l.special.special, 13);
    assert_eq!(l.special.args, [99, 0, 0, 0, 0]);
    assert_eq!(l.id, 0); // line id is UDMF-only; Hexen linedefs leave it 0
    assert!(l.left.is_none()); // 0xffff == one-sided
    // The fixture's on-disk Hexen flags are 0x0007 (all skills, no game-mode
    // bits). Hexen's game-mode bits are positive, Doom's are negative, so a
    // thing naming no mode normalizes to bits 4/5/6 SET — it appears nowhere.
    assert_eq!(t.flags, 0x0077);
}

/// Hexen `THINGS` flags are translated into the graph's Doom/Boom-MBF layout at
/// assembly (ADR-0019 §2), end to end: the positive Hexen game-mode bits
/// (`0x0100` single / `0x0200` co-op / `0x0400` deathmatch) invert into Doom's
/// negative bits 4/5/6, and `dormant` plus the class filters — which collide
/// with Doom's bits 4–7 on disk — are dropped rather than misread.
#[test]
fn hexen_thing_flags_are_normalized_to_the_doom_layout() {
    use crustywad::map::Map;

    /// Builds a Hexen map whose single thing carries the on-disk `flags` word,
    /// and returns the assembled (normalized) `MapThing::flags`.
    fn normalized_flags(flags: u16) -> u32 {
        let bytes = common::hexen_map_bytes_with_thing_flags(flags);
        let wad = crustywad::Wad::from_bytes(bytes).expect("parses");
        let group = wad.map_group("MAP01").expect("group");
        let map = Map::assemble(&wad, &group).expect("assembles");
        map.things()[0].flags
    }

    // Present in all three game modes -> Doom's "not in X" bits 4/5/6 all clear.
    assert_eq!(normalized_flags(0x0100 | 0x0200 | 0x0400), 0x0000);
    // Present in none -> bits 4/5/6 all set.
    assert_eq!(normalized_flags(0x0000), 0x0070);
    // Skills and ambush survive verbatim (same meaning, same position).
    assert_eq!(normalized_flags(0x000F | 0x0700), 0x000F);
    // dormant (0x0010) and the fighter/cleric/mage class filters
    // (0x0020/0x0040/0x0080) have no Doom equivalent and are dropped — they must
    // not be mistaken for Doom's game-mode or friend bits.
    assert_eq!(normalized_flags(0x00F0 | 0x0700), 0x0000);
    // Single-player only: excluded from deathmatch (bit 5) and co-op (bit 6).
    assert_eq!(normalized_flags(0x0100), 0x0060);
}

#[test]
fn hexen_lenient_recovers_dangling_linedef_vertex() {
    use crustywad::ParseOptions;
    use crustywad::map::Map;
    // Linedef references vertex index 99 (out of range: only 2 vertices).
    let vertexes = [0i16, 0, 64, 0]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect::<Vec<u8>>();
    let mut sidedef = vec![0u8; 4];
    for _ in 0..3 {
        sidedef.extend_from_slice(&[b'-', 0, 0, 0, 0, 0, 0, 0]);
    }
    sidedef.extend_from_slice(&0u16.to_le_bytes());
    let mut sector = Vec::new();
    sector.extend_from_slice(&0i16.to_le_bytes());
    sector.extend_from_slice(&128i16.to_le_bytes());
    sector.extend_from_slice(&[0u8; 16]);
    sector.extend_from_slice(&160i16.to_le_bytes());
    sector.extend_from_slice(&0i16.to_le_bytes());
    sector.extend_from_slice(&0i16.to_le_bytes());
    // Hexen linedef: start=99 (dangling), end=1, right=0, left=0xffff.
    let linedef: Vec<u8> = vec![
        0x63, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF,
        0xFF,
    ];
    let bytes = common::build_hexen_map_wad(
        "MAP01",
        Vec::new(),
        linedef,
        sidedef,
        vertexes,
        sector,
        b"ACS\0".to_vec(),
    );
    let wad = crustywad::Wad::from_bytes(bytes).expect("parses");
    let group = wad.map_group("MAP01").expect("group");

    // Strict: dangling reference is fatal.
    assert!(Map::assemble(&wad, &group).is_err());
    // Lenient: recovers and records a warning.
    let opts = ParseOptions::lenient();
    let map = Map::assemble_with_options(&wad, &group, opts).expect("lenient recovers");
    assert!(!map.warnings().is_empty());
}

#[test]
fn detects_hexen_format_with_behavior() {
    use crustywad::map::{MapFormat, detect_map_format};
    let bytes = common::build_named_lumps(&[
        ("MAP01", Vec::new()),
        ("VERTEXES", vec![0u8; 4]),
        ("BEHAVIOR", b"ACS\0".to_vec()),
    ]);
    let wad = crustywad::Wad::from_bytes(bytes).expect("parses");
    let group = wad.map_group("MAP01").expect("group");
    assert_eq!(detect_map_format(&wad, &group), MapFormat::Hexen);
}

// ADR-0021 §3: classic assembly produces TextureRef::Name; the PartialEq<&str>
// impl keeps name comparisons ergonomic; Index never equals a name.
#[test]
fn classic_assembly_produces_texture_names() {
    use crustywad::map::TextureRef;
    let bytes = common::build_doom_map_wad(
        "E1M1",
        vec![],
        linedef(0, 1, 0, 0xffff),
        sidedef(0),
        [vertex(0, 0), vertex(64, 0)].concat(),
        sector(),
    );
    let wad = crustywad::Wad::from_bytes(bytes).unwrap();
    let map = Map::assemble(&wad, &wad.map_group("E1M1").unwrap()).unwrap();
    assert_eq!(
        map.sectors()[0].floor_flat,
        TextureRef::Name("FLOOR".into())
    );
    assert_eq!(map.sectors()[0].floor_flat, "FLOOR"); // PartialEq<&str>
    assert_eq!(map.sidedefs()[0].middle.as_name(), Some("WALL"));
    assert_ne!(TextureRef::Index(7), "FLOOR");
}

// A nested-WAD MAPxx lump is a Doom 64 map group (ADR-0021 §1): marker only,
// empty data run, detected as MapFormat::Doom64. A classic empty MAP02
// marker (no nested magic, no data run) is not a group at all, and a
// nested-WAD lump with a non-MAPxx name is not a map.
#[test]
fn doom64_nested_map_lump_forms_a_group() {
    let nested = common::build_wad(*b"IWAD", &[("THINGS", &[])]);
    let bytes = common::build_named_lumps(&[
        ("MAP01", nested.clone()),
        ("MAP02", vec![]),
        ("RESOURCE", nested),
    ]);
    let wad = Wad::from_bytes(bytes).unwrap();
    let groups = wad.map_groups();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].name, "MAP01");
    assert!(groups[0].data_indices.is_empty());
    assert_eq!(
        crustywad::map::detect_map_format(&wad, &groups[0]),
        MapFormat::Doom64
    );
    assert!(wad.map_group("MAP01").is_some());
    assert!(wad.map_group("RESOURCE").is_none());
}

#[test]
fn doom64_group_assembles_or_errors_without_panicking() {
    // Interim guard until the assemble arm lands: must not panic.
    let nested = common::build_wad(*b"IWAD", &[("THINGS", &[])]);
    let bytes = common::build_named_lumps(&[("MAP01", nested)]);
    let wad = Wad::from_bytes(bytes).unwrap();
    let group = wad.map_group("MAP01").unwrap();
    let _ = Map::assemble(&wad, &group); // Ok or Err both fine; no panic
}

// Grouping requires the MAPxx name AND nested magic, but format detection keys
// on the marker's magic alone — so a classic-named marker ("E1M1", which fails
// is_doom64_map_name) whose bytes carry nested IWAD/PWAD magic groups
// classically with a real data run, yet detects as Doom64. Assembling such a
// group must error (UnsupportedFormat), never panic, in both strictness modes
// (ADR-0016: no panic on untrusted input).
#[test]
fn classic_named_marker_with_nested_magic_errors_instead_of_panicking() {
    let mut marker = b"IWAD".to_vec();
    marker.extend_from_slice(&[0u8; 8]); // >= 12 bytes: passes is_doom64_map_lump
    let bytes = common::build_named_lumps(&[
        ("E1M1", marker),
        ("VERTEXES", [vertex(0, 0), vertex(64, 0)].concat()),
        ("SECTORS", sector()),
        ("SIDEDEFS", sidedef(0)),
    ]);
    let wad = Wad::from_bytes(bytes).unwrap();
    let group = wad.map_group("E1M1").unwrap();
    assert!(
        !group.data_indices.is_empty(),
        "the classic-named marker must group with its data run"
    );
    assert_eq!(
        crustywad::map::detect_map_format(&wad, &group),
        MapFormat::Doom64
    );
    for options in [ParseOptions::default(), ParseOptions::lenient()] {
        let err = Map::assemble_with_options(&wad, &group, options).unwrap_err();
        assert!(
            matches!(err, MapAssembleError::UnsupportedFormat { .. }),
            "expected UnsupportedFormat, got {err:?}"
        );
    }
}
