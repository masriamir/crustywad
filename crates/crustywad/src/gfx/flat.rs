//! Raw 64×64 flats (ADR-0022 §3): floor/ceiling textures with no header —
//! the 4096-byte shape is an assumption vanilla makes only at render time.

use crate::{ParseOptions, Strictness};

use super::{GfxError, GfxWarning, IndexedImage, Palette, RgbaImage};

/// A flat: 4096 raw palette indices (64×64, row-major).
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
    /// Strict mode: [`GfxError::FlatSize`] when the length is not exactly
    /// 4096. Lenient mode keeps the actual bytes (ADR-0022 §3: "proceeds
    /// with what is present"), warns, and never fails.
    pub fn parse(bytes: &[u8], options: &ParseOptions) -> Result<Self, GfxError> {
        let mut warnings = Vec::new();
        if bytes.len() != 4096 {
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

    /// The raw palette indices as stored (row-major; exactly 4096 for a
    /// well-formed flat, possibly fewer/more after lenient recovery).
    #[must_use]
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Non-fatal issues recovered during lenient parsing.
    #[must_use]
    pub fn warnings(&self) -> &[GfxWarning] {
        &self.warnings
    }

    /// The 64×64 fully-covered indexed view. A lenient wrong-size flat is
    /// normalized here: short lumps zero-pad to 4096 (the virtual-pad
    /// precedent), long lumps truncate.
    #[must_use]
    pub fn to_indexed(&self) -> IndexedImage {
        let mut pixels = self.pixels.clone();
        pixels.resize(4096, 0);
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
