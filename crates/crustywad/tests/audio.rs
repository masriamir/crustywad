//! Integration tests for the classic audio layer (ADR-0023 §1-§2):
//! [`AudioKind::detect`], [`DmxSound`], and [`PcSpeakerSound`], plus the
//! optional retail sweep anchor.

#[cfg(feature = "sweep-tests")]
mod common;

use crustywad::ParseOptions;
use crustywad::audio::{AudioError, AudioKind, AudioWarning, DmxSound, PcSpeakerSound};
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
