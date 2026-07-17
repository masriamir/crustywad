//! Classic graphics decode (#156, ADR-0022 §3): PLAYPAL, COLORMAP, flats.

mod common;

use crustywad::ParseOptions;
use crustywad::gfx::{Colormap, Flat, GfxError, GfxWarning, Playpal};

#[test]
fn playpal_parses_multiple_palettes_and_indexes_rgb() {
    // Two palettes: entry i of palette 0 is [i, 0, 0]; palette 1 is [0, i, 0].
    let mut bytes = Vec::with_capacity(1536);
    for i in 0..=255u8 {
        bytes.extend_from_slice(&[i, 0, 0]);
    }
    for i in 0..=255u8 {
        bytes.extend_from_slice(&[0, i, 0]);
    }
    let pal = Playpal::parse(&bytes, &ParseOptions::strict()).unwrap();
    assert_eq!(pal.palettes().len(), 2);
    assert!(pal.warnings().is_empty());
    assert_eq!(pal.palettes()[0].rgb(7), [7, 0, 0]);
    assert_eq!(pal.palettes()[1].rgb(255), [0, 255, 0]);
}

#[test]
fn playpal_size_strict_errors_lenient_truncates_and_warns() {
    let bytes = vec![0u8; 800]; // 768 + 32 trailing
    assert!(matches!(
        Playpal::parse(&bytes, &ParseOptions::strict()).unwrap_err(),
        GfxError::PlaypalSize { len: 800 }
    ));
    let pal = Playpal::parse(&bytes, &ParseOptions::lenient()).unwrap();
    assert_eq!(pal.palettes().len(), 1);
    assert!(matches!(
        pal.warnings(),
        [GfxWarning::PlaypalSize { len: 800 }]
    ));
}

#[test]
fn playpal_empty_is_a_strict_error_and_a_lenient_zero_palette_warning() {
    // 0 is a multiple of 768; the "positive multiple" rule rejects it.
    assert!(matches!(
        Playpal::parse(&[], &ParseOptions::strict()).unwrap_err(),
        GfxError::PlaypalSize { len: 0 }
    ));
    let pal = Playpal::parse(&[], &ParseOptions::lenient()).unwrap();
    assert!(pal.palettes().is_empty());
    assert!(matches!(
        pal.warnings(),
        [GfxWarning::PlaypalSize { len: 0 }]
    ));
}

#[test]
fn colormap_exact_strict_pad_and_truncate_lenient() {
    let exact = vec![3u8; 8192];
    let map = Colormap::parse(&exact, &ParseOptions::strict()).unwrap();
    assert_eq!(map.tables()[31][255], 3);
    assert!(map.warnings().is_empty());

    // Short: strict errors; lenient zero-pads to 8192 (the #256 precedent).
    let short = vec![9u8; 300];
    assert!(matches!(
        Colormap::parse(&short, &ParseOptions::strict()).unwrap_err(),
        GfxError::ColormapSize { len: 300 }
    ));
    let map = Colormap::parse(&short, &ParseOptions::lenient()).unwrap();
    assert_eq!(map.tables()[0][255], 9); // byte 255
    assert_eq!(map.tables()[1][43], 9); // byte 299, the last real one
    assert_eq!(map.tables()[1][44], 0); // byte 300: zero-padded from here
    assert_eq!(map.tables()[31][255], 0);
    assert!(matches!(
        map.warnings(),
        [GfxWarning::ColormapSize { len: 300 }]
    ));

    // Long: lenient truncates.
    let long = vec![5u8; 9000];
    let map = Colormap::parse(&long, &ParseOptions::lenient()).unwrap();
    assert_eq!(map.tables()[31][255], 5);
    assert!(matches!(
        map.warnings(),
        [GfxWarning::ColormapSize { len: 9000 }]
    ));
}

#[test]
fn flat_exact_strict_tolerant_lenient() {
    let exact = vec![7u8; 4096];
    let flat = Flat::parse(&exact, &ParseOptions::strict()).unwrap();
    assert_eq!(flat.pixels().len(), 4096);
    assert!(flat.warnings().is_empty());
    assert_eq!((Flat::WIDTH, Flat::HEIGHT), (64, 64));

    let short = vec![1u8; 100];
    assert!(matches!(
        Flat::parse(&short, &ParseOptions::strict()).unwrap_err(),
        GfxError::FlatSize { len: 100 }
    ));
    let flat = Flat::parse(&short, &ParseOptions::lenient()).unwrap();
    assert_eq!(flat.pixels().len(), 100); // kept as-is (ADR-0022 §3)
    assert!(matches!(
        flat.warnings(),
        [GfxWarning::FlatSize { len: 100 }]
    ));
}
