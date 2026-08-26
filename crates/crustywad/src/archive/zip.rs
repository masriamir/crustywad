//! Zip container reader. Filled in by Tasks 4–5.

use super::{ArchiveError, Container, RawEntry};

#[derive(Debug)]
#[allow(dead_code)] // `bytes` is read by the real `read_entry` from Task 4 on
pub(crate) struct ZipContainer {
    bytes: Vec<u8>,
    entries: Vec<RawEntry>,
}

impl ZipContainer {
    // The stub never fails; the real central-directory parser from Task 4 on
    // does, so the `Result` return type stays as the real signature.
    #[allow(clippy::unnecessary_wraps)]
    pub(crate) fn open(bytes: Vec<u8>, max_members: usize) -> Result<Self, ArchiveError> {
        let _ = max_members;
        Ok(Self {
            bytes,
            entries: Vec::new(),
        })
    }
}

impl Container for ZipContainer {
    fn entries(&self) -> &[RawEntry] {
        &self.entries
    }
    fn read_entry(&self, index: usize, cap: usize) -> Result<Vec<u8>, ArchiveError> {
        let _ = (index, cap, &self.bytes);
        Ok(Vec::new())
    }
}
