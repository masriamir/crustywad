#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let result = crustywad::Wad::from_bytes_with_options(
        data.to_vec(),
        crustywad::ParseOptions::lenient(),
    );
    if let Ok(wad) = result {
        // Guard against unbounded warning growth
        assert!(
            wad.warnings().len() <= wad.lump_count().saturating_mul(5).saturating_add(5),
            "warning count exceeded expected upper bound"
        );
        let _ = std::hint::black_box(wad);
    }
});
