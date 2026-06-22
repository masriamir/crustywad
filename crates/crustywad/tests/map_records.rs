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
    bytes.extend_from_slice(&90_u16.to_le_bytes()); // angle
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
fn seg_angle_high_bit_is_unsigned() {
    // 0x8000 = 180° in BAMS. As i16 this was -32768; as u16 it must be 32768.
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0_u16.to_le_bytes()); // start_vertex
    bytes.extend_from_slice(&1_u16.to_le_bytes()); // end_vertex
    bytes.extend_from_slice(&0x8000_u16.to_le_bytes()); // angle (high bit set)
    bytes.extend_from_slice(&0_u16.to_le_bytes()); // linedef
    bytes.extend_from_slice(&0_u16.to_le_bytes()); // direction
    bytes.extend_from_slice(&0_i16.to_le_bytes()); // offset
    let records = parse_records::<Seg>(&bytes).expect("seg should parse");
    assert_eq!(records[0].angle, 0x8000);
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
fn parse_records_uses_on_disk_size_not_size_of() {
    // Regression: parse_records must derive per-record size from BinRead cursor
    // advancement, not from size_of::<T>(). For a #[repr(C)] struct whose fields
    // contain alignment padding, size_of is larger than the bytes BinRead consumes.
    #[repr(C)]
    #[derive(binrw::BinRead, Debug, PartialEq)]
    #[br(little)]
    struct Padded {
        a: u8,
        b: u16,
    }
    // BinRead reads: a (1 byte) + b (2 bytes) = 3 bytes on disk.
    // size_of::<Padded>() == 4 because #[repr(C)] inserts a padding byte before b.
    assert_eq!(
        std::mem::size_of::<Padded>(),
        4,
        "sanity: repr(C) must pad to 4"
    );
    // Two records back-to-back = 6 bytes. The old size_of-based approach would
    // treat this as 6 % 4 != 0 and return TrailingBytes; cursor-based detects 3.
    let bytes: Vec<u8> = vec![1, 2, 0, 3, 4, 0];
    let records = parse_records::<Padded>(&bytes).expect("should parse 2 padded records");
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].a, 1);
    assert_eq!(records[0].b, 2);
    assert_eq!(records[1].a, 3);
    assert_eq!(records[1].b, 4);
}

#[test]
fn parses_name8_lossily() {
    let record = Name8(*b"START\0\0\0");
    assert_eq!(record.as_str_lossy(), "START");
}
