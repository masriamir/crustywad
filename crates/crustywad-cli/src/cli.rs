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

/// On-disk node format for `convert --nodes` and `build --nodes`. The
/// classic and non-GL extended
/// formats (`classic`/`xnod`/`znod`) carry their stream in `NODES`; the GL
/// formats (`xgln`/`xgl2`/`xgl3`/`gl` and their `z*` zlib twins) instead carry
/// theirs in `SSECTORS`.
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub(crate) enum NodeFormatArg {
    /// Classic SEGS/SSECTORS/NODES (16-bit indices; vanilla-compatible).
    #[default]
    Classic,
    /// `ZDoom` uncompressed extended nodes (an `XNOD` stream in `NODES`).
    Xnod,
    /// `ZDoom` zlib-compressed extended nodes (`ZNOD`); requires cwad built with
    /// the `extended-nodes-zlib` feature.
    Znod,
    /// The uncompressed `ZDoom` GL extended stream with a 16-bit seg linedef
    /// reference and whole-unit `i16` node partitions (`XGLN`); the minimal GL
    /// dialect.
    Xgln,
    /// Like `xgln` but with a 32-bit seg linedef reference (`XGL2`); still
    /// requires whole-unit `i16` node partitions.
    Xgl2,
    /// The uncompressed `ZDoom` GL extended stream with fractional node
    /// partitions (`XGL3`).
    Xgl3,
    /// Auto-selects the minimal sufficient GL dialect (`xgln`, escalating to
    /// `xgl2` or `xgl3` only if the geometry requires it), then emits it
    /// uncompressed.
    Gl,
    /// The zlib-compressed twin of `xgln` (`ZGLN`); requires cwad built with
    /// the `extended-nodes-zlib` feature.
    Zgln,
    /// The zlib-compressed twin of `xgl2` (`ZGL2`); requires cwad built with
    /// the `extended-nodes-zlib` feature.
    Zgl2,
    /// The zlib-compressed twin of `xgl3` (`ZGL3`); requires cwad built with
    /// the `extended-nodes-zlib` feature.
    Zgl3,
    /// Like `gl` but emits the selected dialect's zlib-compressed twin;
    /// requires cwad built with the `extended-nodes-zlib` feature.
    Zgl,
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
    /// Validate WAD correctness. Exits 0 if clean, 1 when `--deep` finds map
    /// validation errors, 2 on I/O or parse error.
    Validate {
        /// Path to the WAD file.
        path: PathBuf,
        /// Also assemble every map in the WAD — all formats, including Doom 64
        /// nested-WAD maps — reporting per-map errors and warnings.
        #[arg(long)]
        deep: bool,
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
    /// all platforms. Each lump is written as `<SAFE_NAME>` plus an
    /// extension — `.bin` by default, or a content-derived audio extension
    /// (see the audio-aware paragraph below). When two or
    /// more lumps produce the same safe filename (whether from duplicate lump
    /// names or distinct names that sanitize identically), subsequent files are
    /// suffixed with an occurrence count (e.g. `PATCH.bin`, `PATCH_1.bin`,
    /// `PATCH_2.bin`). Exits 0 on success, 2 on I/O or parse error, 3 on
    /// argument error.
    ///
    /// Extraction is audio-aware (content-detected, never by lump name): a DMX
    /// digital-sound lump is wrapped in a canonical 44-byte WAV header and
    /// written as `.wav`; a MUS lump is written as raw `.mus` bytes (and, with
    /// `--midi`, additionally converted to a `.mid`); standard-MIDI and
    /// RIFF/WAVE lumps pass through as `.mid` and `.wav`. Everything else,
    /// including PC-speaker effects (which have no container), is written raw
    /// as `.bin`. A MUS or standard-MIDI lump that fails even a lenient
    /// parse (both detections are magic-only, so a truncated lump can match)
    /// falls back
    /// to a raw `.bin` write with a warning on stderr.
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
        /// Also write a converted `.mid` file alongside each extracted MUS
        /// lump. The conversion follows Chocolate Doom's `mus2mid`
        /// converter (a format-0 standard MIDI file). Without this flag a MUS
        /// lump is extracted only as raw `.mus` bytes.
        #[arg(long)]
        midi: bool,
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
        /// After packing, build engine-playable node lumps (plus REJECT and
        /// BLOCKMAP) for each Doom map group in the result — the lump layout
        /// follows `--node-format`. Hexen/Doom 64/UDMF groups are skipped with
        /// a note.
        #[arg(long)]
        nodes: bool,
        /// On-disk node format for `--nodes` (applies to the Doom-format map
        /// groups `--nodes` rebuilds). GL streams (`xgln`/`xgl2`/`xgl3`/`gl`
        /// and their `z*` twins) are carried in `SSECTORS` instead of `NODES`.
        /// Every `z*` value requires cwad built with the `extended-nodes-zlib`
        /// feature.
        #[arg(long = "node-format", default_value = "classic", value_name = "FORMAT")]
        node_format: NodeFormatArg,
    },
    /// Convert every map in a WAD between the UDMF and classic Doom formats.
    ///
    /// Maps already in the target format, and all non-map lumps, pass through
    /// unchanged in directory order. Conversion is lossy in one direction:
    /// Doom -> UDMF -> Doom is exact, but UDMF -> Doom rounds fractional
    /// coordinates and drops fields Doom cannot represent (linedef args and id;
    /// thing special, args, height, and id — the UDMF/Hexen tid). In strict mode
    /// any such loss is an error; `--lenient` accepts the loss and reports each
    /// instance as a warning, naming the field exactly as listed here.
    ///
    /// Converting to `doom` emits empty SEGS/SSECTORS/NODES/REJECT/BLOCKMAP
    /// lumps — pass `--nodes` to build them in place, or run an external
    /// nodebuilder (zdbsp, bsp) before playing the map.
    ///
    /// A converted map keeps only the lumps the target format defines. Any
    /// other lump inside the map group — BEHAVIOR (compiled ACS), SCRIPTS,
    /// ZNODES, DIALOGUE, ... — is bound to the source map's specials or
    /// geometry and is dropped rather than carried across: strict mode refuses
    /// and names each such lump; `--lenient` drops them and warns.
    ///
    /// Exits 0 on success, 2 on I/O or parse error, 3 if a map cannot be
    /// converted, if `--map NAME` matches no map in the WAD, or if the output
    /// WAD fails write validation (e.g. a pass-through lump whose name is not
    /// ASCII — such a name decodes under a lenient read but is rejected on
    /// write, in both strictness modes).
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
        /// If the name matches no map in the WAD the command exits with code 3.
        #[arg(short, long, value_name = "NAME")]
        map: Option<String>,
        /// Output WAD kind.
        #[arg(long, default_value = "pwad", value_name = "KIND")]
        kind: WadKindArg,
        /// Build engine-playable SEGS/SSECTORS/NODES/REJECT/BLOCKMAP (classic
        /// Doom output only; ignored for `--to udmf`).
        #[arg(long)]
        nodes: bool,
        /// On-disk node format for `--nodes` (classic Doom output only). GL
        /// streams (`xgln`/`xgl2`/`xgl3`/`gl` and their `z*` twins) are carried
        /// in `SSECTORS` instead of `NODES`. Every `z*` value requires cwad
        /// built with the `extended-nodes-zlib` feature.
        #[arg(long = "node-format", default_value = "classic", value_name = "FORMAT")]
        node_format: NodeFormatArg,
    },
}
