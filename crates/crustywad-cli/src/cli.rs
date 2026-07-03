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

/// Available `cwad` subcommands.
#[derive(Debug, Subcommand)]
pub(crate) enum SubCommand {
    /// Print WAD metadata: kind, lump count, total data size, and detected map names.
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
    /// Compare two WAD files lump by lump.
    ///
    /// Exits 0 if both WADs have identical per-name lump data (same lump names,
    /// same count of each name, same data for each occurrence; directory order
    /// of distinct lump names is
    /// not significant, but for duplicate lump names the per-name sequence of
    /// data is compared in directory order). Exits 1 if any differences are
    /// found, or 2 on I/O or parse error. JSON output is one record per line
    /// (NDJSON). When no differences are found, CSV output is empty (no header
    /// row); differences produce a `kind,name` header followed by one row each.
    Diff {
        /// Path to the first WAD file.
        file1: PathBuf,
        /// Path to the second WAD file.
        file2: PathBuf,
    },
    /// Extract lumps from a WAD file to a directory.
    ///
    /// Lump names are sanitized to safe filename components: any character that
    /// is not ASCII alphanumeric, `_`, or `-` is replaced with `_`; the result
    /// is then normalized to uppercase (so `patch` and `PATCH` both produce
    /// `PATCH.bin`); an empty name becomes `UNNAMED`. Each lump is written as
    /// `<SAFE_NAME>.bin`. When two or more lumps produce the same safe filename
    /// (whether from duplicate lump names or distinct names that sanitize
    /// identically), subsequent files are suffixed with an occurrence count
    /// (e.g. `PATCH.bin`, `PATCH_1.bin`, `PATCH_2.bin`). Exits 0 on success,
    /// 2 on I/O or parse error, 3 on argument error.
    Extract {
        /// Path to the WAD file.
        path: PathBuf,
        /// Directory to write extracted lumps into (must already exist).
        #[arg(short, long, value_name = "DIR")]
        output: PathBuf,
        /// Extract all lumps with this name; if the name appears more than once
        /// in the WAD, every occurrence is extracted. If not given, all lumps
        /// are extracted. If the name is not found the command exits with
        /// code 2.
        #[arg(short, long, value_name = "NAME")]
        lump: Option<String>,
    },
}
