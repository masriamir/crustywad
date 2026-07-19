//! Integration tests for the classic audio layer (ADR-0023 §1-§2):
//! [`AudioKind::detect`], [`DmxSound`], and [`PcSpeakerSound`], plus the
//! optional retail sweep anchor.

#[cfg(feature = "sweep-tests")]
mod common;

use crustywad::ParseOptions;
use crustywad::audio::{
    AudioError, AudioKind, AudioWarning, DmxSound, Dmxgus, DmxgusEntry, Genmidi, GenmidiOp,
    GenmidiVoice, MidiInfo, MidiTrack, MusEvent, MusEventKind, MusScore, PcSpeakerSound, WavSound,
};
use proptest::prelude::*;

const BOTH_MODES: [fn() -> ParseOptions; 2] = [ParseOptions::strict, ParseOptions::lenient];

/// An 8-byte DMX header: format `3`, the given sample rate, and length field.
fn dmx_header(rate: u16, length: u32) -> Vec<u8> {
    let mut v = Vec::with_capacity(8);
    v.extend_from_slice(&3u16.to_le_bytes());
    v.extend_from_slice(&rate.to_le_bytes());
    v.extend_from_slice(&length.to_le_bytes());
    v
}

/// Fixture D1: a valid 60-byte DMX lump — rate 11025, length 52, 20 PCM
/// samples `0..=19` between the two 16-byte pads.
fn d1() -> Vec<u8> {
    let mut v = dmx_header(11025, 52);
    v.extend_from_slice(&[0xAA; 16]);
    v.extend(0u8..=19);
    v.extend_from_slice(&[0xBB; 16]);
    v
}

#[test]
fn d1_valid_dmx_zero_warnings() {
    let d1 = d1();
    for opts in BOTH_MODES {
        let snd = DmxSound::parse(&d1, &opts()).expect("D1 parses");
        assert_eq!(snd.sample_rate(), 11025);
        assert_eq!(snd.length(), 52);
        assert_eq!(snd.samples(), (0u8..=19).collect::<Vec<u8>>().as_slice());
        assert_eq!(snd.payload().len(), 52);
        assert!(snd.warnings().is_empty());
    }
}

#[test]
fn d2_floor_placeholder_dssmfire_shape() {
    // length 32 (16 + 16 pads, zero samples): a valid but unplayable lump.
    let mut d2 = dmx_header(11025, 32);
    d2.extend_from_slice(&[0xAA; 16]);
    d2.extend_from_slice(&[0xBB; 16]);
    assert_eq!(d2.len(), 40);
    for opts in BOTH_MODES {
        let snd = DmxSound::parse(&d2, &opts()).expect("D2 parses");
        assert!(snd.samples().is_empty());
        assert_eq!(
            snd.warnings(),
            &[AudioWarning::PlayabilityFloor { length: 32 }]
        );
    }
}

#[test]
fn d3_trailing_slack() {
    let mut d3 = d1();
    d3.extend_from_slice(&[0xFF; 4]);
    assert_eq!(d3.len(), 64);
    for opts in BOTH_MODES {
        let snd = DmxSound::parse(&d3, &opts()).expect("D3 parses");
        assert_eq!(snd.samples(), (0u8..=19).collect::<Vec<u8>>().as_slice());
        assert_eq!(
            snd.warnings(),
            &[AudioWarning::TrailingSlack {
                expected: 60,
                lump_len: 64,
            }]
        );
    }
}

#[test]
fn d4_zero_sample_rate() {
    let mut d4 = d1();
    d4[2] = 0;
    d4[3] = 0;
    for opts in BOTH_MODES {
        let snd = DmxSound::parse(&d4, &opts()).expect("D4 parses");
        assert_eq!(snd.sample_rate(), 0);
        assert_eq!(snd.samples(), (0u8..=19).collect::<Vec<u8>>().as_slice());
        assert_eq!(snd.warnings(), &[AudioWarning::ZeroSampleRate]);
    }
}

#[test]
fn d5_truncated_header() {
    let d5 = vec![0u8; 7];
    for opts in BOTH_MODES {
        let err = DmxSound::parse(&d5, &opts()).unwrap_err();
        assert!(matches!(
            err,
            AudioError::TruncatedHeader { len: 7, needed: 8 }
        ));
    }
}

#[test]
fn d6_wrong_format() {
    let mut d6 = d1();
    d6[0] = 4;
    d6[1] = 0;
    for opts in BOTH_MODES {
        let err = DmxSound::parse(&d6, &opts()).unwrap_err();
        assert!(matches!(
            err,
            AudioError::UnexpectedFormat {
                expected: 3,
                found: 4,
            }
        ));
    }
}

#[test]
fn d7_overrun() {
    // D1 body but a length field of 100 (available is only 52).
    let mut d7 = d1();
    d7[4..8].copy_from_slice(&100u32.to_le_bytes());

    let strict_err = DmxSound::parse(&d7, &ParseOptions::strict()).unwrap_err();
    assert!(matches!(
        strict_err,
        AudioError::LengthOutOfRange {
            length: 100,
            min: 32,
            available: 52,
        }
    ));

    let snd = DmxSound::parse(&d7, &ParseOptions::lenient()).expect("D7 recovers leniently");
    assert_eq!(snd.samples(), (0u8..=19).collect::<Vec<u8>>().as_slice());
    assert_eq!(
        snd.warnings(),
        &[AudioWarning::LengthOutOfRange {
            length: 100,
            min: 32,
            available: 52,
        }]
    );
}

#[test]
fn d8_under_min() {
    // D1 body but a length field of 20 (below the 32-byte pad minimum).
    let mut d8 = d1();
    d8[4..8].copy_from_slice(&20u32.to_le_bytes());

    let strict_err = DmxSound::parse(&d8, &ParseOptions::strict()).unwrap_err();
    assert!(matches!(
        strict_err,
        AudioError::LengthOutOfRange {
            length: 20,
            min: 32,
            available: 52,
        }
    ));

    let snd = DmxSound::parse(&d8, &ParseOptions::lenient()).expect("D8 recovers leniently");
    assert!(snd.samples().is_empty());
    assert_eq!(
        snd.warnings(),
        &[
            AudioWarning::LengthOutOfRange {
                length: 20,
                min: 32,
                available: 52,
            },
            AudioWarning::TrailingSlack {
                expected: 28,
                lump_len: 60,
            },
        ]
    );
}

/// Fixture P1: a valid 9-byte PC-speaker lump — format 0, five tones.
fn p1() -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&0u16.to_le_bytes());
    v.extend_from_slice(&5u16.to_le_bytes());
    v.extend_from_slice(&[10, 42, 0, 127, 96]);
    v
}

#[test]
fn p1_valid_pc_speaker() {
    let p1 = p1();
    for opts in BOTH_MODES {
        let snd = PcSpeakerSound::parse(&p1, &opts()).expect("P1 parses");
        assert_eq!(snd.declared_count(), 5);
        assert_eq!(snd.tones(), &[10, 42, 0, 127, 96]);
        assert!(snd.warnings().is_empty());
    }
}

#[test]
fn p2_out_of_range_tones() {
    let mut p2 = Vec::new();
    p2.extend_from_slice(&0u16.to_le_bytes());
    p2.extend_from_slice(&3u16.to_le_bytes());
    p2.extend_from_slice(&[1, 200, 130]);
    for opts in BOTH_MODES {
        let snd = PcSpeakerSound::parse(&p2, &opts()).expect("P2 parses");
        assert_eq!(snd.tones(), &[1, 200, 130]);
        assert_eq!(
            snd.warnings(),
            &[AudioWarning::OutOfRangeTones { count: 2 }]
        );
    }
}

#[test]
fn p3_trailing_slack() {
    let mut p3 = p1();
    p3.extend_from_slice(&[0xFF; 2]);
    assert_eq!(p3.len(), 11);
    for opts in BOTH_MODES {
        let snd = PcSpeakerSound::parse(&p3, &opts()).expect("P3 parses");
        assert_eq!(snd.tones(), &[10, 42, 0, 127, 96]);
        assert_eq!(
            snd.warnings(),
            &[AudioWarning::TrailingSlack {
                expected: 9,
                lump_len: 11,
            }]
        );
    }
}

#[test]
fn p4_truncated_header() {
    let p4 = vec![0u8; 3];
    for opts in BOTH_MODES {
        let err = PcSpeakerSound::parse(&p4, &opts()).unwrap_err();
        assert!(matches!(
            err,
            AudioError::TruncatedHeader { len: 3, needed: 4 }
        ));
    }
}

#[test]
fn p5_wrong_format() {
    let p5 = vec![0x01, 0x00, 0x02, 0x00, 0x05, 0x06];
    for opts in BOTH_MODES {
        let err = PcSpeakerSound::parse(&p5, &opts()).unwrap_err();
        assert!(matches!(
            err,
            AudioError::UnexpectedFormat {
                expected: 0,
                found: 1,
            }
        ));
    }
}

#[test]
fn p6_overrun() {
    let mut p6 = Vec::new();
    p6.extend_from_slice(&0u16.to_le_bytes());
    p6.extend_from_slice(&10u16.to_le_bytes());
    p6.extend_from_slice(&[1, 2, 3, 4, 5]);

    let strict_err = PcSpeakerSound::parse(&p6, &ParseOptions::strict()).unwrap_err();
    assert!(matches!(
        strict_err,
        AudioError::LengthOutOfRange {
            length: 10,
            min: 0,
            available: 5,
        }
    ));

    let snd = PcSpeakerSound::parse(&p6, &ParseOptions::lenient()).expect("P6 recovers leniently");
    assert_eq!(snd.tones(), &[1, 2, 3, 4, 5]);
    assert_eq!(
        snd.warnings(),
        &[AudioWarning::LengthOutOfRange {
            length: 10,
            min: 0,
            available: 5,
        }]
    );
}

#[test]
fn p7_zero_count() {
    let p7 = vec![0u8; 4];
    for opts in BOTH_MODES {
        let snd = PcSpeakerSound::parse(&p7, &opts()).expect("P7 parses");
        assert_eq!(snd.declared_count(), 0);
        assert!(snd.tones().is_empty());
        assert!(snd.warnings().is_empty());
    }
}

#[test]
fn detect_classifies_by_content() {
    assert_eq!(AudioKind::detect(&d1()), AudioKind::Dmx);
    assert_eq!(AudioKind::detect(&p1()), AudioKind::PcSpeaker);
    assert_eq!(
        AudioKind::detect(&[b'M', b'T', b'h', b'd', 0, 0, 0, 6]),
        AudioKind::Midi
    );
    assert_eq!(
        AudioKind::detect(&[
            b'R', b'I', b'F', b'F', 0xA4, 0x7D, 0, 0, b'W', b'A', b'V', b'E'
        ]),
        AudioKind::Wav
    );
    assert_eq!(
        AudioKind::detect(&[b'M', b'U', b'S', 0x1A, 0, 0, 0, 0]),
        AudioKind::Mus
    );

    // The DSTAA0 sprite/`DS`-prefix collision (ADR-0023 §1): a picture header
    // (width 39, height 56, …) that is not audio at all.
    let strife_sprite = [
        0x27, 0x00, 0x38, 0x00, 0x13, 0x00, 0x33, 0x00, 0xA4, 0x00, 0x00, 0x00,
    ];
    assert_eq!(AudioKind::detect(&strife_sprite), AudioKind::Unknown);

    assert_eq!(AudioKind::detect(&[]), AudioKind::Unknown);
    assert_eq!(AudioKind::detect(&[0u8; 3]), AudioKind::Unknown);

    // D2's length is exactly at the 32-byte boundary detect requires.
    let mut d2 = dmx_header(11025, 32);
    d2.extend_from_slice(&[0xAA; 16]);
    d2.extend_from_slice(&[0xBB; 16]);
    assert_eq!(AudioKind::detect(&d2), AudioKind::Dmx);

    // D7's overrun length fails detect's arithmetic (100 > 60 - 8).
    let mut d7 = d1();
    d7[4..8].copy_from_slice(&100u32.to_le_bytes());
    assert_eq!(AudioKind::detect(&d7), AudioKind::Unknown);
}

proptest! {
    /// Detection and the parsers are coherent, and neither parser panics on
    /// arbitrary bytes in either mode: a `Dmx`/`PcSpeaker` classification
    /// guarantees the matching strict parse succeeds.
    #[test]
    fn detect_parse_coherence(data in proptest::collection::vec(any::<u8>(), 0..=512usize)) {
        let kind = AudioKind::detect(&data);
        let dmx_strict = DmxSound::parse(&data, &ParseOptions::strict());
        let pc_strict = PcSpeakerSound::parse(&data, &ParseOptions::strict());
        if kind == AudioKind::Dmx {
            prop_assert!(
                dmx_strict.is_ok(),
                "detect==Dmx but strict DmxSound::parse failed: {:?}",
                dmx_strict.err()
            );
        }
        if kind == AudioKind::PcSpeaker {
            prop_assert!(
                pc_strict.is_ok(),
                "detect==PcSpeaker but strict PcSpeakerSound::parse failed: {:?}",
                pc_strict.err()
            );
        }
        let _ = std::hint::black_box(DmxSound::parse(&data, &ParseOptions::lenient()));
        let _ = std::hint::black_box(PcSpeakerSound::parse(&data, &ParseOptions::lenient()));
    }
}

// ---------------------------------------------------------------------------
// MUS music (ADR-0023 §2): MusScore
// ---------------------------------------------------------------------------

// M1's event stream: press-key ch0 note 60 velocity 100, release-key ch0 note
// 60 delay 70, score-end.
const M1_EVENTS: [u8; 7] = [0x10, 0xBC, 0x64, 0x80, 0x3C, 0x46, 0x60];

/// Builds a MUS lump: magic, header (primary 1, secondary 0), the instrument
/// list, then the raw event bytes. `score_length`/`score_start` are supplied
/// directly so out-of-bounds fixtures can declare a wrong offset.
fn mus(score_length: u16, score_start: u16, instruments: &[u16], events: &[u8]) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&[0x4D, 0x55, 0x53, 0x1A]);
    v.extend_from_slice(&score_length.to_le_bytes());
    v.extend_from_slice(&score_start.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes()); // primary channels
    v.extend_from_slice(&0u16.to_le_bytes()); // secondary channels
    v.extend_from_slice(&u16::try_from(instruments.len()).unwrap().to_le_bytes());
    for &patch in instruments {
        v.extend_from_slice(&patch.to_le_bytes());
    }
    v.extend_from_slice(events);
    v
}

// Fixture M1: a valid 23-byte MUS lump (score length 7, score start 16, one
// instrument [1], three events).
fn m1() -> Vec<u8> {
    mus(7, 16, &[1], &M1_EVENTS)
}

#[test]
fn m1_valid_mus() {
    let bytes = m1();
    assert_eq!(bytes.len(), 23);
    for opts in BOTH_MODES {
        let score = MusScore::parse(&bytes, &opts()).expect("M1 parses");
        assert_eq!(score.score_length(), 7);
        assert_eq!(score.score_start(), 16);
        assert_eq!(score.primary_channels(), 1);
        assert_eq!(score.secondary_channels(), 0);
        assert_eq!(score.instruments(), &[1]);
        assert_eq!(
            score.events(),
            &[
                MusEvent {
                    channel: 0,
                    kind: MusEventKind::PressKey {
                        note: 60,
                        velocity: Some(100),
                    },
                    delay: 0,
                },
                MusEvent {
                    channel: 0,
                    kind: MusEventKind::ReleaseKey { note: 60 },
                    delay: 70,
                },
                MusEvent {
                    channel: 0,
                    kind: MusEventKind::ScoreEnd,
                    delay: 0,
                },
            ]
        );
        assert!(score.warnings().is_empty());
    }
}

#[test]
fn m2_varint_delta() {
    // M1 with event 2's single delta byte replaced by a 2-byte varint
    // (0x81 0x48 = 1 * 128 + 0x48 = 200); score_length 8, total 24 bytes.
    let bytes = mus(
        8,
        16,
        &[1],
        &[0x10, 0xBC, 0x64, 0x80, 0x3C, 0x81, 0x48, 0x60],
    );
    assert_eq!(bytes.len(), 24);
    for opts in BOTH_MODES {
        let score = MusScore::parse(&bytes, &opts()).expect("M2 parses");
        assert_eq!(
            score.events()[1],
            MusEvent {
                channel: 0,
                kind: MusEventKind::ReleaseKey { note: 60 },
                delay: 200,
            }
        );
        assert!(score.warnings().is_empty());
    }
}

#[test]
fn m3_truncated_event() {
    // M1 truncated so the second event's trailing delta byte and the score-end
    // are gone (keeps `... 80 3C`, drops `46 60`).
    let mut bytes = m1();
    bytes.truncate(21);

    let err = MusScore::parse(&bytes, &ParseOptions::strict()).unwrap_err();
    assert!(matches!(err, AudioError::TruncatedEvent { offset: 21 }));

    // Lenient drops the incomplete trailing event, keeping PressKey only, and
    // records both the score-length overrun and the truncation.
    let score = MusScore::parse(&bytes, &ParseOptions::lenient()).expect("M3 recovers");
    assert_eq!(
        score.events(),
        &[MusEvent {
            channel: 0,
            kind: MusEventKind::PressKey {
                note: 60,
                velocity: Some(100),
            },
            delay: 0,
        }]
    );
    assert_eq!(
        score.warnings(),
        &[
            AudioWarning::ScoreLengthOverrun {
                declared_end: 23,
                lump_len: 21,
            },
            AudioWarning::TruncatedEvent { offset: 21 },
        ]
    );
}

#[test]
fn m4_bad_system_controller() {
    // SystemEvent with controller 5 (valid range is 10..=14).
    let bytes = mus(3, 16, &[1], &[0x30, 0x05, 0x60]);
    assert_eq!(bytes.len(), 19);

    let err = MusScore::parse(&bytes, &ParseOptions::strict()).unwrap_err();
    assert!(matches!(
        err,
        AudioError::InvalidSystemController {
            controller: 5,
            offset: 16,
        }
    ));

    let score = MusScore::parse(&bytes, &ParseOptions::lenient()).expect("M4 recovers");
    assert!(score.events().is_empty());
    assert_eq!(
        score.warnings(),
        &[AudioWarning::InvalidSystemController {
            controller: 5,
            offset: 16,
        }]
    );
}

#[test]
fn m5_unknown_event_type() {
    // Descriptor 0x50 selects an undefined event type.
    let bytes = mus(3, 16, &[1], &[0x50, 0x00, 0x60]);

    let err = MusScore::parse(&bytes, &ParseOptions::strict()).unwrap_err();
    assert!(matches!(
        err,
        AudioError::UnknownEventType {
            event_type: 0x50,
            offset: 16,
        }
    ));

    let score = MusScore::parse(&bytes, &ParseOptions::lenient()).expect("M5 recovers");
    assert!(score.events().is_empty());
    assert_eq!(
        score.warnings(),
        &[AudioWarning::UnknownEventType {
            event_type: 0x50,
            offset: 16,
        }]
    );
}

#[test]
fn m6_score_start_out_of_bounds() {
    // M1 body but score_start declares 200, well past the 23-byte lump.
    let bytes = mus(7, 200, &[1], &M1_EVENTS);
    assert_eq!(bytes.len(), 23);

    let err = MusScore::parse(&bytes, &ParseOptions::strict()).unwrap_err();
    assert!(matches!(
        err,
        AudioError::OffsetOutOfBounds {
            offset: 200,
            lump_len: 23,
        }
    ));

    let score = MusScore::parse(&bytes, &ParseOptions::lenient()).expect("M6 recovers");
    assert!(score.events().is_empty());
    assert_eq!(score.instruments(), &[1]);
    assert_eq!(
        score.warnings(),
        &[AudioWarning::OffsetOutOfBounds {
            offset: 200,
            lump_len: 23,
        }]
    );
}

#[test]
fn m7_no_score_end() {
    // A single PressKey and then EOF — the stream never reaches a score-end.
    let bytes = mus(3, 16, &[1], &[0x10, 0xBC, 0x64]);
    assert_eq!(bytes.len(), 19);

    let err = MusScore::parse(&bytes, &ParseOptions::strict()).unwrap_err();
    assert!(matches!(err, AudioError::MissingScoreEnd { offset: 19 }));

    let score = MusScore::parse(&bytes, &ParseOptions::lenient()).expect("M7 recovers");
    assert_eq!(
        score.events(),
        &[MusEvent {
            channel: 0,
            kind: MusEventKind::PressKey {
                note: 60,
                velocity: Some(100),
            },
            delay: 0,
        }]
    );
    assert_eq!(
        score.warnings(),
        &[AudioWarning::MissingScoreEnd { offset: 19 }]
    );
}

// ---------------------------------------------------------------------------
// Standard MIDI index (ADR-0023 §2, §4): MidiInfo
// ---------------------------------------------------------------------------

/// Fixture MI1: a valid 26-byte SMF lump — format 0, one track, division 96,
/// with a single 4-byte `MTrk` payload at offset 22.
fn mi1() -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(b"MThd");
    v.extend_from_slice(&6u32.to_be_bytes());
    v.extend_from_slice(&0u16.to_be_bytes()); // format
    v.extend_from_slice(&1u16.to_be_bytes()); // ntracks
    v.extend_from_slice(&96u16.to_be_bytes()); // division
    v.extend_from_slice(b"MTrk");
    v.extend_from_slice(&4u32.to_be_bytes());
    v.extend_from_slice(&[0x00, 0xFF, 0x2F, 0x00]);
    v
}

#[test]
fn mi1_valid_midi() {
    let bytes = mi1();
    assert_eq!(bytes.len(), 26);
    for opts in BOTH_MODES {
        let midi = MidiInfo::parse(&bytes, &opts()).expect("MI1 parses");
        assert_eq!(midi.format(), 0);
        assert_eq!(midi.declared_tracks(), 1);
        assert_eq!(midi.division(), 96);
        assert_eq!(
            midi.tracks(),
            &[MidiTrack {
                offset: 22,
                length: 4,
            }]
        );
        assert!(midi.warnings().is_empty());
    }
}

#[test]
fn mi2_empty_lump() {
    for opts in BOTH_MODES {
        let midi = MidiInfo::parse(&[], &opts()).expect("MI2 parses (NOSOUND placeholder)");
        assert_eq!(midi.format(), 0);
        assert!(midi.tracks().is_empty());
        assert!(midi.warnings().is_empty());
    }
}

#[test]
fn mi3_track_overrun() {
    // MI1 with the MTrk length field inflated to 100 (only 4 bytes remain).
    let mut bytes = mi1();
    bytes[18..22].copy_from_slice(&100u32.to_be_bytes());

    let err = MidiInfo::parse(&bytes, &ParseOptions::strict()).unwrap_err();
    assert!(matches!(
        err,
        AudioError::ChunkOverrun {
            offset: 14,
            declared: 100,
            available: 4,
        }
    ));

    let midi = MidiInfo::parse(&bytes, &ParseOptions::lenient()).expect("MI3 recovers");
    assert!(midi.tracks().is_empty());
    assert_eq!(
        midi.warnings(),
        &[
            AudioWarning::ChunkOverrun {
                offset: 14,
                declared: 100,
                available: 4,
            },
            AudioWarning::TrackCountMismatch {
                declared: 1,
                found: 0,
            },
        ]
    );
}

#[test]
fn mi4_alien_chunk() {
    // MI1 with the MTrk id replaced by `XFIR` — a non-MTrk chunk, skipped.
    let mut bytes = mi1();
    bytes[14..18].copy_from_slice(b"XFIR");
    for opts in BOTH_MODES {
        let midi = MidiInfo::parse(&bytes, &opts()).expect("MI4 parses");
        assert!(midi.tracks().is_empty());
        assert_eq!(
            midi.warnings(),
            &[
                AudioWarning::AlienChunk { id: *b"XFIR" },
                AudioWarning::TrackCountMismatch {
                    declared: 1,
                    found: 0,
                },
            ]
        );
    }
}

#[test]
fn mi5_extended_header_realigns_walk() {
    // An MThd declaring 8 header bytes (2 beyond the standard 6): the chunk
    // walk must start at 8 + 8 = 16, not the fixed offset 14 — otherwise the
    // two extra header bytes would be misread as the start of a chunk frame.
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"MThd");
    bytes.extend_from_slice(&8u32.to_be_bytes());
    bytes.extend_from_slice(&0u16.to_be_bytes());
    bytes.extend_from_slice(&1u16.to_be_bytes());
    bytes.extend_from_slice(&96u16.to_be_bytes());
    bytes.extend_from_slice(&[0xEE, 0xEE]);
    bytes.extend_from_slice(b"MTrk");
    bytes.extend_from_slice(&4u32.to_be_bytes());
    bytes.extend_from_slice(&[0x00, 0xFF, 0x2F, 0x00]);
    assert_eq!(bytes.len(), 28);

    let err = MidiInfo::parse(&bytes, &ParseOptions::strict()).unwrap_err();
    assert!(matches!(
        err,
        AudioError::UnexpectedChunkSize {
            expected: 6,
            found: 8,
        }
    ));

    let midi = MidiInfo::parse(&bytes, &ParseOptions::lenient()).expect("MI5 recovers");
    assert_eq!(
        midi.tracks(),
        &[MidiTrack {
            offset: 24,
            length: 4,
        }]
    );
    assert_eq!(
        midi.warnings(),
        &[AudioWarning::UnexpectedChunkSize {
            expected: 6,
            found: 8,
        }]
    );
}

#[test]
fn mi6_extended_header_overrun_diagnosed() {
    // A declared MThd size that overruns the lump must surface as a
    // ChunkOverrun diagnostic, not a silent clamp.
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"MThd");
    bytes.extend_from_slice(&1000u32.to_be_bytes());
    bytes.extend_from_slice(&0u16.to_be_bytes());
    bytes.extend_from_slice(&1u16.to_be_bytes());
    bytes.extend_from_slice(&96u16.to_be_bytes());
    bytes.extend_from_slice(&[0xEE, 0xEE]);
    bytes.extend_from_slice(b"MTrk");
    bytes.extend_from_slice(&4u32.to_be_bytes());
    bytes.extend_from_slice(&[0x00, 0xFF, 0x2F, 0x00]);
    assert_eq!(bytes.len(), 28);

    let midi = MidiInfo::parse(&bytes, &ParseOptions::lenient()).expect("MI6 recovers");
    assert!(midi.tracks().is_empty());
    assert_eq!(
        midi.warnings(),
        &[
            AudioWarning::UnexpectedChunkSize {
                expected: 6,
                found: 1000,
            },
            AudioWarning::ChunkOverrun {
                offset: 0,
                declared: 1000,
                available: 20,
            },
            AudioWarning::TrackCountMismatch {
                declared: 1,
                found: 0,
            },
        ]
    );
}

// ---------------------------------------------------------------------------
// RIFF/WAVE walk (ADR-0023 §2, §4): WavSound
// ---------------------------------------------------------------------------

/// Fixture W1: a valid canonical 48-byte PCM WAV — mono, 22050 Hz, 16-bit,
/// with 4 data bytes.
fn w1() -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(b"RIFF");
    v.extend_from_slice(&40u32.to_le_bytes());
    v.extend_from_slice(b"WAVE");
    v.extend_from_slice(b"fmt ");
    v.extend_from_slice(&16u32.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes()); // format tag (PCM)
    v.extend_from_slice(&1u16.to_le_bytes()); // channels
    v.extend_from_slice(&22050u32.to_le_bytes()); // sample rate
    v.extend_from_slice(&44100u32.to_le_bytes()); // byte rate
    v.extend_from_slice(&2u16.to_le_bytes()); // block align
    v.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    v.extend_from_slice(b"data");
    v.extend_from_slice(&4u32.to_le_bytes());
    v.extend_from_slice(&[0x00, 0x01, 0xFF, 0x7F]);
    v
}

#[test]
fn w1_valid_wav() {
    let bytes = w1();
    assert_eq!(bytes.len(), 48);
    for opts in BOTH_MODES {
        let wav = WavSound::parse(&bytes, &opts()).expect("W1 parses");
        assert_eq!(wav.format_tag(), 1);
        assert_eq!(wav.channels(), 1);
        assert_eq!(wav.sample_rate(), 22050);
        assert_eq!(wav.byte_rate(), 44100);
        assert_eq!(wav.block_align(), 2);
        assert_eq!(wav.bits_per_sample(), 16);
        assert_eq!(wav.data(), &[0x00, 0x01, 0xFF, 0x7F]);
        assert!(wav.warnings().is_empty());
    }
}

#[test]
fn w2_missing_data_chunk() {
    // W1 truncated after the fmt chunk (36 bytes), riff size corrected to 28.
    let mut bytes = w1();
    bytes.truncate(36);
    bytes[4..8].copy_from_slice(&28u32.to_le_bytes());

    let err = WavSound::parse(&bytes, &ParseOptions::strict()).unwrap_err();
    assert!(matches!(err, AudioError::MissingChunk { id } if &id == b"data"));

    let wav = WavSound::parse(&bytes, &ParseOptions::lenient()).expect("W2 recovers");
    assert_eq!(wav.format_tag(), 1);
    assert!(wav.data().is_empty());
    assert_eq!(
        wav.warnings(),
        &[AudioWarning::MissingChunk { id: *b"data" }]
    );
}

#[test]
fn w3_chunk_overrun() {
    // W1 with the data chunk size field inflated to 100.
    let mut bytes = w1();
    bytes[40..44].copy_from_slice(&100u32.to_le_bytes());

    let err = WavSound::parse(&bytes, &ParseOptions::strict()).unwrap_err();
    assert!(matches!(
        err,
        AudioError::ChunkOverrun {
            offset: 36,
            declared: 100,
            available: 4,
        }
    ));

    let wav = WavSound::parse(&bytes, &ParseOptions::lenient()).expect("W3 recovers");
    assert!(wav.data().is_empty());
    assert_eq!(
        wav.warnings(),
        &[
            AudioWarning::ChunkOverrun {
                offset: 36,
                declared: 100,
                available: 4,
            },
            AudioWarning::MissingChunk { id: *b"data" },
        ]
    );
}

#[test]
fn w4_riff_size_mismatch() {
    // W1 with the riff size field inflated to 100 (implies an end of 108).
    let mut bytes = w1();
    bytes[4..8].copy_from_slice(&100u32.to_le_bytes());
    for opts in BOTH_MODES {
        let wav = WavSound::parse(&bytes, &opts()).expect("W4 parses");
        assert_eq!(wav.format_tag(), 1);
        assert_eq!(wav.data(), &[0x00, 0x01, 0xFF, 0x7F]);
        assert_eq!(
            wav.warnings(),
            &[AudioWarning::RiffSizeMismatch {
                declared_end: 108,
                lump_len: 48,
            }]
        );
    }
}

#[test]
fn detect_music_and_containers() {
    assert_eq!(AudioKind::detect(&m1()), AudioKind::Mus);
    assert_eq!(AudioKind::detect(&mi1()), AudioKind::Midi);
    assert_eq!(AudioKind::detect(&w1()), AudioKind::Wav);
}

// ---------------------------------------------------------------------------
// GENMIDI OPL instrument bank (ADR-0023 §2): Genmidi
// ---------------------------------------------------------------------------

const GENMIDI_TOTAL: usize = 11908;
const GENMIDI_NAMES_START: usize = 6308;

/// An all-zero 11908-byte bank with only the `#OPL_II#` magic set.
fn genmidi_base() -> Vec<u8> {
    let mut v = vec![0u8; GENMIDI_TOTAL];
    v[..8].copy_from_slice(b"#OPL_II#");
    v
}

/// A zero [`GenmidiVoice`] — the shape of every voice left untouched in a base
/// bank (used to assert instrument 0's second voice).
fn zero_voice() -> GenmidiVoice {
    let zero_op = GenmidiOp {
        tremolo: 0,
        attack: 0,
        sustain: 0,
        waveform: 0,
        scale: 0,
        level: 0,
    };
    GenmidiVoice {
        modulator: zero_op,
        feedback: 0,
        carrier: zero_op,
        unused: 0,
        base_note_offset: 0,
    }
}

/// Fixture G1: a valid 11908-byte `GENMIDI` bank with melodic instrument 0 and
/// two names populated to known values.
fn g1() -> Vec<u8> {
    let mut v = genmidi_base();

    // Melodic instrument 0 at offset 8.
    let off = 8;
    v[off..off + 2].copy_from_slice(&0x0005u16.to_le_bytes()); // flags: fixed pitch + double voice
    v[off + 2] = 0x80; // fine_tuning
    v[off + 3] = 60; // fixed_note

    // Voice 0 at off+4: modulator (6), feedback, carrier (6), unused, offset.
    let vo = off + 4;
    v[vo..vo + 6].copy_from_slice(&[1, 2, 3, 4, 5, 6]);
    v[vo + 6] = 7;
    v[vo + 7..vo + 13].copy_from_slice(&[8, 9, 10, 11, 12, 13]);
    v[vo + 13] = 0;
    v[vo + 14..vo + 16].copy_from_slice(&(-12i16).to_le_bytes());

    // Melodic name 0.
    let n0 = b"ACPIANO";
    v[GENMIDI_NAMES_START..GENMIDI_NAMES_START + n0.len()].copy_from_slice(n0);

    // Percussion name 46 (the last one).
    let perc46 = GENMIDI_NAMES_START + (128 + 46) * 32;
    let n1 = b"LASTONE";
    v[perc46..perc46 + n1.len()].copy_from_slice(n1);

    v
}

#[test]
fn g1_valid_genmidi() {
    let bytes = g1();
    assert_eq!(bytes.len(), GENMIDI_TOTAL);
    for opts in BOTH_MODES {
        let bank = Genmidi::parse(&bytes, &opts()).expect("G1 parses");
        assert_eq!(bank.instruments().len(), 128);
        assert_eq!(bank.percussion().len(), 47);
        assert_eq!(bank.instrument_names().len(), 128);
        assert_eq!(bank.percussion_names().len(), 47);

        let instr = bank.instruments()[0];
        assert_eq!(instr.flags, 0x0005);
        assert!(instr.is_fixed_pitch());
        assert!(instr.is_double_voice());
        assert_eq!(instr.fine_tuning, 0x80);
        assert_eq!(instr.fixed_note, 60);
        assert_eq!(
            instr.voices[0],
            GenmidiVoice {
                modulator: GenmidiOp {
                    tremolo: 1,
                    attack: 2,
                    sustain: 3,
                    waveform: 4,
                    scale: 5,
                    level: 6,
                },
                feedback: 7,
                carrier: GenmidiOp {
                    tremolo: 8,
                    attack: 9,
                    sustain: 10,
                    waveform: 11,
                    scale: 12,
                    level: 13,
                },
                unused: 0,
                base_note_offset: -12,
            }
        );
        assert_eq!(instr.voices[1], zero_voice());

        assert_eq!(bank.instrument_names()[0], "ACPIANO");
        assert_eq!(bank.percussion_names()[46], "LASTONE");
        assert!(bank.warnings().is_empty());
    }
}

#[test]
fn g2_short_lump() {
    let bytes = g1()[..5000].to_vec();

    let err = Genmidi::parse(&bytes, &ParseOptions::strict()).unwrap_err();
    assert!(matches!(
        err,
        AudioError::TruncatedHeader {
            len: 5000,
            needed: 11908,
        }
    ));

    // 5000 - 8 = 4992 -> 138 complete records (128 melodic + 10 percussion),
    // zero names (names begin at offset 6308, past the lump).
    let bank = Genmidi::parse(&bytes, &ParseOptions::lenient()).expect("G2 recovers");
    assert_eq!(bank.instruments().len(), 128);
    assert_eq!(bank.percussion().len(), 10);
    assert_eq!(bank.instrument_names().len(), 0);
    assert_eq!(bank.percussion_names().len(), 0);
    assert_eq!(
        bank.warnings(),
        &[AudioWarning::TruncatedBank {
            len: 5000,
            needed: 11908,
        }]
    );
}

#[test]
fn g3_trailing_slack() {
    let mut bytes = g1();
    bytes.extend_from_slice(&[0xEE; 16]);
    assert_eq!(bytes.len(), 11924);
    for opts in BOTH_MODES {
        let bank = Genmidi::parse(&bytes, &opts()).expect("G3 parses");
        assert_eq!(bank.instruments().len(), 128);
        assert_eq!(bank.percussion().len(), 47);
        assert_eq!(bank.instrument_names().len(), 128);
        assert_eq!(bank.percussion_names().len(), 47);
        assert_eq!(
            bank.warnings(),
            &[AudioWarning::TrailingSlack {
                expected: 11908,
                lump_len: 11924,
            }]
        );
    }
}

#[test]
fn g4_bad_magic() {
    let mut bytes = g1();
    bytes[0] = 0x00;

    let err = Genmidi::parse(&bytes, &ParseOptions::strict()).unwrap_err();
    assert!(matches!(err, AudioError::BadMagic { .. }));

    let bank = Genmidi::parse(&bytes, &ParseOptions::lenient()).expect("G4 recovers");
    assert_eq!(bank.instruments().len(), 128);
    assert_eq!(bank.warnings().len(), 1);
    assert!(matches!(bank.warnings()[0], AudioWarning::BadMagic { .. }));
}

// ---------------------------------------------------------------------------
// DMXGUS patch-mapping lump (ADR-0023 §2): Dmxgus
// ---------------------------------------------------------------------------

#[test]
fn u1_valid_dmxgus() {
    let text =
        "# comment\n0, 2, 2, 2, 2, acpiano.pat\n128, 1, 1, 1, 1, drum.pat  # trailing comment\n";
    for opts in BOTH_MODES {
        let gus = Dmxgus::parse(text.as_bytes(), &opts()).expect("U1 parses");
        assert_eq!(
            gus.entries(),
            &[
                DmxgusEntry {
                    instrument: 0,
                    mappings: [2, 2, 2, 2],
                    patch: "acpiano.pat".to_owned(),
                },
                DmxgusEntry {
                    instrument: 128,
                    mappings: [1, 1, 1, 1],
                    patch: "drum.pat".to_owned(),
                },
            ]
        );
        // Reserved-gap ids (128 here) are data in every retail DMXGUS
        // (ADR-0023 §2 amendment) — classified, never warned.
        assert!(gus.warnings().is_empty());
        assert!(gus.entries()[0].is_gm_mapped());
        assert!(!gus.entries()[1].is_gm_mapped());
    }
}

#[test]
fn u2_too_few_fields() {
    let text = "0, 1, 2\n";

    let err = Dmxgus::parse(text.as_bytes(), &ParseOptions::strict()).unwrap_err();
    assert!(matches!(err, AudioError::MalformedGusLine { line: 1 }));

    let gus = Dmxgus::parse(text.as_bytes(), &ParseOptions::lenient()).expect("U2 recovers");
    assert!(gus.entries().is_empty());
    assert_eq!(
        gus.warnings(),
        &[AudioWarning::MalformedGusLine { line: 1 }]
    );
}

#[test]
fn u3_non_numeric_id() {
    let text = "abc, 1, 2, 3, 4, x.pat\n";

    let err = Dmxgus::parse(text.as_bytes(), &ParseOptions::strict()).unwrap_err();
    assert!(matches!(err, AudioError::MalformedGusLine { line: 1 }));

    let gus = Dmxgus::parse(text.as_bytes(), &ParseOptions::lenient()).expect("U3 recovers");
    assert!(gus.entries().is_empty());
    assert_eq!(
        gus.warnings(),
        &[AudioWarning::MalformedGusLine { line: 1 }]
    );
}

#[test]
fn u4_crlf_and_empty_lines() {
    let text = "\r\n0, 1, 2, 3, 4, ok.pat\r\n\r\n";
    for opts in BOTH_MODES {
        let gus = Dmxgus::parse(text.as_bytes(), &opts()).expect("U4 parses");
        assert_eq!(
            gus.entries(),
            &[DmxgusEntry {
                instrument: 0,
                mappings: [1, 2, 3, 4],
                patch: "ok.pat".to_owned(),
            }]
        );
        assert!(gus.warnings().is_empty());
    }
}

/// Retail sweep anchor (ADR-0023 §5): every audio lump in the collection is
/// coherent with detection and every warning on a *named* `DS`/`DP` lump is a
/// [`AudioWarning::PlayabilityFloor`]. Optionally pins the single curated-
/// collection floor (strife1's `DSSMFIRE`, length 32) when
/// `CRUSTYWAD_SWEEP_EXPECT_GATE_CONTRACTS` is set.
#[cfg(feature = "sweep-tests")]
#[test]
fn retail_audio_decode_strict_clean() {
    use crustywad::Wad;

    let paths = common::wad_files("CRUSTYWAD_SWEEP_DIR");
    if paths.is_empty() {
        return; // skip note already printed by the helper
    }

    let mut dmx_parsed = 0usize;
    let mut pc_parsed = 0usize;
    let mut dmx_named = 0usize;
    let mut pcs_named = 0usize;
    let mut floor_lengths: Vec<u32> = Vec::new();

    for path in &paths {
        let wad = Wad::from_path_with_options(path, ParseOptions::strict())
            .unwrap_or_else(|e| panic!("{}: container failed strict parse: {e}", path.display()));

        for i in 0..wad.lump_count() {
            let bytes = wad.lump_bytes(i).expect("lump index in range");
            let kind = AudioKind::detect(bytes);

            // 1. Coherence over every lump: a classification must parse strict.
            match kind {
                AudioKind::Dmx => {
                    DmxSound::parse(bytes, &ParseOptions::strict()).unwrap_or_else(|e| {
                        panic!(
                            "{}: lump {i} detects Dmx but failed strict: {e}",
                            path.display()
                        )
                    });
                    dmx_parsed += 1;
                }
                AudioKind::PcSpeaker => {
                    PcSpeakerSound::parse(bytes, &ParseOptions::strict()).unwrap_or_else(|e| {
                        panic!(
                            "{}: lump {i} detects PcSpeaker but failed strict: {e}",
                            path.display()
                        )
                    });
                    pc_parsed += 1;
                }
                _ => {}
            }

            // 2. Named contract: DS/DP-prefixed lumps (excluding DS_/DP_
            //    section markers) whose content matches must parse with only
            //    PlayabilityFloor warnings — anything else is a failure.
            let name = wad.lump(i).expect("lump index in range").name().to_owned();
            let is_dmx_name = name.starts_with("DS") && !name.starts_with("DS_");
            let is_pcs_name = name.starts_with("DP") && !name.starts_with("DP_");

            let warnings: Vec<AudioWarning> = if is_dmx_name && kind == AudioKind::Dmx {
                dmx_named += 1;
                let snd = DmxSound::parse(bytes, &ParseOptions::strict())
                    .unwrap_or_else(|e| panic!("{}: named {name} strict: {e}", path.display()));
                snd.warnings().to_vec()
            } else if is_pcs_name && kind == AudioKind::PcSpeaker {
                pcs_named += 1;
                let snd = PcSpeakerSound::parse(bytes, &ParseOptions::strict())
                    .unwrap_or_else(|e| panic!("{}: named {name} strict: {e}", path.display()));
                snd.warnings().to_vec()
            } else {
                Vec::new()
            };

            for w in warnings {
                match w {
                    AudioWarning::PlayabilityFloor { length } => floor_lengths.push(length),
                    other => panic!(
                        "{}: named lump {name} produced a non-floor warning: {other:?}",
                        path.display()
                    ),
                }
            }
        }
    }

    // 3. Curated-collection pin (opt-in, keyed by warning identity): exactly
    //    one PlayabilityFloor, length 32 (strife1's DSSMFIRE placeholder).
    if std::env::var_os("CRUSTYWAD_SWEEP_EXPECT_GATE_CONTRACTS").is_some() {
        assert_eq!(
            floor_lengths.len(),
            1,
            "expected exactly one PlayabilityFloor across the curated collection, saw {floor_lengths:?}"
        );
        assert_eq!(
            floor_lengths[0], 32,
            "the single floor lump must be strife1's DSSMFIRE (length 32)"
        );
    }

    // 4. A sweep that found WADs must have decoded audio in both families.
    assert!(dmx_parsed > 0, "sweep detected no DMX sounds");
    assert!(pc_parsed > 0, "sweep detected no PC-speaker sounds");
    eprintln!(
        "audio sweep: {} WAD(s), {dmx_parsed} DMX parsed, {pc_parsed} PC-speaker parsed, \
         {dmx_named} named DS, {pcs_named} named DP, {} PlayabilityFloor",
        paths.len(),
        floor_lengths.len()
    );
}

/// Retail sweep anchor for the music and bank formats (ADR-0023 §5): every
/// content-detected MUS/MIDI/WAV lump and every name-detected
/// `GENMIDI`/`DMXGUS`/`DMXGUSC` lump in the collection must parse strict with
/// **zero** warnings. A retail anomaly (a strict failure or any warning) is an
/// adjudication for the reviewer, not something to loosen away.
#[cfg(feature = "sweep-tests")]
#[test]
fn retail_music_banks_strict_clean() {
    use crustywad::Wad;

    let paths = common::wad_files("CRUSTYWAD_SWEEP_DIR");
    if paths.is_empty() {
        return; // skip note already printed by the helper
    }

    let mut mus_parsed = 0usize;
    let mut midi_parsed = 0usize;
    let mut wav_parsed = 0usize;
    let mut genmidi_parsed = 0usize;
    let mut dmxgus_parsed = 0usize;
    // Every strict failure or warning is collected (not fail-fast) so a single
    // run surfaces the whole retail adjudication set for the reviewer.
    let mut anomalies: Vec<String> = Vec::new();

    for path in &paths {
        let wad = Wad::from_path_with_options(path, ParseOptions::strict())
            .unwrap_or_else(|e| panic!("{}: container failed strict parse: {e}", path.display()));
        let wad_name = path.display();

        for i in 0..wad.lump_count() {
            let bytes = wad.lump_bytes(i).expect("lump index in range");
            let name = wad.lump(i).expect("lump index in range").name().to_owned();

            // 1. Content-detected music/container formats.
            match AudioKind::detect(bytes) {
                AudioKind::Mus => match MusScore::parse(bytes, &ParseOptions::strict()) {
                    Ok(score) => {
                        mus_parsed += 1;
                        if !score.warnings().is_empty() {
                            anomalies.push(format!(
                                "{wad_name}: MUS {name} warned: {:?}",
                                score.warnings()
                            ));
                        }
                    }
                    Err(e) => anomalies.push(format!("{wad_name}: MUS {name} strict failed: {e}")),
                },
                AudioKind::Midi => match MidiInfo::parse(bytes, &ParseOptions::strict()) {
                    Ok(midi) => {
                        midi_parsed += 1;
                        if !midi.warnings().is_empty() {
                            anomalies.push(format!(
                                "{wad_name}: MIDI {name} warned: {:?}",
                                midi.warnings()
                            ));
                        }
                    }
                    Err(e) => anomalies.push(format!("{wad_name}: MIDI {name} strict failed: {e}")),
                },
                AudioKind::Wav => match WavSound::parse(bytes, &ParseOptions::strict()) {
                    Ok(wav) => {
                        wav_parsed += 1;
                        if !wav.warnings().is_empty() {
                            anomalies.push(format!(
                                "{wad_name}: WAV {name} warned: {:?}",
                                wav.warnings()
                            ));
                        }
                    }
                    Err(e) => anomalies.push(format!("{wad_name}: WAV {name} strict failed: {e}")),
                },
                _ => {}
            }

            // 2. Name-detected instrument banks.
            if name == "GENMIDI" {
                match Genmidi::parse(bytes, &ParseOptions::strict()) {
                    Ok(bank) => {
                        genmidi_parsed += 1;
                        if !bank.warnings().is_empty() {
                            anomalies
                                .push(format!("{wad_name}: GENMIDI warned: {:?}", bank.warnings()));
                        }
                    }
                    Err(e) => anomalies.push(format!("{wad_name}: GENMIDI strict failed: {e}")),
                }
            } else if name == "DMXGUS" || name == "DMXGUSC" {
                match Dmxgus::parse(bytes, &ParseOptions::strict()) {
                    Ok(gus) => {
                        dmxgus_parsed += 1;
                        if !gus.warnings().is_empty() {
                            anomalies
                                .push(format!("{wad_name}: {name} warned: {:?}", gus.warnings()));
                        }
                    }
                    Err(e) => anomalies.push(format!("{wad_name}: {name} strict failed: {e}")),
                }
            }
        }
    }

    assert!(mus_parsed > 0, "sweep detected no MUS lumps");
    assert!(midi_parsed > 0, "sweep detected no MIDI lumps");
    assert!(wav_parsed > 0, "sweep detected no WAV lumps");
    assert!(genmidi_parsed > 0, "sweep found no GENMIDI lumps");
    assert!(dmxgus_parsed > 0, "sweep found no DMXGUS/DMXGUSC lumps");
    eprintln!(
        "music/banks sweep: {} WAD(s), {mus_parsed} MUS, {midi_parsed} MIDI, {wav_parsed} WAV, \
         {genmidi_parsed} GENMIDI, {dmxgus_parsed} DMXGUS",
        paths.len()
    );
    assert!(
        anomalies.is_empty(),
        "music/banks sweep found {} strict failure(s)/warning(s):\n{}",
        anomalies.len(),
        anomalies.join("\n")
    );
}
