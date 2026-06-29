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
fn strict_rejects_unknown_magic() {
    let result = WadBuilder::new(WadKind::Unknown(*b"XWAD")).build();
    assert!(result.is_err());
}

#[test]
fn lenient_allows_unknown_magic() {
    let (bytes, _) = WadBuilder::new(WadKind::Unknown(*b"XWAD"))
        .build_with_options(&WriteOptions::lenient())
        .unwrap();
    assert_eq!(&bytes[0..4], b"XWAD");
}
