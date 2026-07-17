#![no_main]
use libfuzzer_sys::fuzz_target;

use crustywad::map::{Map, write_doom_map};
use crustywad::{ParseOptions, Wad, WriteOptions};

fuzz_target!(|data: &[u8]| {
    let Ok(wad) = Wad::from_bytes_with_options(data.to_vec(), ParseOptions::lenient()) else {
        return;
    };
    for group in wad.map_groups() {
        let Ok(map) = Map::assemble_with_options(&wad, &group, ParseOptions::lenient()) else {
            continue;
        };

        // Strict may reject (that is the point of the loss policy); it must
        // never panic.
        let _ = write_doom_map(&map, &WriteOptions::strict());

        // Lenient must always succeed unless an arena is unindexable.
        if let Ok((lumps, warnings)) = write_doom_map(&map, &WriteOptions::lenient()) {
            // O(input) output (ADR-0016 §1): every lump is exactly
            // element_count * record_size, and each element count is bounded by
            // the assembled map's arenas, which are themselves bounded by the
            // WAD input length.
            assert_eq!(lumps.vertexes.len(), map.vertices().len() * 4);
            assert_eq!(lumps.linedefs.len(), map.linedefs().len() * 14);
            assert_eq!(lumps.sidedefs.len(), map.sidedefs().len() * 30);
            assert_eq!(lumps.sectors.len(), map.sectors().len() * 26);
            assert_eq!(lumps.things.len(), map.things().len() * 10);

            // Bounded warning growth: at most a small constant per element,
            // plus the single always-on NodesNotBuilt warning, plus at most
            // one more ColoredLightingDropped warning for a Doom 64-sourced
            // map (ADR-0021 §5 amendment 3 — one per map, not per element).
            // 16 is a loose upper bound on the per-element warning count (the
            // widest element, a thing, can emit at most 4 drops + 2 per
            // coordinate + 1 flags).
            let elements = map.vertices().len()
                + map.linedefs().len()
                + map.sidedefs().len()
                + map.sectors().len()
                + map.things().len();
            let bound = elements.saturating_mul(16).saturating_add(2);
            assert!(
                warnings.len() <= bound,
                "warning count {} exceeded upper bound {bound}",
                warnings.len()
            );
        }
    }
});
