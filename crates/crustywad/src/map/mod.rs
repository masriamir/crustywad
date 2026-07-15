//! Doom map lump record types.
//!
//! A Doom map is stored as a group of lumps whose names follow a well-known
//! sequence (e.g. `E1M1`, `THINGS`, `LINEDEFS`, …). Record structs are
//! organized by map format:
//!
//! - [`doom`] — the classic Doom binary layout ([`Thing`][doom::Thing],
//!   [`Linedef`][doom::Linedef]); also used by Heretic.
//! - [`hexen`] — the Hexen binary layout ([`Thing`][hexen::Thing],
//!   [`Linedef`][hexen::Linedef]); extends `THINGS`/`LINEDEFS` only.
//! - [`common`] — records whose byte layout is identical across formats
//!   ([`Vertex`], [`Sidedef`], [`Sector`], [`Seg`], [`Subsector`], [`Node`],
//!   [`Name8`]). These are re-exported at the `map` root, so `map::Vertex` and
//!   `map::common::Vertex` both resolve.
//!
//! The [`parse_records`] helper reads a raw lump byte slice into a `Vec` of the
//! appropriate fixed-size record type. All structs use little-endian byte order
//! as specified by the Doom WAD format.

use std::io::Cursor;

use binrw::{BinRead, BinReaderExt};
use thiserror::Error;

pub mod assemble;
pub mod common;
pub mod doom;
pub mod doom64;
pub mod graph;
pub mod group;
pub mod hexen;
pub mod udmf;

pub use assemble::MapAssembleError;
pub use common::{BlockmapLump, Name8, Node, RejectLump, Sector, Seg, Sidedef, Subsector, Vertex};
#[cfg(feature = "write")]
pub use doom::{DoomMapLumps, DoomWriteError, DoomWriteWarning, add_doom_map, write_doom_map};
pub use doom64::{
    Doom64Map, Doom64ReadError, Doom64Warning, is_doom64_map_lump, is_doom64_map_name,
    read_doom64_map,
};
pub use graph::{
    LightIdx, LinedefIdx, Map, MapBlockmap, MapFormat, MapLight, MapLinedef, MapNode, MapReject,
    MapSector, MapSeg, MapSidedef, MapSubsector, MapThing, MapVertex, MapWarning, NodeChild,
    NodeIdx, SectorIdx, SegIdx, SidedefIdx, Special, SubsectorIdx, TextureRef, VertexIdx,
};
pub use group::{MapGroup, detect_map_format};
pub use udmf::{UdmfParseError, parse_udmf};
#[cfg(feature = "write")]
pub use udmf::{UdmfWriteError, UdmfWriteWarning, add_udmf_map, write_udmf};

/// Errors returned when decoding typed map records from a lump byte slice.
#[derive(Debug, Error)]
pub enum MapParseError {
    /// The lump byte slice length is not an exact multiple of the on-disk
    /// record size consumed by `BinRead`.
    ///
    /// This indicates a corrupt or truncated lump.  `offset` is the byte
    /// position where the trailing partial record begins — i.e. the first byte
    /// that does not belong to a complete record.
    #[error("record stream ended mid-record at byte offset {offset}")]
    TrailingBytes {
        /// The byte offset of the start of the trailing partial record
        /// (i.e. the first byte that does not belong to a complete record).
        offset: u64,
    },
    /// `binrw` failed to decode a record from the byte stream.
    ///
    /// This typically indicates that the lump data is corrupted or does not
    /// actually contain the expected record type.
    #[error("failed to parse map records: {0}")]
    Binrw(#[from] binrw::Error),
}

/// Parses a raw lump byte slice into a `Vec` of the requested map record type.
///
/// This is the primary entry point for decoding map lumps.  Pass the raw bytes
/// from a lump (obtained via [`Wad::lump_bytes`][crate::Wad::lump_bytes] or
/// [`Wad::lump_data`][crate::Wad::lump_data]) and specify the target type:
///
/// ```rust,no_run
/// use crustywad::map::doom::Thing;
/// use crustywad::map::parse_records;
///
/// # let raw_lump_bytes: &[u8] = &[];
/// let things: Vec<Thing> = parse_records(raw_lump_bytes)?;
/// # Ok::<(), crustywad::map::MapParseError>(())
/// ```
///
/// The function reads the entire slice as a sequence of fixed-size little-endian
/// records.  All record types in this module implement [`BinRead`] with the
/// correct on-disk field layout.
///
/// # Errors
///
/// - [`MapParseError::TrailingBytes`] — the slice length is not an exact
///   multiple of the on-disk record size (measured by how many bytes `BinRead`
///   actually consumes for the first record).  The lump is likely truncated or
///   contains the wrong record type.
/// - [`MapParseError::Binrw`] — `binrw` failed to decode a record.  This
///   usually means the bytes are corrupt.
pub fn parse_records<T>(bytes: &[u8]) -> Result<Vec<T>, MapParseError>
where
    T: for<'a> BinRead<Args<'a> = ()>,
{
    if bytes.is_empty() {
        return Ok(Vec::new());
    }

    let mut cursor = Cursor::new(bytes);
    let bytes_len = bytes.len() as u64;

    // Parse the first record to learn the actual on-disk size that BinRead
    // consumes. This avoids relying on size_of::<T>(), which reflects the
    // in-memory layout (including any alignment padding) and may not match
    // the number of bytes binrw reads per record.
    //
    // An EOF on the very first record means the buffer is shorter than one
    // record — map that to TrailingBytes rather than Binrw so the error is
    // consistent with the truncation case (not a corruption case). binrw
    // wraps IO errors in Error::Backtrace, so use is_eof() instead of
    // matching Error::Io directly.
    let first: T = cursor.read_le().map_err(|e| {
        if e.is_eof() {
            return MapParseError::TrailingBytes { offset: 0 };
        }
        MapParseError::Binrw(e)
    })?;
    // Safe: cursor is backed by `bytes: &[u8]`, so position() ≤ bytes.len() ≤ usize::MAX.
    #[allow(clippy::cast_possible_truncation)]
    let record_size = cursor.position() as usize;

    if record_size == 0 {
        // BinRead consumed zero bytes — T has no on-disk representation.
        // Any non-empty input is unresolvable trailing data.
        return Err(MapParseError::TrailingBytes { offset: 0 });
    }

    if bytes.len() % record_size != 0 {
        return Err(MapParseError::TrailingBytes {
            offset: (bytes.len() / record_size * record_size) as u64,
        });
    }

    let mut records = Vec::with_capacity(bytes.len() / record_size);
    records.push(first);
    while cursor.position() < bytes_len {
        records.push(cursor.read_le()?);
    }

    Ok(records)
}
