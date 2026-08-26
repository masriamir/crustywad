//! Error and warning types for WAD parsing.
//!
//! [`ParseError`] is returned by all fallible loading functions when parsing
//! cannot continue.  [`ParseWarning`] is collected during lenient parsing (see
//! [`ParseOptions::lenient()`][crate::ParseOptions::lenient]) for conditions
//! that were recovered from without aborting.

use thiserror::Error;

/// Flattens line breaks to spaces and strips raw ESC bytes from a string
/// that is interpolated into a `Display` message.
///
/// This keeps the enum-level single-line/no-escape guarantee true even when
/// the interpolated content comes from outside the crate (an OS error string,
/// a caller-supplied path).
pub(crate) fn flatten_control(s: &str) -> String {
    s.replace(['\n', '\r'], " ").replace('\u{1b}', "")
}

/// Renders a [`binrw::Error`] as a concise single-line phrase for `Display`.
///
/// `binrw`'s own `Display` for its `Backtrace` variant is a multi-line report
/// with box-drawing characters, optional ANSI escapes, and machine-local
/// source paths, so it must never be interpolated into a `ParseError`
/// message (#416).  Instead the error's root cause is mapped to a short
/// phrase, and any embedded line breaks are flattened to spaces.
fn concise_binrw(err: &binrw::Error) -> String {
    // `root_cause()` unwraps a `Backtrace` to the error that caused it and is
    // guaranteed to never return the `Backtrace` variant itself.
    let msg = match err.root_cause() {
        binrw::Error::Io(io_err) if io_err.kind() == std::io::ErrorKind::UnexpectedEof => {
            "unexpected end of input".to_owned()
        }
        binrw::Error::Io(io_err) => format!("I/O error: {io_err}"),
        binrw::Error::BadMagic { pos, .. } => format!("bad magic at offset 0x{pos:x}"),
        binrw::Error::AssertFail { pos, message } => format!("{message} at offset 0x{pos:x}"),
        binrw::Error::Custom { pos, .. } => format!("custom parser error at offset 0x{pos:x}"),
        binrw::Error::NoVariantMatch { pos } | binrw::Error::EnumErrors { pos, .. } => {
            format!("no matching variant at offset 0x{pos:x}")
        }
        // `binrw::Error` is `#[non_exhaustive]`; `Backtrace` also lands here
        // in the (normally impossible) case `root_cause()` ever returned one.
        _ => "binary read error".to_owned(),
    };
    flatten_control(&msg)
}

/// Errors that can occur while reading a WAD.
///
/// In lenient mode the parser recovers from several of these conditions and
/// records a [`ParseWarning`] instead — only truly unrecoverable failures
/// (like an unreadable file or a truncated header) still return `ParseError`.
///
/// To handle errors programmatically, match on the variant you care about and
/// fall back to the `Display` message for logging or user-facing output.
/// Every `Display` message is a single line with no terminal escape
/// sequences — input-derived content (such as the magic bytes) is escaped or
/// flattened — so it is safe to surface directly in CLI output, logs, or UIs.
#[derive(Debug, Error)]
pub enum ParseError {
    /// The file could not be read from disk.
    ///
    /// This is the first error checked when loading from a path.  Common causes
    /// are a missing file, insufficient permissions, or an I/O failure on the
    /// underlying storage device.  The `source` field contains the underlying
    /// [`std::io::Error`] from the OS.
    #[error("failed to read `{}`: {}", flatten_control(.path), flatten_control(&.source.to_string()))]
    Io {
        /// The file path that could not be read, as a display string.
        path: String,
        /// The underlying I/O error from the OS.
        #[source]
        source: std::io::Error,
    },
    /// The WAD header could not be decoded.
    ///
    /// The header occupies the first 12 bytes of the file.  This error means
    /// the buffer is shorter than 12 bytes or `binrw` failed to read the fixed
    /// fields.  The buffer likely does not contain a WAD at all.
    ///
    /// The `Display` message summarizes the underlying read failure in a
    /// single line (for example `unexpected end of input`); the full
    /// [`binrw::Error`] diagnostic remains available through
    /// [`std::error::Error::source`].
    #[error("failed to parse WAD header: {}", concise_binrw(.0))]
    Header(#[source] binrw::Error),
    /// A lump directory entry could not be decoded.
    ///
    /// Each directory entry is exactly 16 bytes.  This error fires if `binrw`
    /// cannot read entry number `index` — for example because the buffer was
    /// truncated mid-entry.  Check `index` to identify which lump was affected.
    ///
    /// The `Display` message summarizes the underlying read failure in a
    /// single line; the full [`binrw::Error`] diagnostic remains available
    /// through [`std::error::Error::source`].
    #[error("failed to parse WAD directory entry {index}: {}", concise_binrw(.source))]
    Directory {
        /// The zero-based index of the directory entry that could not be decoded.
        index: usize,
        /// The underlying binary read error from `binrw`.
        #[source]
        source: binrw::Error,
    },
    /// The first 4 bytes of the WAD are not a recognized magic value.
    ///
    /// Valid WADs start with either `"IWAD"` or `"PWAD"` (ASCII, no NUL
    /// terminator).  Any other value is rejected in strict mode.  Switch to
    /// lenient mode ([`ParseOptions::lenient()`][crate::ParseOptions::lenient])
    /// if you need to inspect files with non-standard magic bytes.
    ///
    /// In the `Display` message the magic is rendered with control and other
    /// non-printable characters escaped (e.g. `\u{1b}` for ESC); the `magic`
    /// field itself keeps the unescaped lossy string.
    #[error("invalid WAD magic `{}`", .magic.escape_debug())]
    InvalidMagic {
        /// The invalid 4-byte magic field rendered as a lossy UTF-8 string.
        magic: String,
    },
    /// A signed header or directory field contained a negative value where only
    /// non-negative values are meaningful.
    ///
    /// The WAD format stores `numlumps` and `infotableofs` as `i32`, but both
    /// must be non-negative to be useful.  A negative `filepos` or `size` in a
    /// directory entry triggers this error for that entry.  In lenient mode the
    /// value is clamped to `0` and a [`ParseWarning::NegativeValue`] is
    /// recorded instead.
    #[error("negative value {value} for `{field}`")]
    NegativeValue {
        /// The name of the header or directory field that held the negative
        /// value (e.g. `"numlumps"`, `"infotableofs"`, `"filepos"`, `"size"`).
        field: &'static str,
        /// The raw negative `i32` read from the WAD.
        value: i32,
    },
    /// A header offset, directory position, or lump range exceeded the
    /// available buffer.
    ///
    /// This error fires when:
    /// - `infotableofs` points past the end of the buffer (`field = "directory"`), or
    /// - a lump's `filepos + size` range exceeds the buffer (`field = "lump data"`).
    ///
    /// In lenient mode the range is clamped to the buffer boundary and a
    /// [`ParseWarning::OutOfBounds`] is recorded instead.
    #[error("{field} points outside the WAD buffer (offset {offset}, size {size}, len {len})")]
    OutOfBounds {
        /// A short description of the range that is out of bounds
        /// (e.g. `"directory"` or `"lump data"`).
        field: &'static str,
        /// The byte offset of the start of the range.
        offset: usize,
        /// The byte size of the range.
        size: usize,
        /// The total length of the WAD buffer.
        len: usize,
    },
    /// A lump name contained bytes outside the ASCII range.
    ///
    /// The Doom WAD spec requires lump names to be ASCII.  When a non-ASCII
    /// byte is encountered in strict mode the lump is rejected outright.  In
    /// lenient mode the name is decoded lossily (replacing invalid bytes with
    /// U+FFFD) and a [`ParseWarning::NonAsciiName`] is recorded instead.
    #[error("lump `{index}` contains a non-ASCII name")]
    NonAsciiName {
        /// The zero-based index of the lump whose name contained non-ASCII
        /// bytes.
        index: usize,
    },
    /// A numeric calculation overflowed while validating the WAD.
    ///
    /// This is a defense-in-depth check for pathological inputs — for example
    /// a `numlumps` value large enough that multiplying it by 16 (bytes per
    /// directory entry) would overflow a `usize`.  In lenient mode the
    /// calculation is saturated and a [`ParseWarning::Overflow`] is recorded
    /// instead.
    #[error("numeric overflow while validating `{field}`")]
    Overflow {
        /// The field or derived value that overflowed
        /// (e.g. `"directory length"`, `"lump range"`).
        field: &'static str,
    },
}

/// Non-fatal warnings reported by lenient parsing.
///
/// Each variant corresponds to a recoverable anomaly that lenient mode handles
/// without aborting.  After loading with [`ParseOptions::lenient()`][crate::ParseOptions::lenient],
/// inspect [`Wad::warnings()`][crate::Wad::warnings] to see what the parser
/// found and how it recovered.
///
/// In strict mode the parser returns a [`ParseError`] for the equivalent
/// condition rather than a warning.
///
/// As with [`ParseError`], every `Display` message is a single line with no
/// terminal escape sequences and is safe to surface directly.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ParseWarning {
    /// The header magic was not `"IWAD"` or `"PWAD"`.
    ///
    /// The parser preserved the raw 4-byte magic in [`WadKind::Unknown`][crate::WadKind::Unknown]
    /// and continued parsing the rest of the header.  The resulting [`Wad`][crate::Wad]
    /// may or may not contain usable data.
    ///
    /// In the `Display` message the magic is rendered with control and other
    /// non-printable characters escaped; the contained string keeps the
    /// unescaped lossy value.
    #[error("unrecognized WAD magic `{}`", .0.escape_debug())]
    InvalidMagic(String),
    /// A signed header or directory field was negative and was clamped to zero.
    ///
    /// The parser treated the field as `0` and continued.  A clamped
    /// `numlumps` means no lumps will be parsed; a clamped `filepos` or `size`
    /// means the affected lump will appear to have zero size at offset `0`.
    #[error("negative value {value} for `{field}`; clamped to 0")]
    NegativeValue {
        /// The field that held the negative value.
        field: &'static str,
        /// The raw negative value from the WAD.
        value: i32,
    },
    /// A directory or lump byte range exceeded the buffer and was clamped.
    ///
    /// When the lump directory starts past the end of the buffer, the parser
    /// reduces the lump count to only those entries that fit.  When an
    /// individual lump's range extends past the buffer, `filepos` and `size`
    /// are clamped so that `filepos + size <= buffer.len()`.
    #[error(
        "{field} points outside the WAD buffer (offset {offset}, size {size}, len {len}); truncated"
    )]
    OutOfBounds {
        /// A description of the range that was clamped.
        field: &'static str,
        /// The original byte offset before clamping.
        offset: usize,
        /// The original byte size before clamping.
        size: usize,
        /// The total length of the WAD buffer.
        len: usize,
    },
    /// A lump name contained non-ASCII bytes and was decoded lossily.
    ///
    /// Non-ASCII bytes in the 8-byte name field are replaced with U+FFFD
    /// (the Unicode replacement character) in the decoded [`String`].  The
    /// lump is still accessible by its lossy name via
    /// [`Wad::lump_by_name()`][crate::Wad::lump_by_name].
    #[error("lump `{index}` has a non-ASCII name; decoded lossily")]
    NonAsciiName {
        /// The zero-based index of the affected lump.
        index: usize,
    },
    /// A validation calculation overflowed and was saturated.
    ///
    /// The overflowing value was replaced with [`usize::MAX`] so that
    /// subsequent boundary checks treat it as unreachably large.  This is
    /// only triggered by crafted or corrupted WADs; real-world WADs never
    /// produce values large enough to overflow.
    #[error("numeric overflow while validating `{field}`; saturated")]
    Overflow {
        /// The field or derived value that overflowed.
        field: &'static str,
    },
}
