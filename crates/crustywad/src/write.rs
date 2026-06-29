//! WAD write support — builder pattern serialization.
//!
//! This module is gated behind the `write` feature flag.
//!
//! # Overview
//!
//! Use [`WadBuilder`] to construct a new WAD byte buffer from scratch, or call
//! [`Wad::to_builder()`][crate::Wad::to_builder] to edit a parsed WAD and
//! re-serialize it.
//!
//! ```
//! use crustywad::{WadBuilder, WadKind};
//!
//! let bytes = WadBuilder::new(WadKind::Pwad)
//!     .add_lump("MAP01", b"")
//!     .build()
//!     .unwrap();
//! assert!(crustywad::Wad::from_bytes(bytes).is_ok());
//! ```

use std::io::Cursor;

use binrw::BinWrite as _;

use crate::{RawDirectoryEntry, RawHeader, Strictness, WadKind};

/// Errors that can occur while building a WAD.
///
/// All variants are returned from [`WadBuilder::build`] and
/// [`WadBuilder::build_with_options`].
#[derive(Debug, thiserror::Error)]
pub enum WriteError {
    /// A lump name contains a NUL byte, which would be silently truncated on re-parse.
    #[error("lump name {name:?} contains a NUL byte")]
    NulInName {
        /// The offending lump name.
        name: String,
    },
    /// A lump name contains non-ASCII bytes.
    #[error("lump name {name:?} contains non-ASCII bytes")]
    NonAsciiName {
        /// The offending lump name.
        name: String,
    },
    /// A lump name is longer than 8 bytes in strict mode.
    #[error("lump name {name:?} is {len} bytes; WAD names are at most 8 bytes")]
    NameTooLong {
        /// The offending lump name.
        name: String,
        /// The actual byte length of the name.
        len: usize,
    },
    /// A lump data payload exceeds `i32::MAX` bytes.
    #[error("lump {name:?} data size {size} exceeds i32::MAX")]
    LumpTooLarge {
        /// The name of the offending lump.
        name: String,
        /// The actual size of the lump data.
        size: usize,
    },
    /// The total lump count exceeds `i32::MAX`.
    #[error("lump count {count} exceeds i32::MAX")]
    TooManyLumps {
        /// The actual lump count.
        count: usize,
    },
    /// A computed byte offset exceeds `i32::MAX`.
    #[error("computed offset {offset} exceeds i32::MAX")]
    OffsetOverflow {
        /// The computed offset that overflowed.
        offset: usize,
    },
    /// [`WadKind::Unknown`] magic is not permitted in strict mode.
    #[error("WadKind::Unknown is not permitted in strict mode")]
    UnknownMagicStrict,
    /// A `binrw` serialization error.
    #[error("serialization error: {0}")]
    Binrw(#[from] binrw::Error),
}

/// Non-fatal conditions encountered during lenient WAD building.
///
/// Warnings are returned alongside the serialized bytes from
/// [`WadBuilder::build_with_options`] when
/// [`WriteOptions::strictness`] is [`Strictness::Lenient`].
#[derive(Debug, thiserror::Error)]
pub enum WriteWarning {
    /// A lump name longer than 8 bytes was truncated to fit the WAD name field.
    #[error("lump name {name:?} was truncated to 8 bytes")]
    NameTruncated {
        /// The original (un-truncated) lump name.
        name: String,
    },
}

/// Options controlling write-time validation behavior.
///
/// # Examples
///
/// ```
/// use crustywad::WriteOptions;
///
/// let strict = WriteOptions::strict();
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
    /// Strict validation — any invalid input returns an error immediately.
    ///
    /// This is the default.
    #[must_use]
    pub const fn strict() -> Self {
        Self {
            strictness: Strictness::Strict,
        }
    }

    /// Lenient validation — recoverable issues produce [`WriteWarning`]s rather than errors.
    #[must_use]
    pub const fn lenient() -> Self {
        Self {
            strictness: Strictness::Lenient,
        }
    }
}

struct LumpEntry {
    name: String,
    data: Vec<u8>,
}

/// A WAD builder. Accumulates lumps and serializes to `Vec<u8>` on [`build`][Self::build].
///
/// Construct with [`WadBuilder::new`], add lumps with [`add_lump`][Self::add_lump],
/// then call [`build`][Self::build] for strict serialization or
/// [`build_with_options`][Self::build_with_options] for lenient mode.
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
/// assert!(crustywad::Wad::from_bytes(bytes).is_ok());
/// ```
pub struct WadBuilder {
    kind: WadKind,
    lumps: Vec<LumpEntry>,
}

impl WadBuilder {
    /// Creates a new empty builder for a WAD of the given kind.
    #[must_use]
    pub fn new(kind: WadKind) -> Self {
        Self {
            kind,
            lumps: Vec::new(),
        }
    }

    /// Appends a lump with the given name and data payload.
    ///
    /// Validation (name length, ASCII, NUL bytes, data size) is deferred to
    /// [`build`][Self::build] or [`build_with_options`][Self::build_with_options].
    pub fn add_lump(&mut self, name: &str, data: impl Into<Vec<u8>>) -> &mut Self {
        self.lumps.push(LumpEntry {
            name: name.to_owned(),
            data: data.into(),
        });
        self
    }

    /// Serializes the WAD to bytes using strict validation.
    ///
    /// # Errors
    ///
    /// Returns [`WriteError`] if any lump name or payload violates WAD format
    /// constraints:
    ///
    /// - [`WriteError::NulInName`] — a name contains a NUL byte (both modes).
    /// - [`WriteError::NonAsciiName`] — a name contains non-ASCII bytes (both modes).
    /// - [`WriteError::NameTooLong`] — a name is longer than 8 bytes (strict only).
    /// - [`WriteError::LumpTooLarge`] — a lump data size exceeds `i32::MAX` (both modes).
    /// - [`WriteError::TooManyLumps`] — the lump count exceeds `i32::MAX` (both modes).
    /// - [`WriteError::OffsetOverflow`] — a computed byte offset exceeds `i32::MAX` (both modes).
    /// - [`WriteError::UnknownMagicStrict`] — [`WadKind::Unknown`] in strict mode.
    /// - [`WriteError::Binrw`] — a `binrw` serialization failure.
    pub fn build(&self) -> Result<Vec<u8>, WriteError> {
        self.build_with_options(&WriteOptions::strict())
            .map(|(bytes, _)| bytes)
    }

    /// Serializes the WAD to bytes with the given [`WriteOptions`].
    ///
    /// Returns the serialized bytes and any non-fatal [`WriteWarning`]s collected
    /// in lenient mode.
    ///
    /// # Errors
    ///
    /// Returns [`WriteError`] for unrecoverable validation failures regardless of
    /// [`WriteOptions::strictness`]. See [`build`][Self::build] for the full list
    /// of error variants; in lenient mode [`WriteError::NameTooLong`] and
    /// [`WriteError::UnknownMagicStrict`] are replaced by [`WriteWarning::NameTruncated`]
    /// and a raw-bytes write respectively.
    ///
    /// # Panics
    ///
    /// Does not panic. The internal `expect` calls on `i32::try_from` are
    /// preceded by explicit bounds checks that return [`WriteError`] before any
    /// overflow can reach them.
    pub fn build_with_options(
        &self,
        opts: &WriteOptions,
    ) -> Result<(Vec<u8>, Vec<WriteWarning>), WriteError> {
        let lenient = matches!(opts.strictness, Strictness::Lenient);
        let mut warnings: Vec<WriteWarning> = Vec::new();

        // Validate magic kind.
        let magic: [u8; 4] = match self.kind {
            WadKind::Iwad => *b"IWAD",
            WadKind::Pwad => *b"PWAD",
            WadKind::Unknown(b) => {
                if lenient {
                    b
                } else {
                    return Err(WriteError::UnknownMagicStrict);
                }
            }
        };

        // Validate lump count fits i32.
        if self.lumps.len() > i32::MAX as usize {
            return Err(WriteError::TooManyLumps {
                count: self.lumps.len(),
            });
        }

        // Validate and encode each lump name; validate data sizes.
        let mut encoded_names: Vec<[u8; 8]> = Vec::with_capacity(self.lumps.len());
        for entry in &self.lumps {
            if entry.name.contains('\0') {
                return Err(WriteError::NulInName {
                    name: entry.name.clone(),
                });
            }
            if !entry.name.is_ascii() {
                return Err(WriteError::NonAsciiName {
                    name: entry.name.clone(),
                });
            }
            let name_bytes = entry.name.as_bytes();
            let mut buf = [0u8; 8];
            if name_bytes.len() > 8 {
                if lenient {
                    warnings.push(WriteWarning::NameTruncated {
                        name: entry.name.clone(),
                    });
                    buf.copy_from_slice(&name_bytes[..8]);
                } else {
                    return Err(WriteError::NameTooLong {
                        name: entry.name.clone(),
                        len: name_bytes.len(),
                    });
                }
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

        // Compute filepos for each lump.
        // Layout: [12-byte header][lump data blobs][16-byte directory entries]
        let mut filepos_list: Vec<usize> = Vec::with_capacity(self.lumps.len());
        let mut offset: usize = 12; // 12-byte header
        for entry in &self.lumps {
            if offset > i32::MAX as usize {
                return Err(WriteError::OffsetOverflow { offset });
            }
            filepos_list.push(offset);
            offset = offset
                .checked_add(entry.data.len())
                .ok_or(WriteError::OffsetOverflow { offset })?;
        }
        let infotableofs = offset;
        if infotableofs > i32::MAX as usize {
            return Err(WriteError::OffsetOverflow {
                offset: infotableofs,
            });
        }

        // Allocate buffer and serialize.
        let data_size: usize = self.lumps.iter().map(|e| e.data.len()).sum();
        let capacity = 12 + data_size + self.lumps.len() * 16;
        let mut buf: Cursor<Vec<u8>> = Cursor::new(Vec::with_capacity(capacity));

        // Write the 12-byte header.
        let header = RawHeader {
            magic,
            numlumps: i32::try_from(self.lumps.len()).expect("validated: lump count fits i32"),
            infotableofs: i32::try_from(infotableofs).expect("validated: infotableofs fits i32"),
        };
        header.write_le(&mut buf)?;

        // Write lump data blobs.
        for entry in &self.lumps {
            // Use io::Write to advance the cursor position alongside the data.
            std::io::Write::write_all(&mut buf, &entry.data).expect("write to Vec is infallible");
        }

        // Write directory entries.
        for (i, entry) in self.lumps.iter().enumerate() {
            let dir = RawDirectoryEntry {
                filepos: i32::try_from(filepos_list[i]).expect("validated: filepos fits i32"),
                size: i32::try_from(entry.data.len()).expect("validated: lump size fits i32"),
                name: encoded_names[i],
            };
            dir.write_le(&mut buf)?;
        }

        Ok((buf.into_inner(), warnings))
    }
}
