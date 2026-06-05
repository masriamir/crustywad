//! Integration tests for the main WAD reader API.

mod common;

use crustywad::{ParseOptions, ParseWarning, Wad, WadKind};
use tempfile::NamedTempFile;

#[test]
fn loads_from_path() {
    let bytes = common::build_wad(*b"IWAD", &[("DEMO1", &[1, 2, 3, 4])]);
    let file = NamedTempFile::new().expect("tempfile should be created");
    std::fs::write(file.path(), bytes).expect("wad should be written");

    let wad = Wad::from_path(file.path()).expect("wad should load from path");
    assert_eq!(wad.kind(), WadKind::Iwad);
    assert_eq!(wad.lump_bytes(0), Some(&[1, 2, 3, 4][..]));
}

#[test]
fn finds_lumps_by_name() {
    let lumps = [("TITLEPIC", &[9, 9][..]), ("PLAYPAL", &[3][..])];
    let wad = Wad::from_bytes(common::build_wad(*b"PWAD", &lumps)).expect("wad should parse");
    let lump_map = common::lump_map(&lumps);
    let playpal = wad.lump_by_name("PLAYPAL").expect("PLAYPAL missing");
    assert_eq!(wad.lump_data(playpal), lump_map["PLAYPAL"]);
}

#[test]
fn lenient_mode_recovers_directory_overflow() {
    let mut bytes = common::build_wad(*b"PWAD", &[("A", &[1]), ("B", &[2])]);
    bytes[8..12].copy_from_slice(&999_i32.to_le_bytes());

    let wad = Wad::from_bytes_with_options(bytes, ParseOptions::lenient())
        .expect("lenient mode should recover");
    assert_eq!(wad.lump_count(), 0);
    assert!(wad.warnings().iter().any(|warning| matches!(
        warning,
        ParseWarning::OutOfBounds {
            field: "directory",
            ..
        }
    )));
}
