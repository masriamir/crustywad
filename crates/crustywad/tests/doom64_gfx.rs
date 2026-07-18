#![cfg(feature = "doom64-gfx")]
//! Doom 64 PNG decode (#282, ADR-0022 §5): the `doom64-gfx` feature.

mod common;

use crustywad::Limits;

#[test]
fn limits_gains_decoded_pixels_cap() {
    let limits = Limits::new();
    assert_eq!(limits.max_decoded_pixels, 1 << 24);
    assert_eq!(
        Limits::new().with_max_decoded_pixels(64).max_decoded_pixels,
        64
    );
}
