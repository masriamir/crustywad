#![no_main]
use libfuzzer_sys::fuzz_target;

use crustywad::map::Map;
use crustywad::map::build::{
    BuiltNodes, NodeBuildError, NodeBuildOptions, NodeBuildWarning, build_nodes,
};
use crustywad::{ParseOptions, Wad};

// The 15-bit BSP child-reference ceiling (subsectors/nodes) and the u16 index
// ceiling (vertices/segs), ADR-0024 §5. `build_nodes` errors above these, so a
// successful build stays within them.
const MAX_BSP_INDEX: usize = 0x8000;
const MAX_U16_INDEXED: usize = 0x1_0000;

fuzz_target!(|data: &[u8]| {
    let Ok(wad) = Wad::from_bytes_with_options(data.to_vec(), ParseOptions::lenient()) else {
        return;
    };
    for group in wad.map_groups() {
        let Ok(map) = Map::assemble_with_options(&wad, &group, ParseOptions::lenient()) else {
            continue;
        };

        // Primary oracle: `build_nodes` never panics in either mode. Strict may
        // reject (empty geometry, a §5 ceiling, or a mixed-sector fan — the loss
        // policy, ADR-0024 §7 amendment), which is a clean `Err`, not a panic.
        let _ = build_nodes(&map, &NodeBuildOptions::strict());

        // Lenient build: on success, every warning is an expected recovery
        // variant (mixed-sector is now tolerated) and the arenas obey the §5
        // ceilings and the exact-partition invariant.
        let Ok((built, warnings)) = build_nodes(&map, &NodeBuildOptions::lenient()) else {
            continue;
        };

        for warning in &warnings {
            match warning {
                // The narrowing recoveries, the vanilla soft ceiling, and the
                // tolerated mixed-sector fan are the only node-build warnings.
                NodeBuildWarning::Write(_)
                | NodeBuildWarning::VanillaCeilingExceeded { .. }
                | NodeBuildWarning::MixedSectorSubsector { .. } => {}
                // `DegeneratePartition` is an error, never a warning, and is
                // unreachable after the classify<->split unification; the
                // blockmap-only warnings never come from `build_nodes`.
                other => panic!("unexpected build_nodes warning: {other:?}"),
            }
        }

        assert_output_bounds(&map, &built);

        // Serialization either succeeds or fails only with the additive
        // structural-ceiling error; `build_nodes` clamps offsets, so the
        // defensive offset error is never tripped by a real build.
        match built.to_lump_bytes() {
            Ok(_) => {}
            Err(NodeBuildError::TooManyElements { .. }) => {}
            other => panic!("unexpected to_lump_bytes result: {other:?}"),
        }
    }
});

/// The ADR-0024 §5 output bounds a successful build must satisfy.
fn assert_output_bounds(map: &Map, built: &BuiltNodes) {
    let vertices = map.vertices().len() + built.split_vertices.len();
    assert!(
        vertices <= MAX_U16_INDEXED,
        "vertices {vertices} exceed the u16 ceiling"
    );
    assert!(
        built.segs.len() <= MAX_U16_INDEXED,
        "segs {} exceed the u16 ceiling",
        built.segs.len()
    );
    assert!(
        built.subsectors.len() <= MAX_BSP_INDEX,
        "subsectors {} exceed the 15-bit ceiling",
        built.subsectors.len()
    );
    assert!(
        built.nodes.len() <= MAX_BSP_INDEX,
        "nodes {} exceed the 15-bit ceiling",
        built.nodes.len()
    );

    // A full binary tree of leaves: exactly one more subsector than node, and at
    // least one subsector (an empty build is an `Err`, filtered above).
    assert!(
        !built.subsectors.is_empty(),
        "a successful build has >= 1 subsector"
    );
    assert_eq!(
        built.subsectors.len(),
        built.nodes.len() + 1,
        "nodes must equal subsectors - 1"
    );

    // Subsector seg ranges are contiguous and partition the seg arena exactly.
    let mut cursor = 0;
    for subsector in &built.subsectors {
        assert_eq!(
            subsector.segs.start, cursor,
            "subsector segs are not contiguous"
        );
        assert!(
            subsector.segs.end >= subsector.segs.start,
            "inverted subsector seg range"
        );
        cursor = subsector.segs.end;
    }
    assert_eq!(
        cursor,
        built.segs.len(),
        "subsectors must partition segs exactly"
    );
}
