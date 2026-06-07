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
    // SAFETY: `map` requires the file not be truncated while the Mmap is alive;
    // truncation by any process would cause a SIGBUS on access. Opening read-only
    // prevents *this process* from truncating it. Concurrent truncation or
    // modification by another process is not mitigated and is documented as
    // unsupported in the public API (`Wad::from_path_mapped`).
    unsafe { MmapOptions::new().map(&file) }.map_err(|source| ParseError::Io {
        path: path.display().to_string(),
        source,
    })
}
