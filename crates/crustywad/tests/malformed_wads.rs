//! Synthetic test corpus for malformed and large WAD inputs.
//!
//! Covers six categories:
//! 1. Invalid magic bytes
//! 2. Directory offset issues
//! 3. Lump directory corruption
//! 4. Lump name edge cases
//! 5. Empty and minimal WADs
//! 6. Large WADs

mod common;

use crustywad::{ParseError, ParseOptions, ParseWarning, Wad, WadKind};

// ---------------------------------------------------------------------------
// Raw WAD builder for malformed inputs
// ---------------------------------------------------------------------------

/// Builds a raw WAD byte buffer from its constituent parts.
///
/// `extra` is appended after the 12-byte header and represents whatever
/// follows it (lump payloads, directory, or intentional garbage).
fn raw_wad(magic: [u8; 4], num_lumps: i32, infotableofs: i32, extra: &[u8]) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&magic);
    v.extend_from_slice(&num_lumps.to_le_bytes());
    v.extend_from_slice(&infotableofs.to_le_bytes());
    v.extend_from_slice(extra);
    v
}

/// Builds a single raw directory entry (16 bytes).
fn dir_entry(filepos: i32, size: i32, name: [u8; 8]) -> [u8; 16] {
    let mut entry = [0u8; 16];
    entry[0..4].copy_from_slice(&filepos.to_le_bytes());
    entry[4..8].copy_from_slice(&size.to_le_bytes());
    entry[8..16].copy_from_slice(&name);
    entry
}

// ---------------------------------------------------------------------------
// Category 1: Invalid magic bytes
// ---------------------------------------------------------------------------

#[test]
fn strict_rejects_unknown_magic_xwad() {
    let bytes = raw_wad(*b"XWAD", 0, 12, &[]);
    let err = Wad::from_bytes(bytes).expect_err("unknown magic should fail in strict mode");
    assert!(matches!(err, ParseError::InvalidMagic { .. }));
}

#[test]
fn lenient_warns_and_sets_unknown_kind_for_xwad() {
    let bytes = raw_wad(*b"XWAD", 0, 12, &[]);
    let wad = Wad::from_bytes_with_options(bytes, ParseOptions::lenient())
        .expect("lenient should recover from unknown magic");
    assert!(matches!(wad.kind(), WadKind::Unknown(_)));
    assert!(
        wad.warnings()
            .iter()
            .any(|w| matches!(w, ParseWarning::InvalidMagic(_)))
    );
}

#[test]
fn strict_rejects_zero_magic() {
    let bytes = raw_wad(*b"\0\0\0\0", 0, 12, &[]);
    let err = Wad::from_bytes(bytes).expect_err("zero magic should fail in strict mode");
    assert!(matches!(err, ParseError::InvalidMagic { .. }));
}

#[test]
fn lenient_warns_and_sets_unknown_kind_for_zero_magic() {
    let bytes = raw_wad(*b"\0\0\0\0", 0, 12, &[]);
    let wad = Wad::from_bytes_with_options(bytes, ParseOptions::lenient())
        .expect("lenient should recover from zero magic");
    assert!(matches!(wad.kind(), WadKind::Unknown([0, 0, 0, 0])));
    assert!(
        wad.warnings()
            .iter()
            .any(|w| matches!(w, ParseWarning::InvalidMagic(_)))
    );
}

#[test]
fn strict_rejects_truncated_to_two_bytes() {
    // Only 2 bytes — header read will fail before magic is checked.
    let bytes = b"IW";
    let err = Wad::from_bytes(bytes.to_vec()).expect_err("truncated input should fail");
    assert!(matches!(err, ParseError::Header(_)));
}

#[test]
fn strict_rejects_truncated_to_four_bytes_bad_magic() {
    // Exactly 4 bytes — magic can be read but header (12 bytes) will fail.
    let bytes = b"IWAD";
    let err = Wad::from_bytes(bytes.to_vec()).expect_err("truncated IWAD header should fail");
    assert!(matches!(err, ParseError::Header(_)));
}

// ---------------------------------------------------------------------------
// Category 2: Directory offset issues
// ---------------------------------------------------------------------------

#[test]
fn strict_rejects_infotableofs_beyond_eof() {
    // 12-byte header only, infotableofs = 9999 → past end of file.
    let bytes = raw_wad(*b"IWAD", 1, 9999, &[]);
    let err = Wad::from_bytes(bytes).expect_err("infotableofs beyond EOF should fail");
    assert!(matches!(
        err,
        ParseError::OutOfBounds {
            field: "directory",
            ..
        }
    ));
}

#[test]
fn lenient_recovers_infotableofs_beyond_eof() {
    let bytes = raw_wad(*b"IWAD", 1, 9999, &[]);
    let wad = Wad::from_bytes_with_options(bytes, ParseOptions::lenient())
        .expect("lenient should recover from out-of-bounds directory");
    assert_eq!(wad.lump_count(), 0);
    assert!(wad.warnings().iter().any(|w| matches!(
        w,
        ParseWarning::OutOfBounds {
            field: "directory",
            ..
        }
    )));
}

#[test]
fn strict_rejects_infotableofs_into_lump_payload() {
    // Build a valid PWAD with one lump, then corrupt infotableofs to point
    // into the lump payload (byte 13 — middle of the 4-byte payload).
    let mut bytes = common::build_wad(*b"PWAD", &[("TEST", &[0xAA, 0xBB, 0xCC, 0xDD])]);
    // infotableofs lives at bytes 8..12; point it to offset 13 (inside payload).
    bytes[8..12].copy_from_slice(&13_i32.to_le_bytes());
    // num_lumps stays 1 → directory at offset 13, size 16 → extends to 29,
    // but the WAD is only 12 + 4 + 16 = 32 bytes. The parser should
    // detect the directory partially or fully overlaps data.
    let err = Wad::from_bytes(bytes).expect_err("directory into payload should fail");
    // Could be OutOfBounds or a lump validation error.
    assert!(matches!(
        err,
        ParseError::OutOfBounds { .. } | ParseError::NonAsciiName { .. }
    ));
}

#[test]
fn strict_rejects_infotableofs_zero_pointing_to_header() {
    // infotableofs = 0 makes the directory overlap the header itself.
    // num_lumps = 1 → needs 16 bytes from offset 0 → into header.
    let bytes = raw_wad(*b"IWAD", 1, 0, &[]);
    // With only 12 bytes and directory at 0, size 16 → OOB.
    let err = Wad::from_bytes(bytes).expect_err("directory at offset 0 should fail");
    assert!(matches!(err, ParseError::OutOfBounds { .. }));
}

#[test]
fn lenient_recovers_infotableofs_zero() {
    // infotableofs = 0, num_lumps = 0 → empty directory, valid in lenient mode.
    let bytes = raw_wad(*b"IWAD", 0, 0, &[]);
    let wad = Wad::from_bytes_with_options(bytes, ParseOptions::lenient())
        .expect("lenient should handle infotableofs=0 with 0 lumps");
    assert_eq!(wad.lump_count(), 0);
}

// ---------------------------------------------------------------------------
// Category 3: Lump directory corruption
// ---------------------------------------------------------------------------

#[test]
fn strict_rejects_num_lumps_exceeding_available_entries() {
    // Build a valid 1-lump WAD then claim there are 100 lumps.
    let mut bytes = common::build_wad(*b"IWAD", &[("FLAT", &[1, 2, 3, 4])]);
    bytes[4..8].copy_from_slice(&100_i32.to_le_bytes());
    let err = Wad::from_bytes(bytes).expect_err("num_lumps > available entries should fail");
    assert!(matches!(err, ParseError::OutOfBounds { .. }));
}

#[test]
fn lenient_clamps_num_lumps_exceeding_available_entries() {
    let mut bytes = common::build_wad(*b"IWAD", &[("FLAT", &[1, 2, 3, 4])]);
    bytes[4..8].copy_from_slice(&100_i32.to_le_bytes());
    let wad = Wad::from_bytes_with_options(bytes, ParseOptions::lenient())
        .expect("lenient should clamp oversized num_lumps");
    // Only 1 complete 16-byte directory entry exists.
    assert_eq!(wad.lump_count(), 1);
    assert!(wad.warnings().iter().any(|w| matches!(
        w,
        ParseWarning::OutOfBounds {
            field: "directory",
            ..
        }
    )));
}

#[test]
fn strict_rejects_lump_filepos_beyond_eof() {
    // 1-lump WAD; set filepos to a value beyond the file length.
    let mut bytes = common::build_wad(*b"IWAD", &[("DATA", &[0xFF])]);
    // filepos is at bytes[directory_start .. directory_start+4].
    // Directory starts at offset 13 (12 header + 1 byte payload).
    let dir_start = 13_usize;
    bytes[dir_start..dir_start + 4].copy_from_slice(&99999_i32.to_le_bytes());
    let err = Wad::from_bytes(bytes).expect_err("filepos beyond EOF should fail");
    assert!(matches!(err, ParseError::OutOfBounds { .. }));
}

#[test]
fn lenient_clamps_lump_filepos_beyond_eof() {
    let mut bytes = common::build_wad(*b"IWAD", &[("DATA", &[0xFF])]);
    let dir_start = 13_usize;
    bytes[dir_start..dir_start + 4].copy_from_slice(&99999_i32.to_le_bytes());
    let wad = Wad::from_bytes_with_options(bytes, ParseOptions::lenient())
        .expect("lenient should clamp lump beyond EOF");
    assert_eq!(wad.lump_bytes(0), Some(&[][..]));
    assert!(wad.warnings().iter().any(|w| matches!(
        w,
        ParseWarning::OutOfBounds {
            field: "lump data",
            ..
        }
    )));
}

#[test]
fn strict_rejects_lump_size_extending_beyond_eof() {
    // filepos is valid but size is large enough to extend past EOF.
    let mut bytes = common::build_wad(*b"IWAD", &[("DATA", &[1, 2, 3])]);
    let dir_start = 15_usize;
    // Write a size of 9999 (well past end of file).
    bytes[dir_start + 4..dir_start + 8].copy_from_slice(&9999_i32.to_le_bytes());
    let err = Wad::from_bytes(bytes).expect_err("oversized lump should fail");
    assert!(matches!(err, ParseError::OutOfBounds { .. }));
}

#[test]
fn lenient_clamps_lump_size_extending_beyond_eof() {
    let mut bytes = common::build_wad(*b"IWAD", &[("DATA", &[1, 2, 3])]);
    let dir_start = 15_usize;
    bytes[dir_start + 4..dir_start + 8].copy_from_slice(&9999_i32.to_le_bytes());
    let wad = Wad::from_bytes_with_options(bytes, ParseOptions::lenient())
        .expect("lenient should clamp oversized lump size");
    // The clamped data should be the original 3 bytes.
    assert_eq!(wad.lump_bytes(0), Some(&[1, 2, 3][..]));
    assert!(wad.warnings().iter().any(|w| matches!(
        w,
        ParseWarning::OutOfBounds {
            field: "lump data",
            ..
        }
    )));
}

#[test]
fn strict_rejects_huge_filepos_and_size_out_of_bounds() {
    // filepos and size near i32::MAX — both are out of bounds on any target.
    // We use a raw WAD with a crafted directory entry.
    let lump_payload = [0xAB_u8; 4];
    let lump_name = b"OVERFLOW";
    // Place directory at offset 16 (after 12-byte header + 4-byte payload).
    let entry = dir_entry(i32::MAX - 2, i32::MAX - 2, *lump_name);
    let mut extra = Vec::new();
    extra.extend_from_slice(&lump_payload);
    extra.extend_from_slice(&entry);
    let bytes = raw_wad(*b"IWAD", 1, 16, &extra);
    let err = Wad::from_bytes(bytes).expect_err("overflow of filepos+size should fail");
    assert!(matches!(
        err,
        ParseError::OutOfBounds { .. } | ParseError::Overflow { .. }
    ));
}

#[test]
fn lenient_handles_negative_lump_size() {
    // Negative size is stored as a large unsigned value in little-endian;
    // the parser reads i32 and must handle it gracefully.
    let mut bytes = common::build_wad(*b"IWAD", &[("SNEG", &[1, 2, 3])]);
    let dir_start = 15_usize;
    bytes[dir_start + 4..dir_start + 8].copy_from_slice(&(-5_i32).to_le_bytes());
    let wad = Wad::from_bytes_with_options(bytes, ParseOptions::lenient())
        .expect("lenient should handle negative lump size");
    assert_eq!(wad.lump_bytes(0), Some(&[][..]));
    assert!(wad.warnings().iter().any(|w| matches!(
        w,
        ParseWarning::NegativeValue {
            field: "size",
            value: -5
        }
    )));
}

#[test]
fn strict_rejects_negative_lump_size() {
    let mut bytes = common::build_wad(*b"IWAD", &[("SNEG", &[1, 2, 3])]);
    let dir_start = 15_usize;
    bytes[dir_start + 4..dir_start + 8].copy_from_slice(&(-5_i32).to_le_bytes());
    let err = Wad::from_bytes(bytes).expect_err("negative lump size should fail in strict mode");
    assert!(matches!(
        err,
        ParseError::NegativeValue {
            field: "size",
            value: -5
        }
    ));
}

// ---------------------------------------------------------------------------
// Category 4: Lump name edge cases
// ---------------------------------------------------------------------------

#[test]
fn strict_rejects_non_ascii_lump_name() {
    let mut bytes = common::build_wad(*b"IWAD", &[("GOOD", &[0xDE, 0xAD])]);
    // Overwrite first byte of lump name with a non-ASCII byte.
    let name_offset = bytes.len() - 8;
    bytes[name_offset] = 0xFE;
    let err = Wad::from_bytes(bytes).expect_err("non-ASCII name should fail in strict mode");
    assert!(matches!(err, ParseError::NonAsciiName { index: 0 }));
}

#[test]
fn lenient_warns_on_non_ascii_lump_name() {
    let mut bytes = common::build_wad(*b"IWAD", &[("GOOD", &[0xDE, 0xAD])]);
    let name_offset = bytes.len() - 8;
    bytes[name_offset] = 0xFE;
    let wad = Wad::from_bytes_with_options(bytes, ParseOptions::lenient())
        .expect("lenient should decode non-ASCII name lossily");
    assert!(
        wad.warnings()
            .iter()
            .any(|w| matches!(w, ParseWarning::NonAsciiName { index: 0 }))
    );
    assert_eq!(wad.lump_count(), 1);
}

#[test]
fn parses_lump_name_with_null_in_middle() {
    // null bytes in positions 4-7 are valid — the name is null-terminated.
    // "ABC\0\0\0\0\0" → name = "ABC"
    let mut bytes = common::build_wad(*b"IWAD", &[("ABCDEFGH", &[7, 8, 9])]);
    // Inject a null at position 3 of the 8-byte name field.
    let name_offset = bytes.len() - 8;
    bytes[name_offset + 3] = 0x00;
    let wad = Wad::from_bytes(bytes).expect("null-terminated name should parse");
    assert_eq!(wad.lump(0).expect("missing lump").name(), "ABC");
}

#[test]
fn parses_all_null_lump_name_as_marker() {
    // All-null name → virtual / marker lump with empty name string.
    let mut bytes = common::build_wad(*b"IWAD", &[("MARKER", &[])]);
    let name_offset = bytes.len() - 8;
    bytes[name_offset..name_offset + 8].fill(0x00);
    let wad = Wad::from_bytes(bytes).expect("all-null name should parse as marker");
    assert_eq!(wad.lump(0).expect("missing lump").name(), "");
}

// ---------------------------------------------------------------------------
// Category 5: Empty and minimal WADs
// ---------------------------------------------------------------------------

#[test]
fn zero_lump_iwad_parses_successfully() {
    let bytes = common::build_wad(*b"IWAD", &[]);
    let wad = Wad::from_bytes(bytes).expect("zero-lump IWAD should parse");
    assert_eq!(wad.kind(), WadKind::Iwad);
    assert_eq!(wad.lump_count(), 0);
    assert!(wad.warnings().is_empty());
}

#[test]
fn zero_lump_pwad_parses_successfully() {
    let bytes = common::build_wad(*b"PWAD", &[]);
    let wad = Wad::from_bytes(bytes).expect("zero-lump PWAD should parse");
    assert_eq!(wad.kind(), WadKind::Pwad);
    assert_eq!(wad.lump_count(), 0);
}

#[test]
fn single_lump_with_empty_payload_parses() {
    // A virtual lump: filepos and size are valid but payload is 0 bytes.
    let bytes = common::build_wad(*b"IWAD", &[("VIRTUAL", &[])]);
    let wad = Wad::from_bytes(bytes).expect("single virtual lump should parse");
    assert_eq!(wad.lump_count(), 1);
    assert_eq!(wad.lump(0).expect("missing lump").size(), 0);
    assert_eq!(wad.lump_bytes(0), Some(&[][..]));
}

#[test]
fn minimal_wad_exactly_12_bytes_no_lumps() {
    // Exactly 12 bytes: 4 magic + 4 num_lumps (0) + 4 infotableofs (12).
    // infotableofs points just past the header — valid for 0 lumps.
    let bytes = raw_wad(*b"IWAD", 0, 12, &[]);
    let wad = Wad::from_bytes(bytes).expect("12-byte header-only WAD should parse");
    assert_eq!(wad.lump_count(), 0);
    assert_eq!(wad.header().info_table_offset, 12);
}

#[test]
fn minimal_wad_11_bytes_fails() {
    // 11 bytes — header is incomplete (needs 12).
    let bytes = &b"IWAD\x00\x00\x00\x00\x0c\x00\x00"[..];
    let err = Wad::from_bytes(bytes.to_vec()).expect_err("11-byte WAD should fail to read header");
    assert!(matches!(err, ParseError::Header(_)));
}

// ---------------------------------------------------------------------------
// Category 6: Large WADs
// ---------------------------------------------------------------------------

#[test]
fn wad_with_1000_lumps_parses_correctly() {
    let mut lumps: Vec<(String, Vec<u8>)> = Vec::with_capacity(1000);
    for i in 0..1000_u32 {
        // WAD lump names are at most 8 ASCII chars; use a compact format.
        let name = format!("L{i:07}");
        let payload = u8::try_from(i % 256)
            .expect("i % 256 fits u8")
            .wrapping_mul(3)
            .to_le_bytes()
            .to_vec();
        lumps.push((name, payload));
    }

    let lump_refs: Vec<(&str, &[u8])> = lumps
        .iter()
        .map(|(name, data)| (name.as_str(), data.as_slice()))
        .collect();

    let bytes = common::build_wad(*b"IWAD", &lump_refs);
    let wad = Wad::from_bytes(bytes).expect("1000-lump WAD should parse");

    assert_eq!(wad.lump_count(), 1000);
    // Spot-check first, middle, and last lumps.
    assert_eq!(wad.lump(0).expect("lump 0 missing").name(), "L0000000");
    assert_eq!(wad.lump(499).expect("lump 499 missing").name(), "L0000499");
    assert_eq!(wad.lump(999).expect("lump 999 missing").name(), "L0000999");
}

#[test]
fn lump_by_name_finds_correct_lump_in_large_wad() {
    let mut lumps: Vec<(String, Vec<u8>)> = Vec::with_capacity(1000);
    for i in 0..1000_u32 {
        let name = format!("L{i:07}");
        let payload = vec![u8::try_from(i % 256).expect("i % 256 fits u8")];
        lumps.push((name, payload));
    }

    let lump_refs: Vec<(&str, &[u8])> = lumps
        .iter()
        .map(|(name, data)| (name.as_str(), data.as_slice()))
        .collect();

    let bytes = common::build_wad(*b"IWAD", &lump_refs);
    let wad = Wad::from_bytes(bytes).expect("1000-lump WAD should parse");

    let needle = wad.lump_by_name("L0000742").expect("L0000742 must exist");
    assert_eq!(needle.name(), "L0000742");
    // Payload is (742 % 256) = 230.
    assert_eq!(wad.lump_data(needle), &[230_u8][..]);
}

#[test]
fn single_lump_with_1mb_payload_parses() {
    const MB: usize = 1024 * 1024;
    let payload = vec![0u8; MB];
    let bytes = common::build_wad(*b"IWAD", &[("BIGFLAT", &payload)]);
    let wad = Wad::from_bytes(bytes).expect("1MB lump WAD should parse");
    assert_eq!(wad.lump_count(), 1);
    assert_eq!(wad.lump(0).expect("missing lump").size(), MB);
    let data = wad.lump_bytes(0).expect("lump bytes should be present");
    assert_eq!(data.len(), MB);
    assert!(data.iter().all(|&b| b == 0));
}

#[test]
fn large_wad_lump_bytes_are_correct() {
    // Build a WAD with 100 lumps each containing a recognizable pattern.
    let mut lumps: Vec<(String, Vec<u8>)> = Vec::with_capacity(100);
    for i in 0..100_u8 {
        let name = format!("PAT{i:05}");
        // Each lump contains 4 bytes: [i, i+1, i+2, i+3].
        let payload = vec![i, i.wrapping_add(1), i.wrapping_add(2), i.wrapping_add(3)];
        lumps.push((name, payload));
    }
    let lump_refs: Vec<(&str, &[u8])> = lumps
        .iter()
        .map(|(n, d)| (n.as_str(), d.as_slice()))
        .collect();

    let bytes = common::build_wad(*b"PWAD", &lump_refs);
    let wad = Wad::from_bytes(bytes).expect("100-lump pattern WAD should parse");

    for i in 0..100_u8 {
        let data = wad.lump_bytes(usize::from(i)).expect("lump bytes missing");
        assert_eq!(
            data,
            &[i, i.wrapping_add(1), i.wrapping_add(2), i.wrapping_add(3)][..]
        );
    }
}
