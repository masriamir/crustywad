#![no_main]
use libfuzzer_sys::fuzz_target;

use crustywad::ParseOptions;
use crustywad::audio::{AudioKind, Dmxgus, Genmidi, MidiInfo, MusScore, WavSound};

fuzz_target!(|data: &[u8]| {
    // Detection is computed but deliberately *not* used as a parse oracle for
    // Mus/Midi/Wav: unlike Dmx/PcSpeaker (see fuzz_audio_sfx), these are
    // magic-only classifications — the leading bytes match but the full
    // structural parse can still fail — so a positive detect guarantees
    // nothing beyond no-panic here.
    let _ = std::hint::black_box(AudioKind::detect(data));

    for opts in [ParseOptions::strict(), ParseOptions::lenient()] {
        if let Ok(mus) = MusScore::parse(data, &opts) {
            // Oracles (ADR-0016): output and warnings bounded by the input.
            assert!(mus.events().len() <= data.len());
            assert!(mus.warnings().len() <= data.len() + 8);
        }
        if let Ok(midi) = MidiInfo::parse(data, &opts) {
            // Each track needs an 8-byte chunk header.
            assert!(midi.tracks().len() <= data.len() / 8 + 1);
            assert!(midi.warnings().len() <= data.len() + 8);
        }
        if let Ok(wav) = WavSound::parse(data, &opts) {
            assert!(wav.data().len() <= data.len());
            assert!(wav.warnings().len() <= data.len() + 8);
        }
        if let Ok(bank) = Genmidi::parse(data, &opts) {
            // Records and names are fixed-count; only warnings can grow.
            assert!(bank.warnings().len() <= data.len() + 8);
        }
        if let Ok(gus) = Dmxgus::parse(data, &opts) {
            // At most one entry per line; lines <= bytes + 1.
            assert!(gus.entries().len() <= data.len() + 1);
            // Warning growth is O(input): alien chunks and malformed/extra-field
            // lines each consume input bytes (unlike sfx's O(1) warnings).
            assert!(gus.warnings().len() <= data.len() + 8);
        }
    }
});
