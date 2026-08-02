#![no_main]
use libfuzzer_sys::fuzz_target;

use crustywad::map::Map;
use crustywad::map::build::{
    BuiltGlNodes, NodeBuildError, NodeBuildOptions, NodeBuildWarning, NodeFormat,
    add_doom_map_with_nodes, build_gl_nodes,
};
use crustywad::{ParseOptions, Wad, WadBuilder, WadKind, WriteOptions};

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

        // Serialize -> decode oracle (ADR-0026 §5, PR #370's hardening
        // statement, extended to the `Gl` auto-resolution path by #365 Task
        // 4): the *fully public* write path — `add_doom_map_with_nodes`
        // targeting `NodeFormat::Gl` — must never panic, and neither may
        // feeding its output straight back through the crate's own reader and
        // assembler. This is the writer-side twin of the read-only oracle
        // above: `build_gl_nodes` alone proves the in-memory kernel is safe,
        // this proves the on-disk auto-resolved stream (`XGLN`, escalating to
        // `XGL2`/`XGL3` as the geometry demands) it produces is too.
        //
        // `add_doom_map_with_nodes` re-runs its own write pass, `REJECT`,
        // `BLOCKMAP`, and `build_gl_nodes` internally (independent of the
        // in-memory `built` above), so it can legitimately fail with any of
        // the same errors those builders document — this is not a narrower
        // guarantee than the read-only oracle already gave. `format` is
        // always a GL variant here, so `UnsupportedNodeFormat` — a
        // non-GL-format-serializing-GL-data caller-misuse guard — can never
        // fire; if it ever does, that is a real bug and the catch-all below
        // panics on it.
        //
        // `NodeFormat::Gl` drives `resolve_gl_dialect`'s escalation
        // (`Xgln` -> `Xgl2` -> `Xgl3`) through this same fuzz surface, so two
        // error shapes are *deliberately* excluded from the tolerated set
        // below even though they are `TooManyElements`/`PartitionPrecision`
        // in spirit: `NodeBuildError::PartitionPrecision` and the
        // `Xgln`-dialect seg-linedef `TooManyElements { kind: "linedefs" }`
        // are documented as unreachable once resolution has escalated past
        // them (`to_extended_lump_bytes`'s doc comment, `resolve_gl_dialect`'s
        // doc comment) — seeing either here would mean the auto-resolution
        // logic failed to escalate far enough, a real bug, not a tolerated
        // narrowing. `PartitionPrecision` is its own variant, so it is simply
        // left out of the match below and falls to the catch-all panic.
        // `TooManyElements` splits by `kind`: the `"linedefs"` case panics (a
        // linedef ceiling on this path can only mean resolution failed to
        // escalate past XGLN — parsed-map linedef indices sit far below the
        // u32 ceilings), while the general arena/index-ceiling kinds (segs,
        // vertices, subsectors, nodes) stay tolerated as legitimate outcomes.
        //
        // `NodeBuildOptions` is `#[non_exhaustive]`, so it cannot be built with
        // struct-literal syntax outside the crate; start from `lenient()` (which
        // already sets `strictness: Strictness::Lenient`) and override the
        // public `format` field in place.
        let mut gl_opts = NodeBuildOptions::lenient();
        gl_opts.format = NodeFormat::Gl;
        let mut builder = WadBuilder::new(WadKind::Pwad);
        match add_doom_map_with_nodes(
            &mut builder,
            "MAP01",
            &map,
            &WriteOptions::lenient(),
            &gl_opts,
        ) {
            Ok(_warnings) => {
                // Finish the WAD, then feed it back through the crate's own
                // reader and assembler. Neither call is asserted to succeed —
                // only to never panic — mirroring every other oracle in this
                // suite; genuine round-trip fidelity is covered by the golden
                // and round-trip integration tests (ADR-0026 §6), not fuzzing.
                if let Ok((bytes, _write_warnings)) =
                    builder.build_with_options(&WriteOptions::lenient())
                    && let Ok(wad2) = Wad::from_bytes_with_options(bytes, ParseOptions::lenient())
                {
                    // The just-written map must be discoverable: a written
                    // WAD whose MAP01 run is unrecognizable would otherwise
                    // make this whole oracle a silent no-op.
                    assert!(
                        !wad2.map_groups().is_empty(),
                        "one-shot auto-resolved GL output lost its map group on re-parse"
                    );
                    for group2 in wad2.map_groups() {
                        let _ = Map::assemble_with_options(&wad2, &group2, ParseOptions::lenient());
                    }
                }
            }
            // The known, tolerated error set every other builder in this
            // target already accepts (structural ceilings, hardening guards,
            // and write-path narrowing) — a clean `Err`, not a bug.
            // `NodeBuildError::PartitionPrecision` is deliberately absent: on
            // the `Gl` auto-resolution path it is documented unreachable (see
            // the comment above `gl_opts.format`), so seeing it here is a
            // resolution bug and must panic via the catch-all. The same holds
            // for the XGLN linedef-sentinel ceiling: on this path linedef
            // indices come from parsed map linedefs (bounded far below the
            // u32 ceilings), so a `kind: "linedefs"` overflow can only mean
            // resolution failed to escalate past XGLN — panic on it while
            // tolerating the general arena/index ceilings.
            Err(NodeBuildError::TooManyElements {
                kind: "linedefs", ..
            }) => panic!("Gl auto-resolution failed to escalate past the XGLN linedef ceiling"),
            Err(
                NodeBuildError::Write(_)
                | NodeBuildError::EmptyGeometry
                | NodeBuildError::BlockmapOverflow { .. }
                | NodeBuildError::TooManyElements { .. }
                | NodeBuildError::MinisegUnsupported { .. }
                | NodeBuildError::MixedSectorSubsector { .. }
                | NodeBuildError::DegeneratePartition { .. }
                | NodeBuildError::CompressionUnavailable
                | NodeBuildError::InvalidStructure(_),
            ) => {}
            Err(other) => panic!("unexpected add_doom_map_with_nodes error: {other:?}"),
        }
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
