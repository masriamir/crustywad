//! Command-line tool for inspecting Doom WAD files.

mod cli;

use std::process;

use std::fmt::Write as _;

use anyhow::{Context as _, Result};
use clap::Parser as _;
use crustywad::{ParseOptions, Wad, WadBuilder, WadKind};

use cli::{Cli, Format, SubCommand, WadKindArg};

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
                    process::exit(3);
                };
                if name.is_empty() || file_path.is_empty() {
                    eprintln!(
                        "error: invalid lump specification {spec:?}: name and file must not be empty"
                    );
                    process::exit(3);
                }
                let data = std::fs::read(file_path)
                    .with_context(|| format!("failed to read lump file {file_path:?}"))?;
                builder.add_lump(name, data);
            }

            let write_opts = if cli.lenient {
                crustywad::WriteOptions::lenient()
            } else {
                crustywad::WriteOptions::strict()
            };

            let (bytes, warnings) = builder
                .build_with_options(&write_opts)
                .with_context(|| format!("failed to build WAD {}", output.display()))?;

            for w in &warnings {
                eprintln!("warning: {w}");
            }

            std::fs::write(&output, &bytes)
                .with_context(|| format!("failed to write {}", output.display()))?;

            let lump_count = lumps.len();
            match cli.format {
                Format::Human => println!(
                    "wrote {}: kind={:?} lumps: {lump_count}",
                    output.display(),
                    wad_kind
                ),
                Format::Json => println!(r#"{{"ok":true,"lumps":{lump_count}}}"#),
                Format::Csv => {
                    println!("ok");
                    println!("true");
                }
            }
            Ok(0)
        }
    }
}
