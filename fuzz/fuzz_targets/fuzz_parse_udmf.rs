#![no_main]
use libfuzzer_sys::fuzz_target;

use crustywad::Limits;
use crustywad::map::udmf::parse_udmf;

fuzz_target!(|data: &[u8]| {
    // Only valid UTF-8 reaches the parser; malformed UTF-8 is skipped (byte->str
    // decoding belongs to a later assembly pass, not to `parse_udmf`). Feed
    // arbitrary text and assert the no-panic oracle plus an O(input) output-size
    // bound.
    if let Ok(text) = std::str::from_utf8(data) {
        if let Ok(map) = parse_udmf(text, Limits::default()) {
            // O(input) element-count bound (ADR-0016 §1): each produced element
            // requires at least one `{`...`}` pair, so the total element count
            // is bounded by the input byte length. (This oracle checks element
            // count, not total allocated bytes.)
            let elements = map.vertices.len()
                + map.linedefs.len()
                + map.sidedefs.len()
                + map.sectors.len()
                + map.things.len();
            assert!(
                elements <= data.len(),
                "element count {elements} exceeds O(input) bound {}",
                data.len()
            );
            std::hint::black_box(&map);
        }
    }
});
