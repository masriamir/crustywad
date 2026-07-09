#![no_main]
use libfuzzer_sys::fuzz_target;

use crustywad::map::Map;
use crustywad::{ParseOptions, Wad};

fuzz_target!(|data: &[u8]| {
    if let Ok(wad) = Wad::from_bytes_with_options(data.to_vec(), ParseOptions::lenient()) {
        for group in wad.map_groups() {
            let _ = std::hint::black_box(Map::assemble_with_options(
                &wad,
                &group,
                ParseOptions::lenient(),
            ));
        }
    }
});
