//! Doom map lump record types.
//!
//! These types intentionally stop at the individual record level for now. A future
//! milestone will assemble them into a richer `Map` graph once the crate has stable
//! building blocks for the raw lump data.

use std::io::Cursor;

use binrw::{BinRead, BinReaderExt};
use thiserror::Error;

/// A fixed-width 8-byte Doom name field.
#[derive(Debug, Clone, PartialEq, Eq, BinRead)]
pub struct Name8(
    /// The raw 8-byte encoded name.
    pub [u8; 8],
);

impl Name8 {
    /// Returns the name with trailing NUL padding removed.
    #[must_use]
    pub fn as_str_lossy(&self) -> String {
        let end = self
            .0
            .iter()
            .position(|&byte| byte == 0)
            .unwrap_or(self.0.len());
        String::from_utf8_lossy(&self.0[..end]).into_owned()
    }
}

/// A THINGS record.
#[derive(Debug, Clone, PartialEq, Eq, BinRead)]
#[br(little)]
pub struct Thing {
    /// X coordinate in map units.
    pub x: i16,
    /// Y coordinate in map units.
    pub y: i16,
    /// Facing angle in degrees (0–359, clockwise from east).
    pub angle: u16,
    /// Editor number / thing type.
    pub type_id: u16,
    /// Doom thing flags.
    pub flags: u16,
}

/// A LINEDEFS record.
#[derive(Debug, Clone, PartialEq, Eq, BinRead)]
#[br(little)]
pub struct Linedef {
    /// Start vertex index.
    pub start_vertex: u16,
    /// End vertex index.
    pub end_vertex: u16,
    /// Linedef flags.
    pub flags: u16,
    /// Special action.
    pub special_type: u16,
    /// Sector tag.
    pub sector_tag: u16,
    /// Right sidedef index.
    pub right_sidedef: u16,
    /// Left sidedef index, or `0xffff` when absent.
    pub left_sidedef: u16,
}

/// A SIDEDEFS record.
#[derive(Debug, Clone, PartialEq, Eq, BinRead)]
#[br(little)]
pub struct Sidedef {
    /// Horizontal texture offset.
    pub x_offset: i16,
    /// Vertical texture offset.
    pub y_offset: i16,
    /// Upper texture name.
    pub upper_texture: Name8,
    /// Lower texture name.
    pub lower_texture: Name8,
    /// Middle texture name.
    pub middle_texture: Name8,
    /// Owning sector index.
    pub sector: u16,
}

/// A VERTEXES record.
#[derive(Debug, Clone, PartialEq, Eq, BinRead)]
#[br(little)]
pub struct Vertex {
    /// X coordinate in map units.
    pub x: i16,
    /// Y coordinate in map units.
    pub y: i16,
}

/// A SEGS record.
#[derive(Debug, Clone, PartialEq, Eq, BinRead)]
#[br(little)]
pub struct Seg {
    /// Start vertex index.
    pub start_vertex: u16,
    /// End vertex index.
    pub end_vertex: u16,
    /// Binary angle.
    pub angle: i16,
    /// Parent linedef index.
    pub linedef: u16,
    /// Direction flag.
    pub direction: u16,
    /// Offset along the linedef in map units (signed; negative when the seg
    /// starts before the linedef's start vertex).
    pub offset: i16,
}

/// An SSECTORS record.
#[derive(Debug, Clone, PartialEq, Eq, BinRead)]
#[br(little)]
pub struct Subsector {
    /// Number of segs in the subsector.
    pub seg_count: u16,
    /// Index of the first seg.
    pub first_seg: u16,
}

/// A NODES record.
#[derive(Debug, Clone, PartialEq, Eq, BinRead)]
#[br(little)]
pub struct Node {
    /// Partition line origin X.
    pub x: i16,
    /// Partition line origin Y.
    pub y: i16,
    /// Partition line delta X.
    pub dx: i16,
    /// Partition line delta Y.
    pub dy: i16,
    /// Right bounding box top, bottom, left, right.
    pub right_bbox: [i16; 4],
    /// Left bounding box top, bottom, left, right.
    pub left_bbox: [i16; 4],
    /// Right child index.
    pub right_child: u16,
    /// Left child index.
    pub left_child: u16,
}

/// A SECTORS record.
#[derive(Debug, Clone, PartialEq, Eq, BinRead)]
#[br(little)]
pub struct Sector {
    /// Floor height.
    pub floor_height: i16,
    /// Ceiling height.
    pub ceiling_height: i16,
    /// Floor texture name.
    pub floor_texture: Name8,
    /// Ceiling texture name.
    pub ceiling_texture: Name8,
    /// Light level.
    pub light_level: i16,
    /// Sector special type.
    pub special_type: i16,
    /// Sector tag.
    pub tag: i16,
}

/// Placeholder for the REJECT lump.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RejectLump;

/// Placeholder for the BLOCKMAP lump.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BlockmapLump;

/// Errors returned when decoding typed map records.
#[derive(Debug, Error)]
pub enum MapParseError {
    /// The record stream length was not a whole number of records.
    #[error("record stream ended mid-record at byte offset {offset}")]
    TrailingBytes {
        /// The cursor position where parsing stopped.
        offset: u64,
    },
    /// `binrw` failed to decode a record.
    #[error("failed to parse map records: {0}")]
    Binrw(#[from] binrw::Error),
}

/// Parses a map lump into a vector of records of the requested type.
///
/// # Errors
///
/// Returns [`MapParseError::TrailingBytes`] if the byte slice length is not an
/// exact multiple of `size_of::<T>()`, and [`MapParseError::Binrw`] if
/// `binrw` cannot decode a record.
pub fn parse_records<T>(bytes: &[u8]) -> Result<Vec<T>, MapParseError>
where
    T: for<'a> BinRead<Args<'a> = ()>,
{
    let record_size = std::mem::size_of::<T>();
    if record_size == 0 {
        // ZST records have no binary representation. An empty buffer produces
        // zero records; any non-empty buffer has unresolvable trailing bytes.
        return if bytes.is_empty() {
            Ok(Vec::new())
        } else {
            Err(MapParseError::TrailingBytes { offset: 0 })
        };
    }
    if bytes.len() % record_size != 0 {
        return Err(MapParseError::TrailingBytes {
            offset: (bytes.len() / record_size * record_size) as u64,
        });
    }

    let mut cursor = Cursor::new(bytes);
    let mut records = Vec::with_capacity(bytes.len() / record_size);
    let bytes_len = bytes.len() as u64;

    while cursor.position() < bytes_len {
        records.push(cursor.read_le()?);
    }

    Ok(records)
}
