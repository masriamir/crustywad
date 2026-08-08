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
fn blockmap_block_offset_past_lump_strict_errors_lenient_discards() {
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
    let parsed = MapBlockmap::parse(&bytes, 1, Strictness::Lenient, &mut warnings).unwrap();
    assert!(
        parsed.is_none(),
        "defective blockmap is discarded, not patched"
    );
    assert_eq!(warnings.len(), 1);
    assert!(matches!(
        warnings[0],
        MapWarning::BlockmapBlockOffset {
            block: 0,
            offset: 99,
            ..
        }
    ));
}

#[test]
fn blockmap_unterminated_list_strict_errors_lenient_discards() {
    // Block 0's list starts at word 5 and the lump ends without 0xFFFF.
    let bytes = words_to_bytes(&[0, 0, 1, 1, 5, 0, 1]);
    let mut warnings = Vec::new();
    assert!(matches!(
        MapBlockmap::parse(&bytes, 2, Strictness::Strict, &mut warnings).unwrap_err(),
        MapAssembleError::UnterminatedBlockmapList { block: 0 }
    ));
    let mut warnings = Vec::new();
    let parsed = MapBlockmap::parse(&bytes, 2, Strictness::Lenient, &mut warnings).unwrap();
    assert!(
        parsed.is_none(),
        "defective blockmap is discarded, not truncated"
    );
    assert_eq!(warnings.len(), 1);
    assert!(matches!(
        warnings[0],
        MapWarning::UnterminatedBlockmapList { block: 0 }
    ));
}

#[test]
fn blockmap_dangling_linedef_strict_errors_lenient_discards() {
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
    let parsed = MapBlockmap::parse(&bytes, 2, Strictness::Lenient, &mut warnings).unwrap();
    assert!(
        parsed.is_none(),
        "defective blockmap is discarded, not patched"
    );
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
fn blockmap_aliased_invalid_lists_discard_once_with_single_warning() {
    // Two blocks alias the same invalid list ({7} with only 2 linedefs).
    // Discard happens at the FIRST defective block: exactly one warning,
    // no matter how many blocks alias the bad list — aliasing cannot
    // multiply warnings, and detection stays O(input) (the diagnostic is
    // precomputed, not re-scanned).
    let bytes = words_to_bytes(&[0, 0, 1, 2, 6, 6, 7, 0xFFFF]);
    let mut warnings = Vec::new();
    let parsed = MapBlockmap::parse(&bytes, 2, Strictness::Lenient, &mut warnings).unwrap();
    assert!(parsed.is_none());
    assert_eq!(warnings.len(), 1, "one warning for the first defect only");
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

// --- Assembly integration: the group's lumps land on the Map ---

use crustywad::Wad;
use crustywad::map::Map;

/// Encodes a Doom 8-byte name field, NUL-padded on the right.
fn name8(name: &str) -> [u8; 8] {
    let mut out = [0u8; 8];
    for (slot, byte) in out.iter_mut().zip(name.as_bytes()) {
        *slot = *byte;
    }
    out
}

/// One `THINGS` record (10 bytes): x, y (`i16`), angle/type/flags (`u16`).
fn thing_bytes(x: i16, y: i16, angle: u16, type_id: u16, flags: u16) -> Vec<u8> {
    [
        &x.to_le_bytes()[..],
        &y.to_le_bytes(),
        &angle.to_le_bytes(),
        &type_id.to_le_bytes(),
        &flags.to_le_bytes(),
    ]
    .concat()
}

/// One classic `LINEDEFS` record (14 bytes, all `u16` fields).
fn linedef_bytes(
    start_vertex: u16,
    end_vertex: u16,
    flags: u16,
    special_type: u16,
    sector_tag: u16,
    right_sidedef: u16,
    left_sidedef: u16,
) -> Vec<u8> {
    [
        start_vertex,
        end_vertex,
        flags,
        special_type,
        sector_tag,
        right_sidedef,
        left_sidedef,
    ]
    .iter()
    .flat_map(|v| v.to_le_bytes())
    .collect()
}

/// One `SIDEDEFS` record (30 bytes): offsets, three 8-byte texture names,
/// then the sector index.
fn sidedef_bytes(upper: &str, lower: &str, middle: &str, sector: u16) -> Vec<u8> {
    [
        &0i16.to_le_bytes()[..],
        &0i16.to_le_bytes(),
        &name8(upper),
        &name8(lower),
        &name8(middle),
        &sector.to_le_bytes(),
    ]
    .concat()
}

/// `VERTEXES` records (4 bytes each) from `(x, y)` pairs.
fn vertexes_bytes(points: &[(i16, i16)]) -> Vec<u8> {
    points
        .iter()
        .flat_map(|(x, y)| [x.to_le_bytes(), y.to_le_bytes()].concat())
        .collect()
}

/// One `SECTORS` record (26 bytes): heights, two 8-byte flat names, light,
/// special, and tag.
fn sector_bytes(
    floor_height: i16,
    ceiling_height: i16,
    floor_texture: &str,
    ceiling_texture: &str,
    light_level: i16,
    special_type: i16,
    tag: i16,
) -> Vec<u8> {
    [
        &floor_height.to_le_bytes()[..],
        &ceiling_height.to_le_bytes(),
        &name8(floor_texture),
        &name8(ceiling_texture),
        &light_level.to_le_bytes(),
        &special_type.to_le_bytes(),
        &tag.to_le_bytes(),
    ]
    .concat()
}

/// A one-sector classic Doom map whose group carries the given REJECT and
/// BLOCKMAP lump bytes.
fn classic_map_with(reject: &[u8], blockmap: &[u8]) -> Wad {
    let bytes = common::build_doom_map_wad_with_lumps(
        "MAP01",
        thing_bytes(32, 32, 0, 1, 7),
        linedef_bytes(0, 1, 1, 0, 0, 0, 0xffff),
        sidedef_bytes("-", "-", "STARTAN3", 0),
        vertexes_bytes(&[(0, 0), (64, 0)]),
        sector_bytes(0, 128, "FLOOR4_8", "CEIL3_5", 160, 0, 0),
        &[("REJECT", reject), ("BLOCKMAP", blockmap)],
    );
    Wad::from_bytes(bytes).expect("fixture WAD parses")
}

#[test]
fn assembly_decodes_reject_and_blockmap_from_the_group() {
    // 1 sector => 1-byte REJECT; minimal 1×1 blockmap listing linedef 0.
    let blockmap = words_to_bytes(&[0, 0, 1, 1, 5, 0, 0, 0xFFFF]);
    let wad = classic_map_with(&[0b0000_0001], &blockmap);
    let map = Map::assemble(&wad, &wad.map_group("MAP01").unwrap()).unwrap();

    let reject = map.reject().expect("REJECT decoded");
    assert_eq!(reject.is_rejected(SectorIdx(0), SectorIdx(0)), Some(true));
    let bm = map.blockmap().expect("BLOCKMAP decoded");
    assert_eq!(bm.block(0, 0), Some(&[LinedefIdx(0)][..]));
}

#[test]
fn assembly_treats_empty_lumps_as_absent() {
    // Our own writer emits zero-length REJECT/BLOCKMAP (ADR-0019 §4);
    // assembly must read that back as "not built", warning-free.
    let wad = classic_map_with(&[], &[]);
    let map = Map::assemble(&wad, &wad.map_group("MAP01").unwrap()).unwrap();
    assert!(map.reject().is_none());
    assert!(map.blockmap().is_none());
    assert!(map.warnings().is_empty());
}

#[test]
fn assembly_strict_surfaces_reject_error_lenient_recovers() {
    use crustywad::ParseOptions;
    // 1 sector needs 1 byte — an undersized table can't exist; instead use
    // a malformed BLOCKMAP (3-word header) to exercise the error plumbing.
    let wad = classic_map_with(&[], &words_to_bytes(&[0, 0, 1]));
    let group = wad.map_group("MAP01").unwrap();
    assert!(matches!(
        Map::assemble(&wad, &group).unwrap_err(),
        MapAssembleError::MalformedBlockmap { .. }
    ));
    let map = Map::assemble_with_options(&wad, &group, ParseOptions::lenient()).unwrap();
    assert!(map.blockmap().is_none());
    assert!(
        map.warnings()
            .iter()
            .any(|w| matches!(w, MapWarning::MalformedBlockmap { .. }))
    );
}

#[test]
fn doom64_nested_reject_and_blockmap_are_decoded() {
    let blockmap = words_to_bytes(&[0, 0, 1, 1, 5, 0, 0, 0xFFFF]);
    let bytes = common::build_doom64_map_wad_full(
        "MAP01",
        &[],
        &common::d64_linedef(0, 1, 0, 0, 0xffff),
        &common::d64_sidedef(0, 0, 0, 0),
        &[common::d64_vertex(0.0, 0.0), common::d64_vertex(64.0, 0.0)].concat(),
        &common::d64_sector(0, 0, [0; 5], 0),
        &common::d64_light(0, 0, 0, 0),
        &[],
        &[],
        &[],
        &[0b0000_0001], // REJECT: 1 sector => 1 byte
        &blockmap,
    );
    let wad = Wad::from_bytes(bytes).unwrap();
    let map = Map::assemble(&wad, &wad.map_group("MAP01").unwrap()).unwrap();
    assert_eq!(
        map.reject()
            .unwrap()
            .is_rejected(SectorIdx(0), SectorIdx(0)),
        Some(true)
    );
    assert_eq!(
        map.blockmap().unwrap().block(0, 0),
        Some(&[LinedefIdx(0)][..])
    );
}

#[test]
fn udmf_group_carrying_blockmap_is_decoded() {
    // A nodebuilder-compiled UDMF map may carry binary REJECT/BLOCKMAP
    // between TEXTMAP and ENDMAP; they decode like any other group's.
    let textmap = concat!(
        "namespace = \"doom\";\n",
        "vertex { x = 0; y = 0; }\n",
        "vertex { x = 64; y = 0; }\n",
        "sector { texturefloor = \"F\"; textureceiling = \"C\"; }\n",
        "sidedef { sector = 0; }\n",
        "linedef { v1 = 0; v2 = 1; sidefront = 0; }\n",
    );
    let blockmap = words_to_bytes(&[0, 0, 1, 1, 5, 0, 0, 0xFFFF]);
    let bytes = common::build_wad(
        *b"PWAD",
        &[
            ("MAP01", &[]),
            ("TEXTMAP", textmap.as_bytes()),
            ("REJECT", &[0b0000_0001]),
            ("BLOCKMAP", &blockmap),
            ("ENDMAP", &[]),
        ],
    );
    let wad = Wad::from_bytes(bytes).unwrap();
    let map = Map::assemble(&wad, &wad.map_group("MAP01").unwrap()).unwrap();
    assert_eq!(
        map.reject()
            .unwrap()
            .is_rejected(SectorIdx(0), SectorIdx(0)),
        Some(true)
    );
    assert_eq!(
        map.blockmap().unwrap().block(0, 0),
        Some(&[LinedefIdx(0)][..])
    );
}

#[test]
fn doom64_strict_surfaces_blockmap_error() {
    // A 3-word header is malformed regardless of nesting; exercises the
    // `decode_reject_blockmap` error path inside Doom 64 assembly.
    let bytes = common::build_doom64_map_wad_full(
        "MAP01",
        &[],
        &common::d64_linedef(0, 1, 0, 0, 0xffff),
        &common::d64_sidedef(0, 0, 0, 0),
        &[common::d64_vertex(0.0, 0.0), common::d64_vertex(64.0, 0.0)].concat(),
        &common::d64_sector(0, 0, [0; 5], 0),
        &common::d64_light(0, 0, 0, 0),
        &[],
        &[],
        &[],
        &[],
        &words_to_bytes(&[0, 0, 1]),
    );
    let wad = Wad::from_bytes(bytes).unwrap();
    assert!(matches!(
        Map::assemble(&wad, &wad.map_group("MAP01").unwrap()).unwrap_err(),
        MapAssembleError::MalformedBlockmap { .. }
    ));
}

#[test]
fn udmf_strict_surfaces_blockmap_error() {
    // Same malformed-header case, exercised through UDMF assembly's own
    // `decode_reject_blockmap` call site.
    let textmap = concat!(
        "namespace = \"doom\";\n",
        "vertex { x = 0; y = 0; }\n",
        "vertex { x = 64; y = 0; }\n",
        "sector { texturefloor = \"F\"; textureceiling = \"C\"; }\n",
        "sidedef { sector = 0; }\n",
        "linedef { v1 = 0; v2 = 1; sidefront = 0; }\n",
    );
    let bytes = common::build_wad(
        *b"PWAD",
        &[
            ("MAP01", &[]),
            ("TEXTMAP", textmap.as_bytes()),
            ("BLOCKMAP", &words_to_bytes(&[0, 0, 1])),
            ("ENDMAP", &[]),
        ],
    );
    let wad = Wad::from_bytes(bytes).unwrap();
    assert!(matches!(
        Map::assemble(&wad, &wad.map_group("MAP01").unwrap()).unwrap_err(),
        MapAssembleError::MalformedBlockmap { .. }
    ));
}

#[test]
fn blockmap_list_ends_exactly_at_lump_end_after_delimiter() {
    // Block 0's list is just the leading-0 delimiter, positioned as the
    // lump's very last word: after skipping it, `start == words.len()`,
    // so no terminator can possibly follow. Strict mode errors; lenient
    // mode discards the whole blockmap (#422).
    let bytes = words_to_bytes(&[0, 0, 1, 1, 5, 0]);
    let mut warnings = Vec::new();
    assert!(matches!(
        MapBlockmap::parse(&bytes, 1, Strictness::Strict, &mut warnings).unwrap_err(),
        MapAssembleError::UnterminatedBlockmapList { block: 0 }
    ));
    let mut warnings = Vec::new();
    let parsed = MapBlockmap::parse(&bytes, 1, Strictness::Lenient, &mut warnings).unwrap();
    assert!(
        parsed.is_none(),
        "defective blockmap is discarded, not patched"
    );
    assert_eq!(warnings.len(), 1);
    assert!(matches!(
        warnings[0],
        MapWarning::UnterminatedBlockmapList { block: 0 }
    ));
}

// --- Property: grid arithmetic agrees between block() and block_at() ---

use proptest::prelude::*;

proptest! {
    #[test]
    fn block_at_cell_center_agrees_with_block(
        origin_x in -8192_i16..8192,
        origin_y in -8192_i16..8192,
        columns in 1_u16..12,
        rows in 1_u16..12,
        col in 0_usize..12,
        row in 0_usize..12,
    ) {
        prop_assume!(col < usize::from(columns) && row < usize::from(rows));
        // Every block's offset points at one shared empty list (word after
        // the table), keeping the fixture O(1) regardless of grid size.
        let block_count = usize::from(columns) * usize::from(rows);
        let list_offset = u16::try_from(4 + block_count).unwrap();
        let mut w = vec![
            u16::from_le_bytes(origin_x.to_le_bytes()),
            u16::from_le_bytes(origin_y.to_le_bytes()),
            columns,
            rows,
        ];
        w.extend(std::iter::repeat_n(list_offset, block_count));
        w.push(0xFFFF);
        let bytes = words_to_bytes(&w);
        let mut warnings = Vec::new();
        let bm = MapBlockmap::parse(&bytes, 1, Strictness::Strict, &mut warnings)
            .unwrap()
            .unwrap();
        // The center of cell (col, row) in map space.
        #[allow(clippy::cast_precision_loss)] // cols/rows < 12: lossless
        let x = f64::from(origin_x) + (col as f64 + 0.5) * 128.0;
        #[allow(clippy::cast_precision_loss)] // cols/rows < 12: lossless
        let y = f64::from(origin_y) + (row as f64 + 0.5) * 128.0;
        prop_assert_eq!(bm.block_at(x, y), bm.block(col, row));
        prop_assert_eq!(bm.block(col, row), Some(&[][..]));
    }
}

#[test]
fn reject_astronomical_sector_count_answers_virtual_false_without_overflow() {
    // A standalone caller can hand parse() a sector_count whose bit indices
    // exceed usize; such bits lie beyond any storable byte and must read as
    // virtual padding rather than wrapping (debug panic / release
    // mis-index).
    let mut warnings = Vec::new();
    let reject = MapReject::parse(&[0xFF], usize::MAX / 2, Strictness::Lenient, &mut warnings)
        .unwrap()
        .unwrap();
    assert_eq!(warnings.len(), 1);
    assert_eq!(
        reject.is_rejected(SectorIdx(usize::MAX / 4), SectorIdx(1)),
        Some(false)
    );
}
