#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let result = crustywad::Wad::from_bytes_with_options(
        data.to_vec(),
        crustywad::ParseOptions::lenient(),
    );
    if let Ok(wad) = result {
        // O(input) allocation invariant (ADR-0016 §1): 16-byte directory entries
        // bound the lump count by input_len / 16 even in lenient mode, where
        // lump_count is capped at the available directory span.
        assert!(
            wad.lump_count() <= data.len() / 16,
            "lump_count {} exceeds O(input) bound {}",
            wad.lump_count(),
            data.len() / 16
        );
        // Guard against unbounded warning growth
        let warning_count = wad.warnings().len();
        let bound = wad.lump_count().saturating_mul(5).saturating_add(5);
        assert!(
            warning_count <= bound,
            "warning count {warning_count} exceeded upper bound {bound}"
        );
        let _ = std::hint::black_box(wad);
    }
});
