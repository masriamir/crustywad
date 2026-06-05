//! Placeholder support for the future `mmap` feature.
//!
//! The current implementation deliberately falls back to ordinary file reads so the
//! public feature flag and module wiring can stabilize before any memory-mapped I/O
//! is introduced.

use std::fs;
use std::path::Path;

use crate::ParseError;

/// Reads a file into memory while the real memory-mapped backend is still pending.
pub(crate) fn read(path: &Path) -> Result<Vec<u8>, ParseError> {
    fs::read(path).map_err(|source| ParseError::Io {
        path: path.display().to_string(),
        source,
    })
}
