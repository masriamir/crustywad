//! Integration tests for WAD write support.
#![cfg(feature = "write")]

use crustywad::{WadBuilder, WadKind, WriteOptions};

#[test]
fn builder_produces_parseable_empty_iwad() {
    let bytes = WadBuilder::new(WadKind::Iwad)
        .build()
        .expect("empty IWAD build should succeed");
    let wad = crustywad::Wad::from_bytes(bytes).expect("should re-parse");
    assert_eq!(wad.lump_count(), 0);
    assert_eq!(wad.kind(), WadKind::Iwad);
}

#[test]
fn builder_produces_parseable_single_lump() {
    let bytes = WadBuilder::new(WadKind::Pwad)
        .add_lump("TESTLUMP", b"hello")
        .build()
        .expect("single-lump PWAD build should succeed");
    let wad = crustywad::Wad::from_bytes(bytes).expect("should re-parse");
    assert_eq!(wad.lump_count(), 1);
    assert_eq!(wad.lumps()[0].name(), "TESTLUMP");
    assert_eq!(wad.lumps()[0].size(), 5);
    assert_eq!(wad.lump_data(&wad.lumps()[0]), b"hello");
}

#[test]
fn lump_filepos_and_size_are_correct() {
    let bytes = WadBuilder::new(WadKind::Pwad)
        .add_lump("A", b"hello")
        .add_lump("B", b"world!!")
        .build()
        .unwrap();
    let wad = crustywad::Wad::from_bytes(bytes).unwrap();
    assert_eq!(wad.lumps()[0].name(), "A");
    assert_eq!(wad.lumps()[0].size(), 5);
    assert_eq!(wad.lumps()[0].filepos(), 12); // right after 12-byte header
    assert_eq!(wad.lumps()[1].name(), "B");
    assert_eq!(wad.lumps()[1].size(), 7);
    assert_eq!(wad.lumps()[1].filepos(), 17); // 12 + 5
}

#[test]
fn wad_to_builder_round_trips() {
    use crustywad::Wad;
    // Build a test WAD using the builder itself
    let original_bytes = WadBuilder::new(WadKind::Pwad)
        .add_lump("MAP01", b"data")
        .add_lump("THINGS", b"more")
        .build()
        .unwrap();
    let wad = Wad::from_bytes(original_bytes).unwrap();
    let rebuilt = wad.to_builder().build().expect("round-trip should succeed");
    let wad2 = Wad::from_bytes(rebuilt).unwrap();
    assert_eq!(wad2.lump_count(), 2);
    assert_eq!(wad2.lumps()[0].name(), "MAP01");
    assert_eq!(wad2.lumps()[1].name(), "THINGS");
}

#[test]
fn strict_mode_rejects_nul_in_name() {
    let result = WadBuilder::new(WadKind::Pwad)
        .add_lump("BAD\0NAME", b"")
        .build();
    assert!(matches!(
        result,
        Err(crustywad::WriteError::NulInName { .. })
    ));
}

#[test]
fn strict_mode_rejects_non_ascii_name() {
    let result = WadBuilder::new(WadKind::Pwad)
        .add_lump("BÄDNAME", b"")
        .build();
    assert!(matches!(
        result,
        Err(crustywad::WriteError::NonAsciiName { .. })
    ));
}

#[test]
fn lenient_truncates_name_longer_than_8() {
    let (bytes, warnings) = WadBuilder::new(WadKind::Pwad)
        .add_lump("TOOLONGNAME", b"")
        .build_with_options(&WriteOptions::lenient())
        .unwrap();
    assert!(!warnings.is_empty());
    let wad = crustywad::Wad::from_bytes(bytes).unwrap();
    assert_eq!(wad.lumps()[0].name(), "TOOLONGN");
}

#[test]
fn strict_rejects_name_longer_than_8() {
    let result = WadBuilder::new(WadKind::Pwad)
        .add_lump("TOOLONGNAME", b"")
        .build();
    assert!(matches!(
        result,
        Err(crustywad::WriteError::NameTooLong { .. })
    ));
}

#[test]
fn strict_rejects_unknown_magic() {
    let result = WadBuilder::new(WadKind::Unknown(*b"XWAD")).build();
    assert!(matches!(
        result,
        Err(crustywad::WriteError::UnknownMagicStrict)
    ));
}

#[test]
fn lenient_allows_unknown_magic_with_warning() {
    let (bytes, warnings) = WadBuilder::new(WadKind::Unknown(*b"XWAD"))
        .build_with_options(&WriteOptions::lenient())
        .unwrap();
    assert_eq!(&bytes[0..4], b"XWAD");
    assert!(
        warnings.iter().any(
            |w| matches!(w, crustywad::WriteWarning::UnknownMagic { magic } if magic == b"XWAD")
        ),
        "expected UnknownMagic warning, got: {warnings:?}"
    );
}

#[test]
fn write_options_default_is_strict() {
    let opts = WriteOptions::default();
    assert_eq!(opts.strictness, crustywad::Strictness::Strict);
}

// --- Error field assertions ---

#[test]
fn nul_in_name_error_carries_name() {
    let result = WadBuilder::new(WadKind::Pwad)
        .add_lump("BAD\0NAME", b"")
        .build();
    match result {
        Err(crustywad::WriteError::NulInName { name }) => {
            assert_eq!(name, "BAD\0NAME");
        }
        other => panic!("expected NulInName, got {other:?}"),
    }
}

#[test]
fn non_ascii_name_error_carries_name() {
    let result = WadBuilder::new(WadKind::Pwad)
        .add_lump("BÄDNAME", b"")
        .build();
    match result {
        Err(crustywad::WriteError::NonAsciiName { name }) => {
            assert_eq!(name, "BÄDNAME");
        }
        other => panic!("expected NonAsciiName, got {other:?}"),
    }
}

#[test]
fn name_too_long_error_carries_name_and_len() {
    let result = WadBuilder::new(WadKind::Pwad)
        .add_lump("TOOLONGNAME", b"")
        .build();
    match result {
        Err(crustywad::WriteError::NameTooLong { name, len }) => {
            assert_eq!(name, "TOOLONGNAME");
            assert_eq!(len, 11);
        }
        other => panic!("expected NameTooLong, got {other:?}"),
    }
}

// --- WriteWarning field assertions ---

#[test]
fn lenient_name_truncated_warning_carries_original_name() {
    let (_, warnings) = WadBuilder::new(WadKind::Pwad)
        .add_lump("TOOLONGNAME", b"")
        .build_with_options(&WriteOptions::lenient())
        .unwrap();
    assert_eq!(warnings.len(), 1);
    assert_eq!(
        warnings[0],
        crustywad::WriteWarning::NameTruncated {
            name: "TOOLONGNAME".to_owned(),
        }
    );
}

#[test]
fn strict_mode_build_with_options_returns_no_warnings() {
    let (bytes, warnings) = WadBuilder::new(WadKind::Iwad)
        .add_lump("MAP01", b"data")
        .build_with_options(&WriteOptions::strict())
        .unwrap();
    assert!(warnings.is_empty());
    let wad = crustywad::Wad::from_bytes(bytes).unwrap();
    assert_eq!(wad.lump_count(), 1);
}

// --- Empty lump ---

#[test]
fn empty_lump_writes_zero_size_and_correct_filepos() {
    // An empty lump (zero-byte data) should have filepos pointing right after the
    // 12-byte header and size 0.
    let bytes = WadBuilder::new(WadKind::Iwad)
        .add_lump("EMPTYLMP", b"")
        .build()
        .unwrap();
    let wad = crustywad::Wad::from_bytes(bytes).unwrap();
    assert_eq!(wad.lump_count(), 1);
    assert_eq!(wad.lumps()[0].name(), "EMPTYLMP");
    assert_eq!(wad.lumps()[0].size(), 0);
    // filepos still points to byte 12 (right after the 12-byte header), even
    // though no data bytes are stored there.
    assert_eq!(wad.lumps()[0].filepos(), 12);
    assert_eq!(wad.lump_data(&wad.lumps()[0]), b"");
}

// --- Multiple lumps with the same name ---

#[test]
fn duplicate_names_are_preserved_in_order() {
    let bytes = WadBuilder::new(WadKind::Pwad)
        .add_lump("FLAT1", b"first")
        .add_lump("FLAT1", b"second")
        .add_lump("FLAT1", b"third")
        .build()
        .unwrap();
    let wad = crustywad::Wad::from_bytes(bytes).unwrap();
    assert_eq!(wad.lump_count(), 3);
    assert_eq!(wad.lumps()[0].name(), "FLAT1");
    assert_eq!(wad.lumps()[1].name(), "FLAT1");
    assert_eq!(wad.lumps()[2].name(), "FLAT1");
    assert_eq!(wad.lump_data(&wad.lumps()[0]), b"first");
    assert_eq!(wad.lump_data(&wad.lumps()[1]), b"second");
    assert_eq!(wad.lump_data(&wad.lumps()[2]), b"third");
}

// --- Exact 8-byte name ---

#[test]
fn exactly_8_byte_name_is_not_truncated_and_emits_no_warning() {
    let (bytes, warnings) = WadBuilder::new(WadKind::Pwad)
        .add_lump("ABCDEFGH", b"payload")
        .build_with_options(&WriteOptions::lenient())
        .unwrap();
    assert!(
        warnings.is_empty(),
        "8-byte name must not produce a warning"
    );
    let wad = crustywad::Wad::from_bytes(bytes).unwrap();
    assert_eq!(wad.lumps()[0].name(), "ABCDEFGH");
}

// --- IWAD to_builder round-trip ---

#[test]
fn iwad_to_builder_round_trip_preserves_kind() {
    use crustywad::Wad;
    let original = WadBuilder::new(WadKind::Iwad)
        .add_lump("E1M1", b"level")
        .add_lump("THINGS", b"tdata")
        .build()
        .unwrap();
    let wad = Wad::from_bytes(original).unwrap();
    assert_eq!(wad.kind(), WadKind::Iwad);
    let rebuilt = wad
        .to_builder()
        .build()
        .expect("IWAD round-trip should succeed");
    let wad2 = Wad::from_bytes(rebuilt).unwrap();
    assert_eq!(wad2.kind(), WadKind::Iwad);
    assert_eq!(wad2.lump_count(), 2);
    assert_eq!(wad2.lumps()[0].name(), "E1M1");
    assert_eq!(wad2.lumps()[1].name(), "THINGS");
    assert_eq!(wad2.lump_data(&wad2.lumps()[0]), b"level");
}

// --- Multiple lumps: contiguous layout ---

#[test]
fn multiple_lump_offsets_are_contiguous() {
    // Three lumps: 3, 5, 7 bytes.
    // Expected filepos: 12, 15, 20 (header=12, then cumulative data sizes).
    let bytes = WadBuilder::new(WadKind::Pwad)
        .add_lump("L1", b"abc")
        .add_lump("L2", b"defgh")
        .add_lump("L3", b"ijklmno")
        .build()
        .unwrap();
    let wad = crustywad::Wad::from_bytes(bytes).unwrap();
    assert_eq!(wad.lumps()[0].filepos(), 12);
    assert_eq!(wad.lumps()[0].size(), 3);
    assert_eq!(wad.lumps()[1].filepos(), 15);
    assert_eq!(wad.lumps()[1].size(), 5);
    assert_eq!(wad.lumps()[2].filepos(), 20);
    assert_eq!(wad.lumps()[2].size(), 7);
}

// --- Short lump name pads with NUL bytes ---

#[test]
fn short_lump_name_round_trips_correctly() {
    let bytes = WadBuilder::new(WadKind::Pwad)
        .add_lump("A", b"data")
        .build()
        .unwrap();
    let wad = crustywad::Wad::from_bytes(bytes).unwrap();
    assert_eq!(wad.lumps()[0].name(), "A");
}

// --- Lenient unknown-magic emits no WriteWarning ---

#[test]
fn lenient_unknown_magic_emits_no_warning() {
    // WadKind::Unknown in lenient mode should NOT emit a WriteWarning —
    // the raw bytes are written directly; there is no warning type for magic.
    let (bytes, warnings) = WadBuilder::new(WadKind::Unknown(*b"XWAD"))
        .build_with_options(&WriteOptions::lenient())
        .unwrap();
    assert!(
        warnings.is_empty(),
        "Unknown magic in lenient mode must not emit a warning"
    );
    assert_eq!(&bytes[0..4], b"XWAD");
}
