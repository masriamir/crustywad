//! Doom 64-format map record types.
//!
//! Doom 64 stores each `MAPxx` as a **nested WAD**: the map lump's bytes are a
//! complete WAD whose sub-lumps hold the records. The records diverge from the
//! classic Doom binary layout in width and field content:
//!
//! - `VERTEXES` are 16.16 fixed-point `i32` pairs (not `i16`).
//! - `THINGS` gain a spawn height (`z`) and a thing ID (`id`).
//! - `LINEDEFS` widen `flags` to `u32`.
//! - `SIDEDEFS` reference textures by `u16` index, not 8-byte name.
//! - `SECTORS` reference flats by `u16` index and carry five colored-lighting
//!   IDs and a new `LIGHTS` palette lump.
//!
//! The BSP lumps `SEGS`, `SSECTORS`, and `NODES` are byte-identical to classic
//! Doom, so they are decoded with [`super::common::Seg`],
//! [`super::common::Subsector`], and [`super::common::Node`] rather than
//! redefined here.
//!
//! This module reads records **raw** (un-normalized): the texture/flat/color
//! `u16` indices are preserved as-is; resolving them needs a texture/graphics
//! layer that does not exist yet. Use [`read_doom64_map`] to read a whole
//! `MAPxx` lump's nested WAD into a [`Doom64Map`].

use binrw::BinRead;

use crate::map::common::{Node, Seg, Subsector};
use crate::map::{MapParseError, parse_records};
use crate::{ParseError, ParseOptions, Strictness, Wad};

/// Returns `true` if `bytes` look like a Doom 64 map lump: a lump whose content
/// is itself a WAD (leading `IWAD` or `PWAD` magic).
///
/// This is the structural detection signal for Doom 64 maps (ADR-0018): a
/// Doom 64 `MAPxx` lump's bytes begin with WAD magic, whereas a classic
/// Doom/Hexen map marker is a conventionally empty lump carrying no such magic.
/// The check is cheap and does not parse the nested directory.
#[must_use]
pub fn is_doom64_map_lump(bytes: &[u8]) -> bool {
    bytes.len() >= 12 && (bytes[0..4] == *b"IWAD" || bytes[0..4] == *b"PWAD")
}

/// A single `VERTEXES` record (8 bytes): a map vertex in **16.16 fixed-point**.
///
/// Unlike classic Doom's `i16` integer coordinates, Doom 64 stores each axis as
/// a 32-bit fixed-point value with 16 fractional bits: the map-unit value is
/// `raw as f64 / 65536.0` (e.g. raw `20971520` = `320.0`, raw `-65536` = `-1.0`).
/// The raw `i32`s are preserved here without conversion.
#[derive(Debug, Clone, PartialEq, Eq, BinRead)]
#[br(little)]
pub struct Vertex {
    /// X coordinate, 16.16 fixed-point (positive = East). Divide by `65536.0`
    /// for map units.
    pub x: i32,
    /// Y coordinate, 16.16 fixed-point (positive = North). Divide by `65536.0`
    /// for map units.
    pub y: i32,
}

/// A single `THINGS` record (14 bytes).
///
/// Extends the classic Doom thing with a spawn height (`z`) and a thing ID
/// (`id`, the tag referenced by specials/scripts). All fields are signed
/// 16-bit as stored on disk.
#[derive(Debug, Clone, PartialEq, Eq, BinRead)]
#[br(little)]
pub struct Thing {
    /// X coordinate in map units (positive = East).
    pub x: i16,
    /// Y coordinate in map units (positive = North).
    pub y: i16,
    /// Spawn height above the sector floor, in map units.
    pub z: i16,
    /// Facing angle in degrees (`0` = East, increasing counter-clockwise).
    pub angle: i16,
    /// Numeric thing-type identifier (doomednum) selecting the actor class.
    pub type_id: i16,
    /// Bitfield of spawn/behavior flags; stored opaquely (bits not interpreted).
    pub flags: i16,
    /// Thing ID (tag/tid) referenced by specials and scripts; `0` means none.
    pub id: i16,
}

/// A single `LINEDEFS` record (16 bytes).
///
/// Diverges from classic Doom by widening `flags` to `u32`. `sideback ==
/// 0xffff` marks a one-sided linedef (same sentinel meaning as Doom's
/// `left_sidedef`).
#[derive(Debug, Clone, PartialEq, Eq, BinRead)]
#[br(little)]
pub struct Linedef {
    /// Index into `VERTEXES` for the start vertex.
    pub v1: u16,
    /// Index into `VERTEXES` for the end vertex.
    pub v2: u16,
    /// Bitfield of linedef behavior flags (widened to 32 bits in Doom 64).
    /// Stored opaquely; individual bits are not interpreted here.
    pub flags: u32,
    /// Special action type triggered when the linedef is activated; `0` = none.
    pub special: u16,
    /// Sector tag identifying which sectors the special affects.
    pub tag: u16,
    /// Index into `SIDEDEFS` for the right (front) sidedef.
    pub sidefront: u16,
    /// Index into `SIDEDEFS` for the left (back) sidedef, or `0xffff` when the
    /// linedef is one-sided.
    pub sideback: u16,
}

/// A single `SIDEDEFS` record (12 bytes).
///
/// Doom 64 references wall textures by **`u16` index** into the engine's
/// texture table rather than by 8-byte name, so the record is 12 bytes instead
/// of classic Doom's 30.
#[derive(Debug, Clone, PartialEq, Eq, BinRead)]
#[br(little)]
pub struct Sidedef {
    /// Horizontal texture offset in map units.
    pub x_offset: i16,
    /// Vertical texture offset in map units.
    pub y_offset: i16,
    /// Upper-texture index (into the texture table). Preserved raw.
    pub upper: u16,
    /// Lower-texture index. Preserved raw.
    pub lower: u16,
    /// Middle-texture index. Preserved raw.
    pub middle: u16,
    /// Index into `SECTORS` for the sector this sidedef faces.
    pub sector: u16,
}

/// A single `SECTORS` record (24 bytes).
///
/// Doom 64 references floor/ceiling flats by **`u16` index** and adds five
/// colored-lighting IDs (`colors`) that index the map's `LIGHTS` palette.
#[derive(Debug, Clone, PartialEq, Eq, BinRead)]
#[br(little)]
pub struct Sector {
    /// Floor height in map units.
    pub floor_height: i16,
    /// Ceiling height in map units.
    pub ceiling_height: i16,
    /// Floor-flat index (into the texture/flat table). Preserved raw.
    pub floor_tex: u16,
    /// Ceiling-flat index. Preserved raw.
    pub ceiling_tex: u16,
    /// Five colored-lighting IDs indexing this map's `LIGHTS` palette
    /// (Doom 64 colored lighting). Preserved raw; semantics per Doom64 EX.
    pub colors: [u16; 5],
    /// Sector special type; `0` = none.
    pub special: u16,
    /// Sector tag matched by linedef specials.
    pub tag: u16,
    /// Bitfield of sector flags; stored opaquely.
    pub flags: u16,
}

/// A single `LIGHTS` record (6 bytes): one colored-lighting palette entry.
///
/// The `LIGHTS` lump is the color palette that [`Sector::colors`] indexes.
/// Measured from a retail `DOOM64.WAD`: `r`/`g`/`b` are the color channels,
/// `tag` is a small identifier (observed values `0`–`2`), and `unknown` is a
/// 16-bit field whose high byte is always zero in retail data. The exact
/// meaning of `tag`/`unknown` is not yet confirmed (per Doom64 EX); the raw
/// bytes are preserved.
#[derive(Debug, Clone, PartialEq, Eq, BinRead)]
#[br(little)]
pub struct Light {
    /// Red channel (`0`–`255`).
    pub r: u8,
    /// Green channel (`0`–`255`).
    pub g: u8,
    /// Blue channel (`0`–`255`).
    pub b: u8,
    /// Small identifier (observed `0`–`2`); tentative semantics, preserved raw.
    pub tag: u8,
    /// 16-bit trailing field (high byte always `0` in retail data); tentative
    /// semantics, preserved raw.
    pub unknown: u16,
}

/// All records read from one Doom 64 `MAPxx` nested WAD, raw and un-normalized.
///
/// Produced by [`read_doom64_map`]. Record vectors hold the decoded fixed-size
/// records; `reject`/`blockmap`/`leafs`/`macros` are kept as raw bytes
/// (recognition-only this pass). Non-fatal issues collected in lenient mode are
/// available via [`Doom64Map::warnings`].
#[derive(Debug, Clone)]
pub struct Doom64Map {
    /// Decoded `THINGS` records.
    pub things: Vec<Thing>,
    /// Decoded `LINEDEFS` records.
    pub linedefs: Vec<Linedef>,
    /// Decoded `SIDEDEFS` records.
    pub sidedefs: Vec<Sidedef>,
    /// Decoded `VERTEXES` records (16.16 fixed-point).
    pub vertexes: Vec<Vertex>,
    /// Decoded `SECTORS` records.
    pub sectors: Vec<Sector>,
    /// Decoded `LIGHTS` records (colored-lighting palette).
    pub lights: Vec<Light>,
    /// Decoded `SEGS` records (classic Doom layout, [`common::Seg`](crate::map::common::Seg)).
    pub segs: Vec<Seg>,
    /// Decoded `SSECTORS` records ([`common::Subsector`](crate::map::common::Subsector)).
    pub subsectors: Vec<Subsector>,
    /// Decoded `NODES` records ([`common::Node`](crate::map::common::Node)).
    pub nodes: Vec<Node>,
    /// Raw `REJECT` bytes (undecoded); empty if the lump is absent.
    pub reject: Vec<u8>,
    /// Raw `BLOCKMAP` bytes (undecoded); empty if the lump is absent.
    pub blockmap: Vec<u8>,
    /// Raw `LEAFS` bytes (render leaves, undecoded); empty if absent.
    pub leafs: Vec<u8>,
    /// Raw `MACROS` bytes (compiled scripts, undecoded); empty if absent.
    pub macros: Vec<u8>,
    pub(crate) warnings: Vec<Doom64Warning>,
}

impl Doom64Map {
    /// Returns the non-fatal warnings collected during a lenient-mode read.
    ///
    /// Always empty after a successful strict-mode read. These are only Doom 64
    /// map-specific, record-level warnings (missing record lumps / trailing
    /// bytes); the nested WAD container is parsed strictly and produces none.
    #[must_use]
    pub fn warnings(&self) -> &[Doom64Warning] {
        &self.warnings
    }
}

/// A non-fatal issue recovered while reading a Doom 64 map in lenient mode.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Doom64Warning {
    /// An expected record sub-lump was absent; its vector was left empty.
    #[error("Doom 64 map is missing the {name} lump; treated as empty")]
    MissingLump {
        /// The absent sub-lump's name.
        name: &'static str,
    },
    /// A record sub-lump's length was not a whole multiple of the record size;
    /// the whole records were kept and the trailing partial record dropped.
    #[error("{lump} lump has trailing bytes at offset {offset}; kept whole records only")]
    TrailingBytes {
        /// The sub-lump's name.
        lump: &'static str,
        /// Byte offset where the trailing partial record begins.
        offset: u64,
    },
}

/// An error that prevents reading a Doom 64 map.
#[derive(Debug, thiserror::Error)]
pub enum Doom64ReadError {
    /// The bytes are not a Doom 64 map lump: they lack the leading `IWAD`/`PWAD`
    /// nested-WAD magic that structurally identifies a Doom 64 map.
    ///
    /// Returned in **both** strictness modes before any parsing, so non-Doom 64
    /// data (e.g. a classic flat map marker or arbitrary bytes) can never be
    /// silently misread as an empty Doom 64 map. See [`is_doom64_map_lump`].
    #[error("bytes are not a Doom 64 map lump (missing nested IWAD/PWAD magic)")]
    NotADoom64Map,
    /// The map lump's bytes carried valid magic but did not parse as a nested
    /// WAD (e.g. a truncated or out-of-bounds directory).
    #[error("Doom 64 map lump is not a valid nested WAD: {0}")]
    NestedWad(#[from] ParseError),
    /// A record sub-lump failed to decode (strict mode, or a corrupt record in
    /// either mode).
    #[error("failed to decode {lump} records: {source}")]
    Records {
        /// The sub-lump's name.
        lump: &'static str,
        /// The underlying record-parse error.
        #[source]
        source: MapParseError,
    },
    /// An expected record sub-lump was absent (strict mode only).
    #[error("Doom 64 map is missing the required {name} lump")]
    MissingLump {
        /// The absent sub-lump's name.
        name: &'static str,
    },
}

/// Reads a Doom 64 `MAPxx` lump's bytes — themselves a nested WAD — into typed
/// records.
///
/// The `bytes` must be a Doom 64 map lump — leading `IWAD`/`PWAD` magic is
/// required (see [`is_doom64_map_lump`]) and enforced in **both** strictness
/// modes, so non-Doom 64 data cannot be silently misread as an empty map.
///
/// The nested-WAD **container** is always parsed strictly, regardless of the
/// caller's mode: a structurally corrupt container (e.g. a truncated or
/// out-of-bounds directory) errors in both modes rather than being recovered
/// into an empty map. The caller's [`Strictness`] governs only the per-record
/// recovery below. Each record sub-lump is looked up by name and decoded with
/// [`parse_records`]; BSP lumps reuse the classic [`common`](crate::map::common)
/// records, and `REJECT`/`BLOCKMAP`/`LEAFS`/`MACROS` are kept as raw bytes.
///
/// In [`Strictness::Lenient`] mode, a missing expected sub-lump yields an empty
/// vector plus a [`Doom64Warning::MissingLump`], and a sub-lump whose size is
/// not a whole multiple of the record size keeps the whole records and warns
/// with [`Doom64Warning::TrailingBytes`]. In [`Strictness::Strict`] mode either
/// condition is an error. The returned [`Doom64Map::warnings`] therefore hold
/// only these record-level warnings (the container is parsed strictly and
/// produces none).
///
/// # Errors
///
/// - [`Doom64ReadError::NotADoom64Map`] — `bytes` lack the `IWAD`/`PWAD` magic
///   (both modes).
/// - [`Doom64ReadError::NestedWad`] — `bytes` have valid magic but the nested
///   container does not parse as a WAD (both modes).
/// - [`Doom64ReadError::MissingLump`] — an expected sub-lump is absent (strict).
/// - [`Doom64ReadError::Records`] — a record sub-lump failed to decode (strict,
///   or a corrupt record mid-stream in either mode).
pub fn read_doom64_map(bytes: &[u8], options: &ParseOptions) -> Result<Doom64Map, Doom64ReadError> {
    if !is_doom64_map_lump(bytes) {
        return Err(Doom64ReadError::NotADoom64Map);
    }
    // Parse the nested-WAD container strictly regardless of the caller's mode: a
    // corrupt container is a structural failure, not a recoverable record-level
    // issue, so it errors in both modes. The caller's strictness governs only the
    // per-sub-lump record decoding below. (Limits are carried through but are
    // ignored by binary WAD parsing.)
    let container_options = ParseOptions {
        strictness: Strictness::Strict,
        limits: options.limits,
    };
    let nested = Wad::from_bytes_with_options(bytes.to_vec(), container_options)?;
    let strictness = options.strictness;
    let mut warnings = Vec::new();

    let things = decode_lump::<Thing>(&nested, "THINGS", strictness, &mut warnings)?;
    let linedefs = decode_lump::<Linedef>(&nested, "LINEDEFS", strictness, &mut warnings)?;
    let sidedefs = decode_lump::<Sidedef>(&nested, "SIDEDEFS", strictness, &mut warnings)?;
    let vertexes = decode_lump::<Vertex>(&nested, "VERTEXES", strictness, &mut warnings)?;
    let segs = decode_lump::<Seg>(&nested, "SEGS", strictness, &mut warnings)?;
    let subsectors = decode_lump::<Subsector>(&nested, "SSECTORS", strictness, &mut warnings)?;
    let nodes = decode_lump::<Node>(&nested, "NODES", strictness, &mut warnings)?;
    let sectors = decode_lump::<Sector>(&nested, "SECTORS", strictness, &mut warnings)?;
    let lights = decode_lump::<Light>(&nested, "LIGHTS", strictness, &mut warnings)?;

    let raw = |name: &str| -> Vec<u8> {
        nested
            .lump_by_name(name)
            .map(|lump| nested.lump_data(lump).to_vec())
            .unwrap_or_default()
    };

    Ok(Doom64Map {
        things,
        linedefs,
        sidedefs,
        vertexes,
        sectors,
        lights,
        segs,
        subsectors,
        nodes,
        reject: raw("REJECT"),
        blockmap: raw("BLOCKMAP"),
        leafs: raw("LEAFS"),
        macros: raw("MACROS"),
        warnings,
    })
}

/// Decodes one expected record sub-lump by name, honoring strictness.
///
/// Missing lump: strict → `MissingLump` error; lenient → empty vec + warning.
/// Trailing bytes: strict → `Records` error; lenient → keep the whole records
/// (re-parse the clean prefix) + warning. A corrupt (`Binrw`) record is an
/// error in both modes.
fn decode_lump<T>(
    nested: &Wad,
    name: &'static str,
    strictness: Strictness,
    warnings: &mut Vec<Doom64Warning>,
) -> Result<Vec<T>, Doom64ReadError>
where
    T: for<'a> BinRead<Args<'a> = ()>,
{
    let Some(lump) = nested.lump_by_name(name) else {
        return match strictness {
            Strictness::Strict => Err(Doom64ReadError::MissingLump { name }),
            Strictness::Lenient => {
                warnings.push(Doom64Warning::MissingLump { name });
                Ok(Vec::new())
            }
        };
    };
    let data = nested.lump_data(lump);
    match parse_records::<T>(data) {
        Ok(records) => Ok(records),
        Err(MapParseError::TrailingBytes { offset }) => match strictness {
            Strictness::Strict => Err(Doom64ReadError::Records {
                lump: name,
                source: MapParseError::TrailingBytes { offset },
            }),
            Strictness::Lenient => {
                warnings.push(Doom64Warning::TrailingBytes { lump: name, offset });
                // `offset` is a whole-record boundary within `data`; re-parse the
                // clean prefix (cannot itself have trailing bytes). Clamp
                // defensively so a pathological `offset` can never panic.
                let end = usize::try_from(offset)
                    .unwrap_or(data.len())
                    .min(data.len());
                parse_records::<T>(&data[..end])
                    .map_err(|source| Doom64ReadError::Records { lump: name, source })
            }
        },
        Err(source @ MapParseError::Binrw(_)) => {
            Err(Doom64ReadError::Records { lump: name, source })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A record type that fails to decode on *content* (a magic mismatch), so
    /// `parse_records` returns [`MapParseError::Binrw`] rather than
    /// [`MapParseError::TrailingBytes`]. The real Doom 64 record types are plain
    /// fixed-size integers and only ever fail on EOF (which becomes
    /// `TrailingBytes`), so this synthetic type is the only way to exercise
    /// `decode_lump`'s `Binrw` error arm.
    #[derive(BinRead, Debug)]
    #[br(little, magic = b"OK")]
    struct MagicRecord {
        #[allow(dead_code)]
        value: u8,
    }

    /// Builds a minimal well-formed single-lump `PWAD` in memory.
    fn wad_with_lump(name: &str, data: &[u8]) -> Wad {
        let directory_offset = 12 + data.len();
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"PWAD");
        bytes.extend_from_slice(&1_i32.to_le_bytes());
        bytes.extend_from_slice(&i32::try_from(directory_offset).unwrap().to_le_bytes());
        bytes.extend_from_slice(data);
        bytes.extend_from_slice(&12_i32.to_le_bytes());
        bytes.extend_from_slice(&i32::try_from(data.len()).unwrap().to_le_bytes());
        let mut encoded = [0_u8; 8];
        for (slot, byte) in name.as_bytes().iter().take(8).enumerate() {
            encoded[slot] = *byte;
        }
        bytes.extend_from_slice(&encoded);
        Wad::from_bytes(bytes).expect("hand-built WAD should parse")
    }

    #[test]
    fn decode_lump_maps_binrw_error_to_records() {
        // 3 bytes that are not the `OK` magic: parse_records fails on the first
        // record with a non-EOF (content) error -> MapParseError::Binrw, which
        // decode_lump must map to Doom64ReadError::Records in either mode.
        let wad = wad_with_lump("THINGS", b"NO!");
        for strictness in [Strictness::Strict, Strictness::Lenient] {
            let mut warnings = Vec::new();
            let result = decode_lump::<MagicRecord>(&wad, "THINGS", strictness, &mut warnings);
            assert!(
                matches!(
                    result,
                    Err(Doom64ReadError::Records {
                        lump: "THINGS",
                        source: MapParseError::Binrw(_),
                    })
                ),
                "expected Records/Binrw error, got {result:?}"
            );
            assert!(warnings.is_empty());
        }
    }
}
