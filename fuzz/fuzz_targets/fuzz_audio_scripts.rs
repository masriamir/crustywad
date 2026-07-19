#![no_main]
use libfuzzer_sys::fuzz_target;

use crustywad::ParseOptions;
use crustywad::audio::{SndCurve, SndInfo, SndSeq};

fuzz_target!(|data: &[u8]| {
    for opts in [ParseOptions::strict(), ParseOptions::lenient()] {
        if let Ok(curve) = SndCurve::parse(data, &opts) {
            // The table is an owned copy of the whole lump.
            assert!(curve.bytes().len() <= data.len());
            assert!(curve.warnings().is_empty());
        }
        if let Ok(info) = SndInfo::parse(data, &opts) {
            // Oracles (ADR-0016): output and warnings bounded by the input.
            // Each entry consumes two tokens and each map song three, so both
            // are bounded by the token count, which is bounded by the length.
            assert!(info.entries().len() <= data.len().saturating_add(8));
            assert!(info.map_songs().len() <= data.len().saturating_add(8));
            assert!(info.warnings().len() <= data.len().saturating_add(8));
        }
        if let Ok(seq) = SndSeq::parse(data, &opts) {
            let commands: usize = seq.sequences().iter().map(|s| s.commands.len()).sum();
            assert!(seq.sequences().len() <= data.len().saturating_add(8));
            assert!(commands <= data.len().saturating_add(8));
            assert!(seq.warnings().len() <= data.len().saturating_add(8));
        }
    }
});
