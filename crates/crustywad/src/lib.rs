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

| Feature | Default | Description |
|---------|---------|-------------|
| `mmap`  | off     | Enables `Wad::from_path_mapped` for zero-copy memory-mapped loading |

# Strictness

By default, parsing uses [`Strictness::Strict`] and returns the first
[`ParseError`] encountered. Enable [`Strictness::Lenient`] via
[`ParseOptions::lenient()`] to let the parser recover from well-understood
anomalies (negative field values, out-of-bounds ranges, non-ASCII names) and
collect [`ParseWarning`] values instead of aborting.
"#]

//! The current milestone implements real header and directory parsing plus typed
//! scaffolding for the classic map record lumps.

mod error;
pub mod map;
#[cfg(feature = "mmap")]
mod mmap;

use std::fs;
use std::io::Cursor;
use std::ops::Deref;
use std::path::Path;

use binrw::{BinRead, BinReaderExt};
#[cfg(feature = "mmap")]
use memmap2::Mmap;

pub use error::{ParseError, ParseWarning};

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

/// Controls how aggressively the parser validates malformed input.
///
/// Use `Strict` (the default) when loading files you expect to be well-formed
/// — it surfaces problems immediately rather than silently producing a partial
/// result.  Use `Lenient` when you need to inspect or salvage WADs that violate
/// the spec, such as files produced by buggy editors or heavily modified game
/// builds.  In lenient mode the parser emits [`ParseWarning`] values instead
/// of aborting on recoverable anomalies; consult [`Wad::warnings()`] after
/// loading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strictness {
    /// Abort on the first validation error and return it as [`ParseError`].
    ///
    /// Choose this when loading files that are expected to be spec-compliant.
    /// It is the default and is equivalent to `ParseOptions::default()`.
    Strict,
    /// Attempt best-effort recovery and accumulate [`ParseWarning`] values.
    ///
    /// Recoverable conditions (negative header fields, out-of-bounds lump
    /// ranges, non-ASCII lump names) are clamped or decoded lossily and
    /// recorded as warnings accessible via [`Wad::warnings()`].  Parsing
    /// only fails if the underlying byte stream is fundamentally unreadable.
    Lenient,
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
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            strictness: Strictness::Strict,
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

#[derive(Debug, BinRead)]
#[br(little)]
struct RawHeader {
    magic: [u8; 4],
    numlumps: i32,
    infotableofs: i32,
}

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
    /// `SIGBUS` on the next lump data access.  On Windows the mapping prevents
    /// truncation but concurrent writes by another process may expose
    /// inconsistent data.  Reading the file via this crate is always safe.
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
    #[must_use]
    pub fn lump_by_name(&self, name: &str) -> Option<&Lump> {
        self.lumps.iter().find(|lump| lump.name() == name)
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
    let end = bytes
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(bytes.len());
    let trimmed = &bytes[..end];
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
