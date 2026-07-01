//! CLI argument types for `cwad`.
//!
//! This module is also included verbatim by `build.rs` via `#[path]` so that
//! the build script can call `Cli::command()` to generate shell completions.
//! For that reason this module must not import anything from the `crustywad`
//! library crate or any other runtime-only dependency.

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

/// The WAD kind to use when writing the output WAD.
///
/// Passed to `--kind` on subcommands that write a WAD file (e.g. `merge`).
#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum WadKindArg {
    /// Write an IWAD (main game data file).
    Iwad,
    /// Write a PWAD (patch/add-on WAD).
    Pwad,
}

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
    /// Merge multiple WAD files into one.
    Merge {
        /// Input WAD files to merge (in order).
        inputs: Vec<PathBuf>,
        /// Output WAD file path.
        #[arg(short, long)]
        output: PathBuf,
        /// Output WAD kind.
        #[arg(long, default_value = "pwad")]
        kind: WadKindArg,
    },
}
