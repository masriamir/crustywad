#![deny(unsafe_code)]
#![doc = r#"
`crustywad` is a small Rust library for safe Doom WAD parsing.

# Example

```rust
use crustywad::Wad;

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

/// The identified WAD variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WadKind {
    /// An IWAD file.
    Iwad,
    /// A PWAD file.
    Pwad,
    /// A non-standard or malformed magic value preserved in lenient mode.
    Unknown([u8; 4]),
}

/// Controls how aggressively the parser validates malformed input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strictness {
    /// Return the first validation error.
    Strict,
    /// Attempt best-effort recovery and accumulate warnings.
    Lenient,
}

/// Parser configuration.
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
    #[must_use]
    pub const fn strict() -> Self {
        Self {
            strictness: Strictness::Strict,
        }
    }

    /// Returns a lenient parser configuration.
    #[must_use]
    pub const fn lenient() -> Self {
        Self {
            strictness: Strictness::Lenient,
        }
    }
}

/// The parsed WAD header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WadHeader {
    /// The WAD file kind.
    pub kind: WadKind,
    /// The number of lump directory entries successfully parsed.
    ///
    /// In lenient mode this may be less than what the file header declares when
    /// the directory extends beyond the available bytes.
    pub num_lumps: usize,
    /// The byte offset of the lump directory.
    pub info_table_offset: usize,
}

/// A validated lump directory entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lump {
    name: String,
    filepos: usize,
    size: usize,
}

impl Lump {
    /// Returns the lump name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the byte offset within the WAD buffer.
    #[must_use]
    pub const fn filepos(&self) -> usize {
        self.filepos
    }

    /// Returns the validated byte length.
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

/// An owned Doom WAD loaded into memory.
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
    /// Parses a WAD from an owned or borrowed in-memory byte buffer.
    ///
    /// Accepts any type that can be converted into a `Vec<u8>` — including
    /// `Vec<u8>` (moved without copying), `&[u8]`, and fixed-size byte arrays.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] when the buffer does not contain a valid WAD
    /// according to the default strict parsing rules.
    pub fn from_bytes(bytes: impl Into<Vec<u8>>) -> Result<Self, ParseError> {
        Self::from_bytes_with_options(bytes.into(), ParseOptions::default())
    }

    /// Parses a WAD from an in-memory byte buffer using explicit parse options.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] when the bytes cannot be decoded or validated
    /// according to the supplied [`ParseOptions`].
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

    /// Reads a WAD from a file path using strict parsing.
    ///
    /// The file is read into memory in full. For large WADs where only a
    /// subset of lumps will be accessed, consider
    /// [`from_path_mapped`][Wad::from_path_mapped] (requires the `mmap`
    /// feature).
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the file cannot be read or the WAD fails
    /// strict validation.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, ParseError> {
        Self::from_path_with_options(path, ParseOptions::default())
    }

    /// Reads a WAD from a file path using explicit parse options.
    ///
    /// The file is read into memory in full. For large WADs where only a
    /// subset of lumps will be accessed, consider
    /// [`from_path_mapped_with_options`][Wad::from_path_mapped_with_options]
    /// (requires the `mmap` feature).
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the file cannot be read or the WAD fails
    /// validation under the provided [`ParseOptions`].
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

    /// Reads a WAD from a file path using memory-mapped I/O and strict
    /// parsing.
    ///
    /// The file is mapped read-only and the mapping is held for the lifetime
    /// of the returned [`Wad`] — no heap copy is made on load. This is more
    /// efficient than [`from_path`][Wad::from_path] for large WADs where only
    /// a subset of lumps will be accessed.
    ///
    /// **Warning:** truncating or modifying the file from another process
    /// while the [`Wad`] is alive is unsupported. On Unix this causes a
    /// `SIGBUS`; on Windows the mapping prevents truncation but concurrent
    /// writes may produce inconsistent data.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the file cannot be opened, mapped, or parsed.
    #[cfg(feature = "mmap")]
    pub fn from_path_mapped(path: impl AsRef<Path>) -> Result<Self, ParseError> {
        Self::from_path_mapped_with_options(path, ParseOptions::default())
    }

    /// Reads a WAD from a file path using memory-mapped I/O and explicit
    /// parse options.
    ///
    /// The file is mapped read-only and the mapping is held for the lifetime
    /// of the returned [`Wad`] — no heap copy is made on load.
    ///
    /// **Warning:** truncating or modifying the file from another process
    /// while the [`Wad`] is alive is unsupported. On Unix this causes a
    /// `SIGBUS`; on Windows the mapping prevents truncation but concurrent
    /// writes may produce inconsistent data.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the file cannot be opened, mapped, or parsed.
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

    /// Returns the WAD kind.
    #[must_use]
    pub const fn kind(&self) -> WadKind {
        self.header.kind
    }

    /// Returns the parsed header.
    #[must_use]
    pub const fn header(&self) -> &WadHeader {
        &self.header
    }

    /// Returns the number of parsed lumps.
    #[must_use]
    pub fn lump_count(&self) -> usize {
        self.lumps.len()
    }

    /// Returns the parsed lumps.
    #[must_use]
    pub fn lumps(&self) -> &[Lump] {
        &self.lumps
    }

    /// Returns a lump by zero-based index.
    #[must_use]
    pub fn lump(&self, index: usize) -> Option<&Lump> {
        self.lumps.get(index)
    }

    /// Returns the first lump with the requested name.
    #[must_use]
    pub fn lump_by_name(&self, name: &str) -> Option<&Lump> {
        self.lumps.iter().find(|lump| lump.name() == name)
    }

    /// Returns the raw bytes for the lump at the requested index.
    #[must_use]
    pub fn lump_bytes(&self, index: usize) -> Option<&[u8]> {
        self.lump(index)
            .map(|lump| &self.bytes[lump.filepos..lump.filepos + lump.size])
    }

    /// Returns the raw bytes for the provided lump metadata.
    ///
    /// # Panics
    ///
    /// Panics if `lump` does not belong to this [`Wad`] — for example if it was
    /// cloned from a different, larger WAD whose lump range falls outside this
    /// buffer.
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
    #[must_use]
    pub fn warnings(&self) -> &[ParseWarning] {
        &self.warnings
    }

    /// Returns the owned bytes backing the WAD.
    ///
    /// When the `mmap` feature is enabled and the WAD was loaded from a file,
    /// this copies the mapped bytes into a new heap allocation and releases the
    /// mapping.
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
