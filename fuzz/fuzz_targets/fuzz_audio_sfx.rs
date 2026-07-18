#![no_main]
use libfuzzer_sys::fuzz_target;

use crustywad::ParseOptions;
use crustywad::audio::{AudioKind, DmxSound, PcSpeakerSound};

fuzz_target!(|data: &[u8]| {
    let kind = AudioKind::detect(data);

    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        if let Ok(snd) = DmxSound::parse(data, &options) {
            // Oracles (ADR-0016): output and work bounded by the input.
            assert!(snd.payload().len() <= data.len());
            assert!(snd.samples().len() <= data.len());
            assert!(snd.warnings().len() <= 4);
        }
        if let Ok(snd) = PcSpeakerSound::parse(data, &options) {
            assert!(snd.tones().len() <= data.len());
            assert!(snd.warnings().len() <= 4);
        }
    }

    // Coherence: a positive classification guarantees the strict parse succeeds.
    if kind == AudioKind::Dmx {
        assert!(DmxSound::parse(data, &ParseOptions::strict()).is_ok());
    }
    if kind == AudioKind::PcSpeaker {
        assert!(PcSpeakerSound::parse(data, &ParseOptions::strict()).is_ok());
    }
});
