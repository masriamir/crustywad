//! Conversion tests: UDMF ↔ Doom (ADR-0019).
//!
//! The headline guarantee is that Doom → UDMF → Doom is **byte-identical** for
//! all five data lumps, for any map inside the round-trip envelope documented on
//! [`crustywad::map::doom`]: linedef flags within the nine standard bits (0–8),
//! thing flags within the eight mapped bits (0–7), and thing angles already in
//! `0..360`. The reverse leg is deliberately **not** reversible — UDMF → Doom
//! rounds fractional coordinates and drops fields Doom has no slot for — and the
//! tests below assert that asymmetry rather than hide it.
#![cfg(feature = "write")]

mod common;

use proptest::prelude::*;

use crustywad::map::{DoomWriteWarning, write_doom_map};
use crustywad::map::{Map, MapFormat, Vertex, add_doom_map, add_udmf_map, parse_records};
use crustywad::{ParseOptions, Wad, WadBuilder, WadKind, WriteOptions};

/// Builds a PWAD holding one classic Doom map from raw lump bytes.
fn doom_wad(
    name: &str,
    things: &[u8],
    linedefs: &[u8],
    sidedefs: &[u8],
    vertexes: &[u8],
    sectors: &[u8],
) -> Wad {
    let bytes = common::build_doom_map_wad(
        name,
        things.to_vec(),
        linedefs.to_vec(),
        sidedefs.to_vec(),
        vertexes.to_vec(),
        sectors.to_vec(),
    );
    Wad::from_bytes(bytes).expect("the synthetic Doom map WAD should parse")
}

/// Builds a PWAD holding one UDMF map from `TEXTMAP` source.
fn udmf_wad(name: &str, textmap: &str) -> Wad {
    let bytes = common::build_named_lumps(&[
        (name, Vec::new()),
        ("TEXTMAP", textmap.as_bytes().to_vec()),
        ("ENDMAP", Vec::new()),
    ]);
    Wad::from_bytes(bytes).expect("the synthetic UDMF WAD should parse")
}

/// Assembles the map named `name` out of `wad`.
fn assemble(wad: &Wad, name: &str) -> Map {
    let group = wad.map_group(name).expect("the map group should be found");
    Map::assemble(wad, &group).expect("the map should assemble")
}

/// Converts an assembled map to UDMF, re-reads it, and returns the round-tripped
/// [`Map`] — the middle leg of every Doom → UDMF → Doom test below.
fn through_udmf(map: &Map, name: &str) -> Map {
    let mut builder = WadBuilder::new(WadKind::Pwad);
    add_udmf_map(&mut builder, name, map, &WriteOptions::strict()).expect("UDMF write should work");
    let wad = Wad::from_bytes(builder.build().expect("the UDMF WAD should build"))
        .expect("the UDMF WAD should parse");
    assemble(&wad, name)
}

// ---------------------------------------------------------------------------
// The round-trip identity guarantee
// ---------------------------------------------------------------------------

/// The headline guarantee of ADR-0019: Doom → UDMF → Doom is byte-identical for
/// all five data lumps.
#[test]
fn doom_to_udmf_to_doom_is_byte_identical() {
    // Two vertices, one two-sided linedef, two sidedefs, two sectors, one thing
    // — with every flag bit the round-trip envelope covers set (linedef bits
    // 0–8, thing bits 0–7).
    let vertexes = [0i16, 0, 64, 0]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect::<Vec<u8>>();

    let mut linedefs = Vec::new();
    linedefs.extend_from_slice(&0u16.to_le_bytes()); // start vertex
    linedefs.extend_from_slice(&1u16.to_le_bytes()); // end vertex
    linedefs.extend_from_slice(&0x01ffu16.to_le_bytes()); // flags: all 9 Doom bits
    linedefs.extend_from_slice(&11u16.to_le_bytes()); // special
    linedefs.extend_from_slice(&7u16.to_le_bytes()); // sector tag
    linedefs.extend_from_slice(&0u16.to_le_bytes()); // right sidedef
    linedefs.extend_from_slice(&1u16.to_le_bytes()); // left sidedef

    let mut sidedefs = Vec::new();
    for (sector, tex) in [(0u16, b"STARTAN3"), (1u16, b"BROWN96\0")] {
        sidedefs.extend_from_slice(&4i16.to_le_bytes());
        sidedefs.extend_from_slice(&(-8i16).to_le_bytes());
        sidedefs.extend_from_slice(b"-\0\0\0\0\0\0\0");
        sidedefs.extend_from_slice(b"-\0\0\0\0\0\0\0");
        sidedefs.extend_from_slice(tex);
        sidedefs.extend_from_slice(&sector.to_le_bytes());
    }

    let mut sectors = Vec::new();
    for (floor, tag) in [(0i16, 0i16), (-8i16, 3i16)] {
        sectors.extend_from_slice(&floor.to_le_bytes());
        sectors.extend_from_slice(&128i16.to_le_bytes());
        sectors.extend_from_slice(b"FLOOR4_8");
        sectors.extend_from_slice(b"CEIL3_5\0");
        sectors.extend_from_slice(&160i16.to_le_bytes());
        sectors.extend_from_slice(&0i16.to_le_bytes());
        sectors.extend_from_slice(&tag.to_le_bytes());
    }

    let mut things = Vec::new();
    things.extend_from_slice(&32i16.to_le_bytes()); // x
    things.extend_from_slice(&32i16.to_le_bytes()); // y
    things.extend_from_slice(&90u16.to_le_bytes()); // angle
    things.extend_from_slice(&1u16.to_le_bytes()); // type
    things.extend_from_slice(&0x00ffu16.to_le_bytes()); // all 8 mapped flag bits

    let wad = doom_wad("MAP01", &things, &linedefs, &sidedefs, &vertexes, &sectors);
    let doom_map = assemble(&wad, "MAP01");

    // Doom → UDMF (into a real WAD, so the group is re-detected on read) → Doom.
    let udmf_map = through_udmf(&doom_map, "MAP01");
    assert_eq!(udmf_map.format(), MapFormat::Udmf);
    let (lumps, warnings) = write_doom_map(&udmf_map, &WriteOptions::strict())
        .expect("the round-tripped map should be Doom-writable in strict mode");

    assert_eq!(lumps.vertexes, vertexes, "VERTEXES round-trip");
    assert_eq!(lumps.linedefs, linedefs, "LINEDEFS round-trip");
    assert_eq!(lumps.sidedefs, sidedefs, "SIDEDEFS round-trip");
    assert_eq!(lumps.sectors, sectors, "SECTORS round-trip");
    assert_eq!(lumps.things, things, "THINGS round-trip");

    // Nodes are never built — in either strictness mode.
    assert_eq!(warnings, vec![DoomWriteWarning::NodesNotBuilt]);
}

/// The round-trip envelope has edges, and they are documented rather than
/// silently tolerated: a linedef flag bit ≥ 9 (Boom's `passuse`, `0x200`) and a
/// thing flag bit ≥ 8 have no UDMF boolean, so the UDMF leg drops them, and a
/// thing angle ≥ 360 is normalized modulo 360. This test pins those
/// *limitations* — if it ever starts failing because the bits now survive, the
/// module docs on `map::doom` need updating too.
///
/// The angle case is not hypothetical: 226 things across 10 Freedoom maps carry
/// a literal `angle = 360` (see `tests/freedoom.rs`).
#[test]
fn values_outside_the_envelope_are_dropped_or_normalized_by_the_udmf_leg() {
    let vertexes = [0i16, 0, 64, 0]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect::<Vec<u8>>();

    let mut linedefs = Vec::new();
    linedefs.extend_from_slice(&0u16.to_le_bytes());
    linedefs.extend_from_slice(&1u16.to_le_bytes());
    linedefs.extend_from_slice(&0x0201u16.to_le_bytes()); // impassable + passuse (bit 9)
    linedefs.extend_from_slice(&0u16.to_le_bytes());
    linedefs.extend_from_slice(&0u16.to_le_bytes());
    linedefs.extend_from_slice(&0u16.to_le_bytes());
    linedefs.extend_from_slice(&0xffffu16.to_le_bytes()); // one-sided

    let mut sidedefs = Vec::new();
    sidedefs.extend_from_slice(&0i16.to_le_bytes());
    sidedefs.extend_from_slice(&0i16.to_le_bytes());
    sidedefs.extend_from_slice(b"-\0\0\0\0\0\0\0");
    sidedefs.extend_from_slice(b"-\0\0\0\0\0\0\0");
    sidedefs.extend_from_slice(b"STARTAN3");
    sidedefs.extend_from_slice(&0u16.to_le_bytes());

    let mut sectors = Vec::new();
    sectors.extend_from_slice(&0i16.to_le_bytes());
    sectors.extend_from_slice(&128i16.to_le_bytes());
    sectors.extend_from_slice(b"FLOOR4_8");
    sectors.extend_from_slice(b"CEIL3_5\0");
    sectors.extend_from_slice(&160i16.to_le_bytes());
    sectors.extend_from_slice(&0i16.to_le_bytes());
    sectors.extend_from_slice(&0i16.to_le_bytes());

    let mut things = Vec::new();
    things.extend_from_slice(&32i16.to_le_bytes());
    things.extend_from_slice(&32i16.to_le_bytes());
    things.extend_from_slice(&720u16.to_le_bytes()); // angle ≥ 360: normalized to 0
    things.extend_from_slice(&1u16.to_le_bytes());
    things.extend_from_slice(&0x0102u16.to_le_bytes()); // skill3 (bit 1) + bit 8

    let wad = doom_wad("MAP01", &things, &linedefs, &sidedefs, &vertexes, &sectors);
    let udmf_map = through_udmf(&assemble(&wad, "MAP01"), "MAP01");
    let (lumps, _) = write_doom_map(&udmf_map, &WriteOptions::strict()).unwrap();

    // Geometry still round-trips exactly; only the out-of-envelope bits are lost.
    assert_eq!(lumps.vertexes, vertexes, "VERTEXES round-trip");
    assert_eq!(lumps.sidedefs, sidedefs, "SIDEDEFS round-trip");
    assert_eq!(lumps.sectors, sectors, "SECTORS round-trip");

    let out_linedefs: Vec<crustywad::map::doom::Linedef> = parse_records(&lumps.linedefs).unwrap();
    assert_eq!(
        out_linedefs[0].flags, 0x0001,
        "linedef flag bit 9 (passuse) has no UDMF boolean and is dropped"
    );

    let out_things: Vec<crustywad::map::doom::Thing> = parse_records(&lumps.things).unwrap();
    assert_eq!(
        out_things[0].flags, 0x0002,
        "thing flag bit 8 has no UDMF boolean and is dropped"
    );
    assert_eq!(
        out_things[0].angle, 0,
        "a thing angle of 720 is normalized modulo 360 on the way out to UDMF"
    );
}

// ---------------------------------------------------------------------------
// The one-way leg: UDMF → Doom
// ---------------------------------------------------------------------------

/// A UDMF map with ZDoom-style extensions fails strict conversion (ADR-0019
/// tier 3) and converts with warnings in lenient mode.
#[test]
fn zdoom_style_udmf_needs_lenient_to_convert() {
    let textmap = r#"
        namespace = "zdoom";
        vertex { x = 0; y = 0; }
        vertex { x = 64; y = 0; }
        sector { texturefloor = "FLOOR4_8"; textureceiling = "CEIL3_5"; }
        sidedef { sector = 0; }
        linedef { v1 = 0; v2 = 1; sidefront = 0; special = 80; arg0 = 1; arg1 = 2; }
        thing { x = 32; y = 32; type = 1; height = 16; id = 42; }
    "#;
    let wad = udmf_wad("MAP01", textmap);
    let map = assemble(&wad, "MAP01");

    // Strict: the first unrepresentable field aborts the conversion.
    assert!(write_doom_map(&map, &WriteOptions::strict()).is_err());

    // Lenient: it converts, and every dropped field is reported.
    let (lumps, warnings) = write_doom_map(&map, &WriteOptions::lenient()).unwrap();
    assert!(!lumps.linedefs.is_empty());
    let dropped: Vec<String> = warnings.iter().map(ToString::to_string).collect();
    assert!(dropped.iter().any(|w| w.contains("arg1")), "{dropped:?}");
    assert!(dropped.iter().any(|w| w.contains("height")), "{dropped:?}");
    assert!(dropped.iter().any(|w| w.contains("id")), "{dropped:?}");

    // What Doom *can* hold survives: the special and its first arg (the tag).
    let linedefs: Vec<crustywad::map::doom::Linedef> = parse_records(&lumps.linedefs).unwrap();
    assert_eq!(linedefs[0].special_type, 80);
    assert_eq!(linedefs[0].sector_tag, 1);
}

/// Fractional UDMF geometry rounds in lenient mode — the one-way half of the
/// conversion (ADR-0019: UDMF → Doom is not reversible).
#[test]
fn fractional_udmf_geometry_rounds_in_lenient() {
    let textmap = r#"
        namespace = "doom";
        vertex { x = 0.4; y = 0.0; }
        vertex { x = 63.5; y = 0.0; }
        sector { texturefloor = "FLOOR4_8"; textureceiling = "CEIL3_5"; }
        sidedef { sector = 0; }
        linedef { v1 = 0; v2 = 1; sidefront = 0; }
    "#;
    let wad = udmf_wad("MAP01", textmap);
    let map = assemble(&wad, "MAP01");

    assert!(write_doom_map(&map, &WriteOptions::strict()).is_err());

    let (lumps, warnings) = write_doom_map(&map, &WriteOptions::lenient()).unwrap();
    let vertices: Vec<Vertex> = parse_records(&lumps.vertexes).unwrap();
    assert_eq!(vertices[0].x, 0, "0.4 rounds to 0");
    assert_eq!(vertices[1].x, 64, "63.5 rounds to 64 (half away from zero)");

    // The rounding is reported, not silent.
    let rounded: Vec<String> = warnings.iter().map(ToString::to_string).collect();
    assert!(
        rounded.iter().any(|w| w.contains("rounded")),
        "the coordinate rounding must be reported: {rounded:?}"
    );
}

/// The converted map is a complete, re-readable Doom map group.
#[test]
fn converted_map_reassembles_from_the_output_wad() {
    let textmap = r#"
        namespace = "doom";
        vertex { x = 0; y = 0; }
        vertex { x = 64; y = 0; }
        sector { texturefloor = "FLOOR4_8"; textureceiling = "CEIL3_5"; }
        sidedef { sector = 0; }
        linedef { v1 = 0; v2 = 1; sidefront = 0; }
        thing { x = 32; y = 32; type = 1; skill3 = true; }
    "#;
    let wad = udmf_wad("MAP01", textmap);
    let map = assemble(&wad, "MAP01");

    let mut out = WadBuilder::new(WadKind::Pwad);
    add_doom_map(&mut out, "MAP01", &map, &WriteOptions::strict()).unwrap();
    let out_wad = Wad::from_bytes(out.build().unwrap()).unwrap();

    let group = out_wad.map_group("MAP01").unwrap();
    let reassembled = Map::assemble_with_options(&out_wad, &group, ParseOptions::strict()).unwrap();
    assert_eq!(reassembled.format(), MapFormat::Doom);
    assert_eq!(reassembled.vertices().len(), 2);
    assert_eq!(reassembled.linedefs().len(), 1);
    assert_eq!(reassembled.sectors().len(), 1);
    assert_eq!(reassembled.things()[0].flags, 0b10, "skill3");
}

// ---------------------------------------------------------------------------
// Property test
// ---------------------------------------------------------------------------

proptest! {
    /// For any Doom map whose values sit inside the round-trip envelope,
    /// Doom → UDMF → Doom reproduces the source lumps byte for byte.
    ///
    /// The generators stay inside that envelope on purpose: thing flags are
    /// drawn from the eight mapped bits (`0..0xff`) and linedef flags from the
    /// nine standard bits (`0..0x200`), because bits above those have no UDMF
    /// boolean and are dropped by the UDMF leg by design (see the `map::doom`
    /// module docs). Generating them would test that documented limitation, not
    /// the conversion.
    #[test]
    fn doom_udmf_doom_identity(
        xs in prop::collection::vec(-1000i16..1000, 2..8),
        thing_flags in 0u16..0x0100,
        line_flags in 0u16..0x0200,
        angle in 0u16..360,
        light in 0i16..256,
        tag in 0i16..64,
    ) {
        let vertexes: Vec<u8> = xs.iter().flat_map(|x| {
            let mut v = x.to_le_bytes().to_vec();
            v.extend_from_slice(&0i16.to_le_bytes());
            v
        }).collect();

        let mut sidedefs = Vec::new();
        sidedefs.extend_from_slice(&0i16.to_le_bytes());
        sidedefs.extend_from_slice(&0i16.to_le_bytes());
        sidedefs.extend_from_slice(b"-\0\0\0\0\0\0\0");
        sidedefs.extend_from_slice(b"-\0\0\0\0\0\0\0");
        sidedefs.extend_from_slice(b"STARTAN3");
        sidedefs.extend_from_slice(&0u16.to_le_bytes());

        let mut sectors = Vec::new();
        sectors.extend_from_slice(&0i16.to_le_bytes());
        sectors.extend_from_slice(&128i16.to_le_bytes());
        sectors.extend_from_slice(b"FLOOR4_8");
        sectors.extend_from_slice(b"CEIL3_5\0");
        sectors.extend_from_slice(&light.to_le_bytes());
        sectors.extend_from_slice(&0i16.to_le_bytes());
        sectors.extend_from_slice(&tag.to_le_bytes());

        // A one-sided linedef: the 0xFFFF back reference is Doom's "no sidedef"
        // sentinel, which must survive the round-trip as an absent sidedef.
        let mut linedefs = Vec::new();
        linedefs.extend_from_slice(&0u16.to_le_bytes()); // v1
        linedefs.extend_from_slice(&1u16.to_le_bytes()); // v2
        linedefs.extend_from_slice(&line_flags.to_le_bytes());
        linedefs.extend_from_slice(&0u16.to_le_bytes()); // special
        let tag_u16 = u16::try_from(tag).expect("tag is generated in 0..64");
        linedefs.extend_from_slice(&tag_u16.to_le_bytes()); // sector tag
        linedefs.extend_from_slice(&0u16.to_le_bytes()); // right sidedef
        linedefs.extend_from_slice(&0xffffu16.to_le_bytes()); // no left sidedef

        let mut things = Vec::new();
        things.extend_from_slice(&0i16.to_le_bytes());
        things.extend_from_slice(&0i16.to_le_bytes());
        things.extend_from_slice(&angle.to_le_bytes());
        things.extend_from_slice(&1u16.to_le_bytes());
        things.extend_from_slice(&thing_flags.to_le_bytes());

        let wad = doom_wad("MAP01", &things, &linedefs, &sidedefs, &vertexes, &sectors);
        let map = assemble(&wad, "MAP01");
        let udmf_map = through_udmf(&map, "MAP01");
        let (lumps, _) = write_doom_map(&udmf_map, &WriteOptions::strict())
            .expect("an in-envelope Doom map must survive the round-trip in strict mode");

        prop_assert_eq!(lumps.vertexes, vertexes);
        prop_assert_eq!(lumps.things, things);
        prop_assert_eq!(lumps.sectors, sectors);
        prop_assert_eq!(lumps.linedefs, linedefs);
        prop_assert_eq!(lumps.sidedefs, sidedefs);
    }
}
