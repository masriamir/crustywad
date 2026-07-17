//! Classic Doom graphics lumps (ADR-0022 §3): the picture format used by
//! patches and sprites, raw 64×64 flats, the `PLAYPAL` palette collection,
//! and the `COLORMAP` light-diminishing tables. Decoding is dependency-free
//! and lives in the core crate (the map-parsing precedent — no feature
//! flag). Doom 64 graphics are a different family (PNG lumps) and are not
//! handled here.

mod flat;
mod palette;
mod picture;
mod texture;

pub use flat::Flat;
pub use palette::{Colormap, Palette, Playpal};
pub use picture::{Column, IndexedImage, Picture, Post, RgbaImage};
pub use texture::{Pnames, TextureDef, TexturePatchRef, TextureX};

/// A fatal problem decoding a classic graphics lump in strict mode; every
/// variant's lenient recovery is described on the matching [`GfxWarning`].
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum GfxError {
    /// Lump too short for the picture header (< 8 bytes; unrecoverable in
    /// both modes) or, in strict mode, for its column offset table
    /// (8 + width×4 bytes; lenient clamps the width to the offsets present).
    #[error("picture lump is {len} bytes; {needed} needed")]
    TruncatedPicture {
        /// The lump's actual length.
        len: usize,
        /// Bytes required (8 for the header, or 8 + width×4 with the table).
        needed: usize,
    },
    /// A negative picture width or height (lenient clamps the field to 0).
    #[error("picture {field} is negative ({value})")]
    NegativeDimension {
        /// `"width"` or `"height"`.
        field: &'static str,
        /// The raw on-disk value.
        value: i16,
    },
    /// A column offset points outside the lump (lenient records an empty
    /// column instead). Vanilla dereferences these unchecked (ADR-0022 §6).
    #[error("column {column} offset {offset} is outside the {len}-byte lump")]
    ColumnOffsetOutOfBounds {
        /// 0-based column index.
        column: usize,
        /// The raw on-disk offset (from lump start).
        offset: i32,
        /// The lump's length.
        len: usize,
    },
    /// A post chain ran past the lump end without a `0xFF` terminator
    /// (lenient keeps the posts fully read).
    #[error("column {column} post chain ran past the lump end without a 0xFF terminator")]
    UnterminatedColumn {
        /// 0-based column index.
        column: usize,
    },
    /// A post's rows exceed the picture height (lenient clips it). Vanilla
    /// writes these out of bounds (ADR-0022 §6).
    #[error(
        "column {column} post (top {top_delta}, length {length}) exceeds picture height {height}"
    )]
    PostOutOfBounds {
        /// 0-based column index.
        column: usize,
        /// The post's starting row.
        top_delta: u8,
        /// The post's pixel count.
        length: u8,
        /// The picture height the post exceeds.
        height: u16,
    },
    /// Cumulative post-chain bytes consumed exceeded the lump length — only
    /// possible when column offsets alias the same bytes (lenient stops
    /// decoding further columns). Bounds parse work and memory to
    /// `O(lump length)` in both modes (ADR-0016 §1).
    #[error(
        "cumulative post data exceeded the {len}-byte lump at column {column}; column offsets alias the same bytes"
    )]
    ExcessivePostData {
        /// The column at which the budget ran out.
        column: usize,
        /// The lump's length (the budget).
        len: usize,
    },
    /// `PLAYPAL` length is not a positive multiple of 768 (lenient
    /// truncates the remainder; zero palettes also warns).
    #[error("PLAYPAL length {len} is not a positive multiple of 768")]
    PlaypalSize {
        /// The lump's actual length.
        len: usize,
    },
    /// COLORMAP length is not a whole number of 256-byte tables totaling
    /// at least 8192 bytes — vanilla's 32-table floor (lenient zero-pads
    /// short lumps to 8192 / truncates a trailing partial table). Retail
    /// lumps carry 34 tables (ADR-0022 §3 correction amendment).
    #[error("COLORMAP length {len} is not a 256-byte multiple of at least 8192")]
    ColormapSize {
        /// The lump's actual length.
        len: usize,
    },
    /// Flat length is not a whole number of 64-pixel rows totaling at
    /// least 4096 bytes (lenient keeps the actual bytes; the rendered view
    /// is always the first 64×64). Heretic ships 4160-byte and Hexen
    /// 8192-byte flats (ADR-0022 §3 correction amendment).
    #[error("flat length {len} is not a 64-byte multiple of at least 4096")]
    FlatSize {
        /// The lump's actual length.
        len: usize,
    },
    /// `PNAMES` lump is too short for its 4-byte count field (unrecoverable
    /// in strict mode; lenient treats the count as 0).
    #[error("PNAMES lump is {len} bytes; at least 4 needed for the count")]
    TruncatedPnames {
        /// The lump's actual length.
        len: usize,
    },
    /// `PNAMES` declared a negative name count (lenient treats the count
    /// as 0).
    #[error("PNAMES count {count} is negative")]
    NegativePnamesCount {
        /// The raw on-disk count.
        count: i32,
    },
    /// `PNAMES` declared more names than the lump holds (lenient clamps the
    /// count to the names actually present).
    #[error("PNAMES count {count} needs more bytes than the {len}-byte lump holds")]
    PnamesCountExceedsLump {
        /// The declared count.
        count: i32,
        /// The lump's actual length.
        len: usize,
    },
    /// `TEXTUREx` lump is too short for its header or offset table
    /// (lenient parses no textures).
    #[error("TEXTUREx lump is {len} bytes; {needed} needed")]
    TruncatedTextureX {
        /// The lump's actual length.
        len: usize,
        /// The bytes required.
        needed: usize,
    },
    /// `TEXTUREx` declared a negative texture count (lenient treats the
    /// count as 0).
    #[error("TEXTUREx texture count {count} is negative")]
    NegativeTextureCount {
        /// The raw on-disk count.
        count: i32,
    },
    /// A texture's offset points outside the lump (lenient skips the
    /// texture).
    #[error("texture {texture} offset {offset} is outside the {len}-byte lump")]
    TextureOffsetOutOfBounds {
        /// 0-based texture index.
        texture: usize,
        /// The raw on-disk offset.
        offset: i32,
        /// The lump's actual length.
        len: usize,
    },
    /// A texture declared a negative patch count (lenient keeps the
    /// texture with no patch references).
    #[error("texture {texture} declares a negative patch count {count}")]
    NegativePatchCount {
        /// 0-based texture index.
        texture: usize,
        /// The raw on-disk patch count.
        count: i16,
    },
    /// A texture's full extent (header + patch references) runs past the
    /// lump (lenient clamps to the patch references in bounds).
    #[error("texture {texture} extends to byte {needed}, past the {len}-byte lump")]
    TextureExtentOutOfBounds {
        /// 0-based texture index.
        texture: usize,
        /// The byte offset the texture's declared extent requires.
        needed: usize,
        /// The lump's actual length.
        len: usize,
    },
    /// Cumulative texture-data bytes consumed exceeded the lump length —
    /// only possible when texture offsets alias the same bytes (lenient
    /// stops decoding further textures). Bounds parse work and memory to
    /// `O(lump length)` in both modes (ADR-0016 §1).
    #[error(
        "cumulative texture data exceeded the {len}-byte lump at texture {texture}; offsets alias the same bytes"
    )]
    ExcessiveTextureData {
        /// The texture at which the budget ran out.
        texture: usize,
        /// The lump's actual length (the budget).
        len: usize,
    },
}

/// A non-fatal issue recovered while decoding a classic graphics lump in
/// lenient mode; mirrors the strict [`GfxError`] rows one-to-one.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GfxWarning {
    /// The column offset table was truncated; width was clamped to the
    /// offsets actually present during lenient parsing.
    #[error(
        "picture offset table truncated ({len} of {needed} bytes); width clamped during lenient parsing"
    )]
    TruncatedPicture {
        /// The lump's actual length.
        len: usize,
        /// Bytes the declared width required.
        needed: usize,
    },
    /// A negative dimension was clamped to 0 during lenient parsing.
    #[error("picture {field} was negative ({value}); clamped to 0 during lenient parsing")]
    NegativeDimension {
        /// `"width"` or `"height"`.
        field: &'static str,
        /// The raw on-disk value.
        value: i16,
    },
    /// An out-of-bounds column offset; the column was left empty during
    /// lenient parsing.
    #[error(
        "column {column} offset {offset} was outside the {len}-byte lump; column left empty during lenient parsing"
    )]
    ColumnOffsetOutOfBounds {
        /// 0-based column index.
        column: usize,
        /// The raw on-disk offset.
        offset: i32,
        /// The lump's length.
        len: usize,
    },
    /// An unterminated post chain; the posts fully read were kept during
    /// lenient parsing.
    #[error(
        "column {column} post chain ran past the lump end; kept the posts fully read during lenient parsing"
    )]
    UnterminatedColumn {
        /// 0-based column index.
        column: usize,
    },
    /// An out-of-bounds post was clipped to the picture height (or dropped
    /// when no rows remained) during lenient parsing.
    #[error(
        "column {column} post (top {top_delta}, length {length}) exceeded picture height {height}; clipped during lenient parsing"
    )]
    PostOutOfBounds {
        /// 0-based column index.
        column: usize,
        /// The post's starting row.
        top_delta: u8,
        /// The post's pixel count.
        length: u8,
        /// The picture height the post exceeded.
        height: u16,
    },
    /// The consumed-post-bytes budget ran out; remaining columns were left
    /// empty during lenient parsing.
    #[error(
        "cumulative post data exceeded the {len}-byte lump at column {column}; remaining columns left empty during lenient parsing"
    )]
    ExcessivePostData {
        /// The column at which the budget ran out.
        column: usize,
        /// The lump's length (the budget).
        len: usize,
    },
    /// A `PLAYPAL` remainder (or the whole zero-length lump) was dropped
    /// during lenient parsing.
    #[error(
        "PLAYPAL length {len} is not a positive multiple of 768; remainder truncated during lenient parsing"
    )]
    PlaypalSize {
        /// The lump's actual length.
        len: usize,
    },
    /// A wrong-size `COLORMAP` was zero-padded or truncated to a whole
    /// number of tables during lenient parsing.
    #[error(
        "COLORMAP length {len} is not a 256-byte multiple of at least 8192; zero-padded to 8192 or truncated to whole tables during lenient parsing"
    )]
    ColormapSize {
        /// The lump's actual length.
        len: usize,
    },
    /// A wrong-size flat was kept as-is during lenient parsing.
    #[error(
        "flat length {len} is not a 64-byte multiple of at least 4096; kept as-is during lenient parsing"
    )]
    FlatSize {
        /// The lump's actual length.
        len: usize,
    },
    /// `PNAMES` was too short for its count field; count treated as 0
    /// during lenient parsing.
    #[error(
        "PNAMES lump is {len} bytes; at least 4 needed for the count; count treated as 0 during lenient parsing"
    )]
    TruncatedPnames {
        /// The lump's actual length.
        len: usize,
    },
    /// A negative `PNAMES` count; count treated as 0 during lenient
    /// parsing.
    #[error("PNAMES count {count} is negative; count treated as 0 during lenient parsing")]
    NegativePnamesCount {
        /// The raw on-disk count.
        count: i32,
    },
    /// A `PNAMES` count exceeding the lump; count clamped to the names
    /// present during lenient parsing.
    #[error(
        "PNAMES count {count} needs more bytes than the {len}-byte lump holds; count clamped to the names present during lenient parsing"
    )]
    PnamesCountExceedsLump {
        /// The declared count.
        count: i32,
        /// The lump's actual length.
        len: usize,
    },
    /// `TEXTUREx` was too short for its header or offset table; no
    /// textures parsed during lenient parsing.
    #[error(
        "TEXTUREx lump is {len} bytes; {needed} needed; no textures parsed during lenient parsing"
    )]
    TruncatedTextureX {
        /// The lump's actual length.
        len: usize,
        /// The bytes required.
        needed: usize,
    },
    /// A negative `TEXTUREx` texture count; count treated as 0 during
    /// lenient parsing.
    #[error(
        "TEXTUREx texture count {count} is negative; count treated as 0 during lenient parsing"
    )]
    NegativeTextureCount {
        /// The raw on-disk count.
        count: i32,
    },
    /// A texture offset outside the lump; texture skipped during lenient
    /// parsing.
    #[error(
        "texture {texture} offset {offset} is outside the {len}-byte lump; texture skipped during lenient parsing"
    )]
    TextureOffsetOutOfBounds {
        /// 0-based texture index.
        texture: usize,
        /// The raw on-disk offset.
        offset: i32,
        /// The lump's actual length.
        len: usize,
    },
    /// A negative patch count; texture kept with no patch references
    /// during lenient parsing.
    #[error(
        "texture {texture} declares a negative patch count {count}; texture kept with no patch references during lenient parsing"
    )]
    NegativePatchCount {
        /// 0-based texture index.
        texture: usize,
        /// The raw on-disk patch count.
        count: i16,
    },
    /// A texture's extent ran past the lump; references clamped to those
    /// in bounds during lenient parsing.
    #[error(
        "texture {texture} extends to byte {needed}, past the {len}-byte lump; references clamped to those in bounds during lenient parsing"
    )]
    TextureExtentOutOfBounds {
        /// 0-based texture index.
        texture: usize,
        /// The byte offset the texture's declared extent requires.
        needed: usize,
        /// The lump's actual length.
        len: usize,
    },
    /// The consumed-texture-bytes budget ran out; remaining textures
    /// skipped during lenient parsing.
    #[error(
        "cumulative texture data exceeded the {len}-byte lump at texture {texture}; remaining textures skipped during lenient parsing"
    )]
    ExcessiveTextureData {
        /// The texture at which the budget ran out.
        texture: usize,
        /// The lump's actual length (the budget).
        len: usize,
    },
}
