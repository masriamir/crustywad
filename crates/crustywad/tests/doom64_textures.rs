//! Doom 64 texture-name hash + resolution table (#281, ADR-0022 §1/§4).

mod common;

use crustywad::map::texture_name_hash;
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
