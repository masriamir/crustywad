//! Integration tests for Strife dialogue lump parsing (ADR-0028 §5).

use crustywad::ParseOptions;
use crustywad::map::strife::{DialogueFormat, DialogueRecord, DialogueWarning, parse_dialogue};

/// Builds one 228-byte choice with distinct sentinel values derived from `seed`.
///
/// `text_ok` and `text_no` deliberately carry distinct sentinel content
/// (`OK_<digit>` vs `NO_<digit>`, rest NUL) rather than both being all-NUL:
/// a `text_ok`/`text_no` transposition in `RawChoice` must fail the
/// round-trip tests that check both fields.
fn choice_bytes(seed: i32) -> Vec<u8> {
    let mut b = Vec::with_capacity(228);
    b.extend_from_slice(&seed.to_le_bytes()); // giveitem
    for k in 0..3i32 {
        b.extend_from_slice(&(seed + 10 + k).to_le_bytes()); // needitems
    }
    for k in 0..3i32 {
        b.extend_from_slice(&(seed + 20 + k).to_le_bytes()); // needamounts
    }
    let digit = b'0' + u8::try_from(seed.rem_euclid(10)).unwrap();
    let mut text = [0_u8; 32];
    text[..4].copy_from_slice(b"CHT_");
    text[4] = digit;
    b.extend_from_slice(&text); // text
    let mut text_ok = [0_u8; 80];
    text_ok[..3].copy_from_slice(b"OK_");
    text_ok[3] = digit;
    b.extend_from_slice(&text_ok); // textok
    b.extend_from_slice(&(seed + 30).to_le_bytes()); // next
    b.extend_from_slice(&(seed + 40).to_le_bytes()); // objective
    let mut text_no = [0_u8; 80];
    text_no[..3].copy_from_slice(b"NO_");
    text_no[3] = digit;
    b.extend_from_slice(&text_no); // textno
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
            let digit = b'0' + u8::try_from(seed.rem_euclid(10)).unwrap();
            assert_eq!(c.give_item, seed);
            assert_eq!(c.need_items, [seed + 10, seed + 11, seed + 12]);
            assert_eq!(c.need_amounts, [seed + 20, seed + 21, seed + 22]);
            assert_eq!(&c.text[..4], b"CHT_");
            assert_eq!(&c.text_ok[..3], b"OK_");
            assert_eq!(c.text_ok[3], digit);
            assert_eq!(&c.text_no[..3], b"NO_");
            assert_eq!(c.text_no[3], digit);
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

/// Builds one exact 1488-byte demo record. Field order per strife-ve
/// `P_ParseDemoDialogLump` @ ac2381d: speakerid, dropitem, voicenumber (i32,
/// THIRD — before name), name[16], text[320], then the 5 choices.
fn demo_record_bytes(voicenumber: i32) -> Vec<u8> {
    let mut b = Vec::with_capacity(1488);
    b.extend_from_slice(&501_i32.to_le_bytes()); // speakerid
    b.extend_from_slice(&502_i32.to_le_bytes()); // dropitem
    b.extend_from_slice(&voicenumber.to_le_bytes()); // voicenumber
    let mut name = [0_u8; 16];
    name[..4].copy_from_slice(b"DEMO");
    b.extend_from_slice(&name);
    let mut text = [0_u8; 320];
    text[..3].copy_from_slice(b"HEY");
    b.extend_from_slice(&text);
    assert_eq!(b.len(), 348);
    for i in 0..5i32 {
        b.extend_from_slice(&choice_bytes(50 + i));
    }
    assert_eq!(b.len(), 1488);
    b
}

#[test]
fn demo_record_round_trips_with_engine_normalization() {
    let bytes = demo_record_bytes(7);
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        let (records, format, warnings) = parse_dialogue(&bytes, &options).expect("parses");
        assert_eq!(format, DialogueFormat::Demo);
        assert!(warnings.is_empty());
        let r = &records[0];
        assert_eq!(r.speaker_id, 501);
        assert_eq!(r.drop_item, 502);
        assert_eq!(r.check_items, None);
        assert_eq!(r.jump_to_conversation, None);
        assert_eq!(r.backpic, None);
        assert_eq!(&r.name[..4], b"DEMO");
        assert_eq!(&r.text[..3], b"HEY");
        assert_eq!(r.voice, *b"VOC7\0\0\0\0");
        assert_eq!(r.choices[0].give_item, 50);
        // seed 50: digit = 50 % 10 = 0 -> "OK_0" / "NO_0". Also guards
        // against a text_ok/text_no transposition in RawChoice.
        assert_eq!(&r.choices[0].text_ok[..3], b"OK_");
        assert_eq!(r.choices[0].text_ok[3], b'0');
        assert_eq!(&r.choices[0].text_no[..3], b"NO_");
        assert_eq!(r.choices[0].text_no[3], b'0');
    }
}

#[test]
fn demo_voice_reconstruction_matches_the_engine() {
    // voicenumber <= 0 -> all-NUL (engine: `if (voicenumber > 0)`).
    for n in [0, -3] {
        let bytes = demo_record_bytes(n);
        let (records, ..) = parse_dialogue(&bytes, &ParseOptions::strict()).expect("parses");
        assert_eq!(records[0].voice, [0_u8; 8], "voicenumber {n}");
    }
    // 7-char exact fit: VOC9999 + NUL.
    let (records, ..) =
        parse_dialogue(&demo_record_bytes(9999), &ParseOptions::strict()).expect("parses");
    assert_eq!(records[0].voice, *b"VOC9999\0");
    // M_snprintf truncation: "VOC123456" won't fit 8 with a NUL -> "VOC1234".
    let (records, ..) =
        parse_dialogue(&demo_record_bytes(123_456), &ParseOptions::strict()).expect("parses");
    assert_eq!(records[0].voice, *b"VOC1234\0");
}

#[test]
fn two_demo_records_parse() {
    let mut bytes = demo_record_bytes(1);
    bytes.extend_from_slice(&demo_record_bytes(2));
    let (records, format, _) = parse_dialogue(&bytes, &ParseOptions::strict()).expect("parses");
    assert_eq!((records.len(), format), (2, DialogueFormat::Demo));
}

#[test]
fn retail_wins_the_lcm_ambiguity() {
    // lcm(1516, 1488) = 563,952 = 1516 * 372 = 1488 * 379 — divisible by
    // both; the retail-first heuristic (SVE precedence) must pick Retail.
    let bytes = vec![0_u8; 563_952];
    let (records, format, warnings) =
        parse_dialogue(&bytes, &ParseOptions::strict()).expect("parses");
    assert_eq!(format, DialogueFormat::Retail);
    assert_eq!(records.len(), 372);
    assert!(warnings.is_empty());
}

#[test]
fn lenient_sub_record_length_yields_zero_records_and_warning() {
    let bytes = [0xCD_u8; 100];
    let (records, format, warnings) =
        parse_dialogue(&bytes, &ParseOptions::lenient()).expect("parses");
    assert!(records.is_empty());
    assert_eq!(format, DialogueFormat::Retail);
    assert_eq!(
        warnings,
        [DialogueWarning::TrailingBytes {
            kept: 0,
            trailing: 100,
            format: DialogueFormat::Retail
        }]
    );
    assert!(parse_dialogue(&bytes, &ParseOptions::strict()).is_err());
}
