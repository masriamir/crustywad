//! Integration tests for the main WAD reader API.

mod common;

use crustywad::{ParseError, ParseOptions, ParseWarning, Wad, WadKind};
use proptest::prelude::*;
use tempfile::NamedTempFile;

#[test]
fn loads_from_path() {
    let bytes = common::build_wad(*b"IWAD", &[("DEMO1", &[1, 2, 3, 4])]);
    let file = NamedTempFile::new().expect("tempfile should be created");
    std::fs::write(file.path(), bytes).expect("wad should be written");

    let wad = Wad::from_path(file.path()).expect("wad should load from path");
    assert_eq!(wad.kind(), WadKind::Iwad);
    assert_eq!(wad.lump_bytes(0), Some(&[1, 2, 3, 4][..]));
}

#[cfg(feature = "mmap")]
#[test]
fn loads_from_path_mapped() {
    let bytes = common::build_wad(*b"IWAD", &[("DEMO1", &[1, 2, 3, 4])]);
    let file = NamedTempFile::new().expect("tempfile should be created");
    std::fs::write(file.path(), bytes).expect("wad should be written");

    let wad = Wad::from_path_mapped(file.path()).expect("wad should load via mmap");
    assert_eq!(wad.kind(), WadKind::Iwad);
    assert_eq!(wad.lump_bytes(0), Some(&[1, 2, 3, 4][..]));
}

#[cfg(feature = "mmap")]
#[test]
fn mmap_nonexistent_file_returns_io_error() {
    let err =
        Wad::from_path_mapped("/nonexistent/path/file.wad").expect_err("missing file should fail");
    assert!(matches!(err, ParseError::Io { .. }));
}

#[cfg(feature = "mmap")]
#[test]
fn mmap_empty_file_fails() {
    // On Linux, mmap of a zero-length file returns EINVAL → ParseError::Io.
    // On macOS, the mapping succeeds but the WAD parser can't read the header → ParseError::Header.
    // Either way, loading an empty file must fail.
    let file = NamedTempFile::new().expect("tempfile should be created");
    Wad::from_path_mapped(file.path()).expect_err("empty file should not load as a WAD");
}

#[test]
fn finds_lumps_by_name() {
    let lumps = [("TITLEPIC", &[9, 9][..]), ("PLAYPAL", &[3][..])];
    let wad = Wad::from_bytes(common::build_wad(*b"PWAD", &lumps)).expect("wad should parse");
    let lump_map = common::lump_map(&lumps);
    let playpal = wad.lump_by_name("PLAYPAL").expect("PLAYPAL missing");
    assert_eq!(wad.lump_data(playpal), lump_map["PLAYPAL"]);
}

#[test]
fn lenient_mode_recovers_directory_overflow() {
    let mut bytes = common::build_wad(*b"PWAD", &[("A", &[1]), ("B", &[2])]);
    bytes[8..12].copy_from_slice(&999_i32.to_le_bytes());

    let wad = Wad::from_bytes_with_options(bytes, ParseOptions::lenient())
        .expect("lenient mode should recover");
    assert_eq!(wad.lump_count(), 0);
    assert!(wad.warnings().iter().any(|warning| matches!(
        warning,
        ParseWarning::OutOfBounds {
            field: "directory",
            ..
        }
    )));
}

#[test]
fn parses_basic_wad() {
    let wad = Wad::from_bytes(common::build_wad(*b"IWAD", &[("PLAYPAL", &[1, 2, 3])]))
        .expect("wad should parse");
    assert_eq!(wad.kind(), WadKind::Iwad);
    assert_eq!(wad.lump_count(), 1);
    assert_eq!(wad.lump_by_name("PLAYPAL").expect("missing lump").size(), 3);
    assert_eq!(wad.lump_bytes(0), Some(&[1, 2, 3][..]));
}

#[test]
fn strict_mode_rejects_bad_magic() {
    let err = Wad::from_bytes(common::build_wad(*b"NOPE", &[])).expect_err("magic should fail");
    assert!(matches!(err, ParseError::InvalidMagic { .. }));
}

#[test]
fn lenient_mode_collects_warnings() {
    let mut wad = common::build_wad(*b"NOPE", &[("TEST", &[1, 2, 3])]);
    wad[4..8].copy_from_slice(&1_i32.to_le_bytes());
    wad[8..12].copy_from_slice(&128_i32.to_le_bytes());
    let parsed = Wad::from_bytes_with_options(wad, ParseOptions::lenient())
        .expect("lenient parse should succeed");
    assert!(matches!(parsed.kind(), WadKind::Unknown(_)));
    assert!(
        parsed
            .warnings()
            .iter()
            .any(|warning| matches!(warning, ParseWarning::InvalidMagic(_)))
    );
    assert_eq!(parsed.lump_count(), 0);
}

#[test]
fn strict_mode_rejects_non_ascii_names() {
    let mut wad = common::build_wad(*b"PWAD", &[("TEST", &[1])]);
    let name_offset = wad.len() - 8;
    wad[name_offset] = 0xFF;
    let err = Wad::from_bytes(wad).expect_err("non-ascii name should fail");
    assert!(matches!(err, ParseError::NonAsciiName { .. }));
}

#[test]
fn lenient_mode_clamps_oversized_lumps() {
    let mut wad = common::build_wad(*b"PWAD", &[("TEST", &[1, 2, 3])]);
    let size_offset = wad.len() - 16;
    wad[size_offset + 4..size_offset + 8].copy_from_slice(&999_i32.to_le_bytes());
    let parsed = Wad::from_bytes_with_options(wad, ParseOptions::lenient())
        .expect("lenient parse should succeed");
    assert_eq!(parsed.lump_bytes(0), Some(&[1, 2, 3][..]));
    assert!(
        parsed
            .warnings()
            .iter()
            .any(|warning| matches!(warning, ParseWarning::OutOfBounds { .. }))
    );
}

#[test]
fn parse_options_default_to_strict() {
    use crustywad::Strictness;
    assert_eq!(ParseOptions::default().strictness, Strictness::Strict);
}

#[test]
fn strict_mode_rejects_lump_inside_directory() {
    let mut wad = common::build_wad(*b"IWAD", &[("TEST", &[1, 2, 3])]);
    // Directory starts at byte 15. Set filepos to 17 — inside the directory region [15, 31).
    wad[15..19].copy_from_slice(&17_i32.to_le_bytes());
    let err = Wad::from_bytes(wad).expect_err("lump pointing into directory should fail");
    assert!(matches!(
        err,
        ParseError::OutOfBounds {
            field: "lump data",
            ..
        }
    ));
}

#[test]
fn lenient_mode_clamps_lump_inside_directory() {
    let mut wad = common::build_wad(*b"IWAD", &[("TEST", &[1, 2, 3])]);
    wad[15..19].copy_from_slice(&17_i32.to_le_bytes());
    let parsed = Wad::from_bytes_with_options(wad, ParseOptions::lenient())
        .expect("lenient parse should succeed");
    assert!(parsed.warnings().iter().any(|w| matches!(
        w,
        ParseWarning::OutOfBounds {
            field: "lump data",
            ..
        }
    )));
    assert_eq!(parsed.lump_bytes(0), Some(&[][..]));
}

#[test]
fn strict_mode_rejects_negative_lump_filepos() {
    let mut wad = common::build_wad(*b"IWAD", &[("TEST", &[1, 2, 3])]);
    wad[15..19].copy_from_slice(&(-1_i32).to_le_bytes());
    let err = Wad::from_bytes(wad).expect_err("negative filepos should fail");
    assert!(matches!(
        err,
        ParseError::NegativeValue {
            field: "filepos",
            value: -1
        }
    ));
}

#[test]
fn lenient_mode_recovers_negative_lump_size() {
    let mut wad = common::build_wad(*b"IWAD", &[("TEST", &[1, 2, 3])]);
    wad[19..23].copy_from_slice(&(-10_i32).to_le_bytes());
    let parsed = Wad::from_bytes_with_options(wad, ParseOptions::lenient())
        .expect("lenient parse should succeed");
    assert!(parsed.warnings().iter().any(|w| matches!(
        w,
        ParseWarning::NegativeValue {
            field: "size",
            value: -10
        }
    )));
    assert_eq!(parsed.lump_bytes(0), Some(&[][..]));
}

#[test]
fn into_bytes_round_trips() {
    let original = common::build_wad(*b"IWAD", &[("FLAT", &[0xAA, 0xBB])]);
    let wad = Wad::from_bytes(original.clone()).expect("wad should parse");
    assert_eq!(wad.into_bytes(), original);
}

proptest! {
    #[test]
    fn strict_parser_handles_generated_empty_wads(kind in prop_oneof![Just(*b"IWAD"), Just(*b"PWAD")]) {
        let wad = Wad::from_bytes(common::build_wad(kind, &[])).expect("generated wad should parse");
        prop_assert_eq!(wad.lump_count(), 0);
        prop_assert!(matches!(wad.kind(), WadKind::Iwad | WadKind::Pwad));
    }

    // I-1: No panic on arbitrary bytes (strict mode)
    #[test]
    fn no_panic_strict_arbitrary_bytes(data in proptest::collection::vec(any::<u8>(), 0..=8192usize)) {
        let _ = std::hint::black_box(
            Wad::from_bytes_with_options(data, ParseOptions::strict())
        );
    }

    // I-1: No panic on arbitrary bytes (lenient mode)
    #[test]
    fn no_panic_lenient_arbitrary_bytes(data in proptest::collection::vec(any::<u8>(), 0..=8192usize)) {
        let _ = std::hint::black_box(
            Wad::from_bytes_with_options(data, ParseOptions::lenient())
        );
    }

    // I-2: lump_count() == lumps().len() == header().num_lumps for any structurally valid WAD
    #[test]
    fn lump_count_consistent(bytes in common::arb_valid_wad()) {
        let result = Wad::from_bytes(bytes);
        prop_assert!(result.is_ok(), "arb_valid_wad() must produce parseable bytes: {:?}", result.err());
        let wad = result.unwrap();
        prop_assert_eq!(wad.lump_count(), wad.lumps().len());
        prop_assert_eq!(wad.lump_count(), wad.header().num_lumps);
    }

    // I-3: lump_by_name agrees with lumps() for every lump in the directory
    #[test]
    fn lump_by_name_agrees_with_lumps(bytes in common::arb_valid_wad()) {
        let result = Wad::from_bytes(bytes);
        prop_assert!(result.is_ok(), "arb_valid_wad() must produce parseable bytes: {:?}", result.err());
        let wad = result.unwrap();
        for lump in wad.lumps() {
            prop_assert!(
                wad.lump_by_name(lump.name()).is_some(),
                "lump_by_name returned None for {:?}", lump.name()
            );
        }
    }

    // I-4: All lump names in strict mode are valid ASCII and at most 8 chars
    #[test]
    fn strict_lump_names_are_ascii(bytes in common::arb_valid_wad()) {
        let result = Wad::from_bytes(bytes);
        prop_assert!(result.is_ok(), "arb_valid_wad() must produce parseable bytes: {:?}", result.err());
        let wad = result.unwrap();
        for lump in wad.lumps() {
            prop_assert!(lump.name().is_ascii(), "non-ASCII name: {:?}", lump.name());
            prop_assert!(lump.name().len() <= 8, "name too long: {:?}", lump.name());
        }
    }

    // I-6: If strict parsing fails, lenient either also fails or produces at
    // least one warning — strict Err must not become lenient Ok with no warnings.
    #[test]
    fn strict_errors_appear_in_lenient(
        bytes in proptest::collection::vec(any::<u8>(), 0..=8192usize)
    ) {
        let strict = Wad::from_bytes_with_options(
            bytes.clone(),
            ParseOptions::strict(),
        );
        if strict.is_err() {
            let lenient = Wad::from_bytes_with_options(
                bytes,
                ParseOptions::lenient(),
            );
            if let Ok(wad) = lenient {
                prop_assert!(
                    !wad.warnings().is_empty(),
                    "strict Err but lenient Ok with no warnings"
                );
            }
        }
    }

    // I-7: lump_bytes returns Some for every valid index and the returned slice
    // is fully within the original input bytes (correct range, correct content).
    #[test]
    fn lump_bytes_always_in_bounds(bytes in common::arb_valid_wad()) {
        let original = bytes.clone();
        let result = Wad::from_bytes(bytes);
        prop_assert!(result.is_ok(), "arb_valid_wad() must produce parseable bytes: {:?}", result.err());
        let wad = result.unwrap();
        for i in 0..wad.lump_count() {
            let lump = wad.lump(i).unwrap();
            let filepos = lump.filepos();
            let size = lump.size();
            let slice = wad.lump_bytes(i);
            prop_assert!(slice.is_some(), "lump_bytes({i}) returned None");
            let slice = slice.unwrap();
            prop_assert_eq!(
                slice.len(), size,
                "lump_bytes({}) length {} != lump.size() {}", i, slice.len(), size
            );
            prop_assert_eq!(
                slice, &original[filepos..filepos + size],
                "lump_bytes({}) content does not match original bytes[{}..{}]",
                i, filepos, filepos + size
            );
        }
    }
}

#[test]
fn header_returns_parsed_header() {
    let wad = Wad::from_bytes(common::build_wad(*b"IWAD", &[("FLAT", &[0xAA])]))
        .expect("wad should parse");
    let header = wad.header();
    assert_eq!(header.kind, WadKind::Iwad);
    assert_eq!(header.num_lumps, 1);
}

#[test]
fn clone_produces_independent_copy() {
    let wad = Wad::from_bytes(common::build_wad(*b"PWAD", &[("DEMO", &[1, 2, 3])]))
        .expect("wad should parse");
    let cloned = wad.clone();
    assert_eq!(cloned.kind(), WadKind::Pwad);
    assert_eq!(cloned.lump_count(), 1);
    assert_eq!(cloned.lump_bytes(0), Some(&[1, 2, 3][..]));
    let original_bytes = wad.into_bytes();
    let cloned_bytes = cloned.into_bytes();
    assert_eq!(original_bytes, cloned_bytes);
    assert_ne!(original_bytes.as_ptr(), cloned_bytes.as_ptr());
}

#[test]
fn lump_by_name_returns_none_for_missing_lump() {
    let wad =
        Wad::from_bytes(common::build_wad(*b"IWAD", &[("EXIST", &[1])])).expect("wad should parse");
    assert!(wad.lump_by_name("NOPE").is_none());
}

#[test]
fn lump_returns_none_for_out_of_bounds_index() {
    let wad =
        Wad::from_bytes(common::build_wad(*b"IWAD", &[("FLAT", &[0])])).expect("wad should parse");
    assert!(wad.lump(99).is_none());
}

#[test]
fn lump_bytes_returns_none_for_out_of_bounds_index() {
    let wad = Wad::from_bytes(common::build_wad(*b"IWAD", &[])).expect("wad should parse");
    assert!(wad.lump_bytes(0).is_none());
}

#[test]
fn lump_accessors_return_correct_values() {
    let wad = Wad::from_bytes(common::build_wad(*b"IWAD", &[("PLAYPAL", &[7, 8, 9])]))
        .expect("wad should parse");
    let lump = wad.lump(0).expect("lump 0 should exist");
    assert_eq!(lump.name(), "PLAYPAL");
    assert_eq!(lump.size(), 3);
    // filepos should be right after the 12-byte WAD header
    assert_eq!(lump.filepos(), 12);
}

#[test]
fn non_empty_lump_has_nonzero_size() {
    let wad = Wad::from_bytes(common::build_wad(*b"IWAD", &[("MAP01", &[1, 2, 3])]))
        .expect("wad should parse");
    let lump = wad.lump(0).expect("lump should exist");
    assert_eq!(lump.size(), 3);
}

#[test]
fn strict_mode_rejects_directory_extending_past_end() {
    // Build a WAD with numlumps set so that the directory region extends past the end of the file
    let wad = common::build_wad(*b"IWAD", &[("TEST", &[1, 2, 3])]);
    // directory offset is valid but numlumps is much too large
    let mut corrupt = wad.clone();
    // Set numlumps to 1000 (way beyond what the file contains)
    corrupt[4..8].copy_from_slice(&1000_i32.to_le_bytes());
    let err = Wad::from_bytes(corrupt).expect_err("directory overflow should fail in strict mode");
    assert!(matches!(
        err,
        ParseError::OutOfBounds {
            field: "directory",
            ..
        }
    ));
}

#[test]
fn lenient_mode_non_ascii_name_decoded_lossily() {
    let mut wad = common::build_wad(*b"PWAD", &[("TEST", &[1])]);
    let name_offset = wad.len() - 8;
    wad[name_offset] = 0xFF;
    let parsed = Wad::from_bytes_with_options(wad, ParseOptions::lenient())
        .expect("lenient mode should handle non-ascii names");
    assert_eq!(parsed.lump_count(), 1);
    assert!(
        parsed
            .warnings()
            .iter()
            .any(|w| matches!(w, ParseWarning::NonAsciiName { index: 0 }))
    );
}

#[test]
fn wad_kind_unknown_preserved_in_lenient_mode() {
    let wad = common::build_wad(*b"XWAD", &[]);
    let parsed = Wad::from_bytes_with_options(wad, ParseOptions::lenient())
        .expect("lenient mode should accept unknown magic");
    assert!(matches!(parsed.kind(), WadKind::Unknown(_)));
    if let WadKind::Unknown(magic) = parsed.kind() {
        assert_eq!(&magic, b"XWAD");
    }
}

#[test]
fn parse_error_display_formats_correctly() {
    let err =
        Wad::from_bytes(common::build_wad(*b"NOPE", &[])).expect_err("invalid magic should fail");
    let msg = err.to_string();
    assert!(
        msg.contains("NOPE"),
        "error message should contain the invalid magic"
    );
}

#[test]
fn parse_warning_display_formats_correctly() {
    let mut wad = common::build_wad(*b"NOPE", &[]);
    wad[4..8].copy_from_slice(&0_i32.to_le_bytes()); // 0 lumps
    let parsed = Wad::from_bytes_with_options(wad, ParseOptions::lenient())
        .expect("lenient parse should succeed");
    let warnings = parsed.warnings();
    assert!(!warnings.is_empty());
    let msg = warnings[0].to_string();
    assert!(!msg.is_empty(), "warning should have a display message");
}

#[test]
fn parse_options_strict_factory() {
    use crustywad::Strictness;
    let opts = ParseOptions::strict();
    assert_eq!(opts.strictness, Strictness::Strict);
}

#[test]
fn parse_options_lenient_factory() {
    use crustywad::Strictness;
    let opts = ParseOptions::lenient();
    assert_eq!(opts.strictness, Strictness::Lenient);
}

#[test]
fn wad_lumps_slice_matches_lump_count() {
    let wad = Wad::from_bytes(common::build_wad(*b"IWAD", &[("A", &[1]), ("B", &[2])]))
        .expect("wad should parse");
    assert_eq!(wad.lumps().len(), wad.lump_count());
}

#[test]
fn lump_data_returns_correct_slice() {
    let wad = Wad::from_bytes(common::build_wad(*b"IWAD", &[("DEMO1", &[10, 20, 30])]))
        .expect("wad should parse");
    let lump = wad.lump(0).expect("lump should exist");
    let data = wad.lump_data(lump);
    assert_eq!(data, &[10, 20, 30]);
}

#[cfg(feature = "mmap")]
#[test]
fn mmap_into_bytes_recovers_mapped_data() {
    let original = common::build_wad(*b"IWAD", &[("FLAT", &[0xAA, 0xBB])]);
    let file = NamedTempFile::new().expect("tempfile should be created");
    std::fs::write(file.path(), &original).expect("wad should be written");
    let wad = Wad::from_path_mapped(file.path()).expect("wad should load via mmap");
    let recovered = wad.into_bytes();
    assert_eq!(recovered, original);
}

#[test]
fn lump_data_after_directory_parses_correctly() {
    // Build a WAD where the lump data lives AFTER the directory (unusual but valid).
    // This exercises the `filepos >= directory_end` branch in validate_entry.
    //
    // Layout: header (12) | directory (16) | lump data (3)
    //   header:    magic=IWAD, numlumps=1, infotableofs=12
    //   directory: filepos=28, size=3, name=b"AFTER\0\0\0"
    //   lump data: [0xDE, 0xAD, 0xBE] at offset 28
    let mut bytes = Vec::new();
    // Header
    bytes.extend_from_slice(b"IWAD");
    bytes.extend_from_slice(&1_i32.to_le_bytes()); // numlumps = 1
    bytes.extend_from_slice(&12_i32.to_le_bytes()); // infotableofs = 12
    // Directory entry (16 bytes)
    bytes.extend_from_slice(&28_i32.to_le_bytes()); // filepos = 28 (after directory)
    bytes.extend_from_slice(&3_i32.to_le_bytes()); // size = 3
    bytes.extend_from_slice(b"AFTER\0\0\0"); // name (8 bytes)
    // Lump data at offset 28
    bytes.extend_from_slice(&[0xDE, 0xAD, 0xBE]);
    assert_eq!(bytes.len(), 31);

    let wad = Wad::from_bytes(bytes).expect("WAD with post-directory lump data should parse");
    assert_eq!(wad.lump_count(), 1);
    assert_eq!(wad.lump_bytes(0), Some(&[0xDE, 0xAD, 0xBE][..]));
}

#[cfg(feature = "mmap")]
#[test]
fn from_path_mapped_with_options_lenient() {
    let original = common::build_wad(*b"IWAD", &[("FLAT", &[0xCC])]);
    let file = NamedTempFile::new().expect("tempfile should be created");
    std::fs::write(file.path(), &original).expect("wad should be written");
    let wad = Wad::from_path_mapped_with_options(file.path(), ParseOptions::lenient())
        .expect("mmap with lenient options should succeed");
    assert_eq!(wad.kind(), WadKind::Iwad);
}
