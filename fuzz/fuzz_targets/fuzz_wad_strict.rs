#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(wad) = crustywad::Wad::from_bytes(data.to_vec()) {
        // O(input) allocation invariant (ADR-0016 §1): a lump directory entry is
        // 16 bytes, so the parsed lump count cannot exceed input_len / 16.
        assert!(
            wad.lump_count() <= data.len() / 16,
            "lump_count {} exceeds O(input) bound {}",
            wad.lump_count(),
            data.len() / 16
        );
        let _ = std::hint::black_box(wad);
    }
});
