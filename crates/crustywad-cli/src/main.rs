//! Command-line tool for inspecting Doom WAD files.

mod cli;
mod mus2mid;

use std::borrow::Cow;
use std::collections::HashMap;
use std::fs;
use std::process;

use std::fmt::Write as _;

use anyhow::{Context as _, Result};
use clap::Parser as _;
use crustywad::audio::{AudioKind, DmxSound, MidiInfo, MusScore, WavSound};
use crustywad::{ParseOptions, Wad, WadBuilder, WadKind};

use cli::{Cli, Format, MapFormatArg, NodeFormatArg, SubCommand, WadKindArg};

/// Assembles every map group in `wad` under `options`, reporting per-map
/// results (#251).
///
/// Validation continues past failing maps so one corrupt map cannot mask
/// another. Per-map diagnostics go to stderr in every output format (stdout
/// stays machine-readable); JSON additionally emits one newline-delimited
/// record per map followed by the same summary object the shallow mode
/// prints, and CSV emits a `map,ok,error` table. Exits `0` when every
/// map assembles (lenient-mode warnings allowed, reported on stderr) and `1`
/// when any map fails — ADR-0008 §2's "negative result: validation errors
/// found" code, distinct from exit `2` (the container itself is unreadable or
/// malformed). Container-level lenient warnings print after the summary, per
/// ADR-0008 §3.
fn deep_validate(wad: &Wad, path: &std::path::Path, format: Format, options: ParseOptions) -> i32 {
    use crustywad::map::Map;

    let groups = wad.map_groups();
    let mut failed = 0usize;

    if matches!(format, Format::Csv) {
        println!("map,ok,error");
    }
    for group in &groups {
        match Map::assemble_with_options(wad, group, options) {
            Ok(map) => {
                for w in map.warnings() {
                    eprintln!("warning: map {}: {w}", group.name);
                }
                match format {
                    Format::Human => {}
                    Format::Json => println!(
                        r#"{{"map":{},"ok":true,"warnings":{}}}"#,
                        json_string(&group.name),
                        map.warnings().len()
                    ),
                    Format::Csv => {
                        println!("{},true,", csv_field(&group.name));
                    }
                }
            }
            Err(e) => {
                failed += 1;
                eprintln!("error: map {}: {e:#}", group.name);
                match format {
                    Format::Human => {}
                    Format::Json => println!(
                        r#"{{"map":{},"ok":false,"error":{}}}"#,
                        json_string(&group.name),
                        json_string(&e.to_string())
                    ),
                    Format::Csv => {
                        println!(
                            "{},false,{}",
                            csv_field(&group.name),
                            csv_field(&e.to_string())
                        );
                    }
                }
            }
        }
    }

    let code = if failed == 0 {
        match format {
            Format::Human => {
                println!("ok: {} ({} map(s) validated)", path.display(), groups.len());
            }
            Format::Json => println!(r#"{{"ok":true}}"#),
            Format::Csv => {}
        }
        0
    } else {
        let summary = format!("{failed} of {} map(s) failed validation", groups.len());
        match format {
            Format::Human => eprintln!("error: {}: {summary}", path.display()),
            Format::Json => {
                println!(r#"{{"ok":false,"error":{}}}"#, json_string(&summary));
            }
            Format::Csv => {}
        }
        1
    };
    // ADR-0008 §3: container-level lenient warnings print after the result.
    for w in wad.warnings() {
        eprintln!("warning: {w}");
    }
    code
}

/// Returns the marker-lump names of every map group in `wad`, in directory
/// order.
///
/// Delegates to [`Wad::map_groups`] so the CLI and the library can never
/// disagree about what counts as a map (#253): a map is a marker lump followed
/// by a recognized data-lump run — whatever the marker is named — or a Doom 64
/// nested-WAD `MAPxx` lump (ADR-0021 §1). Unlike the name-pattern heuristic
/// this replaces, a stray map-named lump with no data run is not reported.
/// The names come straight from the `name` field of each
/// [`MapGroup`][crustywad::map::MapGroup] — the library's own record of the
/// map's identity.
fn detect_maps(wad: &Wad) -> Vec<String> {
    wad.map_groups()
        .into_iter()
        .map(|group| group.name)
        .collect()
}

/// Windows device names that are reserved regardless of file extension.
const WINDOWS_RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Converts a raw lump name to a safe, uppercase filename component.
///
/// Replaces any character that is not ASCII alphanumeric, `_`, or `-` with
/// `_`, preventing path traversal from lump names that contain `/`, `\`, or
/// other special characters. The result is then uppercased so that lump names
/// differing only in case (e.g. `PATCH` and `patch`) map to the same key and
/// are correctly deduplicated on case-insensitive filesystems (Windows/macOS).
/// Returns `"UNNAMED"` for empty inputs.
///
/// Windows-reserved device names (`CON`, `PRN`, `AUX`, `NUL`, `COM1`–`COM9`,
/// `LPT1`–`LPT9`) are prefixed with `_` so extraction succeeds on all
/// platforms.
fn sanitize_lump_name(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .to_ascii_uppercase();
    if s.is_empty() {
        return String::from("UNNAMED");
    }
    if WINDOWS_RESERVED.iter().any(|r| s.eq_ignore_ascii_case(r)) {
        format!("_{s}")
    } else {
        s
    }
}

/// Encodes a string as a JSON string literal (including surrounding `"`).
/// Uses standard JSON `\uXXXX` escapes for control characters, ensuring
/// output is always valid JSON regardless of the input content.
fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => write!(out, "\\u{:04x}", c as u32).unwrap(),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Lump names that a map conversion accounts for — those it either *reads* to
/// build the `Map` graph or *emits* as part of the converted map, and which are
/// therefore safe to replace rather than carry across.
///
/// Note the two roles are not the same set. Assembly reads only `THINGS`,
/// `LINEDEFS`, `SIDEDEFS`, `VERTEXES`, and `SECTORS`; the node lumps
/// (`SEGS`/`SSECTORS`/`NODES`/`REJECT`/`BLOCKMAP`) are never consumed, and
/// appear here because `add_doom_map` emits them (empty — see the
/// `NodesNotBuilt` warning). `TEXTMAP`/`ENDMAP` are the UDMF equivalents,
/// read by assembly and emitted by `add_udmf_map`.
///
/// The classification is by name, and is deliberately narrower than the
/// library's map-group membership rule (which also admits `BEHAVIOR` and any
/// lump appearing between `TEXTMAP` and `ENDMAP`). Anything else found inside a
/// map group — `BEHAVIOR` (compiled ACS), `SCRIPTS`, `ZNODES`, `DIALOGUE`, GL
/// node lumps — is bound to the *source* map's specials or geometry, so it is
/// neither converted nor copied into the converted map; see
/// [`dropped_group_lumps`].
const CONVERTED_MAP_LUMPS: &[&str] = &[
    "THINGS", "LINEDEFS", "SIDEDEFS", "VERTEXES", "SEGS", "SSECTORS", "NODES", "SECTORS", "REJECT",
    "BLOCKMAP", "TEXTMAP", "ENDMAP",
];

/// Returns the names of the lumps in `group` that a conversion would drop —
/// every data lump whose name is not in [`CONVERTED_MAP_LUMPS`] — in directory
/// order, preserving duplicates.
///
/// Carrying such a lump through into the converted map would be worse than
/// dropping it: a `BEHAVIOR` lump is compiled ACS bound to the source map's
/// linedef/thing specials, and node lumps describe the source geometry, so a
/// pass-through would look intact while being subtly wrong.
fn dropped_group_lumps(wad: &Wad, group: &crustywad::map::MapGroup) -> Vec<String> {
    group
        .data_indices
        .iter()
        .filter_map(|&i| wad.lump(i))
        .map(crustywad::Lump::name)
        .filter(|name| !CONVERTED_MAP_LUMPS.contains(name))
        .map(ToOwned::to_owned)
        .collect()
}

/// A map writer's refusal, normalized across the Doom and UDMF writers for
/// `cwad convert` error reporting: the rendered message plus the strictness
/// classification the follow-up note dispatches on (#264) — the `--lenient`
/// hint is only honest when lenient mode actually recovers the error.
struct Refusal {
    /// The writer error's `Display` rendering.
    message: String,
    /// Whether re-running with `--lenient` turns this error into warnings.
    lenient_recoverable: bool,
}

/// Groups each lump's data slice by name; duplicate names accumulate in directory order.
fn lump_data_map(wad: &Wad) -> HashMap<String, Vec<&[u8]>> {
    let mut map: HashMap<String, Vec<&[u8]>> = HashMap::new();
    for lump in wad.lumps() {
        let data = wad.lump_data(lump);
        if let Some(vec) = map.get_mut(lump.name()) {
            vec.push(data);
        } else {
            map.insert(lump.name().to_owned(), vec![data]);
        }
    }
    map
}

/// Classifies a single lump-level difference found by `cwad diff`.
#[derive(Debug)]
enum DiffKind {
    /// The lump exists only in the first WAD.
    OnlyInFirst,
    /// The lump exists only in the second WAD.
    OnlyInSecond,
    /// The lump name exists in both WADs but the per-name sequence of data slices differs
    /// (different data, different duplicate count, or different duplicate order).
    Changed,
}

/// Escapes a field value per RFC 4180: wraps in double-quotes if the value
/// contains a comma, double-quote, or newline; internal double-quotes are
/// escaped by doubling them.
fn csv_field(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_owned()
    }
}

/// Wraps a parsed DMX sound's PCM samples in a canonical 44-byte RIFF/WAVE
/// header (ADR-0023 §2 / issue #304).
///
/// DMX samples are unsigned 8-bit mono PCM, so the container is fixed at
/// `bits_per_sample = 8`, `channels = 1`, `block_align = 1`, and
/// `byte_rate = sample_rate`. Every size is derived from the pad-stripped
/// sample span: `data_len = samples.len()` and `riff_size = 36 + data_len`.
fn dmx_to_wav(sound: &DmxSound) -> Vec<u8> {
    let samples = sound.samples();
    let data_len = u32::try_from(samples.len()).unwrap_or(u32::MAX);
    let sample_rate = u32::from(sound.sample_rate());
    let riff_size = 36u32.saturating_add(data_len);

    let mut wav = Vec::with_capacity(44 + samples.len());
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&riff_size.to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
    wav.extend_from_slice(&1u16.to_le_bytes()); // audio format: PCM
    wav.extend_from_slice(&1u16.to_le_bytes()); // channels: mono
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes()); // byte_rate = rate * 1 * 1
    wav.extend_from_slice(&1u16.to_le_bytes()); // block_align: 1 byte/frame
    wav.extend_from_slice(&8u16.to_le_bytes()); // bits_per_sample: 8
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    wav.extend_from_slice(samples);
    wav
}

/// The default filename extension for a lump with no audio-aware container.
const DEFAULT_EXTENSION: &str = "bin";

/// Chooses the extraction output(s) for one lump: a list of
/// `(extension, bytes)` pairs to write. Classification is by content
/// ([`AudioKind::detect`]), never by lump name; audio parses are lenient (an
/// extract tool should extract). A MUS lump that fails even a lenient parse
/// (its detection is magic-only, unlike DMX, whose detection guarantees a
/// successful parse — the DMX fallback arm below is purely defensive) falls
/// back to a raw `.bin` write and pushes a warning onto `warnings`.
///
/// `raw_name` is the lump's on-disk name, used only for warning messages.
fn extract_outputs<'a>(
    data: &'a [u8],
    raw_name: &str,
    midi: bool,
    warnings: &mut Vec<String>,
) -> Vec<(&'static str, Cow<'a, [u8]>)> {
    let opts = ParseOptions::lenient();
    match AudioKind::detect(data) {
        AudioKind::Dmx => match DmxSound::parse(data, &opts) {
            Ok(sound) => vec![("wav", Cow::Owned(dmx_to_wav(&sound)))],
            // Unreachable in practice: a Dmx detection guarantees the strict
            // (and so the lenient) parse succeeds. Kept as a defensive
            // fallback rather than an unwrap.
            Err(e) => {
                warnings.push(format!(
                    "{raw_name}: could not decode DMX sound ({e}); extracted raw bytes"
                ));
                vec![(DEFAULT_EXTENSION, Cow::Borrowed(data))]
            }
        },
        AudioKind::Mus => match MusScore::parse(data, &opts) {
            Ok(score) => {
                // The raw MUS bytes are always emitted; --midi adds the
                // converted format-0 SMF from the typed event stream.
                let mut outputs: Vec<(&'static str, Cow<'a, [u8]>)> =
                    vec![("mus", Cow::Borrowed(data))];
                if midi {
                    outputs.push(("mid", Cow::Owned(mus2mid::convert(&score))));
                }
                outputs
            }
            Err(e) => {
                warnings.push(format!(
                    "{raw_name}: could not decode MUS score ({e}); extracted raw bytes"
                ));
                vec![(DEFAULT_EXTENSION, Cow::Borrowed(data))]
            }
        },
        // Standard MIDI and RIFF/WAVE already are their own containers, but
        // MIDI detection is magic-only (4 bytes), so a truncated lump must
        // take the same warned raw fallback as MUS rather than masquerade as
        // a playable `.mid`.
        AudioKind::Midi => match MidiInfo::parse(data, &opts) {
            Ok(_) => vec![("mid", Cow::Borrowed(data))],
            Err(e) => {
                warnings.push(format!(
                    "{raw_name}: could not index MIDI chunks ({e}); extracted raw bytes"
                ));
                vec![(DEFAULT_EXTENSION, Cow::Borrowed(data))]
            }
        },
        // A 12-byte WAV detection always parses leniently (warnings at
        // worst), so the passthrough needs no fallback arm.
        AudioKind::Wav => vec![("wav", Cow::Borrowed(data))],
        // PC-speaker data has no standard container, `Unknown` is not audio,
        // and (`AudioKind` being `#[non_exhaustive]`) any future variant is
        // unhandled: all extract raw under the default extension.
        _ => vec![(DEFAULT_EXTENSION, Cow::Borrowed(data))],
    }
}

/// A per-lump audio classification for `cwad list`/`info`, produced by a
/// lenient parse of a detected audio lump.
struct AudioAnnotation {
    /// The detected kind name (`Dmx`, `Mus`, `Midi`, `Wav`, `PcSpeaker`).
    kind: &'static str,
    /// Human-readable cheap details (e.g. `rate=11025 samples=20`), empty when
    /// the lump is only a kind (parse failed, or `PcSpeaker`).
    detail_human: String,
    /// The same details as JSON object fields, each prefixed with a comma
    /// (e.g. `,"sample_rate":11025,"samples":20`); empty when there are none.
    detail_json: String,
}

impl AudioAnnotation {
    /// An annotation carrying only the kind name (no cheap details).
    fn bare(kind: &'static str) -> Self {
        Self {
            kind,
            detail_human: String::new(),
            detail_json: String::new(),
        }
    }

    /// The `list` human-format suffix, e.g. ` [audio: Dmx rate=11025 samples=20]`.
    fn human_suffix(&self) -> String {
        if self.detail_human.is_empty() {
            format!(" [audio: {}]", self.kind)
        } else {
            format!(" [audio: {} {}]", self.kind, self.detail_human)
        }
    }

    /// The `list` CSV cell, e.g. `Dmx rate=11025 samples=20`.
    fn csv_cell(&self) -> String {
        if self.detail_human.is_empty() {
            self.kind.to_owned()
        } else {
            format!("{} {}", self.kind, self.detail_human)
        }
    }
}

/// Classifies a lump for `cwad list` annotation, or `None` when the bytes are
/// not a recognized audio format.
///
/// Detection ([`AudioKind::detect`]) is cheap and never allocates; a typed
/// parse (always lenient — a display path should never reject) runs only for a
/// detected audio lump, never for [`AudioKind::Unknown`], so the common
/// non-audio path is untouched. A failed parse degrades to the bare kind.
fn audio_annotation(data: &[u8]) -> Option<AudioAnnotation> {
    let opts = ParseOptions::lenient();
    let annotation = match AudioKind::detect(data) {
        AudioKind::Dmx => match DmxSound::parse(data, &opts) {
            Ok(s) => AudioAnnotation {
                kind: "Dmx",
                detail_human: format!("rate={} samples={}", s.sample_rate(), s.samples().len()),
                detail_json: format!(
                    r#","sample_rate":{},"samples":{}"#,
                    s.sample_rate(),
                    s.samples().len()
                ),
            },
            // Unreachable in practice (detect guarantees the parse); a
            // defensive fallback rather than an unwrap.
            Err(_) => AudioAnnotation::bare("Dmx"),
        },
        AudioKind::Mus => match MusScore::parse(data, &opts) {
            Ok(s) => AudioAnnotation {
                kind: "Mus",
                detail_human: format!("events={}", s.events().len()),
                detail_json: format!(r#","events":{}"#, s.events().len()),
            },
            Err(_) => AudioAnnotation::bare("Mus"),
        },
        AudioKind::Midi => match MidiInfo::parse(data, &opts) {
            Ok(s) => AudioAnnotation {
                kind: "Midi",
                detail_human: format!("tracks={}", s.tracks().len()),
                detail_json: format!(r#","tracks":{}"#, s.tracks().len()),
            },
            Err(_) => AudioAnnotation::bare("Midi"),
        },
        AudioKind::Wav => match WavSound::parse(data, &opts) {
            Ok(s) => AudioAnnotation {
                kind: "Wav",
                detail_human: format!(
                    "rate={} channels={} bits={}",
                    s.sample_rate(),
                    s.channels(),
                    s.bits_per_sample()
                ),
                detail_json: format!(
                    r#","sample_rate":{},"channels":{},"bits":{}"#,
                    s.sample_rate(),
                    s.channels(),
                    s.bits_per_sample()
                ),
            },
            // Unreachable leniently (a 12-byte detection always parses with
            // warnings at worst); a defensive fallback rather than an unwrap.
            Err(_) => AudioAnnotation::bare("Wav"),
        },
        AudioKind::PcSpeaker => AudioAnnotation::bare("PcSpeaker"),
        // `AudioKind::Unknown` and (it being `#[non_exhaustive]`) any unmapped
        // future variant are not annotated rather than mislabelled.
        _ => return None,
    };
    Some(annotation)
}

/// Tallies detected audio lumps by kind for the `cwad info` summary, in a
/// stable display order. Only [`AudioKind::detect`] runs (no per-lump parse),
/// and detect inspects at most a few header bytes, so the summary is
/// `O(number of lumps)` and never allocates per lump.
fn audio_summary(wad: &Wad) -> Vec<(&'static str, usize)> {
    let (mut dmx, mut pc, mut mus, mut midi, mut wav) = (0usize, 0, 0, 0, 0);
    for lump in wad.lumps() {
        match AudioKind::detect(wad.lump_data(lump)) {
            AudioKind::Dmx => dmx += 1,
            AudioKind::PcSpeaker => pc += 1,
            AudioKind::Mus => mus += 1,
            AudioKind::Midi => midi += 1,
            AudioKind::Wav => wav += 1,
            _ => {}
        }
    }
    [
        ("Dmx", dmx),
        ("PcSpeaker", pc),
        ("Mus", mus),
        ("Midi", midi),
        ("Wav", wav),
    ]
    .into_iter()
    .filter(|&(_, count)| count > 0)
    .collect()
}

fn main() {
    let cli = match Cli::try_parse() {
        Ok(c) => c,
        Err(e) => {
            use clap::error::ErrorKind;
            if matches!(e.kind(), ErrorKind::DisplayHelp | ErrorKind::DisplayVersion) {
                e.exit();
            }
            let _ = e.print();
            process::exit(3);
        }
    };

    match run(cli) {
        Ok(code) => process::exit(code),
        Err(e) => {
            eprintln!("error: {e:#}");
            process::exit(2);
        }
    }
}

#[allow(clippy::too_many_lines)]
fn run(cli: Cli) -> Result<i32> {
    let options = if cli.lenient {
        ParseOptions::lenient()
    } else {
        ParseOptions::strict()
    };

    match cli.command {
        SubCommand::Info { path } => {
            let wad = Wad::from_path_with_options(&path, options)
                .with_context(|| format!("failed to load {}", path.display()))?;
            let data_size: u64 = wad.lumps().iter().map(|l| l.size() as u64).sum();
            let maps = detect_maps(&wad);
            // Per-kind tally of detected audio lumps (detect-only, no parse).
            let audio = audio_summary(&wad);
            match cli.format {
                Format::Human => {
                    println!("kind:      {:?}", wad.kind());
                    println!("lumps:     {}", wad.lump_count());
                    let unit = if data_size == 1 { "byte" } else { "bytes" };
                    println!("data size: {data_size} {unit}");
                    if !maps.is_empty() {
                        println!("maps:      {}", maps.join(", "));
                    }
                    if !audio.is_empty() {
                        let rendered = audio
                            .iter()
                            .map(|(kind, count)| format!("{kind}: {count}"))
                            .collect::<Vec<_>>()
                            .join(", ");
                        println!("audio:     {rendered}");
                    }
                }
                Format::Json => {
                    let maps_json: String = maps
                        .iter()
                        .map(|m| json_string(m))
                        .collect::<Vec<_>>()
                        .join(",");
                    let audio_json: String = audio
                        .iter()
                        .map(|(kind, count)| format!(r#""{kind}":{count}"#))
                        .collect::<Vec<_>>()
                        .join(",");
                    println!(
                        r#"{{"kind":"{:?}","lumps":{},"data_size":{},"maps":[{}],"audio":{{{}}}}}"#,
                        wad.kind(),
                        wad.lump_count(),
                        data_size,
                        maps_json,
                        audio_json
                    );
                }
                Format::Csv => {
                    println!("kind,lumps,data_size,maps");
                    println!(
                        "{},{},{},{}",
                        csv_field(&format!("{:?}", wad.kind())),
                        wad.lump_count(),
                        data_size,
                        csv_field(&maps.join(" "))
                    );
                }
            }
            for w in wad.warnings() {
                eprintln!("warning: {w}");
            }
            Ok(0)
        }

        SubCommand::List { path } => {
            let wad = Wad::from_path_with_options(&path, options)
                .with_context(|| format!("failed to load {}", path.display()))?;
            // Per-lump audio classification (content-detected, lenient parse);
            // `None` for non-audio lumps, which stay unannotated.
            let annotations: Vec<Option<AudioAnnotation>> = wad
                .lumps()
                .iter()
                .map(|lump| audio_annotation(wad.lump_data(lump)))
                .collect();
            match cli.format {
                Format::Human => {
                    for (i, lump) in wad.lumps().iter().enumerate() {
                        let suffix = annotations[i]
                            .as_ref()
                            .map(AudioAnnotation::human_suffix)
                            .unwrap_or_default();
                        println!(
                            "{i:04} {:>8} {:>8} {}{suffix}",
                            lump.filepos(),
                            lump.size(),
                            lump.name()
                        );
                    }
                }
                Format::Json => {
                    for (i, lump) in wad.lumps().iter().enumerate() {
                        let audio = annotations[i].as_ref().map_or_else(String::new, |a| {
                            format!(r#","audio":{{"kind":"{}"{}}}"#, a.kind, a.detail_json)
                        });
                        println!(
                            r#"{{"index":{i},"filepos":{},"size":{},"name":{}{audio}}}"#,
                            lump.filepos(),
                            lump.size(),
                            json_string(lump.name())
                        );
                    }
                }
                Format::Csv => {
                    println!("index,filepos,size,name,audio");
                    for (i, lump) in wad.lumps().iter().enumerate() {
                        let audio = annotations[i]
                            .as_ref()
                            .map(AudioAnnotation::csv_cell)
                            .unwrap_or_default();
                        println!(
                            "{i},{},{},{},{}",
                            lump.filepos(),
                            lump.size(),
                            csv_field(lump.name()),
                            csv_field(&audio)
                        );
                    }
                }
            }
            for w in wad.warnings() {
                eprintln!("warning: {w}");
            }
            Ok(0)
        }

        SubCommand::Diff { file1, file2 } => {
            let wad1 = Wad::from_path_with_options(&file1, options)
                .with_context(|| format!("failed to load {}", file1.display()))?;
            let wad2 = Wad::from_path_with_options(&file2, options)
                .with_context(|| format!("failed to load {}", file2.display()))?;

            // Collect each distinct lump name in first-seen order across both WADs.
            let mut all_names: Vec<String> = Vec::new();
            let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
            for lump in wad1.lumps().iter().chain(wad2.lumps().iter()) {
                let name = lump.name();
                if seen.insert(name) {
                    all_names.push(name.to_owned());
                }
            }

            let map1 = lump_data_map(&wad1);
            let map2 = lump_data_map(&wad2);

            let mut diffs: Vec<(DiffKind, String)> = Vec::new();
            for name in &all_names {
                match (map1.get(name), map2.get(name)) {
                    (Some(_), None) => diffs.push((DiffKind::OnlyInFirst, name.clone())),
                    (None, Some(_)) => diffs.push((DiffKind::OnlyInSecond, name.clone())),
                    (Some(v1), Some(v2)) if v1 != v2 => {
                        diffs.push((DiffKind::Changed, name.clone()));
                    }
                    _ => {}
                }
            }

            for w in wad1.warnings() {
                eprintln!("warning: {w}");
            }
            for w in wad2.warnings() {
                eprintln!("warning: {w}");
            }

            if diffs.is_empty() {
                return Ok(0);
            }

            match cli.format {
                Format::Human => {
                    for (kind, name) in &diffs {
                        match kind {
                            DiffKind::OnlyInFirst => {
                                println!("Only in {}:  {name}", file1.display());
                            }
                            DiffKind::OnlyInSecond => {
                                println!("Only in {}:  {name}", file2.display());
                            }
                            DiffKind::Changed => {
                                println!("Changed:           {name}");
                            }
                        }
                    }
                }
                Format::Json => {
                    for (kind, name) in &diffs {
                        let kind_str = match kind {
                            DiffKind::OnlyInFirst => "only_in_first",
                            DiffKind::OnlyInSecond => "only_in_second",
                            DiffKind::Changed => "changed",
                        };
                        println!(
                            r#"{{"kind":{kind_json},"name":{name_json}}}"#,
                            kind_json = json_string(kind_str),
                            name_json = json_string(name)
                        );
                    }
                }
                Format::Csv => {
                    println!("kind,name");
                    for (kind, name) in &diffs {
                        let kind_str = match kind {
                            DiffKind::OnlyInFirst => "only_in_first",
                            DiffKind::OnlyInSecond => "only_in_second",
                            DiffKind::Changed => "changed",
                        };
                        println!("{},{}", csv_field(kind_str), csv_field(name));
                    }
                }
            }

            Ok(1)
        }

        SubCommand::Validate { path, deep } => {
            match Wad::from_path_with_options(&path, options) {
                Ok(wad) => {
                    if deep {
                        return Ok(deep_validate(&wad, &path, cli.format, options));
                    }
                    match cli.format {
                        Format::Human => println!("ok: {}", path.display()),
                        Format::Json => println!(r#"{{"ok":true}}"#),
                        Format::Csv => {
                            println!("ok");
                            println!("true");
                        }
                    }
                    // ADR-0008 §3: lenient container warnings print after the
                    // successful result.
                    for w in wad.warnings() {
                        eprintln!("warning: {w}");
                    }
                    Ok(0)
                }
                Err(e) => {
                    // All parse and I/O errors exit 2 per ADR-0008 (malformed WAD = parse error).
                    // Result output goes to stdout; human diagnostic to stderr.
                    match cli.format {
                        Format::Human => eprintln!("error: {}: {e:#}", path.display()),
                        Format::Json => {
                            println!(r#"{{"ok":false,"error":{}}}"#, json_string(&e.to_string()));
                        }
                        Format::Csv => {
                            println!("ok");
                            println!("false");
                        }
                    }
                    Ok(2)
                }
            }
        }

        SubCommand::Merge {
            inputs,
            output,
            kind,
        } => {
            let wad_kind = match kind {
                WadKindArg::Iwad => WadKind::Iwad,
                WadKindArg::Pwad => WadKind::Pwad,
            };
            let mut builder = WadBuilder::new(wad_kind);
            for path in &inputs {
                let wad = Wad::from_path_with_options(path, options)
                    .with_context(|| format!("failed to load {}", path.display()))?;
                for w in wad.warnings() {
                    eprintln!("warning: {}: {w}", path.display());
                }
                for lump in wad.lumps() {
                    builder.add_lump(lump.name(), wad.lump_data(lump));
                }
            }

            let write_opts = if cli.lenient {
                crustywad::WriteOptions::lenient()
            } else {
                crustywad::WriteOptions::strict()
            };

            // Lump-name/size validation failures are usage errors (bad input data),
            // distinct from the I/O failures handled via `?` elsewhere in this arm.
            let (bytes, warnings) = match builder.build_with_options(&write_opts) {
                Ok(result) => result,
                Err(e) => {
                    eprintln!(
                        "error: failed to build merged WAD {}: {e}",
                        output.display()
                    );
                    return Ok(3);
                }
            };

            for w in &warnings {
                eprintln!("warning: {w}");
            }

            std::fs::write(&output, &bytes)
                .with_context(|| format!("failed to write {}", output.display()))?;
            Ok(0)
        }

        SubCommand::Extract {
            path,
            output,
            lump,
            midi,
        } => {
            if !output.is_dir() {
                anyhow::bail!(
                    "output path does not exist or is not a directory: {}",
                    output.display()
                );
            }

            let wad = Wad::from_path_with_options(&path, options)
                .with_context(|| format!("failed to load {}", path.display()))?;

            for w in wad.warnings() {
                eprintln!("warning: {w}");
            }

            // Collect the lumps to extract: either the named lump, or all lumps.
            let indices: Vec<usize> = if let Some(ref name) = lump {
                let found: Vec<usize> = wad
                    .lumps()
                    .iter()
                    .enumerate()
                    .filter(|(_, l)| l.name() == name.as_str())
                    .map(|(i, _)| i)
                    .collect();
                if found.is_empty() {
                    eprintln!("error: lump {name:?} not found in {}", path.display());
                    return Ok(2);
                }
                found
            } else {
                (0..wad.lump_count()).collect()
            };

            // Track how many times each name has already been written so we can
            // generate unique filenames for duplicate lump names.
            let mut name_count: HashMap<String, usize> = HashMap::new();

            if matches!(cli.format, Format::Csv) {
                println!("filename");
            }

            for index in indices {
                let lump_meta = wad
                    .lump(index)
                    .ok_or_else(|| anyhow::anyhow!("lump index {index} out of range"))?;
                let raw_name = lump_meta.name().to_owned();
                let lump_name = sanitize_lump_name(&raw_name);
                let data = wad
                    .lump_bytes(index)
                    .ok_or_else(|| anyhow::anyhow!("lump index {index} out of range"))?;

                // One base name per lump; duplicate safe names get an
                // occurrence suffix. Every output for this lump (e.g. a MUS's
                // `.mus` and `.mid`) shares the base and differs only by
                // extension.
                let count = name_count.entry(lump_name.clone()).or_insert(0);
                let base = if *count == 0 {
                    lump_name.clone()
                } else {
                    format!("{lump_name}_{count}")
                };
                *count += 1;

                let mut extract_warnings = Vec::new();
                let outputs = extract_outputs(data, &raw_name, midi, &mut extract_warnings);
                for w in &extract_warnings {
                    eprintln!("warning: {w}");
                }

                for (extension, bytes) in outputs {
                    let filename = format!("{base}.{extension}");
                    let dest = output.join(&filename);
                    fs::write(&dest, bytes.as_ref())
                        .with_context(|| format!("failed to write {}", dest.display()))?;
                    match cli.format {
                        Format::Human => println!("{filename}"),
                        Format::Json => {
                            println!(r#"{{"filename":{}}}"#, json_string(&filename));
                        }
                        Format::Csv => println!("{}", csv_field(&filename)),
                    }
                }
            }

            Ok(0)
        }

        SubCommand::Build {
            output,
            kind,
            lumps,
            nodes,
        } => {
            let wad_kind = match kind {
                WadKindArg::Iwad => WadKind::Iwad,
                WadKindArg::Pwad => WadKind::Pwad,
            };
            let mut builder = WadBuilder::new(wad_kind);

            // Parse and load each NAME=FILE lump specification.
            for spec in &lumps {
                let Some((name, file_path)) = spec.split_once('=') else {
                    eprintln!("error: invalid lump specification {spec:?}: expected NAME=FILE");
                    return Ok(3);
                };
                if name.is_empty() || file_path.is_empty() {
                    eprintln!(
                        "error: invalid lump specification {spec:?}: name and file must not be empty"
                    );
                    return Ok(3);
                }
                let data = std::fs::read(file_path)
                    .with_context(|| format!("failed to read lump file {file_path}"))?;
                builder.add_lump(name, data);
            }

            let write_opts = if cli.lenient {
                crustywad::WriteOptions::lenient()
            } else {
                crustywad::WriteOptions::strict()
            };

            // Lump-name/size validation failures are usage errors (bad input data),
            // distinct from the I/O failures handled via `?` elsewhere in this arm.
            let (bytes, warnings) = match builder.build_with_options(&write_opts) {
                Ok(result) => result,
                Err(e) => {
                    eprintln!("error: failed to build WAD {}: {e}", output.display());
                    return Ok(3);
                }
            };

            for w in &warnings {
                eprintln!("warning: {w}");
            }

            let final_bytes = if nodes {
                use crustywad::ParseOptions;
                use crustywad::map::build::{NodeBuildOptions, add_doom_map_with_nodes};
                use crustywad::map::{Map, MapFormat, MapGroup, detect_map_format};
                use std::collections::{HashMap, HashSet};

                let parse_opts = if cli.lenient {
                    ParseOptions::lenient()
                } else {
                    ParseOptions::strict()
                };
                let build_opts = if cli.lenient {
                    NodeBuildOptions::lenient()
                } else {
                    NodeBuildOptions::strict()
                };
                // Re-read our own freshly-built WAD to detect map groups.
                let wad = match Wad::from_bytes_with_options(bytes.clone(), parse_opts) {
                    Ok(w) => w,
                    Err(e) => {
                        eprintln!("error: failed to re-read built WAD for node building: {e}");
                        return Ok(3);
                    }
                };
                // Classify each map group: Doom groups are rebuilt; others are
                // passed through with a note. `MapFormat` is `#[non_exhaustive]`,
                // so an unknown future format falls through to plain pass-through.
                let mut doom_starts: HashMap<usize, MapGroup> = HashMap::new();
                let mut absorbed: HashSet<usize> = HashSet::new();
                for group in wad.map_groups() {
                    match detect_map_format(&wad, &group) {
                        MapFormat::Doom => {
                            absorbed.insert(group.marker_index);
                            absorbed.extend(group.data_indices.iter().copied());
                            doom_starts.insert(group.marker_index, group);
                        }
                        MapFormat::Hexen => eprintln!(
                            "note: {} is a Hexen map; node building for Hexen is not yet supported (skipped; see #352)",
                            group.name
                        ),
                        MapFormat::Doom64 => eprintln!(
                            "note: {} is a Doom 64 map; node building is not supported (skipped; see #353)",
                            group.name
                        ),
                        MapFormat::Udmf => eprintln!(
                            "note: {} is a UDMF map; node building needs GL nodes (skipped; see #354)",
                            group.name
                        ),
                        _ => {}
                    }
                }
                if doom_starts.is_empty() {
                    eprintln!("note: no Doom map groups found; --nodes had no effect");
                    bytes
                } else {
                    let mut out = WadBuilder::new(wad_kind);
                    for (i, lump) in wad.lumps().iter().enumerate() {
                        if let Some(group) = doom_starts.get(&i) {
                            let map = match Map::assemble_with_options(&wad, group, parse_opts) {
                                Ok(m) => m,
                                Err(e) => {
                                    eprintln!("error: failed to assemble map {}: {e}", group.name);
                                    return Ok(3);
                                }
                            };
                            for w in map.warnings() {
                                eprintln!("warning: {}: {w}", group.name);
                            }
                            match add_doom_map_with_nodes(
                                &mut out,
                                &group.name,
                                &map,
                                &write_opts,
                                &build_opts,
                            ) {
                                Ok(ws) => {
                                    for w in &ws {
                                        eprintln!("warning: {}: {w}", group.name);
                                    }
                                }
                                Err(e) => {
                                    eprintln!(
                                        "error: failed to build nodes for map {}: {e}",
                                        group.name
                                    );
                                    if e.is_lenient_recoverable() && !cli.lenient {
                                        eprintln!("note: re-run with --lenient to build anyway");
                                    }
                                    return Ok(3);
                                }
                            }
                        } else if !absorbed.contains(&i) {
                            out.add_lump(lump.name(), wad.lump_data(lump));
                        }
                    }
                    match out.build_with_options(&write_opts) {
                        Ok((b, ws)) => {
                            for w in &ws {
                                eprintln!("warning: {w}");
                            }
                            b
                        }
                        Err(e) => {
                            eprintln!("error: failed to build WAD {}: {e}", output.display());
                            return Ok(3);
                        }
                    }
                }
            } else {
                bytes
            };

            std::fs::write(&output, &final_bytes)
                .with_context(|| format!("failed to write {}", output.display()))?;

            // Without `--nodes` the output is exactly the packed specs, so
            // `lumps.len()` is the true count. With `--nodes` the rebuilt
            // groups replace placeholder lumps with synthesized node lumps, so
            // report the actual output count by re-reading `final_bytes` (a
            // parse failure falls back to the input spec count).
            let lump_count = if nodes {
                Wad::from_bytes(final_bytes.clone())
                    .map_or_else(|_| lumps.len(), |w| w.lumps().len())
            } else {
                lumps.len()
            };
            match cli.format {
                Format::Human | Format::Csv => println!(
                    "wrote {}: kind={:?} lumps: {lump_count}",
                    output.display(),
                    wad_kind
                ),
                Format::Json => println!(r#"{{"ok":true,"lumps":{lump_count}}}"#),
            }
            Ok(0)
        }

        SubCommand::Convert {
            input,
            output,
            to,
            map,
            kind,
            nodes,
            node_format,
        } => {
            use crustywad::map::build::{NodeBuildOptions, NodeFormat, add_doom_map_with_nodes};
            use crustywad::map::detect_map_format;
            use crustywad::map::{Map, MapFormat, MapGroup, add_doom_map, add_udmf_map};

            let wad = Wad::from_path_with_options(&input, options)
                .with_context(|| format!("failed to load {}", input.display()))?;
            for w in wad.warnings() {
                eprintln!("warning: {}: {w}", input.display());
            }

            let wad_kind = match kind {
                WadKindArg::Iwad => WadKind::Iwad,
                WadKindArg::Pwad => WadKind::Pwad,
            };
            let write_opts = if cli.lenient {
                crustywad::WriteOptions::lenient()
            } else {
                crustywad::WriteOptions::strict()
            };
            // Node building mirrors the same strict/lenient choice as the write
            // path, so a `--lenient` conversion also recovers node-build
            // overflows into warnings.
            let mut build_opts = if cli.lenient {
                NodeBuildOptions::lenient()
            } else {
                NodeBuildOptions::strict()
            };
            // `--nodes` builds binary node lumps, which only the Doom format
            // has; UDMF stores geometry as text and lets the engine (or an
            // external tool) build nodes, so the flag is a no-op there. Note it
            // once rather than silently ignoring it.
            if nodes && matches!(to, MapFormatArg::Udmf) {
                eprintln!(
                    "note: --nodes has no effect with --to udmf (UDMF has no binary node lumps); ignoring"
                );
            }
            // `--node-format` only takes effect when `--nodes` builds Doom node
            // lumps (`--to doom`); note-and-ignore rather than silently dropping
            // it (house style; no auto-implying `--nodes`).
            if !matches!(node_format, NodeFormatArg::Classic) && !nodes {
                eprintln!("note: --node-format has no effect without --nodes; ignoring");
            }
            if nodes && matches!(to, MapFormatArg::Doom) {
                build_opts.format = match node_format {
                    NodeFormatArg::Classic => NodeFormat::Classic,
                    NodeFormatArg::Xnod => NodeFormat::Xnod,
                    NodeFormatArg::Znod => {
                        #[cfg(feature = "extended-nodes-zlib")]
                        {
                            NodeFormat::Znod
                        }
                        #[cfg(not(feature = "extended-nodes-zlib"))]
                        {
                            eprintln!(
                                "error: --node-format znod requires cwad built with the extended-nodes-zlib feature"
                            );
                            return Ok(3);
                        }
                    }
                };
            }
            // `target` is only compared against `detect_map_format` (to skip a
            // map already in the target format); the writer is chosen from `to`
            // instead. `target_name` is the user-facing spelling.
            let (target, target_name) = match to {
                MapFormatArg::Doom => (MapFormat::Doom, "doom"),
                MapFormatArg::Udmf => (MapFormat::Udmf, "udmf"),
            };

            let groups = wad.map_groups();

            // A `--map NAME` that matches nothing is a usage error: without this
            // the command would happily write a verbatim copy of the input and
            // report "converted 0 maps", making a typo look like success.
            if let Some(name) = map.as_deref()
                && !groups.iter().any(|g| g.name == name)
            {
                eprintln!("error: map {name:?} not found in {}", input.display());
                if groups.is_empty() {
                    eprintln!("note: {} contains no maps", input.display());
                } else {
                    let available: Vec<&str> = groups.iter().map(|g| g.name.as_str()).collect();
                    eprintln!("note: available maps: {}", available.join(", "));
                }
                return Ok(3);
            }

            // Directory index -> the group that starts there, for the groups we
            // are converting. Every lump index inside a converted group (the
            // marker and all of its data lumps) is recorded in `absorbed` and
            // skipped on the pass-through walk, so the original binary lumps
            // are not emitted alongside the converted ones.
            let mut starts: HashMap<usize, MapGroup> = HashMap::new();
            let mut absorbed: std::collections::HashSet<usize> = std::collections::HashSet::new();
            // Per converted group, the lumps a conversion drops (see `dropped_group_lumps`).
            let mut dropped: Vec<(String, Vec<String>)> = Vec::new();
            for group in groups {
                if map.as_deref().is_some_and(|n| n != group.name) {
                    continue;
                }
                // A map already in the target format normally passes through
                // untouched — except `--nodes` targeting Doom, which must
                // (re)build the node lumps even for a Doom-format input (e.g. an
                // editor's empty-node output, the canonical "make it playable"
                // case). Routing it through `starts` sends it to
                // `add_doom_map_with_nodes` below.
                if detect_map_format(&wad, &group) == target
                    && !(nodes && matches!(to, MapFormatArg::Doom))
                {
                    continue; // already in the target format: pass through
                }
                let extra = dropped_group_lumps(&wad, &group);
                if !extra.is_empty() {
                    dropped.push((group.name.clone(), extra));
                }
                absorbed.insert(group.marker_index);
                absorbed.extend(group.data_indices.iter().copied());
                starts.insert(group.marker_index, group);
            }

            // Dropping a lump the target format has no place for is data loss,
            // and is handled exactly like an unrepresentable field (ADR-0019):
            // strict refuses, lenient converts and warns.
            if !dropped.is_empty() {
                if cli.lenient {
                    for (name, lumps) in &dropped {
                        eprintln!(
                            "warning: {name}: dropped lump(s) not carried into the {target_name} map: {}",
                            lumps.join(", ")
                        );
                    }
                } else {
                    for (name, lumps) in &dropped {
                        eprintln!(
                            "error: cannot convert map {name} to {target_name}: it contains lump(s) that cannot be carried into the converted map: {}",
                            lumps.join(", ")
                        );
                    }
                    eprintln!("note: re-run with --lenient to convert anyway and drop them");
                    return Ok(3);
                }
            }

            let mut builder = WadBuilder::new(wad_kind);
            let mut converted = 0_usize;

            for (i, lump) in wad.lumps().iter().enumerate() {
                if let Some(group) = starts.get(&i) {
                    let assembled = match Map::assemble_with_options(&wad, group, options) {
                        Ok(m) => m,
                        Err(e) => {
                            eprintln!("error: failed to assemble map {}: {e}", group.name);
                            return Ok(3);
                        }
                    };
                    // Lenient assembly repairs what it can (clamping an
                    // out-of-range cross-reference, coercing a field) and records
                    // each repair. Those are changes to the map the user is about
                    // to write out, so surface them alongside the write warnings
                    // rather than dropping them — otherwise a repaired map looks
                    // like a clean one.
                    for w in assembled.warnings() {
                        eprintln!("warning: {}: {w}", group.name);
                    }
                    // Conversion warnings (rounding, clamping, dropped fields,
                    // and the unconditional `NodesNotBuilt` when targeting Doom)
                    // are reported to stderr; a strict-mode refusal is fatal.
                    // Dispatch on the CLI argument, not on `MapFormat`: the latter
                    // is `#[non_exhaustive]`, so a wildcard arm would silently
                    // route a future format to the UDMF writer. `MapFormatArg` is
                    // exhaustive, so adding a target here is a compile error until
                    // it is given a writer.
                    //
                    // Both writers report the same shape (warnings on success, a
                    // refusal on loss), so normalize the refusal — keeping the
                    // typed error's strictness classification, which the message
                    // below dispatches on — and handle it once rather than per
                    // target format.
                    let written: Result<Vec<String>, Refusal> = match to {
                        MapFormatArg::Doom if nodes => add_doom_map_with_nodes(
                            &mut builder,
                            &group.name,
                            &assembled,
                            &write_opts,
                            &build_opts,
                        )
                        .map(|ws| ws.iter().map(ToString::to_string).collect())
                        .map_err(|e| Refusal {
                            lenient_recoverable: e.is_lenient_recoverable(),
                            message: e.to_string(),
                        }),
                        MapFormatArg::Doom => {
                            add_doom_map(&mut builder, &group.name, &assembled, &write_opts)
                                .map(|ws| ws.iter().map(ToString::to_string).collect())
                                .map_err(|e| Refusal {
                                    lenient_recoverable: e.is_lenient_recoverable(),
                                    message: e.to_string(),
                                })
                        }
                        MapFormatArg::Udmf => {
                            add_udmf_map(&mut builder, &group.name, &assembled, &write_opts)
                                .map(|ws| ws.iter().map(ToString::to_string).collect())
                                .map_err(|e| Refusal {
                                    lenient_recoverable: e.is_lenient_recoverable(),
                                    message: e.to_string(),
                                })
                        }
                    };
                    let warnings: Vec<String> = match written {
                        Ok(ws) => ws,
                        Err(refusal) => {
                            eprintln!(
                                "error: cannot convert map {} to {target_name}: {}",
                                group.name, refusal.message
                            );
                            // The hint is only honest for errors lenient mode
                            // actually recovers (#264).
                            if refusal.lenient_recoverable && !cli.lenient {
                                eprintln!("note: re-run with --lenient to accept the data loss");
                            }
                            return Ok(3);
                        }
                    };
                    for w in &warnings {
                        eprintln!("warning: {}: {w}", group.name);
                    }
                    converted += 1;
                } else if !absorbed.contains(&i) {
                    builder.add_lump(lump.name(), wad.lump_data(lump));
                }
            }

            // Lump-name/size validation failures are usage errors (bad input
            // data), distinct from the I/O failures handled via `?` below.
            let (bytes, warnings) = match builder.build_with_options(&write_opts) {
                Ok(result) => result,
                Err(e) => {
                    eprintln!("error: failed to build {}: {e}", output.display());
                    return Ok(3);
                }
            };
            for w in &warnings {
                eprintln!("warning: {w}");
            }

            std::fs::write(&output, &bytes)
                .with_context(|| format!("failed to write {}", output.display()))?;

            match cli.format {
                Format::Human | Format::Csv => {
                    let maps = if converted == 1 { "map" } else { "maps" };
                    println!(
                        "wrote {}: converted {converted} {maps} to {target_name}",
                        output.display()
                    );
                }
                Format::Json => println!(
                    r#"{{"ok":true,"converted":{converted},"format":{}}}"#,
                    json_string(target_name)
                ),
            }
            Ok(0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::sanitize_lump_name;

    #[test]
    fn sanitize_lump_name_reserved_windows_names_get_prefixed() {
        for name in &[
            "CON", "con", "Con", "PRN", "AUX", "NUL", "COM1", "COM9", "LPT1", "LPT9",
        ] {
            let result = sanitize_lump_name(name);
            assert!(
                result.starts_with('_'),
                "expected '{name}' to be prefixed, got '{result}'"
            );
        }
    }

    #[test]
    fn sanitize_lump_name_normal_names_unchanged() {
        assert_eq!(sanitize_lump_name("PLAYPAL"), "PLAYPAL");
        assert_eq!(sanitize_lump_name("E1M1"), "E1M1");
        assert_eq!(sanitize_lump_name("MY-LUMP"), "MY-LUMP");
    }

    #[test]
    fn sanitize_lump_name_empty_returns_unnamed() {
        assert_eq!(sanitize_lump_name(""), "UNNAMED");
    }

    #[test]
    fn sanitize_lump_name_path_traversal_replaced() {
        assert_eq!(sanitize_lump_name("A/B"), "A_B");
        assert_eq!(sanitize_lump_name("../etc"), "___ETC");
    }
}
