//! Classic graphics decode (#156, ADR-0022 §3): PLAYPAL, COLORMAP, flats.

mod common;

use crustywad::ParseOptions;
use crustywad::gfx::{Colormap, Flat, GfxError, GfxWarning, Picture, Playpal, Post};

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
    assert!(Picture::parse(&bytes, &ParseOptions::strict()).is_err());
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
