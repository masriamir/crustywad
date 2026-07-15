//! Typed REJECT/BLOCKMAP parsing (#256): standalone parsers and their
//! strict/lenient policy table, plus assembly integration.

mod common;

use crustywad::Strictness;
use crustywad::map::{MapAssembleError, MapReject, MapWarning, SectorIdx};

// --- REJECT: standalone parser (policy table, spec §Strictness) ---

#[test]
fn reject_empty_lump_is_absent_in_both_modes() {
    for s in [Strictness::Strict, Strictness::Lenient] {
        let mut warnings = Vec::new();
        let parsed = MapReject::parse(&[], 4, s, &mut warnings).unwrap();
        assert!(parsed.is_none());
        assert!(warnings.is_empty());
    }
}

#[test]
fn reject_bit_semantics_row_major_lsb_first() {
    // 2 sectors => 4 bits, 1 byte. Bit index a*n + b, LSB-first (verified
    // against Chocolate Doom P_CheckSight, Task 1 Step 0). Set only the
    // (0, 1) bit: index 0*2 + 1 = 1 => byte 0, mask 1 << 1 = 0b0000_0010.
    let mut warnings = Vec::new();
    let reject = MapReject::parse(&[0b0000_0010], 2, Strictness::Strict, &mut warnings)
        .unwrap()
        .unwrap();
    assert_eq!(reject.sector_count(), 2);
    assert_eq!(reject.is_rejected(SectorIdx(0), SectorIdx(1)), Some(true));
    assert_eq!(reject.is_rejected(SectorIdx(1), SectorIdx(0)), Some(false));
    assert_eq!(reject.is_rejected(SectorIdx(0), SectorIdx(0)), Some(false));
    assert!(warnings.is_empty());
}

#[test]
fn reject_out_of_range_sector_is_none() {
    let mut warnings = Vec::new();
    let reject = MapReject::parse(&[0], 2, Strictness::Strict, &mut warnings)
        .unwrap()
        .unwrap();
    assert_eq!(reject.is_rejected(SectorIdx(2), SectorIdx(0)), None);
    assert_eq!(reject.is_rejected(SectorIdx(0), SectorIdx(2)), None);
}

#[test]
fn reject_undersized_strict_errors() {
    // 4 sectors => 16 bits => 2 bytes expected; supply 1.
    let mut warnings = Vec::new();
    let err = MapReject::parse(&[0xFF], 4, Strictness::Strict, &mut warnings).unwrap_err();
    assert!(matches!(
        err,
        MapAssembleError::UndersizedReject {
            actual: 1,
            expected: 2,
            sectors: 4
        }
    ));
}

#[test]
fn reject_undersized_lenient_pads_virtually_and_warns() {
    // The stored table is the 1 supplied byte; bits past it read "not
    // rejected" (a deterministic choice — vanilla instead pads with garbage
    // bytes emulating its overflow bug, PadRejectArray).
    let mut warnings = Vec::new();
    let reject = MapReject::parse(&[0xFF], 4, Strictness::Lenient, &mut warnings)
        .unwrap()
        .unwrap();
    assert_eq!(warnings.len(), 1);
    assert!(matches!(
        warnings[0],
        MapWarning::UndersizedReject {
            actual: 1,
            expected: 2,
            sectors: 4
        }
    ));
    // Bit 0 (sector 0 -> 0) lives in the supplied 0xFF byte: rejected.
    assert_eq!(reject.is_rejected(SectorIdx(0), SectorIdx(0)), Some(true));
    // Bit 15 (sector 3 -> 3) lives in the missing byte: virtual zero.
    assert_eq!(reject.is_rejected(SectorIdx(3), SectorIdx(3)), Some(false));
}

#[test]
fn reject_oversized_is_accepted_and_tail_ignored_in_both_modes() {
    // 2 sectors need 1 byte; supply 8 (power-of-two padding is common in
    // the wild; vanilla reads minlength and ignores the tail).
    for s in [Strictness::Strict, Strictness::Lenient] {
        let mut warnings = Vec::new();
        let bytes = [0b0000_0001, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x11];
        let reject = MapReject::parse(&bytes, 2, s, &mut warnings)
            .unwrap()
            .unwrap();
        assert_eq!(reject.is_rejected(SectorIdx(0), SectorIdx(0)), Some(true));
        assert!(warnings.is_empty());
    }
}

// --- BLOCKMAP: standalone parser (policy table, spec §Strictness) ---

use crustywad::map::{LinedefIdx, MapBlockmap};

/// Encodes words little-endian into lump bytes.
fn words_to_bytes(words: &[u16]) -> Vec<u8> {
    words.iter().flat_map(|w| w.to_le_bytes()).collect()
}

/// A 1×2 blockmap at origin (0, 0): block 0's list is `{0}` (with the
/// conventional leading-0 delimiter), block 1 shares block 0's list via an
/// aliased offset. Word layout:
///   [0]..[3]  header: `origin_x=0`, `origin_y=0`, `columns=1`, `rows=2`
///   [4], [5]  offset table: both 6
///   [6]       0      (leading delimiter)
///   [7]       0      (linedef 0)
///   [8]       0xFFFF (terminator)
fn shared_list_blockmap() -> Vec<u8> {
    words_to_bytes(&[0, 0, 1, 2, 6, 6, 0, 0, 0xFFFF])
}

#[test]
fn blockmap_empty_lump_is_absent_in_both_modes() {
    for s in [Strictness::Strict, Strictness::Lenient] {
        let mut warnings = Vec::new();
        assert!(
            MapBlockmap::parse(&[], 1, s, &mut warnings)
                .unwrap()
                .is_none()
        );
        assert!(warnings.is_empty());
    }
}

#[test]
fn blockmap_decodes_header_grid_and_shared_lists() {
    let mut warnings = Vec::new();
    let bm = MapBlockmap::parse(
        &shared_list_blockmap(),
        1,
        Strictness::Strict,
        &mut warnings,
    )
    .unwrap()
    .unwrap();
    assert!(warnings.is_empty());
    assert_eq!(bm.origin(), (0.0, 0.0));
    assert_eq!((bm.columns(), bm.rows()), (1, 2));
    // Leading 0 is the delimiter; the list is {linedef 0}.
    assert_eq!(bm.block(0, 0), Some(&[LinedefIdx(0)][..]));
    // Aliased offset: block (0, 1) shares the identical list.
    assert_eq!(bm.block(0, 1), Some(&[LinedefIdx(0)][..]));
    assert_eq!(bm.block(1, 0), None);
    assert_eq!(bm.block(0, 2), None);
}

#[test]
fn blockmap_negative_origin_and_no_delimiter_list() {
    // origin (-64, -64); one block whose list omits the leading 0 —
    // vanilla itself never skips a word (P_BlockLinesIterator), so a list
    // starting with a nonzero linedef index is read verbatim.
    let ox = u16::from_le_bytes((-64_i16).to_le_bytes());
    let bytes = words_to_bytes(&[ox, ox, 1, 1, 5, 1, 0xFFFF]);
    let mut warnings = Vec::new();
    let bm = MapBlockmap::parse(&bytes, 2, Strictness::Strict, &mut warnings)
        .unwrap()
        .unwrap();
    assert_eq!(bm.origin(), (-64.0, -64.0));
    assert_eq!(bm.block(0, 0), Some(&[LinedefIdx(1)][..]));
}

#[test]
fn blockmap_tail_sharing_offsets_overlap() {
    // ZokumBSP-style tail sharing: block 0's list is {2, 1} (no leading-0
    // delimiter, so nothing is stripped), block 1's offset points one word
    // later, into block 0's tail: {1}. Overlapping ranges into the shared
    // arena — no copies.
    let bytes = words_to_bytes(&[0, 0, 1, 2, 6, 7, 2, 1, 0xFFFF]);
    let mut warnings = Vec::new();
    let bm = MapBlockmap::parse(&bytes, 3, Strictness::Strict, &mut warnings)
        .unwrap()
        .unwrap();
    assert_eq!(bm.block(0, 0), Some(&[LinedefIdx(2), LinedefIdx(1)][..]));
    assert_eq!(bm.block(0, 1), Some(&[LinedefIdx(1)][..]));
}

#[test]
fn blockmap_short_header_strict_errors_lenient_discards() {
    let bytes = words_to_bytes(&[0, 0, 1]);
    let mut warnings = Vec::new();
    let err = MapBlockmap::parse(&bytes, 1, Strictness::Strict, &mut warnings).unwrap_err();
    assert!(matches!(err, MapAssembleError::MalformedBlockmap { .. }));

    let mut warnings = Vec::new();
    let parsed = MapBlockmap::parse(&bytes, 1, Strictness::Lenient, &mut warnings).unwrap();
    assert!(parsed.is_none());
    assert_eq!(warnings.len(), 1);
    assert!(matches!(warnings[0], MapWarning::MalformedBlockmap { .. }));
}

#[test]
fn blockmap_nonpositive_dimensions_strict_errors_lenient_discards() {
    // columns = 0.
    let zero_cols = words_to_bytes(&[0, 0, 0, 1]);
    // rows = -1 (0xFFFF as i16).
    let neg_rows = words_to_bytes(&[0, 0, 1, 0xFFFF]);
    for bytes in [zero_cols, neg_rows] {
        let mut warnings = Vec::new();
        assert!(matches!(
            MapBlockmap::parse(&bytes, 1, Strictness::Strict, &mut warnings).unwrap_err(),
            MapAssembleError::MalformedBlockmap { .. }
        ));
        let mut warnings = Vec::new();
        assert!(
            MapBlockmap::parse(&bytes, 1, Strictness::Lenient, &mut warnings)
                .unwrap()
                .is_none()
        );
        assert_eq!(warnings.len(), 1);
    }
}

#[test]
fn blockmap_truncated_offset_table_strict_errors_lenient_discards() {
    // 2×2 grid needs 4 offsets (words 4..8); lump ends after 2.
    let bytes = words_to_bytes(&[0, 0, 2, 2, 6, 6]);
    let mut warnings = Vec::new();
    assert!(matches!(
        MapBlockmap::parse(&bytes, 1, Strictness::Strict, &mut warnings).unwrap_err(),
        MapAssembleError::MalformedBlockmap { .. }
    ));
    let mut warnings = Vec::new();
    assert!(
        MapBlockmap::parse(&bytes, 1, Strictness::Lenient, &mut warnings)
            .unwrap()
            .is_none()
    );
}

#[test]
fn blockmap_block_offset_past_lump_strict_errors_lenient_empties_block() {
    // One block whose offset (99) is outside the 6-word lump.
    let bytes = words_to_bytes(&[0, 0, 1, 1, 99, 0xFFFF]);
    let mut warnings = Vec::new();
    assert!(matches!(
        MapBlockmap::parse(&bytes, 1, Strictness::Strict, &mut warnings).unwrap_err(),
        MapAssembleError::BlockmapBlockOffset {
            block: 0,
            offset: 99,
            ..
        }
    ));
    let mut warnings = Vec::new();
    let bm = MapBlockmap::parse(&bytes, 1, Strictness::Lenient, &mut warnings)
        .unwrap()
        .unwrap();
    assert_eq!(bm.block(0, 0), Some(&[][..]));
    assert_eq!(warnings.len(), 1);
    assert!(matches!(
        warnings[0],
        MapWarning::BlockmapBlockOffset { block: 0, .. }
    ));
}

#[test]
fn blockmap_unterminated_list_strict_errors_lenient_truncates() {
    // Block 0's list starts at word 5 and the lump ends without 0xFFFF.
    let bytes = words_to_bytes(&[0, 0, 1, 1, 5, 0, 1]);
    let mut warnings = Vec::new();
    assert!(matches!(
        MapBlockmap::parse(&bytes, 2, Strictness::Strict, &mut warnings).unwrap_err(),
        MapAssembleError::UnterminatedBlockmapList { block: 0 }
    ));
    let mut warnings = Vec::new();
    let bm = MapBlockmap::parse(&bytes, 2, Strictness::Lenient, &mut warnings)
        .unwrap()
        .unwrap();
    // Leading 0 delimiter skipped; truncated list carries linedef 1.
    assert_eq!(bm.block(0, 0), Some(&[LinedefIdx(1)][..]));
    assert_eq!(warnings.len(), 1);
    assert!(matches!(
        warnings[0],
        MapWarning::UnterminatedBlockmapList { block: 0 }
    ));
}

#[test]
fn blockmap_dangling_linedef_strict_errors_lenient_empties_block() {
    // List {7} but only 2 linedefs exist.
    let bytes = words_to_bytes(&[0, 0, 1, 1, 5, 7, 0xFFFF]);
    let mut warnings = Vec::new();
    assert!(matches!(
        MapBlockmap::parse(&bytes, 2, Strictness::Strict, &mut warnings).unwrap_err(),
        MapAssembleError::DanglingReference {
            referent: "linedef",
            index: 7,
            from: "blockmap block",
            count: 2
        }
    ));
    let mut warnings = Vec::new();
    let bm = MapBlockmap::parse(&bytes, 2, Strictness::Lenient, &mut warnings)
        .unwrap()
        .unwrap();
    assert_eq!(bm.block(0, 0), Some(&[][..]));
    assert_eq!(warnings.len(), 1);
    assert!(matches!(
        warnings[0],
        MapWarning::BlockmapListDangling {
            block: 0,
            index: 7,
            count: 2
        }
    ));
}

#[test]
fn blockmap_aliased_invalid_lists_warn_per_block_and_empty_each() {
    // Two blocks alias the same invalid list ({7} with only 2 linedefs):
    // lenient mode must warn once per block, empty both, and stay O(input)
    // while doing it (the diagnostic is precomputed, not re-scanned).
    let bytes = words_to_bytes(&[0, 0, 1, 2, 6, 6, 7, 0xFFFF]);
    let mut warnings = Vec::new();
    let bm = MapBlockmap::parse(&bytes, 2, Strictness::Lenient, &mut warnings)
        .unwrap()
        .unwrap();
    assert_eq!(bm.block(0, 0), Some(&[][..]));
    assert_eq!(bm.block(0, 1), Some(&[][..]));
    assert_eq!(warnings.len(), 2);
    assert!(warnings.iter().all(|w| matches!(
        w,
        crustywad::map::MapWarning::BlockmapListDangling {
            index: 7,
            count: 2,
            ..
        }
    )));
}

#[test]
fn blockmap_block_at_maps_coordinates_through_128_unit_grid() {
    let mut warnings = Vec::new();
    let bm = MapBlockmap::parse(
        &shared_list_blockmap(),
        1,
        Strictness::Strict,
        &mut warnings,
    )
    .unwrap()
    .unwrap();
    // (0, 0) and (127.9, 127.9) land in block (0, 0); y = 128 crosses into
    // row 1; anything left of / below the origin is outside.
    assert_eq!(bm.block_at(0.0, 0.0), bm.block(0, 0));
    assert_eq!(bm.block_at(127.9, 127.9), bm.block(0, 0));
    assert_eq!(bm.block_at(0.0, 128.0), bm.block(0, 1));
    assert_eq!(bm.block_at(-0.1, 0.0), None);
    assert_eq!(bm.block_at(128.0, 0.0), None);
    assert_eq!(bm.block_at(f64::NAN, 0.0), None);
}
