//! Optional integration test against a local Hexen IWAD.
//!
//! Hexen's IWAD is not freely redistributable, so it is never fetched or
//! committed; supply your own via `CRUSTYWAD_HEXEN_DIR` to run this test.

#![cfg(feature = "hexen-tests")]

mod common;

use crustywad::Wad;

#[test]
fn parses_hexen_when_fixtures_are_available() {
    use crustywad::map::{Map, MapFormat};
    for path in common::iwad_files("CRUSTYWAD_HEXEN_DIR", &["hexen.wad"]) {
        let wad = Wad::from_path(&path).expect("fixture should parse");
        assert!(
            wad.lump_count() > 0,
            "{} should contain lumps",
            path.display()
        );
        // Assemble the first map group and confirm it is detected as Hexen.
        let group = wad
            .map_groups()
            .into_iter()
            .next()
            .expect("at least one map");
        let map = Map::assemble(&wad, &group).expect("hexen map assembles");
        assert_eq!(map.format(), MapFormat::Hexen);
        assert!(!map.things().is_empty(), "hexen map should have things");
    }
}
