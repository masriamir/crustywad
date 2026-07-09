//! Integration tests for positional map-group detection.

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
