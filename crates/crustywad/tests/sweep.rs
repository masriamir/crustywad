//! Optional retail-WAD sweep: assembles every map of every WAD in a local
//! collection, in both strictness modes (#254).
//!
//! Retail WADs are not redistributable, so nothing is fetched or committed —
//! point `CRUSTYWAD_SWEEP_DIR` at a directory of WAD files. **The path must be
//! absolute**: cargo runs the test binary with its CWD at the package root
//! (`crates/crustywad`), so a workspace-relative path never resolves and the
//! sweep skips silently instead of failing.
//!
//! The sweep asserts the hard invariant established by the 2026-07-13 gap
//! analysis and #252/ADR-0020: every map in the retail collection parses and
//! assembles cleanly in **both** modes with **zero warnings** — no allowlist.
//! A WAD that legitimately breaks this invariant is a bug to fix (or a policy
//! decision to record), not an exception to carve out here.

#![cfg(feature = "sweep-tests")]

mod common;

use crustywad::map::{Map, is_doom64_map_lump, read_doom64_map};
use crustywad::{ParseOptions, Wad};

/// A Doom 64 map marker lump is named `MAPxx` (`MAP` + two ASCII digits).
///
/// Doom 64 maps are nested WADs stored *inside* their marker lump, invisible
/// to `Wad::map_groups` until #243 integrates them; the sweep reads them
/// through the dedicated `read_doom64_map` entrypoint instead. Requiring the
/// name pattern *and* the nested-WAD magic keeps an arbitrary data lump that
/// happens to start with `IWAD`/`PWAD` from being misread as a map.
fn is_doom64_map_name(name: &str) -> bool {
    name.len() == 5 && name.starts_with("MAP") && name[3..].bytes().all(|b| b.is_ascii_digit())
}

#[test]
fn sweep_assembles_every_map_of_every_wad() {
    let paths = common::wad_files("CRUSTYWAD_SWEEP_DIR");
    if paths.is_empty() {
        return; // skip note already printed by the helper
    }

    let mut groups_swept = 0usize;
    let mut doom64_swept = 0usize;

    for path in &paths {
        // Retail containers must parse strictly (strict mode collects no
        // warnings by construction; assert emptiness anyway as a tripwire).
        let wad = Wad::from_path_with_options(path, ParseOptions::default())
            .unwrap_or_else(|e| panic!("{}: container failed strict parse: {e}", path.display()));
        assert!(
            wad.warnings().is_empty(),
            "{}: strict container parse produced warnings: {:?}",
            path.display(),
            wad.warnings()
        );

        // Classic marker+run maps (Doom / Heretic / Hexen / UDMF).
        for group in wad.map_groups() {
            for options in [ParseOptions::default(), ParseOptions::lenient()] {
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

        // Doom 64 nested-WAD maps.
        for lump in wad.lumps() {
            let bytes = wad.lump_data(lump);
            if !is_doom64_map_name(lump.name()) || !is_doom64_map_lump(bytes) {
                continue;
            }
            for options in [ParseOptions::default(), ParseOptions::lenient()] {
                let map = read_doom64_map(bytes, &options).unwrap_or_else(|e| {
                    panic!(
                        "{}: Doom 64 map {} failed {:?} read: {e}",
                        path.display(),
                        lump.name(),
                        options.strictness
                    )
                });
                assert!(
                    map.warnings().is_empty(),
                    "{}: Doom 64 map {} produced {:?} warnings: {:?}",
                    path.display(),
                    lump.name(),
                    options.strictness,
                    map.warnings()
                );
            }
            doom64_swept += 1;
        }
    }

    // The env var was set on purpose: a sweep that found WADs but no maps at
    // all means the collection (or the sweep) is broken, not clean.
    assert!(
        groups_swept + doom64_swept > 0,
        "CRUSTYWAD_SWEEP_DIR contained {} WAD file(s) but no maps were found",
        paths.len()
    );
    eprintln!(
        "swept {} WAD(s): {groups_swept} map group(s) + {doom64_swept} Doom 64 map(s), all clean in both modes",
        paths.len()
    );
}
