//! Memory-mapped I/O backend for the `mmap` feature.
#![allow(
    unsafe_code,
    reason = "sole unsafe boundary in the workspace: MmapOptions::map for read-only file mapping"
)]

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
    // SAFETY: `map` requires the file not be truncated while the Mmap lives —
    // truncation by another process would cause a SIGBUS on access. The file is
    // opened read-only (preventing truncation by this process), and the Mmap is
    // stored in the owning Wad so it lives at least as long as any slice from it.
    unsafe { MmapOptions::new().map(&file) }.map_err(|source| ParseError::Io {
        path: path.display().to_string(),
        source,
    })
}
