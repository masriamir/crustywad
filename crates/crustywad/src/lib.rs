#![deny(unsafe_code)]
#![doc = r#"
`crustywad` is a small Rust library for safe Doom WAD parsing.

A WAD (*Where's All the Data?*) is the binary container format used by id
Software's classic Doom engine to bundle levels, graphics, sounds, and music.
WAD files come in two flavors: **IWAD** (the main game data that ships with
Doom/Doom II) and **PWAD** (user-created patches that override or extend the
IWAD). This library lets you load either format, inspect the lump directory, and
extract raw lump data or typed map records — without performing any unsafe I/O
directly.

# Quick start

```rust
use crustywad::Wad;

// Build a minimal 1-lump IWAD in memory.
let mut bytes = Vec::new();
bytes.extend_from_slice(b"IWAD");
bytes.extend_from_slice(&1_i32.to_le_bytes());   // numlumps = 1
bytes.extend_from_slice(&16_i32.to_le_bytes());  // infotableofs = 16 (after lump data)
bytes.extend_from_slice(&[1, 2, 3, 4]);           // lump data at offset 12
bytes.extend_from_slice(&12_i32.to_le_bytes());  // directory: filepos = 12
bytes.extend_from_slice(&4_i32.to_le_bytes());   // directory: size = 4
bytes.extend_from_slice(b"TEST\0\0\0\0");         // directory: name

let wad = Wad::from_bytes(bytes)?;
assert_eq!(wad.kind(), crustywad::WadKind::Iwad);
assert_eq!(wad.lump_count(), 1);
assert_eq!(wad.lump(0).expect("missing lump").name(), "TEST");
# Ok::<(), crustywad::ParseError>(())
```

# Feature flags

| Feature          | Default | Description |
|------------------|---------|-------------|
| `mmap`           | no  | Enables `Wad::from_path_mapped` for zero-copy memory-mapped loading |
| `write`          | no  | Enables `WadBuilder`, `WriteError`, `WriteOptions`, and `WriteWarning` for WAD serialization |
| `freedoom-tests` | no  | Enables integration tests against local Freedoom WAD fixtures (test-only; not useful as a library dependency) |

# Strictness

[`Strictness`] controls validation on both the **read path** and the **write path**
(requires the `write` feature).

**Reading:** by default, parsing uses [`Strictness::Strict`] and returns the first
[`ParseError`] encountered. Enable [`Strictness::Lenient`] via
[`ParseOptions::lenient()`] to let the parser recover from well-understood
anomalies (negative field values, out-of-bounds ranges, non-ASCII names) and
collect [`ParseWarning`] values instead of aborting.

**Writing:** [`WriteOptions::strict()`] (the default) rejects over-length lump names
and non-standard magic with a [`WriteError`]. [`WriteOptions::lenient()`] truncates
over-length names and writes non-standard magic as-is, returning [`WriteWarning`]
values alongside the serialized bytes.
"#]

//! The current milestone implements real header and directory parsing plus typed
//! scaffolding for the classic map record lumps.

pub mod audio;
mod error;
pub mod gfx;
// Compile-checks the user guide's Rust samples as doctests; exists only during
// doctest collection, so normal builds and `cargo doc` never see it.
#[cfg(all(doctest, feature = "guide-doctests", has_guide_sources))]
mod guide_doctests;
pub mod map;
#[cfg(feature = "mmap")]
mod mmap;
pub mod sections;
mod util;
#[cfg(feature = "write")]
pub mod write;

use std::fs;
use std::io::Cursor;
use std::ops::Deref;
use std::path::Path;

use binrw::{BinRead, BinReaderExt};
#[cfg(feature = "mmap")]
use memmap2::Mmap;

pub use error::{ParseError, ParseWarning};
pub use sections::{Section, SectionError, SectionKind, SectionTable, SectionWarning};
#[cfg(feature = "write")]
pub use write::{WadBuilder, WriteError, WriteOptions, WriteWarning};

/// The identified WAD variant, determined by the 4-byte magic at the start of
/// the file.
///
/// Doom distinguishes two canonical WAD types:
/// - **IWAD** — the base game data distributed with Doom, Doom II, Heretic, and
///   other id/Raven titles. An IWAD is self-contained; the engine requires
///   exactly one.
/// - **PWAD** — a patch WAD that layers additional or replacement lumps on top
///   of the IWAD. PWADs are the standard format for community-created mods,
///   level packs, and total conversions.
///
/// The [`Unknown`][WadKind::Unknown] variant is only produced in lenient mode
/// when the magic bytes are neither `"IWAD"` nor `"PWAD"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WadKind {
    /// An IWAD — the main game data file (e.g. `doom.wad`, `doom2.wad`).
    ///
    /// IWADs contain the complete set of lumps required by the engine: levels,
    /// textures, sprites, sounds, music, and palette data. Loading one of these
    /// is a prerequisite for running the game.
    Iwad,
    /// A PWAD — a patch or add-on WAD (e.g. a user-made map or total
    /// conversion).
    ///
    /// PWADs only need to include the lumps they wish to add or override. The
    /// engine merges them with the IWAD at runtime, with PWAD lumps taking
    /// precedence for any name that appears in both.
    Pwad,
    /// A non-standard or malformed magic value preserved in lenient mode.
    ///
    /// This variant is only produced when the parser is configured with
    /// [`Strictness::Lenient`] and encounters a 4-byte magic that is neither
    /// `b"IWAD"` nor `b"PWAD"`. Strict mode returns
    /// [`ParseError::InvalidMagic`] instead.
    Unknown([u8; 4]),
}

/// Controls how strictly `crustywad` validates WAD data during both reading and
/// writing.
///
/// `Strictness` is shared by [`ParseOptions`] (read path) and
/// [`WriteOptions`] (write path, requires the `write` feature).
///
/// **Reading:** use `Strict` (the default) when loading files you expect to be
/// well-formed — it surfaces problems immediately rather than silently producing
/// a partial result.  Use `Lenient` when you need to inspect or salvage WADs
/// that violate the spec, such as files produced by buggy editors or heavily
/// modified game builds.  In lenient mode the parser emits [`ParseWarning`]
/// values instead of aborting on recoverable anomalies; consult
/// [`Wad::warnings()`] after loading.
///
/// **Writing** (requires `write` feature): use `Strict` to reject any
/// non-standard input (over-length lump names, non-standard magic).  Use
/// `Lenient` to recover where possible — over-length names are truncated and
/// non-standard magic is written as-is — and collect [`WriteWarning`]
/// values returned by [`WadBuilder::build_with_options`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strictness {
    /// Abort on the first validation error and return it immediately.
    ///
    /// On the read path this returns [`ParseError`]; on the write path this
    /// returns [`WriteError`].  Choose this when the input is expected to be
    /// spec-compliant.  It is the default and is equivalent to
    /// `ParseOptions::default()` / `WriteOptions::default()`.
    Strict,
    /// Attempt best-effort recovery and accumulate warnings rather than errors.
    ///
    /// On the **read path**, recoverable conditions (negative header fields,
    /// out-of-bounds lump ranges, non-ASCII lump names) are clamped or decoded
    /// lossily and recorded as [`ParseWarning`] values accessible via
    /// [`Wad::warnings()`].  Parsing only fails if the underlying byte stream
    /// is fundamentally unreadable.
    ///
    /// On the **write path** (requires `write` feature), recoverable conditions
    /// (over-length lump names, non-standard magic) produce [`WriteWarning`]
    /// values returned alongside the serialized bytes.  Unrecoverable conditions
    /// (NUL in a name, non-ASCII names, size or offset overflow) still return
    /// [`WriteError`] in both modes.
    Lenient,
}

/// Resource limits applied by parsers that would otherwise trust on-disk
/// counts and dimensions.
///
/// Bounds UDMF text nesting depth (`max_depth`), the pixel allocation of a
/// single texture composition (`max_composite_pixels`), the pixel allocation
/// of a single `doom64-gfx` PNG decode (`max_decoded_pixels`), and the inflated
/// output of a single compressed extended-node lump (`max_decoded_node_bytes`,
/// applied during binary **or** UDMF map assembly when the `extended-nodes-zlib`
/// feature is enabled). A path that touches none of these — e.g. a classic map
/// with uncompressed nodes — is unaffected. Construct via [`Limits::new`]
/// and the `with_*` setters — the struct is `#[non_exhaustive]` so future
/// limits can be added without a breaking change.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Maximum block-nesting depth accepted by the UDMF text parser.
    pub max_depth: usize,
    /// Maximum `width × height` in pixels a single texture composition may
    /// allocate, enforced in BOTH strictness modes — a `TEXTUREx` header
    /// can declare 32767 × 32767 (≈ 1 GiB) from a 30-byte lump
    /// (ADR-0022 §3/§6).
    pub max_composite_pixels: usize,
    /// Maximum `width × height` in pixels a single `doom64-gfx` PNG decode
    /// may allocate, enforced in BOTH strictness modes — Doom64 EX sizes
    /// an uncapped allocation from library-reported dimensions
    /// (ADR-0022 §5). Independent of the `png` crate's internal limits.
    pub max_decoded_pixels: usize,
    /// Maximum number of bytes a single compressed extended-node lump
    /// (`ZNOD`/`ZGLN`/`ZGL2`/`ZGL3`) may inflate to, enforced in BOTH
    /// strictness modes on the `extended-nodes-zlib` path. A tiny compressed
    /// lump can otherwise expand without bound, so this cap is passed straight
    /// to `miniz_oxide`'s length-limited inflater as the ADR-0016 §1
    /// bounded-output guard (ADR-0025 §5). Exceeding it is treated like a
    /// corrupt stream: strict errors, lenient degrades to empty arenas.
    pub max_decoded_node_bytes: usize,
}

impl Limits {
    /// The default limits (`max_depth = 64`, `max_composite_pixels = 1 <<
    /// 24`, `max_decoded_pixels = 1 << 24`, `max_decoded_node_bytes = 1 <<
    /// 26`).
    #[must_use]
    pub const fn new() -> Self {
        Self {
            max_depth: 64,
            max_composite_pixels: 1 << 24,
            max_decoded_pixels: 1 << 24,
            max_decoded_node_bytes: 1 << 26,
        }
    }

    /// Returns these limits with `max_depth` replaced.
    #[must_use]
    pub const fn with_max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth;
        self
    }

    /// Returns these limits with `max_composite_pixels` replaced.
    #[must_use]
    pub const fn with_max_composite_pixels(mut self, max_composite_pixels: usize) -> Self {
        self.max_composite_pixels = max_composite_pixels;
        self
    }

    /// Returns these limits with `max_decoded_pixels` replaced.
    #[must_use]
    pub const fn with_max_decoded_pixels(mut self, max_decoded_pixels: usize) -> Self {
        self.max_decoded_pixels = max_decoded_pixels;
        self
    }

    /// Returns these limits with `max_decoded_node_bytes` replaced.
    #[must_use]
    pub const fn with_max_decoded_node_bytes(mut self, max_decoded_node_bytes: usize) -> Self {
        self.max_decoded_node_bytes = max_decoded_node_bytes;
        self
    }
}

impl Default for Limits {
    fn default() -> Self {
        Self::new()
    }
}

/// Parser configuration passed to the `_with_options` loading functions.
///
/// The default configuration uses [`Strictness::Strict`], which is the right
/// choice for almost all production use.  Switch to [`Strictness::Lenient`]
/// only when you need to inspect or recover data from non-compliant WADs.
///
/// # Examples
///
/// ```rust
/// use crustywad::{ParseOptions, Strictness};
///
/// // Strict is the default.
/// assert_eq!(ParseOptions::default().strictness, Strictness::Strict);
///
/// // Use the convenience constructors to avoid spelling out the field.
/// let opts = ParseOptions::lenient();
/// assert_eq!(opts.strictness, Strictness::Lenient);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseOptions {
    /// The chosen validation strategy.
    pub strictness: Strictness,
    /// Resource limits applied to parsing (currently: UDMF nesting depth).
    /// Ignored by all binary-format paths.
    pub limits: Limits,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            strictness: Strictness::Strict,
            limits: Limits::new(),
        }
    }
}

impl ParseOptions {
    /// Returns a strict parser configuration.
    ///
    /// Equivalent to `ParseOptions::default()`. Parsing aborts on the first
    /// validation error.
    #[must_use]
    pub const fn strict() -> Self {
        Self {
            strictness: Strictness::Strict,
            limits: Limits::new(),
        }
    }

    /// Returns a lenient parser configuration.
    ///
    /// The parser attempts best-effort recovery on recoverable anomalies and
    /// collects [`ParseWarning`] values instead of returning an error.  Check
    /// [`Wad::warnings()`] after loading to inspect any issues found.
    #[must_use]
    pub const fn lenient() -> Self {
        Self {
            strictness: Strictness::Lenient,
            limits: Limits::new(),
        }
    }
}

/// The parsed WAD header, decoded from the first 12 bytes of the file.
///
/// The on-disk layout is: 4-byte magic (`"IWAD"` or `"PWAD"`), a little-endian
/// `i32` lump count, and a little-endian `i32` byte offset to the lump
/// directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WadHeader {
    /// The WAD file kind, derived from the 4-byte magic field.
    pub kind: WadKind,
    /// The number of lump directory entries successfully parsed.
    ///
    /// In strict mode this equals the value in the header field (`numlumps`).
    /// In lenient mode this may be less than `numlumps` when the declared
    /// directory extends beyond the available bytes — only the entries that
    /// fit within the buffer are parsed.
    pub num_lumps: usize,
    /// The byte offset of the lump directory from the start of the WAD buffer.
    ///
    /// This corresponds to the `infotableofs` header field. Each directory
    /// entry is 16 bytes: a 4-byte `filepos`, a 4-byte `size`, and an 8-byte
    /// NUL-padded ASCII name.
    pub info_table_offset: usize,
}

/// A validated lump directory entry describing one named data chunk.
///
/// In the WAD format every piece of data — a level map, a texture, a sound
/// effect — is stored as a *lump*.  The lump directory at the end of the file
/// records where each lump starts (`filepos`), how large it is (`size`), and
/// what it is called (`name`).
///
/// # Name encoding
///
/// Lump names in the WAD directory are stored as 8-byte ASCII fields padded
/// with `\0` bytes.  Names are therefore at most 8 characters long; the parser
/// strips trailing `\0` bytes when constructing the [`String`] returned by
/// [`name()`][Lump::name].  Doom WAD names are conventionally uppercase ASCII
/// (e.g. `"E1M1"`, `"THINGS"`, `"TITLEPIC"`), though the format itself does
/// not enforce case.
///
/// Note that lump names are **not unique** within a WAD — both IWADs and PWADs
/// routinely contain multiple lumps with the same name (e.g. `"TEXTURE1"` can
/// appear in both). [`Wad::lump_by_name()`] always returns the *first* match
/// in directory order; use [`Wad::lumps()`] to iterate all entries when you
/// need to handle duplicates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lump {
    name: String,
    filepos: usize,
    size: usize,
}

impl Lump {
    /// Returns the lump name with trailing `\0` padding removed.
    ///
    /// The value is at most 8 ASCII characters long. In strict mode all
    /// returned names are guaranteed to be valid ASCII. In lenient mode a
    /// non-ASCII name is decoded lossily and a [`ParseWarning::NonAsciiName`]
    /// is recorded.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the byte offset of this lump's data within the WAD buffer.
    ///
    /// Pass this offset to index the raw byte slice returned by
    /// [`Wad::into_bytes()`], or use the higher-level [`Wad::lump_bytes()`]
    /// and [`Wad::lump_data()`] helpers instead.
    #[must_use]
    pub const fn filepos(&self) -> usize {
        self.filepos
    }

    /// Returns the validated byte length of this lump's data.
    ///
    /// The lump data spans `filepos..filepos + size` within the WAD buffer.
    /// Both `filepos` and `size` are guaranteed to be in-bounds after parsing
    /// (in strict mode an out-of-bounds range is a hard error; in lenient mode
    /// the range is clamped and a [`ParseWarning::OutOfBounds`] is recorded).
    #[must_use]
    pub const fn size(&self) -> usize {
        self.size
    }
}

#[derive(Debug)]
enum WadData {
    Owned(Vec<u8>),
    #[cfg(feature = "mmap")]
    Mapped(Mmap),
}

impl WadData {
    fn as_slice(&self) -> &[u8] {
        match self {
            WadData::Owned(v) => v,
            #[cfg(feature = "mmap")]
            WadData::Mapped(m) => m,
        }
    }
}

impl Deref for WadData {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        self.as_slice()
    }
}

/// An owned Doom WAD loaded into memory, with its parsed header and lump
/// directory.
///
/// `Wad` holds the complete WAD bytes together with the validated header and
/// lump directory.  Use the [`from_bytes`][Wad::from_bytes],
/// [`from_path`][Wad::from_path], or (with the `mmap` feature)
/// `from_path_mapped` constructors to obtain one.
///
/// Once loaded, iterate the lump directory via [`lumps()`][Wad::lumps()], look
/// up by name with [`lump_by_name()`][Wad::lump_by_name()], and extract raw
/// bytes with [`lump_bytes()`][Wad::lump_bytes()] or
/// [`lump_data()`][Wad::lump_data()].
#[derive(Debug)]
pub struct Wad {
    header: WadHeader,
    lumps: Vec<Lump>,
    bytes: WadData,
    warnings: Vec<ParseWarning>,
}

impl Clone for Wad {
    fn clone(&self) -> Self {
        Self {
            header: self.header,
            lumps: self.lumps.clone(),
            bytes: WadData::Owned(self.bytes.to_vec()),
            warnings: self.warnings.clone(),
        }
    }
}

#[cfg_attr(feature = "write", derive(binrw::BinWrite))]
#[cfg_attr(feature = "write", bw(little))]
#[derive(Debug, BinRead)]
#[br(little)]
struct RawHeader {
    magic: [u8; 4],
    numlumps: i32,
    infotableofs: i32,
}

#[cfg_attr(feature = "write", derive(binrw::BinWrite))]
#[cfg_attr(feature = "write", bw(little))]
#[derive(Debug, Clone, Copy, BinRead)]
#[br(little)]
struct RawDirectoryEntry {
    filepos: i32,
    size: i32,
    name: [u8; 8],
}

impl Wad {
    /// Parses a WAD from an owned or borrowed in-memory byte buffer using the
    /// default strict rules.
    ///
    /// Accepts any type that converts into `Vec<u8>` — including `Vec<u8>`
    /// (moved without copying), `&[u8]`, and fixed-size byte arrays.  Use
    /// [`from_bytes_with_options`][Wad::from_bytes_with_options] to opt into
    /// lenient parsing.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] when the buffer does not contain a valid WAD
    /// according to the default strict parsing rules:
    ///
    /// - [`ParseError::Header`] — the first 12 bytes cannot be decoded.
    /// - [`ParseError::InvalidMagic`] — the magic is not `"IWAD"` or `"PWAD"`.
    /// - [`ParseError::NegativeValue`] — `numlumps` or `infotableofs` is
    ///   negative.
    /// - [`ParseError::OutOfBounds`] — the declared directory extends past the
    ///   buffer end.
    /// - [`ParseError::Directory`] — a directory entry cannot be decoded.
    /// - [`ParseError::NonAsciiName`] — a lump name contains non-ASCII bytes.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crustywad::{Wad, WadKind};
    ///
    /// // Build a minimal valid PWAD with one zero-byte lump.
    /// let mut bytes = Vec::new();
    /// bytes.extend_from_slice(b"PWAD");
    /// bytes.extend_from_slice(&1_i32.to_le_bytes());   // numlumps = 1
    /// bytes.extend_from_slice(&12_i32.to_le_bytes());  // infotableofs = 12
    /// // Directory entry: filepos=0, size=0, name="MYMAP\0\0\0"
    /// bytes.extend_from_slice(&0_i32.to_le_bytes());
    /// bytes.extend_from_slice(&0_i32.to_le_bytes());
    /// bytes.extend_from_slice(b"MYMAP\0\0\0");
    ///
    /// let wad = Wad::from_bytes(bytes)?;
    /// assert_eq!(wad.kind(), WadKind::Pwad);
    /// assert_eq!(wad.lump_count(), 1);
    /// assert_eq!(wad.lump(0).unwrap().name(), "MYMAP");
    /// # Ok::<(), crustywad::ParseError>(())
    /// ```
    pub fn from_bytes(bytes: impl Into<Vec<u8>>) -> Result<Self, ParseError> {
        Self::from_bytes_with_options(bytes.into(), ParseOptions::default())
    }

    /// Parses a WAD from an in-memory byte buffer using explicit parse options.
    ///
    /// Use [`ParseOptions::lenient()`] to recover partial data from malformed
    /// WADs; check [`Wad::warnings()`] afterwards for any issues encountered.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] when the bytes cannot be decoded or validated
    /// according to the supplied [`ParseOptions`]. In lenient mode the parser
    /// recovers from many validation anomalies (bad magic, negative values,
    /// out-of-bounds ranges, non-ASCII names) and records [`ParseWarning`]s
    /// instead; truly unrecoverable failures (truncated header, unreadable
    /// directory entry) still return [`ParseError`]. See [`ParseError`] for
    /// the full list.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crustywad::{ParseOptions, Wad, WadKind};
    ///
    /// // A WAD with an invalid magic is normally rejected …
    /// let mut bytes = Vec::new();
    /// bytes.extend_from_slice(b"XWAD");           // unknown magic
    /// bytes.extend_from_slice(&0_i32.to_le_bytes()); // numlumps = 0
    /// bytes.extend_from_slice(&12_i32.to_le_bytes()); // infotableofs = 12
    ///
    /// // … but lenient mode preserves it and records a warning.
    /// let wad = Wad::from_bytes_with_options(bytes, ParseOptions::lenient())?;
    /// assert!(matches!(wad.kind(), WadKind::Unknown(_)));
    /// assert!(!wad.warnings().is_empty());
    /// # Ok::<(), crustywad::ParseError>(())
    /// ```
    pub fn from_bytes_with_options(
        bytes: impl Into<Vec<u8>>,
        options: ParseOptions,
    ) -> Result<Self, ParseError> {
        let bytes = bytes.into();
        let (header, lumps, warnings) = parse_bytes(&bytes, options)?;
        Ok(Self {
            header,
            lumps,
            bytes: WadData::Owned(bytes),
            warnings,
        })
    }

    /// Reads a WAD from a file path into memory using strict parsing.
    ///
    /// The entire file is read into a `Vec<u8>` before parsing.  For large
    /// WADs where only a subset of lumps will be accessed you can avoid the
    /// heap copy by using [`from_path_mapped`][Wad::from_path_mapped] (requires
    /// the `mmap` feature).
    ///
    /// # Errors
    ///
    /// Returns [`ParseError::Io`] if the file cannot be read, or any strict
    /// parse error if the WAD fails validation.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, ParseError> {
        Self::from_path_with_options(path, ParseOptions::default())
    }

    /// Reads a WAD from a file path into memory using explicit parse options.
    ///
    /// The entire file is read into a `Vec<u8>` before parsing.  For large
    /// WADs where only a subset of lumps will be accessed you can avoid the
    /// heap copy by using
    /// [`from_path_mapped_with_options`][Wad::from_path_mapped_with_options]
    /// (requires the `mmap` feature).
    ///
    /// # Errors
    ///
    /// Returns [`ParseError::Io`] if the file cannot be read, or a parse error
    /// if the WAD fails validation under the provided [`ParseOptions`].
    pub fn from_path_with_options(
        path: impl AsRef<Path>,
        options: ParseOptions,
    ) -> Result<Self, ParseError> {
        let path = path.as_ref();
        let bytes = fs::read(path).map_err(|source| ParseError::Io {
            path: path.display().to_string(),
            source,
        })?;
        let (header, lumps, warnings) = parse_bytes(&bytes, options)?;
        Ok(Self {
            header,
            lumps,
            bytes: WadData::Owned(bytes),
            warnings,
        })
    }

    /// Reads a WAD from a file using read-only memory-mapped I/O and strict
    /// parsing.
    ///
    /// The file is mapped into the process address space without copying it to
    /// the heap.  The mapping is held for the lifetime of the returned [`Wad`],
    /// which makes this more efficient than [`from_path`][Wad::from_path] for
    /// large WADs when only a subset of lumps will be accessed — pages are
    /// faulted in on demand by the OS rather than read eagerly.
    ///
    /// **Warning:** the WAD file must not be truncated or replaced while the
    /// [`Wad`] is alive.  On Unix, truncation from another process triggers a
    /// `SIGBUS` on the next lump data access, which will abort the process.
    /// On Windows the mapping prevents truncation but concurrent writes by
    /// another process may expose inconsistent data.  This crate performs no
    /// unsafe memory operations on the mapping, but it cannot protect the
    /// process from OS-level signals caused by external file modification.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError::Io`] if the file cannot be opened or mapped, or
    /// any strict parse error if the WAD fails validation.
    #[cfg(feature = "mmap")]
    pub fn from_path_mapped(path: impl AsRef<Path>) -> Result<Self, ParseError> {
        Self::from_path_mapped_with_options(path, ParseOptions::default())
    }

    /// Reads a WAD from a file using read-only memory-mapped I/O and explicit
    /// parse options.
    ///
    /// The file is mapped read-only; no heap copy is made.  See
    /// [`from_path_mapped`][Wad::from_path_mapped] for the full discussion of
    /// the safety constraints around file lifetime.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError::Io`] if the file cannot be opened or mapped, or a
    /// parse error if the WAD fails validation under the provided
    /// [`ParseOptions`].
    #[cfg(feature = "mmap")]
    pub fn from_path_mapped_with_options(
        path: impl AsRef<Path>,
        options: ParseOptions,
    ) -> Result<Self, ParseError> {
        let mapped = mmap::open(path.as_ref())?;
        let (header, lumps, warnings) = parse_bytes(&mapped, options)?;
        Ok(Self {
            header,
            lumps,
            bytes: WadData::Mapped(mapped),
            warnings,
        })
    }

    /// Returns the WAD kind (`Iwad`, `Pwad`, or `Unknown`).
    #[must_use]
    pub const fn kind(&self) -> WadKind {
        self.header.kind
    }

    /// Returns the parsed WAD header containing the kind, lump count, and
    /// directory offset.
    #[must_use]
    pub const fn header(&self) -> &WadHeader {
        &self.header
    }

    /// Returns the number of lumps parsed from the directory.
    ///
    /// In lenient mode this may be smaller than the count declared in the
    /// header when the declared directory extends beyond the buffer end.
    #[must_use]
    pub fn lump_count(&self) -> usize {
        self.lumps.len()
    }

    /// Returns the full slice of parsed lump directory entries.
    ///
    /// Lumps are in directory order, which is the order the engine processes
    /// them. Use this when you need to iterate all entries or handle duplicate
    /// names.
    #[must_use]
    pub fn lumps(&self) -> &[Lump] {
        &self.lumps
    }

    /// Returns the lump at a zero-based directory index, or `None` if the
    /// index is out of range.
    #[must_use]
    pub fn lump(&self, index: usize) -> Option<&Lump> {
        self.lumps.get(index)
    }

    /// Returns the first lump whose name matches `name`, or `None` if no lump
    /// has that name.
    ///
    /// # Doom WAD name semantics
    ///
    /// - Lump names are stored in the directory as 8-byte NUL-padded ASCII
    ///   fields, so names longer than 8 characters will never match.
    /// - Names are **not unique** — PWADs deliberately contain lumps with the
    ///   same name as the IWAD lumps they override.  This method returns only
    ///   the **first** directory entry with the given name (lowest index).
    ///   Iterate [`lumps()`][Wad::lumps()] directly if you need all matches.
    /// - Matching is exact and case-sensitive.  Pass the name in the correct
    ///   case (conventionally uppercase in WAD files, e.g. `"E1M1"`, `"THINGS"`).
    ///
    /// This performs a linear scan; see ADR-0013 for why that is the right
    /// tradeoff today, and the conditions under which it should be revisited.
    #[must_use]
    pub fn lump_by_name(&self, name: &str) -> Option<&Lump> {
        self.lumps.iter().find(|lump| lump.name() == name)
    }

    /// Identifies every map lump group in the directory, in order.
    #[must_use]
    pub fn map_groups(&self) -> Vec<crate::map::MapGroup> {
        crate::map::group::map_groups(self)
    }

    /// Returns the first map group whose marker lump is named `name`
    /// (exact, case-sensitive — consistent with [`Wad::lump_by_name`]).
    #[must_use]
    pub fn map_group(&self, name: &str) -> Option<crate::map::MapGroup> {
        crate::map::group::map_group(self, name)
    }

    /// Scans the directory for marker-delimited sections, strictly (the
    /// [`Map::assemble`](crate::map::Map::assemble) idiom — the first
    /// malformed marker layout is an error).
    ///
    /// # Errors
    ///
    /// Every [`SectionError`] variant documents its condition; lenient
    /// mode ([`Wad::sections_with_options`]) recovers each into a
    /// [`SectionWarning`] instead.
    pub fn sections(&self) -> Result<crate::sections::SectionTable, crate::sections::SectionError> {
        self.sections_with_options(ParseOptions::strict())
    }

    /// [`Wad::sections`] honoring the given [`ParseOptions`]' strictness;
    /// lenient mode never fails and records recoveries as warnings on the
    /// returned table.
    ///
    /// # Errors
    ///
    /// Strict mode only: the first marker anomaly, per [`SectionError`].
    pub fn sections_with_options(
        &self,
        options: ParseOptions,
    ) -> Result<crate::sections::SectionTable, crate::sections::SectionError> {
        crate::sections::scan(self, options.strictness)
    }

    /// Builds the Doom 64 texture-name resolution table strictly (the
    /// [`Wad::sections`] idiom). `Ok(None)` when the WAD has no `Textures`
    /// section — not an error (a bare nested-map WAD is legitimate).
    ///
    /// # Errors
    ///
    /// The first marker anomaly from the underlying strict section scan,
    /// per [`SectionError`].
    pub fn doom64_texture_names(
        &self,
    ) -> Result<Option<crate::map::Doom64TextureNames>, crate::sections::SectionError> {
        self.doom64_texture_names_with_options(ParseOptions::strict())
    }

    /// [`Wad::doom64_texture_names`] honoring the given strictness. Lenient
    /// scan warnings are discarded here — callers wanting them scan via
    /// [`Wad::sections_with_options`] themselves (map assembly does, to
    /// bridge them into its warning stream).
    ///
    /// # Errors
    ///
    /// Strict mode only: the first marker anomaly, per [`SectionError`].
    pub fn doom64_texture_names_with_options(
        &self,
        options: ParseOptions,
    ) -> Result<Option<crate::map::Doom64TextureNames>, crate::sections::SectionError> {
        let sections = self.sections_with_options(options)?;
        Ok(crate::map::Doom64TextureNames::from_sections(
            self, &sections,
        ))
    }

    /// Parses the WAD's `PLAYPAL` lump strictly, or `Ok(None)` when no such
    /// lump exists (PWADs commonly omit it). Uses the crate's documented
    /// first-match [`Wad::lump_by_name`] contract; vanilla's backward
    /// directory scan would take the last occurrence instead (ADR-0022 §2's
    /// precedence note) — duplicate `PLAYPAL` lumps in one WAD are
    /// degenerate, and crate-wide consistency wins.
    ///
    /// # Errors
    ///
    /// [`gfx::GfxError::PlaypalSize`] per strict parsing.
    pub fn playpal(&self) -> Result<Option<gfx::Playpal>, gfx::GfxError> {
        self.playpal_with_options(ParseOptions::strict())
    }

    /// [`Wad::playpal`] honoring the given strictness.
    ///
    /// # Errors
    ///
    /// Strict mode only: [`gfx::GfxError::PlaypalSize`].
    pub fn playpal_with_options(
        &self,
        options: ParseOptions,
    ) -> Result<Option<gfx::Playpal>, gfx::GfxError> {
        self.lump_by_name("PLAYPAL")
            .map(|lump| gfx::Playpal::parse(self.lump_data(lump), &options))
            .transpose()
    }

    /// Parses the WAD's `COLORMAP` lump strictly, or `Ok(None)` when no
    /// such lump exists (same lookup contract as [`Wad::playpal`]).
    ///
    /// # Errors
    ///
    /// [`gfx::GfxError::ColormapSize`] per strict parsing.
    pub fn colormap(&self) -> Result<Option<gfx::Colormap>, gfx::GfxError> {
        self.colormap_with_options(ParseOptions::strict())
    }

    /// [`Wad::colormap`] honoring the given strictness.
    ///
    /// # Errors
    ///
    /// Strict mode only: [`gfx::GfxError::ColormapSize`].
    pub fn colormap_with_options(
        &self,
        options: ParseOptions,
    ) -> Result<Option<gfx::Colormap>, gfx::GfxError> {
        self.lump_by_name("COLORMAP")
            .map(|lump| gfx::Colormap::parse(self.lump_data(lump), &options))
            .transpose()
    }

    /// Builds the WAD's [`gfx::TextureSet`] strictly from its `TEXTURE1`
    /// and/or `TEXTURE2` lumps (plus `PNAMES` and the patch lumps they
    /// reference), or `Ok(None)` when neither `TEXTUREx` lump is present.
    ///
    /// # Errors
    ///
    /// Strict mode: [`gfx::GfxError::MissingPnames`],
    /// [`gfx::GfxError::PatchIndexOutOfBounds`],
    /// [`gfx::GfxError::UnresolvedPatchName`],
    /// [`gfx::GfxError::PatchPictureFailed`], or any [`gfx::GfxError`] from
    /// parsing `TEXTUREx`/`PNAMES`/a patch picture. For best-effort
    /// recovery of each case (with warnings on the returned set), use
    /// [`Wad::texture_set_with_options`] with [`ParseOptions::lenient`].
    pub fn texture_set(&self) -> Result<Option<gfx::TextureSet>, gfx::GfxError> {
        self.texture_set_with_options(ParseOptions::strict())
    }

    /// [`Wad::texture_set`] honoring the given strictness.
    ///
    /// # Errors
    ///
    /// Strict mode only: the rows listed on [`Wad::texture_set`].
    pub fn texture_set_with_options(
        &self,
        options: ParseOptions,
    ) -> Result<Option<gfx::TextureSet>, gfx::GfxError> {
        gfx::TextureSet::from_wad(self, &options)
    }

    /// Returns the raw bytes for the lump at the given zero-based index, or
    /// `None` if the index is out of range.
    ///
    /// The returned slice is a view into the WAD buffer; no allocation is
    /// performed.
    #[must_use]
    pub fn lump_bytes(&self, index: usize) -> Option<&[u8]> {
        self.lump(index)
            .map(|lump| &self.bytes[lump.filepos..lump.filepos + lump.size])
    }

    /// Returns the raw bytes for the provided lump metadata.
    ///
    /// This is a convenience alternative to [`lump_bytes()`][Wad::lump_bytes()]
    /// when you already hold a reference to a [`Lump`] (e.g. obtained from
    /// [`lump_by_name()`][Wad::lump_by_name()]).  The returned slice is a
    /// view into the WAD buffer; no allocation is performed.
    ///
    /// # Panics
    ///
    /// Panics if `lump` does not belong to this [`Wad`].  This can happen when
    /// a `Lump` obtained from a *different*, larger `Wad` is passed here and
    /// its byte range falls outside this buffer.  Always use lumps returned by
    /// the same `Wad` instance.
    #[must_use]
    pub fn lump_data(&self, lump: &Lump) -> &[u8] {
        let end = lump.filepos + lump.size;
        assert!(
            end <= self.bytes.len(),
            "lump range {}..{} is out of bounds for a buffer of length {}",
            lump.filepos,
            end,
            self.bytes.len()
        );
        &self.bytes[lump.filepos..end]
    }

    /// Returns any non-fatal warnings produced during lenient parsing.
    ///
    /// In strict mode (the default) this slice is always empty because parsing
    /// aborts on the first problem.  In lenient mode each recoverable anomaly
    /// appends a [`ParseWarning`] here instead of returning an error.
    #[must_use]
    pub fn warnings(&self) -> &[ParseWarning] {
        &self.warnings
    }

    /// Consumes the `Wad` and returns the underlying byte buffer.
    ///
    /// When the WAD was loaded with [`from_bytes`][Wad::from_bytes] or
    /// [`from_path`][Wad::from_path] the original `Vec<u8>` is returned
    /// without copying.  When the `mmap` feature is enabled and the WAD was
    /// loaded via [`from_path_mapped`][Wad::from_path_mapped], the mapped
    /// region is copied into a new `Vec<u8>` and the mapping is released.
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        match self.bytes {
            WadData::Owned(v) => v,
            #[cfg(feature = "mmap")]
            WadData::Mapped(m) => m.to_vec(),
        }
    }

    /// Converts this `Wad` into a [`WadBuilder`] for round-tripping or editing.
    ///
    /// All lump data is copied into the builder. Memory usage roughly doubles
    /// during the conversion.
    ///
    /// # Examples
    ///
    /// ```
    /// use crustywad::Wad;
    ///
    /// # let mut bytes = Vec::new();
    /// # bytes.extend_from_slice(b"IWAD");
    /// # bytes.extend_from_slice(&1_i32.to_le_bytes());
    /// # bytes.extend_from_slice(&16_i32.to_le_bytes());
    /// # bytes.extend_from_slice(&[1, 2, 3, 4]);
    /// # bytes.extend_from_slice(&12_i32.to_le_bytes());
    /// # bytes.extend_from_slice(&4_i32.to_le_bytes());
    /// # bytes.extend_from_slice(b"TEST\0\0\0\0");
    /// let wad = Wad::from_bytes(bytes)?;
    ///
    /// let mut builder = wad.to_builder();
    /// builder.add_lump("EXTRA", b"more data");
    /// let rebuilt = builder.build()?;
    ///
    /// let reparsed = Wad::from_bytes(rebuilt)?;
    /// assert_eq!(reparsed.lump_count(), 2);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[cfg(feature = "write")]
    #[must_use]
    pub fn to_builder(&self) -> write::WadBuilder {
        let mut builder = write::WadBuilder::new(self.kind());
        for lump in self.lumps() {
            let data = self.lump_data(lump).to_vec();
            builder.add_lump(lump.name(), data);
        }
        builder
    }
}

fn parse_bytes(
    bytes: &[u8],
    options: ParseOptions,
) -> Result<(WadHeader, Vec<Lump>, Vec<ParseWarning>), ParseError> {
    let mut cursor = Cursor::new(bytes);
    let raw = cursor.read_le::<RawHeader>().map_err(ParseError::Header)?;
    let len = bytes.len();
    let mut warnings = Vec::new();

    let kind = match &raw.magic {
        b"IWAD" => WadKind::Iwad,
        b"PWAD" => WadKind::Pwad,
        magic => match options.strictness {
            Strictness::Strict => {
                return Err(ParseError::InvalidMagic {
                    magic: String::from_utf8_lossy(magic.as_slice()).into_owned(),
                });
            }
            Strictness::Lenient => {
                warnings.push(ParseWarning::InvalidMagic(
                    String::from_utf8_lossy(magic.as_slice()).into_owned(),
                ));
                WadKind::Unknown(*magic)
            }
        },
    };

    let num_lumps = coerce_i32(raw.numlumps, "numlumps", options.strictness, &mut warnings)?;
    let info_table_offset = coerce_i32(
        raw.infotableofs,
        "infotableofs",
        options.strictness,
        &mut warnings,
    )?;

    let dir_span = checked_mul(
        num_lumps,
        16,
        "directory length",
        options.strictness,
        &mut warnings,
    )?;
    let available_entries = available_entries(len, info_table_offset);
    let lump_count = if info_table_offset > len || info_table_offset.saturating_add(dir_span) > len
    {
        match options.strictness {
            Strictness::Strict => {
                return Err(ParseError::OutOfBounds {
                    field: "directory",
                    offset: info_table_offset,
                    size: dir_span,
                    len,
                });
            }
            Strictness::Lenient => {
                warnings.push(ParseWarning::OutOfBounds {
                    field: "directory",
                    offset: info_table_offset,
                    size: dir_span,
                    len,
                });
                num_lumps.min(available_entries)
            }
        }
    } else {
        num_lumps
    };

    let header = WadHeader {
        kind,
        num_lumps: lump_count,
        info_table_offset,
    };

    let directory_end = info_table_offset
        .saturating_add(lump_count.saturating_mul(16))
        .min(len);

    let mut directory_cursor = Cursor::new(&bytes[info_table_offset.min(len)..]);
    let mut lumps = Vec::with_capacity(lump_count);
    for index in 0..lump_count {
        let raw_entry = directory_cursor
            .read_le::<RawDirectoryEntry>()
            .map_err(|source| ParseError::Directory { index, source })?;
        lumps.push(validate_entry(
            index,
            raw_entry,
            len,
            info_table_offset,
            directory_end,
            options.strictness,
            &mut warnings,
        )?);
    }

    Ok((header, lumps, warnings))
}

fn coerce_i32(
    value: i32,
    field: &'static str,
    strictness: Strictness,
    warnings: &mut Vec<ParseWarning>,
) -> Result<usize, ParseError> {
    if value < 0 {
        return match strictness {
            Strictness::Strict => Err(ParseError::NegativeValue { field, value }),
            Strictness::Lenient => {
                warnings.push(ParseWarning::NegativeValue { field, value });
                Ok(0)
            }
        };
    }

    usize::try_from(value).map_err(|_| ParseError::Overflow { field })
}

fn checked_mul(
    left: usize,
    right: usize,
    field: &'static str,
    strictness: Strictness,
    warnings: &mut Vec<ParseWarning>,
) -> Result<usize, ParseError> {
    left.checked_mul(right).map_or_else(
        || match strictness {
            Strictness::Strict => Err(ParseError::Overflow { field }),
            Strictness::Lenient => {
                warnings.push(ParseWarning::Overflow { field });
                Ok(usize::MAX)
            }
        },
        Ok,
    )
}

fn available_entries(len: usize, offset: usize) -> usize {
    len.saturating_sub(offset) / 16
}

fn validate_entry(
    index: usize,
    raw_entry: RawDirectoryEntry,
    len: usize,
    directory_offset: usize,
    directory_end: usize,
    strictness: Strictness,
    warnings: &mut Vec<ParseWarning>,
) -> Result<Lump, ParseError> {
    let filepos = coerce_i32(raw_entry.filepos, "filepos", strictness, warnings)?;
    let size = coerce_i32(raw_entry.size, "size", strictness, warnings)?;
    let name = decode_name(index, raw_entry.name, strictness, warnings)?;

    let end = match filepos.checked_add(size) {
        Some(end) => end,
        None => match strictness {
            Strictness::Strict => {
                return Err(ParseError::Overflow {
                    field: "lump range",
                });
            }
            Strictness::Lenient => {
                warnings.push(ParseWarning::Overflow {
                    field: "lump range",
                });
                usize::MAX
            }
        },
    };
    // Lump data must not overlap the directory region. Any filepos before
    // directory_end is capped at directory_offset; filepos at or after
    // directory_end may extend freely to end-of-file.
    let max_end = if filepos < directory_end {
        directory_offset.min(len)
    } else {
        len
    };
    let (filepos, size) = if filepos > max_end || end > max_end {
        match strictness {
            Strictness::Strict => {
                return Err(ParseError::OutOfBounds {
                    field: "lump data",
                    offset: filepos,
                    size,
                    len,
                });
            }
            Strictness::Lenient => {
                warnings.push(ParseWarning::OutOfBounds {
                    field: "lump data",
                    offset: filepos,
                    size,
                    len,
                });
                let clamped_start = filepos.min(max_end);
                let clamped_end = end.min(max_end);
                (clamped_start, clamped_end.saturating_sub(clamped_start))
            }
        }
    } else {
        (filepos, size)
    };

    Ok(Lump {
        name,
        filepos,
        size,
    })
}

fn decode_name(
    index: usize,
    bytes: [u8; 8],
    strictness: Strictness,
    warnings: &mut Vec<ParseWarning>,
) -> Result<String, ParseError> {
    let trimmed = crate::util::trim_nul(&bytes);
    if !trimmed.is_ascii() {
        return match strictness {
            Strictness::Strict => Err(ParseError::NonAsciiName { index }),
            Strictness::Lenient => {
                warnings.push(ParseWarning::NonAsciiName { index });
                Ok(String::from_utf8_lossy(trimmed).into_owned())
            }
        };
    }
    Ok(String::from_utf8_lossy(trimmed).into_owned())
}

#[cfg(test)]
mod tests {
    use super::{Limits, ParseOptions, Strictness};

    #[test]
    fn limits_new_and_default_use_max_depth_64() {
        assert_eq!(Limits::new().max_depth, 64);
        assert_eq!(Limits::default().max_depth, 64);
        assert_eq!(Limits::default(), Limits::new());
    }

    #[test]
    fn with_max_decoded_node_bytes_replaces_only_that_field() {
        let base = Limits::new();
        let updated = base.with_max_decoded_node_bytes(1234);
        assert_eq!(updated.max_decoded_node_bytes, 1234);
        // Every other field is untouched.
        assert_eq!(updated.max_depth, base.max_depth);
        assert_eq!(updated.max_composite_pixels, base.max_composite_pixels);
        assert_eq!(updated.max_decoded_pixels, base.max_decoded_pixels);
    }

    #[test]
    fn parse_options_carry_default_limits() {
        assert_eq!(ParseOptions::default().limits, Limits::new());
        assert_eq!(ParseOptions::strict().limits, Limits::new());
        assert_eq!(ParseOptions::lenient().limits, Limits::new());
        assert_eq!(ParseOptions::default().strictness, Strictness::Strict);
        assert_eq!(ParseOptions::lenient().strictness, Strictness::Lenient);
    }
}
