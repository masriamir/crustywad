//! Command-line tool for inspecting Doom WAD files.

mod cli;

use std::collections::HashMap;
use std::fs;
use std::process;

use std::fmt::Write as _;

use anyhow::{Context as _, Result};
use clap::Parser as _;
use crustywad::{ParseOptions, Wad, WadBuilder, WadKind};

use cli::{Cli, Format, MapFormatArg, SubCommand, WadKindArg};

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
            match cli.format {
                Format::Human => {
                    println!("kind:      {:?}", wad.kind());
                    println!("lumps:     {}", wad.lump_count());
                    let unit = if data_size == 1 { "byte" } else { "bytes" };
                    println!("data size: {data_size} {unit}");
                    if !maps.is_empty() {
                        println!("maps:      {}", maps.join(", "));
                    }
                }
                Format::Json => {
                    let maps_json: String = maps
                        .iter()
                        .map(|m| json_string(m))
                        .collect::<Vec<_>>()
                        .join(",");
                    println!(
                        r#"{{"kind":"{:?}","lumps":{},"data_size":{},"maps":[{}]}}"#,
                        wad.kind(),
                        wad.lump_count(),
                        data_size,
                        maps_json
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
            match cli.format {
                Format::Human => {
                    for (i, lump) in wad.lumps().iter().enumerate() {
                        println!(
                            "{i:04} {:>8} {:>8} {}",
                            lump.filepos(),
                            lump.size(),
                            lump.name()
                        );
                    }
                }
                Format::Json => {
                    for (i, lump) in wad.lumps().iter().enumerate() {
                        println!(
                            r#"{{"index":{i},"filepos":{},"size":{},"name":{}}}"#,
                            lump.filepos(),
                            lump.size(),
                            json_string(lump.name())
                        );
                    }
                }
                Format::Csv => {
                    println!("index,filepos,size,name");
                    for (i, lump) in wad.lumps().iter().enumerate() {
                        println!(
                            "{i},{},{},{}",
                            lump.filepos(),
                            lump.size(),
                            csv_field(lump.name())
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

        SubCommand::Extract { path, output, lump } => {
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
                let lump_name = sanitize_lump_name(lump_meta.name());
                let data = wad
                    .lump_bytes(index)
                    .ok_or_else(|| anyhow::anyhow!("lump index {index} out of range"))?;

                let count = name_count.entry(lump_name.clone()).or_insert(0);
                let filename = if *count == 0 {
                    format!("{lump_name}.bin")
                } else {
                    format!("{lump_name}_{count}.bin")
                };
                *count += 1;

                let dest = output.join(&filename);
                fs::write(&dest, data)
                    .with_context(|| format!("failed to write {}", dest.display()))?;
                match cli.format {
                    Format::Human => println!("{filename}"),
                    Format::Json => {
                        println!(r#"{{"filename":{}}}"#, json_string(&filename));
                    }
                    Format::Csv => println!("{}", csv_field(&filename)),
                }
            }

            Ok(0)
        }

        SubCommand::Build {
            output,
            kind,
            lumps,
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

            std::fs::write(&output, &bytes)
                .with_context(|| format!("failed to write {}", output.display()))?;

            let lump_count = lumps.len();
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
        } => {
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
            if let Some(name) = map.as_deref() {
                if !groups.iter().any(|g| g.name == name) {
                    eprintln!("error: map {name:?} not found in {}", input.display());
                    if groups.is_empty() {
                        eprintln!("note: {} contains no maps", input.display());
                    } else {
                        let available: Vec<&str> = groups.iter().map(|g| g.name.as_str()).collect();
                        eprintln!("note: available maps: {}", available.join(", "));
                    }
                    return Ok(3);
                }
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
                if detect_map_format(&wad, &group) == target {
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
