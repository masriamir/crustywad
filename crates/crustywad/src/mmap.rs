//! Memory-mapped I/O backend for the `mmap` feature.
#![allow(unsafe_code)]

use std::fs::File;
use std::path::Path;

use memmap2::{Mmap, MmapOptions};

use crate::ParseError;

/// Opens `path` as a read-only memory-mapped file.
///
/// # Errors
///
/// Returns [`ParseError::Io`] if the file cannot be opened or mapped.
pub(crate) fn open(path: &Path) -> Result<Mmap, ParseError> {
    let file = File::open(path).map_err(|source| ParseError::Io {
        path: path.display().to_string(),
        source,
    })?;
    // SAFETY: the Mmap is held for the lifetime of the owning Wad; we never
    // modify or truncate the file while the mapping is live.
    unsafe { MmapOptions::new().map(&file) }.map_err(|source| ParseError::Io {
        path: path.display().to_string(),
        source,
    })
}
