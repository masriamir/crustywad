//! Doom 64 audio sweep (ADR-0023 §4, #303): the KEX-remaster `DOOM64.WAD`
//! brackets its music in a `DM_START..DM_END` [`SectionKind::Music`] section
//! (24 standard-MIDI lumps) and its sound effects in `DS_START..DS_END`
//! ([`SectionKind::Sounds`], 93 canonical PCM WAVs). This sweep pins that
//! shape across the retail collection: every Music-section lump is
//! content-detected MIDI and parses [`MidiInfo`] strict-clean, every
//! Sounds-section lump is content-detected WAV and parses [`WavSound`]
//! strict-clean, with the ADR-surveyed per-WAD counts.

#[cfg(feature = "sweep-tests")]
mod common;

/// Retail Doom 64 audio-container sweep (ADR-0023 §4). Gated by the
/// `sweep-tests` feature and `CRUSTYWAD_SWEEP_DIR`; skips gracefully when the
/// variable is unset (the helper prints the skip note). Container parse is
/// strict; the section scan is LENIENT (the gfx-sweep precedent —
/// `SVE.wad`'s bare top-level `P3_START` strict-errors the scan). A retail
/// anomaly — a strict failure, any warning, a content-detection mismatch, or
/// a miscount — is an adjudication for the reviewer, not something to loosen.
#[cfg(feature = "sweep-tests")]
#[test]
#[allow(clippy::too_many_lines)]
fn retail_doom64_audio_containers_strict_clean() {
    use crustywad::audio::{AudioKind, MidiInfo, WavSound};
    use crustywad::{ParseOptions, SectionKind, Wad};

    // The ADR survey pinned these per-WAD counts for the KEX remaster's
    // DOOM64.WAD (ADR-0023 §4).
    const MUSIC_LUMPS_PER_WAD: usize = 24;
    const SOUND_LUMPS_PER_WAD: usize = 93;

    let paths = common::wad_files("CRUSTYWAD_SWEEP_DIR");
    if paths.is_empty() {
        return; // skip note already printed by the helper
    }

    let mut music_wads = 0usize;
    let mut total_music = 0usize;
    let mut total_sounds = 0usize;

    for path in &paths {
        let wad = Wad::from_path_with_options(path, ParseOptions::strict())
            .unwrap_or_else(|e| panic!("{}: container failed strict parse: {e}", path.display()));
        let sections = wad
            .sections_with_options(ParseOptions::lenient())
            .expect("lenient scan never fails");

        // 1. Skip WADs with no Music section.
        if sections.of_kind(SectionKind::Music).next().is_none() {
            continue;
        }
        music_wads += 1;

        // The scan is lenient only for the sake of OTHER WADs in the dir
        // (SVE.wad); a Music-bearing WAD itself must scan clean.
        assert!(
            sections.warnings().is_empty(),
            "{}: section scan warned on a Music-bearing WAD: {:?}",
            path.display(),
            sections.warnings()
        );

        // 2 + 4. Every Music-section lump: content-detected MIDI, and a
        //        strict, zero-warning MidiInfo parse.
        let mut wad_music = 0usize;
        for section in sections.of_kind(SectionKind::Music) {
            for i in section.lumps.clone() {
                let bytes = wad.lump_bytes(i).expect("lump index in range");
                let name = wad.lump(i).expect("lump index in range").name().to_owned();
                assert_eq!(
                    AudioKind::detect(bytes),
                    AudioKind::Midi,
                    "{}: Music-section lump {name} does not content-detect as MIDI",
                    path.display()
                );
                let midi = MidiInfo::parse(bytes, &ParseOptions::strict()).unwrap_or_else(|e| {
                    panic!(
                        "{}: Music lump {name} failed strict MidiInfo::parse: {e}",
                        path.display()
                    )
                });
                assert!(
                    midi.warnings().is_empty(),
                    "{}: Music lump {name} parsed with warnings: {:?}",
                    path.display(),
                    midi.warnings()
                );
                wad_music += 1;
            }
        }

        // 3 + 4. Every Sounds-section lump: content-detected WAV, and a
        //        strict, zero-warning WavSound parse.
        let mut wad_sounds = 0usize;
        for section in sections.of_kind(SectionKind::Sounds) {
            for i in section.lumps.clone() {
                let bytes = wad.lump_bytes(i).expect("lump index in range");
                let name = wad.lump(i).expect("lump index in range").name().to_owned();
                assert_eq!(
                    AudioKind::detect(bytes),
                    AudioKind::Wav,
                    "{}: Sounds-section lump {name} does not content-detect as WAV",
                    path.display()
                );
                let wav = WavSound::parse(bytes, &ParseOptions::strict()).unwrap_or_else(|e| {
                    panic!(
                        "{}: Sounds lump {name} failed strict WavSound::parse: {e}",
                        path.display()
                    )
                });
                assert!(
                    wav.warnings().is_empty(),
                    "{}: Sounds lump {name} parsed with warnings: {:?}",
                    path.display(),
                    wav.warnings()
                );
                wad_sounds += 1;
            }
        }

        // 5. Per-WAD counts, asserted for every WAD that has a Music section
        //    (a second D64-style WAD in the dir would be checked too).
        assert_eq!(
            wad_music,
            MUSIC_LUMPS_PER_WAD,
            "{}: expected {MUSIC_LUMPS_PER_WAD} Music lumps, saw {wad_music}",
            path.display()
        );
        assert_eq!(
            wad_sounds,
            SOUND_LUMPS_PER_WAD,
            "{}: expected {SOUND_LUMPS_PER_WAD} Sounds lumps, saw {wad_sounds}",
            path.display()
        );
        total_music += wad_music;
        total_sounds += wad_sounds;
    }

    // At least one WAD had a Music section.
    assert!(
        music_wads > 0,
        "doom64 audio sweep found no WAD with a Music (DM_) section"
    );
    eprintln!(
        "doom64 audio sweep: {} WAD(s) scanned, {music_wads} with a Music section, \
         {total_music} MIDI music lump(s), {total_sounds} WAV sound(s), all strict-clean",
        paths.len()
    );
}
