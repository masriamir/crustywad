#![no_main]
use libfuzzer_sys::fuzz_target;

use crustywad::gfx::{Pnames, TextureX};
use crustywad::{Limits, ParseOptions};

/// Wraps the input as the TEXTURE1 lump of a tiny synthetic WAD whose
/// PNAMES has one real resolvable patch, so set-build + compose paths run.
fn wrap_as_texture1(data: &[u8]) -> Vec<u8> {
    // PNAMES: 1 entry "FUZZPAT".
    let mut pnames = Vec::new();
    pnames.extend_from_slice(&1i32.to_le_bytes());
    pnames.extend_from_slice(b"FUZZPAT\0");
    // FUZZPAT: a valid 1x1 picture (header + 1 offset + one 1-px post).
    let patch: Vec<u8> = vec![
        1, 0, 1, 0, 0, 0, 0, 0, // w=1 h=1 offsets 0,0
        12, 0, 0, 0, // column offset -> 12
        0, 1, 0, 7, 0, 0xFF, // post @0 len1 pad 7 pad, terminator
    ];
    let lumps: [(&str, &[u8]); 3] =
        [("PNAMES", &pnames), ("TEXTURE1", data), ("FUZZPAT", &patch)];
    // Minimal WAD writer (12-byte header + lumps + directory).
    let mut body = Vec::new();
    let mut dir = Vec::new();
    let mut pos = 12i32;
    for (name, bytes) in lumps {
        body.extend_from_slice(bytes);
        dir.extend_from_slice(&pos.to_le_bytes());
        // Never panic in the harness: an oversized fuzz input must produce
        // a (rejectable) malformed WAD, not a spurious crash (the
        // fuzz_doom64_map convention).
        let len = i32::try_from(bytes.len()).unwrap_or(i32::MAX);
        dir.extend_from_slice(&len.to_le_bytes());
        let mut field = [0u8; 8];
        field[..name.len()].copy_from_slice(name.as_bytes());
        dir.extend_from_slice(&field);
        pos = pos.saturating_add(len);
    }
    let mut wad = Vec::new();
    wad.extend_from_slice(b"IWAD");
    wad.extend_from_slice(&3i32.to_le_bytes());
    wad.extend_from_slice(&pos.to_le_bytes());
    wad.extend_from_slice(&body);
    wad.extend_from_slice(&dir);
    wad
}

fuzz_target!(|data: &[u8]| {
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        // Raw-parser oracles (ADR-0016): counts bounded by the input.
        if let Ok(p) = Pnames::parse(data, &options) {
            assert!(p.names().len() <= data.len().saturating_sub(4) / 8);
        }
        if let Ok(tx) = TextureX::parse(data, &options) {
            // Tight bound: every texture needs a 4-byte offset entry after
            // the 4-byte count, and skipped textures only shrink the vec.
            assert!(tx.textures().len() <= data.len().saturating_sub(4) / 4);
            let consumed: usize =
                tx.textures().iter().map(|t| 22 + 10 * t.patches.len()).sum();
            assert!(consumed <= data.len(), "texture budget exceeded the lump");
        }
    }

    // Set-build + compose-all path, lenient, tiny composite cap.
    let wad_bytes = wrap_as_texture1(data);
    if let Ok(wad) = crustywad::Wad::from_bytes(wad_bytes) {
        let options = crustywad::ParseOptions {
            limits: Limits::new().with_max_composite_pixels(1 << 16),
            ..crustywad::ParseOptions::lenient()
        };
        if let Ok(Some(set)) = wad.texture_set_with_options(options) {
            for i in 0..set.textures().len() {
                if let Ok((img, _warnings)) = set.compose(i, &options) {
                    assert!(
                        usize::from(img.width) * usize::from(img.height) <= 1 << 16,
                        "composite exceeded the cap"
                    );
                }
            }
        }
    }
});
