//! Optional integration test against a local Doom 64 IWAD.
//!
//! The Doom 64 IWAD is not freely redistributable, so it is never fetched or
//! committed; supply your own via `CRUSTYWAD_DOOM64_DIR` to run this test.

#![cfg(feature = "doom64-tests")]

mod common;

use crustywad::Wad;

#[test]
fn parses_doom64_when_fixtures_are_available() {
    for path in common::iwad_files("CRUSTYWAD_DOOM64_DIR", &["doom64.wad"]) {
        let wad = Wad::from_path(&path).expect("fixture should parse");
        assert!(wad.lump_count() > 0, "{} should contain lumps", path.display());
    }
}
