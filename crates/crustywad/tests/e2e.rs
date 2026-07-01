//! End-to-end integration tests: read → modify → write → verify.
//!
//! These tests exercise the full pipeline: load a WAD from bytes, mutate it via
//! [`WadBuilder`], serialize back to bytes, and re-parse to confirm the result.
#![cfg(feature = "write")]

mod common;

use crustywad::{Wad, WadBuilder, WadKind};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a minimal two-lump IWAD byte buffer using the low-level helper.
fn two_lump_iwad() -> Vec<u8> {
    common::build_wad(
        *b"IWAD",
        &[("THINGS", b"\x01\x02\x03\x04"), ("LINEDEFS", b"\x05\x06")],
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Loading a WAD, converting it to a builder, and immediately re-building it
/// must produce an identical parse result.
#[test]
fn round_trip_preserves_kind_and_lumps() {
    let original = two_lump_iwad();
    let wad = Wad::from_bytes(original).expect("original should parse");

    let rebuilt = wad
        .to_builder()
        .build()
        .expect("round-trip build should succeed");
    let wad2 = Wad::from_bytes(rebuilt).expect("rebuilt bytes should parse");

    assert_eq!(wad2.kind(), WadKind::Iwad);
    assert_eq!(wad2.lump_count(), 2);
    assert_eq!(wad2.lumps()[0].name(), "THINGS");
    assert_eq!(wad2.lump_data(&wad2.lumps()[0]), b"\x01\x02\x03\x04");
    assert_eq!(wad2.lumps()[1].name(), "LINEDEFS");
    assert_eq!(wad2.lump_data(&wad2.lumps()[1]), b"\x05\x06");
}

/// After converting a WAD to a builder, appending a new lump, and re-building,
/// the new lump must appear last in the directory with its payload intact.
#[test]
fn add_lump_and_read_back() {
    let original = two_lump_iwad();
    let wad = Wad::from_bytes(original).expect("original should parse");

    let built = wad
        .to_builder()
        .add_lump("NEWLUMP", b"hello world")
        .build()
        .expect("build with extra lump should succeed");
    let wad2 = Wad::from_bytes(built).expect("extended WAD should parse");

    assert_eq!(wad2.lump_count(), 3);
    let new_lump = wad2
        .lump_by_name("NEWLUMP")
        .expect("NEWLUMP should be present");
    assert_eq!(wad2.lump_data(new_lump), b"hello world");
    // Original lumps must still be intact.
    let things = wad2
        .lump_by_name("THINGS")
        .expect("THINGS should still be present");
    assert_eq!(wad2.lump_data(things), b"\x01\x02\x03\x04");
}

/// Rebuilding a WAD from a fresh builder that omits one lump effectively
/// removes it; lumps that are retained must have their payload preserved.
#[test]
fn remove_lump_by_selective_rebuild() {
    let original = two_lump_iwad();
    let wad = Wad::from_bytes(original).expect("original should parse");

    let kind = wad.kind();
    let mut builder = WadBuilder::new(kind);
    for lump in wad.lumps() {
        if lump.name() != "LINEDEFS" {
            builder.add_lump(lump.name(), wad.lump_data(lump));
        }
    }
    let written = builder.build().expect("selective rebuild should succeed");
    let wad2 = Wad::from_bytes(written).expect("rebuilt WAD should parse");

    assert_eq!(wad2.lump_count(), 1);
    assert!(
        wad2.lump_by_name("LINEDEFS").is_none(),
        "LINEDEFS should have been dropped"
    );
    let things = wad2
        .lump_by_name("THINGS")
        .expect("THINGS should be retained");
    assert_eq!(wad2.lump_data(things), b"\x01\x02\x03\x04");
}

/// Replacing a lump's payload: copy all lumps into the builder, overwriting the
/// data for one name, and verify the new content on re-parse.
#[test]
fn replace_lump_data_and_read_back() {
    let original = two_lump_iwad();
    let wad = Wad::from_bytes(original).expect("original should parse");

    let kind = wad.kind();
    let mut builder = WadBuilder::new(kind);
    for lump in wad.lumps() {
        let data: &[u8] = if lump.name() == "THINGS" {
            b"replaced"
        } else {
            wad.lump_data(lump)
        };
        builder.add_lump(lump.name(), data);
    }
    let written = builder.build().expect("replace build should succeed");
    let wad2 = Wad::from_bytes(written).expect("WAD with replaced lump should parse");

    assert_eq!(wad2.lump_count(), 2);
    let things = wad2
        .lump_by_name("THINGS")
        .expect("THINGS should be present");
    assert_eq!(wad2.lump_data(things), b"replaced");
    // Other lump must be unchanged.
    let linedefs = wad2
        .lump_by_name("LINEDEFS")
        .expect("LINEDEFS should be present");
    assert_eq!(wad2.lump_data(linedefs), b"\x05\x06");
}

/// A WAD kind change: load an IWAD, convert it to a PWAD, verify the magic on
/// re-parse.
#[test]
fn change_wad_kind_iwad_to_pwad() {
    let original = common::build_wad(*b"IWAD", &[("DEMO1", b"data")]);
    let wad = Wad::from_bytes(original).expect("IWAD should parse");
    assert_eq!(wad.kind(), WadKind::Iwad);

    let mut builder = WadBuilder::new(WadKind::Pwad);
    for lump in wad.lumps() {
        builder.add_lump(lump.name(), wad.lump_data(lump));
    }
    let written = builder.build().expect("PWAD build should succeed");
    let wad2 = Wad::from_bytes(written).expect("PWAD should parse");

    assert_eq!(wad2.kind(), WadKind::Pwad);
    assert_eq!(wad2.lump_count(), 1);
    assert_eq!(wad2.lump_data(&wad2.lumps()[0]), b"data");
}

/// A WAD that starts with zero lumps can have lumps added and then round-tripped.
#[test]
fn add_lumps_to_empty_wad() {
    let empty = WadBuilder::new(WadKind::Pwad)
        .build()
        .expect("empty PWAD should build");
    let wad = Wad::from_bytes(empty).expect("empty PWAD should parse");
    assert_eq!(wad.lump_count(), 0);

    let written = wad
        .to_builder()
        .add_lump("FIRST", b"aaa")
        .add_lump("SECOND", b"bbb")
        .build()
        .expect("build after add should succeed");
    let wad2 = Wad::from_bytes(written).expect("populated PWAD should parse");

    assert_eq!(wad2.lump_count(), 2);
    assert_eq!(
        wad2.lump_data(wad2.lump_by_name("FIRST").expect("FIRST")),
        b"aaa"
    );
    assert_eq!(
        wad2.lump_data(wad2.lump_by_name("SECOND").expect("SECOND")),
        b"bbb"
    );
}

/// A lump with an empty payload must survive the round-trip.
#[test]
fn marker_lump_round_trips() {
    let original = common::build_wad(*b"PWAD", &[("SS_START", b""), ("SS_END", b"")]);
    let wad = Wad::from_bytes(original).expect("marker WAD should parse");

    let written = wad
        .to_builder()
        .build()
        .expect("marker WAD round-trip should succeed");
    let wad2 = Wad::from_bytes(written).expect("round-tripped marker WAD should parse");

    assert_eq!(wad2.lump_count(), 2);
    assert_eq!(wad2.lump_data(&wad2.lumps()[0]), b"");
    assert_eq!(wad2.lump_data(&wad2.lumps()[1]), b"");
}

/// Byte-level fidelity: lump data containing every possible byte value (0x00–
/// 0xff) must be preserved exactly.
#[test]
fn binary_payload_fidelity() {
    let payload: Vec<u8> = (0u8..=255).collect();
    let original = Wad::from_bytes(
        WadBuilder::new(WadKind::Pwad)
            .add_lump("BINDATA", payload.as_slice())
            .build()
            .expect("binary payload build"),
    )
    .expect("binary payload parse");

    let written = original
        .to_builder()
        .build()
        .expect("binary payload round-trip build");
    let wad2 = Wad::from_bytes(written).expect("binary payload round-trip parse");

    let lump = wad2.lump_by_name("BINDATA").expect("BINDATA");
    assert_eq!(wad2.lump_data(lump), payload.as_slice());
}
