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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn nonexistent_path_returns_io_error() {
        let err = open(Path::new("/nonexistent/path/file.wad"))
            .expect_err("missing file should fail");
        assert!(matches!(err, ParseError::Io { .. }));
    }

    // On Linux, mmap of a zero-length file returns EINVAL, exercising the second map_err.
    // On macOS the OS accepts a zero-length mapping, so this case is only testable on Linux.
    #[cfg(target_os = "linux")]
    #[test]
    fn empty_file_map_returns_io_error() {
        let file = NamedTempFile::new().expect("tempfile should be created");
        let err = open(file.path()).expect_err("mmap of empty file should fail on Linux");
        assert!(matches!(err, ParseError::Io { .. }));
    }
}
