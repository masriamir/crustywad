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
    /// [`TextureSet`] is built).
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
        // Saturating: a hostile count must not overflow 32-bit usize before the guard (ADR-0016).
        let needed = 4usize.saturating_add(count.saturating_mul(4));
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

use super::Picture;

/// Parsed texture definitions plus their resolved patch pictures — built
/// once via [`Wad::texture_set`](crate::Wad::texture_set), composed per
/// texture with [`TextureSet::compose`]. Patch names resolve through the
/// crate's first-match [`lump_by_name`](crate::Wad::lump_by_name) after
/// uppercasing (vanilla uppercases its search name — ADR-0022 §3); a
/// PWAD referencing base-IWAD patches therefore cannot resolve here
/// (multi-WAD merge is out of scope) — strict errors honestly, lenient
/// builds with unresolved slots and composes with holes.
#[derive(Debug, Clone)]
pub struct TextureSet {
    textures: Vec<TextureDef>,
    // `compose` indexes `patches` directly by `PNAMES` position; it never
    // needs the *names* themselves (no diagnostic in the compose path
    // surfaces a patch's name — dead refs are already named at build time
    // via `GfxWarning::UnresolvedPatchName`/`PatchPictureFailed`). Retained
    // as struct data (parallel to `patches`) since it is the natural home
    // for a future name-surfacing accessor; until one exists the field
    // itself is unread, hence the explicit allow.
    #[allow(dead_code)]
    pnames: Vec<String>,
    patches: Vec<Option<Picture>>, // parallel to pnames
    warnings: Vec<GfxWarning>,
}

impl TextureSet {
    // Long like the crate's other policy orchestrators (`TextureX::parse`,
    // `Picture::parse`): one strict/lenient fork per build stage.
    #[allow(clippy::too_many_lines)]
    pub(crate) fn from_wad(
        wad: &crate::Wad,
        options: &crate::ParseOptions,
    ) -> Result<Option<Self>, GfxError> {
        let strictness = options.strictness;
        let mut warnings = Vec::new();

        let mut textures = Vec::new();
        let mut any_lump = false;
        for lump_name in ["TEXTURE1", "TEXTURE2"] {
            if let Some(lump) = wad.lump_by_name(lump_name) {
                any_lump = true;
                // Same-module destructure: move the parsed vectors instead
                // of cloning hundreds of defs per retail lump.
                let TextureX {
                    textures: parsed_textures,
                    warnings: parsed_warnings,
                } = TextureX::parse(wad.lump_data(lump), options)?;
                warnings.extend(parsed_warnings);
                textures.extend(parsed_textures);
            }
        }
        if !any_lump {
            return Ok(None);
        }

        let mut pnames_present = true;
        let pnames = match wad.lump_by_name("PNAMES") {
            Some(lump) => {
                // Same-module destructure: move the name strings out rather
                // than re-allocating each one.
                let Pnames {
                    names,
                    warnings: pnames_warnings,
                } = Pnames::parse(wad.lump_data(lump), options)?;
                warnings.extend(pnames_warnings);
                names
            }
            None => match strictness {
                crate::Strictness::Strict => return Err(GfxError::MissingPnames),
                crate::Strictness::Lenient => {
                    warnings.push(GfxWarning::MissingPnames);
                    pnames_present = false;
                    Vec::new()
                }
            },
        };

        // Validate every ref's index once, at build (set-level warnings).
        // When PNAMES is absent entirely, `MissingPnames` above already
        // explains every reference — per-ref bounds warnings would be a
        // low-signal blast (hundreds for a real TEXTUREx), so they are
        // suppressed; a PRESENT but short/empty PNAMES still warns per ref.
        for (t, def) in textures.iter().enumerate() {
            for patch_ref in &def.patches {
                if patch_ref.patch < 0 || usize::try_from(patch_ref.patch).unwrap() >= pnames.len()
                {
                    match strictness {
                        crate::Strictness::Strict => {
                            return Err(GfxError::PatchIndexOutOfBounds {
                                texture: t,
                                patch: patch_ref.patch,
                                pnames_len: pnames.len(),
                            });
                        }
                        crate::Strictness::Lenient => {
                            if pnames_present {
                                warnings.push(GfxWarning::PatchIndexOutOfBounds {
                                    texture: t,
                                    patch: patch_ref.patch,
                                    pnames_len: pnames.len(),
                                });
                            }
                        }
                    }
                }
            }
        }

        // Resolve + parse each REFERENCED name once.
        let mut referenced = vec![false; pnames.len()];
        for def in &textures {
            for patch_ref in &def.patches {
                if let Ok(i) = usize::try_from(patch_ref.patch)
                    && i < pnames.len()
                {
                    referenced[i] = true;
                }
            }
        }
        let mut patches: Vec<Option<Picture>> = vec![None; pnames.len()];
        for (i, name) in pnames.iter().enumerate() {
            if !referenced[i] {
                continue;
            }
            // ASCII uppercase only: WAD lump names are ASCII bytes and vanilla's
            // toupper is C-locale — Unicode case mapping could change length.
            let upper = name.to_ascii_uppercase();
            let Some(lump) = wad.lump_by_name(&upper) else {
                match strictness {
                    crate::Strictness::Strict => {
                        return Err(GfxError::UnresolvedPatchName { name: name.clone() });
                    }
                    crate::Strictness::Lenient => {
                        warnings.push(GfxWarning::UnresolvedPatchName { name: name.clone() });
                        continue;
                    }
                }
            };
            match Picture::parse(wad.lump_data(lump), options) {
                Ok(picture) => patches[i] = Some(picture),
                Err(source) => match strictness {
                    crate::Strictness::Strict => {
                        return Err(GfxError::PatchPictureFailed {
                            name: name.clone(),
                            source: Box::new(source),
                        });
                    }
                    crate::Strictness::Lenient => {
                        warnings.push(GfxWarning::PatchPictureFailed { name: name.clone() });
                    }
                },
            }
        }

        Ok(Some(Self {
            textures,
            pnames,
            patches,
            warnings,
        }))
    }

    /// The texture definitions: `TEXTURE1`'s then `TEXTURE2`'s, lump order.
    #[must_use]
    pub fn textures(&self) -> &[TextureDef] {
        &self.textures
    }

    /// First texture with this exact, case-sensitive name (vanilla's
    /// "earlier entries win"; retail names are uppercase — vanilla
    /// uppercases its query, this method does not).
    #[must_use]
    pub fn find(&self, name: &str) -> Option<usize> {
        self.textures.iter().position(|t| t.name == name)
    }

    /// Set-level non-fatal issues from lenient building (lump parses,
    /// index validation, patch resolution). Per-compose issues are
    /// returned by [`TextureSet::compose`] instead.
    #[must_use]
    pub fn warnings(&self) -> &[GfxWarning] {
        &self.warnings
    }

    /// Composes one texture into an indexed image + coverage mask,
    /// reimplementing the `R_GenerateComposite` contract (ADR-0022 §3):
    /// patches draw in def order (later placements overwrite), horizontal
    /// placement is clamped to the canvas, rows outside the canvas are
    /// silently clipped (vanilla behavior), and a column no live patch
    /// spans is the Medusa case — strict error, lenient warning + hole.
    ///
    /// Returns the composed image plus per-compose warnings (set-level
    /// warnings stay on [`TextureSet::warnings`]).
    ///
    /// # Errors
    ///
    /// [`GfxError::CompositeTooLarge`] in BOTH modes when
    /// `width × height` exceeds
    /// [`Limits::max_composite_pixels`](crate::Limits::max_composite_pixels);
    /// strict mode additionally: [`GfxError::NegativeDimension`] and
    /// [`GfxError::MedusaColumn`].
    ///
    /// # Panics
    ///
    /// Panics if `index >= self.textures().len()` (obtain indices from
    /// [`TextureSet::find`] or by iterating [`TextureSet::textures`]).
    pub fn compose(
        &self,
        index: usize,
        options: &crate::ParseOptions,
    ) -> Result<(super::IndexedImage, Vec<GfxWarning>), GfxError> {
        let strictness = options.strictness;
        let def = &self.textures[index];
        let mut warnings = Vec::new();
        let width = super::picture::clamp_dimension(def.width, "width", strictness, &mut warnings)?;
        let height =
            super::picture::clamp_dimension(def.height, "height", strictness, &mut warnings)?;
        let (w, h) = (usize::from(width), usize::from(height));
        if w * h > options.limits.max_composite_pixels {
            // Both modes: a DoS cap, not a recoverable anomaly (spec).
            return Err(GfxError::CompositeTooLarge {
                width: def.width,
                height: def.height,
                max_pixels: options.limits.max_composite_pixels,
            });
        }

        let mut pixels = vec![0u8; w * h];
        let mut mask = vec![false; w * h];
        let mut contributors = vec![0u32; w];

        for patch_ref in &def.patches {
            let Some(picture) = usize::try_from(patch_ref.patch)
                .ok()
                .filter(|i| *i < self.patches.len())
                .and_then(|i| self.patches[i].as_ref())
            else {
                continue; // dead ref: warned at build, not a contributor
            };
            let x1 = i32::from(patch_ref.origin_x).max(0);
            let x2 = (i32::from(patch_ref.origin_x) + i32::from(picture.width))
                .min(i32::try_from(w).unwrap_or(i32::MAX));
            for x in x1..x2 {
                #[allow(clippy::cast_sign_loss)] // x1 >= 0
                let xu = x as usize;
                contributors[xu] += 1;
                #[allow(clippy::cast_sign_loss)] // x >= x1 >= origin_x
                let px = (x - i32::from(patch_ref.origin_x)) as usize;
                for post in &picture.columns()[px].posts {
                    for (i, &value) in post.pixels.iter().enumerate() {
                        let y = i32::from(patch_ref.origin_y)
                            + i32::from(post.top_delta)
                            + i32::try_from(i).unwrap_or(i32::MAX);
                        // Rows outside the canvas silently clip (vanilla
                        // R_DrawColumnInCache behavior — not an anomaly).
                        if let Ok(yu) = usize::try_from(y)
                            && yu < h
                        {
                            pixels[yu * w + xu] = value;
                            mask[yu * w + xu] = true;
                        }
                    }
                }
            }
        }

        // Medusa scan: columns no LIVE patch spans (dead refs never count —
        // documented divergence from vanilla, which counts refs from the
        // def regardless of lookup failure; only-live is what the explicit
        // holes model requires).
        let mut medusa_iter = contributors.iter().enumerate().filter(|(_, c)| **c == 0);
        if let Some((first_column, _)) = medusa_iter.next() {
            let count = 1 + medusa_iter.count();
            match strictness {
                crate::Strictness::Strict => {
                    return Err(GfxError::MedusaColumn {
                        column: first_column,
                    });
                }
                crate::Strictness::Lenient => {
                    warnings.push(GfxWarning::MedusaColumns {
                        first_column,
                        count,
                    });
                }
            }
        }

        Ok((
            super::IndexedImage {
                width,
                height,
                pixels,
                mask,
            },
            warnings,
        ))
    }

    /// [`TextureSet::compose`] plus palette application (tier 3).
    ///
    /// # Errors
    ///
    /// Exactly [`TextureSet::compose`]'s errors.
    ///
    /// # Panics
    ///
    /// Exactly [`TextureSet::compose`]'s panic condition.
    pub fn compose_rgba(
        &self,
        index: usize,
        options: &crate::ParseOptions,
        palette: &super::Palette,
    ) -> Result<(super::RgbaImage, Vec<GfxWarning>), GfxError> {
        let (image, warnings) = self.compose(index, options)?;
        Ok((image.to_rgba(palette), warnings))
    }
}
