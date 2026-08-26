//! Error and warning types for archive (pk3) reading.
//!
//! [`ArchiveError`] is returned by every fallible [`Archive`][super::Archive]
//! operation; [`ArchiveWarning`] is collected during lenient opening for
//! conditions recovered from without aborting. As with
//! [`ParseError`][crate::ParseError], every `Display` message is a single
//! line with no terminal escape sequences and names the member it concerns.

use thiserror::Error;

use super::{ContainerKind, Method};
use crate::ParseError;
use crate::error::flatten_control;

/// A fatal archive-level failure.
///
/// Facts the central directory reveals (methods, flags, declared sizes,
/// names) are reported by [`Archive::from_bytes`][super::Archive::from_bytes]
/// in strict mode; facts only extraction reveals (local-header mismatch,
/// inflate failure, size or CRC lies) are reported by
/// [`Archive::read`][super::Archive::read] in both modes (ADR-0031 §6).
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ArchiveError {
    /// The file could not be read from disk.
    #[error("failed to read `{}`: {}", flatten_control(.path), flatten_control(&.source.to_string()))]
    Io {
        /// The file path that could not be read, as a display string.
        path: String,
        /// The underlying I/O error from the OS.
        #[source]
        source: std::io::Error,
    },
    /// The bytes carry no recognized archive signature, or a zip signature
    /// with no end-of-central-directory record in the last 65,557 bytes.
    #[error(
        "not an archive: no zip local-header signature, or no end-of-central-directory record behind one"
    )]
    NotAnArchive,
    /// A container format this crate recognizes but does not decode.
    #[error("{0} archives are not supported yet")]
    UnsupportedContainer(ContainerKind),
    /// The bytes begin with the empty-archive signature `PK\x05\x06`.
    #[error("empty archive: the zip contains no members")]
    EmptyArchive,
    /// The bytes begin with the spanned-archive signature `PK\x07\x08`.
    #[error("spanned (multi-part) archives are not supported")]
    SpannedArchive,
    /// A central-directory or local-header structure is inconsistent with
    /// the buffer (`index` is the zero-based entry number).
    #[error("corrupt archive directory at entry {index}: {reason}")]
    CorruptDirectory {
        /// Zero-based central-directory entry index.
        index: usize,
        /// What was inconsistent.
        reason: &'static str,
    },
    /// The central directory declares more members than
    /// [`Limits::max_archive_members`][crate::Limits::max_archive_members].
    #[error("archive declares {declared} members, more than the limit of {limit}")]
    TooManyMembers {
        /// The declared entry count.
        declared: u64,
        /// The configured limit.
        limit: usize,
    },
    /// The member uses a compression method other than stored or deflate.
    #[error("member `{}` uses unsupported compression method {method}", flatten_control(.path))]
    UnsupportedMethod {
        /// The member path.
        path: String,
        /// The method the central directory records.
        method: Method,
    },
    /// The member is encrypted (general-purpose flag bit 0).
    #[error("member `{}` is encrypted; encrypted members are not supported", flatten_control(.path))]
    Encrypted {
        /// The member path.
        path: String,
    },
    /// The member declares (or decodes to) more than
    /// [`Limits::max_decoded_member_bytes`][crate::Limits::max_decoded_member_bytes].
    #[error("member `{}` declares {declared} decoded bytes, more than the limit of {limit}", flatten_control(.path))]
    MemberTooLarge {
        /// The member path.
        path: String,
        /// The declared uncompressed size.
        declared: u64,
        /// The configured limit.
        limit: usize,
    },
    /// The decoded length did not match the declared size. `actual` is
    /// `None` when the stream held *more* than the declared size (decoding
    /// stopped at the declared length).
    #[error("member `{}` decoded to {} bytes, expected {declared}", flatten_control(.path), .actual.map_or_else(|| "more than the declared".to_string(), |n| n.to_string()))]
    SizeMismatch {
        /// The member path.
        path: String,
        /// The declared uncompressed size.
        declared: u64,
        /// The decoded length, or `None` when it exceeded `declared`.
        actual: Option<u64>,
    },
    /// The deflate stream could not be decoded.
    #[error("member `{}` has a corrupt deflate stream", flatten_control(.path))]
    CorruptStream {
        /// The member path.
        path: String,
    },
    /// The decoded bytes do not match the central directory's CRC-32.
    #[error("member `{}` failed its CRC-32 check", flatten_control(.path))]
    ChecksumMismatch {
        /// The member path.
        path: String,
    },
    /// A member path contains non-ASCII bytes (strict mode only; lenient
    /// records [`ArchiveWarning::NonAsciiName`]).
    #[error("member path `{}` is not ASCII", flatten_control(.path))]
    NonAsciiName {
        /// The member path, decoded lossily.
        path: String,
    },
    /// The [`Member`][super::Member] was obtained from a different
    /// [`Archive`][super::Archive]; members are only valid against the
    /// archive that produced them.
    #[error("member `{}` does not belong to this archive", flatten_control(.path))]
    ForeignMember {
        /// The member path.
        path: String,
    },
    /// [`Archive::wad`][super::Archive::wad] was called on a member that is
    /// not a `.wad` file.
    #[error("member `{}` is not a WAD", flatten_control(.path))]
    NotAWad {
        /// The member path.
        path: String,
    },
    /// The member is a WAD but failed to parse.
    #[error("member `{}`: {source}", flatten_control(.path))]
    Wad {
        /// The member path.
        path: String,
        /// The WAD parse failure.
        #[source]
        source: ParseError,
    },
}

/// A non-fatal condition recorded while opening an archive in lenient mode.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ArchiveWarning {
    /// The member is listed but [`Archive::read`][super::Archive::read] will
    /// fail on it (unsupported method or encryption).
    #[error("member `{}` cannot be read: {}", flatten_control(.path), flatten_control(.reason))]
    UnreadableMember {
        /// The member path.
        path: String,
        /// Why reading will fail.
        reason: String,
    },
    /// The member declares more than the decoded-size limit;
    /// [`read`][super::Archive::read] will fail.
    #[error("member `{}` declares {declared} decoded bytes, more than the limit of {limit}", flatten_control(.path))]
    MemberTooLarge {
        /// The member path.
        path: String,
        /// The declared uncompressed size.
        declared: u64,
        /// The configured limit.
        limit: usize,
    },
    /// The member path is not ASCII; it has no short name.
    #[error("member path `{}` is not ASCII; no short name derived", flatten_control(.path))]
    NonAsciiName {
        /// The member path, decoded lossily.
        path: String,
    },
    /// Two members share a path; the later one wins lookups.
    #[error("member path `{}` appears more than once; the later entry wins lookups", flatten_control(.path))]
    DuplicatePath {
        /// The duplicated path.
        path: String,
    },
}
