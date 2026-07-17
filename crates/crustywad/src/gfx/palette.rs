//! `PLAYPAL` palettes and `COLORMAP` light-diminishing tables (ADR-0022 §3).

use crate::{ParseOptions, Strictness};

use super::{GfxError, GfxWarning};

/// One 256-entry RGB palette (768 bytes on disk).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Palette(pub [[u8; 3]; 256]);

impl Palette {
    /// The RGB triple for a palette index.
    #[must_use]
    pub fn rgb(&self, index: u8) -> [u8; 3] {
        self.0[usize::from(index)]
    }
}

/// The `PLAYPAL` lump: `N × 768` bytes of 256-entry RGB palettes. The count
/// is derived from the lump length alone — there is no count field on disk,
/// and vanilla never validates the length (ADR-0022 §3).
#[derive(Debug, Clone)]
pub struct Playpal {
    palettes: Vec<Palette>,
    warnings: Vec<GfxWarning>,
}

impl Playpal {
    /// Parses a `PLAYPAL` lump.
    ///
    /// # Errors
    ///
    /// Strict mode: [`GfxError::PlaypalSize`] when the length is not a
    /// positive multiple of 768. Lenient mode truncates the remainder and
    /// warns (a zero-palette result also warns) and never fails.
    pub fn parse(bytes: &[u8], options: &ParseOptions) -> Result<Self, GfxError> {
        let mut warnings = Vec::new();
        if bytes.is_empty() || bytes.len() % 768 != 0 {
            match options.strictness {
                Strictness::Strict => {
                    return Err(GfxError::PlaypalSize { len: bytes.len() });
                }
                Strictness::Lenient => {
                    warnings.push(GfxWarning::PlaypalSize { len: bytes.len() });
                }
            }
        }
        let palettes = bytes
            .chunks_exact(768)
            .map(|chunk| {
                let mut entries = [[0u8; 3]; 256];
                for (entry, rgb) in entries.iter_mut().zip(chunk.chunks_exact(3)) {
                    entry.copy_from_slice(rgb);
                }
                Palette(entries)
            })
            .collect();
        Ok(Self { palettes, warnings })
    }

    /// The palettes, in lump order.
    #[must_use]
    pub fn palettes(&self) -> &[Palette] {
        &self.palettes
    }

    /// Non-fatal issues recovered during lenient parsing.
    #[must_use]
    pub fn warnings(&self) -> &[GfxWarning] {
        &self.warnings
    }
}

/// The `COLORMAP` lump: 32 light-diminishing tables of 256 palette-index
/// remappings each (8192 bytes; the 32 is a vanilla compile-time constant,
/// never read from the lump — ADR-0022 §3).
#[derive(Debug, Clone)]
pub struct Colormap {
    tables: Box<[[u8; 256]; 32]>,
    warnings: Vec<GfxWarning>,
}

impl Colormap {
    /// Parses a `COLORMAP` lump.
    ///
    /// # Errors
    ///
    /// Strict mode: [`GfxError::ColormapSize`] when the length is not
    /// exactly 8192. Lenient mode zero-pads a short lump / truncates a long
    /// one (the virtual-pad precedent), warns, and never fails.
    pub fn parse(bytes: &[u8], options: &ParseOptions) -> Result<Self, GfxError> {
        let mut warnings = Vec::new();
        if bytes.len() != 8192 {
            match options.strictness {
                Strictness::Strict => {
                    return Err(GfxError::ColormapSize { len: bytes.len() });
                }
                Strictness::Lenient => {
                    warnings.push(GfxWarning::ColormapSize { len: bytes.len() });
                }
            }
        }
        let mut tables = Box::new([[0u8; 256]; 32]);
        for (i, byte) in bytes.iter().take(8192).enumerate() {
            tables[i / 256][i % 256] = *byte;
        }
        Ok(Self { tables, warnings })
    }

    /// The 32 diminishing tables, brightest first (lump order).
    #[must_use]
    pub fn tables(&self) -> &[[u8; 256]; 32] {
        &self.tables
    }

    /// Non-fatal issues recovered during lenient parsing.
    #[must_use]
    pub fn warnings(&self) -> &[GfxWarning] {
        &self.warnings
    }
}
