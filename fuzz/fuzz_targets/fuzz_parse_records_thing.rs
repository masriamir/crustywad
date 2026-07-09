#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = std::hint::black_box(crustywad::map::parse_records::<crustywad::map::doom::Thing>(data));
});
