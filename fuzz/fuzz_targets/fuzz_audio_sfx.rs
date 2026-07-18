#![no_main]
use libfuzzer_sys::fuzz_target;

use crustywad::ParseOptions;
use crustywad::audio::{AudioKind, DmxSound, PcSpeakerSound};

fuzz_target!(|data: &[u8]| {
    let kind = AudioKind::detect(data);

    let dmx_strict = DmxSound::parse(data, &ParseOptions::strict());
    let dmx_lenient = DmxSound::parse(data, &ParseOptions::lenient());
    let pc_strict = PcSpeakerSound::parse(data, &ParseOptions::strict());
    let pc_lenient = PcSpeakerSound::parse(data, &ParseOptions::lenient());

    // Oracles (ADR-0016): output and work bounded by the input.
    for snd in dmx_strict.iter().chain(dmx_lenient.iter()) {
        assert!(snd.payload().len() <= data.len());
        assert!(snd.samples().len() <= data.len());
        assert!(snd.warnings().len() <= 4);
    }
    for snd in pc_strict.iter().chain(pc_lenient.iter()) {
        assert!(snd.tones().len() <= data.len());
        assert!(snd.warnings().len() <= 4);
    }

    // Coherence: a positive classification guarantees the strict parse succeeds.
    if kind == AudioKind::Dmx {
        assert!(dmx_strict.is_ok());
    }
    if kind == AudioKind::PcSpeaker {
        assert!(pc_strict.is_ok());
    }
});
