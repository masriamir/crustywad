//! Public-API tests for the `nodebuild` node-lump builders (ADR-0024 §9.1).
//!
//! Task 1 covers the zero-fill REJECT builder: assemble a real WAD through the
//! public path, build its REJECT, and round-trip the bytes back through
//! [`MapReject::parse`] (ADR-0024 §7 / Global Constraint 4).
#![cfg(feature = "nodebuild")]

mod common;

use crustywad::map::build::build_reject;
use crustywad::map::{Map, MapReject, MapWarning};
use crustywad::{Strictness, Wad};

/// Encodes a Doom 8-byte name field, NUL-padded on the right.
fn name8(name: &str) -> [u8; 8] {
    let mut out = [0u8; 8];
    for (slot, byte) in out.iter_mut().zip(name.as_bytes()) {
        *slot = *byte;
    }
    out
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

/// One `SIDEDEFS` record (30 bytes): offsets, three 8-byte texture names, then
/// the sector index.
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

/// `VERTEXES` records (4 bytes each) from `(x, y)` pairs.
fn vertexes_bytes(points: &[(i16, i16)]) -> Vec<u8> {
    points
        .iter()
        .flat_map(|(x, y)| [x.to_le_bytes(), y.to_le_bytes()].concat())
        .collect()
}

/// One `SECTORS` record (26 bytes): heights, two 8-byte flat names, light,
/// special, tag.
fn sector_bytes(tag: i16) -> Vec<u8> {
    [
        &0i16.to_le_bytes()[..],
        &128i16.to_le_bytes(),
        &name8("FLOOR4_8"),
        &name8("CEIL3_5"),
        &160i16.to_le_bytes(),
        &0i16.to_le_bytes(),
        &tag.to_le_bytes(),
    ]
    .concat()
}

/// Assembles a one-linedef classic Doom map carrying `n` sectors. Only sector 0
/// is referenced by a sidedef; the rest are unreferenced (valid — assembly does
/// not require every sector to be used), so `map.sectors().len() == n`.
fn map_with_sectors(n: usize) -> Map {
    let mut sectors = Vec::new();
    for i in 0..n {
        sectors.extend(sector_bytes(i16::try_from(i).unwrap()));
    }
    let bytes = common::build_doom_map_wad(
        "MAP01",
        thing_bytes(32, 32, 0, 1, 7),
        linedef_bytes(0, 1, 1, 0, 0, 0, 0xffff),
        sidedef_bytes("-", "-", "STARTAN3", 0),
        vertexes_bytes(&[(0, 0), (64, 0)]),
        sectors,
    );
    let wad = Wad::from_bytes(bytes).expect("fixture WAD parses");
    let group = wad.map_group("MAP01").expect("map group present");
    Map::assemble(&wad, &group).expect("map assembles")
}

#[test]
fn build_reject_sizes_match_the_sector_count() {
    // ceil(n² / 8): 1 -> 1 byte, 3 -> 2 bytes, 8 -> 8 bytes.
    assert_eq!(build_reject(&map_with_sectors(1)).to_lump_bytes().len(), 1);
    assert_eq!(build_reject(&map_with_sectors(3)).to_lump_bytes().len(), 2);
    assert_eq!(build_reject(&map_with_sectors(8)).to_lump_bytes().len(), 8);
}

#[test]
fn build_reject_three_sectors_is_two_zero_bytes() {
    let map = map_with_sectors(3);
    let reject = build_reject(&map);
    assert_eq!(reject.sector_count(), 3);
    assert_eq!(reject.to_lump_bytes(), vec![0u8, 0u8]);
}

#[test]
fn build_reject_round_trips_through_parse_strict() {
    // ADR-0024 §7 / Global Constraint 4: the built bytes re-parse against the
    // owning sector count into an exact copy, warning-free, in strict mode.
    for n in [1usize, 3, 8] {
        let map = map_with_sectors(n);
        let built = build_reject(&map);
        let mut warnings: Vec<MapWarning> = Vec::new();
        let parsed = MapReject::parse(&built.to_lump_bytes(), n, Strictness::Strict, &mut warnings)
            .expect("built REJECT parses")
            .expect("built REJECT is present");
        assert_eq!(parsed, built);
        assert!(warnings.is_empty());
    }
}
