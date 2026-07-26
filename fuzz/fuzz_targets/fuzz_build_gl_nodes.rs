#![no_main]
use libfuzzer_sys::fuzz_target;

use crustywad::map::Map;
use crustywad::map::build::{BuiltGlNodes, NodeBuildOptions, NodeBuildWarning, build_gl_nodes};
use crustywad::{ParseOptions, Wad};

// GL nodes are always an extended format, so every arena (segs, subsectors,
// nodes, and the combined original+GL vertex namespace) is bounded by the
// extended index ceiling, ADR-0026 §2 / ADR-0025 §5. `build_gl_nodes` errors
// above this, so a successful build stays within it. Declared locally, mirroring
// how `fuzz_build_nodes` declares its own ceiling consts.
const MAX_EXTENDED_INDEX: usize = 0x8000_0000;

fuzz_target!(|data: &[u8]| {
    let Ok(wad) = Wad::from_bytes_with_options(data.to_vec(), ParseOptions::lenient()) else {
        return;
    };
    for group in wad.map_groups() {
        let Ok(map) = Map::assemble_with_options(&wad, &group, ParseOptions::lenient()) else {
            continue;
        };

        // Primary oracle: `build_gl_nodes` never panics in either mode. Strict may
        // reject (empty geometry, a ceiling, a mixed-sector fan), which is a clean
        // `Err`, not a panic.
        let _ = build_gl_nodes(&map, &NodeBuildOptions::strict());

        // Lenient build: on success, every warning is an expected recovery variant
        // and the arenas obey the extended ceilings, the exact-partition
        // invariant, and the structural `validate` oracle.
        let Ok((built, warnings)) = build_gl_nodes(&map, &NodeBuildOptions::lenient()) else {
            continue;
        };

        for warning in &warnings {
            match warning {
                // The tolerated warning set: the narrowing recovery, the vanilla
                // soft ceiling, and the mixed-sector fan. This is a superset for
                // parity with the classic build — the GL kernel never emits
                // `VanillaCeilingExceeded` itself, but accepting it keeps this
                // oracle aligned with the shared contract.
                NodeBuildWarning::Write(_)
                | NodeBuildWarning::VanillaCeilingExceeded { .. }
                | NodeBuildWarning::MixedSectorSubsector { .. } => {}
                other => panic!("unexpected build_gl_nodes warning: {other:?}"),
            }
        }

        assert_output_bounds(&map, &built);
    }
});

/// The ADR-0026 §2 output bounds a successful GL build must satisfy.
fn assert_output_bounds(map: &Map, built: &BuiltGlNodes) {
    // Structural validation passes unconditionally in both modes: every leaf,
    // degenerate or not, is closed into a cyclic loop by the connecting-miniseg
    // path (a degenerate 1-seg leaf becomes a closed 2-vertex loop).
    if let Err(e) = built.validate(map.vertices().len()) {
        panic!("BuiltGlNodes::validate failed: {e:?}");
    }

    // Extended-index ceilings on every arena (the combined original+GL vertex
    // namespace, segs, subsectors, nodes).
    let vertices = map.vertices().len() + built.gl_vertices.len();
    assert!(
        vertices <= MAX_EXTENDED_INDEX,
        "vertices {vertices} exceed the extended ceiling"
    );
    assert!(
        built.segs.len() <= MAX_EXTENDED_INDEX,
        "segs {} exceed the extended ceiling",
        built.segs.len()
    );
    assert!(
        built.subsectors.len() <= MAX_EXTENDED_INDEX,
        "subsectors {} exceed the extended ceiling",
        built.subsectors.len()
    );
    assert!(
        built.nodes.len() <= MAX_EXTENDED_INDEX,
        "nodes {} exceed the extended ceiling",
        built.nodes.len()
    );

    // A full binary tree of leaves: exactly one more subsector than node, and at
    // least one subsector (an empty build is an `Err`, filtered above).
    assert!(
        !built.subsectors.is_empty(),
        "a successful build has >= 1 subsector"
    );
    if built.nodes.is_empty() {
        assert_eq!(
            built.subsectors.len(),
            1,
            "no nodes => exactly one subsector"
        );
    } else {
        assert_eq!(
            built.subsectors.len(),
            built.nodes.len() + 1,
            "nodes must equal subsectors - 1"
        );
    }

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
