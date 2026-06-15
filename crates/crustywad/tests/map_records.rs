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
fn thing_all_fields_including_large_unsigned() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&(-100_i16).to_le_bytes()); // x
    bytes.extend_from_slice(&200_i16.to_le_bytes()); // y
    bytes.extend_from_slice(&0x8000_u16.to_le_bytes()); // angle — would read as −32768 under the old i16 type
    bytes.extend_from_slice(&3004_u16.to_le_bytes()); // type_id
    bytes.extend_from_slice(&0x001F_u16.to_le_bytes()); // flags
    let records = parse_records::<Thing>(&bytes).expect("thing should parse");
    assert_eq!(records[0].x, -100);
    assert_eq!(records[0].y, 200);
    assert_eq!(records[0].angle, 0x8000);
    assert_eq!(records[0].type_id, 3004);
    assert_eq!(records[0].flags, 0x001F);
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

#[test]
fn parse_records_returns_empty_vec_for_empty_slice() {
    let records = parse_records::<Thing>(&[]).expect("empty slice should parse to empty vec");
    assert!(records.is_empty());
}

#[test]
fn parse_records_multiple_records() {
    // Two back-to-back Thing records (each 10 bytes)
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&[1, 0, 2, 0, 0, 0, 1, 0, 0, 0]); // thing 0
    bytes.extend_from_slice(&[3, 0, 4, 0, 45, 0, 2, 0, 7, 0]); // thing 1
    let records = parse_records::<Thing>(&bytes).expect("two things should parse");
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].x, 1);
    assert_eq!(records[1].x, 3);
}

#[test]
fn parse_records_zst_empty_buffer_returns_empty() {
    use crustywad::map::MapParseError;
    // ZST type `()` has size 0; empty buffer → empty vec
    let records = parse_records::<()>(&[]).expect("ZST empty buffer should produce empty vec");
    assert!(records.is_empty());
    // ZST type with non-empty buffer → TrailingBytes error
    let err =
        parse_records::<()>(&[0xFF]).expect_err("non-empty buffer for ZST record type should fail");
    assert!(matches!(err, MapParseError::TrailingBytes { offset: 0 }));
}

#[test]
fn map_parse_error_display_trailing_bytes() {
    let bytes = [1, 0, 2, 0, 90, 0, 4, 0, 5, 0, 99]; // 11 bytes, Thing is 10
    let err = parse_records::<Thing>(&bytes).expect_err("trailing byte should fail");
    let msg = err.to_string();
    assert!(
        msg.contains("10"),
        "error message should mention the offset: {msg}"
    );
}

#[test]
fn name8_full_8_bytes_no_null() {
    let record = Name8(*b"ABCDEFGH");
    assert_eq!(record.as_str_lossy(), "ABCDEFGH");
}

#[test]
fn name8_all_null_returns_empty() {
    let record = Name8([0u8; 8]);
    assert_eq!(record.as_str_lossy(), "");
}
