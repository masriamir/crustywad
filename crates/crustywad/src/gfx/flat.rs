//! Raw 64×64 flats (ADR-0022 §3): floor/ceiling textures with no header —
//! the 4096-byte shape is an assumption vanilla makes only at render time.

use crate::{ParseOptions, Strictness};

use super::{GfxError, GfxWarning, IndexedImage, Palette, RgbaImage};

/// A flat: raw palette indices (row-major), at least 4096 (64×64). Retail
/// data ships larger lumps too — Heretic's 4160-byte and Hexen's 8192-byte
/// flats — with only the first 64×64 ever rendered (ADR-0022 §3 correction
/// amendment).
#[derive(Debug, Clone)]
pub struct Flat {
    pixels: Vec<u8>,
    warnings: Vec<GfxWarning>,
}

impl Flat {
    /// A flat's fixed width in pixels.
    pub const WIDTH: u16 = 64;
    /// A flat's fixed height in pixels.
    pub const HEIGHT: u16 = 64;

    /// Parses a flat lump.
    ///
    /// # Errors
    ///
    /// Strict mode: [`GfxError::FlatSize`] when the length is not a 64-byte
    /// multiple of at least 4096 (accepts retail's 4096, Heretic's 4160,
    /// and Hexen's 8192). Lenient mode keeps the actual bytes (ADR-0022 §3:
    /// "proceeds with what is present"), warns, and never fails.
    pub fn parse(bytes: &[u8], options: &ParseOptions) -> Result<Self, GfxError> {
        let mut warnings = Vec::new();
        if bytes.len() % 64 != 0 || bytes.len() < 4096 {
            match options.strictness {
                Strictness::Strict => {
                    return Err(GfxError::FlatSize { len: bytes.len() });
                }
                Strictness::Lenient => {
                    warnings.push(GfxWarning::FlatSize { len: bytes.len() });
                }
            }
        }
        Ok(Self {
            pixels: bytes.to_vec(),
            warnings,
        })
    }

    /// All stored bytes as parsed (row-major), including any beyond 4096 —
    /// use [`Flat::to_indexed`] for the rendered 64×64 view.
    #[must_use]
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Non-fatal issues recovered during lenient parsing.
    #[must_use]
    pub fn warnings(&self) -> &[GfxWarning] {
        &self.warnings
    }

    /// The rendered 64×64 view — vanilla's renderer reads exactly 64×64
    /// regardless of lump size (ADR-0022 §3), so bytes beyond 4096 are not
    /// rendered (still available via [`Flat::pixels`]); a lenient short flat
    /// zero-pads to 4096 at conversion.
    #[must_use]
    pub fn to_indexed(&self) -> IndexedImage {
        // Build the 4096-byte rendered buffer directly: only the first
        // 64×64 bytes are ever rendered, so cloning an oversized stored
        // lump (strict allows 8192; lenient keeps any length) just to
        // truncate it would be a wasted transient allocation.
        let mut pixels = vec![0u8; 4096];
        let take = self.pixels.len().min(4096);
        pixels[..take].copy_from_slice(&self.pixels[..take]);
        IndexedImage {
            width: Self::WIDTH,
            height: Self::HEIGHT,
            pixels,
            mask: vec![true; 4096],
        }
    }

    /// [`Flat::to_indexed`] plus palette application (tier 3).
    #[must_use]
    pub fn to_rgba(&self, palette: &Palette) -> RgbaImage {
        self.to_indexed().to_rgba(palette)
    }
}
