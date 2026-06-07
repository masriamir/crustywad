//! Integration tests for typed Doom map record parsing.

use crustywad::map::{
    Linedef, Name8, Node, Sector, Seg, Sidedef, Subsector, Thing, Vertex, parse_records,
};

#[test]
fn parses_things() {
    let bytes = [1, 0, 2, 0, 90, 0, 4, 0, 5, 0];
    let records = parse_records::<Thing>(&bytes).expect("thing should parse");
    assert_eq!(records[0].x, 1);
    assert_eq!(records[0].type_id, 4);
}

#[test]
fn parses_linedefs() {
    let bytes = [1, 0, 2, 0, 3, 0, 4, 0, 5, 0, 6, 0, 255, 255];
    let records = parse_records::<Linedef>(&bytes).expect("linedef should parse");
    assert_eq!(records[0].left_sidedef, u16::MAX);
}

#[test]
fn parses_sidedefs() {
    let mut bytes = vec![1, 0, 2, 0];
    bytes.extend_from_slice(b"UPPERTEX");
    bytes.extend_from_slice(b"LOWERTEX");
    bytes.extend_from_slice(b"MIDTEX\0\0");
    bytes.extend_from_slice(&3_u16.to_le_bytes());
    let records = parse_records::<Sidedef>(&bytes).expect("sidedef should parse");
    assert_eq!(records[0].upper_texture.as_str_lossy(), "UPPERTEX");
    assert_eq!(records[0].sector, 3);
}

#[test]
fn parses_vertexes() {
    let bytes = [10, 0, 20, 0];
    let records = parse_records::<Vertex>(&bytes).expect("vertex should parse");
    assert_eq!(records[0].y, 20);
}

#[test]
fn parses_subsectors() {
    let bytes = [4, 0, 2, 0];
    let records = parse_records::<Subsector>(&bytes).expect("subsector should parse");
    assert_eq!(records[0].seg_count, 4);
}

#[test]
fn parses_nodes() {
    let mut bytes = Vec::new();
    for value in [1_i16, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&13_u16.to_le_bytes());
    bytes.extend_from_slice(&14_u16.to_le_bytes());
    let records = parse_records::<Node>(&bytes).expect("node should parse");
    assert_eq!(records[0].left_bbox, [9, 10, 11, 12]);
}

#[test]
fn parses_sectors() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&(-8_i16).to_le_bytes());
    bytes.extend_from_slice(&64_i16.to_le_bytes());
    bytes.extend_from_slice(b"FLOOR\0\0\0");
    bytes.extend_from_slice(b"CEIL\0\0\0\0");
    bytes.extend_from_slice(&160_i16.to_le_bytes());
    bytes.extend_from_slice(&1_i16.to_le_bytes());
    bytes.extend_from_slice(&2_i16.to_le_bytes());
    let records = parse_records::<Sector>(&bytes).expect("sector should parse");
    assert_eq!(records[0].floor_texture.as_str_lossy(), "FLOOR");
    assert_eq!(records[0].tag, 2);
}

#[test]
fn parses_segs() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&1_u16.to_le_bytes()); // start_vertex
    bytes.extend_from_slice(&2_u16.to_le_bytes()); // end_vertex
    bytes.extend_from_slice(&90_i16.to_le_bytes()); // angle
    bytes.extend_from_slice(&3_u16.to_le_bytes()); // linedef
    bytes.extend_from_slice(&0_u16.to_le_bytes()); // direction
    bytes.extend_from_slice(&(-5_i16).to_le_bytes()); // offset — negative, validates i16 type
    let records = parse_records::<Seg>(&bytes).expect("seg should parse");
    assert_eq!(records[0].start_vertex, 1);
    assert_eq!(records[0].angle, 90);
    assert_eq!(records[0].linedef, 3);
    assert_eq!(records[0].offset, -5);
}

#[test]
fn parse_records_rejects_trailing_bytes() {
    // 11 bytes for a 10-byte Thing record leaves 1 trailing byte
    let bytes = [1, 0, 2, 0, 90, 0, 4, 0, 5, 0, 99];
    let err = parse_records::<Thing>(&bytes).expect_err("trailing byte should fail");
    assert!(matches!(
        err,
        crustywad::map::MapParseError::TrailingBytes { offset: 10 }
    ));
}

#[test]
fn parses_name8_lossily() {
    let record = Name8(*b"START\0\0\0");
    assert_eq!(record.as_str_lossy(), "START");
}
