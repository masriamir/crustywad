//! Classic graphics decode (#156, ADR-0022 §3): the picture format,
//! flats, PLAYPAL, and COLORMAP — parser policy rows in both strictness
//! modes, indexed/RGBA view conversions, the `Wad` singleton accessors,
//! and the sweep-gated retail decode gate.

mod common;

use crustywad::Limits;
use crustywad::ParseOptions;
use crustywad::Wad;
use crustywad::gfx::{
    Colormap, Flat, GfxError, GfxWarning, Palette, Picture, Playpal, Pnames, Post, TexturePatchRef,
    TextureX,
};

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
    assert_eq!(map.tables().len(), 32);
    assert_eq!(map.tables()[31][255], 3);
    assert!(map.warnings().is_empty());

    // Retail reality pin: 34 tables (8704 bytes) is strict-clean.
    let retail = vec![6u8; 8704];
    let map = Colormap::parse(&retail, &ParseOptions::strict()).unwrap();
    assert_eq!(map.tables().len(), 34);
    assert_eq!(map.tables()[33][255], 6);
    assert!(map.warnings().is_empty());

    // Short: strict errors; lenient zero-pads to 8192 (the #256 precedent).
    let short = vec![9u8; 300];
    assert!(matches!(
        Colormap::parse(&short, &ParseOptions::strict()).unwrap_err(),
        GfxError::ColormapSize { len: 300 }
    ));
    let map = Colormap::parse(&short, &ParseOptions::lenient()).unwrap();
    assert_eq!(map.tables().len(), 32);
    assert_eq!(map.tables()[0][255], 9); // byte 255
    assert_eq!(map.tables()[1][43], 9); // byte 299, the last real one
    assert_eq!(map.tables()[1][44], 0); // byte 300: zero-padded from here
    assert_eq!(map.tables()[31][255], 0);
    assert!(matches!(
        map.warnings(),
        [GfxWarning::ColormapSize { len: 300 }]
    ));

    // Long, misaligned (9000 % 256 = 40 != 0): strict errors; lenient warns and
    // truncates the trailing partial table to 8960 = 35 whole tables.
    let long = vec![5u8; 9000];
    assert!(matches!(
        Colormap::parse(&long, &ParseOptions::strict()).unwrap_err(),
        GfxError::ColormapSize { len: 9000 }
    ));
    let map = Colormap::parse(&long, &ParseOptions::lenient()).unwrap();
    assert_eq!(map.tables().len(), 35);
    assert_eq!(map.tables()[34][255], 5);
    assert!(matches!(
        map.warnings(),
        [GfxWarning::ColormapSize { len: 9000 }]
    ));

    // Misaligned 8492 (8492 % 256 = 44 != 0): strict errors; lenient
    // truncates to 33 whole tables (8448 bytes).
    let misaligned = vec![4u8; 8492];
    assert!(matches!(
        Colormap::parse(&misaligned, &ParseOptions::strict()).unwrap_err(),
        GfxError::ColormapSize { len: 8492 }
    ));
    let map = Colormap::parse(&misaligned, &ParseOptions::lenient()).unwrap();
    assert_eq!(map.tables().len(), 33);
    assert!(matches!(
        map.warnings(),
        [GfxWarning::ColormapSize { len: 8492 }]
    ));
}

#[test]
fn flat_exact_strict_tolerant_lenient() {
    let exact = vec![7u8; 4096];
    let flat = Flat::parse(&exact, &ParseOptions::strict()).unwrap();
    assert_eq!(flat.pixels().len(), 4096);
    assert!(flat.warnings().is_empty());
    assert_eq!((Flat::WIDTH, Flat::HEIGHT), (64, 64));

    // Heretic's 4160-byte flat: strict-clean, rendered view still 64x64.
    let heretic = vec![8u8; 4160];
    let flat = Flat::parse(&heretic, &ParseOptions::strict()).unwrap();
    assert_eq!(flat.pixels().len(), 4160);
    assert!(flat.warnings().is_empty());
    assert_eq!(flat.to_indexed().pixels.len(), 4096);

    // Hexen's 8192-byte flat: strict-clean likewise.
    let hexen = vec![2u8; 8192];
    let flat = Flat::parse(&hexen, &ParseOptions::strict()).unwrap();
    assert_eq!(flat.pixels().len(), 8192);
    assert!(flat.warnings().is_empty());
    assert_eq!(flat.to_indexed().pixels.len(), 4096);

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

    // 4100 is >= 4096 but not a 64-byte multiple (4100 % 64 = 4): strict
    // errors; lenient keeps the actual bytes and warns.
    let misaligned = vec![3u8; 4100];
    assert!(matches!(
        Flat::parse(&misaligned, &ParseOptions::strict()).unwrap_err(),
        GfxError::FlatSize { len: 4100 }
    ));
    let flat = Flat::parse(&misaligned, &ParseOptions::lenient()).unwrap();
    assert_eq!(flat.pixels().len(), 4100);
    assert!(matches!(
        flat.warnings(),
        [GfxWarning::FlatSize { len: 4100 }]
    ));
}

/// Builds a picture lump. `columns[x]` is that column's post list as
/// (`top_delta`, pixels) pairs; offsets are computed to partition the lump.
fn build_picture(width: i16, height: i16, columns: &[Vec<(u8, Vec<u8>)>]) -> Vec<u8> {
    assert_eq!(columns.len(), usize::try_from(width.max(0)).unwrap());
    let chains: Vec<Vec<u8>> = columns
        .iter()
        .map(|posts| {
            let mut chain = Vec::new();
            for (top, px) in posts {
                chain.push(*top);
                chain.push(u8::try_from(px.len()).unwrap());
                chain.push(0); // leading pad
                chain.extend_from_slice(px);
                chain.push(0); // trailing pad
            }
            chain.push(0xFF);
            chain
        })
        .collect();
    let mut out = Vec::new();
    out.extend_from_slice(&width.to_le_bytes());
    out.extend_from_slice(&height.to_le_bytes());
    out.extend_from_slice(&0i16.to_le_bytes()); // leftoffset
    out.extend_from_slice(&0i16.to_le_bytes()); // topoffset
    let mut offset = 8 + 4 * chains.len();
    for chain in &chains {
        out.extend_from_slice(&i32::try_from(offset).unwrap().to_le_bytes());
        offset += chain.len();
    }
    for chain in &chains {
        out.extend_from_slice(chain);
    }
    out
}

#[test]
fn picture_parses_structure_faithfully() {
    // 2×8: col 0 = one post at row 1 [5,6]; col 1 = two posts (row 0 [7],
    // row 3 [8]) — exercises multi-post chains and empty coverage.
    let bytes = build_picture(
        2,
        8,
        &[vec![(1, vec![5, 6])], vec![(0, vec![7]), (3, vec![8])]],
    );
    let pic = Picture::parse(&bytes, &ParseOptions::strict()).unwrap();
    assert!(pic.warnings().is_empty());
    assert_eq!((pic.width, pic.height), (2, 8));
    assert_eq!((pic.left_offset, pic.top_offset), (0, 0));
    assert_eq!(pic.columns().len(), 2);
    assert_eq!(
        pic.columns()[0].posts,
        vec![Post {
            top_delta: 1,
            pixels: vec![5, 6]
        }]
    );
    assert_eq!(pic.columns()[1].posts.len(), 2);
    assert_eq!(
        pic.columns()[1].posts[1],
        Post {
            top_delta: 3,
            pixels: vec![8]
        }
    );
}

#[test]
fn picture_empty_column_and_zero_size_are_valid() {
    let bytes = build_picture(1, 4, &[vec![]]); // offset points straight at 0xFF
    let pic = Picture::parse(&bytes, &ParseOptions::strict()).unwrap();
    assert!(pic.columns()[0].posts.is_empty());
}

#[test]
fn picture_header_truncation_fails_both_modes() {
    for opts in [ParseOptions::strict(), ParseOptions::lenient()] {
        assert!(matches!(
            Picture::parse(&[1, 0, 1, 0, 0, 0], &opts).unwrap_err(),
            GfxError::TruncatedPicture { len: 6, needed: 8 }
        ));
    }
}

#[test]
fn picture_offset_table_truncation_strict_errors_lenient_clamps_width() {
    // Declared width 4 but only one offset present (12 bytes total).
    let mut bytes = build_picture(1, 4, &[vec![]]);
    bytes[0] = 4; // forge width = 4; lump has offsets for width 1 only
    assert!(matches!(
        Picture::parse(&bytes, &ParseOptions::strict()).unwrap_err(),
        GfxError::TruncatedPicture { needed: 24, .. }
    ));
    let pic = Picture::parse(&bytes, &ParseOptions::lenient()).unwrap();
    assert_eq!(pic.width, 1);
    assert!(matches!(
        pic.warnings(),
        [GfxWarning::TruncatedPicture { needed: 24, .. }]
    ));
}

#[test]
fn picture_negative_dimensions_strict_error_lenient_clamp() {
    let mut bytes = build_picture(1, 4, &[vec![]]);
    bytes[2] = 0xFF;
    bytes[3] = 0xFF; // height = -1
    assert!(matches!(
        Picture::parse(&bytes, &ParseOptions::strict()).unwrap_err(),
        GfxError::NegativeDimension {
            field: "height",
            value: -1
        }
    ));
    let pic = Picture::parse(&bytes, &ParseOptions::lenient()).unwrap();
    assert_eq!(pic.height, 0);
    // The column's post at any row now exceeds height 0 — but this fixture
    // has no posts, so only the dimension warning fires.
    assert!(matches!(
        pic.warnings(),
        [GfxWarning::NegativeDimension {
            field: "height",
            value: -1
        }]
    ));
}

#[test]
fn picture_bad_column_offset_strict_errors_lenient_empties() {
    let mut bytes = build_picture(1, 4, &[vec![(0, vec![1])]]);
    let len = bytes.len();
    bytes[8..12].copy_from_slice(&i32::try_from(len + 10).unwrap().to_le_bytes());
    assert!(matches!(
        Picture::parse(&bytes, &ParseOptions::strict()).unwrap_err(),
        GfxError::ColumnOffsetOutOfBounds { column: 0, .. }
    ));
    let pic = Picture::parse(&bytes, &ParseOptions::lenient()).unwrap();
    assert!(pic.columns()[0].posts.is_empty());
    assert!(matches!(
        pic.warnings(),
        [GfxWarning::ColumnOffsetOutOfBounds { column: 0, .. }]
    ));

    // Negative offset takes the same row.
    bytes[8..12].copy_from_slice(&(-4i32).to_le_bytes());
    assert!(matches!(
        Picture::parse(&bytes, &ParseOptions::strict()).unwrap_err(),
        GfxError::ColumnOffsetOutOfBounds { column: 0, .. }
    ));
    let pic = Picture::parse(&bytes, &ParseOptions::lenient()).unwrap();
    assert!(pic.columns()[0].posts.is_empty());
    assert!(matches!(
        pic.warnings(),
        [GfxWarning::ColumnOffsetOutOfBounds { column: 0, .. }]
    ));
}

#[test]
fn picture_post_header_truncated_mid_length_byte() {
    // A post's top_delta byte is present but the lump ends before its
    // length byte (the `pos + 2 > bytes.len()` guard, distinct from the
    // whole-column EOF check).
    let mut bytes = build_picture(1, 4, &[vec![(0, vec![1])]]);
    bytes.truncate(13); // keep only the top_delta byte (0) at offset 12
    assert!(matches!(
        Picture::parse(&bytes, &ParseOptions::strict()).unwrap_err(),
        GfxError::UnterminatedColumn { column: 0 }
    ));
    let pic = Picture::parse(&bytes, &ParseOptions::lenient()).unwrap();
    assert!(pic.columns()[0].posts.is_empty());
    assert!(matches!(
        pic.warnings(),
        [GfxWarning::UnterminatedColumn { column: 0 }]
    ));
}

#[test]
fn picture_post_body_truncated_mid_pixel_data() {
    // The post's top_delta and length bytes are present but the lump ends
    // before the declared pixel/pad bytes are all readable (the
    // `pos + full > bytes.len()` guard, distinct from both the whole-column
    // EOF check and the header-truncation check above).
    let mut bytes = build_picture(1, 4, &[vec![(0, vec![1, 2])]]);
    bytes.truncate(16); // cuts the second pixel byte, trailing pad, and 0xFF
    assert!(matches!(
        Picture::parse(&bytes, &ParseOptions::strict()).unwrap_err(),
        GfxError::UnterminatedColumn { column: 0 }
    ));
    let pic = Picture::parse(&bytes, &ParseOptions::lenient()).unwrap();
    assert!(pic.columns()[0].posts.is_empty());
    assert!(matches!(
        pic.warnings(),
        [GfxWarning::UnterminatedColumn { column: 0 }]
    ));
}

#[test]
fn picture_unterminated_column_strict_errors_lenient_keeps_read_posts() {
    // One full post, then cut the lump before the 0xFF terminator.
    let mut bytes = build_picture(1, 4, &[vec![(0, vec![9])]]);
    bytes.pop(); // remove the 0xFF
    assert!(matches!(
        Picture::parse(&bytes, &ParseOptions::strict()).unwrap_err(),
        GfxError::UnterminatedColumn { column: 0 }
    ));
    let pic = Picture::parse(&bytes, &ParseOptions::lenient()).unwrap();
    assert_eq!(pic.columns()[0].posts.len(), 1); // the fully-read post survives
    assert!(matches!(
        pic.warnings(),
        [GfxWarning::UnterminatedColumn { column: 0 }]
    ));
}

#[test]
fn picture_post_exceeding_height_strict_errors_lenient_clips() {
    // Height 3; post at row 1 with 4 pixels reaches row 4.
    let bytes = build_picture(1, 3, &[vec![(1, vec![1, 2, 3, 4])]]);
    assert!(matches!(
        Picture::parse(&bytes, &ParseOptions::strict()).unwrap_err(),
        GfxError::PostOutOfBounds {
            column: 0,
            top_delta: 1,
            length: 4,
            height: 3
        }
    ));
    let pic = Picture::parse(&bytes, &ParseOptions::lenient()).unwrap();
    assert_eq!(pic.columns()[0].posts[0].pixels, vec![1, 2]); // rows 1..3 kept
    assert!(matches!(
        pic.warnings(),
        [GfxWarning::PostOutOfBounds { column: 0, .. }]
    ));

    // Entirely out of bounds (top_delta >= height): dropped, not kept empty.
    let bytes = build_picture(1, 2, &[vec![(2, vec![1])]]);
    let pic = Picture::parse(&bytes, &ParseOptions::lenient()).unwrap();
    assert!(pic.columns()[0].posts.is_empty());
}

#[test]
fn picture_aliased_offsets_hit_the_consumed_bytes_budget() {
    // One real column chain; forge MANY offsets all pointing at it. The
    // consumed-bytes budget (per-post 4 + length, cumulative ≤ lump len)
    // must trip: strict errors, lenient stops with empty tail columns.
    let real = build_picture(1, 200, &[vec![(0, vec![42; 100])]]);
    let width = 30i16;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&width.to_le_bytes());
    bytes.extend_from_slice(&200i16.to_le_bytes());
    bytes.extend_from_slice(&[0; 4]); // offsets fields
    let chain_at = 8 + 4 * usize::try_from(width).unwrap();
    for _ in 0..width {
        bytes.extend_from_slice(&i32::try_from(chain_at).unwrap().to_le_bytes());
    }
    bytes.extend_from_slice(&real[12..]); // the single real chain
    let len = bytes.len();
    // Each aliased walk consumes 104 bytes of budget; 30 × 104 > len.
    assert!(matches!(
        Picture::parse(&bytes, &ParseOptions::strict()).unwrap_err(),
        GfxError::ExcessivePostData { .. }
    ));
    let pic = Picture::parse(&bytes, &ParseOptions::lenient()).unwrap();
    let decoded: usize = pic
        .columns()
        .iter()
        .flat_map(|c| c.posts.iter())
        .map(|p| 4 + p.pixels.len())
        .sum();
    assert!(
        decoded <= len,
        "budget must bound decoded bytes to the lump length"
    );
    assert!(pic.columns().last().unwrap().posts.is_empty());
    assert!(
        pic.warnings()
            .iter()
            .any(|w| matches!(w, GfxWarning::ExcessivePostData { .. }))
    );
}

fn gray_palette() -> Palette {
    let mut entries = [[0u8; 3]; 256];
    for (i, entry) in entries.iter_mut().enumerate() {
        let v = u8::try_from(i).unwrap();
        *entry = [v, v, v];
    }
    Palette(entries)
}

#[test]
fn picture_golden_indexed_and_rgba() {
    // The Task 2 fixture: 2×8, col 0 post@1 [5,6]; col 1 posts @0 [7], @3 [8].
    let bytes = build_picture(
        2,
        8,
        &[vec![(1, vec![5, 6])], vec![(0, vec![7]), (3, vec![8])]],
    );
    let pic = Picture::parse(&bytes, &ParseOptions::strict()).unwrap();
    let img = pic.to_indexed();
    assert_eq!((img.width, img.height), (2, 8));
    assert_eq!(img.pixels.len(), 16);
    assert_eq!(img.mask.len(), 16);
    // Row-major index = y * width + x.
    let expect_covered = [(1usize, 0usize, 5u8), (2, 0, 6), (0, 1, 7), (3, 1, 8)];
    for (y, x, v) in expect_covered {
        assert_eq!(img.pixels[y * 2 + x], v, "pixel at ({x},{y})");
        assert!(img.mask[y * 2 + x], "mask at ({x},{y})");
    }
    assert_eq!(img.mask.iter().filter(|m| **m).count(), 4);

    let rgba = img.to_rgba(&gray_palette());
    assert_eq!(rgba.pixels.len(), 64);
    // Covered pixel (0,1): index 5 → gray 5, alpha 255.
    assert_eq!(&rgba.pixels[8..12], &[5, 5, 5, 255]);
    // Uncovered pixel (0,0): alpha 0.
    assert_eq!(rgba.pixels[3], 0);
    // Picture::to_rgba is the same composition.
    assert_eq!(pic.to_rgba(&gray_palette()).pixels, rgba.pixels);
}

#[test]
fn overlapping_posts_later_wins() {
    // Two posts covering row 0: chain order draws the later one over.
    let bytes = build_picture(1, 2, &[vec![(0, vec![1]), (0, vec![2])]]);
    let pic = Picture::parse(&bytes, &ParseOptions::strict()).unwrap();
    assert_eq!(pic.to_indexed().pixels[0], 2);
}

#[test]
fn flat_views_pad_short_lenient_flats() {
    let flat = Flat::parse(&vec![9u8; 4096], &ParseOptions::strict()).unwrap();
    let img = flat.to_indexed();
    assert_eq!((img.width, img.height), (64, 64));
    assert!(img.mask.iter().all(|m| *m));
    assert_eq!(flat.to_rgba(&gray_palette()).pixels.len(), 64 * 64 * 4);

    let short = Flat::parse(&[9u8; 100], &ParseOptions::lenient()).unwrap();
    let img = short.to_indexed();
    assert_eq!(img.pixels.len(), 4096); // zero-padded at conversion
    assert_eq!(img.pixels[99], 9);
    assert_eq!(img.pixels[100], 0);
}

#[test]
fn wad_playpal_and_colormap_singletons() {
    let playpal_bytes = vec![0u8; 768];
    let colormap_bytes = vec![0u8; 8192];
    let wad = Wad::from_bytes(common::build_wad(
        *b"IWAD",
        &[("PLAYPAL", &playpal_bytes), ("COLORMAP", &colormap_bytes)],
    ))
    .unwrap();
    assert_eq!(wad.playpal().unwrap().unwrap().palettes().len(), 1);
    assert!(wad.colormap().unwrap().is_some());

    let bare = Wad::from_bytes(common::build_wad(*b"PWAD", &[("THINGS", &[])])).unwrap();
    assert!(bare.playpal().unwrap().is_none());
    assert!(bare.colormap().unwrap().is_none());

    // Strict surfaces parse errors; lenient recovers.
    let bad = Wad::from_bytes(common::build_wad(*b"IWAD", &[("PLAYPAL", &[0u8; 5])])).unwrap();
    assert!(bad.playpal().is_err());
    assert!(
        bad.playpal_with_options(ParseOptions::lenient())
            .unwrap()
            .unwrap()
            .palettes()
            .is_empty()
    );

    // Duplicate lumps: the crate's documented FIRST-match contract wins
    // (vanilla's backward scan would take the last — noted divergence).
    let one = vec![0u8; 768];
    let two = vec![0u8; 1536];
    let dup = Wad::from_bytes(common::build_wad(
        *b"IWAD",
        &[("PLAYPAL", &one), ("PLAYPAL", &two)],
    ))
    .unwrap();
    assert_eq!(dup.playpal().unwrap().unwrap().palettes().len(), 1);
}

#[test]
fn flat_lenient_truncates_a_long_flat_at_conversion() {
    // Task-3 review gap: the > 4096-byte lenient truncate path had no direct
    // test. 5000 is misaligned (5000 % 64 = 8 != 0), so it still warns even
    // under the corrected 64-byte-multiple rule. Parse keeps the actual
    // bytes (ADR-0022 §3: "proceeds with what is present"); truncation
    // happens only at `to_indexed` conversion.
    let long = vec![3u8; 5000];
    let flat = Flat::parse(&long, &ParseOptions::lenient()).unwrap();
    assert_eq!(flat.pixels().len(), 5000);
    assert!(matches!(
        flat.warnings(),
        [GfxWarning::FlatSize { len: 5000 }]
    ));
    assert_eq!(flat.to_indexed().pixels.len(), 4096);
}

#[cfg(feature = "sweep-tests")]
#[test]
fn retail_classic_graphics_decode_strict_clean() {
    use crustywad::SectionKind; // crate-root re-export (tests/sections.rs idiom)

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
    let mut pictures = 0usize;
    let mut flats = 0usize;
    let mut skipped_skies = 0usize;
    for entry in std::fs::read_dir(&dir).expect("sweep dir reads") {
        let path = entry.expect("dir entry").path();
        let is_wad = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("wad"));
        if !is_wad {
            continue;
        }
        let wad = crustywad::Wad::from_path(&path).expect("retail WAD reads");
        // Lenient enumeration: marker anomalies (SVE.wad's bare top-level
        // P3_START, #292) stay in the sections domain; graphics decode below
        // stays strict.
        let sections = wad
            .sections_with_options(ParseOptions::lenient())
            .expect("lenient scan never fails");
        // Doom 64 signature: a Textures section — its graphics are PNGs
        // (#282), not this format.
        if sections.of_kind(SectionKind::Textures).next().is_some() {
            eprintln!("skipping Doom 64-format WAD: {}", path.display());
            continue;
        }
        wads += 1;
        let name = path.file_name().unwrap().to_string_lossy().into_owned();

        if let Some(pal) = wad
            .playpal()
            .unwrap_or_else(|e| panic!("{name}: PLAYPAL strict: {e}"))
        {
            assert!(!pal.palettes().is_empty(), "{name}: empty PLAYPAL");
        }
        let _ = wad
            .colormap()
            .unwrap_or_else(|e| panic!("{name}: COLORMAP strict: {e}"));

        for kind in [SectionKind::Sprites, SectionKind::Patches] {
            for section in sections.of_kind(kind) {
                for i in section.lumps.clone() {
                    let bytes = wad.lump_bytes(i).unwrap();
                    if bytes.is_empty() {
                        continue; // nested sub-namespace markers
                    }
                    let lump_name = wad.lump(i).unwrap().name().to_owned();
                    let pic = Picture::parse(bytes, &ParseOptions::strict()).unwrap_or_else(|e| {
                        panic!("{name}: {kind:?} lump {lump_name} strict: {e}")
                    });
                    assert!(pic.warnings().is_empty());
                    pictures += 1;
                }
            }
        }
        for section in sections.of_kind(SectionKind::Flats) {
            for i in section.lumps.clone() {
                let bytes = wad.lump_bytes(i).unwrap();
                if bytes.is_empty() {
                    continue;
                }
                let lump_name = wad.lump(i).unwrap().name().to_owned();
                if lump_name.starts_with("F_SKY") {
                    // Engines special-case sky flats by NAME and never read
                    // their pixels (retail placeholders are 4 bytes) — the
                    // skip is engine-faithful, not a data exemption.
                    skipped_skies += 1;
                    continue;
                }
                Flat::parse(bytes, &ParseOptions::strict())
                    .unwrap_or_else(|e| panic!("{name}: flat {lump_name} strict: {e}"));
                flats += 1;
            }
        }
    }
    assert!(wads > 0, "sweep found no classic WADs");
    assert!(pictures > 0 && flats > 0, "sweep decoded nothing");
    eprintln!(
        "gfx sweep: {wads} WAD(s), {pictures} picture(s), {flats} flat(s), \
         {skipped_skies} sky flat(s) skipped, all strict-clean"
    );
}

#[cfg(feature = "sweep-tests")]
#[test]
#[allow(clippy::too_many_lines)]
fn retail_texture_sets_compose_strict_clean() {
    use crustywad::{SectionKind, WadKind};

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

    let mut sets = 0usize;
    let mut composed = 0usize;
    let mut lenient_pwads = 0usize;
    let mut no_textures = 0usize;
    let mut gate_contract_iwads = 0usize;
    for entry in std::fs::read_dir(&dir).expect("sweep dir reads") {
        let path = entry.expect("dir entry").path();
        if !path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("wad"))
        {
            continue;
        }
        let wad = crustywad::Wad::from_path(&path).expect("retail WAD reads");
        // Doom 64 has no PNAMES/TEXTUREx (ADR-0022 §4) — data-driven skip.
        let sections = wad
            .sections_with_options(ParseOptions::lenient())
            .expect("lenient scan never fails");
        if sections.of_kind(SectionKind::Textures).next().is_some() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().into_owned();

        match wad.texture_set() {
            Ok(None) => no_textures += 1,
            Ok(Some(set)) => {
                assert!(
                    set.warnings().is_empty(),
                    "{name}: strict build must be clean"
                );
                sets += 1;
                for i in 0..set.textures().len() {
                    let (_, warnings) =
                        set.compose(i, &ParseOptions::strict()).unwrap_or_else(|e| {
                            panic!(
                                "{name}: texture {} ({}) strict compose: {e}",
                                i,
                                set.textures()[i].name
                            )
                        });
                    assert!(warnings.is_empty());
                    composed += 1;
                }
            }
            Err(crustywad::gfx::GfxError::NegativePatchCount { texture, count }) => {
                // Known-anomaly gate contract (#269 precedent; adjudicated
                // 2026-07-17): exactly one retail IWAD — Strife — ships four
                // TEXTUREx records with genuinely negative on-disk patchcount
                // fields (SIGN12/SIGN13 -96, WALTEK12 -18, STAIR07 -15).
                // Strict correctly refuses (the field is malformed; the
                // engine's signed patch loop simply never iterates, silently
                // yielding zero-patch textures). Keyed by error identity,
                // not filename: the exact first offender is pinned, and the
                // count of such IWADs is asserted to be exactly one below.
                assert_eq!(
                    wad.kind(),
                    WadKind::Iwad,
                    "{name}: gate contract expects an IWAD"
                );
                assert_eq!(
                    (texture, count),
                    (162, -96),
                    "{name}: unexpected negative-patchcount offender"
                );
                gate_contract_iwads += 1;
                let set = wad
                    .texture_set_with_options(ParseOptions::lenient())
                    .expect("lenient build never fails")
                    .expect("TEXTUREx present");
                let neg_warns = set
                    .warnings()
                    .iter()
                    .filter(|w| matches!(w, crustywad::gfx::GfxWarning::NegativePatchCount { .. }))
                    .count();
                assert_eq!(
                    neg_warns, 4,
                    "{name}: expected exactly 4 negative-patchcount warnings"
                );
                for i in 0..set.textures().len() {
                    let _ = set
                        .compose(i, &ParseOptions::lenient())
                        .unwrap_or_else(|e| panic!("{name}: lenient compose {i}: {e}"));
                }
            }
            Err(e) => {
                // PWADs referencing base-IWAD patches cannot resolve without
                // a merge model (spec's PWAD reality note): rerun leniently,
                // composes must not panic. Anything else = STOP.
                let is_unresolved =
                    matches!(e, crustywad::gfx::GfxError::UnresolvedPatchName { .. });
                assert!(
                    is_unresolved && wad.kind() == WadKind::Pwad,
                    "{name}: unexpected strict set-build failure: {e}"
                );
                lenient_pwads += 1;
                let set = wad
                    .texture_set_with_options(ParseOptions::lenient())
                    .expect("lenient build never fails")
                    .expect("TEXTUREx present");
                for i in 0..set.textures().len() {
                    let _ = set
                        .compose(i, &ParseOptions::lenient())
                        .unwrap_or_else(|e| panic!("{name}: lenient compose {i}: {e}"));
                }
            }
        }
    }
    assert!(sets > 0, "sweep composed no IWAD texture sets");
    assert_eq!(
        gate_contract_iwads, 1,
        "expected exactly one gate-contract IWAD (Strife) in the collection"
    );
    eprintln!(
        "texture sweep: {sets} strict set(s), {composed} texture(s) composed strict-clean, {lenient_pwads} PWAD(s) lenient, {no_textures} WAD(s) without TEXTUREx, {gate_contract_iwads} gate-contract IWAD(s)"
    );
}

/// Builds a PNAMES lump from names (8-byte NUL-padded each).
fn build_pnames(names: &[&str]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&i32::try_from(names.len()).unwrap().to_le_bytes());
    for name in names {
        let mut field = [0u8; 8];
        field[..name.len()].copy_from_slice(name.as_bytes());
        out.extend_from_slice(&field);
    }
    out
}

/// One texture def as (name, width, height, patches: &[(ox, oy, `patch_idx`)]).
type DefSpec<'a> = (&'a str, i16, i16, &'a [(i16, i16, i16)]);

/// Builds a `TEXTUREx` lump; offsets computed to partition the lump.
fn build_texturex(defs: &[DefSpec<'_>]) -> Vec<u8> {
    let mut bodies: Vec<Vec<u8>> = Vec::new();
    for (name, w, h, patches) in defs {
        let mut b = Vec::new();
        let mut field = [0u8; 8];
        field[..name.len()].copy_from_slice(name.as_bytes());
        b.extend_from_slice(&field);
        b.extend_from_slice(&0i32.to_le_bytes()); // masked (dead)
        b.extend_from_slice(&w.to_le_bytes());
        b.extend_from_slice(&h.to_le_bytes());
        b.extend_from_slice(&0i32.to_le_bytes()); // columndirectory (dead)
        b.extend_from_slice(&i16::try_from(patches.len()).unwrap().to_le_bytes());
        for (ox, oy, idx) in *patches {
            b.extend_from_slice(&ox.to_le_bytes());
            b.extend_from_slice(&oy.to_le_bytes());
            b.extend_from_slice(&idx.to_le_bytes());
            b.extend_from_slice(&0i16.to_le_bytes()); // stepdir (dead)
            b.extend_from_slice(&0i16.to_le_bytes()); // colormap (dead)
        }
        bodies.push(b);
    }
    let mut out = Vec::new();
    out.extend_from_slice(&i32::try_from(defs.len()).unwrap().to_le_bytes());
    let mut offset = 4 + 4 * defs.len();
    for body in &bodies {
        out.extend_from_slice(&i32::try_from(offset).unwrap().to_le_bytes());
        offset += body.len();
    }
    for body in &bodies {
        out.extend_from_slice(body);
    }
    out
}

#[test]
fn limits_gains_composite_cap_with_const_setters() {
    let limits = Limits::new();
    assert_eq!(limits.max_depth, 64);
    assert_eq!(limits.max_composite_pixels, 1 << 24);
    let tuned = Limits::new()
        .with_max_depth(8)
        .with_max_composite_pixels(1024);
    assert_eq!((tuned.max_depth, tuned.max_composite_pixels), (8, 1024));
}

#[test]
fn pnames_parses_and_trims() {
    let bytes = build_pnames(&["WALL00", "DOOR2"]);
    let pnames = Pnames::parse(&bytes, &ParseOptions::strict()).unwrap();
    assert_eq!(pnames.names(), ["WALL00".to_owned(), "DOOR2".to_owned()]);
    assert!(pnames.warnings().is_empty());
}

#[test]
fn pnames_policy_rows() {
    use crustywad::gfx::{GfxError, GfxWarning};
    // len < 4
    assert!(matches!(
        Pnames::parse(&[1, 0], &ParseOptions::strict()).unwrap_err(),
        GfxError::TruncatedPnames { len: 2 }
    ));
    let p = Pnames::parse(&[1, 0], &ParseOptions::lenient()).unwrap();
    assert!(p.names().is_empty());
    assert!(matches!(
        p.warnings(),
        [GfxWarning::TruncatedPnames { len: 2 }]
    ));
    // negative count
    let neg = (-5i32).to_le_bytes().to_vec();
    assert!(matches!(
        Pnames::parse(&neg, &ParseOptions::strict()).unwrap_err(),
        GfxError::NegativePnamesCount { count: -5 }
    ));
    assert!(
        Pnames::parse(&neg, &ParseOptions::lenient())
            .unwrap()
            .names()
            .is_empty()
    );
    // count exceeds lump: claims 3 names, carries 1
    let mut short = build_pnames(&["ONLY1"]);
    short[0] = 3;
    assert!(matches!(
        Pnames::parse(&short, &ParseOptions::strict()).unwrap_err(),
        GfxError::PnamesCountExceedsLump { count: 3, .. }
    ));
    let p = Pnames::parse(&short, &ParseOptions::lenient()).unwrap();
    assert_eq!(p.names(), ["ONLY1".to_owned()]);
    assert!(matches!(
        p.warnings(),
        [GfxWarning::PnamesCountExceedsLump { count: 3, .. }]
    ));
}

#[test]
fn texturex_parses_defs_faithfully_including_dead_fields() {
    let bytes = build_texturex(&[
        ("TEX0", 64, 128, &[(0, 0, 0), (32, -4, 1)]),
        ("TEX1", 8, 8, &[]),
    ]);
    let tx = TextureX::parse(&bytes, &ParseOptions::strict()).unwrap();
    assert!(tx.warnings().is_empty());
    assert_eq!(tx.textures().len(), 2);
    let t0 = &tx.textures()[0];
    assert_eq!((t0.name.as_str(), t0.width, t0.height), ("TEX0", 64, 128));
    assert_eq!((t0.masked, t0.column_directory), (0, 0)); // dead, preserved
    assert_eq!(
        t0.patches[1],
        TexturePatchRef {
            origin_x: 32,
            origin_y: -4,
            patch: 1,
            step_dir: 0,
            colormap: 0
        }
    );
    assert!(tx.textures()[1].patches.is_empty());
}

#[test]
fn texturex_header_policy_rows() {
    use crustywad::gfx::{GfxError, GfxWarning};
    // len < 4
    assert!(matches!(
        TextureX::parse(&[9, 9], &ParseOptions::strict()).unwrap_err(),
        GfxError::TruncatedTextureX { len: 2, needed: 4 }
    ));
    assert!(
        TextureX::parse(&[9, 9], &ParseOptions::lenient())
            .unwrap()
            .textures()
            .is_empty()
    );
    // Negative count
    let neg = (-2i32).to_le_bytes().to_vec();
    assert!(matches!(
        TextureX::parse(&neg, &ParseOptions::strict()).unwrap_err(),
        GfxError::NegativeTextureCount { count: -2 }
    ));
    assert!(
        TextureX::parse(&neg, &ParseOptions::lenient())
            .unwrap()
            .textures()
            .is_empty()
    );
    // Offset table truncated: claims 10 textures (needed = 4 + 40 = 44)
    // on a 30-byte one-texture lump.
    let mut short = build_texturex(&[("TEX0", 8, 8, &[])]);
    short[0] = 10;
    assert!(matches!(
        TextureX::parse(&short, &ParseOptions::strict()).unwrap_err(),
        GfxError::TruncatedTextureX { needed: 44, .. }
    ));
    let tx = TextureX::parse(&short, &ParseOptions::lenient()).unwrap();
    assert!(matches!(
        tx.warnings()[0],
        GfxWarning::TruncatedTextureX { .. }
    ));
}

#[test]
fn texturex_policy_rows() {
    use crustywad::gfx::{GfxError, GfxWarning};
    // Offset past the lump
    let mut bytes = build_texturex(&[("TEX0", 8, 8, &[])]);
    let len = bytes.len();
    bytes[4..8].copy_from_slice(&i32::try_from(len + 40).unwrap().to_le_bytes());
    assert!(matches!(
        TextureX::parse(&bytes, &ParseOptions::strict()).unwrap_err(),
        GfxError::TextureOffsetOutOfBounds { texture: 0, .. }
    ));
    let tx = TextureX::parse(&bytes, &ParseOptions::lenient()).unwrap();
    assert!(tx.textures().is_empty()); // skipped
    assert!(matches!(
        tx.warnings(),
        [GfxWarning::TextureOffsetOutOfBounds { texture: 0, .. }]
    ));

    // Full extent past the lump: header fits, patch refs cut off.
    let mut bytes = build_texturex(&[("TEX0", 8, 8, &[(0, 0, 0), (1, 1, 1)])]);
    bytes.truncate(bytes.len() - 10); // drop the second 10-byte ref
    assert!(matches!(
        TextureX::parse(&bytes, &ParseOptions::strict()).unwrap_err(),
        GfxError::TextureExtentOutOfBounds { texture: 0, .. }
    ));
    let tx = TextureX::parse(&bytes, &ParseOptions::lenient()).unwrap();
    assert_eq!(tx.textures()[0].patches.len(), 1); // clamped to in-bounds refs
    assert!(matches!(
        tx.warnings(),
        [GfxWarning::TextureExtentOutOfBounds { texture: 0, .. }]
    ));

    // Negative patch count
    let mut bytes = build_texturex(&[("TEX0", 8, 8, &[])]);
    let pc_at = bytes.len() - 2; // patchcount is the def's last field
    bytes[pc_at..].copy_from_slice(&(-1i16).to_le_bytes());
    assert!(matches!(
        TextureX::parse(&bytes, &ParseOptions::strict()).unwrap_err(),
        GfxError::NegativePatchCount {
            texture: 0,
            count: -1
        }
    ));
    let tx = TextureX::parse(&bytes, &ParseOptions::lenient()).unwrap();
    assert!(tx.textures()[0].patches.is_empty());

    // Aliased offsets exhaust the cumulative budget.
    let one = build_texturex(&[("TEX0", 8, 8, &[(0, 0, 0)])]);
    let body_at = 4 + 4; // count + one offset
    let body = &one[body_at..]; // 32 bytes: header 22 + one ref 10
    let count = 40i32;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&count.to_le_bytes());
    let chain_at = 4 + 4 * usize::try_from(count).unwrap();
    for _ in 0..count {
        bytes.extend_from_slice(&i32::try_from(chain_at).unwrap().to_le_bytes());
    }
    bytes.extend_from_slice(body);
    // Each aliased texture consumes 22 + 10 = 32 budget bytes; 40 × 32 = 1280
    // > lump len (4 + 160 + 32 = 196): strict trips, lenient stops + warns.
    assert!(matches!(
        TextureX::parse(&bytes, &ParseOptions::strict()).unwrap_err(),
        GfxError::ExcessiveTextureData { .. }
    ));
    let tx = TextureX::parse(&bytes, &ParseOptions::lenient()).unwrap();
    let consumed: usize = tx
        .textures()
        .iter()
        .map(|t| 22 + 10 * t.patches.len())
        .sum();
    assert!(consumed <= bytes.len());
    assert!(
        tx.warnings()
            .iter()
            .any(|w| matches!(w, GfxWarning::ExcessiveTextureData { .. }))
    );
}

/// A WAD with PNAMES, TEXTURE1, and real patch lumps.
fn textured_wad(pnames: &[&str], texture1: &[DefSpec<'_>], patch_lumps: &[(&str, Vec<u8>)]) -> Wad {
    let pn = build_pnames(pnames);
    let tx = build_texturex(texture1);
    let mut lumps: Vec<(&str, &[u8])> = vec![("PNAMES", &pn), ("TEXTURE1", &tx)];
    for (name, bytes) in patch_lumps {
        lumps.push((name, bytes));
    }
    Wad::from_bytes(common::build_wad(*b"IWAD", &lumps)).unwrap()
}

#[test]
fn texture_set_builds_and_finds_in_order() {
    let patch = build_picture(2, 4, &[vec![(0, vec![1, 2])], vec![(0, vec![3])]]);
    let wad = textured_wad(
        &["PA"],
        &[("TEX0", 2, 4, &[(0, 0, 0)]), ("TEX1", 2, 4, &[(0, 0, 0)])],
        &[("PA", patch)],
    );
    let set = wad.texture_set().unwrap().expect("TEXTURE1 present");
    assert!(set.warnings().is_empty());
    assert_eq!(set.textures().len(), 2);
    assert_eq!(set.find("TEX1"), Some(1));
    assert_eq!(set.find("TEX0"), Some(0));
    assert_eq!(set.find("NOPE"), None);

    let bare = Wad::from_bytes(common::build_wad(*b"PWAD", &[("THINGS", &[])])).unwrap();
    assert!(bare.texture_set().unwrap().is_none());
}

#[test]
fn texture_set_missing_pnames_strict_errors_lenient_warns() {
    use crustywad::gfx::{GfxError, GfxWarning};
    let tx = build_texturex(&[("TEX0", 2, 4, &[(0, 0, 0)])]);
    let wad = Wad::from_bytes(common::build_wad(*b"IWAD", &[("TEXTURE1", &tx)])).unwrap();
    assert!(matches!(
        wad.texture_set().unwrap_err(),
        GfxError::MissingPnames
    ));
    let set = wad
        .texture_set_with_options(ParseOptions::lenient())
        .unwrap()
        .expect("set still builds");
    assert!(
        set.warnings()
            .iter()
            .any(|w| matches!(w, GfxWarning::MissingPnames))
    );
}

#[test]
fn texture_set_bad_index_and_unresolved_name() {
    use crustywad::gfx::{GfxError, GfxWarning};
    let patch = build_picture(1, 1, &[vec![(0, vec![7])]]);
    // Index 5 out of bounds for a 1-name PNAMES.
    let wad = textured_wad(
        &["PA"],
        &[("TEX0", 1, 1, &[(0, 0, 5)])],
        &[("PA", patch.clone())],
    );
    assert!(matches!(
        wad.texture_set().unwrap_err(),
        GfxError::PatchIndexOutOfBounds {
            texture: 0,
            patch: 5,
            pnames_len: 1
        }
    ));
    let set = wad
        .texture_set_with_options(ParseOptions::lenient())
        .unwrap()
        .unwrap();
    assert!(set.warnings().iter().any(|w| matches!(
        w,
        GfxWarning::PatchIndexOutOfBounds {
            texture: 0,
            patch: 5,
            ..
        }
    )));

    // Name that matches no lump.
    let wad = textured_wad(
        &["GHOST"],
        &[("TEX0", 1, 1, &[(0, 0, 0)])],
        &[("PA", patch)],
    );
    assert!(matches!(
        wad.texture_set().unwrap_err(),
        GfxError::UnresolvedPatchName { .. }
    ));
    let set = wad
        .texture_set_with_options(ParseOptions::lenient())
        .unwrap()
        .unwrap();
    assert!(set.warnings().iter().any(|w| matches!(
        w,
        GfxWarning::UnresolvedPatchName { name } if name == "GHOST"
    )));

    // Name resolves but the lump is not a valid picture (6 bytes < header).
    let wad = textured_wad(
        &["BADPIC"],
        &[("TEX0", 1, 1, &[(0, 0, 0)])],
        &[("BADPIC", vec![1, 2, 3, 4, 5, 6])],
    );
    assert!(matches!(
        wad.texture_set().unwrap_err(),
        GfxError::PatchPictureFailed { .. }
    ));
    let set = wad
        .texture_set_with_options(ParseOptions::lenient())
        .unwrap()
        .unwrap();
    assert!(set.warnings().iter().any(|w| matches!(
        w,
        GfxWarning::PatchPictureFailed { name } if name == "BADPIC"
    )));
}

#[test]
fn texture_set_negative_patch_index_strict_errors_lenient_warns_and_composes_as_hole() {
    use crustywad::gfx::{GfxError, GfxWarning};
    // A negative PNAMES index (distinct from an out-of-range *positive*
    // index, already covered above): `usize::try_from` fails for it in the
    // set's referenced-name resolution pass, so it must never mark a name
    // referenced or panic — only the earlier validation pass's bounds check
    // (`patch_ref.patch < 0`) sees it.
    let patch = build_picture(1, 1, &[vec![(0, vec![7])]]);
    let wad = textured_wad(&["PA"], &[("NEG", 1, 1, &[(0, 0, -1)])], &[("PA", patch)]);
    assert!(matches!(
        wad.texture_set().unwrap_err(),
        GfxError::PatchIndexOutOfBounds {
            texture: 0,
            patch: -1,
            pnames_len: 1
        }
    ));
    let set = wad
        .texture_set_with_options(ParseOptions::lenient())
        .unwrap()
        .unwrap();
    assert!(set.warnings().iter().any(|w| matches!(
        w,
        GfxWarning::PatchIndexOutOfBounds {
            texture: 0,
            patch: -1,
            ..
        }
    )));
    // The dead negative ref contributes no column: composes as an all-holes
    // Medusa case, exactly like the positive out-of-bounds index.
    let (img, warnings) = set.compose(0, &ParseOptions::lenient()).unwrap();
    assert!(img.mask.iter().all(|m| !m));
    assert!(matches!(
        warnings.as_slice(),
        [GfxWarning::MedusaColumns {
            first_column: 0,
            count: 1
        }]
    ));
}

// ⚠ TASK-3 TESTS BELOW ⚠ — the next two tests call `compose`, which Task 3
// implements. Task 2's implementer SKIPS them (they are listed here beside
// the fixtures they reuse); Task 3's implementer writes them in ITS Step 1
// alongside the tests shown in Task 3.

#[test]
fn find_texture1_shadows_texture2_and_dead_refs_compose_as_holes() {
    use crustywad::gfx::{GfxError, GfxWarning};
    // TEXTURE1 and TEXTURE2 both define "DUP"; find() must return the
    // TEXTURE1 entry (index 0 — "earlier entries win").
    let patch = build_picture(1, 1, &[vec![(0, vec![7])]]);
    let pn = build_pnames(&["PA"]);
    let tx1 = build_texturex(&[("DUP", 1, 1, &[(0, 0, 0)])]);
    let tx2 = build_texturex(&[("DUP", 2, 2, &[(0, 0, 0)])]);
    let wad = Wad::from_bytes(common::build_wad(
        *b"IWAD",
        &[
            ("PNAMES", &pn),
            ("TEXTURE1", &tx1),
            ("TEXTURE2", &tx2),
            ("PA", &patch),
        ],
    ))
    .unwrap();
    let set = wad.texture_set().unwrap().unwrap();
    assert_eq!(set.textures().len(), 2);
    assert_eq!(set.find("DUP"), Some(0));
    assert_eq!(set.textures()[0].width, 1); // the TEXTURE1 def

    // A texture whose ONLY ref is dead (bad index) composes as all-holes
    // Medusa in lenient mode — dead refs are not contributors, no re-warn
    // beyond the build-time index warning.
    let wad = textured_wad(&["PA"], &[("DEAD", 1, 1, &[(0, 0, 9)])], &[("PA", patch)]);
    let set = wad
        .texture_set_with_options(ParseOptions::lenient())
        .unwrap()
        .unwrap();
    assert!(
        set.warnings()
            .iter()
            .any(|w| matches!(w, GfxWarning::PatchIndexOutOfBounds { patch: 9, .. }))
    );
    let (img, warnings) = set.compose(0, &ParseOptions::lenient()).unwrap();
    assert!(img.mask.iter().all(|m| !m));
    assert!(matches!(
        warnings.as_slice(),
        [GfxWarning::MedusaColumns {
            first_column: 0,
            count: 1
        }]
    ));
    // Strict compose on the same set: Medusa error (build was lenient,
    // compose strictness is the caller's choice per call).
    assert!(matches!(
        set.compose(0, &ParseOptions::strict()).unwrap_err(),
        GfxError::MedusaColumn { column: 0 }
    ));
}

#[test]
fn compose_negative_dimension_strict_errors_lenient_clamps() {
    use crustywad::gfx::{GfxError, GfxWarning};
    let patch = build_picture(1, 1, &[vec![(0, vec![7])]]);
    let wad = textured_wad(&["PA"], &[("NEGW", -3, 4, &[(0, 0, 0)])], &[("PA", patch)]);
    let set = wad.texture_set().unwrap().unwrap();
    assert!(matches!(
        set.compose(0, &ParseOptions::strict()).unwrap_err(),
        GfxError::NegativeDimension {
            field: "width",
            value: -3
        }
    ));
    let (img, warnings) = set.compose(0, &ParseOptions::lenient()).unwrap();
    assert_eq!((img.width, img.height), (0, 4));
    assert!(img.pixels.is_empty());
    // Zero columns: the Medusa scan is vacuous — only the clamp warning.
    assert!(matches!(
        warnings.as_slice(),
        [GfxWarning::NegativeDimension {
            field: "width",
            value: -3
        }]
    ));
}

#[test]
fn compose_golden_two_patches_with_overlap_and_medusa() {
    use crustywad::gfx::GfxWarning;
    // 4×4 texture; PATCHA (2 wide) at (0,0): col0 post@0 [1,2]; col1 post@2 [3].
    // PATCHB (2 wide) at (1,1): col0 post@1 [9]; col1 empty.
    // x=0: A only → rows 0,1 = 1,2. x=1: A col1 row2=3 then B col0 row 1+1=2 → 9.
    // x=2: B col1 only — a contributor with no coverage: NOT Medusa, all holes.
    // x=3: no contributor → Medusa.
    let pa = build_picture(2, 4, &[vec![(0, vec![1, 2])], vec![(2, vec![3])]]);
    let pb = build_picture(2, 4, &[vec![(1, vec![9])], vec![]]);
    let wad = textured_wad(
        &["PA", "PB"],
        &[("TEX0", 4, 4, &[(0, 0, 0), (1, 1, 1)])],
        &[("PA", pa), ("PB", pb)],
    );

    // Strict: Medusa at column 3.
    let set = wad.texture_set().unwrap().unwrap();
    assert!(matches!(
        set.compose(0, &ParseOptions::strict()).unwrap_err(),
        crustywad::gfx::GfxError::MedusaColumn { column: 3 }
    ));

    // Lenient: full image with holes + one aggregated warning.
    let (img, warnings) = set.compose(0, &ParseOptions::lenient()).unwrap();
    assert_eq!((img.width, img.height), (4, 4));
    let at = |x: usize, y: usize| (img.pixels[y * 4 + x], img.mask[y * 4 + x]);
    assert_eq!(at(0, 0), (1, true));
    assert_eq!(at(0, 1), (2, true));
    assert_eq!(at(1, 2), (9, true)); // later patch wins over A's 3
    assert!(!at(2, 0).1); // contributor but no coverage: hole, not Medusa
    assert!(!at(3, 0).1); // Medusa hole
    assert_eq!(img.mask.iter().filter(|m| **m).count(), 3);
    assert!(matches!(
        warnings.as_slice(),
        [GfxWarning::MedusaColumns {
            first_column: 3,
            count: 1
        }]
    ));
}

#[test]
fn compose_single_patch_equivalence_and_clamping() {
    // Equivalence: texture dims == patch dims, one ref at (0,0) → compose
    // output must equal the patch's own indexed view.
    let pa = build_picture(
        2,
        8,
        &[vec![(1, vec![5, 6])], vec![(0, vec![7]), (3, vec![8])]],
    );
    let wad = textured_wad(
        &["PA"],
        &[("SOLO", 2, 8, &[(0, 0, 0)])],
        &[("PA", pa.clone())],
    );
    let set = wad.texture_set().unwrap().unwrap();
    let (img, warnings) = set.compose(0, &ParseOptions::strict()).unwrap();
    assert!(warnings.is_empty());
    let direct = Picture::parse(&pa, &ParseOptions::strict())
        .unwrap()
        .to_indexed();
    assert_eq!((img.pixels, img.mask), (direct.pixels, direct.mask));

    // Horizontal clamping: patch hangs off both edges; vertical clip too.
    let wide = build_picture(2, 4, &[vec![(0, vec![1; 4])], vec![(0, vec![2; 4])]]);
    let wad = textured_wad(
        &["PW"],
        // 1×2 texture, patch at (-1,-1): only patch col1 lands (x=0), rows shift up by 1.
        &[("CLIP", 1, 2, &[(-1, -1, 0)])],
        &[("PW", wide)],
    );
    let set = wad.texture_set().unwrap().unwrap();
    let (img, warnings) = set.compose(0, &ParseOptions::strict()).unwrap();
    assert!(warnings.is_empty());
    // Patch col1 posts: rows 0..4 of value 2; origin_y -1 shifts to rows -1..3;
    // clipped to texture rows 0..2 → both rows covered with 2.
    assert_eq!(img.pixels, vec![2, 2]);
    assert!(img.mask.iter().all(|m| *m));
}

#[test]
fn compose_too_large_errors_in_both_modes() {
    use crustywad::gfx::GfxError;
    // Full-width (300), post-free patch: every column has a contributor, so
    // composing under the default (generous) limit hits neither the size
    // cap nor a spurious Medusa column — isolating this test to the
    // CompositeTooLarge policy alone. A narrower patch would leave most
    // columns with no contributor and legitimately trip MedusaColumn,
    // conflating this test with `compose_golden_two_patches_...`.
    let columns = vec![Vec::new(); 300];
    let pa = build_picture(300, 1, &columns);
    let wad = textured_wad(&["PA"], &[("BIG", 300, 300, &[(0, 0, 0)])], &[("PA", pa)]);
    let set = wad.texture_set().unwrap().unwrap();
    let tight = ParseOptions {
        limits: Limits::new().with_max_composite_pixels(1024),
        ..ParseOptions::strict()
    };
    assert!(matches!(
        set.compose(0, &tight).unwrap_err(),
        GfxError::CompositeTooLarge {
            width: 300,
            height: 300,
            max_pixels: 1024
        }
    ));
    let tight_lenient = ParseOptions {
        limits: Limits::new().with_max_composite_pixels(1024),
        ..ParseOptions::lenient()
    };
    assert!(set.compose(0, &tight_lenient).is_err()); // BOTH modes
    // Default limits admit it fine.
    assert!(set.compose(0, &ParseOptions::strict()).is_ok());
}

#[test]
fn compose_rgba_applies_palette_with_holes_transparent() {
    let pa = build_picture(1, 2, &[vec![(0, vec![5])]]);
    let wad = textured_wad(&["PA"], &[("T", 1, 2, &[(0, 0, 0)])], &[("PA", pa)]);
    let set = wad.texture_set().unwrap().unwrap();
    let (rgba, _) = set
        .compose_rgba(0, &ParseOptions::strict(), &gray_palette())
        .unwrap();
    assert_eq!(&rgba.pixels[0..4], &[5, 5, 5, 255]);
    assert_eq!(rgba.pixels[7], 0); // row 1 uncovered → alpha 0
}
