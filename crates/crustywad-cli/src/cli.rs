//! CLI argument types for `cwad`.
//!
//! This module is also included verbatim by `build.rs` via `#[path]` so that
//! the build script can call `Cli::command()` to generate shell completions.
//! For that reason this module must not import anything from the `crustywad`
//! library crate or any other runtime-only dependency.

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

/// Output format for structured subcommand output.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum Format {
    /// Human-readable text (default).
    Human,
    /// Newline-delimited JSON.
    Json,
    /// RFC 4180 CSV with a header row.
    Csv,
}

/// Inspect Doom WAD files.
#[derive(Debug, Parser)]
#[command(author, version, about)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: SubCommand,
    /// Use lenient parsing instead of strict.
    #[arg(long, global = true)]
    pub(crate) lenient: bool,
    /// Output format.
    #[arg(
        short = 'F',
        long,
        global = true,
        default_value = "human",
        value_name = "FORMAT"
    )]
    pub(crate) format: Format,
}

/// WAD kind for the `build` subcommand.
#[derive(Debug, Clone, Copy, ValueEnum, Default)]
pub(crate) enum WadKindArg {
    /// IWAD — the main game data file.
    Iwad,
    /// PWAD — a patch or add-on WAD (default).
    #[default]
    Pwad,
}

/// Available `cwad` subcommands.
#[derive(Debug, Subcommand)]
pub(crate) enum SubCommand {
    /// Print WAD metadata (kind and lump count).
    Info {
        /// Path to the WAD file.
        path: PathBuf,
    },
    /// Print the lump directory.
    List {
        /// Path to the WAD file.
        path: PathBuf,
    },
    /// Validate WAD correctness. Exits 0 if clean, 2 on I/O or parse error.
    Validate {
        /// Path to the WAD file.
        path: PathBuf,
    },
    /// Build a new WAD file from lump data files.
    ///
    /// Lumps are specified as `NAME=FILE` pairs. The WAD kind defaults to PWAD.
    Build {
        /// Output WAD file path.
        #[arg(short = 'o', long, value_name = "OUTPUT")]
        output: PathBuf,
        /// WAD kind: `iwad` or `pwad` (default: `pwad`).
        #[arg(long, default_value = "pwad", value_name = "KIND")]
        kind: WadKindArg,
        /// Lump specifications as `NAME=FILE` pairs.
        ///
        /// Each argument must be of the form `LUMP_NAME=path/to/data.bin`.
        /// Lumps are added to the WAD in the order they are listed.
        #[arg(value_name = "NAME=FILE")]
        lumps: Vec<String>,
    },
}
