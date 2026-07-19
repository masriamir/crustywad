//! Public-API tests for the `nodebuild` node-lump builders (ADR-0024 §9.1).
//!
//! Task 1 covers the zero-fill REJECT builder: assemble a real WAD through the
//! public path, build its REJECT, and round-trip the bytes back through
//! [`MapReject::parse`] (ADR-0024 §7 / Global Constraint 4).
#![cfg(feature = "nodebuild")]

mod common;

use crustywad::map::build::{NodeBuildError, NodeBuildOptions, build_blockmap, build_reject};
use crustywad::map::{Map, MapBlockmap, MapReject, MapWarning};
use crustywad::{Strictness, Wad};

/// Encodes a Doom 8-byte name field, NUL-padded on the right.
fn name8(name: &str) -> [u8; 8] {
    let mut out = [0u8; 8];
    for (slot, byte) in out.iter_mut().zip(name.as_bytes()) {
        *slot = *byte;
    }
    out
}

/// One classic `LINEDEFS` record (14 bytes, all `u16` fields).
fn linedef_bytes(
    start_vertex: u16,
    end_vertex: u16,
    flags: u16,
    special_type: u16,
    sector_tag: u16,
    right_sidedef: u16,
    left_sidedef: u16,
) -> Vec<u8> {
    [
        start_vertex,
        end_vertex,
        flags,
        special_type,
        sector_tag,
        right_sidedef,
        left_sidedef,
    ]
    .iter()
    .flat_map(|v| v.to_le_bytes())
    .collect()
}

/// One `SIDEDEFS` record (30 bytes): offsets, three 8-byte texture names, then
/// the sector index.
fn sidedef_bytes(upper: &str, lower: &str, middle: &str, sector: u16) -> Vec<u8> {
    [
        &0i16.to_le_bytes()[..],
        &0i16.to_le_bytes(),
        &name8(upper),
        &name8(lower),
        &name8(middle),
        &sector.to_le_bytes(),
    ]
    .concat()
}

/// One `THINGS` record (10 bytes): x, y (`i16`), angle/type/flags (`u16`).
fn thing_bytes(x: i16, y: i16, angle: u16, type_id: u16, flags: u16) -> Vec<u8> {
    [
        &x.to_le_bytes()[..],
        &y.to_le_bytes(),
        &angle.to_le_bytes(),
        &type_id.to_le_bytes(),
        &flags.to_le_bytes(),
    ]
    .concat()
}

/// `VERTEXES` records (4 bytes each) from `(x, y)` pairs.
fn vertexes_bytes(points: &[(i16, i16)]) -> Vec<u8> {
    points
        .iter()
        .flat_map(|(x, y)| [x.to_le_bytes(), y.to_le_bytes()].concat())
        .collect()
}

/// One `SECTORS` record (26 bytes): heights, two 8-byte flat names, light,
/// special, tag.
fn sector_bytes(tag: i16) -> Vec<u8> {
    [
        &0i16.to_le_bytes()[..],
        &128i16.to_le_bytes(),
        &name8("FLOOR4_8"),
        &name8("CEIL3_5"),
        &160i16.to_le_bytes(),
        &0i16.to_le_bytes(),
        &tag.to_le_bytes(),
    ]
    .concat()
}

/// Assembles a one-linedef classic Doom map carrying `n` sectors. Only sector 0
/// is referenced by a sidedef; the rest are unreferenced (valid — assembly does
/// not require every sector to be used), so `map.sectors().len() == n`.
fn map_with_sectors(n: usize) -> Map {
    let mut sectors = Vec::new();
    for i in 0..n {
        sectors.extend(sector_bytes(i16::try_from(i).unwrap()));
    }
    let bytes = common::build_doom_map_wad(
        "MAP01",
        thing_bytes(32, 32, 0, 1, 7),
        linedef_bytes(0, 1, 1, 0, 0, 0, 0xffff),
        sidedef_bytes("-", "-", "STARTAN3", 0),
        vertexes_bytes(&[(0, 0), (64, 0)]),
        sectors,
    );
    let wad = Wad::from_bytes(bytes).expect("fixture WAD parses");
    let group = wad.map_group("MAP01").expect("map group present");
    Map::assemble(&wad, &group).expect("map assembles")
}

#[test]
fn build_reject_sizes_match_the_sector_count() {
    // ceil(n² / 8): 1 -> 1 byte, 3 -> 2 bytes, 8 -> 8 bytes.
    assert_eq!(build_reject(&map_with_sectors(1)).to_lump_bytes().len(), 1);
    assert_eq!(build_reject(&map_with_sectors(3)).to_lump_bytes().len(), 2);
    assert_eq!(build_reject(&map_with_sectors(8)).to_lump_bytes().len(), 8);
}

#[test]
fn build_reject_three_sectors_is_two_zero_bytes() {
    let map = map_with_sectors(3);
    let reject = build_reject(&map);
    assert_eq!(reject.sector_count(), 3);
    assert_eq!(reject.to_lump_bytes(), vec![0u8, 0u8]);
}

/// Assembles a Doom map with the given vertices (as `(x, y)` pairs) and one
/// linedef per `(start, end)` index pair, then returns the assembled [`Map`].
/// Every linedef is one-sided against sector 0 — enough geometry to clear the
/// blockmap builder's empty-geometry gate.
fn assemble_map(points: &[(i16, i16)], lines: &[(u16, u16)]) -> Map {
    let mut linedefs = Vec::new();
    for &(s, e) in lines {
        linedefs.extend(linedef_bytes(s, e, 1, 0, 0, 0, 0xffff));
    }
    let bytes = common::build_doom_map_wad(
        "MAP01",
        thing_bytes(0, 0, 0, 1, 7),
        linedefs,
        sidedef_bytes("-", "-", "STARTAN3", 0),
        vertexes_bytes(points),
        sector_bytes(0),
    );
    let wad = Wad::from_bytes(bytes).expect("fixture WAD parses");
    let group = wad.map_group("MAP01").expect("map group present");
    Map::assemble(&wad, &group).expect("map assembles")
}

#[test]
fn build_blockmap_hand_fixture_round_trips_through_assembly() {
    // (0,0)-(64,0): the controller-verified 16-byte fixture, end to end through
    // real WAD assembly and the public builder.
    let map = assemble_map(&[(0, 0), (64, 0)], &[(0, 1)]);
    let (bm, warnings) = build_blockmap(&map, &NodeBuildOptions::strict()).unwrap();
    assert!(warnings.is_empty());
    assert_eq!(bm.origin(), (0.0, 0.0));
    assert_eq!((bm.columns(), bm.rows()), (1, 1));

    let bytes = bm.to_lump_bytes().unwrap();
    assert_eq!(
        bytes,
        vec![
            0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00,
            0xFF, 0xFF,
        ]
    );

    // Global Constraint 4: re-parse against the linedef count into an exact copy.
    let mut parse_warnings: Vec<MapWarning> = Vec::new();
    let parsed = MapBlockmap::parse(&bytes, 1, Strictness::Strict, &mut parse_warnings)
        .expect("built BLOCKMAP parses")
        .expect("built BLOCKMAP is present");
    assert_eq!(parsed, bm);
    assert!(parse_warnings.is_empty());
}

#[test]
fn build_blockmap_multi_block_spans_columns() {
    // Linedef (0,0)-(300,0) crosses three columns.
    let map = assemble_map(&[(0, 0), (300, 0)], &[(0, 1)]);
    let (bm, _) = build_blockmap(&map, &NodeBuildOptions::strict()).unwrap();
    assert_eq!((bm.columns(), bm.rows()), (3, 1));
    for col in 0..3 {
        assert!(
            !bm.block(col, 0).unwrap().is_empty(),
            "column {col} should list the spanning linedef"
        );
    }
}

#[test]
fn build_blockmap_overflow_strict_vs_lenient() {
    // Diagonal (0,0)-(25000,25000): first blocklist offset 38420 (> 32767).
    let map = assemble_map(&[(0, 0), (25_000, 25_000)], &[(0, 1)]);
    assert!(matches!(
        build_blockmap(&map, &NodeBuildOptions::strict()).unwrap_err(),
        NodeBuildError::BlockmapOverflow { offset: 38_420 }
    ));
    let (_, warnings) = build_blockmap(&map, &NodeBuildOptions::lenient()).unwrap();
    assert_eq!(warnings.len(), 1);
}

#[test]
fn build_reject_round_trips_through_parse_strict() {
    // ADR-0024 §7 / Global Constraint 4: the built bytes re-parse against the
    // owning sector count into an exact copy, warning-free, in strict mode.
    for n in [1usize, 3, 8] {
        let map = map_with_sectors(n);
        let built = build_reject(&map);
        let mut warnings: Vec<MapWarning> = Vec::new();
        let parsed = MapReject::parse(&built.to_lump_bytes(), n, Strictness::Strict, &mut warnings)
            .expect("built REJECT parses")
            .expect("built REJECT is present");
        assert_eq!(parsed, built);
        assert!(warnings.is_empty());
    }
}

/// Optional retail-WAD sweep for the node-lump builders (ADR-0024 §9.1): build
/// REJECT and BLOCKMAP for every classic-format map in a local collection and
/// assert the size, round-trip, and no-missing-listing contracts over real
/// geometry.
///
/// Mirrors [`tests/sweep.rs`]: point `CRUSTYWAD_SWEEP_DIR` at a directory of WAD
/// files. **Use an absolute path** — cargo runs the test binary with its CWD at
/// the package root (`crates/crustywad`), so a relative path resolves against
/// that directory, not the workspace root, and a missed directory only prints a
/// stderr skip note rather than failing.
///
/// Doom 64 maps ship pre-built nodes and are not a build target, so they are
/// skipped exactly as `tests/sweep.rs` detects them (`detect_map_format`). The
/// retail collection must build strict-clean: any warning or failure here is a
/// builder bug or a plan error, not an exception to allowlist.
#[cfg(feature = "sweep-tests")]
#[test]
#[allow(clippy::too_many_lines)]
fn sweep_builds_reject_and_blockmap_for_every_classic_map() {
    use crustywad::ParseOptions;
    use crustywad::map::{LinedefIdx, MapFormat, detect_map_format};

    let paths = common::wad_files("CRUSTYWAD_SWEEP_DIR");
    if paths.is_empty() {
        return; // skip note already printed by the helper
    }

    let mut maps_built = 0usize;
    let mut doom64_skipped = 0usize;
    let mut total_blockmap_bytes = 0usize;

    for path in &paths {
        let wad = Wad::from_path_with_options(path, ParseOptions::strict())
            .unwrap_or_else(|e| panic!("{}: container failed strict parse: {e}", path.display()));

        for group in wad.map_groups() {
            // Doom 64 nested-WAD maps carry pre-built nodes and are not a build
            // target — skip them exactly as `tests/sweep.rs` detects them.
            if detect_map_format(&wad, &group) == MapFormat::Doom64 {
                doom64_skipped += 1;
                continue;
            }

            // Assemble leniently so the sweep exercises the builders over every
            // classic map, not only the strict-clean assembly subset.
            let map = Map::assemble_with_options(&wad, &group, ParseOptions::lenient())
                .unwrap_or_else(|e| {
                    panic!(
                        "{}: map {} failed lenient assembly: {e}",
                        path.display(),
                        group.name
                    )
                });

            // --- REJECT: exact size, then strict round-trip, warning-free. ---
            let sector_count = map.sectors().len();
            let reject = build_reject(&map);
            let reject_bytes = reject.to_lump_bytes();
            let expected_len = sector_count.saturating_mul(sector_count).div_ceil(8);
            assert_eq!(
                reject_bytes.len(),
                expected_len,
                "{}: map {} REJECT size {} != ceil({sector_count}\u{b2}/8)={expected_len}",
                path.display(),
                group.name,
                reject_bytes.len(),
            );
            let mut reject_warnings: Vec<MapWarning> = Vec::new();
            let parsed_reject = MapReject::parse(
                &reject_bytes,
                sector_count,
                Strictness::Strict,
                &mut reject_warnings,
            )
            .unwrap_or_else(|e| {
                panic!(
                    "{}: map {} built REJECT failed strict parse: {e}",
                    path.display(),
                    group.name
                )
            })
            .unwrap_or_else(|| {
                panic!(
                    "{}: map {} built REJECT absent on parse",
                    path.display(),
                    group.name
                )
            });
            assert_eq!(
                parsed_reject,
                reject,
                "{}: map {} REJECT round-trip mismatch",
                path.display(),
                group.name
            );
            assert!(
                reject_warnings.is_empty(),
                "{}: map {} REJECT round-trip warned: {reject_warnings:?}",
                path.display(),
                group.name
            );

            // --- BLOCKMAP: strict build (retail must never trip a ceiling),
            //     zero warnings, then strict round-trip (Global Constraint 4). ---
            let (bm, build_warnings) = build_blockmap(&map, &NodeBuildOptions::strict())
                .unwrap_or_else(|e| {
                    panic!(
                        "{}: map {} strict BLOCKMAP build failed: {e:?}",
                        path.display(),
                        group.name
                    )
                });
            assert!(
                build_warnings.is_empty(),
                "{}: map {} strict BLOCKMAP build warned: {build_warnings:?}",
                path.display(),
                group.name
            );

            let bm_bytes = bm.to_lump_bytes().unwrap_or_else(|e| {
                panic!(
                    "{}: map {} BLOCKMAP serialization failed: {e:?}",
                    path.display(),
                    group.name
                )
            });
            total_blockmap_bytes += bm_bytes.len();

            let linedef_count = map.linedefs().len();
            let mut bm_warnings: Vec<MapWarning> = Vec::new();
            let parsed_bm = MapBlockmap::parse(
                &bm_bytes,
                linedef_count,
                Strictness::Strict,
                &mut bm_warnings,
            )
            .unwrap_or_else(|e| {
                panic!(
                    "{}: map {} built BLOCKMAP failed strict parse: {e}",
                    path.display(),
                    group.name
                )
            })
            .unwrap_or_else(|| {
                panic!(
                    "{}: map {} built BLOCKMAP absent on parse",
                    path.display(),
                    group.name
                )
            });
            assert_eq!(
                parsed_bm,
                bm,
                "{}: map {} BLOCKMAP round-trip mismatch",
                path.display(),
                group.name
            );
            assert!(
                bm_warnings.is_empty(),
                "{}: map {} BLOCKMAP round-trip warned: {bm_warnings:?}",
                path.display(),
                group.name
            );

            // --- Independent no-missing oracle: every point sampled along a
            //     linedef at <= 16-unit steps (endpoints included) must fall in
            //     a block that lists that linedef. Same oracle as the Task 2
            //     proptest, over real geometry. Retail vertex coordinates are
            //     already i16, so narrowing is identity and `bm.origin()` is the
            //     minimum endpoint coordinate. ---
            let verts = map.vertices();
            let (ox, oy) = bm.origin();
            let last_col = f64::from(u32::try_from(bm.columns() - 1).unwrap());
            let last_row = f64::from(u32::try_from(bm.rows() - 1).unwrap());
            for (li, ld) in map.linedefs().iter().enumerate() {
                let (x1, y1) = (verts[ld.start.0].x, verts[ld.start.0].y);
                let (x2, y2) = (verts[ld.end.0].x, verts[ld.end.0].y);
                let span = (x2 - x1).abs().max((y2 - y1).abs());
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let steps = (span / 16.0).ceil().max(1.0) as u32;
                for step in 0..=steps {
                    let t = f64::from(step) / f64::from(steps);
                    let px = x1 + (x2 - x1) * t;
                    let py = y1 + (y2 - y1) * t;
                    let col_f = ((px - ox) / 128.0).floor().clamp(0.0, last_col);
                    let row_f = ((py - oy) / 128.0).floor().clamp(0.0, last_row);
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let (col, row) = (col_f as usize, row_f as usize);
                    let listed = bm.block(col, row).unwrap().contains(&LinedefIdx(li));
                    assert!(
                        listed,
                        "{}: map {} linedef {li} sample ({px},{py}) missing from block ({col},{row})",
                        path.display(),
                        group.name
                    );
                }
            }

            maps_built += 1;
        }
    }

    // The env var was set on purpose: WADs but no classic maps means the
    // collection (or the sweep) is broken, not clean.
    assert!(
        maps_built > 0,
        "CRUSTYWAD_SWEEP_DIR contained {} WAD file(s) but no classic maps were built",
        paths.len()
    );
    eprintln!(
        "built node lumps for {} WAD(s): {maps_built} classic map(s), {doom64_skipped} Doom 64 skipped, {total_blockmap_bytes} total BLOCKMAP bytes",
        paths.len()
    );
}
