//! Doom 64 PNG lumps (`doom64-gfx` feature; ADR-0022 §5): the PC WAD's
//! texture/sprite/gfx lumps are standard palette-type PNGs — `PLTE` of up
//! to 16 rows of 16 colors (runtime variants), optional `tRNS`, sprite
//! offsets in a private `grAb` chunk (big-endian `i32` pair, the `ZDoom`
//! convention). The `png` crate decodes the standard chunks; two gaps are
//! closed here with bounded code of our own: private-chunk access (the
//! crate exposes none) and sub-8-bit index unpacking (the crate's
//! `PACKING` transformation is declared but not implemented in 0.18).

use std::io::Cursor;

use crate::{ParseOptions, Strictness};

use super::{GfxError, GfxWarning};

/// A decoded Doom 64 PNG lump (`doom64-gfx`): indexed pixels, the embedded
/// `PLTE` (up to 16 rows of 16 colors serving runtime variants —
/// ADR-0022 §5), optional per-index `tRNS` alpha, and `grAb` sprite
/// offsets. Row/`PAL`-variant SELECTION is deliberately not interpreted
/// here (follow-up; the rows are exposed raw).
#[derive(Debug, Clone)]
pub struct Doom64Png {
    /// Width in pixels (the per-side cap guarantees `u16`).
    pub width: u16,
    /// Height in pixels.
    pub height: u16,
    /// Sprite draw offsets from the private `grAb` chunk (big-endian
    /// `i32` pair, the `ZDoom` convention); `None` when absent (textures).
    pub offsets: Option<(i32, i32)>,
    pixels: Vec<u8>,
    plte: Vec<[u8; 3]>,
    trns: Vec<u8>,
    warnings: Vec<GfxWarning>,
}

impl Doom64Png {
    /// Decodes a Doom 64 PNG lump.
    ///
    /// # Errors
    ///
    /// Both modes: [`GfxError::PngDecode`] (undecodable),
    /// [`GfxError::NotPaletteIndexed`] (no index data to recover — never
    /// the engine's abort, ADR-0022 §5), [`GfxError::DecodedImageTooLarge`]
    /// (the [`Limits::max_decoded_pixels`](crate::Limits::max_decoded_pixels)
    /// and 65535-per-side caps, checked before any pixel allocation).
    /// Strict mode additionally: [`GfxError::OversizedTrns`],
    /// [`GfxError::PixelIndexOutOfRange`], [`GfxError::MalformedGrab`].
    pub fn decode(bytes: &[u8], options: &ParseOptions) -> Result<Self, GfxError> {
        let strictness = options.strictness;
        let mut warnings = Vec::new();

        let decoder = png::Decoder::new(Cursor::new(bytes));
        let mut reader = decoder.read_info().map_err(|e| GfxError::PngDecode {
            detail: e.to_string(),
        })?;

        let info = reader.info();
        let (width32, height32) = (info.width, info.height);
        if info.color_type != png::ColorType::Indexed {
            return Err(GfxError::NotPaletteIndexed {
                color_type: color_type_name(info.color_type),
            });
        }
        let (width, height) =
            check_dimensions(width32, height32, options.limits.max_decoded_pixels)?;

        let plte: Vec<[u8; 3]> = info
            .palette
            .as_ref()
            .map(|p| p.as_chunks::<3>().0.to_vec())
            .unwrap_or_default();
        if plte.is_empty() {
            // Reachable: the `png` crate only enforces PLTE presence when
            // EXPAND transformations are requested (`create_transform_fn`'s
            // `PaletteRequired` arm). This reader uses the default
            // `Transformations::IDENTITY`, so an Indexed-color PNG with no
            // `PLTE` chunk decodes through the crate and lands here — see
            // `indexed_png_without_plte_hits_the_missing_plte_guard` in
            // `tests/doom64_gfx.rs`.
            return Err(GfxError::PngDecode {
                detail: "missing PLTE".to_owned(),
            });
        }
        let trns = resolve_trns(info, plte.len(), strictness, &mut warnings)?;
        // Explicit map instead of `as u8`: the numeric repr of an external
        // enum is not a stable contract, and an unexpected depth must not
        // reach the unpacking math (which requires a divisor of 8).
        let bit_depth: u8 = match info.bit_depth {
            png::BitDepth::One => 1,
            png::BitDepth::Two => 2,
            png::BitDepth::Four => 4,
            png::BitDepth::Eight => 8,
            // The PNG spec forbids 16-bit indexed images; the crate should
            // reject them first — defensive bridge, never a panic.
            png::BitDepth::Sixteen => {
                return Err(GfxError::PngDecode {
                    detail: "16-bit indexed PNG".to_owned(),
                });
            }
        };
        let pixels = decode_pixels(&mut reader, width, height, bit_depth)?;
        check_pixel_range(&pixels, plte.len(), strictness, &mut warnings)?;
        let offsets = resolve_grab(bytes, strictness, &mut warnings)?;

        Ok(Self {
            width,
            height,
            offsets,
            pixels,
            plte,
            trns,
            warnings,
        })
    }

    /// One index byte per pixel, row-major; `len == width × height`.
    #[must_use]
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// The embedded `PLTE`, as stored (1..=256 RGB entries).
    #[must_use]
    pub fn plte(&self) -> &[[u8; 3]] {
        &self.plte
    }

    /// Per-index `tRNS` alpha (may be shorter than the `PLTE`; empty when
    /// the chunk is absent — treat missing entries as opaque).
    #[must_use]
    pub fn trns(&self) -> &[u8] {
        &self.trns
    }

    /// One 16-color Doom 64 palette row (`PLTE` rows serve runtime
    /// variants — ADR-0022 §5); `None` when the row is not fully present.
    #[must_use]
    pub fn palette_row(&self, row: usize) -> Option<[[u8; 3]; 16]> {
        let start = row.checked_mul(16)?;
        let end = start.checked_add(16)?;
        let slice = self.plte.get(start..end)?;
        let mut out = [[0u8; 3]; 16];
        out.copy_from_slice(slice);
        Some(out)
    }

    /// Non-fatal issues recovered during lenient decoding.
    #[must_use]
    pub fn warnings(&self) -> &[GfxWarning] {
        &self.warnings
    }

    /// The tier-2 view (ADR-0022 §3's shared contract). Mask is `false`
    /// where the pixel's `tRNS` alpha is exactly 0 OR the index has no
    /// `PLTE` entry (a lenient-kept out-of-range index has no resolvable
    /// color); covered everywhere else (missing `tRNS` entries are
    /// opaque).
    #[must_use]
    pub fn to_indexed(&self) -> super::IndexedImage {
        let mask = self
            .pixels
            .iter()
            .map(|&i| {
                let idx = usize::from(i);
                idx < self.plte.len() && self.trns.get(idx).copied().unwrap_or(255) != 0
            })
            .collect();
        super::IndexedImage {
            width: self.width,
            height: self.height,
            pixels: self.pixels.clone(),
            mask,
        }
    }

    /// RGBA over the full embedded `PLTE`: `rgb = plte[index]`,
    /// `alpha = trns[index]` (255 where absent); pixels the mask rule
    /// marks uncovered render transparent black. This is deliberately NOT
    /// `to_indexed().to_rgba(palette)` — the PNG carries per-index alpha
    /// that [`IndexedImage`](super::IndexedImage)'s boolean mask cannot
    /// represent, so this method reads `trns` directly instead of routing
    /// through the tier-2 view. Row/`PAL`-variant rendering is the
    /// recorded follow-up (ADR-0022 §5).
    #[must_use]
    pub fn to_rgba(&self) -> super::RgbaImage {
        // Saturating: the multiply could overflow a 32-bit usize under a
        // raised decode cap; saturation degrades to reallocation, never a
        // panic (ADR-0016).
        let mut out = Vec::with_capacity(self.pixels.len().saturating_mul(4));
        for &i in &self.pixels {
            let idx = usize::from(i);
            match self.plte.get(idx) {
                Some(&[r, g, b]) => {
                    let alpha = self.trns.get(idx).copied().unwrap_or(255);
                    if alpha == 0 {
                        out.extend_from_slice(&[0, 0, 0, 0]);
                    } else {
                        out.extend_from_slice(&[r, g, b, alpha]);
                    }
                }
                None => out.extend_from_slice(&[0, 0, 0, 0]),
            }
        }
        super::RgbaImage {
            width: self.width,
            height: self.height,
            pixels: out,
        }
    }
}

/// Validates the declared PNG dimensions BEFORE any pixel allocation
/// (ADR-0022 §5's uncapped-`Z_Calloc` defect is the anti-pattern): both the
/// 65535-per-side cap (so `width`/`height` narrow losslessly to `u16`) and
/// [`Limits::max_decoded_pixels`](crate::Limits::max_decoded_pixels). Fires
/// in both strictness modes — the DoS-cap exception to ADR-0003.
fn check_dimensions(
    width32: u32,
    height32: u32,
    max_pixels: usize,
) -> Result<(u16, u16), GfxError> {
    let per_side_ok = u16::try_from(width32).is_ok() && u16::try_from(height32).is_ok();
    let area = (width32 as usize).saturating_mul(height32 as usize);
    if !per_side_ok || area > max_pixels {
        return Err(GfxError::DecodedImageTooLarge {
            width: width32,
            height: height32,
            max_pixels,
        });
    }
    #[allow(clippy::cast_possible_truncation)] // per-side cap above
    Ok((width32 as u16, height32 as u16))
}

/// Applies the `tRNS`-oversize policy row: strict mode errors, lenient
/// mode truncates to the `PLTE` length and records a warning.
fn resolve_trns(
    info: &png::Info<'_>,
    plte_len: usize,
    strictness: Strictness,
    warnings: &mut Vec<GfxWarning>,
) -> Result<Vec<u8>, GfxError> {
    let mut trns: Vec<u8> = info.trns.as_ref().map(|t| t.to_vec()).unwrap_or_default();
    if trns.len() > plte_len {
        match strictness {
            Strictness::Strict => {
                return Err(GfxError::OversizedTrns {
                    trns_len: trns.len(),
                    plte_len,
                });
            }
            Strictness::Lenient => {
                warnings.push(GfxWarning::OversizedTrns {
                    trns_len: trns.len(),
                    plte_len,
                });
                trns.truncate(plte_len);
            }
        }
    }
    Ok(trns)
}

/// Decodes the packed frame and unpacks it to one index byte per pixel.
/// Computes `line_size` from PNG-spec math (`ceil(width×depth/8)`) rather
/// than relying on `OutputInfo` accessors (unverified against 0.18.1), and
/// guards the slice explicitly rather than trusting the buffer size.
fn decode_pixels(
    reader: &mut png::Reader<Cursor<&[u8]>>,
    width: u16,
    height: u16,
    bit_depth: u8,
) -> Result<Vec<u8>, GfxError> {
    // Zero-dimension PNGs are a valid EMPTY image per the decode policy —
    // return before any row chunking, where `line_size == 0` would panic
    // in `chunks_exact`. Whether the `png` crate can deliver zero
    // dimensions is deliberately NOT assumed (the missing-PLTE guard
    // taught that crate-side validation assumptions can be wrong); the
    // fuzz target's no-panic oracle covers the question empirically.
    if width == 0 || height == 0 {
        return Ok(Vec::new());
    }
    let line_size = (usize::from(width) * usize::from(bit_depth)).div_ceil(8);
    // Coverage note: `output_buffer_size` returns `None` only when the
    // deinterlaced frame would not fit in `isize`; `check_dimensions`
    // already bounds both sides to `u16`, so the product (<= 65535 x
    // 65535) cannot approach `isize::MAX` (~9.2e18) on any 64-bit target —
    // the whole matrix this crate is tested on. Kept as defense in depth
    // (a 32-bit target could reach it) rather than an unwrap, per
    // ADR-0016.
    let Some(buf_size) = reader.output_buffer_size() else {
        return Err(GfxError::PngDecode {
            detail: "output size overflow".to_owned(),
        });
    };
    // Packed output is at most one byte per pixel for indexed depths, so
    // this allocation is within the caps `check_dimensions` already
    // enforced.
    let mut packed = vec![0u8; buf_size];
    reader
        .next_frame(&mut packed)
        .map_err(|e| GfxError::PngDecode {
            detail: e.to_string(),
        })?;
    let needed = line_size * usize::from(height);
    // Coverage note: `packed` is allocated to exactly `buf_size`
    // (`vec![0u8; buf_size]` above), and with the default (identity)
    // transformations this reader uses, `output_buffer_size` computes the
    // same `raw_row_length(depth, width) * height` this function's
    // `line_size * height` does — so `packed.len() < needed` cannot occur
    // for any stream the `png` crate itself successfully decoded. Guards
    // the slice explicitly (defense in depth, ADR-0016) rather than
    // trusting that invariant to hold across `png` crate versions.
    if packed.len() < needed {
        return Err(GfxError::PngDecode {
            detail: "short output buffer".to_owned(),
        });
    }
    Ok(unpack_indices(
        &packed[..needed],
        usize::from(width),
        usize::from(height),
        line_size,
        bit_depth,
    ))
}

/// Applies the out-of-range-pixel-index policy row: strict mode errors on
/// the first offending pixel, lenient mode aggregates a single warning
/// describing the run and keeps the pixels (rendered as holes downstream).
fn check_pixel_range(
    pixels: &[u8],
    plte_len: usize,
    strictness: Strictness,
    warnings: &mut Vec<GfxWarning>,
) -> Result<(), GfxError> {
    let mut oob = pixels.iter().filter(|&&i| usize::from(i) >= plte_len);
    if let Some(&first_index) = oob.next() {
        let count = 1 + oob.count();
        match strictness {
            Strictness::Strict => {
                return Err(GfxError::PixelIndexOutOfRange {
                    index: first_index,
                    plte_len,
                });
            }
            Strictness::Lenient => warnings.push(GfxWarning::PixelIndexOutOfRange {
                first_index,
                count,
                plte_len,
            }),
        }
    }
    Ok(())
}

/// Applies the malformed-`grAb`-chunk policy row: strict mode errors,
/// lenient mode ignores the chunk and records a warning.
fn resolve_grab(
    bytes: &[u8],
    strictness: Strictness,
    warnings: &mut Vec<GfxWarning>,
) -> Result<Option<(i32, i32)>, GfxError> {
    match find_grab(bytes) {
        Ok(offsets) => Ok(offsets),
        Err(len) => match strictness {
            Strictness::Strict => Err(GfxError::MalformedGrab { len }),
            Strictness::Lenient => {
                warnings.push(GfxWarning::MalformedGrab { len });
                Ok(None)
            }
        },
    }
}

/// Stable names for the error message (no dependency types in our API).
///
/// Coverage note: the `Indexed` arm is unreachable at the sole call site
/// (`decode` only calls this after checking `color_type != Indexed`); kept
/// for exhaustive matching over the upstream enum rather than an
/// unreachable `_` catch-all, per ADR-0016.
fn color_type_name(ct: png::ColorType) -> &'static str {
    match ct {
        png::ColorType::Grayscale => "grayscale",
        png::ColorType::Rgb => "RGB",
        png::ColorType::Indexed => "indexed",
        png::ColorType::GrayscaleAlpha => "grayscale+alpha",
        png::ColorType::Rgba => "RGBA",
    }
}

/// Walks the PNG chunk stream for a `grAb` chunk. `Ok(Some((x, y)))` for a
/// well-formed 8-byte chunk (two big-endian `i32`s), `Ok(None)` when
/// absent or the stream ends/degenerates first (the `png` crate has
/// already vetted the stream when this runs), `Err(len)` for a `grAb`
/// with the wrong data length. Bounded: each iteration advances at least
/// 12 bytes; CRCs are not validated here (the decoder validates the
/// chunks it consumes).
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
        //
        // Coverage note: `len` is a `u32` (max ~4.3e9) and `pos` is bounded
        // by the input slice's length, so this addition cannot overflow
        // `usize` on any 64-bit target (the whole matrix this crate is
        // tested on). Guards the arithmetic explicitly (defense in depth,
        // ADR-0016) rather than relying on that platform assumption.
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
    fn grab_header_present_but_data_truncated_is_none() {
        // The chunk header declares the well-formed 8-byte length, but the
        // stream is cut off partway through the data — nothing to salvage.
        let mut data = Vec::new();
        data.extend_from_slice(&1i32.to_be_bytes());
        data.extend_from_slice(&2i32.to_be_bytes());
        let mut png = stream(&[chunk(b"IHDR", &[0; 13]), chunk(b"grAb", &data)]);
        png.truncate(png.len() - 8); // leaves the 8-byte header, drops all data + CRC
        assert_eq!(find_grab(&png), Ok(None));
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
