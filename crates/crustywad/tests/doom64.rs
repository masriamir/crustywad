//! Optional integration test against a local Doom 64 IWAD.
//!
//! The Doom 64 IWAD is not freely redistributable, so it is never fetched or
//! committed; supply your own via `CRUSTYWAD_DOOM64_DIR` to run this test.

#![cfg(feature = "doom64-tests")]

mod common;

use crustywad::map::{is_doom64_map_lump, read_doom64_map};
use crustywad::{ParseOptions, Wad};

/// A Doom 64 map marker lump is named `MAPxx` (`MAP` + two ASCII digits).
fn is_doom64_map_name(name: &str) -> bool {
    name.len() == 5 && name.starts_with("MAP") && name[3..].bytes().all(|b| b.is_ascii_digit())
}

#[test]
fn parses_doom64_when_fixtures_are_available() {
    for path in common::iwad_files("CRUSTYWAD_DOOM64_DIR", &["doom64.wad"]) {
        let wad = Wad::from_path(&path).expect("fixture should parse");
        assert!(
            wad.lump_count() > 0,
            "{} should contain lumps",
            path.display()
        );

        // Select maps by their `MAPxx` lump name (not by sniffing bytes), so a
        // regression that breaks the nested-WAD header can't silently skip a map,
        // and a non-map lump can't be misclassified as one.
        let mut maps_read = 0usize;
        for lump in wad.lumps() {
            if !is_doom64_map_name(lump.name()) {
                continue;
            }
            let bytes = wad.lump_data(lump);
            assert!(
                is_doom64_map_lump(bytes),
                "map lump {} in {} should be a nested WAD",
                lump.name(),
                path.display()
            );
            let map = read_doom64_map(bytes, &ParseOptions::lenient())
                .expect("a real Doom 64 map lump should read");
            maps_read += 1;
            // A real map has geometry.
            assert!(
                !map.linedefs.is_empty() && !map.sectors.is_empty(),
                "map {} in {} should have geometry",
                lump.name(),
                path.display()
            );
        }
        assert!(
            maps_read > 0,
            "{} should contain at least one Doom 64 map",
            path.display()
        );
    }
}
