//! Wad namespace/section API (#280, ADR-0022 §2): marker grammar, the
//! pairing scan, and the eight-row strict/lenient policy table.

mod common;

use crustywad::{ParseOptions, SectionError, SectionKind, SectionWarning, Wad};

/// Builds a PWAD whose directory is exactly `names`, every lump zero-size
/// except entries suffixed with "+", which get one content byte (proving
/// size is not a marker signal).
fn wad_of(names: &[&str]) -> Wad {
    let lumps: Vec<(&str, &[u8])> = names
        .iter()
        .map(|n| match n.strip_suffix('+') {
            Some(base) => (base, &b"x"[..]),
            None => (*n, &b""[..]),
        })
        .collect();
    Wad::from_bytes(common::build_wad(*b"PWAD", &lumps)).expect("fixture WAD parses")
}

fn lenient(wad: &Wad) -> crustywad::SectionTable {
    wad.sections_with_options(ParseOptions::lenient())
        .expect("lenient never errors")
}

// --- the empirical DOOM.WAD shape (research §7b) ---

#[test]
fn doom_shape_yields_nested_sections_engine_parity_ranges() {
    let wad = wad_of(&[
        "PLAYPAL", "S_START", "TROOA1", "TROOA2", "S_END", "P_START", "P1_START", "WALL01",
        "P1_END", "P2_START", "WALL02", "P2_END", "P_END", "F_START", "F1_START", "FLOOR1",
        "F1_END", "F2_START", "FLOOR2", "F2_END", "F_END",
    ]);
    let table = wad.sections().expect("well-formed layout is strict-clean");
    assert!(table.warnings().is_empty());
    assert_eq!(table.sections().len(), 3);

    let sprites = &table.sections()[0];
    assert_eq!(sprites.kind, SectionKind::Sprites);
    assert_eq!((sprites.start_marker, sprites.end_marker), (1, 4));
    assert_eq!(sprites.lumps, 2..4);
    assert!(sprites.sub_sections.is_empty());

    let patches = &table.sections()[1];
    assert_eq!(patches.kind, SectionKind::Patches);
    // Engine-parity outer extent includes the nested sub-pair markers.
    assert_eq!((patches.start_marker, patches.end_marker), (5, 12));
    assert_eq!(patches.lumps, 6..12);
    assert_eq!(patches.sub_sections.len(), 2);
    assert_eq!(patches.sub_sections[0].lumps, 7..8);
    assert_eq!(patches.sub_sections[1].lumps, 10..11);

    let flats = &table.sections()[2];
    assert_eq!(flats.kind, SectionKind::Flats);
    assert_eq!(flats.sub_sections.len(), 2);

    // of_kind filters top-level sections.
    assert_eq!(table.of_kind(SectionKind::Flats).count(), 1);
    assert_eq!(table.of_kind(SectionKind::Textures).count(), 0);
}

#[test]
fn doom64_shape_with_trailers_and_absent_graphics() {
    let wad = wad_of(&[
        "S_START",
        "SARGA1",
        "S_END",
        "T_START",
        "AWALL",
        "BWALL",
        "T_END",
        "DS_START",
        "DSPISTOL",
        "DS_END",
        "MAP01",
        "CHECKSUM+",
        "ENDOFWAD",
    ]);
    let table = wad.sections().unwrap();
    assert!(table.warnings().is_empty());
    let kinds: Vec<SectionKind> = table.sections().iter().map(|s| s.kind).collect();
    assert_eq!(
        kinds,
        vec![
            SectionKind::Sprites,
            SectionKind::Textures,
            SectionKind::Sounds
        ]
    );
    // CHECKSUM (even with content bytes) and ENDOFWAD are ignored, MAP01 is content.
    assert_eq!(table.of_kind(SectionKind::Graphics).count(), 0);
}

#[test]
fn graphics_pair_when_present_is_a_section() {
    let wad = wad_of(&["G_START", "SFONT", "G_END"]);
    let table = wad.sections().unwrap();
    assert_eq!(table.sections()[0].kind, SectionKind::Graphics);
}

// --- alias grammar (Boom IsMarker; research §7b) ---

#[test]
fn doubled_aliases_normalize_and_mixed_pairs_pair_up() {
    // FF_START..FF_END and a mixed PP_START..P_END both form sections.
    let wad = wad_of(&["FF_START", "FLOORA", "FF_END", "PP_START", "WALLA", "P_END"]);
    let table = wad.sections().unwrap();
    assert!(table.warnings().is_empty());
    assert_eq!(table.sections()[0].kind, SectionKind::Flats);
    assert_eq!(table.sections()[1].kind, SectionKind::Patches);
}

#[test]
fn multi_char_prefixes_never_double_and_junk_is_content() {
    // DSDS_START and TT_START are ordinary content lumps, not markers.
    let wad = wad_of(&[
        "DSDS_START",
        "TT_START",
        "FX_START",
        "F10_START",
        "F0_START",
    ]);
    let table = wad.sections().unwrap();
    assert!(table.sections().is_empty());
    assert!(table.warnings().is_empty());
}

#[test]
fn marker_with_content_bytes_is_still_a_marker() {
    // Size is not a signal (vanilla never checks it).
    let wad = wad_of(&["F_START+", "FLOORA", "F_END"]);
    let table = wad.sections().unwrap();
    assert_eq!(table.sections()[0].lumps, 1..2);
}

#[test]
fn empty_and_marker_free_wads_yield_empty_tables() {
    for names in [&[][..], &["THINGS", "VERTEXES"][..]] {
        let wad = wad_of(names);
        let table = wad.sections().unwrap();
        assert!(table.sections().is_empty());
        assert!(table.warnings().is_empty());
    }
}

// --- policy table rows (spec §Pairing), strict + lenient each ---

#[test]
fn row1_unpaired_start_at_eof() {
    let wad = wad_of(&["F_START", "FLOORA", "FLOORB"]);
    assert!(matches!(
        wad.sections().unwrap_err(),
        SectionError::UnpairedStart {
            kind: SectionKind::Flats,
            index: 0
        }
    ));
    let table = lenient(&wad);
    let s = &table.sections()[0];
    // EOF-closed: end_marker == directory length, no closing lump.
    assert_eq!((s.start_marker, s.end_marker), (0, 3));
    assert_eq!(s.lumps, 1..3);
    assert!(matches!(
        table.warnings()[0],
        SectionWarning::UnpairedStart {
            kind: SectionKind::Flats,
            index: 0
        }
    ));
}

#[test]
fn row2_sub_pair_unclosed_before_parent_end() {
    let wad = wad_of(&["F_START", "F1_START", "FLOORA", "F_END"]);
    assert!(matches!(
        wad.sections().unwrap_err(),
        SectionError::UnpairedStart {
            kind: SectionKind::Flats,
            index: 1
        }
    ));
    let table = lenient(&wad);
    let outer = &table.sections()[0];
    assert_eq!(outer.sub_sections.len(), 1);
    // Child closed at the parent's END marker.
    assert_eq!(outer.sub_sections[0].lumps, 2..3);
    assert_eq!(table.warnings().len(), 1);
}

#[test]
fn row3_unpaired_end_is_ignored() {
    let wad = wad_of(&["FLOORA", "F_END", "S_END"]);
    assert!(matches!(
        wad.sections().unwrap_err(),
        SectionError::UnpairedEnd {
            kind: SectionKind::Flats,
            index: 1
        }
    ));
    let table = lenient(&wad);
    assert!(table.sections().is_empty());
    assert_eq!(table.warnings().len(), 2);
}

#[test]
fn row4_duplicate_sibling_pairs() {
    let wad = wad_of(&["F_START", "A", "F_END", "F_START", "B", "F_END"]);
    assert!(matches!(
        wad.sections().unwrap_err(),
        SectionError::DuplicatePair {
            kind: SectionKind::Flats,
            first_start: 0,
            second_start: 3
        }
    ));
    let table = lenient(&wad);
    assert_eq!(table.sections().len(), 2);
    assert_eq!(table.of_kind(SectionKind::Flats).count(), 2);
    assert_eq!(table.warnings().len(), 1);
}

#[test]
fn row5_same_kind_start_while_open_is_ignored_leniently() {
    // Naive-merge shape: F_START F_START .. F_END F_END. The inner START is
    // ignored; the outer closes at the FIRST END; the second END is then
    // row 3 (unpaired, ignored). Alias spelling exercises normalization.
    let wad = wad_of(&["F_START", "FF_START", "FLOORA", "F_END", "F_END"]);
    assert!(matches!(
        wad.sections().unwrap_err(),
        SectionError::NestedDuplicate {
            kind: SectionKind::Flats,
            outer_start: 0,
            inner_start: 1
        }
    ));
    let table = lenient(&wad);
    assert_eq!(table.sections().len(), 1);
    let s = &table.sections()[0];
    assert_eq!((s.start_marker, s.end_marker), (0, 3));
    assert_eq!(table.warnings().len(), 2); // NestedDuplicate + trailing UnpairedEnd
}

#[test]
fn row6_cross_kind_interleave_closes_matching_open() {
    let wad = wad_of(&["S_START", "SARGA1", "F_START", "S_END", "FLOORA", "F_END"]);
    assert!(matches!(
        wad.sections().unwrap_err(),
        SectionError::Interleaved {
            open_kind: SectionKind::Flats,
            closing_kind: SectionKind::Sprites,
            index: 3,
        }
    ));
    let table = lenient(&wad);
    assert_eq!(table.sections().len(), 2);
    let sprites = table.of_kind(SectionKind::Sprites).next().unwrap();
    assert_eq!((sprites.start_marker, sprites.end_marker), (0, 3));
    let flats = table.of_kind(SectionKind::Flats).next().unwrap();
    assert_eq!((flats.start_marker, flats.end_marker), (2, 5));
    assert_eq!(table.warnings().len(), 1); // ONE Interleaved per END marker
}

#[test]
fn rows7_8_orphan_sub_pairs_promote() {
    // Row 7: no parent at all. Row 8: wrong parent.
    let orphan = wad_of(&["F1_START", "FLOORA", "F1_END"]);
    assert!(matches!(
        orphan.sections().unwrap_err(),
        SectionError::OrphanSubPair {
            kind: SectionKind::Flats,
            index: 0
        }
    ));
    let table = lenient(&orphan);
    assert_eq!(table.sections().len(), 1);
    assert_eq!(table.sections()[0].kind, SectionKind::Flats);
    assert!(table.sections()[0].sub_sections.is_empty());

    let wrong_parent = wad_of(&["P_START", "F1_START", "FLOORA", "F1_END", "P_END"]);
    assert!(matches!(
        wrong_parent.sections().unwrap_err(),
        SectionError::OrphanSubPair {
            kind: SectionKind::Flats,
            index: 1
        }
    ));
    let table = lenient(&wrong_parent);
    // Promoted flats section is top-level; patches has no children.
    assert_eq!(table.of_kind(SectionKind::Flats).count(), 1);
    assert!(
        table
            .of_kind(SectionKind::Patches)
            .next()
            .unwrap()
            .sub_sections
            .is_empty()
    );
}

#[test]
fn sprite_numbered_subs_are_grammar_admitted() {
    // Never observed in retail (research §7b) but the grammar admits them.
    let wad = wad_of(&["S_START", "S1_START", "TROOA1", "S1_END", "S_END"]);
    let table = wad.sections().unwrap();
    assert_eq!(table.sections()[0].sub_sections.len(), 1);
}

use proptest::prelude::*;

proptest! {
    #[test]
    fn well_formed_layouts_scan_strict_clean_and_round_trip(
        layout in proptest::collection::vec(
            (
                prop_oneof![
                    Just(("S_START", "S_END", SectionKind::Sprites)),
                    Just(("F_START", "F_END", SectionKind::Flats)),
                    Just(("P_START", "P_END", SectionKind::Patches)),
                    Just(("T_START", "T_END", SectionKind::Textures)),
                    Just(("DS_START", "DS_END", SectionKind::Sounds)),
                    Just(("G_START", "G_END", SectionKind::Graphics)),
                ],
                0_usize..3, // content lumps inside
            ),
            0..4,
        )
    ) {
        // One section per DISTINCT kind (duplicates would be row 4).
        let mut seen = std::collections::HashSet::new();
        let mut names: Vec<String> = Vec::new();
        let mut expected: Vec<(SectionKind, usize)> = Vec::new();
        for ((start, end, kind), content) in &layout {
            if !seen.insert(*kind) { continue; }
            names.push((*start).to_owned());
            for c in 0..*content {
                names.push(format!("LUMP{c}"));
            }
            names.push((*end).to_owned());
            expected.push((*kind, *content));
        }
        let name_refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let wad = wad_of(&name_refs);
        let table = wad.sections().expect("well-formed layout is strict-clean");
        prop_assert!(table.warnings().is_empty());
        prop_assert_eq!(table.sections().len(), expected.len());
        for (section, (kind, content)) in table.sections().iter().zip(&expected) {
            prop_assert_eq!(section.kind, *kind);
            prop_assert_eq!(section.lumps.len(), *content);
            prop_assert!(section.sub_sections.is_empty());
        }
    }
}
