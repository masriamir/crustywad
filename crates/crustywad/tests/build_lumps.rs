//! Public-API tests for the `nodebuild` node-lump builders (ADR-0024 §9.1).
//!
//! Task 1 covers the zero-fill REJECT builder: assemble a real WAD through the
//! public path, build its REJECT, and round-trip the bytes back through
//! [`MapReject::parse`] (ADR-0024 §7 / Global Constraint 4).
#![cfg(feature = "nodebuild")]

mod common;

use crustywad::map::build::{
    BuiltNodes, NodeBuildError, NodeBuildOptions, NodeBuildWarning, build_blockmap, build_nodes,
    build_reject,
};
use crustywad::map::{Map, MapBlockmap, MapReject, MapWarning, NodeChild};
use crustywad::{Strictness, Wad};
use proptest::prelude::*;

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

/// The raw bytes of the group's data lump named `name`, if present.
#[cfg(feature = "sweep-tests")]
fn group_lump_bytes<'a>(wad: &'a Wad, group: &crustywad::map::MapGroup, name: &str) -> &'a [u8] {
    group
        .data_indices
        .iter()
        .find(|&&i| wad.lumps()[i].name() == name)
        .and_then(|&i| wad.lump_bytes(i))
        .unwrap_or(&[])
}

/// The number of initial segs the builder forms for `map` (one per present,
/// non-zero-length linedef side) — the denominator for seg inflation. Mirrors
/// `Bsp::build_initial_segs`; retail vertices are already `i16`, so the rounded
/// comparison matches the builder's narrowed one.
#[cfg(feature = "sweep-tests")]
fn initial_seg_count(map: &Map) -> usize {
    let verts = map.vertices();
    let mut count = 0usize;
    for ld in map.linedefs() {
        let (a, b) = (verts[ld.start.0], verts[ld.end.0]);
        if (a.x.round(), a.y.round()) == (b.x.round(), b.y.round()) {
            continue; // zero-length after narrowing — no seg
        }
        count += usize::from(ld.right.is_some()) + usize::from(ld.left.is_some());
    }
    count
}

/// Optional retail-WAD sweep for the classic BSP pass (ADR-0024 §9.2): build
/// `NODES`/`SSECTORS`/`SEGS` for every classic-format map in a local collection,
/// run the full validation oracle, and round-trip the serialized lumps back
/// through a real WAD assembly.
///
/// Same gating/skip pattern as [`sweep_builds_reject_and_blockmap_for_every_classic_map`]:
/// point `CRUSTYWAD_SWEEP_DIR` at a directory of WAD files (**absolute path** —
/// see that test's note). Doom 64 maps ship pre-built nodes and are skipped.
///
/// The sweep runs `build_nodes` in **lenient** mode and pins its warning set to
/// the single [`NodeBuildWarning::MixedSectorSubsector`] class — the one soft
/// defect inherent to seg-line node-building that the retail masters themselves
/// ship (ADR-0024 §7 amendment 2026-07-19: 47 mixed-sector subsectors across 30
/// shipped maps). This is a **known-condition pin, not an allowlist**: every
/// other warning, and every error — including [`NodeBuildError::DegeneratePartition`],
/// which the classify↔split unification made unreachable — still fails the sweep.
/// The full oracle otherwise holds, via [`validate_bsp_allowing_mixed_sector`]
/// (which relaxes only the single-sector check). A cheap per-map strict
/// cross-check confirms every map builds strict-clean unless it has a
/// mixed-sector fan, in which case strict errors with exactly
/// `MixedSectorSubsector`.
#[cfg(feature = "sweep-tests")]
#[test]
#[allow(clippy::too_many_lines)]
fn sweep_builds_nodes_for_every_classic_map() {
    use crustywad::ParseOptions;
    use crustywad::map::{MapFormat, detect_map_format};

    let paths = common::wad_files("CRUSTYWAD_SWEEP_DIR");
    if paths.is_empty() {
        return; // skip note already printed by the helper
    }

    let mut maps_built = 0usize;
    let mut doom64_skipped = 0usize;
    let mut total_segs = 0usize;
    let mut mixed_sector_maps = 0usize;
    let mut total_mixed_warnings = 0usize;
    let mut inflations: Vec<u64> = Vec::new();

    for path in &paths {
        let wad = Wad::from_path_with_options(path, ParseOptions::strict())
            .unwrap_or_else(|e| panic!("{}: container failed strict parse: {e}", path.display()));

        for group in wad.map_groups() {
            if detect_map_format(&wad, &group) == MapFormat::Doom64 {
                doom64_skipped += 1;
                continue;
            }

            // Assemble leniently so the sweep exercises the builder over every
            // classic map, matching the stage-1 sweep.
            let map = Map::assemble_with_options(&wad, &group, ParseOptions::lenient())
                .unwrap_or_else(|e| {
                    panic!(
                        "{}: map {} failed lenient assembly: {e}",
                        path.display(),
                        group.name
                    )
                });

            // --- build_nodes LENIENT, warning set pinned to the single
            //     tolerated MixedSectorSubsector class (ADR-0024 §7 amendment).
            //     Known-condition pin, NOT an allowlist. ---
            let (built, warnings) =
                build_nodes(&map, &NodeBuildOptions::lenient()).unwrap_or_else(|e| {
                    panic!(
                        "{}: map {} lenient build_nodes failed: {e:?}",
                        path.display(),
                        group.name
                    )
                });
            for w in &warnings {
                assert!(
                    matches!(w, NodeBuildWarning::MixedSectorSubsector { .. }),
                    "{}: map {} build_nodes emitted a non-mixed-sector warning: {w:?}",
                    path.display(),
                    group.name
                );
            }
            if !warnings.is_empty() {
                mixed_sector_maps += 1;
                total_mixed_warnings += warnings.len();
            }

            // --- Cheap strict cross-check: a map builds strict-clean unless it
            //     has a mixed-sector fan, in which case strict errors with
            //     exactly MixedSectorSubsector — never DegeneratePartition (the
            //     resolved classify↔split guard), never any other variant. ---
            match build_nodes(&map, &NodeBuildOptions::strict()) {
                Ok((_, strict_warnings)) => assert!(
                    strict_warnings.is_empty() && warnings.is_empty(),
                    "{}: map {} strict-Ok but lenient warned (inconsistent)",
                    path.display(),
                    group.name
                ),
                Err(NodeBuildError::MixedSectorSubsector { .. }) => assert!(
                    !warnings.is_empty(),
                    "{}: map {} strict MixedSector but lenient clean (inconsistent)",
                    path.display(),
                    group.name
                ),
                Err(e) => panic!(
                    "{}: map {} strict build_nodes failed with a non-mixed error: {e:?}",
                    path.display(),
                    group.name
                ),
            }

            // --- Full validation oracle over real geometry; mixed-sector leaves
            //     are the one relaxed check (ADR-0024 §7 amendment). ---
            validate_bsp_allowing_mixed_sector(&map, &built);

            // --- Calibration corridor (a tripwire, not a quality bar). ---
            let linedefs = map.linedefs().len();
            assert!(
                built.segs.len() <= 4 * linedefs,
                "{}: map {} seg count {} exceeds 4 * {linedefs} linedefs",
                path.display(),
                group.name,
                built.segs.len(),
            );
            assert!(
                built.split_vertices.len() <= map.vertices().len(),
                "{}: map {} split-vertex count {} exceeds map vertex count {}",
                path.display(),
                group.name,
                built.split_vertices.len(),
                map.vertices().len(),
            );

            // --- Serialize, then round-trip through a real strict WAD assembly:
            //     the assembled BSP arenas must equal the built ones exactly. ---
            let lumps = built.to_lump_bytes().unwrap_or_else(|e| {
                panic!(
                    "{}: map {} node-lump serialization failed: {e:?}",
                    path.display(),
                    group.name
                )
            });
            let mut vertexes = group_lump_bytes(&wad, &group, "VERTEXES").to_vec();
            vertexes.extend(lumps.split_vertexes.clone());
            let mut rt_lumps: Vec<(&str, Vec<u8>)> = vec![
                (group.name.as_str(), Vec::new()),
                ("THINGS", group_lump_bytes(&wad, &group, "THINGS").to_vec()),
                (
                    "LINEDEFS",
                    group_lump_bytes(&wad, &group, "LINEDEFS").to_vec(),
                ),
                (
                    "SIDEDEFS",
                    group_lump_bytes(&wad, &group, "SIDEDEFS").to_vec(),
                ),
                ("VERTEXES", vertexes),
                ("SEGS", lumps.segs.clone()),
                ("SSECTORS", lumps.ssectors.clone()),
                ("NODES", lumps.nodes.clone()),
                (
                    "SECTORS",
                    group_lump_bytes(&wad, &group, "SECTORS").to_vec(),
                ),
                ("REJECT", Vec::new()),
                ("BLOCKMAP", Vec::new()),
            ];
            // A Hexen map's THINGS/LINEDEFS are the wider Hexen records; the
            // round-trip WAD must carry a BEHAVIOR lump so re-assembly detects
            // Hexen (not Doom) and parses them at the right stride.
            if map.format() == MapFormat::Hexen {
                rt_lumps.push((
                    "BEHAVIOR",
                    group_lump_bytes(&wad, &group, "BEHAVIOR").to_vec(),
                ));
            }
            let wad_bytes = common::build_named_lumps(&rt_lumps);
            let rt_wad = Wad::from_bytes(wad_bytes).unwrap_or_else(|e| {
                panic!(
                    "{}: map {} round-trip WAD failed to parse: {e}",
                    path.display(),
                    group.name
                )
            });
            let rt_group = rt_wad.map_group(&group.name).unwrap_or_else(|| {
                panic!(
                    "{}: map {} round-trip group absent",
                    path.display(),
                    group.name
                )
            });
            let assembled = Map::assemble(&rt_wad, &rt_group).unwrap_or_else(|e| {
                panic!(
                    "{}: map {} round-trip failed strict assembly: {e}",
                    path.display(),
                    group.name
                )
            });
            assert!(
                assembled.warnings().is_empty(),
                "{}: map {} round-trip assembly warned: {:?}",
                path.display(),
                group.name,
                assembled.warnings(),
            );
            assert_eq!(
                assembled.segs(),
                built.segs.as_slice(),
                "{}: map {} round-trip SEGS mismatch",
                path.display(),
                group.name
            );
            assert_eq!(
                assembled.subsectors(),
                built.subsectors.as_slice(),
                "{}: map {} round-trip SSECTORS mismatch",
                path.display(),
                group.name
            );
            assert_eq!(
                assembled.nodes(),
                built.nodes.as_slice(),
                "{}: map {} round-trip NODES mismatch",
                path.display(),
                group.name
            );

            // --- Aggregates: total segs and per-map seg inflation (×1000). ---
            total_segs += built.segs.len();
            let initial = initial_seg_count(&map).max(1);
            inflations.push((built.segs.len() as u64 * 1000) / initial as u64);
            maps_built += 1;
        }
    }

    assert!(
        maps_built > 0,
        "CRUSTYWAD_SWEEP_DIR contained {} WAD file(s) but no classic maps were built",
        paths.len()
    );
    inflations.sort_unstable();
    let median_inflation = inflations[inflations.len() / 2];
    eprintln!(
        "built BSP nodes for {} WAD(s): {maps_built} classic map(s), {doom64_skipped} Doom 64 skipped, {total_segs} total built segs, median seg-inflation x1000 = {median_inflation}; {mixed_sector_maps} map(s) carried a tolerated mixed-sector fan ({total_mixed_warnings} warning(s), ADR-0024 §7 amendment)",
        paths.len()
    );
}

#[test]
fn default_options_are_strict() {
    assert_eq!(
        NodeBuildOptions::default().strictness,
        crustywad::Strictness::Strict
    );
}

// --- Task 2: the classic BSP pass (`build_nodes`) ---------------------------

/// Assembles a Doom map from vertices and general linedefs. Each linedef is
/// `(start, end, right_sector, left_sector)`: a `Some(sector)` side gets a fresh
/// sidedef facing that sector; a `None` side is the `0xffff` "no sidedef"
/// sentinel. Enough sectors are emitted to cover the highest referenced index.
fn assemble_general(points: &[(i16, i16)], lines: &[(u16, u16, Option<u16>, Option<u16>)]) -> Map {
    let mut linedefs = Vec::new();
    let mut sidedefs = Vec::new();
    let mut next_side: u16 = 0;
    let mut max_sector: u16 = 0;
    let side_for = |sector: u16, sidedefs: &mut Vec<u8>, next_side: &mut u16| -> u16 {
        sidedefs.extend(sidedef_bytes("-", "-", "STARTAN3", sector));
        let idx = *next_side;
        *next_side += 1;
        idx
    };
    for &(s, e, rs, ls) in lines {
        let right = match rs {
            Some(sec) => {
                max_sector = max_sector.max(sec);
                side_for(sec, &mut sidedefs, &mut next_side)
            }
            None => 0xffff,
        };
        let left = match ls {
            Some(sec) => {
                max_sector = max_sector.max(sec);
                side_for(sec, &mut sidedefs, &mut next_side)
            }
            None => 0xffff,
        };
        let flags: u16 = if ls.is_some() { 0x0004 } else { 0x0001 };
        linedefs.extend(linedef_bytes(s, e, flags, 0, 0, right, left));
    }
    let mut sectors = Vec::new();
    for i in 0..=max_sector {
        sectors.extend(sector_bytes(i16::try_from(i).unwrap()));
    }
    let bytes = common::build_doom_map_wad(
        "MAP01",
        thing_bytes(0, 0, 0, 1, 7),
        linedefs,
        sidedefs,
        vertexes_bytes(points),
        sectors,
    );
    let wad = Wad::from_bytes(bytes).expect("fixture WAD parses");
    let group = wad.map_group("MAP01").expect("map group present");
    Map::assemble(&wad, &group).expect("map assembles")
}

/// The combined `(x, y)` of a seg vertex: map vertices first, then the built
/// split vertices (the [`BuiltNodes`] index-domain contract). Fixture
/// coordinates are whole `i16`, so the `f64`→`i32` round is exact.
#[allow(clippy::cast_possible_truncation)]
fn combined_coord(map: &Map, built: &BuiltNodes, idx: usize) -> (i32, i32) {
    let mvc = map.vertices().len();
    let v = if idx < mvc {
        map.vertices()[idx]
    } else {
        built.split_vertices[idx - mvc]
    };
    (v.x.round() as i32, v.y.round() as i32)
}

/// The sector a built seg faces, resolved through its linedef's own side.
fn seg_sector(map: &Map, seg: &crustywad::map::MapSeg) -> usize {
    let ld = &map.linedefs()[seg.linedef.0];
    let side = if seg.direction == 0 {
        ld.right
    } else {
        ld.left
    };
    map.sidedefs()[side.expect("built seg's side is present").0]
        .sector
        .0
}

/// All subsector indices in the subtree rooted at `child` (iterative).
fn subtree_subsectors(built: &BuiltNodes, child: NodeChild) -> Vec<usize> {
    let mut out = Vec::new();
    let mut stack = vec![child];
    while let Some(c) = stack.pop() {
        match c {
            NodeChild::Subsector(i) => out.push(i.0),
            NodeChild::Node(k) => {
                stack.push(built.nodes[k.0].right);
                stack.push(built.nodes[k.0].left);
            }
        }
    }
    out
}

/// The first subsector holding a seg from `linedef` on `direction`, if any.
fn subsector_of(built: &BuiltNodes, linedef: usize, direction: u16) -> Option<usize> {
    built.subsectors.iter().position(|ss| {
        built.segs[ss.segs.clone()]
            .iter()
            .any(|s| s.linedef.0 == linedef && s.direction == direction)
    })
}

/// The full Task 3 validation oracle — the correctness instrument shared by
/// every `build_nodes` fixture, the proptest, and the retail sweep. It checks:
/// index ranges, acyclic single-visit reachability, root-last, a contiguous seg
/// partition, single-sector subsectors, ancestor-side containment (every seg
/// endpoint within 1.5 units of the correct side of every ancestor partition),
/// exact child bboxes, and `nodes == subsectors - 1`.
///
/// Mixed-sector subsectors are rejected — a well-formed strict build never
/// produces one. Use [`validate_bsp_allowing_mixed_sector`] for the accepted
/// lenient `MixedSectorSubsector` deviation (ADR-0024 §C.2), which relaxes only
/// the single-sector check and leaves every structural invariant enforced.
fn validate_bsp(map: &Map, built: &BuiltNodes) {
    validate_bsp_inner(map, built, false);
}

/// [`validate_bsp`] with the single-sector-subsector check relaxed, for the
/// lenient `MixedSectorSubsector` recovery (ADR-0024 §C.2): every other
/// structural invariant still holds.
fn validate_bsp_allowing_mixed_sector(map: &Map, built: &BuiltNodes) {
    validate_bsp_inner(map, built, true);
}

/// The oracle body; `allow_mixed_sector` gates only the single-sector check.
#[allow(clippy::too_many_lines)]
fn validate_bsp_inner(map: &Map, built: &BuiltNodes, allow_mixed_sector: bool) {
    let total_verts = map.vertices().len() + built.split_vertices.len();

    // nodes == subsectors - 1 (a full binary tree of leaves).
    assert!(
        !built.subsectors.is_empty(),
        "a built map has >= 1 subsector"
    );
    assert_eq!(built.subsectors.len(), built.nodes.len() + 1);

    // Index ranges.
    for s in &built.segs {
        assert!(
            s.start.0 < total_verts && s.end.0 < total_verts,
            "seg vertex in range"
        );
    }
    for n in &built.nodes {
        for child in [n.right, n.left] {
            match child {
                NodeChild::Node(k) => assert!(k.0 < built.nodes.len(), "node child in range"),
                NodeChild::Subsector(i) => {
                    assert!(i.0 < built.subsectors.len(), "subsector child in range");
                }
            }
        }
    }

    // Contiguous seg partition, in subsector order.
    let mut cursor = 0;
    for ss in &built.subsectors {
        assert_eq!(ss.segs.start, cursor, "subsector segs are contiguous");
        assert!(ss.segs.end >= ss.segs.start);
        cursor = ss.segs.end;
    }
    assert_eq!(
        cursor,
        built.segs.len(),
        "subsectors partition every seg exactly once"
    );

    // Single-sector subsectors (relaxed only for the lenient mixed-sector
    // deviation, ADR-0024 §C.2).
    if !allow_mixed_sector {
        for ss in &built.subsectors {
            let segs = &built.segs[ss.segs.clone()];
            if let Some(first) = segs.first() {
                let sector = seg_sector(map, first);
                assert!(
                    segs.iter().all(|s| seg_sector(map, s) == sector),
                    "every seg in a subsector shares one sector"
                );
            }
        }
    }

    // Exact child bboxes: each node's stored bbox equals its subtree's
    // seg-endpoint bounding box.
    for n in &built.nodes {
        for (child, stored) in [(n.right, n.right_bbox), (n.left, n.left_bbox)] {
            let mut bbox = [i32::MIN, i32::MAX, i32::MAX, i32::MIN];
            for sub in subtree_subsectors(built, child) {
                for seg in &built.segs[built.subsectors[sub].segs.clone()] {
                    for endpoint in [seg.start.0, seg.end.0] {
                        let (x, y) = combined_coord(map, built, endpoint);
                        bbox[0] = bbox[0].max(y);
                        bbox[1] = bbox[1].min(y);
                        bbox[2] = bbox[2].min(x);
                        bbox[3] = bbox[3].max(x);
                    }
                }
            }
            assert_eq!(stored, bbox, "node child bbox equals its subtree exactly");
        }
    }

    // Acyclic single-visit reachability + root-last + ancestor-side containment.
    let root = if built.nodes.is_empty() {
        assert_eq!(
            built.subsectors.len(),
            1,
            "no nodes => a single convex subsector"
        );
        NodeChild::Subsector(crustywad::map::SubsectorIdx(0))
    } else {
        NodeChild::Node(crustywad::map::NodeIdx(built.nodes.len() - 1))
    };
    let mut seen_nodes = vec![false; built.nodes.len()];
    let mut seen_subs = vec![false; built.subsectors.len()];
    // Each stack entry carries its ancestor (node_index, is_front) path.
    let mut stack: Vec<(NodeChild, Vec<(usize, bool)>)> = vec![(root, Vec::new())];
    while let Some((child, path)) = stack.pop() {
        match child {
            NodeChild::Node(k) => {
                assert!(!seen_nodes[k.0], "node reached more than once (not a tree)");
                seen_nodes[k.0] = true;
                let n = &built.nodes[k.0];
                let mut front = path.clone();
                front.push((k.0, true));
                let mut back = path.clone();
                back.push((k.0, false));
                stack.push((n.right, front));
                stack.push((n.left, back));
            }
            NodeChild::Subsector(i) => {
                assert!(!seen_subs[i.0], "subsector reached more than once");
                seen_subs[i.0] = true;
                for &(node_idx, is_front) in &path {
                    let n = &built.nodes[node_idx];
                    let len = f64::from(n.dx).hypot(f64::from(n.dy));
                    for seg in &built.segs[built.subsectors[i.0].segs.clone()] {
                        for endpoint in [seg.start.0, seg.end.0] {
                            let (qx, qy) = combined_coord(map, built, endpoint);
                            // Engine cross: > 0 is the front side.
                            let cross = f64::from(qx - n.x) * f64::from(n.dy)
                                - f64::from(qy - n.y) * f64::from(n.dx);
                            let signed = cross / len;
                            if is_front {
                                assert!(
                                    signed >= -1.5,
                                    "front-subtree seg is on/near the front side"
                                );
                            } else {
                                assert!(signed <= 1.5, "back-subtree seg is on/near the back side");
                            }
                        }
                    }
                }
            }
        }
    }
    assert!(
        seen_nodes.iter().all(|&v| v),
        "every node reachable exactly once"
    );
    assert!(
        seen_subs.iter().all(|&v| v),
        "every subsector reachable exactly once"
    );
}

/// The controller-verified square room: four one-sided linedefs, one convex
/// subsector, no split vertices, no nodes — verified typed, byte-for-byte, and
/// through a strict WAD round-trip that assembles back with zero warnings.
#[test]
fn build_nodes_square_room_is_one_convex_subsector() {
    let points = [(0i16, 0i16), (128, 0), (128, 128), (0, 128)];
    let map = assemble_general(
        &points,
        &[
            (0, 1, Some(0), None),
            (1, 2, Some(0), None),
            (2, 3, Some(0), None),
            (3, 0, Some(0), None),
        ],
    );
    let (built, warnings) = build_nodes(&map, &NodeBuildOptions::strict()).expect("builds");
    assert!(warnings.is_empty(), "square room builds strict-clean");

    // Typed shape.
    assert!(built.split_vertices.is_empty());
    assert_eq!(built.segs.len(), 4);
    assert!(built.nodes.is_empty());
    assert_eq!(built.subsectors.len(), 1);
    assert_eq!(built.subsectors[0].segs, 0..4);
    // Axis-aligned angles, exact (Global Constraint 8).
    let angles: Vec<u16> = built.segs.iter().map(|s| s.angle).collect();
    assert_eq!(angles, vec![0x0000, 0x4000, 0x8000, 0xC000]);
    assert!(built.segs.iter().all(|s| s.offset == 0));

    // Serialized bytes match the controller derivation.
    let lumps = built.to_lump_bytes().expect("serializes");
    assert!(lumps.split_vertexes.is_empty());
    assert!(
        lumps.nodes.is_empty(),
        "single convex subsector => numnodes 0"
    );
    assert_eq!(lumps.ssectors, vec![0x04, 0x00, 0x00, 0x00]);
    assert_eq!(lumps.segs.len(), 4 * 12);
    assert_eq!(
        &lumps.segs[..12],
        &[
            0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
        ],
    );

    validate_bsp(&map, &built);

    // Strict WAD round-trip: assemble a WAD carrying the five data lumps plus
    // the built node lumps (canonical order) and confirm it reads back clean.
    let mut vertexes = vertexes_bytes(&points);
    vertexes.extend(lumps.split_vertexes.clone());
    let wad_bytes = common::build_named_lumps(&[
        ("MAP01", Vec::new()),
        ("THINGS", thing_bytes(0, 0, 0, 1, 7)),
        ("LINEDEFS", {
            let mut b = Vec::new();
            for &(s, e) in &[(0u16, 1u16), (1, 2), (2, 3), (3, 0)] {
                b.extend(linedef_bytes(s, e, 1, 0, 0, 0, 0xffff));
            }
            b
        }),
        ("SIDEDEFS", sidedef_bytes("-", "-", "STARTAN3", 0)),
        ("VERTEXES", vertexes),
        ("SEGS", lumps.segs.clone()),
        ("SSECTORS", lumps.ssectors.clone()),
        ("NODES", lumps.nodes.clone()),
        ("SECTORS", sector_bytes(0)),
        ("REJECT", Vec::new()),
        ("BLOCKMAP", Vec::new()),
    ]);
    let wad = Wad::from_bytes(wad_bytes).expect("round-trip WAD parses");
    let group = wad.map_group("MAP01").expect("group present");
    let assembled = Map::assemble(&wad, &group).expect("assembles strict");
    assert!(assembled.warnings().is_empty(), "zero assembly warnings");
    assert_eq!(
        assembled.bsp_root(),
        None,
        "single subsector has no root node"
    );
    assert_eq!(assembled.subsectors().len(), 1);
    assert_eq!(assembled.segs().len(), 4);
}

/// An L-shaped (concave) single-sector room must produce >= 1 node and satisfy
/// the full validation oracle.
#[test]
fn build_nodes_l_room_has_nodes_and_passes_the_oracle() {
    // Concave at (128,128): outer corners (0,0) (256,0) (256,128) (128,128)
    // (128,256) (0,256), chained.
    let points = [
        (0i16, 0i16),
        (256, 0),
        (256, 128),
        (128, 128),
        (128, 256),
        (0, 256),
    ];
    let lines = [
        (0u16, 1u16, Some(0u16), None),
        (1, 2, Some(0), None),
        (2, 3, Some(0), None),
        (3, 4, Some(0), None),
        (4, 5, Some(0), None),
        (5, 0, Some(0), None),
    ];
    let map = assemble_general(&points, &lines);
    let (built, warnings) = build_nodes(&map, &NodeBuildOptions::strict()).expect("builds");
    assert!(warnings.is_empty(), "L-room builds strict-clean");

    assert!(
        !built.nodes.is_empty(),
        "a concave room needs at least one node"
    );
    assert_eq!(built.nodes.len(), built.subsectors.len() - 1);
    validate_bsp(&map, &built);
}

/// Two-sided geometry: a vertical and a horizontal two-sided line crossing at
/// (128,128) inside a bounded room. The builder partitions on one of them; the
/// other is split there. Asserts (a) the partition line's own partner segs
/// (direction 0 and 1) land on OPPOSITE sides of the root, and (b) the split of
/// the other two-sided line produces exactly ONE shared vertex at (128,128)
/// referenced by both directions — no crack.
#[test]
fn build_nodes_two_sided_lines_split_without_a_crack() {
    let points = [
        (0i16, 0i16),
        (256, 0),
        (256, 256),
        (0, 256), // outer 0..4
        (128, 64),
        (128, 192), // V (vertical two-sided) 4,5
        (64, 128),
        (192, 128), // H (horizontal two-sided) 6,7
    ];
    // V and H are two-sided with sector 0 on BOTH sides (a valid
    // self-referencing-line configuration): the whole map stays single-sector,
    // so no mixed-sector region can arise from the deliberately overlapping
    // geometry, while the crossing still forces one line to be split.
    let lines = [
        (0u16, 1u16, Some(0u16), None),
        (1, 2, Some(0), None),
        (2, 3, Some(0), None),
        (3, 0, Some(0), None),
        (4, 5, Some(0), Some(0)), // linedef 4 = V (two-sided, sector 0/0)
        (6, 7, Some(0), Some(0)), // linedef 5 = H (two-sided, sector 0/0)
    ];
    let map = assemble_general(&points, &lines);
    let (built, warnings) = build_nodes(&map, &NodeBuildOptions::strict()).expect("builds");
    assert!(
        warnings.is_empty(),
        "crossing two-sided lines build strict-clean"
    );
    validate_bsp(&map, &built);

    // Exactly one split vertex at (128,128) (dedup: both directions share it).
    let mid_hits: Vec<usize> = built
        .split_vertices
        .iter()
        .enumerate()
        .filter(|(_, v)| (v.x - 128.0).abs() < 1e-9 && (v.y - 128.0).abs() < 1e-9)
        .map(|(i, _)| map.vertices().len() + i)
        .collect();
    assert_eq!(
        mid_hits.len(),
        1,
        "one shared split vertex at the crossing, not two"
    );
    let mid_idx = mid_hits[0];

    // The root partition is one of the crossing lines; identify which.
    let root = built.nodes.last().expect("crossing lines force a node");
    let (root_linedef, other_linedef) = if root.dx == 0 {
        (4usize, 5usize) // vertical => V is the splitter, H is split
    } else {
        assert_eq!(root.dy, 0, "root partition is axis-aligned here");
        (5usize, 4usize)
    };

    // (a) Opposite sides: the splitter's dir-0 and dir-1 partner segs are in
    // subsectors under opposite root children.
    let front_subs = subtree_subsectors(&built, root.right);
    let back_subs = subtree_subsectors(&built, root.left);
    let dir0_sub = subsector_of(&built, root_linedef, 0).expect("splitter dir-0 seg placed");
    let dir1_sub = subsector_of(&built, root_linedef, 1).expect("splitter dir-1 seg placed");
    assert!(
        front_subs.contains(&dir0_sub),
        "front (dir 0) partner under the front child"
    );
    assert!(
        back_subs.contains(&dir1_sub),
        "back (dir 1) partner under the back child"
    );

    // (b) No crack: BOTH directions of the split (other) two-sided line have a
    // fragment ending at the shared (128,128) vertex.
    for dir in [0u16, 1u16] {
        assert!(
            built.segs.iter().any(|s| s.linedef.0 == other_linedef
                && s.direction == dir
                && (s.start.0 == mid_idx || s.end.0 == mid_idx)),
            "direction {dir} of the split line meets the shared vertex"
        );
    }
}

/// Determinism (Global Constraint 8): the same map built twice yields identical
/// serialized bytes.
#[test]
fn build_nodes_is_deterministic() {
    let points = [
        (0i16, 0i16),
        (256, 0),
        (256, 128),
        (128, 128),
        (128, 256),
        (0, 256),
    ];
    let lines = [
        (0u16, 1u16, Some(0u16), None),
        (1, 2, Some(0), None),
        (2, 3, Some(0), None),
        (3, 4, Some(0), None),
        (4, 5, Some(0), None),
        (5, 0, Some(0), None),
    ];
    let map = assemble_general(&points, &lines);
    let a = build_nodes(&map, &NodeBuildOptions::strict())
        .unwrap()
        .0
        .to_lump_bytes()
        .unwrap();
    let b = build_nodes(&map, &NodeBuildOptions::strict())
        .unwrap()
        .0
        .to_lump_bytes()
        .unwrap();
    assert_eq!(a, b);
}

/// Zero-length and fully-sideless linedefs contribute no segs, yet the map still
/// builds from the remaining geometry.
#[test]
fn build_nodes_skips_zero_length_and_sideless_linedefs() {
    // A square, plus a zero-length linedef (5->5) and a fully-sideless one.
    let points = [(0i16, 0i16), (128, 0), (128, 128), (0, 128), (64, 64)];
    let lines = [
        (0u16, 1u16, Some(0u16), None),
        (1, 2, Some(0), None),
        (2, 3, Some(0), None),
        (3, 0, Some(0), None),
        (4, 4, Some(0), None), // zero-length: dropped
        (0, 2, None, None),    // no sidedefs: dropped
    ];
    let map = assemble_general(&points, &lines);
    let (built, warnings) = build_nodes(&map, &NodeBuildOptions::strict()).expect("builds");
    assert!(warnings.is_empty());
    // Only the four square walls become segs.
    assert_eq!(built.segs.len(), 4);
    validate_bsp(&map, &built);
}

/// Empty geometry — and geometry that yields no segs at all — is rejected in
/// both strictness modes.
#[test]
fn build_nodes_rejects_geometry_without_segs() {
    // A single fully-sideless linedef: clears the arena gate but yields no segs.
    let points = [(0i16, 0i16), (64, 0)];
    let map = assemble_general(&points, &[(0, 1, None, None)]);
    for opts in [NodeBuildOptions::strict(), NodeBuildOptions::lenient()] {
        assert_eq!(
            build_nodes(&map, &opts).unwrap_err(),
            NodeBuildError::EmptyGeometry
        );
    }
}

/// §C.2 separation (the accepted spec deviation): a multi-sector region whose
/// sectors share coincident geometry, separated by the sector-separating
/// relaxation. A single two-sided linedef between sector 0 (front) and sector 1
/// (back) is convex under the normal rule (its two colinear partner segs give no
/// solid front/back content), so the builder falls through to §C.2's relaxed
/// selection, which separates the opposite-direction colinear segs into one leaf
/// each. Builds strict-clean and passes the full oracle.
#[test]
fn build_nodes_multi_sector_region_separates_via_relaxed_rule() {
    let points = [(0i16, 0i16), (64, 0)];
    // One two-sided line: right side faces sector 0, left side faces sector 1.
    let map = assemble_general(&points, &[(0, 1, Some(0), Some(1))]);
    let (built, warnings) = build_nodes(&map, &NodeBuildOptions::strict()).expect("builds");
    assert!(
        warnings.is_empty(),
        "a separable multi-sector region builds strict-clean"
    );

    // The relaxed rule split the two colinear partner segs into two leaves.
    assert_eq!(built.segs.len(), 2, "one seg per side of the shared line");
    assert_eq!(built.subsectors.len(), 2, "one subsector per sector");
    assert_eq!(built.nodes.len(), 1, "the separating line is one node");
    // Each subsector is single-sector, and the two face different sectors.
    let sub_sectors: Vec<usize> = built
        .subsectors
        .iter()
        .map(|ss| seg_sector(&map, &built.segs[ss.segs.start]))
        .collect();
    assert!(
        sub_sectors.contains(&0) && sub_sectors.contains(&1),
        "the two leaves face sectors 0 and 1, not merged"
    );
    validate_bsp(&map, &built);
}

/// A1 / §C.2 mixed-sector defect (strict error, lenient warn): truly coincident
/// mixed-sector geometry that NO partition can separate. Two coincident,
/// same-direction one-sided linedefs face different sectors — both segs are
/// colinear-front against every candidate, so neither the normal nor the relaxed
/// selection finds a separating line. Strict rejects with
/// `MixedSectorSubsector`; lenient accepts the mixed subsector and warns once.
#[test]
fn build_nodes_coincident_mixed_sectors_error_strict_warn_lenient() {
    let points = [(0i16, 0i16), (64, 0)];
    // Two coincident one-sided lines, same direction, facing sectors 0 and 1.
    let lines = [(0u16, 1u16, Some(0u16), None), (0, 1, Some(1), None)];

    // Strict: the mixed convex region is an error naming its seg count.
    let map = assemble_general(&points, &lines);
    assert_eq!(
        build_nodes(&map, &NodeBuildOptions::strict()).unwrap_err(),
        NodeBuildError::MixedSectorSubsector { subsector_segs: 2 },
    );

    // Lenient: accepted as one (mixed) subsector, warned once for that leaf.
    let (built, warnings) =
        build_nodes(&map, &NodeBuildOptions::lenient()).expect("builds lenient");
    assert_eq!(
        warnings,
        vec![NodeBuildWarning::MixedSectorSubsector { subsector_segs: 2 }]
    );
    assert_eq!(built.subsectors.len(), 1, "one accepted convex subsector");
    assert!(built.nodes.is_empty(), "no separating node exists");
    assert_eq!(built.segs.len(), 2, "both coincident segs kept");
    // The subsector genuinely spans two sectors — the accepted defect.
    let sectors: Vec<usize> = built.segs.iter().map(|s| seg_sector(&map, s)).collect();
    assert!(
        sectors.contains(&0) && sectors.contains(&1),
        "the accepted subsector spans both sectors"
    );
    // Every structural invariant still holds; only the single-sector rule bends.
    validate_bsp_allowing_mixed_sector(&map, &built);
}

/// The seg-count soft ceiling is unit-tested in `nodes.rs` (a live 33k-seg
/// convex map would make partition selection O(n²); see the Task 2 report).
/// Here we confirm the lenient warning *variant* is public and constructible on
/// the node-build path, so the ceiling contract is exercised end to end.
#[test]
fn vanilla_ceiling_warning_is_a_public_node_build_warning() {
    let warning = NodeBuildWarning::VanillaCeilingExceeded {
        kind: "segs",
        count: 40_000,
        max: 32_768,
    };
    // Displayable and matchable — the lenient soft-ceiling recovery surface.
    assert!(format!("{warning}").contains("32768"));
}

/// Whether a `build_nodes` error is one the plan permits (never a panic): the
/// shared narrowing/format errors plus the BSP-specific ones. The mixed-sector
/// and degenerate-partition guards are included for completeness even though
/// single-sector generated maps should not reach them.
fn is_plan_known_error(err: &NodeBuildError) -> bool {
    matches!(
        err,
        NodeBuildError::EmptyGeometry
            | NodeBuildError::Write(_)
            | NodeBuildError::TooManyElements { .. }
            | NodeBuildError::MixedSectorSubsector { .. }
            | NodeBuildError::DegeneratePartition { .. }
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    /// The oracle holds on every generated single-sector map that builds `Ok`,
    /// and any build that errors does so with a plan-known variant — never a
    /// panic — in both strictness modes. Reuses the stage-1 random-geometry
    /// shape (small integer coordinates, index-wrapped linedefs) extended with
    /// optional two-sided lines so straddling splits are exercised. Every side
    /// faces sector 0, so a produced subsector is single-sector and the strict
    /// oracle applies directly.
    #[test]
    fn build_nodes_random_single_sector_maps_hold_the_oracle(
        coords in prop::collection::vec((-1024i16..=1024, -1024i16..=1024), 2..=10),
        raw_lines in prop::collection::vec((0usize..10, 0usize..10, any::<bool>()), 1..=12),
    ) {
        let n = coords.len();
        let lines: Vec<(u16, u16, Option<u16>, Option<u16>)> = raw_lines
            .iter()
            .map(|&(s, e, two_sided)| {
                let start = u16::try_from(s % n).unwrap();
                let end = u16::try_from(e % n).unwrap();
                (start, end, Some(0u16), two_sided.then_some(0u16))
            })
            .collect();
        let map = assemble_general(&coords, &lines);

        for opts in [NodeBuildOptions::strict(), NodeBuildOptions::lenient()] {
            match build_nodes(&map, &opts) {
                Ok((built, _warnings)) => validate_bsp(&map, &built),
                Err(e) => prop_assert!(
                    is_plan_known_error(&e),
                    "build_nodes returned an unexpected error variant: {e:?}"
                ),
            }
        }
    }
}
