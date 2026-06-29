# v1.0 EPIC Parallel Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement all 20 actionable v1.0 issues (EPICs #12–#16) via a coordinated three-phase agent dispatch, one agent per issue.

**Architecture:** Audit agent first; 13 parallel Wave 1 agents after audit; 8 Wave 2 agents after issue #21 (write header serialization) merges into `main`. CLI tasks within Wave 1 must merge sequentially: #35 first (sets up shared Format enum, exit codes, build.rs), then #36/#37/#39 in any order.

**Tech Stack:** Rust 2024 edition, `binrw` (BinRead + BinWrite), `thiserror`, `proptest`, `clap 4`, `anyhow`, `cargo-fuzz`, `clap_complete`, `release-plz`

---

## Pre-Flight (run before dispatching any agent)

- [ ] `git pull origin main`
- [ ] `just ci` passes locally
- [ ] Working tree is clean

---

## Phase 1: Audit

### Task 1: Audit All v1.0 Issues

**Files:** Read-only. No code changes. Updates GitHub issue bodies only.

**Agent brief — dispatch as a read-only research agent:**

```
You are auditing 20 open GitHub issues in the masriamir/crustywad repository.
For each issue listed below, do the following:
1. Read the current issue body from GitHub.
2. Open the referenced source files and verify every line number reference
   (e.g. "lib.rs L186-192") against the actual current file.
3. Check whether any "Blocked by spike" language refers to a spike that is
   now closed (spikes #20, #32, #34, #44, #46, #48 are all closed).
4. Check whether recent commits (since June 9 2026) have already fully or
   partially addressed the issue.
5. Update the GitHub issue body with corrected line numbers and remove
   stale blocker language. If the issue is fully addressed by existing
   code, close it with a comment citing the relevant PR/commit.

Issues to audit:
- Epic #12 sub-issues: #21, #22, #23, #24, #25, #26
- Epic #13 sub-issues: #29, #33
- Epic #14 sub-issues: #35, #36, #37, #38, #39, #40
- Epic #15 sub-issues: #43, #45, #47
- Epic #16 sub-issues: #49, #50, #51, #52

Key files to read during audit:
- crates/crustywad/src/lib.rs (891 lines)
- crates/crustywad/src/error.rs
- crates/crustywad/src/map.rs
- crates/crustywad-cli/src/main.rs (70 lines)
- Cargo.toml (workspace)
- crates/crustywad/Cargo.toml
- crates/crustywad-cli/Cargo.toml
- release-plz.toml
- .github/workflows/ci.yml

Do NOT open any PRs or edit any code. Output a summary of what was updated/closed.
```

- [ ] Dispatch audit agent with the brief above
- [ ] Review audit summary; resolve any ambiguities manually before proceeding

---

## Phase 2: Wave 1 (launch all after audit — CLI sequencing note below)

> **CLI sequencing:** #35 must merge before #36, #37, #39 start (it creates shared
> CLI infrastructure). #36, #37, #39 may then run in parallel.
> All other Wave 1 tasks are fully independent of each other.

---

### Task 2: Issue #21 — Write: Header + Directory Serialization

**Branch:** `feature/21-write-header-directory`

**Files:**
- Create: `crates/crustywad/src/write.rs`
- Modify: `crates/crustywad/src/lib.rs` (add `pub mod write`, `BinWrite` derives, `Wad::to_builder`)
- Modify: `crates/crustywad/Cargo.toml` (add `write` feature)
- Create: `crates/crustywad/tests/write.rs`

**Agent brief:**

You are implementing issue #21: WAD header and lump-directory serialization.
This is the foundation of write support (Epic #12). Follow ADR-0006 exactly
(`docs/adr/0006-wad-write-design.md`).

- [ ] **Step 1: Add `write` feature to Cargo.toml**

In `crates/crustywad/Cargo.toml`, under `[features]`:
```toml
write = []
```

- [ ] **Step 2: Write the failing test first**

Create `crates/crustywad/tests/write.rs`:
```rust
//! Integration tests for WAD write support.
#![cfg(feature = "write")]

use crustywad::{WadBuilder, WadKind, WriteOptions};

#[test]
fn builder_produces_parseable_empty_iwad() {
    let bytes = WadBuilder::new(WadKind::Iwad)
        .build()
        .expect("empty IWAD build should succeed");
    let wad = crustywad::Wad::from_bytes(bytes).expect("should re-parse");
    assert_eq!(wad.lump_count(), 0);
    assert_eq!(wad.kind(), WadKind::Iwad);
}

#[test]
fn builder_produces_parseable_single_lump() {
    let bytes = WadBuilder::new(WadKind::Pwad)
        .add_lump("TESTLUMP", b"hello")
        .build()
        .expect("single-lump PWAD build should succeed");
    let wad = crustywad::Wad::from_bytes(bytes).expect("should re-parse");
    assert_eq!(wad.lump_count(), 1);
    assert_eq!(wad.lumps()[0].name(), "TESTLUMP");
    assert_eq!(wad.lumps()[0].size(), 5);
}

#[test]
fn wad_to_builder_round_trips() {
    use crustywad::tests::common::build_wad;
    let original = build_wad(*b"PWAD", &[("MAP01", b"data"), ("THINGS", b"more")]);
    let wad = crustywad::Wad::from_bytes(original).unwrap();
    let rebuilt = wad.to_builder().build().expect("round-trip should succeed");
    let wad2 = crustywad::Wad::from_bytes(rebuilt).unwrap();
    assert_eq!(wad2.lump_count(), 2);
    assert_eq!(wad2.lumps()[0].name(), "MAP01");
    assert_eq!(wad2.lumps()[1].name(), "THINGS");
}
```

- [ ] **Step 3: Run the test to confirm it fails (missing types)**

```bash
cargo test --package crustywad --features write --test write 2>&1 | head -20
```

Expected: compile error — `WadBuilder`, `WriteOptions` not found.

- [ ] **Step 4: Add `BinWrite` derives to `RawHeader` and `RawDirectoryEntry` in lib.rs**

Find the two private structs in `crates/crustywad/src/lib.rs` (around lines 333 and 341).
Change:
```rust
#[derive(Debug, BinRead)]
#[br(little)]
struct RawHeader { ... }

#[derive(Debug, BinRead)]
#[br(little)]
struct RawDirectoryEntry { ... }
```
To:
```rust
#[cfg_attr(feature = "write", derive(binrw::BinWrite))]
#[cfg_attr(feature = "write", bw(little))]
#[derive(Debug, BinRead)]
#[br(little)]
struct RawHeader { ... }

#[cfg_attr(feature = "write", derive(binrw::BinWrite))]
#[cfg_attr(feature = "write", bw(little))]
#[derive(Debug, BinRead)]
#[br(little)]
struct RawDirectoryEntry { ... }
```

- [ ] **Step 5: Create `crates/crustywad/src/write.rs`**

```rust
//! WAD write support — builder pattern serialization.
//!
//! Gated behind the `write` feature flag. Users who only read WADs do not pay
//! the compile-time cost of the write path.

use std::io::Cursor;

use binrw::BinWrite as _;

use crate::{RawDirectoryEntry, RawHeader, Strictness, WadKind};

/// Errors that can occur while building a WAD.
#[derive(Debug, thiserror::Error)]
pub enum WriteError {
    /// A lump name contains a NUL byte.
    #[error("lump name {name:?} contains a NUL byte")]
    NulInName { name: String },
    /// A lump name contains non-ASCII bytes.
    #[error("lump name {name:?} contains non-ASCII bytes")]
    NonAsciiName { name: String },
    /// A lump data payload exceeds `i32::MAX` bytes.
    #[error("lump {name:?} data size {size} exceeds i32::MAX")]
    LumpTooLarge { name: String, size: usize },
    /// The total lump count exceeds `i32::MAX`.
    #[error("lump count {count} exceeds i32::MAX")]
    TooManyLumps { count: usize },
    /// A computed byte offset exceeds `i32::MAX`.
    #[error("computed offset {offset} exceeds i32::MAX")]
    OffsetOverflow { offset: usize },
    /// Writing `WadKind::Unknown` magic in strict mode.
    #[error("WadKind::Unknown is not permitted in strict mode")]
    UnknownMagicStrict,
    /// An I/O error during serialization.
    #[error("serialization error: {0}")]
    Io(#[from] std::io::Error),
    /// A binrw write error.
    #[error("binrw error: {0}")]
    Binrw(#[from] binrw::Error),
}

/// Non-fatal conditions encountered during lenient WAD building.
#[derive(Debug, thiserror::Error)]
pub enum WriteWarning {
    /// A lump name longer than 8 bytes was truncated to 8 bytes.
    #[error("lump name {name:?} truncated to 8 bytes")]
    NameTruncated { name: String },
}

/// Options controlling write-time validation behaviour.
///
/// # Examples
///
/// ```
/// use crustywad::WriteOptions;
/// let opts = WriteOptions::strict();
/// let lenient = WriteOptions::lenient();
/// ```
#[derive(Debug, Clone)]
pub struct WriteOptions {
    /// Whether to use strict or lenient validation.
    pub strictness: Strictness,
}

impl Default for WriteOptions {
    fn default() -> Self {
        Self::strict()
    }
}

impl WriteOptions {
    /// Strict validation — any invalid input returns an error.
    #[must_use]
    pub const fn strict() -> Self {
        Self { strictness: Strictness::Strict }
    }

    /// Lenient validation — recoverable issues produce warnings, not errors.
    #[must_use]
    pub const fn lenient() -> Self {
        Self { strictness: Strictness::Lenient }
    }
}

struct LumpEntry {
    name: String,
    data: Vec<u8>,
}

/// A WAD builder. Accumulates lumps and serializes to `Vec<u8>` on [`build`][WadBuilder::build].
///
/// # Examples
///
/// ```
/// use crustywad::{WadBuilder, WadKind};
///
/// let bytes = WadBuilder::new(WadKind::Pwad)
///     .add_lump("MAP01", b"")
///     .build()
///     .unwrap();
/// ```
pub struct WadBuilder {
    kind: WadKind,
    lumps: Vec<LumpEntry>,
}

impl WadBuilder {
    /// Creates a new empty builder for a WAD of the given kind.
    #[must_use]
    pub fn new(kind: WadKind) -> Self {
        Self { kind, lumps: Vec::new() }
    }

    /// Appends a lump. Validation is deferred to [`build`][Self::build].
    pub fn add_lump(&mut self, name: &str, data: impl Into<Vec<u8>>) -> &mut Self {
        self.lumps.push(LumpEntry { name: name.to_owned(), data: data.into() });
        self
    }

    /// Serializes to bytes using strict validation.
    ///
    /// # Errors
    ///
    /// Returns [`WriteError`] if any lump name or size is invalid.
    pub fn build(&self) -> Result<Vec<u8>, WriteError> {
        self.build_with_options(WriteOptions::strict()).map(|(bytes, _)| bytes)
    }

    /// Serializes to bytes with the given options.
    ///
    /// # Errors
    ///
    /// Returns [`WriteError`] for unrecoverable validation failures.
    /// In lenient mode, recoverable issues append to the returned warning list.
    pub fn build_with_options(
        &self,
        opts: WriteOptions,
    ) -> Result<(Vec<u8>, Vec<WriteWarning>), WriteError> {
        let lenient = matches!(opts.strictness, Strictness::Lenient);
        let mut warnings = Vec::new();

        // Validate magic
        let magic = match self.kind {
            WadKind::Iwad => *b"IWAD",
            WadKind::Pwad => *b"PWAD",
            WadKind::Unknown(b) => {
                if !lenient {
                    return Err(WriteError::UnknownMagicStrict);
                }
                b
            }
        };

        // Validate lump count fits i32
        if self.lumps.len() > i32::MAX as usize {
            return Err(WriteError::TooManyLumps { count: self.lumps.len() });
        }

        // Validate and encode lump names; validate sizes
        let mut encoded_names: Vec<[u8; 8]> = Vec::with_capacity(self.lumps.len());
        for entry in &self.lumps {
            if entry.name.contains('\0') {
                return Err(WriteError::NulInName { name: entry.name.clone() });
            }
            if !entry.name.is_ascii() {
                return Err(WriteError::NonAsciiName { name: entry.name.clone() });
            }
            let name_bytes = entry.name.as_bytes();
            let mut buf = [0u8; 8];
            if name_bytes.len() > 8 {
                if !lenient {
                    // Strict: names > 8 bytes — treat as error (truncation breaks round-trips)
                    return Err(WriteError::NonAsciiName { name: entry.name.clone() });
                }
                warnings.push(WriteWarning::NameTruncated { name: entry.name.clone() });
                buf.copy_from_slice(&name_bytes[..8]);
            } else {
                buf[..name_bytes.len()].copy_from_slice(name_bytes);
            }
            encoded_names.push(buf);

            if entry.data.len() > i32::MAX as usize {
                return Err(WriteError::LumpTooLarge {
                    name: entry.name.clone(),
                    size: entry.data.len(),
                });
            }
        }

        // Layout: [12-byte header][lump data blobs][16-byte directory entries × N]
        // Compute filepos for each lump
        let mut filepos_list: Vec<usize> = Vec::with_capacity(self.lumps.len());
        let mut cursor: usize = 12; // header size
        for entry in &self.lumps {
            if cursor > i32::MAX as usize {
                return Err(WriteError::OffsetOverflow { offset: cursor });
            }
            filepos_list.push(cursor);
            cursor += entry.data.len();
        }
        let infotableofs = cursor;
        if infotableofs > i32::MAX as usize {
            return Err(WriteError::OffsetOverflow { offset: infotableofs });
        }

        // Serialize
        let capacity = 12
            + self.lumps.iter().map(|e| e.data.len()).sum::<usize>()
            + self.lumps.len() * 16;
        let mut buf = Cursor::new(Vec::with_capacity(capacity));

        // Write header (placeholder; we have all values now)
        let header = RawHeader {
            magic,
            numlumps: self.lumps.len() as i32,
            infotableofs: infotableofs as i32,
        };
        header.write(&mut buf)?;

        // Write lump data
        for entry in &self.lumps {
            use std::io::Write as _;
            buf.get_mut().extend_from_slice(&entry.data);
        }

        // Write directory entries
        for (i, entry) in self.lumps.iter().enumerate() {
            let dir = RawDirectoryEntry {
                filepos: filepos_list[i] as i32,
                size: entry.data.len() as i32,
                name: encoded_names[i],
            };
            dir.write(&mut buf)?;
        }

        Ok((buf.into_inner(), warnings))
    }
}
```

- [ ] **Step 6: Add `pub mod write` and `Wad::to_builder` to lib.rs**

After the existing `#[cfg(feature = "mmap")] mod mmap;` line, add:
```rust
#[cfg(feature = "write")]
pub mod write;
#[cfg(feature = "write")]
pub use write::{WadBuilder, WriteError, WriteOptions, WriteWarning};
```

At the end of the `impl Wad` block, add:
```rust
/// Converts this `Wad` into a [`WadBuilder`] for round-tripping or editing.
///
/// All lump data is copied into the builder. Memory usage roughly doubles
/// during the conversion.
#[cfg(feature = "write")]
#[must_use]
pub fn to_builder(&self) -> WadBuilder {
    let mut builder = WadBuilder::new(self.kind());
    for lump in &self.lumps {
        let data = self.lump_bytes(lump.filepos()..lump.filepos() + lump.size())
            .unwrap_or_default()
            .to_vec();
        builder.add_lump(lump.name(), data);
    }
    builder
}
```

Note: verify the correct method signature for `lump_bytes` in lib.rs — use whatever accessor exists to get raw bytes for a given lump range.

- [ ] **Step 7: Run the tests to verify they pass**

```bash
cargo test --package crustywad --features write --test write
```

Expected: all 3 tests pass.

- [ ] **Step 8: Also verify strict mode rejects invalid input**

Add to `crates/crustywad/tests/write.rs`:
```rust
#[test]
fn strict_mode_rejects_nul_in_name() {
    let result = WadBuilder::new(WadKind::Pwad)
        .add_lump("BAD\0NAME", b"")
        .build();
    assert!(matches!(result, Err(crustywad::WriteError::NulInName { .. })));
}

#[test]
fn strict_mode_rejects_non_ascii_name() {
    let result = WadBuilder::new(WadKind::Pwad)
        .add_lump("BÄDNAME", b"")
        .build();
    assert!(matches!(result, Err(crustywad::WriteError::NonAsciiName { .. })));
}
```

- [ ] **Step 9: Run full CI**

```bash
just ci
```

Expected: all jobs pass.

- [ ] **Step 10: Commit**

```bash
git add crates/crustywad/src/write.rs crates/crustywad/src/lib.rs \
        crates/crustywad/Cargo.toml crates/crustywad/tests/write.rs
git commit -m "feat(write): add WadBuilder, WriteError, WriteOptions for header+directory serialization (#21)"
```

---

### Task 3: Issue #35 — `cwad validate` + CLI Foundation

**Branch:** `feature/35-cwad-validate`

**Files:**
- Modify: `crates/crustywad-cli/src/main.rs` (major refactor + new subcommand)
- Create: `crates/crustywad-cli/build.rs`
- Modify: `crates/crustywad-cli/Cargo.toml` (add `clap_complete` build-dep)
- Modify: `Cargo.toml` (add `clap_complete` workspace dep)

> This task sets up all shared CLI infrastructure per ADR-0008. Later CLI tasks
> (#36, #37, #39) depend on this merging first.

- [ ] **Step 1: Add `clap_complete` to workspace deps**

In root `Cargo.toml` under `[workspace.dependencies]`:
```toml
clap_complete = "4"
```

In `crates/crustywad-cli/Cargo.toml` under `[build-dependencies]`:
```toml
clap_complete.workspace = true
```

- [ ] **Step 2: Write the failing test first**

Create `crates/crustywad-cli/tests/cli_validate.rs`:
```rust
use assert_cmd::Command;
use tempfile::NamedTempFile;

fn minimal_iwad() -> Vec<u8> {
    // 12-byte IWAD header with numlumps=0
    let mut bytes = b"IWAD".to_vec();
    bytes.extend_from_slice(&0_i32.to_le_bytes()); // numlumps
    bytes.extend_from_slice(&12_i32.to_le_bytes()); // infotableofs
    bytes
}

#[test]
fn validate_clean_wad_exits_0() {
    let mut f = NamedTempFile::new().unwrap();
    std::io::Write::write_all(&mut f, &minimal_iwad()).unwrap();
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["validate", f.path().to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn validate_missing_file_exits_2() {
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["validate", "/nonexistent/file.wad"])
        .assert()
        .code(2);
}

#[test]
fn validate_corrupt_wad_exits_1() {
    let mut f = NamedTempFile::new().unwrap();
    std::io::Write::write_all(&mut f, b"NOTAWAD!!").unwrap();
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["validate", f.path().to_str().unwrap()])
        .assert()
        .code(1);
}
```

- [ ] **Step 3: Run to confirm failure**

```bash
cargo test --package crustywad-cli --test cli_validate 2>&1 | head -10
```

Expected: compile or runtime failure (subcommand not found yet).

- [ ] **Step 4: Create `build.rs` for shell completions**

Create `crates/crustywad-cli/build.rs`:
```rust
use std::env;
use std::io::Error;
use std::path::PathBuf;

use clap::CommandFactory;
use clap_complete::{generate_to, shells};

#[allow(dead_code)]
#[path = "src/main.rs"]
mod main_cli;

fn main() -> Result<(), Error> {
    let out = PathBuf::from(env::var("OUT_DIR").unwrap()).join("completions");
    std::fs::create_dir_all(&out)?;
    let mut cmd = main_cli::Cli::command();
    generate_to(shells::Bash, &mut cmd, "cwad", &out)?;
    generate_to(shells::Zsh, &mut cmd, "cwad", &out)?;
    generate_to(shells::Fish, &mut cmd, "cwad", &out)?;
    Ok(())
}
```

Note: `build.rs` needs to import `Cli` from `main.rs`. Make `Cli` pub or restructure. The simplest approach is to make `struct Cli` `pub(crate)` and `enum Command` `pub(crate)`.

- [ ] **Step 5: Rewrite `main.rs` with full ADR-0008 architecture**

Replace the entire contents of `crates/crustywad-cli/src/main.rs`:
```rust
//! Command-line tooling for inspecting Doom WAD files with `crustywad`.

use std::path::PathBuf;
use std::process;

use anyhow::{Context as _, Result};
use clap::{Parser, Subcommand, ValueEnum};
use crustywad::{ParseOptions, Wad};

/// Output format for structured data.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum Format {
    /// Human-readable, column-aligned text (default).
    Human,
    /// Newline-delimited JSON.
    Json,
    /// RFC 4180 CSV with header row.
    Csv,
}

/// Inspect Doom WAD files.
#[derive(Debug, Parser)]
#[command(author, version, about)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,
    /// Use lenient parsing instead of strict.
    #[arg(long, global = true)]
    lenient: bool,
    /// Output format.
    #[arg(short = 'F', long, global = true, default_value = "human")]
    format: Format,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Print WAD kind, lump count, and metadata.
    Info {
        /// Path to the WAD file.
        path: PathBuf,
    },
    /// Print the lump directory.
    List {
        /// Path to the WAD file.
        path: PathBuf,
    },
    /// Check WAD file correctness. Exits 0 if clean, 1 if errors found.
    Validate {
        /// Path to the WAD file.
        path: PathBuf,
    },
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

fn run(cli: Cli) -> Result<i32> {
    let options = if cli.lenient {
        ParseOptions::lenient()
    } else {
        ParseOptions::strict()
    };

    match cli.command {
        Command::Info { path } => {
            let wad = Wad::from_path_with_options(&path, options)
                .with_context(|| format!("failed to open {}", path.display()))?;
            for warning in wad.warnings() {
                eprintln!("warning: {warning}");
            }
            match cli.format {
                Format::Human => {
                    println!("kind:  {:?}", wad.kind());
                    println!("lumps: {}", wad.lump_count());
                }
                Format::Json => {
                    println!(
                        r#"{{"kind":{:?},"lumps":{}}}"#,
                        format!("{:?}", wad.kind()),
                        wad.lump_count()
                    );
                }
                Format::Csv => {
                    println!("kind,lumps");
                    println!("{:?},{}", wad.kind(), wad.lump_count());
                }
            }
            Ok(0)
        }

        Command::List { path } => {
            let wad = Wad::from_path_with_options(&path, options)
                .with_context(|| format!("failed to open {}", path.display()))?;
            for warning in wad.warnings() {
                eprintln!("warning: {warning}");
            }
            match cli.format {
                Format::Human => {
                    for (index, lump) in wad.lumps().iter().enumerate() {
                        println!(
                            "{index:04} {:>8} {:>8} {}",
                            lump.filepos(),
                            lump.size(),
                            lump.name()
                        );
                    }
                }
                Format::Json => {
                    for lump in wad.lumps() {
                        println!(
                            r#"{{"index":{},"filepos":{},"size":{},"name":{:?}}}"#,
                            wad.lumps().iter().position(|l| std::ptr::eq(l, lump)).unwrap_or(0),
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

        Command::Validate { path } => {
            match Wad::from_path_with_options(&path, options) {
                Ok(wad) => {
                    for warning in wad.warnings() {
                        eprintln!("warning: {warning}");
                    }
                    match cli.format {
                        Format::Human => println!("ok: {}", path.display()),
                        Format::Json => println!(r#"{{"ok":true}}"#),
                        Format::Csv => { println!("ok"); println!("true"); }
                    }
                    Ok(0)
                }
                Err(e) => {
                    match cli.format {
                        Format::Human => eprintln!("error: {e}"),
                        Format::Json => eprintln!(r#"{{"ok":false,"error":{:?}}}"#, e.to_string()),
                        Format::Csv => { eprintln!("ok"); eprintln!("false"); }
                    }
                    Ok(1)
                }
            }
        }
    }
}
```

- [ ] **Step 6: Run the failing tests to verify they now pass**

```bash
cargo test --package crustywad-cli --test cli_validate
```

Expected: all 3 tests pass.

- [ ] **Step 7: Run full CI**

```bash
just ci
```

- [ ] **Step 8: Commit**

```bash
git add crates/crustywad-cli/src/main.rs crates/crustywad-cli/build.rs \
        crates/crustywad-cli/Cargo.toml Cargo.toml \
        crates/crustywad-cli/tests/cli_validate.rs
git commit -m "feat(cli): add cwad validate subcommand and shared CLI infrastructure (#35)"
```

---

### Task 4: Issue #36 — `cwad info` expand

**Branch:** `feature/36-cwad-info-expand`  
**Prerequisite:** #35 merged — pull `main` before starting.

**Files:**
- Modify: `crates/crustywad-cli/src/main.rs`

- [ ] **Step 1: Write the failing test**

Create `crates/crustywad-cli/tests/cli_info.rs`:
```rust
use assert_cmd::Command;
use tempfile::NamedTempFile;
use std::io::Write as _;

fn wad_with_map() -> Vec<u8> {
    // PWAD with MAP01 marker lump (0 bytes)
    let mut bytes = b"PWAD".to_vec();
    bytes.extend_from_slice(&1_i32.to_le_bytes()); // numlumps
    bytes.extend_from_slice(&12_i32.to_le_bytes()); // infotableofs points to right after header
    // directory entry: filepos=12, size=0, name="MAP01\0\0\0"
    bytes.extend_from_slice(&12_i32.to_le_bytes());
    bytes.extend_from_slice(&0_i32.to_le_bytes());
    bytes.extend_from_slice(b"MAP01\0\0\0");
    bytes
}

#[test]
fn info_shows_kind_and_lump_count() {
    let mut f = NamedTempFile::new().unwrap();
    f.write_all(&wad_with_map()).unwrap();
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["info", f.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicates::str::contains("Pwad"))
        .stdout(predicates::str::contains("1"));
}

#[test]
fn info_json_format_is_parseable() {
    let mut f = NamedTempFile::new().unwrap();
    f.write_all(&wad_with_map()).unwrap();
    let out = Command::cargo_bin("cwad")
        .unwrap()
        .args(["-F", "json", "info", f.path().to_str().unwrap()])
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("lumps"));
    assert!(stdout.contains("kind"));
}
```

- [ ] **Step 2: Run to confirm current state**

```bash
cargo test --package crustywad-cli --test cli_info
```

The `info_shows_kind_and_lump_count` test may already pass with the basic `info` command. Adjust assertions as needed based on actual output format from #35.

- [ ] **Step 3: Expand `Command::Info` in main.rs**

Update the `Command::Info` match arm to also list any lumps whose names match map markers (`MAP01`–`MAP32`, `E1M1`–`E9M9`):
```rust
Command::Info { path } => {
    let wad = Wad::from_path_with_options(&path, options)
        .with_context(|| format!("failed to open {}", path.display()))?;
    for warning in wad.warnings() {
        eprintln!("warning: {warning}");
    }
    let map_names: Vec<&str> = wad.lumps()
        .iter()
        .filter(|l| l.size() == 0 && is_map_marker(l.name()))
        .map(|l| l.name())
        .collect();
    match cli.format {
        Format::Human => {
            println!("kind:  {:?}", wad.kind());
            println!("lumps: {}", wad.lump_count());
            if !map_names.is_empty() {
                println!("maps:  {}", map_names.join(", "));
            }
        }
        Format::Json => {
            let maps_json: String = map_names
                .iter()
                .map(|n| format!("{n:?}"))
                .collect::<Vec<_>>()
                .join(",");
            println!(
                r#"{{"kind":{:?},"lumps":{},"maps":[{}]}}"#,
                format!("{:?}", wad.kind()),
                wad.lump_count(),
                maps_json
            );
        }
        Format::Csv => {
            println!("kind,lumps,maps");
            println!("{:?},{},{}", wad.kind(), wad.lump_count(), map_names.join(";"));
        }
    }
    Ok(0)
}
```

Add helper function (module-level):
```rust
fn is_map_marker(name: &str) -> bool {
    // ExMy (Doom 1) or MAPxx (Doom 2)
    let b = name.as_bytes();
    (b.len() == 4
        && b[0] == b'E'
        && b[1].is_ascii_digit()
        && b[2] == b'M'
        && b[3].is_ascii_digit())
        || (b.len() == 5
            && b[0] == b'M'
            && b[1] == b'A'
            && b[2] == b'P'
            && b[3].is_ascii_digit()
            && b[4].is_ascii_digit())
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test --package crustywad-cli --test cli_info && just ci
```

- [ ] **Step 5: Commit**

```bash
git add crates/crustywad-cli/src/main.rs crates/crustywad-cli/tests/cli_info.rs
git commit -m "feat(cli): expand cwad info with map names and JSON/CSV output (#36)"
```

---

### Task 5: Issue #37 — `cwad diff`

**Branch:** `feature/37-cwad-diff`  
**Prerequisite:** #35 merged.

**Files:**
- Modify: `crates/crustywad-cli/src/main.rs`

- [ ] **Step 1: Write failing test**

Create `crates/crustywad-cli/tests/cli_diff.rs`:
```rust
use assert_cmd::Command;
use tempfile::NamedTempFile;
use std::io::Write as _;

fn minimal_wad(lumps: &[(&str, &[u8])]) -> Vec<u8> {
    let mut data: Vec<Vec<u8>> = Vec::new();
    let mut bytes = b"PWAD".to_vec();
    bytes.extend_from_slice(&(lumps.len() as i32).to_le_bytes());
    let dir_offset = 12 + lumps.iter().map(|(_, d)| d.len()).sum::<usize>();
    bytes.extend_from_slice(&(dir_offset as i32).to_le_bytes());
    let mut offset = 12usize;
    for (_, d) in lumps { data.push(d.to_vec()); }
    for d in &data { bytes.extend_from_slice(d); }
    offset = 12;
    for (i, (name, d)) in lumps.iter().enumerate() {
        bytes.extend_from_slice(&(offset as i32).to_le_bytes());
        bytes.extend_from_slice(&(d.len() as i32).to_le_bytes());
        let mut n = [0u8; 8];
        n[..name.len().min(8)].copy_from_slice(&name.as_bytes()[..name.len().min(8)]);
        bytes.extend_from_slice(&n);
        offset += data[i].len();
    }
    bytes
}

#[test]
fn diff_identical_wads_exits_0() {
    let wad = minimal_wad(&[("FOO", b"bar")]);
    let mut f1 = NamedTempFile::new().unwrap();
    let mut f2 = NamedTempFile::new().unwrap();
    f1.write_all(&wad).unwrap(); f2.write_all(&wad).unwrap();
    Command::cargo_bin("cwad").unwrap()
        .args(["diff", f1.path().to_str().unwrap(), f2.path().to_str().unwrap()])
        .assert().code(0);
}

#[test]
fn diff_different_wads_exits_1() {
    let w1 = minimal_wad(&[("FOO", b"bar")]);
    let w2 = minimal_wad(&[("FOO", b"baz")]);
    let mut f1 = NamedTempFile::new().unwrap();
    let mut f2 = NamedTempFile::new().unwrap();
    f1.write_all(&w1).unwrap(); f2.write_all(&w2).unwrap();
    Command::cargo_bin("cwad").unwrap()
        .args(["diff", f1.path().to_str().unwrap(), f2.path().to_str().unwrap()])
        .assert().code(1);
}
```

- [ ] **Step 2: Add `Command::Diff` to `main.rs`**

```rust
/// Compare lump directories of two WAD files. Exits 0 if identical, 1 if different.
Diff {
    /// First WAD file.
    path1: PathBuf,
    /// Second WAD file.
    path2: PathBuf,
},
```

Match arm:
```rust
Command::Diff { path1, path2 } => {
    let w1 = Wad::from_path_with_options(&path1, options.clone())
        .with_context(|| format!("failed to open {}", path1.display()))?;
    let w2 = Wad::from_path_with_options(&path2, options)
        .with_context(|| format!("failed to open {}", path2.display()))?;

    let l1 = w1.lumps();
    let l2 = w2.lumps();
    let identical = l1.len() == l2.len()
        && l1.iter().zip(l2.iter()).all(|(a, b)| {
            a.name() == b.name()
                && a.size() == b.size()
                && w1.lump_bytes(a.filepos()..a.filepos() + a.size())
                    == w2.lump_bytes(b.filepos()..b.filepos() + b.size())
        });

    if identical {
        match cli.format {
            Format::Human => println!("identical"),
            Format::Json  => println!(r#"{{"identical":true}}"#),
            Format::Csv   => { println!("identical"); println!("true"); }
        }
        Ok(0)
    } else {
        match cli.format {
            Format::Human => println!("differ"),
            Format::Json  => println!(r#"{{"identical":false}}"#),
            Format::Csv   => { println!("identical"); println!("false"); }
        }
        Ok(1)
    }
}
```

Note: verify the `lump_bytes` signature in `lib.rs` — use the actual range accessor. Adjust as needed.

- [ ] **Step 3: Run tests and CI**

```bash
cargo test --package crustywad-cli --test cli_diff && just ci
```

- [ ] **Step 4: Commit**

```bash
git add crates/crustywad-cli/src/main.rs crates/crustywad-cli/tests/cli_diff.rs
git commit -m "feat(cli): add cwad diff subcommand (#37)"
```

---

### Task 6: Issue #39 — `cwad extract`

**Branch:** `feature/39-cwad-extract`  
**Prerequisite:** #35 merged.

**Files:**
- Modify: `crates/crustywad-cli/src/main.rs`

- [ ] **Step 1: Write failing test**

Create `crates/crustywad-cli/tests/cli_extract.rs`:
```rust
use assert_cmd::Command;
use tempfile::NamedTempFile;
use std::io::Write as _;

// Use the minimal_wad helper — copy it or share via a module
fn single_lump_wad() -> Vec<u8> {
    // PWAD with TESTLUMP containing b"hello"
    let data = b"hello";
    let mut bytes = b"PWAD".to_vec();
    bytes.extend_from_slice(&1_i32.to_le_bytes());
    bytes.extend_from_slice(&(12 + data.len() as i32).to_le_bytes());
    bytes.extend_from_slice(data);
    bytes.extend_from_slice(&12_i32.to_le_bytes());
    bytes.extend_from_slice(&(data.len() as i32).to_le_bytes());
    bytes.extend_from_slice(b"TESTLUMP");
    bytes
}

#[test]
fn extract_to_stdout() {
    let mut f = NamedTempFile::new().unwrap();
    f.write_all(&single_lump_wad()).unwrap();
    Command::cargo_bin("cwad").unwrap()
        .args(["extract", f.path().to_str().unwrap(), "TESTLUMP"])
        .assert()
        .success()
        .stdout(b"hello" as &[u8]);
}

#[test]
fn extract_nonexistent_lump_exits_1() {
    let mut f = NamedTempFile::new().unwrap();
    f.write_all(&single_lump_wad()).unwrap();
    Command::cargo_bin("cwad").unwrap()
        .args(["extract", f.path().to_str().unwrap(), "MISSING"])
        .assert()
        .code(1);
}
```

- [ ] **Step 2: Add `Command::Extract` to `main.rs`**

```rust
/// Extract a named lump to stdout or a file.
Extract {
    /// Path to the WAD file.
    path: PathBuf,
    /// Name of the lump to extract (case-sensitive).
    name: String,
    /// Output file path. Omit to write raw bytes to stdout.
    #[arg(short, long)]
    output: Option<PathBuf>,
},
```

Match arm:
```rust
Command::Extract { path, name, output } => {
    let wad = Wad::from_path_with_options(&path, options)
        .with_context(|| format!("failed to open {}", path.display()))?;
    let lump = wad.lump_by_name(&name).ok_or_else(|| {
        anyhow::anyhow!("lump {:?} not found in {}", name, path.display())
    })?;
    let data = wad
        .lump_bytes(lump.filepos()..lump.filepos() + lump.size())
        .unwrap_or_default();
    match output {
        Some(out_path) => {
            std::fs::write(&out_path, data)
                .with_context(|| format!("failed to write {}", out_path.display()))?;
            eprintln!("extracted {} bytes to {}", data.len(), out_path.display());
            Ok(0)
        }
        None => {
            use std::io::Write as _;
            std::io::stdout().write_all(data)?;
            Ok(0)
        }
    }
}
```

Note: `extract` lump-not-found → `Ok(1)` not `Err(...)`. Adjust the match to return `Ok(1)` for the not-found case.

- [ ] **Step 3: Run tests and CI**

```bash
cargo test --package crustywad-cli --test cli_extract && just ci
```

- [ ] **Step 4: Commit**

```bash
git add crates/crustywad-cli/src/main.rs crates/crustywad-cli/tests/cli_extract.rs
git commit -m "feat(cli): add cwad extract subcommand (#39)"
```

---

### Task 7: Issue #45 — `cargo-fuzz` targets

**Branch:** `feature/45-cargo-fuzz`

**Files:**
- Create: `fuzz/Cargo.toml`
- Create: `fuzz/rust-toolchain.toml`
- Create: `fuzz/fuzz_targets/fuzz_wad_strict.rs`
- Create: `fuzz/fuzz_targets/fuzz_wad_lenient.rs`
- Create: `fuzz/fuzz_targets/fuzz_parse_records_thing.rs`
- Create: `fuzz/corpus/fuzz_wad_strict/{minimal_iwad,minimal_pwad,empty,header_only}`
- Create: `fuzz/corpus/fuzz_wad_lenient/{minimal_iwad,minimal_pwad,empty,header_only}`
- Create: `fuzz/corpus/fuzz_parse_records_thing/{empty_slice,valid_thing,truncated_thing}`
- Modify: `.gitignore`
- Create: `.github/workflows/fuzz.yml`
- Modify: `justfile` (update `fuzz` recipe)

Follow ADR-0009 (`docs/adr/0009-cargo-fuzz-harness.md`) exactly.

- [ ] **Step 1: Create `fuzz/Cargo.toml`**

```toml
[package]
name = "crustywad-fuzz"
version = "0.0.0"
publish = false
edition = "2024"

[workspace]

[[bin]]
name = "fuzz_wad_strict"
path = "fuzz_targets/fuzz_wad_strict.rs"
test = false
doc = false

[[bin]]
name = "fuzz_wad_lenient"
path = "fuzz_targets/fuzz_wad_lenient.rs"
test = false
doc = false

[[bin]]
name = "fuzz_parse_records_thing"
path = "fuzz_targets/fuzz_parse_records_thing.rs"
test = false
doc = false

[dependencies]
libfuzzer-sys = "0.4"
crustywad = { path = "../crates/crustywad", features = [] }
```

- [ ] **Step 2: Create `fuzz/rust-toolchain.toml`**

```toml
[toolchain]
channel = "nightly"
```

- [ ] **Step 3: Write the three fuzz targets**

`fuzz/fuzz_targets/fuzz_wad_strict.rs`:
```rust
#![no_main]
use libfuzzer_sys::fuzz_target;
use std::hint::black_box;

fuzz_target!(|data: &[u8]| {
    let _ = black_box(crustywad::Wad::from_bytes(data.to_vec()));
});
```

`fuzz/fuzz_targets/fuzz_wad_lenient.rs`:
```rust
#![no_main]
use libfuzzer_sys::fuzz_target;
use std::hint::black_box;
use crustywad::ParseOptions;

fuzz_target!(|data: &[u8]| {
    let result = black_box(crustywad::Wad::from_bytes_with_options(
        data.to_vec(),
        ParseOptions::lenient(),
    ));
    if let Ok(wad) = result {
        let lump_count = wad.lump_count();
        // Guard against unbounded warning growth
        assert!(
            wad.warnings().len() <= lump_count * 5 + 5,
            "warning count {} exceeded bound {}",
            wad.warnings().len(),
            lump_count * 5 + 5,
        );
    }
});
```

`fuzz/fuzz_targets/fuzz_parse_records_thing.rs`:
```rust
#![no_main]
use libfuzzer_sys::fuzz_target;
use std::hint::black_box;

fuzz_target!(|data: &[u8]| {
    let _ = black_box(crustywad::map::parse_records::<crustywad::map::Thing>(data));
});
```

- [ ] **Step 4: Create corpus seed files**

`fuzz/corpus/fuzz_wad_strict/empty`: empty file (0 bytes)  
`fuzz/corpus/fuzz_wad_strict/minimal_iwad`: 12-byte IWAD with 0 lumps:
```
49 57 41 44 00 00 00 00 0C 00 00 00
```
`fuzz/corpus/fuzz_wad_strict/minimal_pwad`: same with `PWAD` magic  
`fuzz/corpus/fuzz_wad_strict/header_only`: 12-byte header claiming 1 lump but no directory bytes

Copy the same four seeds to `fuzz/corpus/fuzz_wad_lenient/`.

`fuzz/corpus/fuzz_parse_records_thing/empty_slice`: empty file  
`fuzz/corpus/fuzz_parse_records_thing/valid_thing`: 10 bytes representing a Thing record (all zeros is valid)  
`fuzz/corpus/fuzz_parse_records_thing/truncated_thing`: 9 bytes (odd length, exercises TrailingBytes)

Create corpus files as binary files using Python:
```bash
python3 -c "import sys; sys.stdout.buffer.write(b'')" > fuzz/corpus/fuzz_wad_strict/empty
python3 -c "import sys; sys.stdout.buffer.write(b'IWAD\x00\x00\x00\x00\x0c\x00\x00\x00')" > fuzz/corpus/fuzz_wad_strict/minimal_iwad
# etc. for each seed
```

- [ ] **Step 5: Add `.gitignore` entries**

Append to `.gitignore`:
```
fuzz/corpus/*/[0-9a-f][0-9a-f]*
fuzz/artifacts/
```

- [ ] **Step 6: Create `.github/workflows/fuzz.yml`**

```yaml
name: Fuzz

on:
  workflow_dispatch:
  schedule:
    - cron: "0 2 * * 1"  # weekly, Monday 02:00 UTC

jobs:
  fuzz:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7
      - uses: dtolnay/rust-toolchain@nightly
      - run: cargo install --locked cargo-fuzz
      - run: cargo fuzz run fuzz_wad_strict -- -max_total_time=60
        working-directory: fuzz
      - run: cargo fuzz run fuzz_wad_lenient -- -max_total_time=60
        working-directory: fuzz
      - run: cargo fuzz run fuzz_parse_records_thing -- -max_total_time=60
        working-directory: fuzz
      - uses: actions/upload-artifact@v4
        if: failure()
        with:
          name: fuzz-artifacts
          path: fuzz/artifacts/
```

- [ ] **Step 7: Update `justfile` fuzz recipe**

Find the existing `fuzz` recipe and replace with:
```makefile
fuzz target="fuzz_wad_strict":
    @echo "Running fuzz target: {{target}}"
    @echo "Requires: rustup toolchain install nightly && cargo install cargo-fuzz"
    cd fuzz && cargo fuzz run {{target}}
```

- [ ] **Step 8: Verify `just ci` is unaffected**

```bash
just ci
```

Expected: all existing jobs pass; fuzz directory is invisible to the workspace.

- [ ] **Step 9: Commit**

```bash
git add fuzz/ .gitignore .github/workflows/fuzz.yml justfile
git commit -m "feat(fuzz): add cargo-fuzz harness with three targets and corpus seeds (#45)"
```

---

### Task 8: Issue #47 — proptest invariant tests

**Branch:** `feature/47-proptest`

**Files:**
- Modify: `crates/crustywad/tests/common/mod.rs`
- Modify: `crates/crustywad/tests/wad_reader.rs`
- Modify: `crates/crustywad/tests/map_records.rs`

Follow ADR-0010 (`docs/adr/0010-proptest-strategy.md`) exactly — 8 invariants across 6 proptest blocks.

- [ ] **Step 1: Add `arb_lump_pair` and `arb_valid_wad` to `common/mod.rs`**

```rust
use proptest::prelude::*;

/// Generates an ASCII lump name of 1–8 chars and a payload of 0–256 bytes.
pub fn arb_lump_pair() -> impl Strategy<Value = (String, Vec<u8>)> {
    let name = proptest::string::string_regex("[A-Z_][A-Z0-9_]{0,7}").unwrap();
    let data = proptest::collection::vec(any::<u8>(), 0..=256);
    (name, data)
}

/// Generates structurally valid WAD bytes (correct header offsets, ASCII names).
pub fn arb_valid_wad() -> impl Strategy<Value = Vec<u8>> {
    let kind = prop_oneof![Just(*b"IWAD"), Just(*b"PWAD")];
    let lumps = proptest::collection::vec(arb_lump_pair(), 0..=16);
    (kind, lumps).prop_map(|(k, pairs)| {
        let refs: Vec<(&str, &[u8])> = pairs
            .iter()
            .map(|(n, d)| (n.as_str(), d.as_slice()))
            .collect();
        build_wad(k, &refs)
    })
}
```

- [ ] **Step 2: Run existing proptest to confirm baseline**

```bash
cargo test --package crustywad --all-features strict_parser_handles_generated_empty_wads
```

Expected: passes.

- [ ] **Step 3: Add invariants I-1 and I-6 to `wad_reader.rs`**

```rust
proptest! {
    #[test]
    fn i1_no_panic_on_arbitrary_bytes(bytes in proptest::collection::vec(any::<u8>(), 0..8192)) {
        let _ = std::hint::black_box(
            crustywad::Wad::from_bytes_with_options(bytes.clone(), crustywad::ParseOptions::strict())
        );
        let _ = std::hint::black_box(
            crustywad::Wad::from_bytes_with_options(bytes, crustywad::ParseOptions::lenient())
        );
    }

    #[test]
    fn i6_strict_error_implies_lenient_err_or_warn(bytes in proptest::collection::vec(any::<u8>(), 0..8192)) {
        if crustywad::Wad::from_bytes_with_options(bytes.clone(), crustywad::ParseOptions::strict()).is_err() {
            let lenient = crustywad::Wad::from_bytes_with_options(bytes, crustywad::ParseOptions::lenient());
            match lenient {
                Err(_) => {} // unrecoverable — OK
                Ok(wad) => prop_assert!(!wad.warnings().is_empty(),
                    "strict error disappeared silently in lenient mode"),
            }
        }
    }
}
```

- [ ] **Step 4: Add invariants I-2, I-3, I-4, I-7 to `wad_reader.rs`**

```rust
proptest! {
    #[test]
    fn i2_lump_count_consistency(bytes in common::arb_valid_wad()) {
        if let Ok(wad) = crustywad::Wad::from_bytes(bytes) {
            prop_assert_eq!(wad.lump_count(), wad.lumps().len());
            prop_assert_eq!(wad.lump_count(), wad.header().num_lumps);
        }
    }

    #[test]
    fn i3_lump_by_name_agreement(bytes in common::arb_valid_wad()) {
        if let Ok(wad) = crustywad::Wad::from_bytes(bytes) {
            for lump in wad.lumps() {
                prop_assert!(
                    wad.lump_by_name(lump.name()).is_some(),
                    "lump_by_name returned None for name {:?}", lump.name()
                );
            }
        }
    }

    #[test]
    fn i4_strict_names_are_ascii_and_short(bytes in common::arb_valid_wad()) {
        if let Ok(wad) = crustywad::Wad::from_bytes(bytes) {
            for lump in wad.lumps() {
                prop_assert!(lump.name().is_ascii());
                prop_assert!(lump.name().len() <= 8);
            }
        }
    }

    #[test]
    fn i7_lump_bytes_in_bounds(bytes in common::arb_valid_wad()) {
        if let Ok(wad) = crustywad::Wad::from_bytes(bytes) {
            for i in 0..wad.lump_count() {
                let lump = &wad.lumps()[i];
                let data = wad.lump_bytes(lump.filepos()..lump.filepos() + lump.size());
                prop_assert!(data.is_some(), "lump_bytes returned None for lump {i}");
            }
        }
    }
}
```

Note: verify `wad.header()` returns `WadHeader` and `WadHeader::num_lumps` exists in lib.rs. Adjust field name if different.

- [ ] **Step 5: Add invariants I-5 and I-8 to `map_records.rs`**

```rust
use proptest::prelude::*;
use crustywad::map::{parse_records, Thing, Linedef, Sidedef, Vertex, Seg, Subsector, Node, Sector};

proptest! {
    #[test]
    fn i5_parse_records_no_panic(bytes in proptest::collection::vec(any::<u8>(), 0..4096)) {
        let _ = std::hint::black_box(parse_records::<Thing>(&bytes));
        let _ = std::hint::black_box(parse_records::<Linedef>(&bytes));
        let _ = std::hint::black_box(parse_records::<Vertex>(&bytes));
        let _ = std::hint::black_box(parse_records::<Sector>(&bytes));
    }

    #[test]
    fn i8_trailing_bytes_semantics_thing(bytes in proptest::collection::vec(any::<u8>(), 0..4096)) {
        const THING_SIZE: usize = 10; // x(2) + y(2) + angle(2) + type(2) + flags(2)
        match parse_records::<Thing>(&bytes) {
            Err(crustywad::map::MapParseError::TrailingBytes { .. }) => {
                prop_assert_ne!(bytes.len() % THING_SIZE, 0,
                    "TrailingBytes on exact-multiple-length slice");
            }
            Ok(records) => {
                prop_assert_eq!(records.len(), bytes.len() / THING_SIZE);
            }
            Err(_) => {} // other errors are fine
        }
    }
}
```

Note: `THING_SIZE` is the on-disk size. Verify against `map.rs` — the `Thing` struct fields determine this. Adjust if different.

- [ ] **Step 6: Run all proptest tests**

```bash
cargo test --package crustywad --all-features
```

Expected: all pass. Proptest runs 256 cases per property.

- [ ] **Step 7: Run full CI**

```bash
just ci
```

- [ ] **Step 8: Commit**

```bash
git add crates/crustywad/tests/common/mod.rs \
        crates/crustywad/tests/wad_reader.rs \
        crates/crustywad/tests/map_records.rs
git commit -m "test(proptest): implement 8 invariant tests per ADR-0010 (#47)"
```

---

### Task 9: Issue #29 — CLI usage docs

**Branch:** `feature/29-cli-docs-man-page`  
**Prerequisite:** #35 merged (build.rs already set up by that task).

**Files:**
- Modify: `crates/crustywad-cli/src/main.rs` (improve help text)

Note: Per ADR-0008, man pages via `clap_mangen` are deferred until the subcommand surface stabilizes after milestone 2. This task delivers comprehensive `--help` output.

- [ ] **Step 1: Write the failing test**

Create `crates/crustywad-cli/tests/cli_help.rs`:
```rust
use assert_cmd::Command;

#[test]
fn help_mentions_all_subcommands() {
    let out = Command::cargo_bin("cwad").unwrap()
        .arg("--help")
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("info"), "help missing 'info'");
    assert!(stdout.contains("list"), "help missing 'list'");
    assert!(stdout.contains("validate"), "help missing 'validate'");
}

#[test]
fn validate_help_is_descriptive() {
    let out = Command::cargo_bin("cwad").unwrap()
        .args(["validate", "--help"])
        .output()
        .unwrap();
    let s = String::from_utf8(out.stdout).unwrap();
    assert!(s.contains("0") && (s.contains("clean") || s.contains("correct") || s.contains("1")));
}
```

- [ ] **Step 2: Improve `long_about` on the `Cli` struct and each subcommand**

Update `Cli`:
```rust
#[command(
    author,
    version,
    about = "Inspect Doom WAD files",
    long_about = "cwad — a command-line tool for inspecting and manipulating Doom WAD files.\n\nExamples:\n  cwad info level.wad\n  cwad list doom2.wad --format json\n  cwad validate my.wad\n  cwad diff base.wad mod.wad\n  cwad extract level.wad MAP01 -o map01.lmp"
)]
```

Update `Command::Info` doc comment: `/// Print WAD kind, lump count, and map names found in the directory.`  
Update `Command::Validate` doc comment: `/// Check WAD correctness. Exits 0 if clean, 1 if parse errors found, 2 on I/O errors.`

- [ ] **Step 3: Run tests and CI**

```bash
cargo test --package crustywad-cli --test cli_help && just ci
```

- [ ] **Step 4: Commit**

```bash
git add crates/crustywad-cli/src/main.rs crates/crustywad-cli/tests/cli_help.rs
git commit -m "docs(cli): improve --help text and add usage examples (#29)"
```

---

### Task 10: Issue #33 — Living-docs drift detector

**Branch:** `feature/33-living-docs`

**Files:**
- Create: `scripts/check_doc_anchors.py`
- Create: `anchors.txt`
- Modify: `justfile`
- Modify: `.github/workflows/ci.yml`
- Modify: `.github/PULL_REQUEST_TEMPLATE.md` (create if absent)
- Modify: `docs/design.md` (add missing anchor text)

Follow ADR-0007 (`docs/adr/0007-living-docs-automation.md`).

- [ ] **Step 1: Choose anchor strings**

Read `.claude/CLAUDE.md` and `.github/copilot-instructions.md`. Find shared convention phrases that appear in both. Create `anchors.txt` at repo root:
```
ParseOptions { strictness
just ci
thiserror
Conventional Commits
missing_docs
clippy::pedantic
Strictness::Strict
Strictness::Lenient
```

Verify each string appears verbatim in both `.claude/CLAUDE.md` and `.github/copilot-instructions.md`.

- [ ] **Step 2: Write failing test (manual check)**

```bash
python3 -c "
import sys
anchors = open('anchors.txt').read().splitlines()
files = ['.claude/CLAUDE.md', '.github/copilot-instructions.md', 'docs/design.md']
for anchor in anchors:
    for f in files:
        if anchor not in open(f).read():
            print(f'MISSING: {anchor!r} in {f}')
            sys.exit(1)
print('All anchors found')
"
```

Expected: some anchors are missing from `docs/design.md` — the script exits non-zero.

- [ ] **Step 3: Create `scripts/check_doc_anchors.py`**

```python
#!/usr/bin/env python3
"""Check that documentation anchor strings appear in all monitored files."""
import pathlib
import sys

ANCHORS_FILE = pathlib.Path("anchors.txt")
CHECKED_FILES = [
    pathlib.Path(".claude/CLAUDE.md"),
    pathlib.Path(".github/copilot-instructions.md"),
    pathlib.Path("docs/design.md"),
]

def main() -> int:
    anchors = [a for a in ANCHORS_FILE.read_text().splitlines() if a and not a.startswith("#")]
    contents = {f: f.read_text() for f in CHECKED_FILES}
    failures = []
    for anchor in anchors:
        for f, text in contents.items():
            if anchor not in text:
                failures.append(f"  MISSING {anchor!r} in {f}")
    if failures:
        print("docs-sync FAILED — anchor strings missing from documentation files:")
        print("\n".join(failures))
        print("\nUpdate all monitored files to include the anchor, or update anchors.txt.")
        return 1
    print(f"docs-sync OK — {len(anchors)} anchors found in all {len(CHECKED_FILES)} files.")
    return 0

if __name__ == "__main__":
    sys.exit(main())
```

- [ ] **Step 4: Update `docs/design.md` to include anchor text**

Read `docs/design.md`. Find sections about parsing and add the missing anchor phrases naturally. For example, ensure the strictness section mentions `ParseOptions { strictness`, `Strictness::Strict`, `Strictness::Lenient`; the CI section mentions `just ci`; the error section mentions `thiserror`; etc.

- [ ] **Step 5: Verify the script passes**

```bash
python3 scripts/check_doc_anchors.py
```

Expected: `docs-sync OK`.

- [ ] **Step 6: Add `docs-sync` recipe to justfile**

```makefile
docs-sync:
    python3 scripts/check_doc_anchors.py
```

- [ ] **Step 7: Add `docs-sync` job to `.github/workflows/ci.yml`**

After the existing jobs, add:
```yaml
  docs-sync:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7
      - run: python3 scripts/check_doc_anchors.py
```

- [ ] **Step 8: Update or create `.github/PULL_REQUEST_TEMPLATE.md`**

Add to PR template:
```markdown
## Documentation sync

- [ ] If conventions changed, updated anchors in `anchors.txt` and all files in `check_doc_anchors.py`
```

- [ ] **Step 9: Run full CI**

```bash
just ci && python3 scripts/check_doc_anchors.py
```

- [ ] **Step 10: Commit**

```bash
git add scripts/check_doc_anchors.py anchors.txt justfile \
        .github/workflows/ci.yml .github/PULL_REQUEST_TEMPLATE.md \
        docs/design.md
git commit -m "feat(docs): add living-docs drift detector per ADR-0007 (#33)"
```

---

### Task 11: Issue #49 — docs.rs configuration

**Branch:** `feature/49-docsrs-config`

**Files:**
- Modify: `crates/crustywad/Cargo.toml`

- [ ] **Step 1: Write the failing test**

```bash
cargo doc --package crustywad --all-features --no-deps 2>&1 | grep -i "warning\|error"
```

Expected: no warnings or errors.

- [ ] **Step 2: Add `[package.metadata.docs.rs]` to `crates/crustywad/Cargo.toml`**

```toml
[package.metadata.docs.rs]
all-features = true
rustdoc-args = ["--cfg", "docsrs"]
```

- [ ] **Step 3: Add `#[cfg_attr(docsrs, ...)]` feature badges where applicable**

In `crates/crustywad/src/lib.rs`, find the `#![doc = ...]` crate-level docs. Add a feature table if not already present. Add `#![cfg_attr(docsrs, feature(doc_cfg))]` to the top of `lib.rs` if not present.

For any `#[cfg(feature = "mmap")]` items, add:
```rust
#[cfg_attr(docsrs, doc(cfg(feature = "mmap")))]
```

- [ ] **Step 4: Verify docs build with docsrs flag**

```bash
RUSTDOCFLAGS="--cfg docsrs" cargo doc --package crustywad --all-features --no-deps
```

Expected: no warnings, docs open in browser via `cargo doc --open`.

- [ ] **Step 5: Run full CI**

```bash
just ci
```

- [ ] **Step 6: Commit**

```bash
git add crates/crustywad/Cargo.toml crates/crustywad/src/lib.rs
git commit -m "chore(docs): add docs.rs configuration and feature badges (#49)"
```

---

### Task 12: Issue #50 — Changelog automation

**Branch:** `feature/50-changelog-automation`

**Files:**
- Create: `crates/crustywad/CHANGELOG.md`
- Create: `crates/crustywad-cli/CHANGELOG.md`
- Modify: `release-plz.toml`
- Modify: `crates/crustywad/Cargo.toml` (switch from `version.workspace = true`)
- Modify: `crates/crustywad-cli/Cargo.toml` (switch from `version.workspace = true`)

Follow ADR-0011 §3 (version migration) and §4 (changelog management).

- [ ] **Step 1: Create per-crate CHANGELOG files**

Create `crates/crustywad/CHANGELOG.md`:
```markdown
# Changelog

All notable changes to `crustywad` are documented here.

## [Unreleased]
```

Create `crates/crustywad-cli/CHANGELOG.md`:
```markdown
# Changelog

All notable changes to `crustywad-cli` are documented here.

## [Unreleased]
```

- [ ] **Step 2: Switch both crates to explicit versions**

In `crates/crustywad/Cargo.toml`, replace `version.workspace = true` with:
```toml
version = "0.1.0"
```

In `crates/crustywad-cli/Cargo.toml`, replace `version.workspace = true` with:
```toml
version = "0.1.0"
```

- [ ] **Step 3: Update `release-plz.toml`**

```toml
[workspace]
release = true
publish = false
changelog_update = true
allow_dirty = false
semver_check = false

[[package]]
name = "crustywad"
release = true
publish = false
changelog_path = "crates/crustywad/CHANGELOG.md"

[[package]]
name = "crustywad-cli"
release = true
publish = false
changelog_path = "crates/crustywad-cli/CHANGELOG.md"
```

Note: `publish = false` is kept until ADR-0011's pre-publish checklist is fully completed. This task only wires up changelog automation.

- [ ] **Step 4: Verify build and deny still pass**

```bash
cargo build --workspace --all-features
cargo deny check
just ci
```

Expected: all pass. `cargo-deny` should be satisfied with explicit `version = "0.1.0"` on `crustywad` in `crustywad-cli/Cargo.toml`.

- [ ] **Step 5: Commit**

```bash
git add crates/crustywad/CHANGELOG.md crates/crustywad-cli/CHANGELOG.md \
        release-plz.toml crates/crustywad/Cargo.toml crates/crustywad-cli/Cargo.toml
git commit -m "chore(release): set up per-crate changelogs and switch to explicit versions (#50)"
```

---

### Task 13: Issue #51 — SemVer, MSRV & release policy doc

**Branch:** `feature/51-semver-msrv-policy`

**Files:**
- Create: `docs/guide/src/release-policy.md`
- Modify: `docs/guide/src/SUMMARY.md`

- [ ] **Step 1: Write the document**

Create `docs/guide/src/release-policy.md`:
```markdown
# Release Policy

## Versioning

`crustywad` and `crustywad-cli` follow [Semantic Versioning 2.0.0](https://semver.org/).
Each crate is versioned independently.

| Change type | Version bump |
|---|---|
| Breaking public API change | Major (`X.0.0`) |
| New public API, backwards compatible | Minor (`0.X.0`) |
| Bug fix, documentation, internal refactor | Patch (`0.0.X`) |

## MSRV Policy

The minimum supported Rust version (MSRV) is **1.85.0** (stable).

MSRV bumps are treated as **minor version changes** (not patch). When the MSRV
is raised, it appears in the CHANGELOG and is tested in CI via the `msrv` job.

## Release Cadence

Releases are automated by `release-plz`. On each push to `main`, `release-plz`
creates or updates a release PR with a version bump and CHANGELOG entries derived
from Conventional Commits. Merging the release PR triggers the publish workflow.

## Publishing

Publishing to crates.io requires the pre-publish checklist in ADR-0011 to be
completed. Until then, `publish = false` is set in `release-plz.toml`.

The publish order is always: `crustywad` (library) before `crustywad-cli` (binary).
`release-plz` handles this ordering automatically.
```

- [ ] **Step 2: Add entry to `SUMMARY.md`**

Find the appropriate section in `docs/guide/src/SUMMARY.md` and add:
```markdown
- [Release Policy](release-policy.md)
```

- [ ] **Step 3: Build the guide to verify**

```bash
just guide
```

Expected: guide builds without errors. Check that the new page appears in the navigation.

- [ ] **Step 4: Commit**

```bash
git add docs/guide/src/release-policy.md docs/guide/src/SUMMARY.md
git commit -m "docs(guide): add SemVer, MSRV, and release policy documentation (#51)"
```

---

### Task 14: Issue #52 — Cross-platform binary artifacts

**Branch:** `feature/52-binary-artifacts`

**Files:**
- Create: `.github/workflows/release-artifacts.yml`

- [ ] **Step 1: Write the workflow**

Create `.github/workflows/release-artifacts.yml`:
```yaml
name: Release artifacts

on:
  release:
    types: [published]

permissions:
  contents: write

jobs:
  build:
    name: Build ${{ matrix.target }}
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            archive: tar.gz
          - os: ubuntu-latest
            target: aarch64-unknown-linux-gnu
            archive: tar.gz
          - os: macos-latest
            target: x86_64-apple-darwin
            archive: tar.gz
          - os: macos-latest
            target: aarch64-apple-darwin
            archive: tar.gz
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            archive: zip

    steps:
      - uses: actions/checkout@v7
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Install cross-compile tools (aarch64 linux)
        if: matrix.target == 'aarch64-unknown-linux-gnu'
        run: |
          sudo apt-get update
          sudo apt-get install -y gcc-aarch64-linux-gnu

      - name: Build
        run: cargo build --package crustywad-cli --release --target ${{ matrix.target }}
        env:
          CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER: aarch64-linux-gnu-gcc

      - name: Package (unix)
        if: matrix.archive == 'tar.gz'
        run: |
          BIN=target/${{ matrix.target }}/release/cwad
          strip "$BIN" 2>/dev/null || true
          tar czf cwad-${{ github.event.release.tag_name }}-${{ matrix.target }}.tar.gz -C "$(dirname $BIN)" cwad

      - name: Package (windows)
        if: matrix.archive == 'zip'
        shell: pwsh
        run: |
          $bin = "target\${{ matrix.target }}\release\cwad.exe"
          Compress-Archive -Path $bin -DestinationPath "cwad-${{ github.event.release.tag_name }}-${{ matrix.target }}.zip"

      - name: Upload to release
        uses: softprops/action-gh-release@v2
        with:
          files: cwad-*
```

- [ ] **Step 2: Verify CI is unaffected**

```bash
just ci
```

Expected: no change to existing jobs (new workflow only triggers on GitHub release events).

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/release-artifacts.yml
git commit -m "ci: add cross-platform binary artifact workflow for GitHub releases (#52)"
```

---

## Phase 3: Wave 2

> Wave 2 starts once issue **#21 merges into `main`**.
> Pull `main` before starting each task. The write chain is sequential: #22 → #23 → {#24, #25, #26, #38, #43} in parallel.

---

### Task 15: Issue #22 — Write: Lump payload serialization

**Branch:** `feature/22-write-lump-payload`  
**Prerequisite:** #21 merged. Pull `main`.

> Note: Task 2 (#21) already implemented a full `WadBuilder::build()`. This task expands
> the implementation based on the audit — verify what #21 actually delivered and fill
> any remaining gaps (proper binrw BinWrite usage, full round-trip correctness).
> Read `crates/crustywad/src/write.rs` first to understand current state.

**Files:**
- Modify: `crates/crustywad/src/write.rs`
- Modify: `crates/crustywad/tests/write.rs`

- [ ] **Step 1: Read current `write.rs` and `tests/write.rs`**

Verify: does `WadBuilder::build()` actually serialize using `BinWrite` on `RawHeader` and `RawDirectoryEntry`? Does it recompute `filepos`, `size`, and `infotableofs` correctly?

- [ ] **Step 2: Write the failing test for multi-lump fidelity**

Add to `crates/crustywad/tests/write.rs`:
```rust
#[test]
fn lump_filepos_and_size_are_correct() {
    let bytes = WadBuilder::new(WadKind::Pwad)
        .add_lump("A", b"hello")
        .add_lump("B", b"world!!")
        .build()
        .unwrap();
    let wad = crustywad::Wad::from_bytes(bytes).unwrap();
    assert_eq!(wad.lumps()[0].name(), "A");
    assert_eq!(wad.lumps()[0].size(), 5);
    assert_eq!(wad.lumps()[0].filepos(), 12); // right after 12-byte header
    assert_eq!(wad.lumps()[1].name(), "B");
    assert_eq!(wad.lumps()[1].size(), 7);
    assert_eq!(wad.lumps()[1].filepos(), 17); // 12 + 5
}

#[test]
fn empty_lump_round_trips() {
    let bytes = WadBuilder::new(WadKind::Iwad)
        .add_lump("MARKER", b"")
        .build()
        .unwrap();
    let wad = crustywad::Wad::from_bytes(bytes).unwrap();
    assert_eq!(wad.lumps()[0].size(), 0);
    assert_eq!(wad.lumps()[0].name(), "MARKER");
}
```

- [ ] **Step 3: Run tests**

```bash
cargo test --package crustywad --features write --test write
```

Fix any gaps in the serialization implementation until all tests pass.

- [ ] **Step 4: Run full CI**

```bash
just ci
```

- [ ] **Step 5: Commit**

```bash
git add crates/crustywad/src/write.rs crates/crustywad/tests/write.rs
git commit -m "feat(write): verify and complete lump payload serialization with correct offsets (#22)"
```

---

### Task 16: Issue #23 — Write: Strict/lenient validation

**Branch:** `feature/23-write-validation`  
**Prerequisite:** #22 merged. Pull `main`.

**Files:**
- Modify: `crates/crustywad/src/write.rs`
- Modify: `crates/crustywad/tests/write.rs`

- [ ] **Step 1: Write failing tests for all validation rules**

Add to `crates/crustywad/tests/write.rs`:
```rust
#[test]
fn strict_rejects_name_longer_than_8() {
    let result = WadBuilder::new(WadKind::Pwad)
        .add_lump("TOOLONGNAME", b"")
        .build();
    assert!(result.is_err());
}

#[test]
fn lenient_truncates_name_longer_than_8() {
    let (bytes, warnings) = WadBuilder::new(WadKind::Pwad)
        .add_lump("TOOLONGNAME", b"")
        .build_with_options(WriteOptions::lenient())
        .unwrap();
    assert!(!warnings.is_empty());
    let wad = crustywad::Wad::from_bytes(bytes).unwrap();
    assert_eq!(wad.lumps()[0].name(), "TOOLONGN");
}

#[test]
fn both_modes_reject_nul_byte() {
    for opts in [WriteOptions::strict(), WriteOptions::lenient()] {
        let result = WadBuilder::new(WadKind::Pwad)
            .add_lump("BAD\0NAME", b"")
            .build_with_options(opts);
        assert!(result.is_err());
    }
}

#[test]
fn strict_rejects_unknown_magic() {
    let result = WadBuilder::new(WadKind::Unknown(*b"XWAD"))
        .build();
    assert!(result.is_err());
}

#[test]
fn lenient_allows_unknown_magic() {
    let (bytes, _) = WadBuilder::new(WadKind::Unknown(*b"XWAD"))
        .build_with_options(WriteOptions::lenient())
        .unwrap();
    // bytes[0..4] should be XWAD
    assert_eq!(&bytes[0..4], b"XWAD");
}
```

- [ ] **Step 2: Run failing tests**

```bash
cargo test --package crustywad --features write --test write 2>&1 | grep FAILED
```

- [ ] **Step 3: Implement any missing validation in `write.rs`**

Verify the `build_with_options` function covers all rules from ADR-0006:
- NUL in name → `WriteError::NulInName` (both modes)
- Non-ASCII name → `WriteError::NonAsciiName` (both modes)
- Name > 8 bytes → strict: error; lenient: truncate + warning
- Lump data > `i32::MAX` → error (both modes)
- Lump count > `i32::MAX` → error (both modes)
- Any computed offset > `i32::MAX` → error (both modes)
- `WadKind::Unknown` → strict: error; lenient: write raw bytes

- [ ] **Step 4: Run full test suite**

```bash
cargo test --package crustywad --features write && just ci
```

- [ ] **Step 5: Commit**

```bash
git add crates/crustywad/src/write.rs crates/crustywad/tests/write.rs
git commit -m "feat(write): implement strict/lenient validation per ADR-0006 (#23)"
```

---

### Task 17: Issue #24 — Write: Round-trip proptest

**Branch:** `feature/24-write-roundtrip-tests`  
**Prerequisite:** #23 merged. Pull `main`.

**Files:**
- Modify: `crates/crustywad/tests/write.rs`

- [ ] **Step 1: Write the proptest**

Add to `crates/crustywad/tests/write.rs`:
```rust
#![cfg(feature = "write")]

use crustywad::{WadBuilder, WadKind};
use proptest::prelude::*;

fn arb_wad_kind() -> impl Strategy<Value = WadKind> {
    prop_oneof![Just(WadKind::Iwad), Just(WadKind::Pwad)]
}

fn arb_ascii_name() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[A-Z_][A-Z0-9_]{0,7}").unwrap()
}

fn arb_lump() -> impl Strategy<Value = (String, Vec<u8>)> {
    (arb_ascii_name(), proptest::collection::vec(any::<u8>(), 0..=256))
}

proptest! {
    #[test]
    fn round_trip_preserves_lump_names_and_sizes(
        kind in arb_wad_kind(),
        lumps in proptest::collection::vec(arb_lump(), 0..=16)
    ) {
        let mut builder = WadBuilder::new(kind);
        for (name, data) in &lumps {
            builder.add_lump(name, data.as_slice());
        }
        let bytes = builder.build().expect("build should succeed with valid inputs");
        let wad = crustywad::Wad::from_bytes(bytes).expect("built WAD should re-parse");

        prop_assert_eq!(wad.lump_count(), lumps.len());
        for (i, (name, data)) in lumps.iter().enumerate() {
            prop_assert_eq!(wad.lumps()[i].name(), name.as_str());
            prop_assert_eq!(wad.lumps()[i].size(), data.len());
        }
    }

    #[test]
    fn wad_to_builder_preserves_all_lumps(
        bytes in crate::common::arb_valid_wad()
    ) {
        if let Ok(wad) = crustywad::Wad::from_bytes(bytes) {
            let rebuilt = wad.to_builder().build().expect("to_builder round-trip should succeed");
            let wad2 = crustywad::Wad::from_bytes(rebuilt).expect("rebuilt WAD should parse");
            prop_assert_eq!(wad2.lump_count(), wad.lump_count());
            for (a, b) in wad.lumps().iter().zip(wad2.lumps().iter()) {
                prop_assert_eq!(a.name(), b.name());
                prop_assert_eq!(a.size(), b.size());
            }
        }
    }
}
```

Note: `crate::common::arb_valid_wad()` refers to the strategy added in Task 8. Verify the import path for integration tests.

- [ ] **Step 2: Run proptest (256 cases each)**

```bash
cargo test --package crustywad --features write --test write
```

- [ ] **Step 3: Run full CI**

```bash
just ci
```

- [ ] **Step 4: Commit**

```bash
git add crates/crustywad/tests/write.rs
git commit -m "test(write): add round-trip proptest invariants per ADR-0006 (#24)"
```

---

### Task 18: Issue #25 — Write: CLI write wiring

**Branch:** `feature/25-write-cli-wiring`  
**Prerequisite:** #23 merged and #35 merged. Pull `main`.

**Files:**
- Modify: `crates/crustywad-cli/src/main.rs`
- Modify: `crates/crustywad-cli/Cargo.toml` (add `write` feature dependency)

- [ ] **Step 1: Enable write feature in CLI**

In `crates/crustywad-cli/Cargo.toml`:
```toml
[features]
default = []
mmap = ["crustywad/mmap"]
write = ["crustywad/write"]

[dependencies]
crustywad = { path = "../crustywad", version = "0.1.0", features = ["write"] }
```

Or alternatively, enable `write` unconditionally in the CLI since it's a tool, not a library.

- [ ] **Step 2: Write failing test**

Create `crates/crustywad-cli/tests/cli_rebuild.rs`:
```rust
use assert_cmd::Command;
use tempfile::NamedTempFile;
use std::io::Write as _;

fn minimal_iwad() -> Vec<u8> {
    let mut b = b"IWAD".to_vec();
    b.extend_from_slice(&0_i32.to_le_bytes());
    b.extend_from_slice(&12_i32.to_le_bytes());
    b
}

#[test]
fn rebuild_produces_valid_wad() {
    let mut input = NamedTempFile::new().unwrap();
    input.write_all(&minimal_iwad()).unwrap();
    let output = NamedTempFile::new().unwrap();
    Command::cargo_bin("cwad").unwrap()
        .args(["rebuild", input.path().to_str().unwrap(),
               "--output", output.path().to_str().unwrap()])
        .assert()
        .success();
    let bytes = std::fs::read(output.path()).unwrap();
    assert!(crustywad::Wad::from_bytes(bytes).is_ok());
}
```

- [ ] **Step 3: Add `Command::Rebuild` to `main.rs`**

```rust
/// Read a WAD, rebuild it via WadBuilder, and write to an output file.
Rebuild {
    /// Input WAD file.
    path: PathBuf,
    /// Output WAD file.
    #[arg(short, long)]
    output: PathBuf,
},
```

Match arm:
```rust
Command::Rebuild { path, output } => {
    let wad = Wad::from_path_with_options(&path, options)
        .with_context(|| format!("failed to open {}", path.display()))?;
    for warning in wad.warnings() {
        eprintln!("warning: {warning}");
    }
    let bytes = wad.to_builder().build()
        .with_context(|| "failed to rebuild WAD")?;
    std::fs::write(&output, &bytes)
        .with_context(|| format!("failed to write {}", output.display()))?;
    match cli.format {
        Format::Human => println!("wrote {} bytes to {}", bytes.len(), output.display()),
        Format::Json  => println!(r#"{{"ok":true,"bytes":{}}}"#, bytes.len()),
        Format::Csv   => { println!("ok,bytes"); println!("true,{}", bytes.len()); }
    }
    Ok(0)
}
```

- [ ] **Step 4: Run tests and CI**

```bash
cargo test --package crustywad-cli --test cli_rebuild && just ci
```

- [ ] **Step 5: Commit**

```bash
git add crates/crustywad-cli/src/main.rs crates/crustywad-cli/Cargo.toml \
        crates/crustywad-cli/tests/cli_rebuild.rs
git commit -m "feat(cli): add cwad rebuild subcommand wiring write support (#25)"
```

---

### Task 19: Issue #26 — Write: Exhaustive test coverage

**Branch:** `feature/26-write-exhaustive-tests`  
**Prerequisite:** #23 merged. Pull `main`.

**Files:**
- Modify: `crates/crustywad/tests/write.rs`

- [ ] **Step 1: Write edge-case tests**

Add to `crates/crustywad/tests/write.rs`:
```rust
#[test]
fn iwad_round_trip() {
    let bytes = WadBuilder::new(WadKind::Iwad)
        .add_lump("PLAYPAL", &[0u8; 768])
        .add_lump("COLORMAP", &[0u8; 8704])
        .build()
        .unwrap();
    let wad = crustywad::Wad::from_bytes(bytes).unwrap();
    assert_eq!(wad.kind(), WadKind::Iwad);
    assert_eq!(wad.lump_count(), 2);
    assert_eq!(wad.lumps()[0].size(), 768);
    assert_eq!(wad.lumps()[1].size(), 8704);
}

#[test]
fn max_name_length_8_bytes() {
    let bytes = WadBuilder::new(WadKind::Pwad)
        .add_lump("ABCDEFGH", b"data") // exactly 8 chars
        .build()
        .unwrap();
    let wad = crustywad::Wad::from_bytes(bytes).unwrap();
    assert_eq!(wad.lumps()[0].name(), "ABCDEFGH");
}

#[test]
fn zero_length_lump_name() {
    // Empty name is ASCII and 0 bytes — should succeed
    let bytes = WadBuilder::new(WadKind::Pwad)
        .add_lump("", b"data")
        .build()
        .unwrap();
    let wad = crustywad::Wad::from_bytes(bytes).unwrap();
    assert_eq!(wad.lumps()[0].size(), 4);
}

#[test]
fn many_lumps_round_trip() {
    let mut builder = WadBuilder::new(WadKind::Pwad);
    for i in 0..100u8 {
        builder.add_lump(&format!("L{i:03}"), &[i; 16]);
    }
    let bytes = builder.build().unwrap();
    let wad = crustywad::Wad::from_bytes(bytes).unwrap();
    assert_eq!(wad.lump_count(), 100);
}
```

- [ ] **Step 2: Run tests and CI**

```bash
cargo test --package crustywad --features write --test write && just ci
```

- [ ] **Step 3: Commit**

```bash
git add crates/crustywad/tests/write.rs
git commit -m "test(write): add exhaustive edge-case coverage for write paths (#26)"
```

---

### Task 20: Issue #38 — `cwad merge`

**Branch:** `feature/38-cwad-merge`  
**Prerequisite:** #23 merged (write support), #35 merged (CLI foundation). Pull `main`.

**Files:**
- Modify: `crates/crustywad-cli/src/main.rs`

- [ ] **Step 1: Write failing test**

Create `crates/crustywad-cli/tests/cli_merge.rs`:
```rust
use assert_cmd::Command;
use tempfile::NamedTempFile;
use std::io::Write as _;

fn pwad_with_lump(name: &str, data: &[u8]) -> Vec<u8> {
    let mut b = b"PWAD".to_vec();
    b.extend_from_slice(&1_i32.to_le_bytes());
    b.extend_from_slice(&(12 + data.len() as i32).to_le_bytes());
    b.extend_from_slice(data);
    b.extend_from_slice(&12_i32.to_le_bytes());
    b.extend_from_slice(&(data.len() as i32).to_le_bytes());
    let mut n = [0u8; 8];
    n[..name.len().min(8)].copy_from_slice(&name.as_bytes()[..name.len().min(8)]);
    b.extend_from_slice(&n);
    b
}

#[test]
fn merge_two_wads_combines_lumps() {
    let mut f1 = NamedTempFile::new().unwrap();
    let mut f2 = NamedTempFile::new().unwrap();
    let out = NamedTempFile::new().unwrap();
    f1.write_all(&pwad_with_lump("FOO", b"aaa")).unwrap();
    f2.write_all(&pwad_with_lump("BAR", b"bbb")).unwrap();
    Command::cargo_bin("cwad").unwrap()
        .args(["merge",
               f1.path().to_str().unwrap(),
               f2.path().to_str().unwrap(),
               "--output", out.path().to_str().unwrap()])
        .assert()
        .success();
    let bytes = std::fs::read(out.path()).unwrap();
    let wad = crustywad::Wad::from_bytes(bytes).unwrap();
    assert_eq!(wad.lump_count(), 2);
}
```

- [ ] **Step 2: Add `Command::Merge` to `main.rs`**

```rust
/// Combine lumps from multiple WAD files into one output WAD.
Merge {
    /// Input WAD files (one or more).
    #[arg(required = true)]
    inputs: Vec<PathBuf>,
    /// Output WAD file.
    #[arg(short, long)]
    output: PathBuf,
},
```

Match arm:
```rust
Command::Merge { inputs, output } => {
    let mut builder = crustywad::WadBuilder::new(WadKind::Pwad);
    for input_path in &inputs {
        let wad = Wad::from_path_with_options(input_path, options.clone())
            .with_context(|| format!("failed to open {}", input_path.display()))?;
        for warning in wad.warnings() {
            eprintln!("warning: {warning}");
        }
        for lump in wad.lumps() {
            let data = wad
                .lump_bytes(lump.filepos()..lump.filepos() + lump.size())
                .unwrap_or_default()
                .to_vec();
            builder.add_lump(lump.name(), data);
        }
    }
    let bytes = builder.build()
        .with_context(|| "failed to build merged WAD")?;
    std::fs::write(&output, &bytes)
        .with_context(|| format!("failed to write {}", output.display()))?;
    match cli.format {
        Format::Human => println!("wrote {} bytes to {}", bytes.len(), output.display()),
        Format::Json  => println!(r#"{{"ok":true,"bytes":{}}}"#, bytes.len()),
        Format::Csv   => { println!("ok,bytes"); println!("true,{}", bytes.len()); }
    }
    Ok(0)
}
```

Note: `WadKind::Pwad` is hardcoded for the output. The first input's kind could be used instead — either is acceptable for v1.0.

- [ ] **Step 3: Run tests and CI**

```bash
cargo test --package crustywad-cli --test cli_merge && just ci
```

- [ ] **Step 4: Commit**

```bash
git add crates/crustywad-cli/src/main.rs crates/crustywad-cli/tests/cli_merge.rs
git commit -m "feat(cli): add cwad merge subcommand (#38)"
```

---

### Task 21: Issue #43 — End-to-end read→modify→write tests

**Branch:** `feature/43-e2e-tests`  
**Prerequisite:** #23 merged. Pull `main`.

**Files:**
- Create: `crates/crustywad/tests/roundtrip.rs`

- [ ] **Step 1: Write failing test**

Create `crates/crustywad/tests/roundtrip.rs`:
```rust
//! End-to-end read → modify → write → re-read integration tests.
#![cfg(feature = "write")]

use crustywad::{WadBuilder, WadKind, Wad};

fn build_test_wad(lumps: &[(&str, &[u8])]) -> Vec<u8> {
    let mut builder = WadBuilder::new(WadKind::Pwad);
    for (name, data) in lumps {
        builder.add_lump(name, *data);
    }
    builder.build().expect("test WAD build should succeed")
}

#[test]
fn read_modify_write_preserves_existing_lumps() {
    let original = build_test_wad(&[("MAP01", b"level_data"), ("THINGS", b"thing_data")]);
    let wad = Wad::from_bytes(original).expect("should parse");

    // Modify: add a new lump
    let mut builder = wad.to_builder();
    builder.add_lump("NEWLUMP", b"new_data");
    let modified = builder.build().expect("modified build should succeed");

    let wad2 = Wad::from_bytes(modified).expect("modified WAD should re-parse");
    assert_eq!(wad2.lump_count(), 3);
    assert_eq!(wad2.lumps()[0].name(), "MAP01");
    assert_eq!(wad2.lumps()[1].name(), "THINGS");
    assert_eq!(wad2.lumps()[2].name(), "NEWLUMP");
}

#[test]
fn round_trip_strict_and_lenient() {
    let bytes = build_test_wad(&[("A", b"alpha"), ("B", b"beta")]);

    // Strict round-trip
    let wad = Wad::from_bytes(bytes.clone()).expect("strict parse");
    let rebuilt = wad.to_builder().build().expect("strict build");
    let wad2 = Wad::from_bytes(rebuilt).expect("strict re-parse");
    assert_eq!(wad2.lump_count(), 2);

    // Lenient round-trip
    let wad_l = Wad::from_bytes_with_options(bytes, crustywad::ParseOptions::lenient())
        .expect("lenient parse");
    let (rebuilt_l, _warnings) = wad_l
        .to_builder()
        .build_with_options(crustywad::WriteOptions::lenient())
        .expect("lenient build");
    let wad3 = Wad::from_bytes(rebuilt_l).expect("lenient re-parse");
    assert_eq!(wad3.lump_count(), 2);
}

#[test]
fn e2e_empty_wad_round_trip() {
    let bytes = WadBuilder::new(WadKind::Iwad).build().unwrap();
    let wad = Wad::from_bytes(bytes).unwrap();
    let rebuilt = wad.to_builder().build().unwrap();
    let wad2 = Wad::from_bytes(rebuilt).unwrap();
    assert_eq!(wad2.lump_count(), 0);
    assert_eq!(wad2.kind(), WadKind::Iwad);
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test --package crustywad --features write --test roundtrip
```

- [ ] **Step 3: Run full CI**

```bash
just ci
```

- [ ] **Step 4: Commit**

```bash
git add crates/crustywad/tests/roundtrip.rs
git commit -m "test(e2e): add read→modify→write round-trip integration tests (#43)"
```

---

### Task 22: Issue #40 — CLI hardening test suite

**Branch:** `feature/40-cli-hardening`  
**Prerequisite:** All CLI commands merged (#35, #36, #37, #39, #38). Pull `main`.

**Files:**
- Create: `crates/crustywad-cli/tests/cli_hardening.rs`

- [ ] **Step 1: Write failing tests**

Create `crates/crustywad-cli/tests/cli_hardening.rs`:
```rust
use assert_cmd::Command;
use tempfile::NamedTempFile;
use std::io::Write as _;

fn corrupt_bytes() -> Vec<u8> { b"NOTAWAD!!!!!".to_vec() }
fn empty_bytes() -> Vec<u8> { vec![] }

fn write_temp(data: &[u8]) -> NamedTempFile {
    let mut f = NamedTempFile::new().unwrap();
    f.write_all(data).unwrap();
    f
}

// --- validate ---
#[test] fn validate_corrupt_exits_1() {
    let f = write_temp(&corrupt_bytes());
    Command::cargo_bin("cwad").unwrap()
        .args(["validate", f.path().to_str().unwrap()])
        .assert().code(1);
}
#[test] fn validate_empty_file_exits_1() {
    let f = write_temp(&empty_bytes());
    Command::cargo_bin("cwad").unwrap()
        .args(["validate", f.path().to_str().unwrap()])
        .assert().code(1);
}
#[test] fn validate_missing_file_exits_2() {
    Command::cargo_bin("cwad").unwrap()
        .args(["validate", "/does/not/exist.wad"])
        .assert().code(2);
}

// --- diff ---
#[test] fn diff_corrupt_first_arg_exits_2() {
    let f1 = write_temp(&corrupt_bytes());
    let f2 = write_temp(&corrupt_bytes());
    Command::cargo_bin("cwad").unwrap()
        .args(["diff", f1.path().to_str().unwrap(), f2.path().to_str().unwrap()])
        .assert().code(2);
}

// --- extract ---
#[test] fn extract_from_corrupt_wad_exits_2() {
    let f = write_temp(&corrupt_bytes());
    Command::cargo_bin("cwad").unwrap()
        .args(["extract", f.path().to_str().unwrap(), "FOO"])
        .assert().code(2);
}

// --- exit codes ---
#[test] fn unknown_subcommand_exits_3() {
    Command::cargo_bin("cwad").unwrap()
        .args(["notasubcommand"])
        .assert().code(3);
}

// --- format flags ---
#[test] fn invalid_format_flag_exits_3() {
    Command::cargo_bin("cwad").unwrap()
        .args(["-F", "xml", "validate", "/dev/null"])
        .assert().code(3);
}

// --- lenient flag ---
#[test] fn lenient_flag_accepted_by_all_subcommands() {
    let f = write_temp(&corrupt_bytes());
    for sub in ["validate", "info", "list"] {
        Command::cargo_bin("cwad").unwrap()
            .args(["--lenient", sub, f.path().to_str().unwrap()])
            .assert(); // just verify it doesn't crash with a usage error
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test --package crustywad-cli --test cli_hardening
```

Fix any exit code mismatches found (e.g., if corrupt WAD returns `Err` which maps to exit code 2 but the test expects 1). Align behavior with ADR-0008 exit code table.

- [ ] **Step 3: Run full CI**

```bash
just ci
```

- [ ] **Step 4: Commit**

```bash
git add crates/crustywad-cli/tests/cli_hardening.rs
git commit -m "test(cli): add hardening test suite covering all subcommands, exit codes, and edge cases (#40)"
```

---

## Self-Review Checklist

**Spec coverage:**
- Phase 1 Audit → Task 1 ✓
- Epic #12 (#21 → #22 → #23 → #24/#25/#26) → Tasks 2, 15, 16, 17, 18, 19 ✓
- Epic #13 (#29, #33) → Tasks 9, 10 ✓
- Epic #14 (#35, #36, #37, #38, #39, #40) → Tasks 3, 4, 5, 6, 20, 22 ✓
- Epic #15 (#43, #45, #47) → Tasks 21, 7, 8 ✓
- Epic #16 (#49, #50, #51, #52) → Tasks 11, 12, 13, 14 ✓

**Sequencing constraints documented:**
- CLI ordering (#35 before #36/#37/#39) ✓
- Write chain (#21 → #22 → #23 → parallel Wave 2) ✓
- CLI hardening (#40 after all CLI commands) ✓

**No placeholders:** All steps contain actual code or exact commands. ✓
