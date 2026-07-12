//! UDMF (`TEXTMAP`) text-map parsing (ADR-0017).
//!
//! [`parse_udmf`] lexes and parses UDMF text into a typed [`UdmfMap`]. This is
//! a lexical + grammatical + per-field-default pass only; cross-reference
//! resolution into [`Map`][crate::map::Map] is map assembly (a later pass).

mod lex;
mod model;
mod parse;

pub use model::{UdmfLinedef, UdmfMap, UdmfSector, UdmfSidedef, UdmfThing, UdmfVertex};
pub use parse::parse_udmf;

use thiserror::Error;

/// An error raised while parsing UDMF `TEXTMAP` text.
///
/// The enum is `#[non_exhaustive]`: map assembly (a later pass) reads `TEXTMAP`
/// as raw lump bytes and will add a UTF-8-decoding variant for that byte
/// entrypoint. [`parse_udmf`] itself takes an already-decoded `&str`, so it
/// never reports an encoding error.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum UdmfParseError {
    /// The `TEXTMAP` bytes are not valid UTF-8.
    #[error("TEXTMAP lump is not valid UTF-8 at byte offset {offset}")]
    InvalidEncoding {
        /// The byte offset of the first invalid UTF-8 sequence.
        offset: usize,
    },
    /// A lexical or grammatical error at a specific source position.
    #[error("syntax error at line {line}, column {column}: {message}")]
    Syntax {
        /// 1-based source line of the error.
        line: usize,
        /// 1-based source column of the error.
        column: usize,
        /// Human-readable description of what was expected or invalid.
        message: String,
    },
    /// Block nesting exceeded the configured `max_depth`.
    #[error(
        "nesting depth exceeded the configured limit ({max_depth}) at line {line}, column {column}"
    )]
    DepthExceeded {
        /// The configured maximum nesting depth.
        max_depth: usize,
        /// 1-based source line where the limit was exceeded.
        line: usize,
        /// 1-based source column where the limit was exceeded.
        column: usize,
    },
    /// A block omitted a field with no valid spec default, or the document
    /// lacked a `namespace` declaration.
    #[error("semantic error: {message}")]
    Semantic {
        /// Human-readable description of the missing field or declaration.
        message: String,
    },
}

/// Decodes `TEXTMAP` lump bytes as UTF-8.
///
/// # Errors
/// Returns [`UdmfParseError::InvalidEncoding`] if `bytes` is not valid UTF-8,
/// carrying the byte offset of the first invalid sequence.
#[allow(dead_code)]
pub(crate) fn decode_textmap(bytes: &[u8]) -> Result<&str, UdmfParseError> {
    std::str::from_utf8(bytes)
        .map_err(|e| UdmfParseError::InvalidEncoding { offset: e.valid_up_to() })
}

#[cfg(test)]
mod tests {
    use super::{UdmfParseError, decode_textmap};

    #[test]
    fn decode_textmap_roundtrips_and_rejects_non_utf8() {
        assert_eq!(decode_textmap(b"namespace = \"doom\";").unwrap(), "namespace = \"doom\";");
        assert_eq!(
            decode_textmap(&[b'a', 0xFF]).unwrap_err(),
            UdmfParseError::InvalidEncoding { offset: 1 }
        );
    }
}
