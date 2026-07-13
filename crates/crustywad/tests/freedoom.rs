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

/// Every real Freedoom map must survive Doom → UDMF → Doom with full fidelity:
/// the round-trip guarantee of ADR-0019, held against real-world geometry rather
/// than synthetic fixtures. Every map in both IWADs is checked.
///
/// `VERTEXES`, `LINEDEFS`, `SIDEDEFS`, and `SECTORS` must be **byte-identical**,
/// with no exception. `THINGS` must be byte-identical too, save for the single
/// difference the conversion documents and Freedoom actually exercises:
///
/// > A thing `angle` ≥ 360 is normalized modulo 360 on the way out to UDMF.
/// > — the "Round-tripping" section of the `crustywad::map::doom` module docs
///
/// 226 things across 10 Freedoom maps (E1M7, E4M1–E4M3, E4M5, E4M6, E4M8, E4M9,
/// MAP28, MAP32) carry a literal `angle = 360`, which comes back as `0`. That is
/// a semantic no-op — 360° and 0° are the same facing — but it *is* a byte-level
/// difference, so this test states it precisely instead of hiding it: where a
/// thing differs, the angle must be the *only* field that differs, the source
/// angle must be out of the `0..360` envelope, and the round-tripped angle must
/// be exactly `source % 360`. Any other divergence — a lost flag bit, a moved
/// vertex, a renamed texture — fails the test.
///
/// The comparison is `write_doom_map(original)` vs `write_doom_map(round_tripped)`,
/// **not** the original on-disk lump bytes: a real IWAD's `SIDEDEFS`/`SECTORS`
/// can carry trailing garbage after the NUL terminator inside an 8-byte name
/// field, which `Name8` preserves on read but the writer re-emits as clean NUL
/// padding. Comparing writer output to writer output isolates conversion
/// fidelity from that pre-existing (and harmless) normalization.
#[test]
#[cfg(feature = "write")]
fn freedoom_maps_round_trip_through_udmf() {
    use crustywad::map::doom::Thing;
    use crustywad::map::{Map, add_udmf_map, parse_records, write_doom_map};
    use crustywad::{WadBuilder, WadKind, WriteOptions};

    for path in common::iwad_files(
        "CRUSTYWAD_FREEDOOM_DIR",
        &["freedoom1.wad", "freedoom2.wad"],
    ) {
        let wad = Wad::from_path(&path).expect("fixture should parse");
        for group in wad.map_groups() {
            let name = &group.name;
            let map = Map::assemble(&wad, &group).expect("fixture map should assemble");
            let (before, _) = write_doom_map(&map, &WriteOptions::strict())
                .expect("a Doom-sourced map is always Doom-writable");

            let mut builder = WadBuilder::new(WadKind::Pwad);
            add_udmf_map(&mut builder, name, &map, &WriteOptions::strict())
                .expect("UDMF write should succeed");
            let udmf_wad = Wad::from_bytes(builder.build().expect("the UDMF WAD should build"))
                .expect("UDMF WAD should parse");
            let udmf_group = udmf_wad
                .map_group(name)
                .expect("the UDMF group should be detected");
            let udmf_map = Map::assemble(&udmf_wad, &udmf_group).expect("UDMF map should assemble");

            let (after, _) = write_doom_map(&udmf_map, &WriteOptions::strict())
                .expect("the round-tripped map should still be Doom-writable");

            // Geometry: byte-identical, unconditionally.
            assert_eq!(before.vertexes, after.vertexes, "{name} VERTEXES");
            assert_eq!(before.linedefs, after.linedefs, "{name} LINEDEFS");
            assert_eq!(before.sidedefs, after.sidedefs, "{name} SIDEDEFS");
            assert_eq!(before.sectors, after.sectors, "{name} SECTORS");

            // Things: byte-identical, except for the documented angle normalization.
            if before.things == after.things {
                continue;
            }
            let before_things: Vec<Thing> = parse_records(&before.things).unwrap();
            let after_things: Vec<Thing> = parse_records(&after.things).unwrap();
            assert_eq!(
                before_things.len(),
                after_things.len(),
                "{name} THINGS count must not change"
            );
            for (i, (b, a)) in before_things.iter().zip(&after_things).enumerate() {
                assert_eq!(
                    (b.x, b.y, b.type_id, b.flags),
                    (a.x, a.y, a.type_id, a.flags),
                    "{name} thing #{i}: angle is the only field the round-trip may change"
                );
                if b.angle == a.angle {
                    continue;
                }
                assert!(
                    b.angle >= 360,
                    "{name} thing #{i}: an in-envelope angle ({}) must round-trip exactly, but \
                     became {}",
                    b.angle,
                    a.angle
                );
                assert_eq!(
                    b.angle % 360,
                    a.angle,
                    "{name} thing #{i}: an out-of-envelope angle must be normalized modulo 360"
                );
            }
        }
    }
}
