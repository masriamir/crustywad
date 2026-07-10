//! Optional integration tests that inspect local Freedoom fixtures.

#![cfg(feature = "freedoom-tests")]

mod common;

use crustywad::Wad;

#[test]
fn parses_freedoom_when_fixtures_are_available() {
    for path in common::iwad_files(
        "CRUSTYWAD_FREEDOOM_DIR",
        &["freedoom1.wad", "freedoom2.wad"],
    ) {
        let wad = Wad::from_path(&path).expect("fixture should parse");
        assert!(
            wad.lump_count() > 0,
            "{} should contain lumps",
            path.display()
        );
    }
}
