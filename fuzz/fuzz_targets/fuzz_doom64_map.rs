#![no_main]
use libfuzzer_sys::fuzz_target;

use crustywad::ParseOptions;
use crustywad::map::read_doom64_map;

/// Wraps nested-WAD map bytes as a single `MAP01` lump in an outer PWAD.
fn wrap_as_map_lump(nested: &[u8]) -> Vec<u8> {
    let mut wad = Vec::with_capacity(12 + nested.len() + 16);
    wad.extend_from_slice(b"PWAD");
    wad.extend_from_slice(&1_i32.to_le_bytes());
    let dir_offset = i32::try_from(12 + nested.len()).unwrap_or(i32::MAX);
    wad.extend_from_slice(&dir_offset.to_le_bytes());
    wad.extend_from_slice(nested);
    wad.extend_from_slice(&12_i32.to_le_bytes());
    wad.extend_from_slice(
        &i32::try_from(nested.len())
            .unwrap_or(i32::MAX)
            .to_le_bytes(),
    );
    wad.extend_from_slice(b"MAP01\0\0\0");
    wad
}

fuzz_target!(|data: &[u8]| {
    // Path 1: arbitrary bytes must never panic. Most inputs are rejected up
    // front by the IWAD/PWAD magic guard (`Doom64ReadError::NotADoom64Map`);
    // this exercises that guard and the short-circuit.
    let _ = read_doom64_map(data, &ParseOptions::lenient());

    // Path 2: force valid `IWAD` magic so the fuzzer reaches the nested-WAD
    // container parse and per-record decoding (which the magic guard would
    // otherwise skip for the vast majority of inputs). Pad to the 12-byte
    // minimum WAD-header length so the magic guard (`len >= 12`) passes even for
    // very short `data` and the container parser is always reached.
    let mut input = Vec::with_capacity((4 + data.len()).max(12));
    input.extend_from_slice(b"IWAD");
    input.extend_from_slice(data);
    if input.len() < 12 {
        input.resize(12, 0);
    }

    if let Ok(map) = read_doom64_map(&input, &ParseOptions::lenient()) {
        // Oracle (ADR-0016 §1): O(input) allocation. Each record vector is
        // decoded from one sub-lump whose bytes are a subset of `input`, so its
        // count is bounded by input.len() / record_size. Bound each arena by its
        // own on-disk record size.
        for (count, record_size, arena) in [
            (map.things.len(), 14, "things"),
            (map.linedefs.len(), 16, "linedefs"),
            (map.sidedefs.len(), 12, "sidedefs"),
            (map.vertexes.len(), 8, "vertexes"),
            (map.sectors.len(), 24, "sectors"),
            (map.lights.len(), 6, "lights"),
            (map.segs.len(), 12, "segs"),
            (map.subsectors.len(), 4, "subsectors"),
            (map.nodes.len(), 28, "nodes"),
        ] {
            assert!(
                count <= input.len() / record_size,
                "{arena} count {count} exceeds O(input) bound {}",
                input.len() / record_size
            );
        }
        // Raw byte lumps are each a subset of the input.
        for raw in [&map.reject, &map.blockmap, &map.leafs, &map.macros] {
            assert!(raw.len() <= input.len(), "raw lump exceeds input length");
        }
        // Each of the 9 expected record lumps contributes at most one warning.
        assert!(
            map.warnings().len() <= 9,
            "warning count {} exceeds one-per-expected-lump bound",
            map.warnings().len()
        );

        // Path 3 (#244): the same nested-WAD bytes assembled through the
        // graph, in both strictness modes. Oracle: no panic; the LEAFS walk
        // consumed its lump exactly — every leaf entry is 4 bytes plus a
        // 2-byte count per subsector record (P_LoadLeafs stride), so the
        // sizes must reconcile with the raw lump. A strict success implies
        // exact consumption unconditionally (any surplus is malformed or a
        // count mismatch, both strict errors), including the all-zero-count
        // case; a lenient success only guarantees it when the arena is
        // non-empty (a lenient whole-degrade empties the arena while the
        // raw lump keeps its bytes).
        let outer = wrap_as_map_lump(&input);
        if let Ok(wad) = crustywad::Wad::from_bytes(outer) {
            if let Some(group) = wad.map_group("MAP01") {
                for (options, is_strict) in
                    [(ParseOptions::strict(), true), (ParseOptions::lenient(), false)]
                {
                    if let Ok(assembled) =
                        crustywad::map::Map::assemble_with_options(&wad, &group, options)
                    {
                        if is_strict || !assembled.leafs().is_empty() {
                            assert_eq!(
                                assembled.leafs().len() * 4 + assembled.subsectors().len() * 2,
                                map.leafs.len(),
                                "LEAFS walk must consume its lump exactly"
                            );
                        }

                        // #245: MACROS exact-consumption identity — header
                        // (4) + one count word (2) per macro + 6 bytes per
                        // action (P_LoadMacros reads count + 1 actions per
                        // record). A strict success on a NON-EMPTY raw lump
                        // implies exact consumption (empty is absent, and
                        // every structural surplus/shortfall is a strict
                        // error); a lenient success only guarantees it when
                        // macros survived (a whole-degrade empties them
                        // while the raw lump keeps its bytes).
                        if (is_strict && !map.macros.is_empty())
                            || !assembled.macros().is_empty()
                        {
                            let total_actions: usize =
                                assembled.macros().iter().map(|m| m.actions.len()).sum();
                            assert_eq!(
                                4 + assembled.macros().len() * 2 + total_actions * 6,
                                map.macros.len(),
                                "MACROS walk must consume its lump exactly"
                            );
                        }
                        std::hint::black_box(&assembled);
                    }
                }
            }
        }

        std::hint::black_box(&map);
    }
});
