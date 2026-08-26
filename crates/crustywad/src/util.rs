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

/// CRC-32 (IEEE 802.3; reflected polynomial `0xEDB8_8320`) of `bytes`, the
/// checksum zip central-directory entries carry for each member (`archive`
/// feature). Table-driven; the table is built at compile time so the crate
/// takes no checksum dependency.
#[cfg(feature = "archive")]
#[allow(dead_code)] // consumed by `archive::zip` from Task 5 on
pub(crate) fn crc32(bytes: &[u8]) -> u32 {
    const TABLE: [u32; 256] = crc32_table();
    let mut crc = 0xFFFF_FFFF_u32;
    for &b in bytes {
        crc = TABLE[((crc ^ u32::from(b)) & 0xFF) as usize] ^ (crc >> 8);
    }
    !crc
}

#[cfg(feature = "archive")]
#[allow(clippy::cast_possible_truncation)] // `i < 256` by construction
const fn crc32_table() -> [u32; 256] {
    let mut table = [0_u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut c = i as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
            k += 1;
        }
        table[i] = c;
        i += 1;
    }
    table
}

#[cfg(all(test, feature = "archive"))]
mod tests {
    use super::crc32;

    #[test]
    fn crc32_matches_the_ieee_check_value() {
        // The standard CRC-32 check value (RFC 1952 §8 uses the same table).
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0);
    }
}
