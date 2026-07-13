//! Optional integration test against a local Doom 64 IWAD.
//!
//! The Doom 64 IWAD is not freely redistributable, so it is never fetched or
//! committed; supply your own via `CRUSTYWAD_DOOM64_DIR` to run this test.

#![cfg(feature = "doom64-tests")]

mod common;

use crustywad::map::{is_doom64_map_lump, read_doom64_map};
use crustywad::{ParseOptions, Wad};

#[test]
fn parses_doom64_when_fixtures_are_available() {
    for path in common::iwad_files("CRUSTYWAD_DOOM64_DIR", &["doom64.wad"]) {
        let wad = Wad::from_path(&path).expect("fixture should parse");
        assert!(
            wad.lump_count() > 0,
            "{} should contain lumps",
            path.display()
        );

        // Every lump whose bytes are a nested WAD is a Doom 64 map; read each.
        let mut maps_read = 0usize;
        for index in 0..wad.lump_count() {
            let bytes = wad.lump_bytes(index).expect("index in range");
            if !is_doom64_map_lump(bytes) {
                continue;
            }
            let map = read_doom64_map(bytes, &ParseOptions::lenient())
                .expect("a real Doom 64 map lump should read");
            maps_read += 1;
            // A real map has geometry.
            assert!(
                !map.linedefs.is_empty() && !map.sectors.is_empty(),
                "map lump {index} in {} should have geometry",
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
