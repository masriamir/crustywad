//! Integration tests for Doom 64 map-record parsing and the nested-WAD reader.

mod common;

use crustywad::map::doom64::{Light, Linedef, Sector, Sidedef, Thing, Vertex};
use crustywad::map::parse_records;

#[test]
fn parses_doom64_vertex_fixed_point() {
    // 16.16 fixed-point: 20971520 == 320.0, -65536 == -1.0.
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&20_971_520_i32.to_le_bytes());
    bytes.extend_from_slice(&(-65_536_i32).to_le_bytes());
    let verts: Vec<Vertex> = parse_records(&bytes).unwrap();
    assert_eq!(verts.len(), 1);
    assert_eq!(verts[0].x, 20_971_520);
    assert_eq!(verts[0].y, -65_536);
}

#[test]
fn parses_doom64_thing_fourteen_bytes() {
    let mut b = Vec::new();
    for v in [10_i16, 20, 24, 90, 3001, 7, 42] {
        b.extend_from_slice(&v.to_le_bytes());
    }
    assert_eq!(b.len(), 14);
    let things: Vec<Thing> = parse_records(&b).unwrap();
    let t = &things[0];
    assert_eq!(t.x, 10);
    assert_eq!(t.y, 20);
    assert_eq!(t.z, 24);
    assert_eq!(t.angle, 90);
    assert_eq!(t.type_id, 3001);
    assert_eq!(t.flags, 7);
    assert_eq!(t.id, 42);
}

#[test]
fn parses_doom64_linedef_sixteen_bytes() {
    let mut b = Vec::new();
    b.extend_from_slice(&1_u16.to_le_bytes()); // v1
    b.extend_from_slice(&2_u16.to_le_bytes()); // v2
    b.extend_from_slice(&0x0001_0004_u32.to_le_bytes()); // flags (u32)
    b.extend_from_slice(&48_u16.to_le_bytes()); // special
    b.extend_from_slice(&99_u16.to_le_bytes()); // tag
    b.extend_from_slice(&5_u16.to_le_bytes()); // sidefront
    b.extend_from_slice(&0xffff_u16.to_le_bytes()); // sideback (one-sided)
    assert_eq!(b.len(), 16);
    let lines: Vec<Linedef> = parse_records(&b).unwrap();
    let l = &lines[0];
    assert_eq!(l.v1, 1);
    assert_eq!(l.v2, 2);
    assert_eq!(l.flags, 0x0001_0004);
    assert_eq!(l.special, 48);
    assert_eq!(l.tag, 99);
    assert_eq!(l.sidefront, 5);
    assert_eq!(l.sideback, 0xffff);
}

#[test]
fn parses_doom64_sidedef_twelve_bytes() {
    let mut b = Vec::new();
    for v in [-4_i16, 8] {
        b.extend_from_slice(&v.to_le_bytes()); // x_offset, y_offset
    }
    for v in [11_u16, 22, 33, 3] {
        b.extend_from_slice(&v.to_le_bytes()); // upper, lower, middle, sector
    }
    assert_eq!(b.len(), 12);
    let sides: Vec<Sidedef> = parse_records(&b).unwrap();
    let s = &sides[0];
    assert_eq!(s.x_offset, -4);
    assert_eq!(s.y_offset, 8);
    assert_eq!(s.upper, 11);
    assert_eq!(s.lower, 22);
    assert_eq!(s.middle, 33);
    assert_eq!(s.sector, 3);
}

#[test]
fn parses_doom64_sector_twenty_four_bytes() {
    let mut b = Vec::new();
    b.extend_from_slice(&0_i16.to_le_bytes()); // floor_height
    b.extend_from_slice(&128_i16.to_le_bytes()); // ceiling_height
    b.extend_from_slice(&5_u16.to_le_bytes()); // floor_tex
    b.extend_from_slice(&6_u16.to_le_bytes()); // ceiling_tex
    for c in [100_u16, 101, 102, 103, 104] {
        b.extend_from_slice(&c.to_le_bytes()); // colors[5]
    }
    b.extend_from_slice(&9_u16.to_le_bytes()); // special
    b.extend_from_slice(&77_u16.to_le_bytes()); // tag
    b.extend_from_slice(&1_u16.to_le_bytes()); // flags
    assert_eq!(b.len(), 24);
    let secs: Vec<Sector> = parse_records(&b).unwrap();
    let s = &secs[0];
    assert_eq!(s.floor_height, 0);
    assert_eq!(s.ceiling_height, 128);
    assert_eq!(s.floor_tex, 5);
    assert_eq!(s.ceiling_tex, 6);
    assert_eq!(s.colors, [100, 101, 102, 103, 104]);
    assert_eq!(s.special, 9);
    assert_eq!(s.tag, 77);
    assert_eq!(s.flags, 1);
}

#[test]
fn parses_doom64_light_six_bytes() {
    // Measured record: r,g,b,tag: u8 then unknown: u16 LE (high byte 0 in real data).
    let b = [0x64_u8, 0x64, 0xc8, 0x02, 0x05, 0x00];
    let lights: Vec<Light> = parse_records(&b).unwrap();
    let l = &lights[0];
    assert_eq!(l.r, 0x64);
    assert_eq!(l.g, 0x64);
    assert_eq!(l.b, 0xc8);
    assert_eq!(l.tag, 0x02);
    assert_eq!(l.unknown, 5);
}

use crustywad::map::is_doom64_map_lump;

#[test]
fn detects_doom64_map_lump_by_nested_magic() {
    // Minimal 12-byte WAD header with IWAD / PWAD magic.
    let mut iwad = b"IWAD".to_vec();
    iwad.extend_from_slice(&[0u8; 8]);
    assert!(is_doom64_map_lump(&iwad));

    let mut pwad = b"PWAD".to_vec();
    pwad.extend_from_slice(&[0u8; 8]);
    assert!(is_doom64_map_lump(&pwad));

    // A classic 0-byte marker lump is not a Doom 64 map.
    assert!(!is_doom64_map_lump(&[]));
    // Too short to hold a WAD header.
    assert!(!is_doom64_map_lump(b"IWA"));
    // Right length, wrong magic.
    assert!(!is_doom64_map_lump(&[0u8; 12]));
    assert!(!is_doom64_map_lump(b"THINGS\0\0\0\0\0\0"));
}

use crustywad::ParseOptions;
use crustywad::map::{Doom64ReadError, Doom64Warning, read_doom64_map};

/// Builds a minimal Doom 64 map lump (a nested IWAD) with one record per lump.
/// Returns the bytes that a `MAPxx` lump would contain.
fn sample_doom64_map_bytes() -> Vec<u8> {
    // One record each; sizes: THINGS 14, LINEDEFS 16, SIDEDEFS 12, VERTEXES 8,
    // SEGS 12, SSECTORS 4, NODES 28, SECTORS 24, LIGHTS 6.
    let things = vec![0u8; 14];
    let linedefs = vec![0u8; 16];
    let sidedefs = vec![0u8; 12];
    let vertexes = vec![0u8; 8];
    let segs = vec![0u8; 12];
    let ssectors = vec![0u8; 4];
    let nodes = vec![0u8; 28];
    let sector_records = vec![0u8; 24];
    let lights = vec![0u8; 6];
    common::build_wad(
        *b"IWAD",
        &[
            ("MAP01", &[]),
            ("THINGS", &things),
            ("LINEDEFS", &linedefs),
            ("SIDEDEFS", &sidedefs),
            ("VERTEXES", &vertexes),
            ("SEGS", &segs),
            ("SSECTORS", &ssectors),
            ("NODES", &nodes),
            ("SECTORS", &sector_records),
            ("REJECT", &[1, 2, 3]),
            ("BLOCKMAP", &[4, 5]),
            ("LEAFS", &[6]),
            ("LIGHTS", &lights),
            ("MACROS", &[7, 8]),
        ],
    )
}

#[test]
fn reads_doom64_map_strict() {
    let bytes = sample_doom64_map_bytes();
    let map = read_doom64_map(&bytes, &ParseOptions::strict()).unwrap();
    assert_eq!(map.things.len(), 1);
    assert_eq!(map.linedefs.len(), 1);
    assert_eq!(map.sidedefs.len(), 1);
    assert_eq!(map.vertexes.len(), 1);
    assert_eq!(map.segs.len(), 1);
    assert_eq!(map.subsectors.len(), 1);
    assert_eq!(map.nodes.len(), 1);
    assert_eq!(map.sectors.len(), 1);
    assert_eq!(map.lights.len(), 1);
    assert_eq!(map.reject, vec![1, 2, 3]);
    assert_eq!(map.blockmap, vec![4, 5]);
    assert_eq!(map.leafs, vec![6]);
    assert_eq!(map.macros, vec![7, 8]);
    assert!(map.warnings().is_empty());
}

#[test]
fn non_doom64_bytes_rejected_both_modes() {
    // Data lacking the leading IWAD/PWAD magic is not a Doom 64 map lump. The
    // reader must reject it in BOTH modes (before any parsing) rather than let
    // lenient mode misread it as an empty map with missing-lump warnings.
    let junk = b"not a wad at all!!".to_vec(); // 18 bytes, no WAD magic
    assert!(matches!(
        read_doom64_map(&junk, &ParseOptions::strict()),
        Err(Doom64ReadError::NotADoom64Map)
    ));
    assert!(matches!(
        read_doom64_map(&junk, &ParseOptions::lenient()),
        Err(Doom64ReadError::NotADoom64Map)
    ));
    // A classic 0-byte map marker is likewise not a Doom 64 map lump.
    assert!(matches!(
        read_doom64_map(&[], &ParseOptions::lenient()),
        Err(Doom64ReadError::NotADoom64Map)
    ));
}

#[test]
fn valid_magic_but_corrupt_directory_errors_both_modes() {
    // Passes the magic guard (12 bytes, IWAD magic) but the directory claims 100
    // lumps at an out-of-bounds offset. The nested container is parsed strictly
    // regardless of the caller's mode, so a corrupt container errors in BOTH
    // modes rather than being recovered into an empty map with warnings.
    let mut bytes = b"IWAD".to_vec();
    bytes.extend_from_slice(&100_i32.to_le_bytes()); // num_lumps
    bytes.extend_from_slice(&4096_i32.to_le_bytes()); // directory offset, out of bounds
    assert!(matches!(
        read_doom64_map(&bytes, &ParseOptions::strict()),
        Err(Doom64ReadError::NestedWad(_))
    ));
    assert!(matches!(
        read_doom64_map(&bytes, &ParseOptions::lenient()),
        Err(Doom64ReadError::NestedWad(_))
    ));
}

#[test]
fn missing_lump_strict_errors_lenient_warns() {
    // Build a map WAD lacking LIGHTS.
    let things = vec![0u8; 14];
    let linedefs = vec![0u8; 16];
    let sidedefs = vec![0u8; 12];
    let vertexes = vec![0u8; 8];
    let segs = vec![0u8; 12];
    let ssectors = vec![0u8; 4];
    let nodes = vec![0u8; 28];
    let sector_records = vec![0u8; 24];
    let bytes = common::build_wad(
        *b"IWAD",
        &[
            ("MAP01", &[]),
            ("THINGS", &things),
            ("LINEDEFS", &linedefs),
            ("SIDEDEFS", &sidedefs),
            ("VERTEXES", &vertexes),
            ("SEGS", &segs),
            ("SSECTORS", &ssectors),
            ("NODES", &nodes),
            ("SECTORS", &sector_records),
            // LIGHTS omitted
        ],
    );
    assert!(matches!(
        read_doom64_map(&bytes, &ParseOptions::strict()),
        Err(Doom64ReadError::MissingLump { name: "LIGHTS" })
    ));
    let map = read_doom64_map(&bytes, &ParseOptions::lenient()).unwrap();
    assert!(map.lights.is_empty());
    assert_eq!(
        map.warnings(),
        &[Doom64Warning::MissingLump { name: "LIGHTS" }]
    );
}

#[test]
fn trailing_bytes_strict_errors_lenient_salvages() {
    // SECTORS lump of 24*2 + 5 bytes: two whole records + a 5-byte remainder.
    let mut sectors = vec![0u8; 48];
    sectors.extend_from_slice(&[9, 9, 9, 9, 9]);
    let build = |sectors: &[u8]| {
        common::build_wad(
            *b"IWAD",
            &[
                ("MAP01", &[]),
                ("THINGS", &[0u8; 14]),
                ("LINEDEFS", &[0u8; 16]),
                ("SIDEDEFS", &[0u8; 12]),
                ("VERTEXES", &[0u8; 8]),
                ("SEGS", &[0u8; 12]),
                ("SSECTORS", &[0u8; 4]),
                ("NODES", &[0u8; 28]),
                ("SECTORS", sectors),
                ("LIGHTS", &[0u8; 6]),
            ],
        )
    };
    let bytes = build(&sectors);
    assert!(matches!(
        read_doom64_map(&bytes, &ParseOptions::strict()),
        Err(Doom64ReadError::Records {
            lump: "SECTORS",
            ..
        })
    ));
    let map = read_doom64_map(&bytes, &ParseOptions::lenient()).unwrap();
    assert_eq!(map.sectors.len(), 2);
    assert_eq!(
        map.warnings(),
        &[Doom64Warning::TrailingBytes {
            lump: "SECTORS",
            offset: 48
        }]
    );
}

#[test]
fn reads_doom64_map_lenient_clean_no_warnings() {
    // A well-formed sample Doom 64 map read with lenient mode must produce
    // zero warnings—clean input has no issues to warn about.
    let bytes = sample_doom64_map_bytes();
    let map = read_doom64_map(&bytes, &ParseOptions::lenient()).unwrap();
    assert!(map.warnings().is_empty());
}

use proptest::prelude::*;

proptest! {
    // Doom 64 record types are plain integer records (no field validation),
    // so a buffer whose length is an exact multiple of the record size must
    // always parse successfully to exactly len / size records.

    #[test]
    fn doom64_vertex_exact_multiple_parses(
        data in proptest::collection::vec(any::<u8>(), 0..4096usize)
    ) {
        const VERTEX_SIZE: usize = 8;
        let result = parse_records::<Vertex>(&data);
        if data.len() % VERTEX_SIZE == 0 {
            prop_assert!(
                result.is_ok(),
                "parse_records::<Vertex> must succeed on {}-byte input (exact multiple of {}): got {:?}",
                data.len(), VERTEX_SIZE, result
            );
            prop_assert_eq!(result.unwrap().len(), data.len() / VERTEX_SIZE);
        }
    }

    #[test]
    fn doom64_thing_exact_multiple_parses(
        data in proptest::collection::vec(any::<u8>(), 0..4096usize)
    ) {
        const THING_SIZE: usize = 14;
        let result = parse_records::<Thing>(&data);
        if data.len() % THING_SIZE == 0 {
            prop_assert!(
                result.is_ok(),
                "parse_records::<Thing> must succeed on {}-byte input (exact multiple of {}): got {:?}",
                data.len(), THING_SIZE, result
            );
            prop_assert_eq!(result.unwrap().len(), data.len() / THING_SIZE);
        }
    }

    #[test]
    fn doom64_linedef_exact_multiple_parses(
        data in proptest::collection::vec(any::<u8>(), 0..4096usize)
    ) {
        const LINEDEF_SIZE: usize = 16;
        let result = parse_records::<Linedef>(&data);
        if data.len() % LINEDEF_SIZE == 0 {
            prop_assert!(
                result.is_ok(),
                "parse_records::<Linedef> must succeed on {}-byte input (exact multiple of {}): got {:?}",
                data.len(), LINEDEF_SIZE, result
            );
            prop_assert_eq!(result.unwrap().len(), data.len() / LINEDEF_SIZE);
        }
    }

    #[test]
    fn doom64_sidedef_exact_multiple_parses(
        data in proptest::collection::vec(any::<u8>(), 0..4096usize)
    ) {
        const SIDEDEF_SIZE: usize = 12;
        let result = parse_records::<Sidedef>(&data);
        if data.len() % SIDEDEF_SIZE == 0 {
            prop_assert!(
                result.is_ok(),
                "parse_records::<Sidedef> must succeed on {}-byte input (exact multiple of {}): got {:?}",
                data.len(), SIDEDEF_SIZE, result
            );
            prop_assert_eq!(result.unwrap().len(), data.len() / SIDEDEF_SIZE);
        }
    }

    #[test]
    fn doom64_sector_exact_multiple_parses(
        data in proptest::collection::vec(any::<u8>(), 0..4096usize)
    ) {
        const SECTOR_SIZE: usize = 24;
        let result = parse_records::<Sector>(&data);
        if data.len() % SECTOR_SIZE == 0 {
            prop_assert!(
                result.is_ok(),
                "parse_records::<Sector> must succeed on {}-byte input (exact multiple of {}): got {:?}",
                data.len(), SECTOR_SIZE, result
            );
            prop_assert_eq!(result.unwrap().len(), data.len() / SECTOR_SIZE);
        }
    }

    #[test]
    fn doom64_light_exact_multiple_parses(
        data in proptest::collection::vec(any::<u8>(), 0..4096usize)
    ) {
        const LIGHT_SIZE: usize = 6;
        let result = parse_records::<Light>(&data);
        if data.len() % LIGHT_SIZE == 0 {
            prop_assert!(
                result.is_ok(),
                "parse_records::<Light> must succeed on {}-byte input (exact multiple of {}): got {:?}",
                data.len(), LIGHT_SIZE, result
            );
            prop_assert_eq!(result.unwrap().len(), data.len() / LIGHT_SIZE);
        }
    }
}

// --- #244: LEAFS decode onto the Map graph ---

use crustywad::Wad;
use crustywad::map::{Map, MapLeaf, MapMacroAction, MapWarning, SegIdx, VertexIdx};

/// Encodes per-subsector leaf lists into LEAFS lump bytes: for each list a
/// u16 count then count × (u16 vertex, i16 seg — supplied here as its u16
/// bit pattern, so 0xFFFF is the on-disk -1 "no seg" sentinel).
fn leafs_bytes(lists: &[&[(u16, u16)]]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for list in lists {
        bytes.extend(u16::try_from(list.len()).unwrap().to_le_bytes());
        for &(vertex, seg) in *list {
            bytes.extend(vertex.to_le_bytes());
            bytes.extend(seg.to_le_bytes());
        }
    }
    bytes
}

/// A one-sector Doom 64 map with 2 subsectors, 2 segs, 2 vertexes, and the
/// given LEAFS bytes. Seg records use the classic 12-byte layout (shared by
/// Doom 64, ADR-0018): v1, v2, angle, linedef, side, offset — all u16/i16.
fn d64_map_with_leafs(leafs: &[u8]) -> Vec<u8> {
    let mut seg = Vec::new();
    for v in [0_u16, 1, 0, 0, 0, 0] {
        seg.extend_from_slice(&v.to_le_bytes());
    }
    let mut segs = seg.clone();
    segs.extend_from_slice(&seg);
    // Two subsectors of one seg each: (count, first) = (1, 0) and (1, 1).
    let mut subsectors = Vec::new();
    for v in [1_u16, 0, 1, 1] {
        subsectors.extend_from_slice(&v.to_le_bytes());
    }
    common::build_doom64_map_wad_from(
        "MAP01",
        &common::Doom64Lumps {
            linedefs: &common::d64_linedef(0, 1, 0, 0, 0xffff),
            sidedefs: &common::d64_sidedef(0, 0, 0, 0),
            vertexes: &[common::d64_vertex(0.0, 0.0), common::d64_vertex(64.0, 0.0)].concat(),
            sectors: &common::d64_sector(0, 0, [0; 5], 0),
            lights: &common::d64_light(0, 0, 0, 0),
            segs: &segs,
            subsectors: &subsectors,
            leafs,
            ..common::Doom64Lumps::default()
        },
    )
}

fn assemble_d64(bytes: Vec<u8>) -> Result<Map, crustywad::map::MapAssembleError> {
    let wad = Wad::from_bytes(bytes).unwrap();
    let group = wad.map_group("MAP01").unwrap();
    Map::assemble(&wad, &group)
}

fn assemble_d64_lenient(bytes: Vec<u8>) -> Map {
    let wad = Wad::from_bytes(bytes).unwrap();
    let group = wad.map_group("MAP01").unwrap();
    Map::assemble_with_options(&wad, &group, ParseOptions::lenient()).unwrap()
}

#[test]
fn leafs_decode_attaches_per_subsector_with_sentinel_and_fields() {
    // Subsector 0: two leaves (vertex 0 + seg 0; vertex 1 + no seg).
    // Subsector 1: one leaf (vertex 1 + seg 1).
    let leafs = leafs_bytes(&[&[(0, 0), (1, 0xFFFF)], &[(1, 1)]]);
    let map = assemble_d64(d64_map_with_leafs(&leafs)).unwrap();

    assert_eq!(map.leafs().len(), 3);
    assert_eq!(
        map.leafs()[0],
        MapLeaf {
            vertex: VertexIdx(0),
            seg: Some(SegIdx(0))
        }
    );
    assert_eq!(
        map.leafs()[1],
        MapLeaf {
            vertex: VertexIdx(1),
            seg: None
        }
    );
    assert_eq!(
        map.leafs()[2],
        MapLeaf {
            vertex: VertexIdx(1),
            seg: Some(SegIdx(1))
        }
    );
    assert_eq!(map.subsectors()[0].leafs, 0..2);
    assert_eq!(map.subsectors()[1].leafs, 2..3);
    assert!(map.warnings().is_empty());
}

#[test]
fn leafs_zero_count_record_yields_empty_range() {
    // Both subsectors present, first has zero leaves (legal per engine).
    let leafs = leafs_bytes(&[&[], &[(0, 0xFFFF)]]);
    let map = assemble_d64(d64_map_with_leafs(&leafs)).unwrap();
    assert_eq!(map.subsectors()[0].leafs, 0..0);
    assert_eq!(map.subsectors()[1].leafs, 0..1);
}

#[test]
fn leafs_empty_lump_with_zero_subsectors_is_silent() {
    // The default fixture (all lumps empty) has 0 subsectors and 0 leaves.
    let bytes = common::build_doom64_map_wad_from(
        "MAP01",
        &common::Doom64Lumps {
            linedefs: &common::d64_linedef(0, 1, 0, 0, 0xffff),
            sidedefs: &common::d64_sidedef(0, 0, 0, 0),
            vertexes: &[common::d64_vertex(0.0, 0.0), common::d64_vertex(64.0, 0.0)].concat(),
            sectors: &common::d64_sector(0, 0, [0; 5], 0),
            lights: &common::d64_light(0, 0, 0, 0),
            ..common::Doom64Lumps::default()
        },
    );
    let map = assemble_d64(bytes).unwrap();
    assert!(map.leafs().is_empty());
    assert!(map.warnings().is_empty());
}

#[test]
fn leafs_count_mismatch_strict_errors_lenient_degrades() {
    // One record, two subsectors — the engine I_Error case.
    let leafs = leafs_bytes(&[&[(0, 0xFFFF)]]);
    let err = assemble_d64(d64_map_with_leafs(&leafs)).unwrap_err();
    assert!(matches!(
        err,
        crustywad::map::MapAssembleError::LeafCountMismatch {
            leaves: 1,
            subsectors: 2
        }
    ));

    let map = assemble_d64_lenient(d64_map_with_leafs(&leafs));
    assert!(map.leafs().is_empty());
    assert!(map.subsectors().iter().all(|ss| ss.leafs == (0..0)));
    assert!(map.warnings().iter().any(|w| matches!(
        w,
        MapWarning::LeafCountMismatch {
            leaves: 1,
            subsectors: 2
        }
    )));
}

#[test]
fn leafs_truncated_record_strict_errors_lenient_degrades() {
    // Count word promises 2 entries but only 1 fits: truncated mid-record.
    let mut leafs = leafs_bytes(&[&[(0, 0xFFFF)]]);
    leafs[0] = 2; // inflate the count past the available bytes
    let err = assemble_d64(d64_map_with_leafs(&leafs)).unwrap_err();
    assert!(matches!(
        err,
        crustywad::map::MapAssembleError::MalformedLeafs { .. }
    ));

    let map = assemble_d64_lenient(d64_map_with_leafs(&leafs));
    assert!(map.leafs().is_empty());
    assert!(
        map.warnings()
            .iter()
            .any(|w| matches!(w, MapWarning::MalformedLeafs { .. }))
    );
}

#[test]
fn leafs_trailing_partial_bytes_strict_errors_lenient_degrades() {
    // A whole valid record for each subsector, then one stray byte.
    let mut leafs = leafs_bytes(&[&[(0, 0xFFFF)], &[(1, 0xFFFF)]]);
    leafs.push(0xAA);
    assert!(matches!(
        assemble_d64(d64_map_with_leafs(&leafs)).unwrap_err(),
        crustywad::map::MapAssembleError::MalformedLeafs { .. }
    ));

    let map = assemble_d64_lenient(d64_map_with_leafs(&leafs));
    assert!(map.leafs().is_empty());
    assert!(map.subsectors().iter().all(|ss| ss.leafs == (0..0)));
    assert!(
        map.warnings()
            .iter()
            .any(|w| matches!(w, MapWarning::MalformedLeafs { .. }))
    );
}

#[test]
fn leafs_dangling_vertex_strict_errors_lenient_degrades() {
    // Vertex 9 with only 2 vertexes in the map.
    let leafs = leafs_bytes(&[&[(9, 0xFFFF)], &[(0, 0xFFFF)]]);
    let err = assemble_d64(d64_map_with_leafs(&leafs)).unwrap_err();
    assert!(matches!(
        err,
        crustywad::map::MapAssembleError::DanglingReference {
            referent: "vertex",
            index: 9,
            from: "leaf",
            count: 2,
        }
    ));

    let map = assemble_d64_lenient(d64_map_with_leafs(&leafs));
    assert!(map.leafs().is_empty());
    assert!(map.subsectors().iter().all(|ss| ss.leafs == (0..0)));
    assert!(map.warnings().iter().any(|w| matches!(
        w,
        MapWarning::LeafsDangling {
            referent: "vertex",
            index: 9,
            count: 2
        }
    )));
}

#[test]
fn leafs_dangling_seg_strict_errors_lenient_degrades() {
    // Seg 7 (not the 0xFFFF sentinel) with only 2 segs in the map.
    let leafs = leafs_bytes(&[&[(0, 7)], &[(0, 0xFFFF)]]);
    let err = assemble_d64(d64_map_with_leafs(&leafs)).unwrap_err();
    assert!(matches!(
        err,
        crustywad::map::MapAssembleError::DanglingReference {
            referent: "seg",
            index: 7,
            from: "leaf",
            count: 2,
        }
    ));

    let map = assemble_d64_lenient(d64_map_with_leafs(&leafs));
    assert!(map.leafs().is_empty());
    assert!(map.subsectors().iter().all(|ss| ss.leafs == (0..0)));
    assert!(map.warnings().iter().any(|w| matches!(
        w,
        MapWarning::LeafsDangling {
            referent: "seg",
            index: 7,
            count: 2
        }
    )));
}

#[test]
fn non_doom64_maps_expose_empty_leafs() {
    // A fresh minimal classic-map fixture: its Map::leafs() must be empty
    // and every subsector range 0..0.
    let bytes = common::build_doom_map_wad("MAP01", vec![], vec![], vec![], vec![], vec![]);
    let wad = Wad::from_bytes(bytes).unwrap();
    let map = Map::assemble(&wad, &wad.map_group("MAP01").unwrap()).unwrap();
    assert!(map.leafs().is_empty());
    assert!(map.subsectors().iter().all(|ss| ss.leafs == (0..0)));
}

proptest! {
    #[test]
    fn leafs_roundtrip_arbitrary_valid_lists(
        lists in proptest::collection::vec(
            proptest::collection::vec((0_u16..2, proptest::bool::ANY), 0..4),
            2..=2,
        )
    ) {
        // Exactly 2 subsectors (matching the fixture); vertex < 2 (the
        // fixture's vertex count); seg is either the sentinel or seg 0.
        let encoded: Vec<Vec<(u16, u16)>> = lists
            .iter()
            .map(|l| l.iter().map(|&(v, s)| (v, if s { 0 } else { 0xFFFF })).collect())
            .collect();
        let borrowed: Vec<&[(u16, u16)]> = encoded.iter().map(Vec::as_slice).collect();
        let bytes = leafs_bytes(&borrowed);
        let map = assemble_d64(d64_map_with_leafs(&bytes)).unwrap();

        let total: usize = encoded.iter().map(Vec::len).sum();
        prop_assert_eq!(map.leafs().len(), total);
        let mut cursor = 0_usize;
        for (i, list) in encoded.iter().enumerate() {
            let range = map.subsectors()[i].leafs.clone();
            prop_assert_eq!(range.clone(), cursor..cursor + list.len());
            for (leaf, &(v, s)) in map.leafs()[range].iter().zip(list) {
                prop_assert_eq!(leaf.vertex, VertexIdx(usize::from(v)));
                prop_assert_eq!(leaf.seg, if s == 0xFFFF { None } else { Some(SegIdx(0)) });
            }
            cursor += list.len();
        }
    }
}

#[test]
fn leafs_surplus_records_report_full_count_without_decoding_extras() {
    // Five records against two subsectors: the walk stops decoding entries
    // once the ranges vec is full (engine two-pass parity — count first,
    // then load) but still reports the FULL record count in the mismatch.
    // The third record carries a dangling vertex that must NOT surface:
    // count parity wins, as in P_LoadLeafs's separate counting pass.
    let leafs = leafs_bytes(&[
        &[(0, 0xFFFF)],
        &[(1, 0xFFFF)],
        &[(9, 0xFFFF)], // dangling vertex in a surplus record — skipped
        &[],
        &[],
    ]);
    let err = assemble_d64(d64_map_with_leafs(&leafs)).unwrap_err();
    assert!(matches!(
        err,
        crustywad::map::MapAssembleError::LeafCountMismatch {
            leaves: 5,
            subsectors: 2
        }
    ));

    let map = assemble_d64_lenient(d64_map_with_leafs(&leafs));
    assert!(map.leafs().is_empty());
    assert!(map.subsectors().iter().all(|ss| ss.leafs == (0..0)));
    assert!(map.warnings().iter().any(|w| matches!(
        w,
        MapWarning::LeafCountMismatch {
            leaves: 5,
            subsectors: 2
        }
    )));
}

// --- #245: MACROS decode onto the Map graph ---

/// Encodes macro defs into `MACROS` lump bytes. Each def is the FULL entry
/// list the engine reads (count + 1 entries, `P_LoadMacros`); the on-disk
/// count field is written as one less than the list length. Header
/// specialcount is written as 0 (semantics unestablished; raw-layer only).
fn macros_bytes(defs: &[&[(i16, i16, i16)]]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend(i16::try_from(defs.len()).unwrap().to_le_bytes());
    bytes.extend(0_i16.to_le_bytes());
    for actions in defs {
        assert!(!actions.is_empty(), "engine reads count + 1 >= 1 entries");
        bytes.extend(i16::try_from(actions.len() - 1).unwrap().to_le_bytes());
        for &(id, tag, special) in *actions {
            bytes.extend(id.to_le_bytes());
            bytes.extend(tag.to_le_bytes());
            bytes.extend(special.to_le_bytes());
        }
    }
    bytes
}

/// A minimal Doom 64 map (no BSP, no leaves) carrying the given MACROS
/// bytes.
fn d64_map_with_macros(macros: &[u8]) -> Vec<u8> {
    common::build_doom64_map_wad_from(
        "MAP01",
        &common::Doom64Lumps {
            linedefs: &common::d64_linedef(0, 1, 0, 0, 0xffff),
            sidedefs: &common::d64_sidedef(0, 0, 0, 0),
            vertexes: &[common::d64_vertex(0.0, 0.0), common::d64_vertex(64.0, 0.0)].concat(),
            sectors: &common::d64_sector(0, 0, [0; 5], 0),
            lights: &common::d64_light(0, 0, 0, 0),
            macros,
            ..common::Doom64Lumps::default()
        },
    )
}

#[test]
fn macros_decode_pins_count_plus_one_and_fields() {
    // Two macros. The engine reads count + 1 actions per macro
    // (P_LoadMacros: `count = macros.def[i].count + 1`), so the on-disk
    // counts here are 1 and 2 while the decoded action lists hold 2 and 3.
    let lump = macros_bytes(&[
        &[(202, 1, 5), (0, 0, 0)],
        &[(100, 2, 8), (101, 2, 9), (0, 0, 0)],
    ]);
    let map = assemble_d64(d64_map_with_macros(&lump)).unwrap();

    assert_eq!(map.macros().len(), 2);
    assert_eq!(map.macros()[0].actions.len(), 2);
    assert_eq!(
        map.macros()[0].actions[0],
        MapMacroAction {
            id: 202,
            tag: 1,
            special: 5
        }
    );
    assert_eq!(
        map.macros()[0].actions[1],
        MapMacroAction {
            id: 0,
            tag: 0,
            special: 0
        }
    );
    assert_eq!(map.macros()[1].actions.len(), 3);
    assert_eq!(
        map.macros()[1].actions[1],
        MapMacroAction {
            id: 101,
            tag: 2,
            special: 9
        }
    );
    assert!(map.warnings().is_empty());
}

#[test]
fn macros_empty_lump_is_absent_and_silent() {
    // The default fixture carries an empty MACROS lump ("not built").
    let lump: &[u8] = &[];
    let map = assemble_d64(d64_map_with_macros(lump)).unwrap();
    assert!(map.macros().is_empty());
    assert!(map.warnings().is_empty());
}

#[test]
fn macros_zero_count_header_only_is_valid_and_silent() {
    // macrocount == 0 with exactly the 4-byte header: legal, empty.
    let map = assemble_d64(d64_map_with_macros(&macros_bytes(&[]))).unwrap();
    assert!(map.macros().is_empty());
    assert!(map.warnings().is_empty());
}

#[test]
fn macros_short_lump_strict_errors_lenient_degrades() {
    // 2 bytes: non-empty but shorter than the header. The engine silently
    // swallows this (its own `TODO - fixme`); we deliberately do not.
    let lump: &[u8] = &[0x01, 0x00];
    assert!(matches!(
        assemble_d64(d64_map_with_macros(lump)).unwrap_err(),
        crustywad::map::MapAssembleError::MalformedMacros { .. }
    ));
    let map = assemble_d64_lenient(d64_map_with_macros(lump));
    assert!(map.macros().is_empty());
    assert!(
        map.warnings()
            .iter()
            .any(|w| matches!(w, MapWarning::MalformedMacros { .. }))
    );
}

#[test]
fn macros_negative_counts_strict_error_lenient_degrade() {
    // Negative macrocount (-1) in the header.
    let mut neg_macrocount = Vec::new();
    neg_macrocount.extend((-1_i16).to_le_bytes());
    neg_macrocount.extend(0_i16.to_le_bytes());
    // Negative per-macro action count (-2): valid header claiming 1 macro.
    let mut neg_action_count = Vec::new();
    neg_action_count.extend(1_i16.to_le_bytes());
    neg_action_count.extend(0_i16.to_le_bytes());
    neg_action_count.extend((-2_i16).to_le_bytes());
    for lump in [neg_macrocount, neg_action_count] {
        assert!(matches!(
            assemble_d64(d64_map_with_macros(&lump)).unwrap_err(),
            crustywad::map::MapAssembleError::MalformedMacros { .. }
        ));
        let map = assemble_d64_lenient(d64_map_with_macros(&lump));
        assert!(map.macros().is_empty());
        assert_eq!(
            map.warnings()
                .iter()
                .filter(|w| matches!(w, MapWarning::MalformedMacros { .. }))
                .count(),
            1
        );
    }
}

#[test]
fn macros_truncated_strict_error_lenient_degrade() {
    // Header promises 2 macros; only one complete record present.
    let mut lump = macros_bytes(&[&[(202, 1, 5), (0, 0, 0)]]);
    lump[0] = 2; // inflate macrocount past the available records
    assert!(matches!(
        assemble_d64(d64_map_with_macros(&lump)).unwrap_err(),
        crustywad::map::MapAssembleError::MalformedMacros { .. }
    ));
    // A macro whose promised actions run past the lump end.
    let mut short_actions = macros_bytes(&[&[(202, 1, 5), (0, 0, 0)]]);
    short_actions[4] = 5; // count word: promises 6 actions, only 2 present
    assert!(matches!(
        assemble_d64(d64_map_with_macros(&short_actions)).unwrap_err(),
        crustywad::map::MapAssembleError::MalformedMacros { .. }
    ));
    let map = assemble_d64_lenient(d64_map_with_macros(&short_actions));
    assert!(map.macros().is_empty());
}

#[test]
fn macros_trailing_bytes_strict_error_lenient_degrade() {
    // A complete, valid lump plus one stray byte: exact consumption fails.
    let mut lump = macros_bytes(&[&[(202, 1, 5), (0, 0, 0)]]);
    lump.push(0xAA);
    assert!(matches!(
        assemble_d64(d64_map_with_macros(&lump)).unwrap_err(),
        crustywad::map::MapAssembleError::MalformedMacros { .. }
    ));
    let map = assemble_d64_lenient(d64_map_with_macros(&lump));
    assert!(map.macros().is_empty());
    assert_eq!(
        map.warnings()
            .iter()
            .filter(|w| matches!(w, MapWarning::MalformedMacros { .. }))
            .count(),
        1
    );
}

#[test]
fn non_doom64_maps_expose_empty_macros() {
    let bytes = common::build_doom_map_wad("MAP01", vec![], vec![], vec![], vec![], vec![]);
    let wad = Wad::from_bytes(bytes).unwrap();
    let map = Map::assemble(&wad, &wad.map_group("MAP01").unwrap()).unwrap();
    assert!(map.macros().is_empty());
}
