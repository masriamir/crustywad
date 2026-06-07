//! Command-line tooling for inspecting Doom WAD files with `crustywad`.

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use crustywad::{ParseOptions, Wad};

#[derive(Debug, Parser)]
#[command(author, version, about = "Inspect Doom WAD files")]
struct Cli {
    #[command(subcommand)]
    command: Command,
    #[arg(
        long,
        global = true,
        help = "Use lenient parsing instead of strict parsing"
    )]
    lenient: bool,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Print the WAD kind and lump count.
    Info {
        /// Path to the WAD file.
        path: PathBuf,
    },
    /// Print the lump directory.
    List {
        /// Path to the WAD file.
        path: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let options = if cli.lenient {
        ParseOptions::lenient()
    } else {
        ParseOptions::strict()
    };

    match cli.command {
        Command::Info { path } => {
            let wad = Wad::from_path_with_options(path, options)?;
            println!("kind: {:?}", wad.kind());
            println!("lumps: {}", wad.lump_count());
            for warning in wad.warnings() {
                eprintln!("warning: {warning}");
            }
        }
        Command::List { path } => {
            let wad = Wad::from_path_with_options(path, options)?;
            for (index, lump) in wad.lumps().iter().enumerate() {
                println!(
                    "{index:04} {:>8} {:>8} {}",
                    lump.filepos(),
                    lump.size(),
                    lump.name()
                );
            }
            for warning in wad.warnings() {
                eprintln!("warning: {warning}");
            }
        }
    }

    Ok(())
}
