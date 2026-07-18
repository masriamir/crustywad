//! Doom 64 PNG lumps (`doom64-gfx` feature; ADR-0022 §5): the PC WAD's
//! texture/sprite/gfx lumps are standard palette-type PNGs — `PLTE` of up
//! to 16 rows of 16 colors (runtime variants), optional `tRNS`, sprite
//! offsets in a private `grAb` chunk (big-endian `i32` pair, the `ZDoom`
//! convention). The `png` crate decodes the standard chunks; two gaps are
//! closed here with bounded code of our own: private-chunk access (the
//! crate exposes none) and sub-8-bit index unpacking (the crate's
//! `PACKING` transformation is declared but not implemented in 0.18).

/// Walks the PNG chunk stream for a `grAb` chunk. `Ok(Some((x, y)))` for a
/// well-formed 8-byte chunk (two big-endian `i32`s), `Ok(None)` when
/// absent or the stream ends/degenerates first (the `png` crate has
/// already vetted the stream when this runs), `Err(len)` for a `grAb`
/// with the wrong data length. Bounded: each iteration advances at least
/// 12 bytes; CRCs are not validated here (the decoder validates the
/// chunks it consumes).
// Not yet called outside tests: `Doom64Png` (Task 2, #282) wires this in.
#[allow(dead_code)]
pub(super) fn find_grab(bytes: &[u8]) -> Result<Option<(i32, i32)>, usize> {
    let mut pos = 8usize; // past the PNG signature
    loop {
        let Some(header) = bytes.get(pos..pos + 8) else {
            return Ok(None);
        };
        let len = u32::from_be_bytes([header[0], header[1], header[2], header[3]]) as usize;
        let kind = &header[4..8];
        if kind == b"grAb" {
            if len != 8 {
                return Err(len);
            }
            let Some(data) = bytes.get(pos + 8..pos + 16) else {
                return Ok(None); // truncated: nothing to salvage
            };
            let x = i32::from_be_bytes([data[0], data[1], data[2], data[3]]);
            let y = i32::from_be_bytes([data[4], data[5], data[6], data[7]]);
            return Ok(Some((x, y)));
        }
        if kind == b"IEND" {
            return Ok(None);
        }
        // length + type + data + CRC
        let Some(next) = pos.checked_add(12).and_then(|p| p.checked_add(len)) else {
            return Ok(None);
        };
        pos = next;
    }
}

/// Expands packed palette indices (PNG bit depths 1/2/4) to one byte per
/// pixel, MSB-first within each byte, rows independently padded to a byte
/// boundary (the PNG spec's packing). Depth 8 copies rows respecting
/// `line_size`. `packed` must hold `height` rows of `line_size` bytes.
// Not yet called outside tests: `Doom64Png` (Task 2, #282) wires this in.
#[allow(dead_code)]
pub(super) fn unpack_indices(
    packed: &[u8],
    width: usize,
    height: usize,
    line_size: usize,
    depth: u8,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(width * height);
    if depth == 8 {
        for row in packed.chunks_exact(line_size).take(height) {
            out.extend_from_slice(&row[..width]);
        }
        return out;
    }
    let per_byte = usize::from(8 / depth);
    let mask = (1u16 << depth) - 1;
    for row in packed.chunks_exact(line_size).take(height) {
        for x in 0..width {
            let byte = row[x / per_byte];
            let slot = x % per_byte;
            let shift = 8 - depth * (u8::try_from(slot).expect("slot < 8") + 1);
            #[allow(clippy::cast_possible_truncation)] // masked to `depth` bits
            out.push(((u16::from(byte) >> shift) & mask) as u8);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{find_grab, unpack_indices};

    /// A minimal chunk stream: PNG signature + one chunk (type, data, zero
    /// CRC — `find_grab` walks structure only; CRC validity is the png
    /// crate's concern on the chunks IT decodes).
    #[allow(clippy::trivially_copy_pass_by_ref)] // matches call sites (`b"IHDR"` etc. are `&[u8; 4]`)
    fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&u32::try_from(data.len()).unwrap().to_be_bytes());
        out.extend_from_slice(kind);
        out.extend_from_slice(data);
        out.extend_from_slice(&[0; 4]); // CRC not validated by the walker
        out
    }
    fn stream(chunks: &[Vec<u8>]) -> Vec<u8> {
        let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
        for c in chunks {
            out.extend_from_slice(c);
        }
        out
    }

    #[test]
    fn finds_grab_with_big_endian_pair() {
        let mut data = Vec::new();
        data.extend_from_slice(&(-16i32).to_be_bytes());
        data.extend_from_slice(&48i32.to_be_bytes());
        let png = stream(&[
            chunk(b"IHDR", &[0; 13]),
            chunk(b"grAb", &data),
            chunk(b"IEND", &[]),
        ]);
        assert_eq!(find_grab(&png), Ok(Some((-16, 48))));
    }

    #[test]
    fn absent_grab_is_none_and_wrong_length_is_err() {
        let png = stream(&[chunk(b"IHDR", &[0; 13]), chunk(b"IEND", &[])]);
        assert_eq!(find_grab(&png), Ok(None));
        let bad = stream(&[chunk(b"grAb", &[0; 5]), chunk(b"IEND", &[])]);
        assert_eq!(find_grab(&bad), Err(5));
        // Truncated stream: walker stops cleanly, no grAb found.
        let mut trunc = stream(&[chunk(b"IHDR", &[0; 13])]);
        trunc.truncate(trunc.len() - 3);
        assert_eq!(find_grab(&trunc), Ok(None));
    }

    #[test]
    fn unpack_expands_msb_first() {
        // 4bpp, width 3 (packed row = 2 bytes, low nibble of byte 1 is pad):
        // 0xAB 0xC0 -> indices [0xA, 0xB, 0xC].
        assert_eq!(
            unpack_indices(&[0xAB, 0xC0], 3, 1, 2, 4),
            vec![0xA, 0xB, 0xC]
        );
        // 1bpp, width 10 (row = 2 bytes): 0b1100_0001, 0b01xx_xxxx.
        assert_eq!(
            unpack_indices(&[0b1100_0001, 0b0100_0000], 10, 1, 2, 1),
            vec![1, 1, 0, 0, 0, 0, 0, 1, 0, 1]
        );
        // 8bpp is a straight copy honoring line_size.
        assert_eq!(unpack_indices(&[7, 8, 9], 3, 1, 3, 8), vec![7, 8, 9]);
        // Two rows exercise the per-row restart.
        assert_eq!(unpack_indices(&[0x12, 0x34], 2, 2, 1, 4), vec![1, 2, 3, 4]);
    }
}
