//! The classic picture format (ADR-0022 §3): patches and sprites. An 8-byte
//! header (four little-endian `i16`: width, height, left offset, top
//! offset), then `width` little-endian `i32` column offsets counted from
//! the lump start, each a chain of posts — `top_delta` (`u8`, `0xFF`
//! terminates), `length` (`u8`), a pad byte, `length` pixels, a pad byte.
//! `top_delta` is plain, not cumulative (no tall-patch handling in
//! vanilla). Vanilla validates none of this; every bound here closes a
//! vector from ADR-0022 §6's hardening table.

use crate::{ParseOptions, Strictness};

use super::{GfxError, GfxWarning};

/// A parsed picture: faithful post structure plus draw offsets. Views over
/// the structure (indexed grid, RGBA8) are separate methods so texture
/// composition (#157) can consume posts directly.
#[derive(Debug, Clone)]
pub struct Picture {
    /// Width in pixels (columns). Validated non-negative at parse.
    pub width: u16,
    /// Height in pixels. Validated non-negative at parse; every post is
    /// validated (or lenient-clipped) to fit within it.
    pub height: u16,
    /// Signed horizontal draw offset (sprite positioning; negative is
    /// legitimate on disk).
    pub left_offset: i16,
    /// Signed vertical draw offset.
    pub top_offset: i16,
    columns: Vec<Column>,
    warnings: Vec<GfxWarning>,
}

/// One column: a chain of posts in lump order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    /// The column's posts, in chain order (a later overlapping post draws
    /// over an earlier one, matching vanilla's draw order).
    pub posts: Vec<Post>,
}

/// One post: a vertical run of pixels starting at `top_delta` rows from the
/// column top.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Post {
    /// Starting row (plain offset from the column top, not cumulative).
    pub top_delta: u8,
    /// The run's palette indices, top to bottom.
    pub pixels: Vec<u8>,
}

impl Picture {
    /// Parses a picture lump.
    ///
    /// # Errors
    ///
    /// Strict mode: the first violation among the policy rows —
    /// [`GfxError::TruncatedPicture`], [`GfxError::NegativeDimension`],
    /// [`GfxError::ColumnOffsetOutOfBounds`],
    /// [`GfxError::UnterminatedColumn`], [`GfxError::PostOutOfBounds`],
    /// [`GfxError::ExcessivePostData`]. Lenient mode recovers each per its
    /// [`GfxWarning`] counterpart and fails only for a lump under 8 bytes.
    #[allow(clippy::too_many_lines)]
    pub fn parse(bytes: &[u8], options: &ParseOptions) -> Result<Self, GfxError> {
        let strictness = options.strictness;
        let mut warnings = Vec::new();
        if bytes.len() < 8 {
            return Err(GfxError::TruncatedPicture {
                len: bytes.len(),
                needed: 8,
            });
        }
        let raw_width = i16::from_le_bytes([bytes[0], bytes[1]]);
        let raw_height = i16::from_le_bytes([bytes[2], bytes[3]]);
        let left_offset = i16::from_le_bytes([bytes[4], bytes[5]]);
        let top_offset = i16::from_le_bytes([bytes[6], bytes[7]]);
        let mut width = clamp_dimension(raw_width, "width", strictness, &mut warnings)?;
        let height = clamp_dimension(raw_height, "height", strictness, &mut warnings)?;

        let needed = 8 + usize::from(width) * 4;
        if bytes.len() < needed {
            match strictness {
                Strictness::Strict => {
                    return Err(GfxError::TruncatedPicture {
                        len: bytes.len(),
                        needed,
                    });
                }
                Strictness::Lenient => {
                    warnings.push(GfxWarning::TruncatedPicture {
                        len: bytes.len(),
                        needed,
                    });
                    #[allow(clippy::cast_possible_truncation)] // (len-8)/4 < len <= i16::MAX*4+8
                    {
                        width = ((bytes.len() - 8) / 4) as u16;
                    }
                }
            }
        }

        // ADR-0016 §1: cumulative CONSUMED post-chain bytes (4 + length per
        // post) across all columns is capped at the lump length. Offsets may
        // alias the same bytes (vanilla allows it); without the cap, aliased
        // zero-length posts make parse work O(width × len). A faithful
        // picture partitions its lump, so real WADs never trip this.
        let mut budget = bytes.len();
        let mut stopped = false;
        let mut columns = Vec::with_capacity(usize::from(width));
        for column in 0..usize::from(width) {
            if stopped {
                columns.push(Column { posts: Vec::new() });
                continue;
            }
            let table_at = 8 + column * 4;
            let offset = i32::from_le_bytes([
                bytes[table_at],
                bytes[table_at + 1],
                bytes[table_at + 2],
                bytes[table_at + 3],
            ]);
            let Ok(start) = usize::try_from(offset) else {
                column_offset_issue(column, offset, bytes.len(), strictness, &mut warnings)?;
                columns.push(Column { posts: Vec::new() });
                continue;
            };
            if start >= bytes.len() {
                column_offset_issue(column, offset, bytes.len(), strictness, &mut warnings)?;
                columns.push(Column { posts: Vec::new() });
                continue;
            }

            let mut posts = Vec::new();
            let mut pos = start;
            loop {
                if pos >= bytes.len() {
                    match strictness {
                        Strictness::Strict => {
                            return Err(GfxError::UnterminatedColumn { column });
                        }
                        Strictness::Lenient => {
                            warnings.push(GfxWarning::UnterminatedColumn { column });
                            break;
                        }
                    }
                }
                let top_delta = bytes[pos];
                if top_delta == 0xFF {
                    break;
                }
                if pos + 2 > bytes.len() {
                    match strictness {
                        Strictness::Strict => {
                            return Err(GfxError::UnterminatedColumn { column });
                        }
                        Strictness::Lenient => {
                            warnings.push(GfxWarning::UnterminatedColumn { column });
                            break;
                        }
                    }
                }
                let length = bytes[pos + 1];
                let full = 4 + usize::from(length); // header + pads + pixels
                if pos + full > bytes.len() {
                    match strictness {
                        Strictness::Strict => {
                            return Err(GfxError::UnterminatedColumn { column });
                        }
                        Strictness::Lenient => {
                            warnings.push(GfxWarning::UnterminatedColumn { column });
                            break;
                        }
                    }
                }
                if full > budget {
                    match strictness {
                        Strictness::Strict => {
                            return Err(GfxError::ExcessivePostData {
                                column,
                                len: bytes.len(),
                            });
                        }
                        Strictness::Lenient => {
                            warnings.push(GfxWarning::ExcessivePostData {
                                column,
                                len: bytes.len(),
                            });
                            stopped = true;
                            break;
                        }
                    }
                }
                budget -= full;

                let end_row = u16::from(top_delta) + u16::from(length);
                let keep = if end_row > height {
                    match strictness {
                        Strictness::Strict => {
                            return Err(GfxError::PostOutOfBounds {
                                column,
                                top_delta,
                                length,
                                height,
                            });
                        }
                        Strictness::Lenient => {
                            warnings.push(GfxWarning::PostOutOfBounds {
                                column,
                                top_delta,
                                length,
                                height,
                            });
                            usize::from(height.saturating_sub(u16::from(top_delta)))
                        }
                    }
                } else {
                    usize::from(length)
                };
                if keep > 0 {
                    posts.push(Post {
                        top_delta,
                        pixels: bytes[pos + 3..pos + 3 + keep].to_vec(),
                    });
                }
                pos += full;
            }
            columns.push(Column { posts });
        }

        Ok(Self {
            width,
            height,
            left_offset,
            top_offset,
            columns,
            warnings,
        })
    }

    /// The columns, left to right (`columns().len() == width`).
    #[must_use]
    pub fn columns(&self) -> &[Column] {
        &self.columns
    }

    /// Non-fatal issues recovered during lenient parsing.
    #[must_use]
    pub fn warnings(&self) -> &[GfxWarning] {
        &self.warnings
    }
}

/// Validates a header dimension: strict errors on negative; lenient clamps
/// to 0 and warns.
fn clamp_dimension(
    value: i16,
    field: &'static str,
    strictness: Strictness,
    warnings: &mut Vec<GfxWarning>,
) -> Result<u16, GfxError> {
    if value >= 0 {
        #[allow(clippy::cast_sign_loss)] // non-negative checked above
        return Ok(value as u16);
    }
    match strictness {
        Strictness::Strict => Err(GfxError::NegativeDimension { field, value }),
        Strictness::Lenient => {
            warnings.push(GfxWarning::NegativeDimension { field, value });
            Ok(0)
        }
    }
}

/// Handles an out-of-range column offset: strict errors; lenient warns (the
/// caller then records an empty column).
fn column_offset_issue(
    column: usize,
    offset: i32,
    len: usize,
    strictness: Strictness,
    warnings: &mut Vec<GfxWarning>,
) -> Result<(), GfxError> {
    match strictness {
        Strictness::Strict => Err(GfxError::ColumnOffsetOutOfBounds {
            column,
            offset,
            len,
        }),
        Strictness::Lenient => {
            warnings.push(GfxWarning::ColumnOffsetOutOfBounds {
                column,
                offset,
                len,
            });
            Ok(())
        }
    }
}
