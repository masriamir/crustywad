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
//! A second collection (`CRUSTYWAD_SWEEP_EXTENDED_DIR`, #269) holds WADs
//! whose `NODES` lumps carry extended/ZDBSP encodings. Since #326 (Stage 1)
//! decodes the uncompressed `X*` formats (XNOD/XGLN/XGL2/XGL3), this
//! collection is now **mixed**: XNOD-generated fixtures positive-read (assert
//! `segs()`/`subsectors()`/`nodes()` are populated and internally
//! consistent) while ZNOD-generated ones still trip the extended-encoding
//! gate (compressed `Z*` is Stage 2, #327) — every map is classified by its
//! *assembly behavior*, not by peeking at the on-disk signature (this is an
//! integration test; it cannot see the crate-internal `ExtendedNodeKind`).
//! Neither branch is an allowlist: a map that assembles with populated BSP
//! arenas but fails the positive-read consistency checks, or that errors
//! with anything other than `UnsupportedNodeEncoding`, fails the test.
//! Fixtures are ZDBSP-derived Freedoom variants (BSD-licensed), regenerable
//! from source: build zdbsp (`cmake -B build -DCMAKE_BUILD_TYPE=Release
//! -DCMAKE_POLICY_VERSION_MINIMUM=3.5 && cmake --build build` — the policy
//! flag is required under `CMake 4`), then `zdbsp -X -o freedoom1-xnod.wad
//! freedoom1.wad` (`XNOD`) and `zdbsp -z -o freedoom1-znod.wad freedoom1.wad`
//! (`ZNOD`).

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

/// The extended-node sweep (#269, split for positive-read by #326): every
/// map in every WAD under `CRUSTYWAD_SWEEP_EXTENDED_DIR` is classified by
/// its strict-assembly *behavior*, not by its on-disk signature (which this
/// integration test cannot inspect):
///
/// - `Ok(map)` with a non-empty `segs()` is a decodable uncompressed `X*`
///   fixture (XNOD/XGLN/XGL2/XGL3, #326 Stage 1). It must satisfy the
///   **positive-read contract**: `segs()`/`subsectors()`/`nodes()` are all
///   non-empty, internally consistent (every subsector's seg range and every
///   node's child indices are in bounds, the BSP root resolves), the map's
///   geometry is intact, and lenient assembly agrees (also populated, with
///   no warnings).
/// - `Err(MapAssembleError::UnsupportedNodeEncoding)` is a still-gated `Z*`/
///   `xNd4` fixture (Stage 2, #327). It must satisfy the **gate contract**:
///   strict fails with that variant, lenient recovers with all three BSP
///   arenas empty plus the gate warning, and the map's geometry is intact.
/// - Any other `Err` is a real regression and fails the test loudly.
///
/// Neither branch is an allowlist — every map in the collection must land in
/// exactly one of the two contracts above. The counters require at least one
/// map overall; a family with zero matches (a dir holding only one fixture
/// family) is tolerated rather than forced.
#[test]
fn extended_sweep_classifies_positive_read_vs_gated() {
    use crustywad::map::{MapAssembleError, MapWarning};

    let paths = common::wad_files("CRUSTYWAD_SWEEP_EXTENDED_DIR");
    if paths.is_empty() {
        return; // skip note already printed by the helper
    }

    let mut positive = 0usize;
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
            match Map::assemble_with_options(&wad, group, ParseOptions::strict()) {
                Ok(map) if !map.segs().is_empty() => {
                    assert_positive_read(path, group, &map);

                    let lenient = Map::assemble_with_options(&wad, group, ParseOptions::lenient())
                        .unwrap_or_else(|e| {
                            panic!(
                                "{}: map {} must also assemble leniently: {e}",
                                path.display(),
                                group.name
                            )
                        });
                    assert_positive_read(path, group, &lenient);
                    assert!(
                        lenient.warnings().is_empty(),
                        "{}: map {} produced lenient warnings on a decodable extended-node map: {:?}",
                        path.display(),
                        group.name,
                        lenient.warnings()
                    );
                    positive += 1;
                }
                Ok(map) => panic!(
                    "{}: map {} assembled strictly but left segs()/subsectors()/nodes() empty \
                     (an X* fixture must decode into a populated BSP; a genuinely gated one \
                     must error with UnsupportedNodeEncoding): {:?}",
                    path.display(),
                    group.name,
                    map
                ),
                Err(MapAssembleError::UnsupportedNodeEncoding { .. }) => {
                    let lenient = Map::assemble_with_options(&wad, group, ParseOptions::lenient())
                        .unwrap_or_else(|e| {
                            panic!(
                                "{}: map {} must recover leniently: {e}",
                                path.display(),
                                group.name
                            )
                        });
                    assert!(lenient.segs().is_empty(), "gated BSP leaves segs empty");
                    assert!(
                        lenient.subsectors().is_empty(),
                        "gated BSP leaves subsectors empty"
                    );
                    assert!(lenient.nodes().is_empty(), "gated BSP leaves nodes empty");
                    assert!(
                        lenient
                            .warnings()
                            .iter()
                            .any(|w| matches!(w, MapWarning::UnsupportedNodeEncoding { .. })),
                        "{}: map {} lacks the gate warning",
                        path.display(),
                        group.name
                    );
                    assert!(
                        !lenient.linedefs().is_empty(),
                        "{}: map {} lost its geometry",
                        path.display(),
                        group.name
                    );
                    gated += 1;
                }
                Err(e) => panic!(
                    "{}: map {} failed strict assembly with {e:?}, neither a positive read nor the gate",
                    path.display(),
                    group.name
                ),
            }
        }
    }

    assert!(
        positive + gated > 0,
        "extended sweep dir set but no maps were classified"
    );
    eprintln!(
        "extended sweep: {} WAD(s), {} map(s), {positive} positive-read, {gated} gated",
        paths.len(),
        positive + gated
    );
}

/// Asserts the positive-read contract for a decodable extended-node map: the
/// three BSP arenas are populated and internally consistent, and the map's
/// geometry survived assembly.
fn assert_positive_read(path: &std::path::Path, group: &crustywad::map::MapGroup, map: &Map) {
    assert!(
        !map.segs().is_empty(),
        "{}: map {} has an empty seg arena",
        path.display(),
        group.name
    );
    assert!(
        !map.subsectors().is_empty(),
        "{}: map {} has an empty subsector arena",
        path.display(),
        group.name
    );
    assert!(
        !map.nodes().is_empty(),
        "{}: map {} has an empty node arena",
        path.display(),
        group.name
    );
    assert!(
        !map.linedefs().is_empty(),
        "{}: map {} lost its geometry",
        path.display(),
        group.name
    );

    let seg_count = map.segs().len();
    for (i, subsector) in map.subsectors().iter().enumerate() {
        assert!(
            subsector.segs.end <= seg_count && subsector.segs.start <= subsector.segs.end,
            "{}: map {} subsector {i} has an out-of-range seg run {:?} (segs len {seg_count})",
            path.display(),
            group.name,
            subsector.segs
        );
    }

    let node_count = map.nodes().len();
    let subsector_count = map.subsectors().len();
    for (i, node) in map.nodes().iter().enumerate() {
        for child in [node.right, node.left] {
            match child {
                crustywad::map::NodeChild::Node(idx) => assert!(
                    idx.0 < node_count,
                    "{}: map {} node {i} has an out-of-range child node index {}",
                    path.display(),
                    group.name,
                    idx.0
                ),
                crustywad::map::NodeChild::Subsector(idx) => assert!(
                    idx.0 < subsector_count,
                    "{}: map {} node {i} has an out-of-range child subsector index {}",
                    path.display(),
                    group.name,
                    idx.0
                ),
            }
        }
    }

    assert!(
        map.bsp_root().is_some(),
        "{}: map {} has nodes but no resolvable BSP root",
        path.display(),
        group.name
    );
}
