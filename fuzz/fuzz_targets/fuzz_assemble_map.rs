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
                let bound = map.linedefs().len() * 4 + map.sidedefs().len();
                assert!(
                    warning_count <= bound,
                    "warning count {warning_count} exceeded upper bound {bound}"
                );
                std::hint::black_box(&map);
            }
        }
    }
});
