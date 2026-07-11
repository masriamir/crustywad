#![no_main]
use libfuzzer_sys::fuzz_target;

use crustywad::map::Map;
use crustywad::{ParseOptions, Wad};

fuzz_target!(|data: &[u8]| {
    if let Ok(wad) = Wad::from_bytes_with_options(data.to_vec(), ParseOptions::lenient()) {
        for group in wad.map_groups() {
            if let Ok(map) = Map::assemble_with_options(&wad, &group, ParseOptions::lenient()) {
                // Guard against unbounded warning growth: normalize_linedefs
                // pushes at most 4 warnings per linedef (start/end vertex,
                // right/left sidedef) and normalize_sidedefs at most 1 per
                // sidedef (sector reference).
                let warning_count = map.warnings().len();
                // Saturating arithmetic keeps the bound meaningful even for a
                // pathologically large synthesized map (no overflow).
                let bound = map
                    .linedefs()
                    .len()
                    .saturating_mul(4)
                    .saturating_add(map.sidedefs().len());
                assert!(
                    warning_count <= bound,
                    "warning count {warning_count} exceeded upper bound {bound}"
                );
                // O(input) allocation invariant (ADR-0016 §1): each arena is
                // decoded from one fixed-size-record lump whose bytes are a
                // subset of the WAD input, so element count <= input_len /
                // min_record_size. The divisors below are the SMALLEST record
                // size per arena across supported formats (Doom linedef 14 <
                // Hexen 16; Doom thing 10 < Hexen 20), so the bound holds for
                // Hexen maps too. Bounded per-arena (not summed) so it holds
                // even if a malicious WAD overlaps lumps.
                for (count, record_size, arena) in [
                    (map.vertices().len(), 4, "vertices"),
                    (map.linedefs().len(), 14, "linedefs"),
                    (map.sidedefs().len(), 30, "sidedefs"),
                    (map.sectors().len(), 26, "sectors"),
                    (map.things().len(), 10, "things"),
                ] {
                    assert!(
                        count <= data.len() / record_size,
                        "{arena} count {count} exceeds O(input) bound {}",
                        data.len() / record_size
                    );
                }
                std::hint::black_box(&map);
            }
        }
    }
});
