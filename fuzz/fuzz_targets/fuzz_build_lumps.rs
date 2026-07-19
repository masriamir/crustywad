#![no_main]
use libfuzzer_sys::fuzz_target;

use crustywad::map::Map;
use crustywad::map::build::{NodeBuildOptions, build_blockmap, build_reject};
use crustywad::{ParseOptions, Wad};

fuzz_target!(|data: &[u8]| {
    let Ok(wad) = Wad::from_bytes_with_options(data.to_vec(), ParseOptions::lenient()) else {
        return;
    };
    for group in wad.map_groups() {
        let Ok(map) = Map::assemble_with_options(&wad, &group, ParseOptions::lenient()) else {
            continue;
        };

        // REJECT is infallible. Its byte length is exactly ceil(sectors^2 / 8)
        // (ADR-0024 §7) — bounded by the sector count, hence O(input) (ADR-0016
        // §1). No panic is the primary oracle; the size formula is the O(input)
        // assertion.
        let reject = build_reject(&map);
        let sectors = map.sectors().len();
        let expected = sectors.saturating_mul(sectors).div_ceil(8);
        let reject_len = reject.to_lump_bytes().len();
        assert_eq!(
            reject_len, expected,
            "REJECT length {reject_len} != ceil({sectors}^2/8)={expected}"
        );

        // Strict may reject (empty geometry, or a blocklist offset past the
        // vanilla/word ceiling — the point of the loss policy); it must never
        // panic.
        let _ = build_blockmap(&map, &NodeBuildOptions::strict());

        // Lenient recovers the vanilla-ceiling overflow with a warning; it still
        // errors on an unencodable (> 65,535 word) offset. On success the grid
        // and the packed lump stay within the documented O(input) output bound
        // (ADR-0024 §5): columns and rows are each at most 512, and the word
        // image is header + offset table + per-block lists.
        if let Ok((bm, _warnings)) = build_blockmap(&map, &NodeBuildOptions::lenient()) {
            let cells = bm.columns().saturating_mul(bm.rows());
            assert!(cells <= 512 * 512, "grid {cells} cells exceeds 512*512");

            // build_blockmap sources every entry from a `u16` word, so
            // `to_lump_bytes` is infallible here (its error guards only a future
            // wider in-crate constructor).
            let len = bm.to_lump_bytes().unwrap().len();
            let linedefs = map.linedefs().len();
            let bound = 2usize.saturating_mul(
                4usize
                    .saturating_add(cells)
                    .saturating_add(cells.saturating_mul(2usize.saturating_add(linedefs))),
            );
            assert!(len <= bound, "BLOCKMAP length {len} exceeds bound {bound}");
        }
    }
});
