//! Optional retail-WAD sweep: assembles every map of every WAD in a local
//! collection, in both strictness modes (#254).
//!
//! Retail WADs are not redistributable, so nothing is fetched or committed —
//! point `CRUSTYWAD_SWEEP_DIR` at a directory of WAD files. **Use an absolute
//! path**: cargo runs the test binary with its CWD at the package root
//! (`crates/crustywad`), so a relative path resolves against that directory —
//! not the workspace root — and can miss (or accidentally hit the wrong)
//! collection; a missed directory only produces a stderr skip note, not a
//! failure.
//!
//! The sweep asserts the hard invariant established by the 2026-07-13 gap
//! analysis and #252/ADR-0020: every map in the retail collection parses and
//! assembles cleanly in **both** modes with **zero warnings** — no allowlist.
//! A WAD that legitimately breaks this invariant is a bug to fix (or a policy
//! decision to record), not an exception to carve out here.
//!
//! Doom 64 nested-WAD maps are stored *inside* their `MAPxx` marker lump, but
//! since #243 `Wad::map_groups` detects and surfaces them as ordinary groups
//! (with empty `data_indices`), so a single `map_groups` loop covers all four
//! formats — no separate sniff pass is needed here.

#![cfg(feature = "sweep-tests")]

mod common;

use crustywad::map::{Map, MapFormat, detect_map_format};
use crustywad::{ParseOptions, Wad};

#[test]
fn sweep_assembles_every_map_of_every_wad() {
    let paths = common::wad_files("CRUSTYWAD_SWEEP_DIR");
    if paths.is_empty() {
        return; // skip note already printed by the helper
    }

    let mut groups_swept = 0usize;
    let mut doom64_groups = 0usize;

    for path in &paths {
        // Retail containers must parse strictly (strict mode collects no
        // warnings by construction; assert emptiness anyway as a tripwire).
        let wad = Wad::from_path_with_options(path, ParseOptions::strict())
            .unwrap_or_else(|e| panic!("{}: container failed strict parse: {e}", path.display()));
        assert!(
            wad.warnings().is_empty(),
            "{}: strict container parse produced warnings: {:?}",
            path.display(),
            wad.warnings()
        );

        // Classic marker+run maps (Doom / Heretic / Hexen / UDMF) and Doom 64
        // nested-WAD maps all surface through `Wad::map_groups` now, so a
        // single loop covers all four formats.
        for group in wad.map_groups() {
            if detect_map_format(&wad, &group) == MapFormat::Doom64 {
                doom64_groups += 1;
            }
            for options in [ParseOptions::strict(), ParseOptions::lenient()] {
                let map = Map::assemble_with_options(&wad, &group, options).unwrap_or_else(|e| {
                    panic!(
                        "{}: map {} failed {:?} assembly: {e}",
                        path.display(),
                        group.name,
                        options.strictness
                    )
                });
                assert!(
                    map.warnings().is_empty(),
                    "{}: map {} produced {:?} warnings: {:?}",
                    path.display(),
                    group.name,
                    options.strictness,
                    map.warnings()
                );
            }
            groups_swept += 1;
        }
    }

    // The env var was set on purpose: a sweep that found WADs but no maps at
    // all means the collection (or the sweep) is broken, not clean.
    assert!(
        groups_swept > 0,
        "CRUSTYWAD_SWEEP_DIR contained {} WAD file(s) but no maps were found",
        paths.len()
    );
    eprintln!(
        "swept {} WAD(s): {groups_swept} map group(s) ({doom64_groups} Doom 64), all clean in both modes",
        paths.len()
    );
}
