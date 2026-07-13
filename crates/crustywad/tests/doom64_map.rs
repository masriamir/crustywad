//! Integration tests for Doom 64 map-record parsing and the nested-WAD reader.

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
