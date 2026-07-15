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
//!
//! A second, gate-expecting collection (`CRUSTYWAD_SWEEP_EXTENDED_DIR`, #269)
//! holds WADs whose `NODES` lumps carry extended/ZDBSP encodings. Those
//! assert a **different contract** — every map errors
//! `UnsupportedNodeEncoding` in strict mode and recovers leniently with empty
//! BSP arenas — which is not an allowlist: the primary sweep's zero-failure
//! invariant is untouched. Fixtures are ZDBSP-derived Freedoom variants
//! (BSD-licensed), regenerable from source: build zdbsp
//! (`cmake -B build -DCMAKE_BUILD_TYPE=Release
//! -DCMAKE_POLICY_VERSION_MINIMUM=3.5 && cmake --build build` — the policy
//! flag is required under `CMake 4`), then `zdbsp -X -o freedoom1-xnod.wad
//! freedoom1.wad` (`XNOD`) and `zdbsp -z -o freedoom1-znod.wad freedoom1.wad`
//! (`ZNOD`). When #199 implements real extended-node reading, this collection
//! becomes its positive-read fixture set.

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

/// The gate-expecting sweep (#269): every map in every WAD under
/// `CRUSTYWAD_SWEEP_EXTENDED_DIR` must trip the extended-encoding gate —
/// strict assembly fails with [`MapAssembleError::UnsupportedNodeEncoding`],
/// lenient assembly succeeds with all three BSP arenas empty, the gate
/// warning recorded, and the map's geometry intact.
#[test]
fn extended_sweep_asserts_the_gate_contract() {
    use crustywad::map::{MapAssembleError, MapWarning};

    let paths = common::wad_files("CRUSTYWAD_SWEEP_EXTENDED_DIR");
    if paths.is_empty() {
        return; // skip note already printed by the helper
    }

    let mut gated = 0usize;
    for path in &paths {
        let wad = Wad::from_path_with_options(path, ParseOptions::strict())
            .unwrap_or_else(|e| panic!("{}: container failed strict parse: {e}", path.display()));

        let groups = wad.map_groups();
        assert!(
            !groups.is_empty(),
            "{}: an extended-node fixture with no maps is a broken fixture",
            path.display()
        );
        for group in &groups {
            let err = Map::assemble_with_options(&wad, group, ParseOptions::strict())
                .expect_err("an extended-node map must fail strict assembly");
            assert!(
                matches!(err, MapAssembleError::UnsupportedNodeEncoding { .. }),
                "{}: map {} failed strict assembly with {err:?}, not the gate",
                path.display(),
                group.name
            );

            let map = Map::assemble_with_options(&wad, group, ParseOptions::lenient())
                .unwrap_or_else(|e| {
                    panic!(
                        "{}: map {} must recover leniently: {e}",
                        path.display(),
                        group.name
                    )
                });
            assert!(map.segs().is_empty(), "gated BSP leaves segs empty");
            assert!(
                map.subsectors().is_empty(),
                "gated BSP leaves subsectors empty"
            );
            assert!(map.nodes().is_empty(), "gated BSP leaves nodes empty");
            assert!(
                map.warnings()
                    .iter()
                    .any(|w| matches!(w, MapWarning::UnsupportedNodeEncoding { .. })),
                "{}: map {} lacks the gate warning",
                path.display(),
                group.name
            );
            assert!(
                !map.linedefs().is_empty(),
                "{}: map {} lost its geometry",
                path.display(),
                group.name
            );
            gated += 1;
        }
    }
    assert!(gated > 0, "extended sweep dir set but no maps were gated");
    eprintln!(
        "extended sweep: {} WAD(s), {gated} map(s), gate contract held in both modes",
        paths.len()
    );
}
