//! Doom 64 PNG decode (#282, ADR-0022 §5): the `doom64-gfx` feature.
#![cfg(feature = "doom64-gfx")]

mod common;

use crustywad::gfx::{Doom64Png, GfxError, GfxWarning};
use crustywad::{Limits, ParseOptions};

#[test]
fn limits_gains_decoded_pixels_cap() {
    let limits = Limits::new();
    assert_eq!(limits.max_decoded_pixels, 1 << 24);
    assert_eq!(
        Limits::new().with_max_decoded_pixels(64).max_decoded_pixels,
        64
    );
}

/// CRC32 (ISO-HDLC), bitwise — for splicing valid custom chunks into
/// encoder output in tests only.
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// Encodes a palette PNG via the `png` crate (tests only — never library
/// code), with optional tRNS; `depth` ∈ {1,2,4,8}; `data` is PACKED rows.
fn build_png(
    width: u32,
    height: u32,
    depth: png::BitDepth,
    plte: &[u8],
    trns: Option<&[u8]>,
    packed_rows: &[u8],
) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(std::io::Cursor::new(&mut out), width, height);
        enc.set_color(png::ColorType::Indexed);
        enc.set_depth(depth);
        enc.set_palette(plte);
        if let Some(t) = trns {
            enc.set_trns(t);
        }
        let mut writer = enc.write_header().expect("test PNG header");
        writer.write_image_data(packed_rows).expect("test PNG data");
    }
    out
}

/// Splices a `grAb` chunk (valid CRC) directly after IHDR (byte 33).
fn splice_grab(png: &mut Vec<u8>, x: i32, y: i32, data_len_override: Option<usize>) {
    let mut data = Vec::new();
    data.extend_from_slice(&x.to_be_bytes());
    data.extend_from_slice(&y.to_be_bytes());
    if let Some(n) = data_len_override {
        data.truncate(n);
        data.resize(n, 0);
    }
    let mut chunk = Vec::new();
    chunk.extend_from_slice(&u32::try_from(data.len()).unwrap().to_be_bytes());
    chunk.extend_from_slice(b"grAb");
    chunk.extend_from_slice(&data);
    let crc_input: Vec<u8> = [b"grAb".as_slice(), &data].concat();
    chunk.extend_from_slice(&crc32(&crc_input).to_be_bytes());
    png.splice(33..33, chunk);
}

#[test]
fn decode_golden_2x2_with_trns_and_grab() {
    // 8bpp 2×2, indices [0,1,2,3]; PLTE red/green/blue/white; tRNS [0,255].
    let plte = [255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255];
    let mut bytes = build_png(
        2,
        2,
        png::BitDepth::Eight,
        &plte,
        Some(&[0, 255]),
        &[0, 1, 2, 3],
    );
    splice_grab(&mut bytes, -7, 12, None);
    let img = Doom64Png::decode(&bytes, &ParseOptions::strict()).unwrap();
    assert!(img.warnings().is_empty());
    assert_eq!((img.width, img.height), (2, 2));
    assert_eq!(img.pixels(), [0, 1, 2, 3]);
    assert_eq!(img.plte().len(), 4);
    assert_eq!(img.plte()[3], [255, 255, 255]);
    assert_eq!(img.trns(), [0, 255]);
    assert_eq!(img.offsets, Some((-7, 12)));
    // Doom 64 palette rows: 4 entries -> no complete 16-color row.
    assert!(img.palette_row(0).is_none());
}

#[test]
fn decode_4bpp_packed_indices() {
    // 4bpp 3×1: packed 0xAB 0xC0 -> [10, 11, 12]; 16-entry PLTE gives row 0.
    let plte: Vec<u8> = (0..48).collect();
    let bytes = build_png(3, 1, png::BitDepth::Four, &plte, None, &[0xAB, 0xC0]);
    let img = Doom64Png::decode(&bytes, &ParseOptions::strict()).unwrap();
    assert_eq!(img.pixels(), [10, 11, 12]);
    assert_eq!(img.offsets, None);
    let row = img.palette_row(0).expect("16 entries = one full row");
    assert_eq!(row[1], [3, 4, 5]);
    assert!(img.palette_row(1).is_none());
}

#[test]
fn non_palette_png_is_a_clean_error_in_both_modes_even_with_trns() {
    // The exact shape that hard-aborts Doom64 EX (ADR-0022 §5): RGB + tRNS.
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(std::io::Cursor::new(&mut out), 1, 1);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_trns(&[0u8, 0, 0, 0, 0, 0][..]);
        let mut w = enc.write_header().unwrap();
        w.write_image_data(&[1, 2, 3]).unwrap();
    }
    for opts in [ParseOptions::strict(), ParseOptions::lenient()] {
        assert!(matches!(
            Doom64Png::decode(&out, &opts).unwrap_err(),
            GfxError::NotPaletteIndexed { color_type: "RGB" }
        ));
    }
}

#[test]
fn non_palette_color_types_report_their_own_name() {
    // Covers the remaining `color_type_name` arms beyond RGB (above):
    // grayscale, grayscale+alpha, and RGBA.
    let cases: [(png::ColorType, &[u8], &str); 3] = [
        (png::ColorType::Grayscale, &[7], "grayscale"),
        (png::ColorType::GrayscaleAlpha, &[7, 255], "grayscale+alpha"),
        (png::ColorType::Rgba, &[1, 2, 3, 255], "RGBA"),
    ];
    for (color_type, pixel, name) in cases {
        let mut out = Vec::new();
        {
            let mut enc = png::Encoder::new(std::io::Cursor::new(&mut out), 1, 1);
            enc.set_color(color_type);
            enc.set_depth(png::BitDepth::Eight);
            let mut w = enc.write_header().unwrap();
            w.write_image_data(pixel).unwrap();
        }
        let err = Doom64Png::decode(&out, &ParseOptions::strict()).unwrap_err();
        assert!(
            matches!(err, GfxError::NotPaletteIndexed { color_type } if color_type == name),
            "{name}: got {err:?}"
        );
    }
}

#[test]
fn garbage_and_truncation_bridge_as_png_decode() {
    for opts in [ParseOptions::strict(), ParseOptions::lenient()] {
        assert!(matches!(
            Doom64Png::decode(b"not a png", &opts).unwrap_err(),
            GfxError::PngDecode { .. }
        ));
    }
    let full = build_png(
        2,
        2,
        png::BitDepth::Eight,
        &[0, 0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 3],
        None,
        &[0, 1, 2, 3],
    );
    let truncated = &full[..full.len() - 8];
    assert!(matches!(
        Doom64Png::decode(truncated, &ParseOptions::lenient()).unwrap_err(),
        GfxError::PngDecode { .. }
    ));
}

#[test]
fn indexed_png_without_plte_hits_the_missing_plte_guard() {
    // Hand-crafted: the encoder refuses palette-less indexed PNGs, but the
    // DECODER accepts them under IDENTITY transformations (no EXPAND), so
    // the missing-PLTE guard is a live path.
    #[allow(clippy::trivially_copy_pass_by_ref)] // matches call sites (`b"IHDR"` etc. are `&[u8; 4]`)
    fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&u32::try_from(data.len()).unwrap().to_be_bytes());
        out.extend_from_slice(kind);
        out.extend_from_slice(data);
        let crc_input: Vec<u8> = [kind.as_slice(), data].concat();
        out.extend_from_slice(&crc32(&crc_input).to_be_bytes());
        out
    }
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&1u32.to_be_bytes()); // width 1
    ihdr.extend_from_slice(&1u32.to_be_bytes()); // height 1
    ihdr.extend_from_slice(&[8, 3, 0, 0, 0]); // depth 8, Indexed, std rest
    // zlib stored block: header 78 01; stored-final 01, len 0002, nlen FDFF;
    // data = [filter 0x00, index 0x07]; adler32(data) = 0x00090008 (BE).
    let idat = [
        0x78, 0x01, 0x01, 0x02, 0x00, 0xFD, 0xFF, 0x00, 0x07, 0x00, 0x09, 0x00, 0x08,
    ];
    let mut png_bytes = b"\x89PNG\r\n\x1a\n".to_vec();
    png_bytes.extend_from_slice(&chunk(b"IHDR", &ihdr));
    png_bytes.extend_from_slice(&chunk(b"IDAT", &idat));
    png_bytes.extend_from_slice(&chunk(b"IEND", &[]));

    for opts in [ParseOptions::strict(), ParseOptions::lenient()] {
        match Doom64Png::decode(&png_bytes, &opts).unwrap_err() {
            GfxError::PngDecode { detail } => assert!(
                detail.contains("missing PLTE"),
                "unexpected detail: {detail}"
            ),
            other => panic!("expected the missing-PLTE guard, got: {other}"),
        }
    }
}

#[test]
fn caps_fire_in_both_modes_before_allocation() {
    let plte = [0u8, 0, 0];
    let bytes = build_png(64, 64, png::BitDepth::Eight, &plte, None, &[0; 64 * 64]);
    let tight = ParseOptions {
        limits: Limits::new().with_max_decoded_pixels(1024),
        ..ParseOptions::strict()
    };
    assert!(matches!(
        Doom64Png::decode(&bytes, &tight).unwrap_err(),
        GfxError::DecodedImageTooLarge {
            width: 64,
            height: 64,
            max_pixels: 1024
        }
    ));
    let tight_lenient = ParseOptions {
        limits: Limits::new().with_max_decoded_pixels(1024),
        ..ParseOptions::lenient()
    };
    assert!(Doom64Png::decode(&bytes, &tight_lenient).is_err());
    assert!(Doom64Png::decode(&bytes, &ParseOptions::strict()).is_ok());
}

#[test]
fn oversized_trns_strict_errors_lenient_truncates() {
    let plte = [1u8, 1, 1, 2, 2, 2]; // 2 entries
    let bytes = build_png(1, 1, png::BitDepth::Eight, &plte, Some(&[9, 9, 9, 9]), &[0]);
    assert!(matches!(
        Doom64Png::decode(&bytes, &ParseOptions::strict()).unwrap_err(),
        GfxError::OversizedTrns {
            trns_len: 4,
            plte_len: 2
        }
    ));
    let img = Doom64Png::decode(&bytes, &ParseOptions::lenient()).unwrap();
    assert_eq!(img.trns(), [9, 9]);
    assert!(matches!(
        img.warnings(),
        [GfxWarning::OversizedTrns {
            trns_len: 4,
            plte_len: 2
        }]
    ));
}

#[test]
fn out_of_range_pixel_index_strict_errors_lenient_aggregates() {
    // 2-entry PLTE, 8bpp pixels [0, 3, 3, 1]: indices 3 have no entry.
    let plte = [1u8, 1, 1, 2, 2, 2];
    let bytes = build_png(2, 2, png::BitDepth::Eight, &plte, None, &[0, 3, 3, 1]);
    assert!(matches!(
        Doom64Png::decode(&bytes, &ParseOptions::strict()).unwrap_err(),
        GfxError::PixelIndexOutOfRange {
            index: 3,
            plte_len: 2
        }
    ));
    let img = Doom64Png::decode(&bytes, &ParseOptions::lenient()).unwrap();
    assert_eq!(img.pixels(), [0, 3, 3, 1]); // kept
    assert!(matches!(
        img.warnings(),
        [GfxWarning::PixelIndexOutOfRange {
            first_index: 3,
            count: 2,
            plte_len: 2
        }]
    ));
}

#[test]
fn malformed_grab_strict_errors_lenient_ignores() {
    let plte = [0u8, 0, 0];
    let mut bytes = build_png(1, 1, png::BitDepth::Eight, &plte, None, &[0]);
    splice_grab(&mut bytes, 1, 2, Some(5)); // 5-byte grAb
    assert!(matches!(
        Doom64Png::decode(&bytes, &ParseOptions::strict()).unwrap_err(),
        GfxError::MalformedGrab { len: 5 }
    ));
    let img = Doom64Png::decode(&bytes, &ParseOptions::lenient()).unwrap();
    assert_eq!(img.offsets, None);
    assert!(matches!(
        img.warnings(),
        [GfxWarning::MalformedGrab { len: 5 }]
    ));
}

#[test]
fn views_apply_the_mask_rule() {
    // PLTE 2 entries; tRNS [0]: index 0 transparent, index 1 opaque.
    let plte = [10u8, 20, 30, 40, 50, 60];
    let bytes = build_png(2, 1, png::BitDepth::Eight, &plte, Some(&[0]), &[0, 1]);
    let img = Doom64Png::decode(&bytes, &ParseOptions::strict()).unwrap();
    let indexed = img.to_indexed();
    assert_eq!((indexed.width, indexed.height), (2, 1));
    assert_eq!(indexed.mask, vec![false, true]); // trns 0 -> hole
    assert_eq!(indexed.pixels[1], 1);
    let rgba = img.to_rgba();
    assert_eq!(&rgba.pixels[0..4], &[0, 0, 0, 0]); // transparent black
    assert_eq!(&rgba.pixels[4..8], &[40, 50, 60, 255]);

    // Lenient out-of-range index renders as a hole.
    let bytes = build_png(2, 1, png::BitDepth::Eight, &plte, None, &[0, 5]);
    let img = Doom64Png::decode(&bytes, &ParseOptions::lenient()).unwrap();
    let indexed = img.to_indexed();
    assert_eq!(indexed.mask, vec![true, false]);
    assert_eq!(&img.to_rgba().pixels[4..8], &[0, 0, 0, 0]);
}

// `zero_dimension_is_a_valid_empty_image` intentionally omitted: the `png`
// crate's `Encoder::write_header` rejects zero-dimension images at encode
// time ("Zero width not allowed" for `build_png(0, 1, ...)`), so there is
// no way to construct a valid zero-dimension PNG byte stream through this
// test-only encoder to drive `Doom64Png::decode`'s success path. Per the
// brief, the spec's 0-dim row holds vacuously for crate-rejected streams;
// any zero/degenerate-dimension byte stream is instead covered by the
// `doom64-gfx` fuzz target's no-panic oracle over arbitrary input.

/// Retail decode gate (#282, ADR-0016's "living pin" pattern — see
/// `retail_classic_graphics_decode_strict_clean` and
/// `retail_texture_sets_compose_strict_clean` in `tests/gfx.rs` for the
/// sibling gates): every non-empty lump in each Doom 64 WAD's `Textures`
/// (`T_START`..`T_END`)
/// and `Sprites` (`S_START`..`S_END`) sections must decode strict-clean,
/// pinning the spike's empirical 503/503 claim (ADR-0022 §5). Gated on
/// `sweep-tests` + `CRUSTYWAD_SWEEP_DIR` (an absolute path to a local WAD
/// collection; never committed — see `tests/fixtures/README.md`).
///
/// Deliberate limit: this sweep only walks the `T_`/`S_` marker sections,
/// per the sections idiom used by the rest of the sweep suite. Doom 64 PNG
/// gfx lumps that live outside those two sections (e.g. HUD/menu/patch
/// graphics referenced individually rather than through a marker range)
/// are NOT covered here — that is a scope choice, not a soundness gap: the
/// fuzz target (`fuzz_doom64_png`) covers `Doom64Png::decode` against
/// arbitrary bytes regardless of section membership.
#[cfg(feature = "sweep-tests")]
#[test]
fn retail_doom64_pngs_decode_strict_clean() {
    use crustywad::SectionKind;

    let Some(dir) = std::env::var_os("CRUSTYWAD_SWEEP_DIR") else {
        eprintln!("skipping: CRUSTYWAD_SWEEP_DIR not set");
        return;
    };
    let dir = std::path::PathBuf::from(dir);
    if !dir.is_absolute() || !dir.is_dir() {
        eprintln!(
            "skipping: CRUSTYWAD_SWEEP_DIR is not an absolute path to a directory: {}",
            dir.display()
        );
        return;
    }

    let mut wads = 0usize;
    let mut textures = 0usize;
    let mut sprites = 0usize;
    let mut sprites_with_grab = 0usize;
    for entry in std::fs::read_dir(&dir).expect("sweep dir reads") {
        let path = entry.expect("dir entry").path();
        if !path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("wad"))
        {
            continue;
        }
        let wad = crustywad::Wad::from_path(&path).expect("retail WAD reads");
        let sections = wad
            .sections_with_options(ParseOptions::lenient())
            .expect("lenient scan never fails");
        // Doom 64 signature: only these WADs carry PNG graphics.
        if sections.of_kind(SectionKind::Textures).next().is_none() {
            continue;
        }
        wads += 1;
        let name = path.file_name().unwrap().to_string_lossy().into_owned();

        for (kind, counter, count_grab) in [
            (SectionKind::Textures, &mut textures, false),
            (SectionKind::Sprites, &mut sprites, true),
        ] {
            for section in sections.of_kind(kind) {
                for i in section.lumps.clone() {
                    let bytes = wad.lump_bytes(i).unwrap();
                    if bytes.is_empty() {
                        continue; // markers
                    }
                    let lump_name = wad.lump(i).unwrap().name().to_owned();
                    let img =
                        Doom64Png::decode(bytes, &ParseOptions::strict()).unwrap_or_else(|e| {
                            panic!("{name}: {kind:?} lump {lump_name} strict: {e}")
                        });
                    assert!(img.warnings().is_empty());
                    *counter += 1;
                    if count_grab && img.offsets.is_some() {
                        sprites_with_grab += 1;
                    }
                }
            }
        }
    }
    assert!(wads > 0, "sweep found no Doom 64 WADs");
    assert!(textures > 0 && sprites > 0, "sweep decoded nothing");
    eprintln!(
        "doom64 png sweep: {wads} WAD(s), {textures} texture(s), {sprites} sprite(s) ({sprites_with_grab} with grAb), all strict-clean"
    );
}
