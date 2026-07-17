//! Doom 64 texture-name hash + resolution table (#281, ADR-0022 §1/§4).

mod common;

use crustywad::map::{Map, MapAssembleError, MapWarning, TextureRef, texture_name_hash};
use crustywad::{ParseOptions, Wad};

/// A WAD whose directory is a `T_START..T_END` section holding `names`.
fn textures_wad(names: &[&str]) -> Wad {
    let mut lumps: Vec<(&str, &[u8])> = vec![("T_START", &[])];
    lumps.extend(names.iter().map(|n| (*n, &b""[..])));
    lumps.push(("T_END", &[]));
    Wad::from_bytes(common::build_wad(*b"IWAD", &lumps)).expect("fixture WAD parses")
}

#[test]
fn hash_matches_the_empirically_validated_vectors() {
    // Retail KEX MAP01 refs resolved against these exact pairs during the
    // #271 spike (82/82; ADR-0022 §1).
    for (name, hash) in [
        ("SDFLTAB", 32_u16),
        ("SDFLTAC", 33),
        ("SDFLTAD", 34),
        ("?", 111),
        ("SDOORA", 2712),
        ("SFLATAE", 4098),
    ] {
        assert_eq!(texture_name_hash(name), hash, "{name}");
    }
    // Case-insensitive (the engine uppercases).
    assert_eq!(texture_name_hash("sdoora"), 2712);
}

#[test]
fn table_resolves_by_hash_and_none_without_a_textures_section() {
    let wad = textures_wad(&["SDOORA", "SFLATAE"]);
    let table = wad
        .doom64_texture_names()
        .expect("clean sections")
        .expect("Textures section present");
    assert_eq!(table.get(2712), Some("SDOORA"));
    assert_eq!(table.get(4098), Some("SFLATAE"));
    assert_eq!(table.get(0xBEEF), None);
    assert_eq!(table.len(), 2);
    assert!(!table.is_empty());

    let bare = Wad::from_bytes(common::build_wad(*b"PWAD", &[("THINGS", &[])])).unwrap();
    assert!(bare.doom64_texture_names().unwrap().is_none());
}

#[test]
fn empty_textures_section_yields_an_empty_table_not_none() {
    let wad = textures_wad(&[]);
    let table = wad.doom64_texture_names().unwrap().expect("section exists");
    assert!(table.is_empty());
}

#[test]
fn first_match_in_disk_order_wins_on_collision() {
    // Find two distinct synthetic names with equal hashes by brute force —
    // 16-bit truncation guarantees collisions in a small search space.
    let mut seen: std::collections::HashMap<u16, String> = std::collections::HashMap::new();
    let mut pair: Option<(String, String)> = None;
    'outer: for a in b'A'..=b'Z' {
        for b in b'A'..=b'Z' {
            for c in b'A'..=b'Z' {
                for d in b'A'..=b'Z' {
                    let name = format!("TX{}{}{}{}", a as char, b as char, c as char, d as char);
                    let h = texture_name_hash(&name);
                    if let Some(prev) = seen.get(&h) {
                        pair = Some((prev.clone(), name));
                        break 'outer;
                    }
                    seen.insert(h, name);
                }
            }
        }
    }
    // 26^4 = 456,976 candidates over a 65,536-value space: pigeonhole
    // guarantees a collision (found long before exhaustion in practice).
    let (first, second) = pair.expect("26^4 names over a 16-bit space must collide");
    let wad = textures_wad(&[&first, &second]);
    let table = wad.doom64_texture_names().unwrap().unwrap();
    // First in disk order wins (engine parity); the collision collapses.
    assert_eq!(table.get(texture_name_hash(&first)), Some(first.as_str()));
    assert_eq!(table.len(), 1);
}

#[test]
fn duplicate_sections_concatenate_in_disk_order_lenient() {
    // Two complete T_ pairs (lenient row 4): resolution iterates both in
    // disk order; a name in the FIRST section beats a colliding name in
    // the second (here: the same name twice — first occurrence wins).
    let bytes = common::build_wad(
        *b"IWAD",
        &[
            ("T_START", &[]),
            ("SDOORA", &[]),
            ("T_END", &[]),
            ("T_START", &[]),
            ("SFLATAE", &[]),
            ("T_END", &[]),
        ],
    );
    let wad = Wad::from_bytes(bytes).unwrap();
    let table = wad
        .doom64_texture_names_with_options(ParseOptions::lenient())
        .unwrap()
        .unwrap();
    assert_eq!(table.get(2712), Some("SDOORA"));
    assert_eq!(table.get(4098), Some("SFLATAE"));
    // Strict errors on the duplicate pair (propagated SectionError).
    assert!(wad.doom64_texture_names().is_err());
}

/// A one-sector Doom 64 map wrapped in an outer IWAD with a T_ section of
/// `textures`. `sidedef` = [upper, lower, middle] hashes; `flats` =
/// [floor, ceiling] hashes. With a table present EVERY texture field is
/// resolved, so tests must make all five resolvable except the one under
/// test.
fn d64_map_in_textured_wad(sidedef: [u16; 3], flats: [u16; 2], textures: &[&str]) -> Wad {
    let nested_lumps = common::Doom64Lumps {
        linedefs: &common::d64_linedef(0, 1, 0, 0, 0xffff),
        sidedefs: &common::d64_sidedef(sidedef[0], sidedef[1], sidedef[2], 0),
        vertexes: &[common::d64_vertex(0.0, 0.0), common::d64_vertex(64.0, 0.0)].concat(),
        sectors: &common::d64_sector(flats[0], flats[1], [0; 5], 0),
        lights: &common::d64_light(0, 0, 0, 0),
        ..common::Doom64Lumps::default()
    };
    let bytes = common::build_doom64_wad_with_textures("MAP01", &nested_lumps, textures);
    Wad::from_bytes(bytes).expect("fixture WAD parses")
}

#[test]
fn assembly_resolves_hashes_to_names_when_the_section_is_present() {
    let wall = texture_name_hash("SDOORA");
    let flat = texture_name_hash("SFLATAE");
    let wad = d64_map_in_textured_wad([wall; 3], [flat; 2], &["SDOORA", "SFLATAE"]);
    let map = Map::assemble(&wad, &wad.map_group("MAP01").unwrap()).unwrap();
    assert!(map.warnings().is_empty());
    assert_eq!(map.sidedefs()[0].upper, TextureRef::Name("SDOORA".into()));
    assert_eq!(map.sidedefs()[0].middle, TextureRef::Name("SDOORA".into()));
    assert_eq!(
        map.sectors()[0].floor_flat,
        TextureRef::Name("SFLATAE".into())
    );
    assert_eq!(
        map.sectors()[0].ceiling_flat,
        TextureRef::Name("SFLATAE".into())
    );
}

#[test]
fn assembly_miss_with_section_strict_errors_lenient_keeps_index() {
    // Upper = 0xBEEF matches no name; every other field resolves, so the
    // miss under test is the only one.
    let wall = texture_name_hash("SDOORA");
    let flat = texture_name_hash("SFLATAE");
    let wad = d64_map_in_textured_wad([0xBEEF, wall, wall], [flat; 2], &["SDOORA", "SFLATAE"]);
    let group = wad.map_group("MAP01").unwrap();
    assert!(matches!(
        Map::assemble(&wad, &group).unwrap_err(),
        MapAssembleError::UnresolvedTextureHash {
            hash: 0xBEEF,
            from: "sidedef"
        }
    ));
    let map = Map::assemble_with_options(&wad, &group, ParseOptions::lenient()).unwrap();
    assert_eq!(map.sidedefs()[0].upper, TextureRef::Index(0xBEEF));
    assert!(map.warnings().iter().any(|w| matches!(
        w,
        MapWarning::UnresolvedTextureHash {
            hash: 0xBEEF,
            from: "sidedef"
        }
    )));
}

#[test]
fn assembly_sector_floor_miss_with_section_strict_errors_lenient_keeps_index() {
    // Floor flat = 0xBEEF matches no name; every other field (walls and the
    // ceiling flat) resolves, so the miss under test is the sector's floor
    // — the `from: "sector"` counterpart to the sidedef-miss test above,
    // isolating the resolve_texture_ref call site for MapSector::floor_flat.
    let wall = texture_name_hash("SDOORA");
    let flat = texture_name_hash("SFLATAE");
    let wad = d64_map_in_textured_wad([wall; 3], [0xBEEF, flat], &["SDOORA", "SFLATAE"]);
    let group = wad.map_group("MAP01").unwrap();
    assert!(matches!(
        Map::assemble(&wad, &group).unwrap_err(),
        MapAssembleError::UnresolvedTextureHash {
            hash: 0xBEEF,
            from: "sector"
        }
    ));
    let map = Map::assemble_with_options(&wad, &group, ParseOptions::lenient()).unwrap();
    assert_eq!(map.sectors()[0].floor_flat, TextureRef::Index(0xBEEF));
    assert!(map.warnings().iter().any(|w| matches!(
        w,
        MapWarning::UnresolvedTextureHash {
            hash: 0xBEEF,
            from: "sector"
        }
    )));
}

#[test]
fn assembly_sector_ceiling_miss_with_section_strict_errors_lenient_keeps_index() {
    // Ceiling flat = 0xBEEF matches no name; the floor flat resolves fine,
    // so the miss under test is specifically the ceiling — the
    // resolve_texture_ref call site for MapSector::ceiling_flat, reached
    // only when the preceding floor resolution succeeds.
    let wall = texture_name_hash("SDOORA");
    let flat = texture_name_hash("SFLATAE");
    let wad = d64_map_in_textured_wad([wall; 3], [flat, 0xBEEF], &["SDOORA", "SFLATAE"]);
    let group = wad.map_group("MAP01").unwrap();
    assert!(matches!(
        Map::assemble(&wad, &group).unwrap_err(),
        MapAssembleError::UnresolvedTextureHash {
            hash: 0xBEEF,
            from: "sector"
        }
    ));
    let map = Map::assemble_with_options(&wad, &group, ParseOptions::lenient()).unwrap();
    assert_eq!(map.sectors()[0].ceiling_flat, TextureRef::Index(0xBEEF));
    assert!(map.warnings().iter().any(|w| matches!(
        w,
        MapWarning::UnresolvedTextureHash {
            hash: 0xBEEF,
            from: "sector"
        }
    )));
}

#[test]
fn assembly_without_a_textures_section_keeps_index_silently() {
    // The plain nested-map builder has no outer T_ section: the pre-#281
    // behavior is preserved exactly for every existing fixture.
    let bytes = common::build_doom64_map_wad_from(
        "MAP01",
        &common::Doom64Lumps {
            linedefs: &common::d64_linedef(0, 1, 0, 0, 0xffff),
            sidedefs: &common::d64_sidedef(7, 0, 0, 0),
            vertexes: &[common::d64_vertex(0.0, 0.0), common::d64_vertex(64.0, 0.0)].concat(),
            sectors: &common::d64_sector(0, 0, [0; 5], 0),
            lights: &common::d64_light(0, 0, 0, 0),
            ..common::Doom64Lumps::default()
        },
    );
    let wad = Wad::from_bytes(bytes).unwrap();
    let map = Map::assemble(&wad, &wad.map_group("MAP01").unwrap()).unwrap();
    assert_eq!(map.sidedefs()[0].upper, TextureRef::Index(7));
    assert!(map.warnings().is_empty());
}

#[test]
fn assembly_bridges_lenient_section_warnings_and_strict_section_errors() {
    // Outer wad with a malformed texture section: unpaired T_START.
    let nested = common::build_doom64_nested_bytes(&common::Doom64Lumps::default());
    let bytes = common::build_wad(
        *b"IWAD",
        &[("T_START", &[]), ("AWALL", &[]), ("MAP01", &nested)],
    );
    let wad = Wad::from_bytes(bytes).unwrap();
    let group = wad.map_group("MAP01").unwrap();
    assert!(matches!(
        Map::assemble(&wad, &group).unwrap_err(),
        MapAssembleError::TextureSections { .. }
    ));
    let map = Map::assemble_with_options(&wad, &group, ParseOptions::lenient()).unwrap();
    assert!(
        map.warnings()
            .iter()
            .any(|w| matches!(w, MapWarning::TextureSection(_)))
    );
}

// ---------------------------------------------------------------------------
// Retail sweep smoke (#281): a real Doom 64 IWAD, gated on `sweep-tests` +
// `CRUSTYWAD_SWEEP_DIR` with the sweep suite's graceful-skip idiom
// (tests/sweep.rs / tests/sections.rs:498).
// ---------------------------------------------------------------------------

#[cfg(feature = "sweep-tests")]
#[test]
fn retail_doom64_map01_resolves_every_texture_ref() {
    let Some(dir) = std::env::var_os("CRUSTYWAD_SWEEP_DIR") else {
        eprintln!("skipping: CRUSTYWAD_SWEEP_DIR not set");
        return;
    };
    let dir = std::path::PathBuf::from(dir);
    // Match the sweep suite's graceful-skip gating: a set-but-unusable
    // variable (relative path — cargo runs tests from the package root —
    // or a non-directory) skips with a note rather than failing hard.
    if !dir.is_absolute() || !dir.is_dir() {
        eprintln!(
            "skipping: CRUSTYWAD_SWEEP_DIR is not an absolute path to a directory: {}",
            dir.display()
        );
        return;
    }
    let path = dir.join("DOOM64.WAD");
    if !path.is_file() {
        eprintln!("skipping: DOOM64.WAD not present in {}", dir.display());
        return;
    }

    let wad = Wad::from_path(&path).expect("DOOM64.WAD reads");
    let group = wad.map_group("MAP01").expect("retail MAP01 exists");
    let map = Map::assemble(&wad, &group).expect("retail MAP01 assembles strict-clean");
    assert!(map.warnings().is_empty());

    // Every texture field on every sidedef and sector resolved to a name.
    let mut refs = 0_usize;
    for side in map.sidedefs() {
        for r in [&side.upper, &side.lower, &side.middle] {
            assert!(
                matches!(r, TextureRef::Name(_)),
                "unresolved sidedef ref: {r:?}"
            );
            refs += 1;
        }
    }
    for sector in map.sectors() {
        for r in [&sector.floor_flat, &sector.ceiling_flat] {
            assert!(
                matches!(r, TextureRef::Name(_)),
                "unresolved sector ref: {r:?}"
            );
            refs += 1;
        }
    }
    assert!(refs > 0, "MAP01 must carry texture refs");
}
