//! Integration tests for Strife dialogue lump parsing (ADR-0028 §5).

use crustywad::ParseOptions;
use crustywad::map::strife::{DialogueFormat, DialogueRecord, DialogueWarning, parse_dialogue};

/// Builds one 228-byte choice with distinct sentinel values derived from `seed`.
fn choice_bytes(seed: i32) -> Vec<u8> {
    let mut b = Vec::with_capacity(228);
    b.extend_from_slice(&seed.to_le_bytes()); // giveitem
    for k in 0..3i32 {
        b.extend_from_slice(&(seed + 10 + k).to_le_bytes()); // needitems
    }
    for k in 0..3i32 {
        b.extend_from_slice(&(seed + 20 + k).to_le_bytes()); // needamounts
    }
    let mut text = [0_u8; 32];
    text[..4].copy_from_slice(b"CHT_");
    text[4] = b'0' + u8::try_from(seed.rem_euclid(10)).unwrap();
    b.extend_from_slice(&text); // text
    b.extend_from_slice(&[0_u8; 80]); // textok (all NUL)
    b.extend_from_slice(&(seed + 30).to_le_bytes()); // next
    b.extend_from_slice(&(seed + 40).to_le_bytes()); // objective
    b.extend_from_slice(&[0_u8; 80]); // textno
    assert_eq!(b.len(), 228);
    b
}

/// Builds one exact 1516-byte retail record (offsets per the #246 research
/// record: header 376 B then 5 x 228 B choices).
fn retail_record_bytes() -> Vec<u8> {
    let mut b = Vec::with_capacity(1516);
    b.extend_from_slice(&101_i32.to_le_bytes()); // speakerid
    b.extend_from_slice(&102_i32.to_le_bytes()); // dropitem
    for k in 0..3i32 {
        b.extend_from_slice(&(200 + k).to_le_bytes()); // checkitem
    }
    b.extend_from_slice(&103_i32.to_le_bytes()); // jumptoconv
    let mut name = [0_u8; 16];
    name[..5].copy_from_slice(b"MACIL");
    b.extend_from_slice(&name);
    let mut voice = [0_u8; 8];
    voice[..4].copy_from_slice(b"VOC1");
    b.extend_from_slice(&voice);
    let mut backpic = [0_u8; 8];
    backpic[..4].copy_from_slice(b"PIC1");
    b.extend_from_slice(&backpic);
    let mut text = [0_u8; 320];
    text[..5].copy_from_slice(b"HELLO");
    b.extend_from_slice(&text);
    assert_eq!(b.len(), 376);
    for i in 0..5i32 {
        b.extend_from_slice(&choice_bytes(i));
    }
    assert_eq!(b.len(), 1516);
    b
}

#[test]
fn retail_record_round_trips_every_field() {
    let bytes = retail_record_bytes();
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        let (records, format, warnings) = parse_dialogue(&bytes, &options).expect("parses");
        assert_eq!(format, DialogueFormat::Retail);
        assert!(warnings.is_empty());
        assert_eq!(records.len(), 1);
        let r: &DialogueRecord = &records[0];
        assert_eq!(r.speaker_id, 101);
        assert_eq!(r.drop_item, 102);
        assert_eq!(r.check_items, Some([200, 201, 202]));
        assert_eq!(r.jump_to_conversation, Some(103));
        assert_eq!(&r.name[..5], b"MACIL");
        assert_eq!(&r.voice[..4], b"VOC1");
        assert_eq!(r.backpic.map(|p| p[..4].to_vec()), Some(b"PIC1".to_vec()));
        assert_eq!(&r.text[..5], b"HELLO");
        for (i, c) in r.choices.iter().enumerate() {
            let seed = i32::try_from(i).unwrap();
            assert_eq!(c.give_item, seed);
            assert_eq!(c.need_items, [seed + 10, seed + 11, seed + 12]);
            assert_eq!(c.need_amounts, [seed + 20, seed + 21, seed + 22]);
            assert_eq!(&c.text[..4], b"CHT_");
            assert_eq!(c.next, seed + 30);
            assert_eq!(c.objective, seed + 40);
        }
    }
}

#[test]
fn two_retail_records_parse() {
    let mut bytes = retail_record_bytes();
    bytes.extend_from_slice(&retail_record_bytes());
    let (records, format, warnings) =
        parse_dialogue(&bytes, &ParseOptions::strict()).expect("parses");
    assert_eq!((records.len(), format), (2, DialogueFormat::Retail));
    assert!(warnings.is_empty());
}

#[test]
fn empty_lump_is_zero_retail_records() {
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        let (records, format, warnings) = parse_dialogue(&[], &options).expect("parses");
        assert!(records.is_empty());
        assert_eq!(format, DialogueFormat::Retail);
        assert!(warnings.is_empty());
    }
}

#[test]
fn unterminated_full_width_strings_do_not_panic() {
    let mut bytes = retail_record_bytes();
    // name field (offset 24, 16 bytes) filled entirely with 'A' — no NUL.
    for b in &mut bytes[24..40] {
        *b = b'A';
    }
    let (records, _, _) = parse_dialogue(&bytes, &ParseOptions::strict()).expect("parses");
    assert_eq!(records[0].name, [b'A'; 16]);
}

#[test]
fn lenient_trailing_bytes_floor_divides_with_warning() {
    let mut bytes = retail_record_bytes();
    bytes.extend_from_slice(&[0xAB; 84]); // 1600 bytes total
    let (records, format, warnings) =
        parse_dialogue(&bytes, &ParseOptions::lenient()).expect("parses");
    assert_eq!((records.len(), format), (1, DialogueFormat::Retail));
    assert_eq!(
        warnings,
        [DialogueWarning::TrailingBytes {
            kept: 1,
            trailing: 84,
            format: DialogueFormat::Retail
        }]
    );
}

#[test]
fn strict_rejects_neither_modulus_length() {
    let mut bytes = retail_record_bytes();
    bytes.extend_from_slice(&[0xAB; 84]);
    let err = parse_dialogue(&bytes, &ParseOptions::strict()).unwrap_err();
    assert!(err.to_string().contains("1600"), "{err}");
}
