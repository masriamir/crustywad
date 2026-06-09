//! Error and warning types for WAD parsing.

use thiserror::Error;

/// Errors that can occur while reading a WAD.
#[derive(Debug, Error)]
pub enum ParseError {
    /// The file could not be read from disk.
    #[error("failed to read `{path}`: {source}")]
    Io {
        /// The path that failed to load.
        path: String,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// The WAD header could not be decoded.
    #[error("failed to parse WAD header: {0}")]
    Header(#[source] binrw::Error),
    /// The WAD directory could not be decoded.
    #[error("failed to parse WAD directory entry {index}: {source}")]
    Directory {
        /// The zero-based directory entry index.
        index: usize,
        /// The underlying binary read error.
        #[source]
        source: binrw::Error,
    },
    /// The WAD magic was not one of the known values.
    #[error("invalid WAD magic `{magic}`")]
    InvalidMagic {
        /// The invalid header magic rendered lossily as text.
        magic: String,
    },
    /// A signed header or directory field was negative when a non-negative value was required.
    #[error("negative value {value} for `{field}`")]
    NegativeValue {
        /// The field name.
        field: &'static str,
        /// The invalid value.
        value: i32,
    },
    /// A header offset or length exceeded the available buffer.
    #[error("{field} points outside the WAD buffer (offset {offset}, size {size}, len {len})")]
    OutOfBounds {
        /// The logical field being validated.
        field: &'static str,
        /// The offset in bytes.
        offset: usize,
        /// The size in bytes.
        size: usize,
        /// The total buffer length.
        len: usize,
    },
    /// A lump name was not valid ASCII.
    #[error("lump `{index}` contains a non-ASCII name")]
    NonAsciiName {
        /// The zero-based lump index.
        index: usize,
    },
    /// A numeric calculation overflowed while validating the WAD.
    #[error("numeric overflow while validating `{field}`")]
    Overflow {
        /// The field name.
        field: &'static str,
    },
}

/// Non-fatal warnings reported by lenient parsing.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ParseWarning {
    /// The header magic was not recognized.
    #[error("unrecognized WAD magic `{0}`")]
    InvalidMagic(String),
    /// A signed header or directory field was negative and was clamped to zero.
    #[error("negative value {value} for `{field}`; clamped to 0")]
    NegativeValue {
        /// The field name.
        field: &'static str,
        /// The invalid value.
        value: i32,
    },
    /// A directory or lump range exceeded the available bytes and was truncated.
    #[error(
        "{field} points outside the WAD buffer (offset {offset}, size {size}, len {len}); truncated"
    )]
    OutOfBounds {
        /// The logical field being validated.
        field: &'static str,
        /// The byte offset involved.
        offset: usize,
        /// The byte size involved.
        size: usize,
        /// The total buffer length.
        len: usize,
    },
    /// A lump name contained non-ASCII bytes and was decoded lossily.
    #[error("lump `{index}` has a non-ASCII name; decoded lossily")]
    NonAsciiName {
        /// The zero-based lump index.
        index: usize,
    },
    /// A validation calculation overflowed and was saturated.
    #[error("numeric overflow while validating `{field}`; saturated")]
    Overflow {
        /// The field name.
        field: &'static str,
    },
}
