#![no_main]
use libfuzzer_sys::fuzz_target;

use crustywad::gfx::Doom64Png;
use crustywad::{Limits, ParseOptions};

fuzz_target!(|data: &[u8]| {
    for strictness_opts in [ParseOptions::strict(), ParseOptions::lenient()] {
        // Tight cap keeps per-iteration work bounded; the cap oracle is
        // the point (ADR-0016): no decode may exceed it.
        let options = ParseOptions {
            limits: Limits::new().with_max_decoded_pixels(1 << 16),
            ..strictness_opts
        };
        if let Ok(img) = Doom64Png::decode(data, &options) {
            let area = usize::from(img.width) * usize::from(img.height);
            assert!(area <= 1 << 16, "decode exceeded the cap");
            assert_eq!(img.pixels().len(), area);
            assert!(img.plte().len() <= 256);
            assert!(img.trns().len() <= img.plte().len());
            let indexed = img.to_indexed();
            assert_eq!(indexed.pixels.len(), area);
            assert_eq!(indexed.mask.len(), area);
            assert_eq!(img.to_rgba().pixels.len(), area * 4);
        }
    }
});
