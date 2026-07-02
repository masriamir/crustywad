//! Command-line tool for inspecting Doom WAD files.

mod cli;

use std::process;

use std::fmt::Write as _;

use anyhow::{Context as _, Result};
use clap::Parser as _;
use crustywad::{ParseOptions, Wad};

use std::collections::HashMap;

use cli::{Cli, Format, SubCommand};

/// Returns the names of map marker lumps found in `wad`, in directory order.
///
/// A lump is treated as a map marker when its name matches the Doom 1 episode
/// format (`E[1-9]M[1-9]`) or the Doom 2 numbered-map format (`MAP[0-9][0-9]`).
/// The function does not check lump size — zero-size marker lumps and non-zero
/// lumps with map names are both included, matching conventional WAD tooling
/// behavior.
fn detect_maps(wad: &Wad) -> Vec<&str> {
    wad.lumps()
        .iter()
        .map(crustywad::Lump::name)
        .filter(|name| is_map_marker(name))
        .collect()
}

/// Returns `true` if `name` matches a Doom map-marker lump name.
///
/// Recognized patterns:
/// - `E[1-9]M[1-9]` — Doom 1 episode/map (e.g. `E1M1`, `E3M9`).
/// - `MAP[0-9][0-9]` — Doom 2 numbered map (e.g. `MAP01`, `MAP32`).
fn is_map_marker(name: &str) -> bool {
    let bytes = name.as_bytes();
    match bytes.len() {
        4 => {
            // E[1-9]M[1-9]
            bytes[0] == b'E'
                && bytes[1].is_ascii_digit()
                && bytes[1] != b'0'
                && bytes[2] == b'M'
                && bytes[3].is_ascii_digit()
                && bytes[3] != b'0'
        }
        5 => {
            // MAP[0-9][0-9]
            bytes[0] == b'M'
                && bytes[1] == b'A'
                && bytes[2] == b'P'
                && bytes[3].is_ascii_digit()
                && bytes[4].is_ascii_digit()
        }
        _ => false,
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
    /// The lump exists in both WADs but its data differs.
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
                    (None, _) => diffs.push((DiffKind::OnlyInSecond, name.clone())),
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

        SubCommand::Validate { path } => {
            match Wad::from_path_with_options(&path, options) {
                Ok(wad) => {
                    match cli.format {
                        Format::Human => println!("ok: {}", path.display()),
                        Format::Json => println!(r#"{{"ok":true}}"#),
                        Format::Csv => {
                            println!("ok");
                            println!("true");
                        }
                    }
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
    }
}
