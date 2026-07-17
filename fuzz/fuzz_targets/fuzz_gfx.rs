#![no_main]
use libfuzzer_sys::fuzz_target;

use crustywad::ParseOptions;
use crustywad::gfx::{Colormap, Flat, Picture, Playpal};

fuzz_target!(|data: &[u8]| {
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        if let Ok(pic) = Picture::parse(data, &options) {
            // Oracles (ADR-0016): structure and work bounded by the input.
            assert_eq!(pic.columns().len(), usize::from(pic.width));
            assert!(usize::from(pic.width) <= data.len() / 4 + 1);
            // Consumed-bytes budget: per-post 4 + pixels, cumulative ≤ len.
            let consumed: usize = pic
                .columns()
                .iter()
                .flat_map(|c| c.posts.iter())
                .map(|p| 4 + p.pixels.len())
                .sum();
            assert!(consumed <= data.len(), "post budget exceeded the lump");
            let img = pic.to_indexed();
            let area = usize::from(pic.width) * usize::from(pic.height);
            assert_eq!(img.pixels.len(), area);
            assert_eq!(img.mask.len(), area);
            let palette = crustywad::gfx::Palette([[0; 3]; 256]);
            assert_eq!(pic.to_rgba(&palette).pixels.len(), area * 4);
        }
        if let Ok(pal) = Playpal::parse(data, &options) {
            assert_eq!(pal.palettes().len(), data.len() / 768);
        }
        if let Ok(map) = Colormap::parse(data, &options) {
            let tables = map.tables().len();
            // Post-parse invariant: >= 32 tables; pad adds at most 8192
            // bytes' worth, truncation only removes, so the count is
            // bounded by the input plus the 32-table floor.
            assert!(tables >= 32);
            assert!(tables <= data.len() / 256 + 32);
        }
        if let Ok(flat) = Flat::parse(data, &options) {
            assert!(flat.pixels().len() <= data.len().max(4096));
            assert_eq!(flat.to_indexed().pixels.len(), 4096);
        }
    }
});
