//! Integration tests for the Raven script lumps (ADR-0023 §3):
//! [`SndCurve`], [`SndInfo`], and [`SndSeq`], plus the optional retail sweep
//! anchor.

#[cfg(feature = "sweep-tests")]
mod common;

use crustywad::ParseOptions;
use crustywad::audio::{
    AudioError, AudioWarning, SndCurve, SndInfo, SndInfoEntry, SndSeq, SndSeqCommand,
};

const BOTH_MODES: [fn() -> ParseOptions; 2] = [ParseOptions::strict, ParseOptions::lenient];

// ---------------------------------------------------------------------------
// SndCurve (ADR-0023 §3): a headerless byte table
// ---------------------------------------------------------------------------

#[test]
fn sndcurve_roundtrips_arbitrary_bytes() {
    let data: Vec<u8> = (0u8..=255).rev().collect();
    for opts in BOTH_MODES {
        let curve = SndCurve::parse(&data, &opts()).expect("SNDCURVE parses");
        assert_eq!(curve.bytes(), data.as_slice());
        assert!(curve.warnings().is_empty());
    }
}

#[test]
fn sndcurve_empty_lump_ok() {
    for opts in BOTH_MODES {
        let curve = SndCurve::parse(&[], &opts()).expect("empty SNDCURVE parses");
        assert!(curve.bytes().is_empty());
        assert!(curve.warnings().is_empty());
    }
}

// ---------------------------------------------------------------------------
// SndInfo (ADR-0023 §3): $-directives and bare tag/lump pairs
// ---------------------------------------------------------------------------

#[test]
fn s1_happy_path() {
    let text =
        "$ARCHIVEPATH x\n$MAP 1 WINNOWR\n; comment\nPlayerFighterNormalDeath PLDTHFGT\nAmbush ?\n";
    for opts in BOTH_MODES {
        let info = SndInfo::parse(text.as_bytes(), &opts()).expect("S1 parses");
        assert_eq!(info.map_songs(), &[(1, "WINNOWR".to_owned())]);
        assert_eq!(
            info.entries(),
            &[
                SndInfoEntry {
                    tag: "PlayerFighterNormalDeath".to_owned(),
                    lump: "PLDTHFGT".to_owned(),
                },
                SndInfoEntry {
                    tag: "Ambush".to_owned(),
                    lump: "?".to_owned(),
                },
            ]
        );
        assert_eq!(info.entries()[0].resolved_lump(), "PLDTHFGT");
        assert_eq!(info.entries()[1].resolved_lump(), "DEFAULT");
        assert!(info.warnings().is_empty());
    }
}

#[test]
fn s2_map_zero_dropped_and_unknown_directive_silent() {
    // `$MAP 0` is consumed and dropped silently (engine behavior).
    for opts in BOTH_MODES {
        let info = SndInfo::parse(b"$MAP 0 IGNORED\n", &opts()).expect("parses");
        assert!(info.map_songs().is_empty());
        assert!(info.entries().is_empty());
        assert!(info.warnings().is_empty());
    }

    // An unknown `$`-directive is silently ignored and consumes no following
    // value, so the trailing `tag lump` becomes one entry.
    //
    // NOTE — DEVIATION from the brief's S2 (which expected 1 warning): the
    // engine (`S_InitScript`, verified against chocolate-doom
    // `src/hexen/s_sound.c`) `continue`s past any unrecognized `$`-directive
    // WITHOUT warning, and ADR-0023 §3 states "unknown `$`-directives ignored".
    // All three retail SNDINFO carriers (HEXEN, HEXDD, hexdd_ex) ship a
    // `$REGISTERED` directive under exactly this path, so the zero-warning
    // retail sweep requires silence here, not a warning. See the module docs.
    for opts in BOTH_MODES {
        let info = SndInfo::parse(b"$UNKNOWNDIR\ntag lump\n", &opts()).expect("parses");
        assert_eq!(
            info.entries(),
            &[SndInfoEntry {
                tag: "tag".to_owned(),
                lump: "lump".to_owned(),
            }]
        );
        assert!(info.warnings().is_empty());
    }
}

#[test]
fn s3_bad_map_number_and_trailing_tag() {
    // `$MAP abc X` — strict error / lenient warn, both tokens skipped.
    let bad_map = b"$MAP abc X\n";
    let err = SndInfo::parse(bad_map, &ParseOptions::strict()).unwrap_err();
    assert!(matches!(err, AudioError::SndInfoBadMapNumber { .. }));

    let info = SndInfo::parse(bad_map, &ParseOptions::lenient()).expect("recovers");
    assert!(info.map_songs().is_empty());
    assert!(info.entries().is_empty());
    assert_eq!(info.warnings().len(), 1);
    assert!(matches!(
        info.warnings()[0],
        AudioWarning::SndInfoBadMapNumber { .. }
    ));

    // A trailing lone tag at EOF — strict error / lenient warn + drop.
    let lone = b"GoodTag GoodLump\nLoneTag\n";
    let err = SndInfo::parse(lone, &ParseOptions::strict()).unwrap_err();
    assert!(matches!(err, AudioError::SndInfoMissingValue { .. }));

    let info = SndInfo::parse(lone, &ParseOptions::lenient()).expect("recovers");
    assert_eq!(
        info.entries(),
        &[SndInfoEntry {
            tag: "GoodTag".to_owned(),
            lump: "GoodLump".to_owned(),
        }]
    );
    assert_eq!(info.warnings().len(), 1);
    assert!(matches!(
        info.warnings()[0],
        AudioWarning::SndInfoMissingValue { .. }
    ));
}

#[test]
fn s4_oversized_token_warns_both_modes() {
    let long_tag = "A".repeat(70);
    let text = format!("{long_tag} LUMP\n");
    for opts in BOTH_MODES {
        let info = SndInfo::parse(text.as_bytes(), &opts()).expect("parses");
        assert_eq!(info.entries().len(), 1);
        assert_eq!(info.entries()[0].tag.len(), 70);
        assert_eq!(info.entries()[0].lump, "LUMP");
        assert_eq!(
            info.warnings(),
            &[AudioWarning::OversizedTokens { count: 1 }]
        );
    }
}

#[test]
fn s5_quoted_string_is_one_token() {
    let text = "tag \"two words\"\n";
    for opts in BOTH_MODES {
        let info = SndInfo::parse(text.as_bytes(), &opts()).expect("parses");
        assert_eq!(
            info.entries(),
            &[SndInfoEntry {
                tag: "tag".to_owned(),
                lump: "two words".to_owned(),
            }]
        );
        assert!(info.warnings().is_empty());
    }
}

#[test]
fn sndinfo_empty_lump_ok() {
    for opts in BOTH_MODES {
        let info = SndInfo::parse(&[], &opts()).expect("empty SNDINFO parses");
        assert!(info.entries().is_empty());
        assert!(info.map_songs().is_empty());
        assert!(info.warnings().is_empty());
    }
}

// ---------------------------------------------------------------------------
// SndSeq (ADR-0023 §3): :-prefixed sequences of typed commands
// ---------------------------------------------------------------------------

#[test]
fn q1_happy_path() {
    let text = ":Platform\nplay PlatformStart\nplayrepeat PlatformLoop\nplaytime DoorClose 10\ndelay 5\ndelayrand 3 9\nvolume 50\nstopsound PlatformStop\nend\n";
    for opts in BOTH_MODES {
        let seq = SndSeq::parse(text.as_bytes(), &opts()).expect("Q1 parses");
        assert_eq!(seq.sequences().len(), 1);
        let sequence = &seq.sequences()[0];
        assert_eq!(sequence.name, "Platform");
        assert_eq!(
            sequence.commands,
            vec![
                SndSeqCommand::Play("PlatformStart".to_owned()),
                SndSeqCommand::PlayRepeat("PlatformLoop".to_owned()),
                SndSeqCommand::PlayTime {
                    sound: "DoorClose".to_owned(),
                    tics: 10,
                },
                SndSeqCommand::Delay(5),
                SndSeqCommand::DelayRand { min: 3, max: 9 },
                SndSeqCommand::Volume(50),
                SndSeqCommand::StopSound("PlatformStop".to_owned()),
                SndSeqCommand::End,
            ]
        );
        assert!(seq.warnings().is_empty());
    }
}

#[test]
fn q2_nested_colon() {
    let text = ":First\nplay X\n:Second\nplay Y\nend\n";
    let err = SndSeq::parse(text.as_bytes(), &ParseOptions::strict()).unwrap_err();
    assert!(matches!(err, AudioError::SndSeqNestedSequence { .. }));

    let seq = SndSeq::parse(text.as_bytes(), &ParseOptions::lenient()).expect("recovers");
    assert_eq!(seq.sequences().len(), 2);
    assert_eq!(seq.sequences()[0].name, "First");
    assert_eq!(
        seq.sequences()[0].commands,
        vec![SndSeqCommand::Play("X".to_owned())]
    );
    assert_eq!(seq.sequences()[1].name, "Second");
    assert_eq!(
        seq.sequences()[1].commands,
        vec![SndSeqCommand::Play("Y".to_owned()), SndSeqCommand::End]
    );
    assert_eq!(seq.warnings().len(), 1);
    assert!(matches!(
        seq.warnings()[0],
        AudioWarning::SndSeqNestedSequence { .. }
    ));
}

#[test]
fn q2_unknown_command() {
    let text = ":A\nbogus\nend\n";
    let err = SndSeq::parse(text.as_bytes(), &ParseOptions::strict()).unwrap_err();
    assert!(matches!(err, AudioError::SndSeqUnknownCommand { .. }));

    let seq = SndSeq::parse(text.as_bytes(), &ParseOptions::lenient()).expect("recovers");
    assert_eq!(seq.sequences().len(), 1);
    assert_eq!(seq.sequences()[0].name, "A");
    assert_eq!(seq.sequences()[0].commands, vec![SndSeqCommand::End]);
    assert_eq!(seq.warnings().len(), 1);
    assert!(matches!(
        seq.warnings()[0],
        AudioWarning::SndSeqUnknownCommand { .. }
    ));
}

#[test]
fn q2_command_before_any_sequence() {
    let text = "stray\n:A\nend\n";
    let err = SndSeq::parse(text.as_bytes(), &ParseOptions::strict()).unwrap_err();
    assert!(matches!(
        err,
        AudioError::SndSeqCommandOutsideSequence { .. }
    ));

    let seq = SndSeq::parse(text.as_bytes(), &ParseOptions::lenient()).expect("recovers");
    assert_eq!(seq.sequences().len(), 1);
    assert_eq!(seq.sequences()[0].name, "A");
    assert_eq!(seq.sequences()[0].commands, vec![SndSeqCommand::End]);
    assert_eq!(seq.warnings().len(), 1);
    assert!(matches!(
        seq.warnings()[0],
        AudioWarning::SndSeqCommandOutsideSequence { .. }
    ));
}

#[test]
fn q2_eof_without_end() {
    let text = ":A\nplay X\n";
    let err = SndSeq::parse(text.as_bytes(), &ParseOptions::strict()).unwrap_err();
    assert!(matches!(err, AudioError::SndSeqUnterminatedSequence { .. }));

    let seq = SndSeq::parse(text.as_bytes(), &ParseOptions::lenient()).expect("recovers");
    assert_eq!(seq.sequences().len(), 1);
    assert_eq!(
        seq.sequences()[0].commands,
        vec![SndSeqCommand::Play("X".to_owned())]
    );
    assert_eq!(seq.warnings().len(), 1);
    assert!(matches!(
        seq.warnings()[0],
        AudioWarning::SndSeqUnterminatedSequence { .. }
    ));
}

#[test]
fn q2_non_numeric_tics() {
    let text = ":A\nplaytime Snd notanum\nend\n";
    let err = SndSeq::parse(text.as_bytes(), &ParseOptions::strict()).unwrap_err();
    assert!(matches!(err, AudioError::SndSeqBadNumber { .. }));

    let seq = SndSeq::parse(text.as_bytes(), &ParseOptions::lenient()).expect("recovers");
    // The malformed `playtime` is skipped; `end` still closes the sequence.
    assert_eq!(seq.sequences().len(), 1);
    assert_eq!(seq.sequences()[0].commands, vec![SndSeqCommand::End]);
    assert_eq!(seq.warnings().len(), 1);
    assert!(matches!(
        seq.warnings()[0],
        AudioWarning::SndSeqBadNumber { .. }
    ));
}

#[test]
fn q_missing_argument_at_eof() {
    // `play` with no sound token before the end of the lump.
    let text = ":A\nplay\n";
    let err = SndSeq::parse(text.as_bytes(), &ParseOptions::strict()).unwrap_err();
    assert!(matches!(err, AudioError::SndSeqMissingArgument { .. }));

    let seq = SndSeq::parse(text.as_bytes(), &ParseOptions::lenient()).expect("recovers");
    // The partial `play` is dropped; the sequence stays open at EOF, so it is
    // kept with an unterminated warning alongside the missing-argument one.
    assert_eq!(seq.sequences().len(), 1);
    assert!(seq.sequences()[0].commands.is_empty());
    assert!(
        seq.warnings()
            .iter()
            .any(|w| matches!(w, AudioWarning::SndSeqMissingArgument { .. }))
    );
    assert!(
        seq.warnings()
            .iter()
            .any(|w| matches!(w, AudioWarning::SndSeqUnterminatedSequence { .. }))
    );
}

#[test]
fn sndseq_empty_lump_ok() {
    for opts in BOTH_MODES {
        let seq = SndSeq::parse(&[], &opts()).expect("empty SNDSEQ parses");
        assert!(seq.sequences().is_empty());
        assert!(seq.warnings().is_empty());
    }
}

// ---------------------------------------------------------------------------
// Retail sweep anchor (ADR-0023 §3, §5)
// ---------------------------------------------------------------------------

/// Every lump NAMED `SNDINFO`/`SNDSEQ`/`SNDCURVE` in the collection must parse
/// strict with **zero** warnings. A retail anomaly (a strict failure or any
/// warning) is an adjudication for the reviewer, not something to loosen away.
///
/// Expected from the ADR survey: 3 SNDINFO (HEXEN, HEXDD, `hexdd_ex`), 1 SNDSEQ
/// (HEXEN), 2 SNDCURVE (HERETIC, HEXEN).
#[cfg(feature = "sweep-tests")]
#[test]
fn retail_scripts_strict_clean() {
    use crustywad::Wad;

    let paths = common::wad_files("CRUSTYWAD_SWEEP_DIR");
    if paths.is_empty() {
        return; // skip note already printed by the helper
    }

    let mut sndinfo_parsed = 0usize;
    let mut sndseq_parsed = 0usize;
    let mut sndcurve_parsed = 0usize;
    // Collect every anomaly (not fail-fast) so a single run surfaces the whole
    // retail adjudication set for the reviewer.
    let mut anomalies: Vec<String> = Vec::new();

    for path in &paths {
        let wad = Wad::from_path_with_options(path, ParseOptions::strict())
            .unwrap_or_else(|e| panic!("{}: container failed strict parse: {e}", path.display()));
        let wad_name = path.display();

        for i in 0..wad.lump_count() {
            let bytes = wad.lump_bytes(i).expect("lump index in range");
            let name = wad.lump(i).expect("lump index in range").name().to_owned();

            match name.as_str() {
                "SNDINFO" => match SndInfo::parse(bytes, &ParseOptions::strict()) {
                    Ok(info) => {
                        sndinfo_parsed += 1;
                        if !info.warnings().is_empty() {
                            anomalies
                                .push(format!("{wad_name}: SNDINFO warned: {:?}", info.warnings()));
                        }
                    }
                    Err(e) => anomalies.push(format!("{wad_name}: SNDINFO strict failed: {e}")),
                },
                "SNDSEQ" => match SndSeq::parse(bytes, &ParseOptions::strict()) {
                    Ok(seq) => {
                        sndseq_parsed += 1;
                        if !seq.warnings().is_empty() {
                            anomalies
                                .push(format!("{wad_name}: SNDSEQ warned: {:?}", seq.warnings()));
                        }
                    }
                    Err(e) => anomalies.push(format!("{wad_name}: SNDSEQ strict failed: {e}")),
                },
                "SNDCURVE" => match SndCurve::parse(bytes, &ParseOptions::strict()) {
                    Ok(curve) => {
                        sndcurve_parsed += 1;
                        if !curve.warnings().is_empty() {
                            anomalies.push(format!(
                                "{wad_name}: SNDCURVE warned: {:?}",
                                curve.warnings()
                            ));
                        }
                    }
                    Err(e) => anomalies.push(format!("{wad_name}: SNDCURVE strict failed: {e}")),
                },
                _ => {}
            }
        }
    }

    assert!(sndinfo_parsed > 0, "sweep found no SNDINFO lumps");
    assert!(sndseq_parsed > 0, "sweep found no SNDSEQ lumps");
    assert!(sndcurve_parsed > 0, "sweep found no SNDCURVE lumps");
    eprintln!(
        "scripts sweep: {} WAD(s), {sndinfo_parsed} SNDINFO, {sndseq_parsed} SNDSEQ, \
         {sndcurve_parsed} SNDCURVE",
        paths.len()
    );
    assert!(
        anomalies.is_empty(),
        "scripts sweep found {} strict failure(s)/warning(s):\n{}",
        anomalies.len(),
        anomalies.join("\n")
    );
}

// ---------------------------------------------------------------------------
// Branch-coverage fixtures: tokenizer corners and argument-truncation paths
// ---------------------------------------------------------------------------

#[test]
fn cov_quoted_token_newline_and_oversized() {
    // A quoted value spanning a newline stays one token (and keeps the line
    // count accurate for the entry after it); a quoted token beyond the
    // engine's 63-byte truncation limit aggregates into OversizedTokens.
    let long = "x".repeat(70);
    let text = format!("tagone \"two\nwords\"\ntagtwo \"{long}\"\n");
    for opts in BOTH_MODES {
        let info = SndInfo::parse(text.as_bytes(), &opts()).expect("parses");
        assert_eq!(info.entries().len(), 2);
        assert_eq!(info.entries()[0].lump, "two\nwords");
        assert_eq!(info.entries()[1].lump, long);
        assert_eq!(
            info.warnings(),
            &[AudioWarning::OversizedTokens { count: 1 }]
        );
    }
}

#[test]
fn cov_sndinfo_directive_truncations() {
    // Each of the three EOF-truncation spots: $ARCHIVEPATH with no value,
    // $MAP with no number, $MAP with a number but no song lump.
    for text in ["$ARCHIVEPATH", "$MAP", "$MAP 5"] {
        let err = SndInfo::parse(text.as_bytes(), &ParseOptions::strict()).unwrap_err();
        assert!(
            matches!(err, AudioError::SndInfoMissingValue { .. }),
            "{text:?} strict"
        );

        let info = SndInfo::parse(text.as_bytes(), &ParseOptions::lenient()).expect("recovers");
        assert!(info.entries().is_empty());
        assert!(info.map_songs().is_empty());
        assert!(
            matches!(info.warnings()[0], AudioWarning::SndInfoMissingValue { .. }),
            "{text:?} lenient"
        );
    }
}

#[test]
fn cov_sndseq_playuntildone() {
    let text = ":Door\nplayuntildone DoorOpen\nend\n";
    for opts in BOTH_MODES {
        let seq = SndSeq::parse(text.as_bytes(), &opts()).expect("parses");
        assert_eq!(seq.sequences().len(), 1);
        assert_eq!(
            seq.sequences()[0].commands,
            [
                SndSeqCommand::PlayUntilDone("DoorOpen".to_owned()),
                SndSeqCommand::End,
            ]
        );
        assert!(seq.warnings().is_empty());
    }
}

#[test]
fn cov_sndseq_missing_argument_per_command() {
    // The `?` edge of every argument-taking arm: each command as the final
    // token before EOF.
    for cmd in [
        "play",
        "playuntildone",
        "playrepeat",
        "stopsound",
        "playtime",
        "delay",
        "volume",
        "delayrand",
    ] {
        let text = format!(":Door\n{cmd}");
        let err = SndSeq::parse(text.as_bytes(), &ParseOptions::strict()).unwrap_err();
        assert!(
            matches!(err, AudioError::SndSeqMissingArgument { .. }),
            "{cmd} strict"
        );
        let seq = SndSeq::parse(text.as_bytes(), &ParseOptions::lenient()).expect("recovers");
        assert!(
            seq.warnings()
                .iter()
                .any(|w| matches!(w, AudioWarning::SndSeqMissingArgument { .. })),
            "{cmd} lenient"
        );
    }
}

#[test]
fn cov_sndseq_missing_numeric_argument() {
    let text = ":Door\ndelay";
    let err = SndSeq::parse(text.as_bytes(), &ParseOptions::strict()).unwrap_err();
    assert!(matches!(err, AudioError::SndSeqMissingArgument { .. }));

    let seq = SndSeq::parse(text.as_bytes(), &ParseOptions::lenient()).expect("recovers");
    assert_eq!(seq.sequences().len(), 1);
    assert!(seq.sequences()[0].commands.is_empty());
    assert!(
        seq.warnings()
            .iter()
            .any(|w| matches!(w, AudioWarning::SndSeqMissingArgument { .. }))
    );
}

#[test]
fn cov_sndseq_oversized_token() {
    let long = "y".repeat(70);
    let text = format!(":Door\nplay {long}\nend\n");
    for opts in BOTH_MODES {
        let seq = SndSeq::parse(text.as_bytes(), &opts()).expect("parses");
        assert_eq!(
            seq.sequences()[0].commands[0],
            SndSeqCommand::Play(long.clone())
        );
        assert_eq!(
            seq.warnings(),
            &[AudioWarning::OversizedTokens { count: 1 }]
        );
    }
}
