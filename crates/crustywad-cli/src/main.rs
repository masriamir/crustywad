//! Command-line tool for inspecting Doom WAD files.

mod cli;

use std::process;

use anyhow::{Context as _, Result};
use clap::Parser as _;
use crustywad::{ParseError, ParseOptions, Wad};

use cli::{Cli, Format, SubCommand};

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

fn run(cli: Cli) -> Result<i32> {
    let options = if cli.lenient {
        ParseOptions::lenient()
    } else {
        ParseOptions::strict()
    };

    match cli.command {
        SubCommand::Info { path } => {
            let wad = Wad::from_path_with_options(&path, options)
                .with_context(|| format!("failed to open {}", path.display()))?;
            for w in wad.warnings() {
                eprintln!("warning: {w}");
            }
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
                    println!("{:?},{}", wad.kind(), wad.lump_count());
                }
            }
            Ok(0)
        }

        SubCommand::List { path } => {
            let wad = Wad::from_path_with_options(&path, options)
                .with_context(|| format!("failed to open {}", path.display()))?;
            for w in wad.warnings() {
                eprintln!("warning: {w}");
            }
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
                            r#"{{"index":{i},"filepos":{},"size":{},"name":{:?}}}"#,
                            lump.filepos(),
                            lump.size(),
                            lump.name()
                        );
                    }
                }
                Format::Csv => {
                    println!("index,filepos,size,name");
                    for (i, lump) in wad.lumps().iter().enumerate() {
                        println!("{i},{},{},{}", lump.filepos(), lump.size(), lump.name());
                    }
                }
            }
            Ok(0)
        }

        SubCommand::Validate { path } => {
            match Wad::from_path_with_options(&path, options) {
                Ok(wad) => {
                    for w in wad.warnings() {
                        eprintln!("warning: {w}");
                    }
                    match cli.format {
                        Format::Human => println!("ok: {}", path.display()),
                        Format::Json => println!(r#"{{"ok":true}}"#),
                        Format::Csv => {
                            println!("ok");
                            println!("true");
                        }
                    }
                    Ok(0)
                }
                Err(e @ ParseError::Io { .. }) => {
                    // I/O failure (missing file, permission denied) — propagate
                    // as an unrecoverable error so the caller receives exit 2.
                    Err(e).with_context(|| format!("failed to open {}", path.display()))
                }
                Err(e) => {
                    match cli.format {
                        Format::Human => eprintln!("error: {e}"),
                        Format::Json => eprintln!(r#"{{"ok":false,"error":{:?}}}"#, e.to_string()),
                        Format::Csv => {
                            eprintln!("ok");
                            eprintln!("false");
                        }
                    }
                    Ok(1)
                }
            }
        }
    }
}
