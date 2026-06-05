#![forbid(unsafe_code)]
#![doc = r#"
`crustywad` is a small Rust library for safe Doom WAD parsing.

# Example

```rust
use crustywad::Wad;

let mut bytes = Vec::new();
bytes.extend_from_slice(b"IWAD");
bytes.extend_from_slice(&1_i32.to_le_bytes());
bytes.extend_from_slice(&12_i32.to_le_bytes());
bytes.extend_from_slice(&0_i32.to_le_bytes());
bytes.extend_from_slice(&0_i32.to_le_bytes());
bytes.extend_from_slice(b"TEST\0\0\0\0");

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

#[cfg(not(feature = "mmap"))]
use std::fs;
use std::io::Cursor;
use std::path::Path;

use binrw::{BinRead, BinReaderExt};

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
    /// The declared number of lump directory entries.
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

/// An owned Doom WAD loaded into memory.
#[derive(Debug, Clone)]
pub struct Wad {
    header: WadHeader,
    lumps: Vec<Lump>,
    bytes: Vec<u8>,
    warnings: Vec<ParseWarning>,
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
    /// # Errors
    ///
    /// Returns [`ParseError`] when the buffer does not contain a valid WAD
    /// according to the default strict parsing rules.
    pub fn from_bytes(bytes: impl AsRef<[u8]>) -> Result<Self, ParseError> {
        Self::from_bytes_with_options(bytes.as_ref().to_vec(), ParseOptions::default())
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
        let mut cursor = Cursor::new(bytes.as_slice());
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

        let num_lumps = coerce_i32(
            raw.numlumps,
            "numlumps",
            len,
            options.strictness,
            &mut warnings,
        )?;
        let info_table_offset = coerce_i32(
            raw.infotableofs,
            "infotableofs",
            len,
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
        let lump_count =
            if info_table_offset > len || info_table_offset.saturating_add(dir_span) > len {
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
                options.strictness,
                &mut warnings,
            )?);
        }

        Ok(Self {
            header,
            lumps,
            bytes,
            warnings,
        })
    }

    /// Reads a WAD from a file path using strict parsing.
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
    /// # Errors
    ///
    /// Returns [`ParseError`] if the file cannot be read or the WAD fails
    /// validation under the provided [`ParseOptions`].
    pub fn from_path_with_options(
        path: impl AsRef<Path>,
        options: ParseOptions,
    ) -> Result<Self, ParseError> {
        let path = path.as_ref();
        #[cfg(feature = "mmap")]
        let bytes = mmap::read(path)?;
        #[cfg(not(feature = "mmap"))]
        let bytes = fs::read(path).map_err(|source| ParseError::Io {
            path: path.display().to_string(),
            source,
        })?;
        Self::from_bytes_with_options(bytes, options)
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
    #[must_use]
    pub fn lump_data(&self, lump: &Lump) -> &[u8] {
        &self.bytes[lump.filepos..lump.filepos + lump.size]
    }

    /// Returns any non-fatal warnings produced during lenient parsing.
    #[must_use]
    pub fn warnings(&self) -> &[ParseWarning] {
        &self.warnings
    }

    /// Returns the owned bytes backing the WAD.
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

fn coerce_i32(
    value: i32,
    field: &'static str,
    len: usize,
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

    let coerced = usize::try_from(value).map_err(|_| ParseError::Overflow { field })?;
    if coerced > len && field == "infotableofs" {
        match strictness {
            Strictness::Strict => Err(ParseError::OutOfBounds {
                field,
                offset: coerced,
                size: 0,
                len,
            }),
            Strictness::Lenient => {
                warnings.push(ParseWarning::OutOfBounds {
                    field,
                    offset: coerced,
                    size: 0,
                    len,
                });
                Ok(coerced)
            }
        }
    } else {
        Ok(coerced)
    }
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
    strictness: Strictness,
    warnings: &mut Vec<ParseWarning>,
) -> Result<Lump, ParseError> {
    let filepos = coerce_i32(raw_entry.filepos, "filepos", len, strictness, warnings)?;
    let size = coerce_i32(raw_entry.size, "size", len, strictness, warnings)?;
    let name = decode_name(index, raw_entry.name, strictness, warnings)?;

    let end = filepos.checked_add(size).ok_or(ParseError::Overflow {
        field: "lump range",
    })?;
    let max_end = if filepos < directory_offset {
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

#[cfg(test)]
mod tests {
    use super::{ParseOptions, ParseWarning, Strictness, Wad, WadKind};
    use proptest::prelude::*;

    fn encode_i32(value: usize) -> [u8; 4] {
        i32::try_from(value)
            .expect("test fixture values should fit within i32")
            .to_le_bytes()
    }

    fn build_wad(kind: [u8; 4], lumps: &[(&str, &[u8])]) -> Vec<u8> {
        let mut data_section = Vec::new();
        let mut directory = Vec::new();
        let directory_offset = 12 + lumps.iter().map(|(_, data)| data.len()).sum::<usize>();

        for (name, bytes) in lumps {
            let filepos = 12 + data_section.len();
            data_section.extend_from_slice(bytes);
            directory.extend_from_slice(&encode_i32(filepos));
            directory.extend_from_slice(&encode_i32(bytes.len()));
            let mut name_bytes = [0_u8; 8];
            for (slot, byte) in name.as_bytes().iter().take(8).enumerate() {
                name_bytes[slot] = *byte;
            }
            directory.extend_from_slice(&name_bytes);
        }

        let mut wad = Vec::new();
        wad.extend_from_slice(&kind);
        wad.extend_from_slice(&encode_i32(lumps.len()));
        wad.extend_from_slice(&encode_i32(directory_offset));
        wad.extend_from_slice(&data_section);
        wad.extend_from_slice(&directory);
        wad
    }

    #[test]
    fn parses_basic_wad() {
        let wad = Wad::from_bytes(build_wad(*b"IWAD", &[("PLAYPAL", &[1, 2, 3])]))
            .expect("wad should parse");
        assert_eq!(wad.kind(), WadKind::Iwad);
        assert_eq!(wad.lump_count(), 1);
        assert_eq!(wad.lump_by_name("PLAYPAL").expect("missing lump").size(), 3);
        assert_eq!(wad.lump_bytes(0), Some(&[1, 2, 3][..]));
    }

    #[test]
    fn strict_mode_rejects_bad_magic() {
        let err = Wad::from_bytes(build_wad(*b"NOPE", &[])).expect_err("magic should fail");
        assert!(matches!(err, super::ParseError::InvalidMagic { .. }));
    }

    #[test]
    fn lenient_mode_collects_warnings() {
        let mut wad = build_wad(*b"NOPE", &[("TEST", &[1, 2, 3])]);
        wad[4..8].copy_from_slice(&1_i32.to_le_bytes());
        wad[8..12].copy_from_slice(&128_i32.to_le_bytes());
        let parsed = Wad::from_bytes_with_options(wad, ParseOptions::lenient())
            .expect("lenient parse should succeed");
        assert!(matches!(parsed.kind(), WadKind::Unknown(_)));
        assert!(
            parsed
                .warnings()
                .iter()
                .any(|warning| matches!(warning, ParseWarning::InvalidMagic(_)))
        );
        assert_eq!(parsed.lump_count(), 0);
    }

    #[test]
    fn strict_mode_rejects_non_ascii_names() {
        let mut wad = build_wad(*b"PWAD", &[("TEST", &[1])]);
        let name_offset = wad.len() - 8;
        wad[name_offset] = 0xFF;
        let err = Wad::from_bytes(wad).expect_err("non-ascii name should fail");
        assert!(matches!(err, super::ParseError::NonAsciiName { .. }));
    }

    #[test]
    fn lenient_mode_clamps_oversized_lumps() {
        let mut wad = build_wad(*b"PWAD", &[("TEST", &[1, 2, 3])]);
        let size_offset = wad.len() - 16;
        wad[size_offset + 4..size_offset + 8].copy_from_slice(&999_i32.to_le_bytes());
        let parsed = Wad::from_bytes_with_options(wad, ParseOptions::lenient())
            .expect("lenient parse should succeed");
        assert_eq!(parsed.lump_bytes(0), Some(&[1, 2, 3][..]));
        assert!(
            parsed
                .warnings()
                .iter()
                .any(|warning| matches!(warning, ParseWarning::OutOfBounds { .. }))
        );
    }

    proptest! {
        #[test]
        fn strict_parser_handles_generated_empty_wads(kind in prop_oneof![Just(*b"IWAD"), Just(*b"PWAD")]) {
            let wad = Wad::from_bytes(build_wad(kind, &[])).expect("generated wad should parse");
            prop_assert_eq!(wad.lump_count(), 0);
            prop_assert!(matches!(wad.kind(), WadKind::Iwad | WadKind::Pwad));
        }
    }

    #[test]
    fn parse_options_default_to_strict() {
        assert_eq!(ParseOptions::default().strictness, Strictness::Strict);
    }
}
