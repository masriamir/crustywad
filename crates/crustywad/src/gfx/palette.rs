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
        if bytes.is_empty() || !bytes.len().is_multiple_of(768) {
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
            .as_chunks::<768>()
            .0
            .iter()
            .map(|chunk| {
                let mut entries = [[0u8; 3]; 256];
                for (entry, rgb) in entries.iter_mut().zip(chunk.as_chunks::<3>().0) {
                    *entry = *rgb;
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

/// The `COLORMAP` lump: `N × 256` light-diminishing tables of 256
/// palette-index remappings each. Vanilla's `NUMCOLORMAPS` compile-time
/// constant is 32 and the engine loads the lump with no size check against
/// it (ADR-0022 §3); retail lumps universally carry 34 tables (11/11 in the
/// collection — ADR-0022 §3 correction amendment, #156), so the strict rule
/// is a whole number of 256-byte tables totaling at least 8192 bytes (the
/// 32-table floor every consumer indexes), and every table on disk is
/// exposed rather than only the first 32.
#[derive(Debug, Clone)]
pub struct Colormap {
    tables: Vec<[u8; 256]>,
    warnings: Vec<GfxWarning>,
}

impl Colormap {
    /// Parses a `COLORMAP` lump.
    ///
    /// # Errors
    ///
    /// Strict mode: [`GfxError::ColormapSize`] when the length is not a
    /// 256-byte multiple of at least 8192. Lenient mode zero-pads a short
    /// lump to 8192 / truncates a long one's trailing partial table, warns,
    /// and never fails.
    pub fn parse(bytes: &[u8], options: &ParseOptions) -> Result<Self, GfxError> {
        let mut warnings = Vec::new();
        if !bytes.len().is_multiple_of(256) || bytes.len() < 8192 {
            match options.strictness {
                Strictness::Strict => {
                    return Err(GfxError::ColormapSize { len: bytes.len() });
                }
                Strictness::Lenient => {
                    warnings.push(GfxWarning::ColormapSize { len: bytes.len() });
                }
            }
        }
        let mut data = bytes.to_vec();
        if data.len() < 8192 {
            data.resize(8192, 0);
        } else {
            data.truncate(data.len() - data.len() % 256);
        }
        let tables = data
            .as_chunks::<256>()
            .0
            .iter()
            .map(|chunk| {
                let mut table = [0u8; 256];
                table.copy_from_slice(chunk);
                table
            })
            .collect();
        Ok(Self { tables, warnings })
    }

    /// The diminishing tables, brightest first (lump order); at least 32 in
    /// both modes — retail lumps carry 34.
    #[must_use]
    pub fn tables(&self) -> &[[u8; 256]] {
        &self.tables
    }

    /// Non-fatal issues recovered during lenient parsing.
    #[must_use]
    pub fn warnings(&self) -> &[GfxWarning] {
        &self.warnings
    }
}
