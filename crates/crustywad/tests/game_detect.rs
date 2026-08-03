//! Integration tests for WAD-level game identification (ADR-0028 §1).

mod common;

use crustywad::{Wad, WadGame};

/// Builds a PWAD holding one lump of the given name and size (zero-filled).
fn wad_with_lump(name: &str, size: usize) -> Wad {
    let data = vec![0_u8; size];
    let bytes = common::build_wad(*b"PWAD", &[(name, data.as_slice())]);
    Wad::from_bytes(bytes).expect("synthetic WAD parses")
}

#[test]
fn detects_strife_from_retail_sized_script_lump() {
    // 1516 = the retail dialogue record (0x5EC); 4548 = 1516 * 3.
    for size in [1516, 4548] {
        assert_eq!(
            wad_with_lump("SCRIPT01", size).detect_game(),
            Some(WadGame::Strife),
            "size {size}"
        );
    }
    assert_eq!(
        wad_with_lump("SCRIPT99", 1516).detect_game(),
        Some(WadGame::Strife)
    );
}

#[test]
fn detects_strife_from_demo_sized_script_lump() {
    // 1488 = the demo dialogue record (0x5D0); 2976 = 1488 * 2.
    for size in [1488, 2976] {
        assert_eq!(
            wad_with_lump("SCRIPT01", size).detect_game(),
            Some(WadGame::Strife),
            "size {size}"
        );
    }
}

#[test]
fn rejects_wrong_sizes() {
    // 0 = empty; 1517 = off-by-one; 3004 = 1516 + 1488, divisible by neither.
    for size in [0, 1517, 3004] {
        assert_eq!(
            wad_with_lump("SCRIPT01", size).detect_game(),
            None,
            "size {size}"
        );
    }
}

#[test]
fn rejects_wrong_names() {
    // 7-char name, non-digit suffix, prefixed name — none match SCRIPT + 2 digits.
    for name in ["SCRIPT1", "SCRIPTAB", "XSCRIPT1"] {
        assert_eq!(wad_with_lump(name, 1516).detect_game(), None, "name {name}");
    }
}

#[test]
fn no_fingerprint_means_none() {
    assert_eq!(wad_with_lump("THINGS", 1516).detect_game(), None);
    let empty = Wad::from_bytes(common::build_wad(*b"PWAD", &[])).expect("empty WAD parses");
    assert_eq!(empty.detect_game(), None);
}
