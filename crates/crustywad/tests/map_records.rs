//! Integration tests for typed Doom map record parsing.

use crustywad::map::doom::{Linedef, Thing};
use crustywad::map::{Name8, Node, Sector, Seg, Sidedef, Subsector, Vertex, parse_records};
use proptest::prelude::*;

#[cfg(feature = "write")]
use binrw::BinWrite;
#[cfg(feature = "write")]
use std::io::Cursor;

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
fn parse_records_empty_slice_returns_empty_vec() {
    let records = parse_records::<Thing>(&[]).expect("empty slice should return empty vec");
    assert!(records.is_empty());
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
fn parse_records_buffer_shorter_than_one_record_returns_trailing_bytes_at_zero() {
    // 5 bytes is shorter than one Thing record (10 bytes). The first BinRead
    // call returns UnexpectedEof, which parse_records maps to TrailingBytes{0}.
    let bytes = [1_u8, 0, 2, 0, 90];
    let err = parse_records::<Thing>(&bytes).expect_err("too-short buffer should fail");
    assert!(matches!(
        err,
        crustywad::map::MapParseError::TrailingBytes { offset: 0 }
    ));
}

#[test]
fn parse_records_zero_size_record_type_returns_trailing_bytes() {
    // A unit struct has no fields so BinRead reads 0 bytes per record.
    // Any non-empty buffer must be rejected as unresolvable trailing data.
    #[derive(binrw::BinRead, Debug)]
    struct Empty;
    let err = parse_records::<Empty>(&[1_u8]).expect_err("zero-size record type should fail");
    assert!(matches!(
        err,
        crustywad::map::MapParseError::TrailingBytes { offset: 0 }
    ));
}

#[test]
fn parse_records_non_io_parse_error_wraps_in_binrw_variant() {
    // A struct with a magic value produces BadMagic (not an Io error) when the
    // bytes do not match. parse_records must wrap it in MapParseError::Binrw.
    #[derive(binrw::BinRead, Debug)]
    #[br(little, magic = 0xDEAD_BEEFu32)]
    struct Magic {
        _value: u32,
    }
    // First 4 bytes [0,0,0,0] do not match the magic 0xDEAD_BEEF.
    let bytes = [0u8; 8];
    let err = parse_records::<Magic>(&bytes).expect_err("bad magic should fail");
    assert!(matches!(err, crustywad::map::MapParseError::Binrw(_)));
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

#[test]
fn decodes_hexen_thing_fields() {
    use crustywad::map::hexen::Thing;
    use crustywad::map::parse_records;
    // tid=42, x=-16, y=32, z=8, angle=90, type=3001, flags=7, special=80, args=[1,2,3,4,5]
    let bytes: &[u8] = &[
        0x2A, 0x00, // tid = 42
        0xF0, 0xFF, // x = -16
        0x20, 0x00, // y = 32
        0x08, 0x00, // z = 8
        0x5A, 0x00, // angle = 90
        0xB9, 0x0B, // type_id = 3001
        0x07, 0x00, // flags = 7
        0x50, // special = 80
        0x01, 0x02, 0x03, 0x04, 0x05, // args
    ];
    let things: Vec<Thing> = parse_records(bytes).expect("decodes");
    assert_eq!(things.len(), 1);
    let t = &things[0];
    assert_eq!(t.tid, 42);
    assert_eq!(t.x, -16);
    assert_eq!(t.y, 32);
    assert_eq!(t.z, 8);
    assert_eq!(t.angle, 90);
    assert_eq!(t.type_id, 3001);
    assert_eq!(t.flags, 7);
    assert_eq!(t.special, 80);
    assert_eq!(t.args, [1, 2, 3, 4, 5]);
}

#[test]
fn decodes_hexen_linedef_fields() {
    use crustywad::map::hexen::Linedef;
    use crustywad::map::parse_records;
    // start=0, end=1, flags=1, special=13, args=[99,0,0,0,0], right=0, left=0xffff
    let bytes: &[u8] = &[
        0x00, 0x00, // start_vertex = 0
        0x01, 0x00, // end_vertex = 1
        0x01, 0x00, // flags = 1
        0x0D, // special = 13
        0x63, 0x00, 0x00, 0x00, 0x00, // args = [99,0,0,0,0]
        0x00, 0x00, // right_sidedef = 0
        0xFF, 0xFF, // left_sidedef = 0xffff
    ];
    let lines: Vec<Linedef> = parse_records(bytes).expect("decodes");
    assert_eq!(lines.len(), 1);
    let l = &lines[0];
    assert_eq!(l.start_vertex, 0);
    assert_eq!(l.end_vertex, 1);
    assert_eq!(l.flags, 1);
    assert_eq!(l.special, 13);
    assert_eq!(l.args, [99, 0, 0, 0, 0]);
    assert_eq!(l.right_sidedef, 0);
    assert_eq!(l.left_sidedef, 0xffff);
}

/// Serializing a record and re-parsing it must reproduce it exactly — this is
/// what lets the conversion writer trust the typed records as its encoder.
#[cfg(feature = "write")]
#[test]
fn doom_records_round_trip_through_binwrite() {
    let thing = Thing {
        x: -32,
        y: 4096,
        angle: 90,
        type_id: 3001,
        flags: 0x00f7,
    };
    let mut buf = Cursor::new(Vec::new());
    thing.write_le(&mut buf).unwrap();
    assert_eq!(buf.get_ref().len(), 10, "a Doom THINGS record is 10 bytes");
    let parsed: Vec<Thing> = parse_records(buf.get_ref()).unwrap();
    assert_eq!(parsed, vec![thing]);
}

#[cfg(feature = "write")]
#[test]
fn common_records_round_trip_through_binwrite() {
    let sector = Sector {
        floor_height: -8,
        ceiling_height: 128,
        floor_texture: Name8(*b"FLOOR4_8"),
        ceiling_texture: Name8(*b"CEIL3_5\0"),
        light_level: 160,
        special_type: 9,
        tag: 3,
    };
    let mut buf = Cursor::new(Vec::new());
    sector.write_le(&mut buf).unwrap();
    assert_eq!(buf.get_ref().len(), 26, "a Doom SECTORS record is 26 bytes");
    let parsed: Vec<Sector> = parse_records(buf.get_ref()).unwrap();
    assert_eq!(parsed, vec![sector]);
}

proptest! {
    // I-5: parse_records::<Thing> never panics on arbitrary bytes
    #[test]
    fn parse_records_thing_no_panic(
        data in proptest::collection::vec(any::<u8>(), 0..4096usize)
    ) {
        let _ = std::hint::black_box(
            parse_records::<Thing>(&data)
        );
    }

    // I-8: parse_records::<Thing> must return Ok with exactly len/10 records
    // when the input length is an exact multiple of 10. Thing has five integer
    // fields (i16/u16) with no validation, so any 10-byte sequence is a valid
    // record — Err is never acceptable for this case.
    #[test]
    fn thing_trailing_bytes_semantics(
        data in proptest::collection::vec(any::<u8>(), 0..4096usize)
    ) {
        const THING_SIZE: usize = 10;
        let result = parse_records::<Thing>(&data);
        if data.len() % THING_SIZE == 0 {
            prop_assert!(
                result.is_ok(),
                "parse_records::<Thing> must succeed on {}-byte input (exact multiple of {}): got {:?}",
                data.len(), THING_SIZE, result
            );
            prop_assert_eq!(result.unwrap().len(), data.len() / THING_SIZE);
        }
        // Non-multiple lengths: only absence of panic is asserted (covered by I-5)
    }
}

// Type-level smoke test for the classic-GL-node graph types (#324 Task 1):
// this only asserts that `GlVertexRef` composes over `GlVertexIdx` as
// expected. It does not exercise assembly or a `Map`'s GL arenas — those are
// covered once decoding lands in a later task.
#[test]
fn gl_vertex_ref_variants_compose() {
    use crustywad::map::graph::{GlVertexIdx, GlVertexRef};

    let r = GlVertexRef::Gl(GlVertexIdx(3));
    assert!(matches!(r, GlVertexRef::Gl(GlVertexIdx(3))));
}

// #324 Task 2: GL-node error/warning variants exist and display sensibly.
// No decoding logic yet — later tasks (4, 8, 9) raise these.
#[test]
fn gl_refused_warning_displays() {
    use crustywad::map::MapWarning;
    let w = MapWarning::GlNodesRefused { version: 1 };
    assert!(w.to_string().contains("GL node"));
}
