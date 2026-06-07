//! Integration tests for the main WAD reader API.

mod common;

use crustywad::{ParseError, ParseOptions, ParseWarning, Wad, WadKind};
use proptest::prelude::*;
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

#[test]
fn parses_basic_wad() {
    let wad = Wad::from_bytes(common::build_wad(*b"IWAD", &[("PLAYPAL", &[1, 2, 3])]))
        .expect("wad should parse");
    assert_eq!(wad.kind(), WadKind::Iwad);
    assert_eq!(wad.lump_count(), 1);
    assert_eq!(wad.lump_by_name("PLAYPAL").expect("missing lump").size(), 3);
    assert_eq!(wad.lump_bytes(0), Some(&[1, 2, 3][..]));
}

#[test]
fn strict_mode_rejects_bad_magic() {
    let err = Wad::from_bytes(common::build_wad(*b"NOPE", &[])).expect_err("magic should fail");
    assert!(matches!(err, ParseError::InvalidMagic { .. }));
}

#[test]
fn lenient_mode_collects_warnings() {
    let mut wad = common::build_wad(*b"NOPE", &[("TEST", &[1, 2, 3])]);
    wad[4..8].copy_from_slice(&1_i32.to_le_bytes());
    wad[8..12].copy_from_slice(&128_i32.to_le_bytes());
    let parsed = Wad::from_bytes_with_options(wad, ParseOptions::lenient())
        .expect("lenient parse should succeed");
    assert!(matches!(parsed.kind(), WadKind::Unknown(_)));
    assert!(
        parsed
            .warnings()
            .iter()
            .any(|warning| matches!(warning, ParseWarning::InvalidMagic(_)))
    );
    assert_eq!(parsed.lump_count(), 0);
}

#[test]
fn strict_mode_rejects_non_ascii_names() {
    let mut wad = common::build_wad(*b"PWAD", &[("TEST", &[1])]);
    let name_offset = wad.len() - 8;
    wad[name_offset] = 0xFF;
    let err = Wad::from_bytes(wad).expect_err("non-ascii name should fail");
    assert!(matches!(err, ParseError::NonAsciiName { .. }));
}

#[test]
fn lenient_mode_clamps_oversized_lumps() {
    let mut wad = common::build_wad(*b"PWAD", &[("TEST", &[1, 2, 3])]);
    let size_offset = wad.len() - 16;
    wad[size_offset + 4..size_offset + 8].copy_from_slice(&999_i32.to_le_bytes());
    let parsed = Wad::from_bytes_with_options(wad, ParseOptions::lenient())
        .expect("lenient parse should succeed");
    assert_eq!(parsed.lump_bytes(0), Some(&[1, 2, 3][..]));
    assert!(
        parsed
            .warnings()
            .iter()
            .any(|warning| matches!(warning, ParseWarning::OutOfBounds { .. }))
    );
}

#[test]
fn parse_options_default_to_strict() {
    use crustywad::Strictness;
    assert_eq!(ParseOptions::default().strictness, Strictness::Strict);
}

#[test]
fn strict_mode_rejects_lump_inside_directory() {
    let mut wad = common::build_wad(*b"IWAD", &[("TEST", &[1, 2, 3])]);
    // Directory starts at byte 15. Set filepos to 17 — inside the directory region [15, 31).
    wad[15..19].copy_from_slice(&17_i32.to_le_bytes());
    let err = Wad::from_bytes(wad).expect_err("lump pointing into directory should fail");
    assert!(matches!(
        err,
        ParseError::OutOfBounds {
            field: "lump data",
            ..
        }
    ));
}

#[test]
fn lenient_mode_clamps_lump_inside_directory() {
    let mut wad = common::build_wad(*b"IWAD", &[("TEST", &[1, 2, 3])]);
    wad[15..19].copy_from_slice(&17_i32.to_le_bytes());
    let parsed = Wad::from_bytes_with_options(wad, ParseOptions::lenient())
        .expect("lenient parse should succeed");
    assert!(parsed.warnings().iter().any(|w| matches!(
        w,
        ParseWarning::OutOfBounds {
            field: "lump data",
            ..
        }
    )));
    assert_eq!(parsed.lump_bytes(0), Some(&[][..]));
}

#[test]
fn strict_mode_rejects_negative_lump_filepos() {
    let mut wad = common::build_wad(*b"IWAD", &[("TEST", &[1, 2, 3])]);
    wad[15..19].copy_from_slice(&(-1_i32).to_le_bytes());
    let err = Wad::from_bytes(wad).expect_err("negative filepos should fail");
    assert!(matches!(
        err,
        ParseError::NegativeValue {
            field: "filepos",
            value: -1
        }
    ));
}

#[test]
fn lenient_mode_recovers_negative_lump_size() {
    let mut wad = common::build_wad(*b"IWAD", &[("TEST", &[1, 2, 3])]);
    wad[19..23].copy_from_slice(&(-10_i32).to_le_bytes());
    let parsed = Wad::from_bytes_with_options(wad, ParseOptions::lenient())
        .expect("lenient parse should succeed");
    assert!(parsed.warnings().iter().any(|w| matches!(
        w,
        ParseWarning::NegativeValue {
            field: "size",
            value: -10
        }
    )));
    assert_eq!(parsed.lump_bytes(0), Some(&[][..]));
}

#[test]
fn into_bytes_round_trips() {
    let original = common::build_wad(*b"IWAD", &[("FLAT", &[0xAA, 0xBB])]);
    let wad = Wad::from_bytes(original.clone()).expect("wad should parse");
    assert_eq!(wad.into_bytes(), original);
}

proptest! {
    #[test]
    fn strict_parser_handles_generated_empty_wads(kind in prop_oneof![Just(*b"IWAD"), Just(*b"PWAD")]) {
        let wad = Wad::from_bytes(common::build_wad(kind, &[])).expect("generated wad should parse");
        prop_assert_eq!(wad.lump_count(), 0);
        prop_assert!(matches!(wad.kind(), WadKind::Iwad | WadKind::Pwad));
    }
}
