//! `PNAMES` and `TEXTURE1`/`TEXTURE2` — the classic texture-definition
//! lumps (ADR-0022 §3): patch-name table, and per-texture patch layouts
//! composed at render time. Vanilla validates none of the counts or
//! offsets it reads (unchecked allocation from the PNAMES count; texture
//! offsets checked only against whole-lump length; patch indices
//! unchecked) — every bound here closes a vector from ADR-0022 §6.

use crate::util::trim_nul;
use crate::{ParseOptions, Strictness};

use super::{GfxError, GfxWarning};

/// The `PNAMES` lump: the ordered patch-name table `TEXTUREx` refs index.
#[derive(Debug, Clone)]
pub struct Pnames {
    names: Vec<String>,
    warnings: Vec<GfxWarning>,
}

impl Pnames {
    /// Parses a `PNAMES` lump.
    ///
    /// # Errors
    ///
    /// Strict mode: [`GfxError::TruncatedPnames`],
    /// [`GfxError::NegativePnamesCount`], or
    /// [`GfxError::PnamesCountExceedsLump`]. Lenient recovers each
    /// (empty / zero / clamped) and never fails.
    pub fn parse(bytes: &[u8], options: &ParseOptions) -> Result<Self, GfxError> {
        let strictness = options.strictness;
        let mut warnings = Vec::new();
        if bytes.len() < 4 {
            match strictness {
                Strictness::Strict => return Err(GfxError::TruncatedPnames { len: bytes.len() }),
                Strictness::Lenient => {
                    warnings.push(GfxWarning::TruncatedPnames { len: bytes.len() });
                    return Ok(Self {
                        names: Vec::new(),
                        warnings,
                    });
                }
            }
        }
        let count = i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        if count < 0 {
            match strictness {
                Strictness::Strict => return Err(GfxError::NegativePnamesCount { count }),
                Strictness::Lenient => {
                    warnings.push(GfxWarning::NegativePnamesCount { count });
                    return Ok(Self {
                        names: Vec::new(),
                        warnings,
                    });
                }
            }
        }
        #[allow(clippy::cast_sign_loss)] // non-negative checked above
        let mut count = count as usize;
        let available = (bytes.len() - 4) / 8;
        if count > available {
            match strictness {
                Strictness::Strict => {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
                    return Err(GfxError::PnamesCountExceedsLump {
                        count: count as i32,
                        len: bytes.len(),
                    });
                }
                Strictness::Lenient => {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
                    warnings.push(GfxWarning::PnamesCountExceedsLump {
                        count: count as i32,
                        len: bytes.len(),
                    });
                    count = available;
                }
            }
        }
        let names = bytes[4..4 + count * 8]
            .chunks_exact(8)
            .map(|chunk| String::from_utf8_lossy(trim_nul(chunk)).into_owned())
            .collect();
        Ok(Self { names, warnings })
    }

    /// The patch names, NUL-trimmed, in lump order.
    #[must_use]
    pub fn names(&self) -> &[String] {
        &self.names
    }

    /// Non-fatal issues recovered during lenient parsing.
    #[must_use]
    pub fn warnings(&self) -> &[GfxWarning] {
        &self.warnings
    }
}

/// One texture definition: a named canvas assembled from patch placements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureDef {
    /// The texture's name (8 bytes on disk, NUL-trimmed).
    pub name: String,
    /// Historically dead on-disk field (`masked`); preserved raw, never
    /// interpreted (ADR-0022 §3).
    pub masked: i32,
    /// Canvas width in pixels.
    pub width: i16,
    /// Canvas height in pixels.
    pub height: i16,
    /// Historically dead on-disk field (`columndirectory`); preserved raw.
    pub column_directory: i32,
    /// Patch placements, in def order (later placements draw over earlier).
    pub patches: Vec<TexturePatchRef>,
}

/// One patch placement inside a texture definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TexturePatchRef {
    /// Horizontal placement of the patch's left edge on the canvas.
    pub origin_x: i16,
    /// Vertical placement of the patch's top edge on the canvas.
    pub origin_y: i16,
    /// Index into the `PNAMES` table (raw; validated when a
    /// `TextureSet` is built).
    pub patch: i16,
    /// Historically dead on-disk field (`stepdir`); preserved raw.
    pub step_dir: i16,
    /// Historically dead on-disk field (`colormap`); preserved raw.
    pub colormap: i16,
}

/// A parsed `TEXTURE1`/`TEXTURE2` lump.
#[derive(Debug, Clone)]
pub struct TextureX {
    textures: Vec<TextureDef>,
    warnings: Vec<GfxWarning>,
}

impl TextureX {
    /// Parses a `TEXTURE1`/`TEXTURE2` lump.
    ///
    /// # Errors
    ///
    /// Strict mode: the first violation among the policy rows (see the
    /// variant docs). Lenient recovers each (empty / clamp / skip / stop)
    /// and never fails.
    #[allow(clippy::too_many_lines)]
    pub fn parse(bytes: &[u8], options: &ParseOptions) -> Result<Self, GfxError> {
        let strictness = options.strictness;
        let mut warnings = Vec::new();
        if bytes.len() < 4 {
            match strictness {
                Strictness::Strict => {
                    return Err(GfxError::TruncatedTextureX {
                        len: bytes.len(),
                        needed: 4,
                    });
                }
                Strictness::Lenient => {
                    warnings.push(GfxWarning::TruncatedTextureX {
                        len: bytes.len(),
                        needed: 4,
                    });
                    return Ok(Self {
                        textures: Vec::new(),
                        warnings,
                    });
                }
            }
        }
        let raw_count = i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        if raw_count < 0 {
            match strictness {
                Strictness::Strict => {
                    return Err(GfxError::NegativeTextureCount { count: raw_count });
                }
                Strictness::Lenient => {
                    warnings.push(GfxWarning::NegativeTextureCount { count: raw_count });
                    return Ok(Self {
                        textures: Vec::new(),
                        warnings,
                    });
                }
            }
        }
        #[allow(clippy::cast_sign_loss)] // non-negative checked above
        let mut count = raw_count as usize;
        let needed = 4 + count * 4;
        if needed > bytes.len() {
            match strictness {
                Strictness::Strict => {
                    return Err(GfxError::TruncatedTextureX {
                        len: bytes.len(),
                        needed,
                    });
                }
                Strictness::Lenient => {
                    warnings.push(GfxWarning::TruncatedTextureX {
                        len: bytes.len(),
                        needed,
                    });
                    count = (bytes.len() - 4) / 4;
                }
            }
        }

        // ADR-0016 §1 (the #156 aliased-offset precedent): offsets may
        // alias the same bytes, so cumulative consumed texture bytes
        // (22 + 10 × patchcount each) are budgeted against the lump
        // length, keeping parse O(len) in both modes.
        let mut budget = bytes.len();
        let mut textures = Vec::new();
        for texture in 0..count {
            let table_at = 4 + texture * 4;
            let offset = i32::from_le_bytes([
                bytes[table_at],
                bytes[table_at + 1],
                bytes[table_at + 2],
                bytes[table_at + 3],
            ]);
            let start = match usize::try_from(offset) {
                Ok(start) if start + 22 <= bytes.len() => start,
                _ => {
                    match strictness {
                        Strictness::Strict => {
                            return Err(GfxError::TextureOffsetOutOfBounds {
                                texture,
                                offset,
                                len: bytes.len(),
                            });
                        }
                        Strictness::Lenient => {
                            warnings.push(GfxWarning::TextureOffsetOutOfBounds {
                                texture,
                                offset,
                                len: bytes.len(),
                            });
                            continue; // texture skipped
                        }
                    }
                }
            };

            let name = String::from_utf8_lossy(trim_nul(&bytes[start..start + 8])).into_owned();
            let masked = i32::from_le_bytes([
                bytes[start + 8],
                bytes[start + 9],
                bytes[start + 10],
                bytes[start + 11],
            ]);
            let width = i16::from_le_bytes([bytes[start + 12], bytes[start + 13]]);
            let height = i16::from_le_bytes([bytes[start + 14], bytes[start + 15]]);
            let column_directory = i32::from_le_bytes([
                bytes[start + 16],
                bytes[start + 17],
                bytes[start + 18],
                bytes[start + 19],
            ]);
            let raw_patchcount = i16::from_le_bytes([bytes[start + 20], bytes[start + 21]]);

            let mut patchcount = if raw_patchcount < 0 {
                match strictness {
                    Strictness::Strict => {
                        return Err(GfxError::NegativePatchCount {
                            texture,
                            count: raw_patchcount,
                        });
                    }
                    Strictness::Lenient => {
                        warnings.push(GfxWarning::NegativePatchCount {
                            texture,
                            count: raw_patchcount,
                        });
                        0
                    }
                }
            } else {
                #[allow(clippy::cast_sign_loss)] // non-negative checked above
                {
                    raw_patchcount as usize
                }
            };

            let full_extent = start + 22 + patchcount * 10;
            if full_extent > bytes.len() {
                match strictness {
                    Strictness::Strict => {
                        return Err(GfxError::TextureExtentOutOfBounds {
                            texture,
                            needed: full_extent,
                            len: bytes.len(),
                        });
                    }
                    Strictness::Lenient => {
                        warnings.push(GfxWarning::TextureExtentOutOfBounds {
                            texture,
                            needed: full_extent,
                            len: bytes.len(),
                        });
                        patchcount = (bytes.len() - start - 22) / 10;
                    }
                }
            }

            let consumed = 22 + 10 * patchcount;
            if consumed > budget {
                match strictness {
                    Strictness::Strict => {
                        return Err(GfxError::ExcessiveTextureData {
                            texture,
                            len: bytes.len(),
                        });
                    }
                    Strictness::Lenient => {
                        warnings.push(GfxWarning::ExcessiveTextureData {
                            texture,
                            len: bytes.len(),
                        });
                        break; // remaining textures skipped
                    }
                }
            }
            budget -= consumed;

            let mut patches = Vec::with_capacity(patchcount);
            for i in 0..patchcount {
                let p = start + 22 + i * 10;
                patches.push(TexturePatchRef {
                    origin_x: i16::from_le_bytes([bytes[p], bytes[p + 1]]),
                    origin_y: i16::from_le_bytes([bytes[p + 2], bytes[p + 3]]),
                    patch: i16::from_le_bytes([bytes[p + 4], bytes[p + 5]]),
                    step_dir: i16::from_le_bytes([bytes[p + 6], bytes[p + 7]]),
                    colormap: i16::from_le_bytes([bytes[p + 8], bytes[p + 9]]),
                });
            }
            textures.push(TextureDef {
                name,
                masked,
                width,
                height,
                column_directory,
                patches,
            });
        }
        Ok(Self { textures, warnings })
    }

    /// The texture definitions, in lump order (skipped textures omitted).
    #[must_use]
    pub fn textures(&self) -> &[TextureDef] {
        &self.textures
    }

    /// Non-fatal issues recovered during lenient parsing.
    #[must_use]
    pub fn warnings(&self) -> &[GfxWarning] {
        &self.warnings
    }
}
