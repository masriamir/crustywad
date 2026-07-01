//! Command-line tool for inspecting Doom WAD files.

mod cli;

use std::process;

use std::fmt::Write as _;

use anyhow::{Context as _, Result};
use clap::Parser as _;
use crustywad::{ParseOptions, Wad};

use std::collections::HashMap;

use cli::{Cli, Format, SubCommand};

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
            match cli.format {
                Format::Human => {
                    println!("kind:  {:?}", wad.kind());
                    println!("lumps: {}", wad.lump_count());
                }
                Format::Json => println!(
                    r#"{{"kind":"{:?}","lumps":{}}}"#,
                    wad.kind(),
                    wad.lump_count()
                ),
                Format::Csv => {
                    println!("kind,lumps");
                    println!(
                        "{},{}",
                        csv_field(&format!("{:?}", wad.kind())),
                        wad.lump_count()
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

            // Build ordered multimap: name -> Vec<lump data bytes>
            let lump_data_map = |wad: &Wad| -> HashMap<String, Vec<Vec<u8>>> {
                let mut map: HashMap<String, Vec<Vec<u8>>> = HashMap::new();
                for lump in wad.lumps() {
                    map.entry(lump.name().to_owned())
                        .or_default()
                        .push(wad.lump_data(lump).to_owned());
                }
                map
            };

            // Collect the set of all unique lump names across both WADs,
            // preserving insertion order from each WAD's directory.
            let mut all_names: Vec<String> = Vec::new();
            let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
            for lump in wad1.lumps().iter().chain(wad2.lumps().iter()) {
                if seen.insert(lump.name().to_owned()) {
                    all_names.push(lump.name().to_owned());
                }
            }

            let map1 = lump_data_map(&wad1);
            let map2 = lump_data_map(&wad2);

            let mut diffs: Vec<(DiffKind, String)> = Vec::new();
            for name in &all_names {
                let in1 = map1.get(name);
                let in2 = map2.get(name);
                match (in1, in2) {
                    (Some(_), None) => diffs.push((DiffKind::OnlyInFirst, name.clone())),
                    (None, Some(_)) => diffs.push((DiffKind::OnlyInSecond, name.clone())),
                    (Some(v1), Some(v2)) => {
                        if v1 != v2 {
                            diffs.push((DiffKind::Changed, name.clone()));
                        }
                    }
                    (None, None) => {}
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
