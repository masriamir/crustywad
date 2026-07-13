#![no_main]
use libfuzzer_sys::fuzz_target;

use crustywad::ParseOptions;
use crustywad::map::read_doom64_map;

fuzz_target!(|data: &[u8]| {
    // Path 1: arbitrary bytes must never panic. Most inputs are rejected up
    // front by the IWAD/PWAD magic guard (`Doom64ReadError::NotADoom64Map`);
    // this exercises that guard and the short-circuit.
    let _ = read_doom64_map(data, &ParseOptions::lenient());

    // Path 2: force valid `IWAD` magic so the fuzzer reaches the nested-WAD
    // container parse and per-record decoding (which the magic guard would
    // otherwise skip for the vast majority of inputs).
    let mut input = Vec::with_capacity(4 + data.len());
    input.extend_from_slice(b"IWAD");
    input.extend_from_slice(data);

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
        std::hint::black_box(&map);
    }
});
