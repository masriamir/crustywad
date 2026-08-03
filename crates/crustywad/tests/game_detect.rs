//! Integration tests for WAD-level game identification (ADR-0028 §1).

mod common;

use crustywad::map::{Map, MapFormat, MapWarning};
use crustywad::{ParseOptions, Wad, WadGame};

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

/// A minimal Doom-format map (marker + empty classic member lumps) plus one
/// qualifying SCRIPT lump when `with_script` is set.
fn strife_flavored_map_wad(with_script: bool) -> Wad {
    let script = vec![0_u8; 1516];
    let mut lumps: Vec<(&str, &[u8])> = vec![
        ("MAP01", &[]),
        ("THINGS", &[]),
        ("LINEDEFS", &[]),
        ("SIDEDEFS", &[]),
        ("VERTEXES", &[]),
        ("SEGS", &[]),
        ("SSECTORS", &[]),
        ("NODES", &[]),
        ("SECTORS", &[]),
        ("REJECT", &[]),
        ("BLOCKMAP", &[]),
    ];
    if with_script {
        lumps.push(("SCRIPT01", script.as_slice()));
    }
    Wad::from_bytes(common::build_wad(*b"IWAD", &lumps)).expect("synthetic WAD parses")
}

#[test]
fn strife_wad_maps_carry_game_attribution_in_both_modes() {
    let wad = strife_flavored_map_wad(true);
    let group = &wad.map_groups()[0];
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        let map = Map::assemble_with_options(&wad, group, options).expect("assembles");
        assert_eq!(
            map.game(),
            Some(WadGame::Strife),
            "{:?}",
            options.strictness
        );
    }
}

#[test]
fn unfingerprinted_wad_maps_have_no_game() {
    let wad = strife_flavored_map_wad(false);
    let group = &wad.map_groups()[0];
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        let map = Map::assemble_with_options(&wad, group, options).expect("assembles");
        assert_eq!(map.game(), None, "{:?}", options.strictness);
    }
}

#[test]
fn strife_doom_format_map_warns_in_lenient_mode_only() {
    let wad = strife_flavored_map_wad(true);
    let group = &wad.map_groups()[0];

    let strict =
        Map::assemble_with_options(&wad, group, ParseOptions::strict()).expect("assembles");
    assert!(
        strict.warnings().is_empty(),
        "strict must not warn: {:?}",
        strict.warnings()
    );

    let lenient =
        Map::assemble_with_options(&wad, group, ParseOptions::lenient()).expect("assembles");
    assert_eq!(lenient.format(), MapFormat::Doom);
    assert_eq!(
        lenient.warnings(),
        &[MapWarning::UnmodeledGameSemantics {
            game: WadGame::Strife
        }][..],
        "exactly one advisory, once per map"
    );
}

#[test]
fn non_strife_map_never_gets_the_advisory() {
    let wad = strife_flavored_map_wad(false);
    let group = &wad.map_groups()[0];
    let lenient =
        Map::assemble_with_options(&wad, group, ParseOptions::lenient()).expect("assembles");
    assert!(lenient.warnings().is_empty());
}

#[test]
fn udmf_map_in_strife_wad_gets_attribution_but_no_advisory() {
    let script = vec![0_u8; 1516];
    let textmap = b"namespace = \"zdoom\";\n";
    let lumps: Vec<(&str, &[u8])> = vec![
        ("MAP01", &[]),
        ("TEXTMAP", textmap.as_slice()),
        ("ENDMAP", &[]),
        ("SCRIPT01", script.as_slice()),
    ];
    let wad = Wad::from_bytes(common::build_wad(*b"PWAD", &lumps)).expect("parses");
    let group = &wad.map_groups()[0];
    let map = Map::assemble_with_options(&wad, group, ParseOptions::lenient()).expect("assembles");
    assert_eq!(map.format(), MapFormat::Udmf);
    assert_eq!(map.game(), Some(WadGame::Strife));
    assert!(map.warnings().is_empty(), "{:?}", map.warnings());
}

#[test]
fn hexen_map_in_strife_wad_gets_attribution_but_no_advisory() {
    // BEHAVIOR in the group makes this a Hexen-format map; the advisory is
    // Doom-format-only even when the WAD fingerprints as Strife.
    let script = vec![0_u8; 1516];
    let lumps: Vec<(&str, &[u8])> = vec![
        ("MAP01", &[]),
        ("THINGS", &[]),
        ("LINEDEFS", &[]),
        ("SIDEDEFS", &[]),
        ("VERTEXES", &[]),
        ("SEGS", &[]),
        ("SSECTORS", &[]),
        ("NODES", &[]),
        ("SECTORS", &[]),
        ("REJECT", &[]),
        ("BLOCKMAP", &[]),
        ("BEHAVIOR", &[]),
        ("SCRIPT01", script.as_slice()),
    ];
    let wad = Wad::from_bytes(common::build_wad(*b"PWAD", &lumps)).expect("parses");
    let group = &wad.map_groups()[0];
    let map = Map::assemble_with_options(&wad, group, ParseOptions::lenient()).expect("assembles");
    assert_eq!(map.format(), MapFormat::Hexen);
    assert_eq!(map.game(), Some(WadGame::Strife));
    assert!(map.warnings().is_empty(), "{:?}", map.warnings());
}
