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
