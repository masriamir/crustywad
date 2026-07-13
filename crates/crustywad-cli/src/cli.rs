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
    /// Use lenient parsing instead of strict when reading a WAD; for `build`
    /// and `merge`, also uses lenient instead of strict validation when
    /// writing one.
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

/// WAD kind for subcommands that write a WAD file (e.g. `build`, `merge`).
#[derive(Debug, Clone, Copy, ValueEnum, Default)]
pub(crate) enum WadKindArg {
    /// IWAD — the main game data file.
    Iwad,
    /// PWAD — a patch or add-on WAD (default).
    #[default]
    Pwad,
}

/// Target map format for `convert`.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum MapFormatArg {
    /// The classic Doom binary layout (THINGS/LINEDEFS/SIDEDEFS/VERTEXES/SECTORS).
    Doom,
    /// The UDMF text layout (TEXTMAP).
    Udmf,
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
    /// Merge multiple WAD files into one.
    Merge {
        /// Input WAD files to merge (in order).
        #[arg(required = true)]
        inputs: Vec<PathBuf>,
        /// Output WAD file path.
        #[arg(short, long)]
        output: PathBuf,
        /// Output WAD kind.
        #[arg(long, default_value = "pwad")]
        kind: WadKindArg,
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
    /// `PATCH.bin`); an empty name becomes `UNNAMED`; Windows-reserved device
    /// names (`CON`, `PRN`, `AUX`, `NUL`, `COM1`–`COM9`, `LPT1`–`LPT9`) are
    /// prefixed with `_` (e.g. `CON` → `_CON.bin`) so extraction succeeds on
    /// all platforms. Each lump is written as `<SAFE_NAME>.bin`. When two or
    /// more lumps produce the same safe filename (whether from duplicate lump
    /// names or distinct names that sanitize identically), subsequent files are
    /// suffixed with an occurrence count (e.g. `PATCH.bin`, `PATCH_1.bin`,
    /// `PATCH_2.bin`). Exits 0 on success, 2 on I/O or parse error, 3 on
    /// argument error.
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
    /// Convert every map in a WAD between the UDMF and classic Doom formats.
    ///
    /// Maps already in the target format, and all non-map lumps, pass through
    /// unchanged in directory order. Conversion is lossy in one direction:
    /// Doom -> UDMF -> Doom is exact, but UDMF -> Doom rounds fractional
    /// coordinates and drops fields Doom cannot represent (linedef args and id;
    /// thing special, args, height, and tid). In strict mode any such loss is an
    /// error; `--lenient` accepts the loss and reports each instance as a
    /// warning.
    ///
    /// Converting to `doom` emits empty SEGS/SSECTORS/NODES/REJECT/BLOCKMAP
    /// lumps — run an external nodebuilder (zdbsp, bsp) before playing the map.
    ///
    /// Exits 0 on success, 2 on I/O or parse error, 3 if a map cannot be
    /// converted.
    Convert {
        /// Path to the input WAD file.
        input: PathBuf,
        /// Output WAD file path.
        #[arg(short, long)]
        output: PathBuf,
        /// Target map format.
        #[arg(long = "to", value_name = "FORMAT")]
        to: MapFormatArg,
        /// Convert only the map with this marker name (e.g. `MAP01`); all other
        /// maps pass through unchanged. If not given, every map is converted.
        #[arg(short, long, value_name = "NAME")]
        map: Option<String>,
        /// Output WAD kind.
        #[arg(long, default_value = "pwad", value_name = "KIND")]
        kind: WadKindArg,
    },
}
