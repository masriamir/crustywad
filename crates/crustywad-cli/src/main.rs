//! Command-line tool for inspecting Doom WAD files.

mod cli;
mod mus2mid;

use std::borrow::Cow;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process;

use std::fmt::Write as _;

use anyhow::{Context as _, Result};
use clap::Parser as _;
#[cfg(feature = "archive")]
use crustywad::archive::Archive;
use crustywad::audio::{AudioKind, DmxSound, MidiInfo, MusScore, WavSound};
use crustywad::{ParseError, ParseOptions, Wad, WadBuilder, WadGame, WadKind};

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
fn deep_validate(wad: &Wad, path: &Path, format: Format, options: ParseOptions) -> i32 {
    if matches!(format, Format::Csv) {
        println!("map,ok,error");
    }
    let (validated, failed) = validate_groups(wad, "", format, options);
    let code = deep_summary(path, format, validated, failed);
    // ADR-0008 §3: container-level lenient warnings print after the result.
    for w in wad.warnings() {
        eprintln!("warning: {w}");
    }
    code
}

/// The per-map body of [`deep_validate`], reusable for the WADs inside an
/// archive.
///
/// Assembles every map group of `wad` under `options`, printing per-map
/// results with `label` prefixed (empty for a standalone WAD), and returns
/// `(validated, failed)` counts. The caller prints the CSV header, so one
/// table can span several WADs.
fn validate_groups(
    wad: &Wad,
    label: &str,
    format: Format,
    options: ParseOptions,
) -> (usize, usize) {
    use crustywad::map::Map;

    let groups = wad.map_groups();
    let mut failed = 0usize;
    let prefix = if label.is_empty() {
        String::new()
    } else {
        format!("{label}: ")
    };

    for group in &groups {
        let name = format!("{prefix}{}", group.name);
        match Map::assemble_with_options(wad, group, options) {
            Ok(map) => {
                for w in map.warnings() {
                    eprintln!("warning: map {name}: {w}");
                }
                match format {
                    Format::Human => {}
                    Format::Json => println!(
                        r#"{{"map":{},"ok":true,"warnings":{}}}"#,
                        json_string(&name),
                        map.warnings().len()
                    ),
                    Format::Csv => {
                        println!("{},true,", csv_field(&name));
                    }
                }
            }
            Err(e) => {
                failed += 1;
                eprintln!("error: map {name}: {e:#}");
                match format {
                    Format::Human => {}
                    Format::Json => println!(
                        r#"{{"map":{},"ok":false,"error":{}}}"#,
                        json_string(&name),
                        json_string(&e.to_string())
                    ),
                    Format::Csv => {
                        println!("{},false,{}", csv_field(&name), csv_field(&e.to_string()));
                    }
                }
            }
        }
    }

    (groups.len(), failed)
}

/// Prints the deep-validation summary and returns the exit code: `0` when
/// nothing failed, `1` otherwise.
fn deep_summary(path: &Path, format: Format, validated: usize, failed: usize) -> i32 {
    if failed == 0 {
        match format {
            Format::Human => {
                println!("ok: {} ({validated} map(s) validated)", path.display());
            }
            Format::Json => println!(r#"{{"ok":true}}"#),
            Format::Csv => {}
        }
        0
    } else {
        let summary = format!("{failed} of {validated} map(s) failed validation");
        match format {
            Format::Human => eprintln!("error: {}: {summary}", path.display()),
            Format::Json => {
                println!(r#"{{"ok":false,"error":{}}}"#, json_string(&summary));
            }
            Format::Csv => {}
        }
        1
    }
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

/// Lowercase display name for a detected [`WadGame`].
///
/// `WadGame` is `#[non_exhaustive]` (ADR-0028 §1), so this crate cannot get a
/// compile-time guarantee that every variant is handled — the wildcard arm
/// below is required by the compiler even though only one variant exists
/// today. A future variant added upstream falls back to `"unknown"` here
/// rather than panicking, until this match is updated to name it explicitly.
fn game_name(game: WadGame) -> &'static str {
    match game {
        WadGame::Strife => "strife",
        _ => "unknown",
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

/// Maps a CLI `--node-format` value to the library's
/// [`NodeFormat`](crustywad::map::build::NodeFormat).
///
/// With the `extended-nodes-zlib` feature on (the default build) every value
/// maps to a real variant, so the signature is infallible — the type records
/// that this binary has no `--node-format` failure path. The
/// `--no-default-features` twin below carries the fallible signature instead.
#[cfg(feature = "extended-nodes-zlib")]
fn node_format_arg_to_lib(arg: NodeFormatArg) -> crustywad::map::build::NodeFormat {
    use crustywad::map::build::NodeFormat;
    match arg {
        NodeFormatArg::Classic => NodeFormat::Classic,
        NodeFormatArg::Xnod => NodeFormat::Xnod,
        NodeFormatArg::Znod => NodeFormat::Znod,
        NodeFormatArg::Xgln => NodeFormat::Xgln,
        NodeFormatArg::Xgl2 => NodeFormat::Xgl2,
        NodeFormatArg::Xgl3 => NodeFormat::Xgl3,
        NodeFormatArg::Gl => NodeFormat::Gl,
        NodeFormatArg::Zgln => NodeFormat::Zgln,
        NodeFormatArg::Zgl2 => NodeFormat::Zgl2,
        NodeFormatArg::Zgl3 => NodeFormat::Zgl3,
        NodeFormatArg::Zgl => NodeFormat::Zgl,
    }
}

/// Maps a CLI `--node-format` value to the library's
/// [`NodeFormat`](crustywad::map::build::NodeFormat).
///
/// # Errors
///
/// Returns `Err` with a human-readable message — printed by the caller as
/// `error: {msg}` before exiting 3 — when `arg` selects a zlib dialect
/// (`znod`/`zgln`/`zgl2`/`zgl3`/`zgl`): this build lacks the
/// `extended-nodes-zlib` feature, so those dialects cannot be emitted.
#[cfg(not(feature = "extended-nodes-zlib"))]
fn node_format_arg_to_lib(arg: NodeFormatArg) -> Result<crustywad::map::build::NodeFormat, String> {
    use crustywad::map::build::NodeFormat;

    macro_rules! zlib_arm {
        ($name:literal) => {
            return Err(format!(
                "--node-format {} requires cwad built with the extended-nodes-zlib feature",
                $name
            ))
        };
    }

    Ok(match arg {
        NodeFormatArg::Classic => NodeFormat::Classic,
        NodeFormatArg::Xnod => NodeFormat::Xnod,
        NodeFormatArg::Znod => zlib_arm!("znod"),
        NodeFormatArg::Xgln => NodeFormat::Xgln,
        NodeFormatArg::Xgl2 => NodeFormat::Xgl2,
        NodeFormatArg::Xgl3 => NodeFormat::Xgl3,
        NodeFormatArg::Gl => NodeFormat::Gl,
        NodeFormatArg::Zgln => zlib_arm!("zgln"),
        NodeFormatArg::Zgl2 => zlib_arm!("zgl2"),
        NodeFormatArg::Zgl3 => zlib_arm!("zgl3"),
        NodeFormatArg::Zgl => zlib_arm!("zgl"),
    })
}

/// Resolves the effective node format for a UDMF map group's `ZNODES` stream.
///
/// UDMF stores geometry as text, so every built node lump is carried by the
/// `ZNODES` lump: the default `Classic` (i.e. `--node-format` unset)
/// auto-selects the GL `Gl` dialect; every other format — the GL dialects
/// (`Xgln`/`Xgl2`/`Xgl3`/`Gl` and their zlib twins) and the non-GL extended
/// formats (`Xnod`/`Znod`) — passes through unchanged, and the library's
/// `add_udmf_map_with_nodes` / `build_gl_nodes` split routes it to the right
/// stream writer.
///
/// A future `NodeFormat` variant the `ZNODES` writers do not support fails
/// downstream with `NodeBuildError::UnsupportedNodeFormat`, surfaced through the
/// standard build error arm — this resolver only rewrites the `Classic`
/// default.
fn effective_udmf_format(
    opts: &crustywad::map::build::NodeBuildOptions,
) -> crustywad::map::build::NodeFormat {
    use crustywad::map::build::NodeFormat;
    match opts.format {
        NodeFormat::Classic => NodeFormat::Gl,
        _ => opts.format,
    }
}

/// Patches a UDMF map group in place: assembles it (ignoring any existing
/// `ZNODES`, so a corrupt one is repaired rather than fatal), builds the node
/// stream for `effective_format` (a GL dialect's XGL*/ZGL* stream, or the
/// non-GL `Xnod`/`Znod` XNOD/ZNOD stream), and emits the group's lumps verbatim
/// in original order with `ZNODES` replaced — or inserted right after `TEXTMAP`
/// if the group has none. Returns `Err(exit_code)` after printing the same
/// error/lenient-hint messages the Doom `--nodes` path uses.
fn patch_udmf_group_znodes(
    out: &mut WadBuilder,
    wad: &Wad,
    group: &crustywad::map::MapGroup,
    parse_opts: ParseOptions,
    effective_opts: &crustywad::map::build::NodeBuildOptions,
    effective_format: crustywad::map::build::NodeFormat,
    lenient: bool,
) -> Result<(), i32> {
    use crustywad::map::Map;
    use crustywad::map::build::{NodeFormat, build_gl_nodes, build_nodes};

    // UDMF groups are patched in place: assemble only to
    // build the node stream, then re-emit the group's own
    // lumps verbatim (TEXTMAP byte-identical) with the built
    // ZNODES stream slotted in.
    //
    // A rebuild discards any existing node lump, so a
    // stale (possibly corrupt) ZNODES is excluded from
    // assembly: only the TEXTMAP geometry matters, and
    // requiring the old nodes to parse would make
    // `--nodes` unable to *fix* a broken ZNODES (strict
    // assembly rejects an unrecognized ZNODES signature).
    //
    // The in-place patch below re-emits the TEXTMAP
    // verbatim, so it depends on assembly staying
    // element-count-faithful to that text: lenient
    // repairs today only clamp/coerce fields, never
    // drop elements, so the built ZNODES indexes the
    // same vertices/lines/sides the verbatim TEXTMAP
    // declares. If a future lenient repair ever *drops*
    // an element, the emitted ZNODES would desync from
    // the untouched TEXTMAP and this path would need to
    // re-serialize the map instead.
    let assemble_group = group.without_lumps(wad, &["ZNODES"]);
    let map = match Map::assemble_with_options(wad, &assemble_group, parse_opts) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: failed to assemble map {}: {e}", group.name);
            return Err(3);
        }
    };
    for w in map.warnings() {
        eprintln!("warning: {}: {w}", group.name);
    }
    // Branch on the format family, mirroring the library's
    // `add_udmf_map_with_nodes`: the non-GL extended formats
    // (`Xnod`/`Znod`) run the classic BSP pass and carry an
    // XNOD/ZNOD stream, while every GL dialect runs the GL
    // kernel and carries an XGL*/ZGL* stream. `NodeFormat`'s
    // `is_extended`/`compressed`/`is_gl` are crate-private,
    // so the non-GL set is matched explicitly (mirroring
    // `effective_udmf_format`'s cfg style), yielding the
    // `compressed` flag `BuiltNodes::to_extended_lump_bytes`
    // needs (`Znod` compresses; `Xnod` does not). The `_` arm
    // cannot silently misroute a future variant: the library's
    // format predicates and serializer dispatches are
    // deliberately exhaustive, so a new `NodeFormat` variant is
    // a compile error there until classified — and one
    // classified as neither GL nor non-GL-extended falls
    // through here to the GL serializer, whose dispatch rejects
    // it with `UnsupportedNodeFormat` via the shared error arm.
    let non_gl_compressed = match effective_format {
        NodeFormat::Xnod => Some(false),
        #[cfg(feature = "extended-nodes-zlib")]
        NodeFormat::Znod => Some(true),
        _ => None,
    };
    let orig_vertex_count = map.vertices().len();
    // Both build stages share one error arm below: the
    // serialization step's failures (arena overflow,
    // unnarrowable coordinate) report identically to the
    // BSP/GL pass's, and both families flow through it.
    let znodes_result = if let Some(compressed) = non_gl_compressed {
        build_nodes(&map, effective_opts).and_then(|(nodes, ws)| {
            for w in &ws {
                eprintln!("warning: {}: {w}", group.name);
            }
            nodes.to_extended_lump_bytes(orig_vertex_count, compressed)
        })
    } else {
        build_gl_nodes(&map, effective_opts).and_then(|(gl, ws)| {
            for w in &ws {
                eprintln!("warning: {}: {w}", group.name);
            }
            gl.to_extended_lump_bytes(orig_vertex_count, effective_format)
        })
    };
    let znodes = match znodes_result {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("error: failed to build nodes for map {}: {e}", group.name);
            if e.is_lenient_recoverable() && !lenient {
                eprintln!("note: re-run with --lenient to build anyway");
            }
            return Err(3);
        }
    };
    // Re-emit the group's lumps in original directory
    // order: replace an existing ZNODES's bytes in place;
    // if the group has none, insert one right after
    // TEXTMAP.
    //
    // Pathological groups with duplicate TEXTMAP/ZNODES
    // lumps are handled deterministically (garbage in):
    // every ZNODES is replaced with the built stream and
    // the insert fires once per TEXTMAP, so the output is
    // a fixed function of the malformed input.
    let has_znodes = std::iter::once(group.marker_index)
        .chain(group.data_indices.iter().copied())
        .any(|idx| wad.lumps()[idx].name() == "ZNODES");
    for idx in std::iter::once(group.marker_index).chain(group.data_indices.iter().copied()) {
        let l = &wad.lumps()[idx];
        if l.name() == "ZNODES" {
            out.add_lump("ZNODES", znodes.as_slice());
        } else {
            out.add_lump(l.name(), wad.lump_data(l));
            if l.name() == "TEXTMAP" && !has_znodes {
                out.add_lump("ZNODES", znodes.as_slice());
            }
        }
    }
    Ok(())
}

/// Patches a Hexen map group in place: assembles it (ignoring the five node
/// lumps, so corrupt ones are repaired rather than fatal), rebuilds
/// `SEGS`/`SSECTORS`/`NODES`, `REJECT`, and `BLOCKMAP` for `build_opts.format`,
/// and re-emits the group's lumps in canonical Doom/Hexen order with the node
/// lumps replaced (or inserted where missing). Returns `Err(exit_code)` after
/// printing the same error/lenient-hint messages the Doom `--nodes` path uses.
///
/// Unlike the UDMF sibling, Hexen accepts every
/// [`NodeFormat`](crustywad::map::build::NodeFormat) — including the
/// [`Classic`](crustywad::map::build::NodeFormat::Classic) default — so
/// `build_opts` is passed straight through with no effective-format resolution.
#[allow(clippy::too_many_lines)]
fn patch_hexen_group_nodes(
    out: &mut WadBuilder,
    wad: &Wad,
    group: &crustywad::map::MapGroup,
    parse_opts: ParseOptions,
    build_opts: &crustywad::map::build::NodeBuildOptions,
    lenient: bool,
) -> Result<(), i32> {
    use crustywad::map::Map;
    use crustywad::map::build::{
        NodeFormat, build_blockmap, build_gl_nodes, build_nodes, build_reject,
    };

    /// The five node lumps this splice always rebuilds. All are excluded from
    /// assembly (below) and re-emitted from the fresh build, so a corrupt one in
    /// the input is repaired rather than a strict-fatal decode of a doomed lump.
    const REBUILT: &[&str] = &["SEGS", "SSECTORS", "NODES", "REJECT", "BLOCKMAP"];

    /// Polyobject anchor/spawn thing editor numbers.
    ///
    /// The vanilla Hexen values are verified against the id Software Hexen source
    /// release (mirror: videogamepreservation/hexen): `P_LOCAL.H` defines
    /// `enum { PO_ANCHOR_TYPE = 3000, PO_SPAWN_TYPE, PO_SPAWNCRUSH_TYPE };`
    /// (i.e. 3000/3001/3002), consumed by `PO_MAN.C`'s `PO_Init()`.
    ///
    /// The 9300-series values are the `ZDoom` "Doom-in-Hexen" editor numbers,
    /// verified against `GZDoom` `wadsrc/static/mapinfo/common.txt` (repo
    /// `ZDoom/gzdoom`), whose `DoomEdNums` block maps `9300 = "$PolyAnchor"`,
    /// `9301 = "$PolySpawn"`, `9302 = "$PolySpawnCrush"`, `9303 = "$PolySpawnHurt"`.
    /// `ZDoom` moved these off 3000–3002 because in Doom-in-Hexen maps the Doom
    /// editor numbers apply, where 3001/3002 are the Imp/Demon — so the vanilla
    /// entries below double as monster numbers and the warning is advisory.
    const POLYOBJECT_THING_TYPES: [u16; 7] = [3000, 3001, 3002, 9300, 9301, 9302, 9303];

    // The three carriers a Hexen node build can produce, mirroring
    // `add_doom_map_with_nodes`'s three arms.
    enum NodeLumps {
        /// Classic three-lump layout: `SEGS`, `SSECTORS`, `NODES`, plus the
        /// split-vertex tail appended to `VERTEXES`.
        Classic(Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>),
        /// A single `XNOD`/`ZNOD` stream carried in `NODES`; `SEGS`/`SSECTORS`
        /// left empty; no `VERTEXES` tail (split verts live in the stream header).
        NonGl(Vec<u8>),
        /// A single `XGL*`/`ZGL*` stream carried in `SSECTORS`; `SEGS`/`NODES`
        /// left empty; no `VERTEXES` tail.
        Gl(Vec<u8>),
    }

    // Assemble only the geometry: exclude the five rebuilt node lumps so a
    // corrupt/garbage one never blocks the rebuild that is about to replace it.
    let assemble_group = group.without_lumps(wad, REBUILT);
    let map = match Map::assemble_with_options(wad, &assemble_group, parse_opts) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: failed to assemble map {}: {e}", group.name);
            return Err(3);
        }
    };
    for w in map.warnings() {
        eprintln!("warning: {}: {w}", group.name);
    }

    // A rebuilt BSP can split a polyobject's subsector, which the engine's
    // polyobject renderer assumes convex, so warn once per map when any
    // anchor/spawn thing (`POLYOBJECT_THING_TYPES`) is present (#389).
    if let Some(ty) = map
        .things()
        .iter()
        .map(|t| t.type_id)
        .find(|ty| POLYOBJECT_THING_TYPES.contains(ty))
    {
        eprintln!(
            "warning: {}: thing type {ty} matches a polyobject anchor/spawn editor number; rebuilt nodes may split polyobject subsectors (see #389)",
            group.name
        );
    }

    // REJECT (infallible, all-zeros: every sector pair visible).
    let reject = build_reject(&map).to_lump_bytes();

    // One fallible chain for every buildable lump: BLOCKMAP, its serialization,
    // and the per-format node build (mirroring the one-shot's arms — `Classic`
    // is a valid Hexen default with its own arm; `Xnod`/`Znod` carry a non-GL
    // extended stream; every remaining GL format runs the GL kernel, and a
    // future non-GL/non-classic variant falls through to the GL serializer,
    // whose dispatch rejects it). Threading all failures through a single
    // `Result` collapses the blockmap and BSP/GL error paths into one arm, and
    // carries both warning vecs so their echo order (blockmap, then BSP/GL)
    // survives. `NodeFormat`'s `is_gl`/`is_extended` predicates are crate-
    // private, so the non-GL set is matched explicitly (mirroring
    // `node_format_arg_to_lib`'s cfg style).
    let orig_vertex_count = map.vertices().len();
    let build_result = build_blockmap(&map, build_opts).and_then(|(blockmap, blockmap_ws)| {
        let blockmap = blockmap.to_lump_bytes()?;
        let (node_lumps, node_ws) = match build_opts.format {
            NodeFormat::Classic => {
                let (nodes, ws) = build_nodes(&map, build_opts)?;
                let l = nodes.to_lump_bytes()?;
                (
                    NodeLumps::Classic(l.segs, l.ssectors, l.nodes, l.split_vertexes),
                    ws,
                )
            }
            NodeFormat::Xnod => {
                let (nodes, ws) = build_nodes(&map, build_opts)?;
                (
                    NodeLumps::NonGl(nodes.to_extended_lump_bytes(orig_vertex_count, false)?),
                    ws,
                )
            }
            #[cfg(feature = "extended-nodes-zlib")]
            NodeFormat::Znod => {
                let (nodes, ws) = build_nodes(&map, build_opts)?;
                (
                    NodeLumps::NonGl(nodes.to_extended_lump_bytes(orig_vertex_count, true)?),
                    ws,
                )
            }
            // The `_` arm cannot silently misroute a future variant: the
            // library's format predicates and serializer dispatches are
            // deliberately exhaustive, so a new `NodeFormat` variant is a
            // compile error there until classified — and one classified as
            // neither GL nor non-GL-extended reaches the GL serializer
            // below, whose dispatch rejects it with `UnsupportedNodeFormat`
            // through the shared error arm (same rationale as the UDMF
            // sibling's routing match).
            _ => {
                let (gl, ws) = build_gl_nodes(&map, build_opts)?;
                (
                    NodeLumps::Gl(gl.to_extended_lump_bytes(orig_vertex_count, build_opts.format)?),
                    ws,
                )
            }
        };
        Ok((blockmap, blockmap_ws, node_lumps, node_ws))
    });
    let (blockmap, blockmap_ws, node_lumps, node_ws) = match build_result {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: failed to build nodes for map {}: {e}", group.name);
            if e.is_lenient_recoverable() && !lenient {
                eprintln!("note: re-run with --lenient to build anyway");
            }
            return Err(3);
        }
    };
    // Deterministic warning order (matching the one-shot's Global Constraint 6):
    // blockmap warnings, then BSP/GL build warnings. There is no write-path
    // prefix here — the geometry lumps are re-emitted verbatim, not serialized.
    for w in blockmap_ws.iter().chain(&node_ws) {
        eprintln!("warning: {}: {w}", group.name);
    }

    // Original bytes of every preserved (non-node) lump, first occurrence wins —
    // a pathological group with duplicate names is handled deterministically.
    let mut orig: HashMap<&str, &[u8]> = HashMap::new();
    for &idx in &group.data_indices {
        let l = &wad.lumps()[idx];
        orig.entry(l.name()).or_insert_with(|| wad.lump_data(l));
    }

    // Re-emit in canonical Doom/Hexen order — THINGS, LINEDEFS, SIDEDEFS,
    // VERTEXES, SEGS, SSECTORS, NODES, SECTORS, REJECT, BLOCKMAP, BEHAVIOR:
    // vanilla engines index a map's lumps by offset from the marker, so this
    // order is load-bearing. Preserved names take their original bytes; the five
    // node lumps take the fresh build (empty `Vec` for an emptied carrier). Only
    // names present in the input (or rebuilt) are emitted — a missing geometry
    // lump would have been assembly-fatal above, and a missing BEHAVIOR would
    // mean the map is not Hexen, so both are unreachable here; the guards are
    // defensive.
    let (segs, ssectors, nodes_bytes): (&[u8], &[u8], &[u8]) = match &node_lumps {
        NodeLumps::Classic(s, ss, n, _) => (s, ss, n),
        NodeLumps::NonGl(stream) => (&[], &[], stream),
        NodeLumps::Gl(stream) => (&[], stream, &[]),
    };

    let marker = &wad.lumps()[group.marker_index];
    out.add_lump(marker.name(), wad.lump_data(marker));
    for name in ["THINGS", "LINEDEFS", "SIDEDEFS"] {
        if let Some(bytes) = orig.get(name) {
            out.add_lump(name, *bytes);
        }
    }
    if let Some(vertexes) = orig.get("VERTEXES") {
        // Classic seg vertex indices reference the split vertices, so the tail
        // must be appended to the original VERTEXES; the extended/GL carriers
        // keep their split verts in the stream header and leave VERTEXES as-is.
        if let NodeLumps::Classic(_, _, _, tail) = &node_lumps {
            let mut vb = vertexes.to_vec();
            vb.extend_from_slice(tail);
            out.add_lump("VERTEXES", vb);
        } else {
            out.add_lump("VERTEXES", *vertexes);
        }
    }
    out.add_lump("SEGS", segs);
    out.add_lump("SSECTORS", ssectors);
    out.add_lump("NODES", nodes_bytes);
    if let Some(bytes) = orig.get("SECTORS") {
        out.add_lump("SECTORS", *bytes);
    }
    out.add_lump("REJECT", reject);
    out.add_lump("BLOCKMAP", blockmap);
    if let Some(bytes) = orig.get("BEHAVIOR") {
        out.add_lump("BEHAVIOR", *bytes);
    }
    Ok(())
}

/// What a path turned out to hold, decided by magic bytes only.
enum Input {
    /// A WAD file.
    Wad(Wad),
    /// A pk3 (zip) resource archive.
    #[cfg(feature = "archive")]
    Archive(Archive),
}

/// True when the leading bytes are a zip or 7z signature (ADR-0031 §7): the
/// three zip records `PK\x03\x04` / `PK\x05\x06` / `PK\x07\x08` and 7z's
/// `7z\xbc\xaf\x27\x1c`.
fn looks_like_archive(bytes: &[u8]) -> bool {
    bytes.starts_with(b"PK\x03\x04")
        || bytes.starts_with(b"PK\x05\x06")
        || bytes.starts_with(b"PK\x07\x08")
        || bytes.starts_with(b"7z\xbc\xaf\x27\x1c")
}

/// Opens `path` as a WAD or, when its magic says so, as an archive.
///
/// Detection is by leading bytes only, never by extension, so a WAD named
/// `.pk3` still reads as a WAD and a zip named `.wad` still reads as an
/// archive.
///
/// The returned error carries **no** added context, so a caller that reports
/// the path itself — `validate` — can print the library message verbatim, the
/// way it did when it called [`Wad::from_path_with_options`] directly. Callers
/// that want the path in the message add `failed to load {path}` themselves.
/// The read failure is reported as [`ParseError::Io`], the same error
/// [`Wad::from_path_with_options`] raises, so the wording does not depend on
/// which of the two opened the file.
///
/// # Errors
///
/// Returns an error when `path` cannot be read, when it holds an archive this
/// build was compiled without support for, or when the WAD or archive fails to
/// parse under `options`.
fn open_input(path: &Path, options: ParseOptions) -> Result<Input> {
    let bytes = fs::read(path).map_err(|source| ParseError::Io {
        path: path.display().to_string(),
        source,
    })?;
    if looks_like_archive(&bytes) {
        #[cfg(feature = "archive")]
        {
            let archive = Archive::from_bytes_with_options(bytes, options)?;
            let archive = match path.file_stem().and_then(|s| s.to_str()) {
                Some(stem) => archive.with_name(stem),
                None => archive,
            };
            return Ok(Input::Archive(archive));
        }
        // No path in the message: every caller already names the file, either
        // through its own `failed to load {path}` context or, for `validate`,
        // in the `error: {path}: …` line it prints.
        #[cfg(not(feature = "archive"))]
        {
            anyhow::bail!(
                "pk3 archive input is unsupported: this build was compiled without the archive feature"
            );
        }
    }
    Ok(Input::Wad(Wad::from_bytes_with_options(bytes, options)?))
}

/// Fails with a clear message when a WAD-only command is handed an archive.
///
/// # Errors
///
/// Returns an error when `path` cannot be opened or read, or when its leading
/// bytes are an archive signature.
fn reject_archive(path: &Path, command: &str) -> Result<()> {
    let mut head = [0_u8; 6];
    let n = fs::File::open(path)
        .and_then(|mut f| std::io::Read::read(&mut f, &mut head))
        .with_context(|| format!("failed to read {}", path.display()))?;
    if looks_like_archive(&head[..n]) {
        anyhow::bail!("{} is a pk3 archive; {command} reads WADs", path.display());
    }
    Ok(())
}

/// The `info`, `list`, and `validate` arms for pk3 (zip) archive input
/// (ADR-0031).
#[cfg(feature = "archive")]
mod archive_cli {
    use super::{Format, ParseOptions, csv_field, deep_summary, json_string, validate_groups};
    use crustywad::archive::{Archive, MapKind, Member, Namespace};
    use std::path::Path;

    /// Human-readable, lowercase namespace label.
    pub(super) fn namespace_label(ns: Namespace) -> &'static str {
        ns.directory().unwrap_or(match ns {
            Namespace::Global => "global",
            _ => "hidden",
        })
    }

    /// Per-namespace member tally in table order, omitting empty namespaces.
    pub(super) fn namespace_tally(archive: &Archive) -> Vec<(&'static str, usize)> {
        let order = [
            Namespace::Global,
            Namespace::Flats,
            Namespace::Textures,
            Namespace::Hires,
            Namespace::Sprites,
            Namespace::Voxels,
            Namespace::Colormaps,
            Namespace::Acs,
            Namespace::Voices,
            Namespace::Patches,
            Namespace::Graphics,
            Namespace::Sounds,
            Namespace::Music,
            Namespace::Hidden,
        ];
        order
            .iter()
            .map(|ns| {
                (
                    namespace_label(*ns),
                    archive
                        .members()
                        .iter()
                        .filter(|m| m.namespace() == *ns)
                        .count(),
                )
            })
            .filter(|(_, n)| *n > 0)
            .collect()
    }

    /// Map names inside an embedded WAD (empty when it fails to parse — the
    /// failure is `validate --deep`'s job to report, not `info`'s).
    pub(super) fn embedded_map_names(archive: &Archive, member: &Member) -> Vec<String> {
        archive
            .wad(member)
            .map(|wad| wad.map_groups().into_iter().map(|g| g.name).collect())
            .unwrap_or_default()
    }

    /// `info` for an archive: container kind, member and namespace tallies,
    /// map names, and every embedded WAD with the maps it holds.
    pub(super) fn info(archive: &Archive, format: Format) {
        let declared: u64 = archive.members().iter().map(Member::size).sum();
        let maps: Vec<String> = archive
            .maps()
            .iter()
            .map(|m| m.name().to_string())
            .collect();
        let embedded: Vec<(String, Vec<String>)> = archive
            .embedded_wads()
            .iter()
            .map(|m| (m.path().to_string(), embedded_map_names(archive, m)))
            .collect();
        let tally = namespace_tally(archive);
        match format {
            Format::Human => {
                println!("kind:      pk3 (zip)");
                println!("members:   {}", archive.members().len());
                let unit = if declared == 1 { "byte" } else { "bytes" };
                println!("data size: {declared} {unit} (declared)");
                if !tally.is_empty() {
                    let rendered = tally
                        .iter()
                        .map(|(k, n)| format!("{k}: {n}"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    println!("namespaces: {rendered}");
                }
                if !maps.is_empty() {
                    println!("maps:      {}", maps.join(", "));
                }
                for (path, names) in &embedded {
                    if names.is_empty() {
                        println!("embedded:  {path}");
                    } else {
                        println!("embedded:  {path} (maps: {})", names.join(", "));
                    }
                }
            }
            Format::Json => {
                let tally_json = tally
                    .iter()
                    .map(|(k, n)| format!(r#""{k}":{n}"#))
                    .collect::<Vec<_>>()
                    .join(",");
                let maps_json = maps
                    .iter()
                    .map(|m| json_string(m))
                    .collect::<Vec<_>>()
                    .join(",");
                let embedded_json = embedded
                    .iter()
                    .map(|(path, names)| {
                        let names = names
                            .iter()
                            .map(|n| json_string(n))
                            .collect::<Vec<_>>()
                            .join(",");
                        format!(r#"{{"path":{},"maps":[{names}]}}"#, json_string(path))
                    })
                    .collect::<Vec<_>>()
                    .join(",");
                println!(
                    r#"{{"kind":"pk3","container":"zip","members":{},"declared_size":{declared},"namespaces":{{{tally_json}}},"maps":[{maps_json}],"embedded_wads":[{embedded_json}]}}"#,
                    archive.members().len()
                );
            }
            Format::Csv => {
                println!("kind,members,declared_size,maps,embedded_wads");
                println!(
                    "pk3,{},{declared},{},{}",
                    archive.members().len(),
                    csv_field(&maps.join(" ")),
                    csv_field(
                        &embedded
                            .iter()
                            .map(|(p, _)| p.as_str())
                            .collect::<Vec<_>>()
                            .join(" ")
                    )
                );
            }
        }
    }

    /// `list` for an archive: one row per member, in central-directory order.
    pub(super) fn list(archive: &Archive, format: Format) {
        match format {
            Format::Human => {
                for m in archive.members() {
                    // `Method`'s Display writes directly, so render it first to
                    // let the column width apply.
                    let method = m.method().to_string();
                    println!(
                        "{:04} {:>9} {method:>7} {:>10} {:<8} {}",
                        m.index(),
                        namespace_label(m.namespace()),
                        m.size(),
                        m.short_name().unwrap_or("-"),
                        m.path()
                    );
                }
            }
            Format::Json => {
                for m in archive.members() {
                    let short = m
                        .short_name()
                        .map_or_else(|| "null".to_string(), json_string);
                    println!(
                        r#"{{"index":{},"path":{},"namespace":"{}","short_name":{short},"method":"{}","size":{},"compressed_size":{},"encrypted":{},"embedded_wad":{}}}"#,
                        m.index(),
                        json_string(m.path()),
                        namespace_label(m.namespace()),
                        m.method(),
                        m.size(),
                        m.compressed_size(),
                        m.is_encrypted(),
                        m.is_embedded_wad()
                    );
                }
            }
            Format::Csv => {
                println!("index,path,namespace,short_name,method,size");
                for m in archive.members() {
                    println!(
                        "{},{},{},{},{},{}",
                        m.index(),
                        csv_field(m.path()),
                        namespace_label(m.namespace()),
                        csv_field(m.short_name().unwrap_or("")),
                        m.method(),
                        m.size()
                    );
                }
            }
        }
    }

    /// `validate --deep` for an archive: every `maps/*.wad` and every
    /// embedded WAD, member path prefixed. Returns the exit code.
    ///
    /// `deep_summary` counts `validated` as *groups validated*, so a member
    /// that cannot be read adds one failure but no validated group — an
    /// all-broken archive can truthfully report "1 of 0 map(s) failed".
    pub(super) fn deep_validate(
        archive: &Archive,
        path: &Path,
        format: Format,
        options: ParseOptions,
    ) -> i32 {
        if matches!(format, Format::Csv) {
            println!("map,ok,error");
        }
        let mut validated = 0usize;
        let mut failed = 0usize;
        let mut targets: Vec<&Member> = archive
            .maps()
            .iter()
            .filter(|m| m.kind() == MapKind::Wad)
            .map(|m| &archive.members()[m.member_index()])
            .collect();
        targets.extend(archive.embedded_wads());
        for member in targets {
            match archive.wad(member) {
                Ok(wad) => {
                    let (v, f) = validate_groups(&wad, member.path(), format, options);
                    validated += v;
                    failed += f;
                    for w in wad.warnings() {
                        eprintln!("warning: {}: {w}", member.path());
                    }
                }
                Err(e) => {
                    failed += 1;
                    eprintln!("error: {e:#}");
                    match format {
                        Format::Human => {}
                        Format::Json => println!(
                            r#"{{"map":{},"ok":false,"error":{}}}"#,
                            json_string(member.path()),
                            json_string(&e.to_string())
                        ),
                        Format::Csv => println!(
                            "{},false,{}",
                            csv_field(member.path()),
                            csv_field(&e.to_string())
                        ),
                    }
                }
            }
        }
        let code = deep_summary(path, format, validated, failed);
        for w in archive.warnings() {
            eprintln!("warning: {w}");
        }
        code
    }
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
            // Without `archive`, `Input` has a single variant and clippy reads
            // this as a needless destructuring match; it is the archive arm's
            // seat, so keep the shape in both builds.
            #[cfg_attr(
                not(feature = "archive"),
                allow(clippy::infallible_destructuring_match)
            )]
            let wad = match open_input(&path, options)
                .with_context(|| format!("failed to load {}", path.display()))?
            {
                Input::Wad(wad) => wad,
                #[cfg(feature = "archive")]
                Input::Archive(archive) => {
                    archive_cli::info(&archive, cli.format);
                    for w in archive.warnings() {
                        eprintln!("warning: {w}");
                    }
                    return Ok(0);
                }
            };
            let data_size: u64 = wad.lumps().iter().map(|l| l.size() as u64).sum();
            let maps = detect_maps(&wad);
            // Per-kind tally of detected audio lumps (detect-only, no parse).
            let audio = audio_summary(&wad);
            let game = wad.detect_game();
            match cli.format {
                Format::Human => {
                    println!("kind:      {:?}", wad.kind());
                    println!("lumps:     {}", wad.lump_count());
                    let unit = if data_size == 1 { "byte" } else { "bytes" };
                    println!("data size: {data_size} {unit}");
                    if !maps.is_empty() {
                        println!("maps:      {}", maps.join(", "));
                    }
                    if let Some(game) = game {
                        println!("game:      {}", game_name(game));
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
                    let game_json = game
                        .map(|g| format!(r#","game":"{}""#, game_name(g)))
                        .unwrap_or_default();
                    println!(
                        r#"{{"kind":"{:?}","lumps":{},"data_size":{},"maps":[{}],"audio":{{{}}}{}}}"#,
                        wad.kind(),
                        wad.lump_count(),
                        data_size,
                        maps_json,
                        audio_json,
                        game_json
                    );
                }
                Format::Csv => {
                    println!("kind,lumps,data_size,maps,game");
                    println!(
                        "{},{},{},{},{}",
                        csv_field(&format!("{:?}", wad.kind())),
                        wad.lump_count(),
                        data_size,
                        csv_field(&maps.join(" ")),
                        csv_field(game.map_or("", game_name))
                    );
                }
            }
            for w in wad.warnings() {
                eprintln!("warning: {w}");
            }
            Ok(0)
        }

        SubCommand::List { path } => {
            // Without `archive`, `Input` has a single variant and clippy reads
            // this as a needless destructuring match; it is the archive arm's
            // seat, so keep the shape in both builds.
            #[cfg_attr(
                not(feature = "archive"),
                allow(clippy::infallible_destructuring_match)
            )]
            let wad = match open_input(&path, options)
                .with_context(|| format!("failed to load {}", path.display()))?
            {
                Input::Wad(wad) => wad,
                #[cfg(feature = "archive")]
                Input::Archive(archive) => {
                    archive_cli::list(&archive, cli.format);
                    for w in archive.warnings() {
                        eprintln!("warning: {w}");
                    }
                    return Ok(0);
                }
            };
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
            reject_archive(&file1, "diff")?;
            reject_archive(&file2, "diff")?;
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
            match open_input(&path, options) {
                #[cfg(feature = "archive")]
                Ok(Input::Archive(archive)) => {
                    if deep {
                        return Ok(archive_cli::deep_validate(
                            &archive, &path, cli.format, options,
                        ));
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
                    for w in archive.warnings() {
                        eprintln!("warning: {w}");
                    }
                    Ok(0)
                }
                Ok(Input::Wad(wad)) => {
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
                    // `{e}`, not `{e:#}`: `open_input` adds no context, so the
                    // outermost error is the library's own single-line message.
                    // The alternate form would instead walk the source chain
                    // into `binrw`'s multi-line, ANSI-colored backtrace.
                    match cli.format {
                        Format::Human => eprintln!("error: {}: {e}", path.display()),
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
            for path in &inputs {
                reject_archive(path, "merge")?;
            }
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

            reject_archive(&path, "extract")?;
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
            node_format,
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

            // `--node-format` only takes effect when `--nodes` rebuilds Doom
            // node lumps; note-and-ignore rather than silently dropping it
            // (house style; no auto-implying `--nodes`).
            if !matches!(node_format, NodeFormatArg::Classic) && !nodes {
                eprintln!("note: --node-format has no effect without --nodes; ignoring");
            }

            let final_bytes = if nodes {
                use crustywad::ParseOptions;
                use crustywad::map::build::{
                    NodeBuildOptions, NodeFormat, add_doom_map_with_nodes,
                };
                use crustywad::map::{Map, MapFormat, MapGroup, detect_map_format};
                use std::collections::{HashMap, HashSet};

                let parse_opts = if cli.lenient {
                    ParseOptions::lenient()
                } else {
                    ParseOptions::strict()
                };
                let mut build_opts = if cli.lenient {
                    NodeBuildOptions::lenient()
                } else {
                    NodeBuildOptions::strict()
                };
                #[cfg(feature = "extended-nodes-zlib")]
                {
                    build_opts.format = node_format_arg_to_lib(node_format);
                }
                #[cfg(not(feature = "extended-nodes-zlib"))]
                {
                    build_opts.format = match node_format_arg_to_lib(node_format) {
                        Ok(format) => format,
                        Err(msg) => {
                            eprintln!("error: {msg}");
                            return Ok(3);
                        }
                    };
                }
                // Re-read our own freshly-built WAD to detect map groups. This is
                // our own serializer output, so a parse failure is an internal
                // invariant violation (not user error) — propagate it rather than
                // handle it as a usage error, and move `bytes` in so the no-map
                // branch can recover the buffer via `into_bytes()` (no clone).
                let wad = Wad::from_bytes_with_options(bytes, parse_opts)
                    .context("failed to re-read the freshly built WAD for node building")?;
                // Classify each map group: Doom groups are rebuilt; others are
                // passed through with a note. `MapFormat` is `#[non_exhaustive]`,
                // so an unknown future format falls through to plain pass-through.
                let mut doom_starts: HashMap<usize, MapGroup> = HashMap::new();
                let mut hexen_starts: HashMap<usize, MapGroup> = HashMap::new();
                let mut udmf_starts: HashMap<usize, MapGroup> = HashMap::new();
                let mut absorbed: HashSet<usize> = HashSet::new();
                for group in wad.map_groups() {
                    match detect_map_format(&wad, &group) {
                        MapFormat::Doom => {
                            absorbed.insert(group.marker_index);
                            absorbed.extend(group.data_indices.iter().copied());
                            doom_starts.insert(group.marker_index, group);
                        }
                        MapFormat::Hexen => {
                            absorbed.insert(group.marker_index);
                            absorbed.extend(group.data_indices.iter().copied());
                            hexen_starts.insert(group.marker_index, group);
                        }
                        MapFormat::Doom64 => eprintln!(
                            "note: {} is a Doom 64 map; node building is not supported (skipped; see #353)",
                            group.name
                        ),
                        MapFormat::Udmf => {
                            absorbed.insert(group.marker_index);
                            absorbed.extend(group.data_indices.iter().copied());
                            // The `Classic` default auto-selects the GL dialect;
                            // note the auto-selection once per group (explicit
                            // formats — GL or non-GL — need no note).
                            if matches!(build_opts.format, NodeFormat::Classic) {
                                eprintln!(
                                    "note: {} is a UDMF map; building GL nodes (gl auto-format) into ZNODES",
                                    group.name
                                );
                            }
                            udmf_starts.insert(group.marker_index, group);
                        }
                        _ => {}
                    }
                }
                if doom_starts.is_empty() && hexen_starts.is_empty() && udmf_starts.is_empty() {
                    eprintln!("note: no buildable map groups found; --nodes had no effect");
                    wad.into_bytes()
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
                        } else if let Some(group) = hexen_starts.get(&i) {
                            // Hexen accepts every NodeFormat (Classic included),
                            // so `build_opts` passes straight through.
                            if let Err(code) = patch_hexen_group_nodes(
                                &mut out,
                                &wad,
                                group,
                                parse_opts,
                                &build_opts,
                                cli.lenient,
                            ) {
                                return Ok(code);
                            }
                        } else if let Some(group) = udmf_starts.get(&i) {
                            let effective_format = effective_udmf_format(&build_opts);
                            let mut effective_opts = build_opts.clone();
                            effective_opts.format = effective_format;
                            if let Err(code) = patch_udmf_group_znodes(
                                &mut out,
                                &wad,
                                group,
                                parse_opts,
                                &effective_opts,
                                effective_format,
                                cli.lenient,
                            ) {
                                return Ok(code);
                            }
                        } else if !absorbed.contains(&i) {
                            out.add_lump(lump.name(), wad.lump_data(lump));
                        }
                    }
                    // Node synthesis adds lumps and grows offsets, so a rebuild can
                    // trip the write-path overflow guards (TooManyLumps,
                    // LumpTooLarge, OffsetOverflow) — a usage error, handled the
                    // same way (exit 3) as the initial pack build above and as
                    // `convert`, not propagated as a generic error.
                    match out.build_with_options(&write_opts) {
                        Ok((rebuilt, ws)) => {
                            for w in &ws {
                                eprintln!("warning: {w}");
                            }
                            rebuilt
                        }
                        Err(e) => {
                            eprintln!("error: failed to rebuild {}: {e}", output.display());
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
                // `final_bytes` is already written above and unused afterward, so
                // move it into the re-read rather than clone the whole buffer.
                Wad::from_bytes(final_bytes).map_or_else(|_| lumps.len(), |w| w.lumps().len())
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
            use crustywad::map::build::{
                NodeBuildOptions, NodeFormat, add_doom_map_with_nodes, add_udmf_map_with_nodes,
            };
            use crustywad::map::detect_map_format;
            use crustywad::map::{Map, MapFormat, MapGroup, add_doom_map, add_udmf_map};

            reject_archive(&input, "convert")?;
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
            // `--node-format` only takes effect when `--nodes` builds node lumps;
            // note-and-ignore rather than silently dropping it (house style; no
            // auto-implying `--nodes`).
            if !matches!(node_format, NodeFormatArg::Classic) && !nodes {
                eprintln!("note: --node-format has no effect without --nodes; ignoring");
            }
            // Both node targets translate `--node-format` into `build_opts`: the
            // Doom target writes the classic (or `X/ZNOD`) node lumps, the UDMF
            // target the GL `ZNODES` carrier.
            if nodes && matches!(to, MapFormatArg::Doom | MapFormatArg::Udmf) {
                #[cfg(feature = "extended-nodes-zlib")]
                {
                    build_opts.format = node_format_arg_to_lib(node_format);
                }
                #[cfg(not(feature = "extended-nodes-zlib"))]
                {
                    build_opts.format = match node_format_arg_to_lib(node_format) {
                        Ok(format) => format,
                        Err(msg) => {
                            eprintln!("error: {msg}");
                            return Ok(3);
                        }
                    };
                }
            }
            // UDMF stores geometry as text, so every built node lump is carried
            // by the `ZNODES` lump: the `Classic` default auto-selects the GL
            // dialect (noted once); explicit formats — GL or non-GL — pass
            // through and route to the right stream writer downstream.
            if nodes
                && matches!(to, MapFormatArg::Udmf)
                && matches!(build_opts.format, NodeFormat::Classic)
            {
                eprintln!(
                    "note: --to udmf --nodes builds GL nodes (gl auto-format) into ZNODES for each converted map"
                );
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
            // Already-UDMF groups that `--to udmf --nodes` patches in place (a GL
            // ZNODES retrofit; #385). Kept apart from `starts` because a retrofit
            // is not a conversion and must not bump the `converted` count.
            let mut retrofit: HashMap<usize, MapGroup> = HashMap::new();
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
                    // `--to udmf --nodes` on a group that is *already* UDMF
                    // patches it in place — like the Doom target (re-routed
                    // above to rebuild its node lumps), it retrofits a GL ZNODES
                    // onto the group's own verbatim TEXTMAP (#385) rather than
                    // converting the map. Route it through `retrofit` so the
                    // emission loop patches it without counting it as a
                    // conversion; absorb its lumps so the pass-through walk does
                    // not also copy them.
                    if nodes && matches!(to, MapFormatArg::Udmf) {
                        absorbed.insert(group.marker_index);
                        absorbed.extend(group.data_indices.iter().copied());
                        retrofit.insert(group.marker_index, group);
                    }
                    continue; // already in the target format: pass through or retrofit
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
                        MapFormatArg::Udmf if nodes => {
                            // Resolve the effective format (`Classic` -> GL auto;
                            // everything else passes through) into a clone.
                            let mut effective_opts = build_opts.clone();
                            effective_opts.format = effective_udmf_format(&build_opts);
                            add_udmf_map_with_nodes(
                                &mut builder,
                                &group.name,
                                &assembled,
                                &write_opts,
                                &effective_opts,
                            )
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
                } else if let Some(group) = retrofit.get(&i) {
                    // Already-UDMF group patched in place: rebuild its GL ZNODES
                    // onto the verbatim TEXTMAP. This is not a conversion, so it
                    // does not bump `converted`; the run-level "builds GL nodes
                    // into ZNODES" note is qualified per group here.
                    eprintln!(
                        "note: {} is already UDMF; rebuilt ZNODES in place (map not converted)",
                        group.name
                    );
                    // Resolve the effective format (`Classic` -> GL auto;
                    // everything else passes through).
                    let effective_format = effective_udmf_format(&build_opts);
                    let mut effective_opts = build_opts.clone();
                    effective_opts.format = effective_format;
                    if let Err(code) = patch_udmf_group_znodes(
                        &mut builder,
                        &wad,
                        group,
                        options,
                        &effective_opts,
                        effective_format,
                        cli.lenient,
                    ) {
                        return Ok(code);
                    }
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
    use super::{game_name, sanitize_lump_name};

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

    #[test]
    fn game_name_is_lowercase_variant() {
        assert_eq!(game_name(crustywad::WadGame::Strife), "strife");
    }
}
