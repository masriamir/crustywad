#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(records) = crustywad::map::parse_records::<crustywad::map::doom::Thing>(data) {
        // O(input) allocation invariant (ADR-0016 §1): each Thing record is
        // 10 bytes, so the decoded record count is bounded by input_len / 10.
        assert!(
            records.len() <= data.len() / 10,
            "record count {} exceeds O(input) bound {}",
            records.len(),
            data.len() / 10
        );
        let _ = std::hint::black_box(records);
    }
});
