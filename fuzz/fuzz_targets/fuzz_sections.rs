#![no_main]
use libfuzzer_sys::fuzz_target;

use crustywad::{ParseOptions, Wad};

// Oracles (ADR-0016): no panic in either strictness mode; section count
// (incl. children) bounded by lump count; every range well-formed and
// in-bounds (end_marker may equal the lump count — the lenient EOF-closed
// convention); nesting depth <= 2 (children have no children); warnings
// bounded by one per marker lump, conservatively <= lump count (a marker
// that warned at open — orphan promotion, duplicate pair — has its EOF
// UnpairedStart suppressed, so the one-per-marker bound holds exactly).
fuzz_target!(|data: &[u8]| {
    if let Ok(wad) = Wad::from_bytes_with_options(data.to_vec(), ParseOptions::lenient()) {
        let n = wad.lumps().len();
        for options in [ParseOptions::strict(), ParseOptions::lenient()] {
            if let Ok(table) = wad.sections_with_options(options) {
                let total: usize = table
                    .sections()
                    .iter()
                    .map(|s| 1 + s.sub_sections.len())
                    .sum();
                assert!(total <= n, "sections exceed lump count");
                for s in table.sections() {
                    assert!(s.start_marker < s.end_marker && s.end_marker <= n);
                    assert!(s.lumps.start == s.start_marker + 1 && s.lumps.end == s.end_marker);
                    for c in &s.sub_sections {
                        assert!(c.sub_sections.is_empty(), "depth > 2");
                        assert!(c.start_marker > s.start_marker && c.end_marker <= s.end_marker);
                    }
                }
                assert!(table.warnings().len() <= n, "warnings exceed lump count");
                std::hint::black_box(&table);
            }
        }
    }
});
