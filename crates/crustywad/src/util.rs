//! Small crate-internal helpers shared across the parsing paths.

/// Returns the slice of `bytes` up to the first NUL byte (exclusive), or the
/// whole slice if there is none.
///
/// Doom stores fixed-width names NUL-padded on the right; this strips that
/// padding without allocating. Shared by the directory-name path
/// (`Lump::name`) and the in-record texture path
/// ([`Name8::as_str_lossy`][crate::map::Name8::as_str_lossy]).
pub(crate) fn trim_nul(bytes: &[u8]) -> &[u8] {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    &bytes[..end]
}
