//! Optional integration test against a local Hexen IWAD.
//!
//! Hexen's IWAD is not freely redistributable, so it is never fetched or
//! committed; supply your own via `CRUSTYWAD_HEXEN_DIR` to run this test.

#![cfg(feature = "hexen-tests")]

mod common;

use crustywad::Wad;

#[test]
fn parses_hexen_when_fixtures_are_available() {
    for path in common::iwad_files("CRUSTYWAD_HEXEN_DIR", &["hexen.wad"]) {
        let wad = Wad::from_path(&path).expect("fixture should parse");
        assert!(
            wad.lump_count() > 0,
            "{} should contain lumps",
            path.display()
        );
    }
}
