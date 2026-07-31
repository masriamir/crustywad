//! Assembling normalized [`Map`]s from a WAD's flat records (ADR-0015 §3–5).

use crate::map::deepbsp::{decode_deepbsp, is_deepbsp};
use crate::map::doom64::Doom64TextureNames;
#[cfg(feature = "extended-nodes-zlib")]
use crate::map::extended::decode_compressed_extended_nodes;
use crate::map::extended::{NodeCompression, classify_signature, decode_extended_nodes};
use crate::map::graph::{
    LightIdx, LinedefIdx, Map, MapBlockmap, MapFormat, MapLeaf, MapLight, MapLinedef, MapMacro,
    MapMacroAction, MapNode, MapReject, MapSector, MapSeg, MapSidedef, MapSubsector, MapThing,
    MapVertex, MapWarning, NodeChild, NodeIdx, SectorIdx, SegIdx, SidedefIdx, Special,
    SubsectorIdx, TextureRef, VertexIdx,
};
use crate::map::{MapGroup, MapParseError, common, doom, doom64, hexen, parse_records};
use crate::{ParseOptions, Strictness, Wad};

/// Fatal errors from [`Map::assemble_with_options`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum MapAssembleError {
    /// A required lump (e.g. `VERTEXES`, `SECTORS`) was not present in the
    /// map's data lumps.
    #[error("map group is missing required lump {lump}")]
    MissingLump {
        /// The name of the missing required lump.
        lump: &'static str,
    },
    /// A required lump's bytes failed to decode into fixed-size records.
    #[error("failed to decode {lump} records: {source}")]
    Records {
        /// The name of the lump whose records failed to decode.
        lump: &'static str,
        /// The underlying decode error.
        #[source]
        source: MapParseError,
    },
    /// A cross-reference (e.g. a linedef's vertex index) pointed outside the
    /// bounds of the referenced arena, and strict mode rejected it.
    #[error("{referent} index {index} referenced from {from} is out of range ({count} available)")]
    DanglingReference {
        /// The name of the arena the out-of-range index referred to (e.g. `"vertex"`).
        referent: &'static str,
        /// The out-of-range index value that was encountered (signed, since UDMF
        /// indices may be negative).
        index: i32,
        /// The name of the element kind the dangling reference was found on (e.g. `"linedef"`).
        from: &'static str,
        /// The number of elements actually available in the referenced arena.
        count: usize,
    },
    /// A Doom 64 map's nested-WAD lump (its `MAPxx` marker, ADR-0021 §1)
    /// failed to read as a [`Doom64Map`](crate::map::doom64::Doom64Map) —
    /// either the nested container itself was structurally invalid, or a
    /// required sub-lump was missing/undecodable in strict mode
    /// (ADR-0021 §2).
    #[error("failed to read Doom 64 map: {source}")]
    Doom64 {
        /// The underlying nested-WAD read error.
        #[source]
        source: crate::map::doom64::Doom64ReadError,
    },
    /// The `TEXTMAP` text failed to decode or parse as UDMF.
    #[error("failed to parse UDMF text map: {source}")]
    Udmf {
        /// The underlying UDMF parse error.
        #[source]
        source: crate::map::udmf::UdmfParseError,
    },
    /// A UDMF map (`TEXTMAP` present) had no `ENDMAP` terminator lump (strict mode).
    #[error("UDMF map '{name}' has no ENDMAP terminator lump")]
    UnterminatedUdmf {
        /// The map's marker name.
        name: String,
    },
    /// A field value was outside its target field's representable range (strict mode).
    #[error("{field} value {value} on {from} is out of range")]
    FieldOutOfRange {
        /// The UDMF field name.
        field: &'static str,
        /// The element kind.
        from: &'static str,
        /// The offending value.
        value: i32,
    },
    /// A `NODES`/`SSECTORS` lump (or the UDMF `ZNODES` lump) carried an
    /// extended node-encoding signature this build cannot decode (strict mode).
    /// The uncompressed `X*` family always decodes into the BSP arenas (#326);
    /// the compressed `Z*` twins decode only with the `extended-nodes-zlib`
    /// feature (#327) and otherwise gate here; any other 4-byte signature is
    /// unsupported. `DeePBSP`'s `xNd4` is decoded on the binary path (its 8-byte
    /// signature is detected ahead of this 4-byte gate, #328), but on the UDMF
    /// `ZNODES` path — where `DeePBSP` never legitimately appears — an `xNd4`
    /// tag is an unrecognized signature and gates here. The classic record
    /// decoder must never misread a gated stream as garbage classic records.
    #[error(
        "{lump} uses an unsupported extended node encoding {}",
        String::from_utf8_lossy(signature).escape_default()
    )]
    UnsupportedNodeEncoding {
        /// The name of the lump carrying the extended encoding (`"NODES"`,
        /// `"SSECTORS"`, or the UDMF `"ZNODES"`).
        lump: &'static str,
        /// The 4-byte signature found at the head of the lump (e.g. `*b"XNOD"`).
        signature: [u8; 4],
    },
    /// An uncompressed `ZDoom` extended-node stream (`XNOD`/`XGLN`/`XGL2`/`XGL3`)
    /// was structurally malformed — a framing defect, as distinct from an
    /// out-of-range cross-reference, which takes [`Self::DanglingReference`]
    /// (strict mode; lenient degrades the BSP to empty arenas and warns).
    /// See [`ExtendedNodeError`](crate::map::ExtendedNodeError) (ADR-0025).
    #[error("malformed {dialect} extended node stream: {reason}")]
    ExtendedNode {
        /// The dialect tag naming the stream (`"XNOD"`, `"XGLN"`, `"XGL2"`, or `"XGL3"`).
        dialect: &'static str,
        /// The specific structural fault.
        #[source]
        reason: crate::map::ExtendedNodeError,
    },
    /// The `REJECT` lump was smaller than the table its map's sector count
    /// requires (strict mode; lenient reads missing bits as "not rejected").
    #[error("REJECT lump is {actual} bytes; {expected} bytes required for {sectors} sectors")]
    UndersizedReject {
        /// The lump's actual byte length.
        actual: usize,
        /// The required table size, `(sectors² + 7) / 8` bytes.
        expected: usize,
        /// The owning map's sector count.
        sectors: usize,
    },
    /// The `BLOCKMAP` lump was structurally unusable — shorter than its
    /// 4-word header, non-positive dimensions, or an offset table extending
    /// past the lump (strict mode; lenient discards the blockmap and warns).
    #[error("BLOCKMAP lump is malformed: {detail}")]
    MalformedBlockmap {
        /// What made the lump unusable.
        detail: &'static str,
    },
    /// A `BLOCKMAP` block's offset pointed outside the lump (strict mode;
    /// lenient empties that block's list and warns).
    #[error("BLOCKMAP block {block} offset {offset} is outside the lump ({words} words)")]
    BlockmapBlockOffset {
        /// The 0-based block (offset-table) index.
        block: usize,
        /// The out-of-range word offset.
        offset: usize,
        /// The lump's total word count.
        words: usize,
    },
    /// A `BLOCKMAP` block's linedef list ran past the end of the lump
    /// without its `0xFFFF` terminator (strict mode; lenient truncates the
    /// list at the lump end and warns).
    #[error("BLOCKMAP block {block} linedef list is unterminated")]
    UnterminatedBlockmapList {
        /// The 0-based block index.
        block: usize,
    },
    /// The `LEAFS` lump was structurally unusable — a record's entries ran
    /// past the lump end, or trailing bytes did not form a whole record
    /// (strict mode; lenient discards all leaves and warns).
    #[error("LEAFS lump is malformed: {detail}")]
    MalformedLeafs {
        /// What made the lump unusable.
        detail: &'static str,
    },
    /// The `LEAFS` lump's record count did not match the subsector count,
    /// which the engine treats as fatal (Doom64 EX `P_LoadLeafs`); strict
    /// mode rejects, lenient discards all leaves and warns.
    #[error("LEAFS record count {leaves} does not match subsector count {subsectors}")]
    LeafCountMismatch {
        /// The number of leaf records the lump encodes.
        leaves: usize,
        /// The owning map's subsector count.
        subsectors: usize,
    },
    /// The `MACROS` lump was structurally unusable — a non-empty lump
    /// shorter than its 4-byte header, a negative count, a record running
    /// past the lump end, or trailing bytes (strict mode; lenient discards
    /// all macros and warns). The engine itself validates none of this
    /// (`P_LoadMacros` trusts the header and silently treats sub-8-byte
    /// lumps as empty under its own `TODO - fixme`); the stricter checks
    /// are this reader's deliberate divergence.
    #[error("MACROS lump is malformed: {detail}")]
    MalformedMacros {
        /// What made the lump unusable.
        detail: &'static str,
    },
    /// Scanning the outer WAD's sections (to build the Doom 64
    /// texture-name table) failed in strict mode. The scan classifies
    /// markers of every kind, so the underlying
    /// [`SectionError`](crate::sections::SectionError) may concern a
    /// non-texture section; it names the section kind itself.
    #[error("scanning sections for Doom 64 texture resolution failed: {source}")]
    TextureSections {
        /// The underlying section-scan error.
        #[source]
        source: crate::sections::SectionError,
    },
    /// A Doom 64 texture hash matched no texture-section lump (strict
    /// mode; lenient keeps the unresolved index and warns).
    #[error("texture name hash {hash:#06x} on {from} matches no texture-section lump")]
    UnresolvedTextureHash {
        /// The on-disk 16-bit name hash.
        hash: u16,
        /// The element kind carrying the reference (`"sidedef"` or `"sector"`).
        from: &'static str,
    },
    /// A `GL_*` node group used an unreadable/refused GL version (V1 or V4).
    #[error(
        "unsupported or refused GL node version (magic {})",
        String::from_utf8_lossy(magic).escape_default()
    )]
    UnsupportedGlNodeVersion {
        /// The 4-byte `GL_VERT`/`gNd?` magic that identified the refused version.
        magic: [u8; 4],
    },
}

/// Finds the bytes of the data lump named `lump` within `group`.
fn lump_bytes<'w>(wad: &'w Wad, group: &MapGroup, lump: &str) -> Option<&'w [u8]> {
    group
        .data_indices
        .iter()
        .copied()
        .find(|&i| wad.lumps().get(i).is_some_and(|l| l.name() == lump))
        .and_then(|i| wad.lump_bytes(i))
}

/// Decodes a required record lump, mapping absence/decoding failure to errors.
fn decode_required<T>(
    wad: &Wad,
    group: &MapGroup,
    lump: &'static str,
) -> Result<Vec<T>, MapAssembleError>
where
    T: for<'a> binrw::BinRead<Args<'a> = ()>,
{
    let bytes = lump_bytes(wad, group, lump).ok_or(MapAssembleError::MissingLump { lump })?;
    parse_records::<T>(bytes).map_err(|source| MapAssembleError::Records { lump, source })
}

/// Known extended/GL node-encoding signatures (ZDBSP family) that the classic
/// record decoder must never touch — reading them is #199's scope (ADR-0015
/// amendment; spec §"Extended-encoding gate").
const EXTENDED_NODE_SIGNATURES: [&[u8; 4]; 8] = [
    b"XNOD", b"ZNOD", b"XGLN", b"ZGLN", b"XGL2", b"XGL3", b"ZGL2", b"ZGL3",
];

/// Decodes an optional BSP lump: absent -> empty vec (ADR-0015 §5).
fn decode_optional<T>(
    wad: &Wad,
    group: &MapGroup,
    lump: &'static str,
) -> Result<Vec<T>, MapAssembleError>
where
    T: for<'a> binrw::BinRead<Args<'a> = ()>,
{
    match lump_bytes(wad, group, lump) {
        None => Ok(Vec::new()),
        Some(bytes) => {
            parse_records::<T>(bytes).map_err(|source| MapAssembleError::Records { lump, source })
        }
    }
}

/// Decodes a group's `REJECT`/`BLOCKMAP` lumps (either may be absent) once
/// the owning map's sector and linedef counts are known. Shared by all
/// three assembly paths so the strict/lenient policy cannot drift between
/// formats.
fn decode_reject_blockmap(
    reject_bytes: Option<&[u8]>,
    blockmap_bytes: Option<&[u8]>,
    sector_count: usize,
    linedef_count: usize,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<(Option<MapReject>, Option<MapBlockmap>), MapAssembleError> {
    let reject = match reject_bytes {
        None => None,
        Some(bytes) => MapReject::parse(bytes, sector_count, strictness, warnings)?,
    };
    let blockmap = match blockmap_bytes {
        None => None,
        Some(bytes) => MapBlockmap::parse(bytes, linedef_count, strictness, warnings)?,
    };
    Ok((reject, blockmap))
}

/// Decodes a Doom 64 `LEAFS` lump (Doom64 EX `P_LoadLeafs`, `p_setup.cc`):
/// per-subsector records of a `u16` leaf count followed by that many
/// (`u16` vertex, `i16` seg) entries, where a seg of `-1` means "no seg".
/// The record count must equal `subsector_count` — the engine fatal-errors
/// otherwise, and this reader mirrors that as an error (strict) or a
/// whole-arena degrade with one warning (lenient). Index validation uses
/// `>=`, deliberately tighter than the engine's off-by-one `>` checks.
///
/// Single forward pass; total entries are bounded by `bytes.len() / 4`, and
/// the ranges vec by `subsector_count` — once it is full, surplus records
/// are tallied without decoding their entries, mirroring the engine's
/// two-pass order (count first, then load) so a surplus lump reports the
/// count mismatch rather than a later per-entry defect (ADR-0016 §1).
fn normalize_leafs(
    bytes: &[u8],
    subsector_count: usize,
    vertex_count: usize,
    seg_count: usize,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<(Vec<MapLeaf>, Vec<std::ops::Range<usize>>), MapAssembleError> {
    /// The on-disk "no seg" sentinel (`-1` in `P_LoadLeafs`).
    const NO_SEG: u16 = 0xFFFF;

    let mut leafs = Vec::new();
    let mut ranges = Vec::with_capacity(subsector_count);
    let mut record_count = 0_usize;
    let mut offset = 0_usize;
    while offset < bytes.len() {
        if bytes.len() - offset < 2 {
            return leafs_malformed("trailing byte is not a whole record", strictness, warnings);
        }
        let count = usize::from(u16::from_le_bytes([bytes[offset], bytes[offset + 1]]));
        offset += 2;
        if (bytes.len() - offset) / 4 < count {
            return leafs_malformed(
                "a record's entries run past the lump end",
                strictness,
                warnings,
            );
        }
        record_count += 1;
        if ranges.len() == subsector_count {
            // More records than subsectors: the count mismatch below is now
            // inevitable, so skip entry decoding and keep only the record
            // tally. This keeps `ranges` genuinely bounded by the subsector
            // count under a surplus-record lump, and mirrors the engine's
            // two-pass order (`P_LoadLeafs` counts records before loading
            // any), so the mismatch is what gets reported.
            offset += count * 4;
            continue;
        }
        let start = leafs.len();
        for _ in 0..count {
            let vertex = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
            let seg = u16::from_le_bytes([bytes[offset + 2], bytes[offset + 3]]);
            offset += 4;
            if usize::from(vertex) >= vertex_count {
                return leafs_dangling("vertex", vertex, vertex_count, strictness, warnings);
            }
            let seg = if seg == NO_SEG {
                None
            } else if usize::from(seg) >= seg_count {
                return leafs_dangling("seg", seg, seg_count, strictness, warnings);
            } else {
                Some(SegIdx(usize::from(seg)))
            };
            leafs.push(MapLeaf {
                vertex: VertexIdx(usize::from(vertex)),
                seg,
            });
        }
        ranges.push(start..leafs.len());
    }
    if record_count != subsector_count {
        let (leaves, subsectors) = (record_count, subsector_count);
        return match strictness {
            Strictness::Strict => Err(MapAssembleError::LeafCountMismatch { leaves, subsectors }),
            Strictness::Lenient => {
                warnings.push(MapWarning::LeafCountMismatch { leaves, subsectors });
                Ok((Vec::new(), Vec::new()))
            }
        };
    }
    Ok((leafs, ranges))
}

/// The `MalformedLeafs` strict-error / lenient-degrade fork.
fn leafs_malformed(
    detail: &'static str,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<(Vec<MapLeaf>, Vec<std::ops::Range<usize>>), MapAssembleError> {
    match strictness {
        Strictness::Strict => Err(MapAssembleError::MalformedLeafs { detail }),
        Strictness::Lenient => {
            warnings.push(MapWarning::MalformedLeafs { detail });
            Ok((Vec::new(), Vec::new()))
        }
    }
}

/// The dangling-reference strict-error / lenient-degrade fork for leaves.
fn leafs_dangling(
    referent: &'static str,
    index: u16,
    count: usize,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<(Vec<MapLeaf>, Vec<std::ops::Range<usize>>), MapAssembleError> {
    match strictness {
        Strictness::Strict => Err(MapAssembleError::DanglingReference {
            referent,
            index: i32::from(index),
            from: "leaf",
            count,
        }),
        Strictness::Lenient => {
            warnings.push(MapWarning::LeafsDangling {
                referent,
                index,
                count,
            });
            Ok((Vec::new(), Vec::new()))
        }
    }
}

/// Decodes a Doom 64 `MACROS` lump (Doom64 EX `P_LoadMacros`,
/// `p_setup.cc`): an `i16 macrocount` + `i16 specialcount` header, then per
/// macro an `i16 count` followed by `count + 1` actions of (`i16 id`,
/// `i16 tag`, `i16 special`) — the engine reads one more action than the
/// count field states. Validation is purely structural (macros carry no
/// arena indices). The engine validates nothing here — its loader
/// allocates from the untrusted header, can read past the lump end, and
/// silently treats any sub-8-byte lump as empty (under a `TODO - fixme`)
/// — so the short-lump and exact-consumption failures are this reader's
/// deliberate divergences. A negative `macrocount` or per-macro `count` is
/// likewise rejected (strict) / degraded (lenient): the engine's own
/// `for (i = 0; i < macrocount; ...)` loops would silently no-op a negative
/// count rather than reject it, but this reader treats it as malformed
/// input instead of quietly reading zero macros. `specialcount` (header
/// bytes 2..4) has unestablished semantics and stays raw-layer-only.
///
/// Single forward pass; total actions bounded by `bytes.len() / 6` and
/// macros by `bytes.len() / 8` (the minimum per-macro record is a 2-byte
/// count plus at least one 6-byte action, since the engine always reads
/// `count + 1` actions), with no allocation from untrusted header counts
/// (ADR-0016 §1).
fn normalize_macros(
    bytes: &[u8],
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<Vec<MapMacro>, MapAssembleError> {
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    if bytes.len() < 4 {
        return macros_malformed("shorter than the 4-byte header", strictness, warnings);
    }
    let macrocount = i16::from_le_bytes([bytes[0], bytes[1]]);
    if macrocount < 0 {
        return macros_malformed("negative macro count", strictness, warnings);
    }
    let macrocount = usize::try_from(macrocount).expect("checked non-negative above");
    let mut macros = Vec::new();
    let mut offset = 4_usize;
    for _ in 0..macrocount {
        if bytes.len() - offset < 2 {
            return macros_malformed(
                "a macro record runs past the lump end",
                strictness,
                warnings,
            );
        }
        let count = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
        offset += 2;
        if count < 0 {
            return macros_malformed("negative action count", strictness, warnings);
        }
        // The engine reads count + 1 actions (`P_LoadMacros`).
        let action_count = usize::try_from(count).expect("checked non-negative above") + 1;
        if (bytes.len() - offset) / 6 < action_count {
            return macros_malformed(
                "a macro's actions run past the lump end",
                strictness,
                warnings,
            );
        }
        // Bounded capacity hint: action_count <= remaining / 6, just checked.
        let mut actions = Vec::with_capacity(action_count);
        for _ in 0..action_count {
            let id = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
            let tag = i16::from_le_bytes([bytes[offset + 2], bytes[offset + 3]]);
            let special = i16::from_le_bytes([bytes[offset + 4], bytes[offset + 5]]);
            offset += 6;
            actions.push(MapMacroAction { id, tag, special });
        }
        macros.push(MapMacro { actions });
    }
    if offset != bytes.len() {
        return macros_malformed(
            "trailing bytes after the last macro record",
            strictness,
            warnings,
        );
    }
    Ok(macros)
}

/// The `MalformedMacros` strict-error / lenient-degrade fork.
fn macros_malformed(
    detail: &'static str,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<Vec<MapMacro>, MapAssembleError> {
    match strictness {
        Strictness::Strict => Err(MapAssembleError::MalformedMacros { detail }),
        Strictness::Lenient => {
            warnings.push(MapWarning::MalformedMacros { detail });
            Ok(Vec::new())
        }
    }
}

impl MapReject {
    /// Parses a `REJECT` lump against its owning map's sector count.
    ///
    /// An empty lump means "not built" (our own writer emits zero-length
    /// `REJECT` by design, ADR-0019 §4) and yields `Ok(None)` with no
    /// warning in both modes. An oversized lump is accepted in both modes
    /// and its tail ignored, as vanilla does (`P_LoadReject` reads
    /// `minlength`). The stored table is `min(actual, expected)` bytes —
    /// bounded by the input (ADR-0016 §1).
    ///
    /// # Errors
    ///
    /// [`MapAssembleError::UndersizedReject`] in strict mode when the lump
    /// is shorter than `(sector_count² + 7) / 8` bytes; lenient mode records
    /// [`MapWarning::UndersizedReject`] instead and the missing bits read as
    /// "not rejected".
    pub fn parse(
        bytes: &[u8],
        sector_count: usize,
        strictness: Strictness,
        warnings: &mut Vec<MapWarning>,
    ) -> Result<Option<Self>, MapAssembleError> {
        if bytes.is_empty() {
            return Ok(None);
        }
        // Saturating: a pathological standalone-caller count still yields a
        // deterministic "undersized" comparison rather than an overflow.
        let expected = sector_count.saturating_mul(sector_count).div_ceil(8);
        if bytes.len() < expected {
            match strictness {
                Strictness::Strict => {
                    return Err(MapAssembleError::UndersizedReject {
                        actual: bytes.len(),
                        expected,
                        sectors: sector_count,
                    });
                }
                Strictness::Lenient => warnings.push(MapWarning::UndersizedReject {
                    actual: bytes.len(),
                    expected,
                    sectors: sector_count,
                }),
            }
        }
        let stored = &bytes[..bytes.len().min(expected)];
        Ok(Some(Self {
            sector_count,
            bits: stored.into(),
        }))
    }
}

impl MapBlockmap {
    /// Parses a `BLOCKMAP` lump against its owning map's linedef count.
    ///
    /// An empty lump means "not built" (ADR-0019 §4) and yields `Ok(None)`
    /// with no warning in both modes. A trailing odd byte is ignored, as
    /// vanilla's word-count division does (`P_LoadBlockMap`). List entries
    /// are read as **unsigned**, zero-extended, with only the `0xFFFF`
    /// terminator special-cased — the Boom fix (`PrBoom+` `P_LoadBlockMap`,
    /// killough 3/1/98: "treating all offsets except -1 as unsigned")
    /// that doubles the addressable linedefs over vanilla's accidental
    /// signed read. Offsets may alias or overlap freely — block lists are
    /// ranges into a single shared word arena, so parse work and memory
    /// stay `O(input)` (ADR-0016 §1).
    ///
    /// # Errors
    ///
    /// In strict mode: [`MapAssembleError::MalformedBlockmap`] for a
    /// structurally unusable lump,
    /// [`MapAssembleError::BlockmapBlockOffset`] for a block offset outside
    /// the lump, [`MapAssembleError::UnterminatedBlockmapList`] for a list
    /// with no terminator, and [`MapAssembleError::DanglingReference`] for
    /// a list entry past the linedef arena. Lenient mode recovers each with
    /// the corresponding [`MapWarning`] (discard / empty block / truncate /
    /// empty block, respectively).
    ///
    /// # Panics
    ///
    /// Does not panic. The internal `expect` calls on `usize::try_from` are
    /// preceded by an explicit `columns <= 0 || rows <= 0` check that returns
    /// (or, in lenient mode, discards the lump) before either conversion runs.
    #[allow(clippy::too_many_lines)]
    pub fn parse(
        bytes: &[u8],
        linedef_count: usize,
        strictness: Strictness,
        warnings: &mut Vec<MapWarning>,
    ) -> Result<Option<Self>, MapAssembleError> {
        /// The list terminator word (`-1` in `P_BlockLinesIterator`).
        const TERMINATOR: u16 = 0xFFFF;

        if bytes.is_empty() {
            return Ok(None);
        }
        let malformed = |detail: &'static str, warnings: &mut Vec<MapWarning>| match strictness {
            Strictness::Strict => Err(MapAssembleError::MalformedBlockmap { detail }),
            Strictness::Lenient => {
                warnings.push(MapWarning::MalformedBlockmap { detail });
                Ok(None)
            }
        };
        if bytes.len() < 8 {
            return malformed("shorter than the 4-word header", warnings);
        }
        let origin_x = f64::from(i16::from_le_bytes([bytes[0], bytes[1]]));
        let origin_y = f64::from(i16::from_le_bytes([bytes[2], bytes[3]]));
        let columns = i16::from_le_bytes([bytes[4], bytes[5]]);
        let rows = i16::from_le_bytes([bytes[6], bytes[7]]);
        if columns <= 0 || rows <= 0 {
            return malformed("non-positive grid dimensions", warnings);
        }
        let columns = usize::try_from(columns).expect("checked positive above");
        let rows = usize::try_from(rows).expect("checked positive above");
        // Fits usize: both factors are <= i16::MAX.
        let block_count = columns * rows;

        let words: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        if 4 + block_count > words.len() {
            return malformed("offset table extends past the lump", warnings);
        }

        // Single reverse pass — O(1) per-block validity AND diagnostic under
        // arbitrary offset aliasing: for each word position, the index of
        // the next terminator at or after it (usize::MAX = none), and the
        // index of the next out-of-range entry at or after it (usize::MAX =
        // none). This is what makes parse time O(input) regardless of how
        // many blocks alias the same offset.
        let mut next_term = vec![usize::MAX; words.len()];
        let mut next_invalid = vec![usize::MAX; words.len()];
        let mut last_term = usize::MAX;
        let mut last_invalid = usize::MAX;
        for i in (0..words.len()).rev() {
            let word = words[i];
            if word == TERMINATOR {
                last_term = i;
            } else if usize::from(word) >= linedef_count {
                last_invalid = i;
            }
            next_term[i] = last_term;
            next_invalid[i] = last_invalid;
        }

        let entries: Vec<LinedefIdx> = words.iter().map(|&w| LinedefIdx(usize::from(w))).collect();
        let mut blocks = Vec::with_capacity(block_count);
        for block in 0..block_count {
            let offset = usize::from(words[4 + block]);
            if offset >= words.len() {
                match strictness {
                    Strictness::Strict => {
                        return Err(MapAssembleError::BlockmapBlockOffset {
                            block,
                            offset,
                            words: words.len(),
                        });
                    }
                    Strictness::Lenient => {
                        warnings.push(MapWarning::BlockmapBlockOffset {
                            block,
                            offset,
                            words: words.len(),
                        });
                        blocks.push(0..0);
                        continue;
                    }
                }
            }
            // A literal leading 0 is the conventional delimiter (see
            // `MapBlockmap::block`); skipping it may land exactly on the
            // lump end, which the unterminated branch below then handles.
            let start = if words[offset] == 0 {
                offset + 1
            } else {
                offset
            };
            let end = if start < words.len() {
                next_term[start]
            } else {
                usize::MAX
            };
            let end = if end == usize::MAX {
                match strictness {
                    Strictness::Strict => {
                        return Err(MapAssembleError::UnterminatedBlockmapList { block });
                    }
                    Strictness::Lenient => {
                        warnings.push(MapWarning::UnterminatedBlockmapList { block });
                        words.len()
                    }
                }
            } else {
                end
            };
            let first_invalid = if start < words.len() {
                next_invalid[start]
            } else {
                usize::MAX
            };
            if first_invalid < end {
                let word = words[first_invalid];
                match strictness {
                    Strictness::Strict => {
                        return Err(MapAssembleError::DanglingReference {
                            referent: "linedef",
                            index: i32::from(word),
                            from: "blockmap block",
                            count: linedef_count,
                        });
                    }
                    Strictness::Lenient => {
                        warnings.push(MapWarning::BlockmapListDangling {
                            block,
                            index: word,
                            count: linedef_count,
                        });
                        blocks.push(0..0);
                        continue;
                    }
                }
            }
            blocks.push(start..end);
        }
        Ok(Some(Self {
            origin_x,
            origin_y,
            columns,
            rows,
            entries,
            blocks,
        }))
    }
}

/// Resolves a **required** reference. Empty target arena is always fatal.
///
/// `index` is `i32` so UDMF's signed, wider indices share this validator with the
/// binary formats (whose non-negative `u16` indices widen losslessly); a negative
/// index is treated as out of range, taking the same dangling-reference path. The
/// raw signed `index` is preserved in the diagnostic (error/warning).
///
/// `pub(crate)` so the `DeePBSP` v4 normalizer ([`crate::map::deepbsp`]) shares the
/// exact strict-error / lenient-clamp discipline used by the classic BSP path.
pub(crate) fn resolve_required(
    index: i32,
    count: usize,
    referent: &'static str,
    from: &'static str,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<usize, MapAssembleError> {
    if count == 0 {
        return Err(MapAssembleError::DanglingReference {
            referent,
            index,
            from,
            count: 0,
        });
    }
    // A negative index (or one past `count`) is out of range.
    if let Ok(idx) = usize::try_from(index)
        && idx < count
    {
        return Ok(idx);
    }
    match strictness {
        Strictness::Strict => Err(MapAssembleError::DanglingReference {
            referent,
            index,
            from,
            count,
        }),
        Strictness::Lenient => {
            warnings.push(MapWarning::DanglingReference {
                referent,
                index,
                from,
                count,
            });
            Ok(0) // clamp to a valid in-range fallback
        }
    }
}

/// Range-checks an **optional** sidedef reference with no binary sentinel.
///
/// Used by the UDMF normalizer, which supplies `sideback` already stripped of
/// its `-1` one-sided sentinel (the parser mapped `-1 -> None`), so a real index
/// — including `65535` — is validated normally rather than treated as the binary
/// `0xffff` "no back side" marker. In range -> `Some(idx)`; otherwise strict
/// error / lenient `None` + `DanglingReference` warning.
fn resolve_optional(
    idx: i32,
    count: usize,
    from: &'static str,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<Option<usize>, MapAssembleError> {
    if let Ok(u) = usize::try_from(idx)
        && u < count
    {
        return Ok(Some(u));
    }
    match strictness {
        Strictness::Strict => Err(MapAssembleError::DanglingReference {
            referent: "sidedef",
            index: idx,
            from,
            count,
        }),
        Strictness::Lenient => {
            warnings.push(MapWarning::DanglingReference {
                referent: "sidedef",
                index: idx,
                from,
                count,
            });
            Ok(None)
        }
    }
}

/// Resolves a binary linedef's sidedef reference (either side) against the
/// `0xffff` sentinel.
///
/// `0xffff` (65535) is the on-disk Doom/Hexen "no side" marker for **both**
/// sidedef fields — vanilla engines guard `sidenum[0]` and `sidenum[1]`
/// identically (`!= -1`; Chocolate Doom/Hexen `P_LoadLineDefs`), so a front of
/// `0xffff` is a valid frontless line, not a defect (ADR-0020). The sentinel
/// maps to `None` in both strictness modes with no warning. Any other value
/// outside `0..count` errors (strict) or becomes `None` + a warning (lenient);
/// a negative index (reachable only via the widened signed parameter) is
/// simply out of range.
///
/// This helper is for the **binary** (Doom/Hexen) normalizers only. The UDMF
/// normalizer must **not** call it: per ADR-0017 §2/§3 it range-checks UDMF
/// sidedef indices directly via [`resolve_optional`] — with `sideback` already
/// normalized to `Option<i32>` (`-1` → `None` in the parser) and `sidefront` a
/// required raw integer — so a valid UDMF sidedef index of 65535 is never
/// mistaken for the binary sentinel. (`raw` is `i32` only to reuse the shared
/// range-check plumbing; binary `u16` fields widen into it losslessly.)
fn resolve_binary_side(
    raw: i32,
    count: usize,
    from: &'static str,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<Option<usize>, MapAssembleError> {
    if raw == 0xffff {
        return Ok(None);
    }
    resolve_optional(raw, count, from, strictness, warnings)
}

/// Resolves a linedef's four cross-references (start/end vertex, right/left
/// sidedef) — the resolution is identical for the Doom and Hexen layouts, so
/// both normalizers share it. `0xffff` in either sidedef field yields `None`
/// for that side (no back side / no front side; ADR-0020).
// The four raw fields plus the two arena counts, strictness, and the warnings
// sink are each independently meaningful (not a natural struct); grouping them
// would only relocate, not reduce, the parameter count.
#[allow(clippy::too_many_arguments)]
fn resolve_linedef_refs(
    start_vertex: u16,
    end_vertex: u16,
    right_sidedef: u16,
    left_sidedef: u16,
    vertex_count: usize,
    sidedef_count: usize,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<(VertexIdx, VertexIdx, Option<SidedefIdx>, Option<SidedefIdx>), MapAssembleError> {
    let start = VertexIdx(resolve_required(
        i32::from(start_vertex),
        vertex_count,
        "vertex",
        "linedef",
        strictness,
        warnings,
    )?);
    let end = VertexIdx(resolve_required(
        i32::from(end_vertex),
        vertex_count,
        "vertex",
        "linedef",
        strictness,
        warnings,
    )?);
    let right = resolve_binary_side(
        i32::from(right_sidedef),
        sidedef_count,
        "linedef",
        strictness,
        warnings,
    )?
    .map(SidedefIdx);
    let left = resolve_binary_side(
        i32::from(left_sidedef),
        sidedef_count,
        "linedef",
        strictness,
        warnings,
    )?
    .map(SidedefIdx);
    Ok((start, end, right, left))
}

/// Widens raw `VERTEXES` records into normalized [`MapVertex`]es.
fn normalize_vertices(raw: &[common::Vertex]) -> Vec<MapVertex> {
    raw.iter()
        .map(|v| MapVertex {
            x: f64::from(v.x),
            y: f64::from(v.y),
        })
        .collect()
}

/// Widens raw `SECTORS` records into normalized [`MapSector`]s.
fn normalize_sectors(raw: &[common::Sector]) -> Vec<MapSector> {
    raw.iter()
        .map(|s| MapSector {
            floor_height: i32::from(s.floor_height),
            ceiling_height: i32::from(s.ceiling_height),
            floor_flat: TextureRef::Name(s.floor_texture.as_str_lossy()),
            ceiling_flat: TextureRef::Name(s.ceiling_texture.as_str_lossy()),
            light: i32::from(s.light_level),
            special: i32::from(s.special_type),
            tag: i32::from(s.tag),
            colors: None,
            flags: 0,
        })
        .collect()
}

/// Widens raw `THINGS` records into normalized [`MapThing`]s.
fn normalize_things(raw: &[doom::Thing]) -> Vec<MapThing> {
    raw.iter()
        .map(|t| MapThing {
            x: f64::from(t.x),
            y: f64::from(t.y),
            angle: t.angle,
            type_id: t.type_id,
            flags: u32::from(t.flags),
            id: 0,
            height: 0.0,
            special: Special {
                special: 0,
                args: [0; 5],
            },
        })
        .collect()
}

/// Widens raw `SIDEDEFS` records into normalized [`MapSidedef`]s, validating
/// each sidedef's sector cross-reference.
fn normalize_sidedefs(
    raw: &[common::Sidedef],
    sector_count: usize,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<Vec<MapSidedef>, MapAssembleError> {
    let mut sidedefs = Vec::with_capacity(raw.len());
    for sd in raw {
        let sector = SectorIdx(resolve_required(
            i32::from(sd.sector),
            sector_count,
            "sector",
            "sidedef",
            strictness,
            warnings,
        )?);
        sidedefs.push(MapSidedef {
            sector,
            x_offset: i32::from(sd.x_offset),
            y_offset: i32::from(sd.y_offset),
            upper: TextureRef::Name(sd.upper_texture.as_str_lossy()),
            lower: TextureRef::Name(sd.lower_texture.as_str_lossy()),
            middle: TextureRef::Name(sd.middle_texture.as_str_lossy()),
        });
    }
    Ok(sidedefs)
}

/// Widens raw `LINEDEFS` records into normalized [`MapLinedef`]s, validating
/// each linedef's vertex and sidedef cross-references.
fn normalize_linedefs(
    raw: &[doom::Linedef],
    vertex_count: usize,
    sidedef_count: usize,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<Vec<MapLinedef>, MapAssembleError> {
    let mut linedefs = Vec::with_capacity(raw.len());
    for ld in raw {
        let (start, end, right, left) = resolve_linedef_refs(
            ld.start_vertex,
            ld.end_vertex,
            ld.right_sidedef,
            ld.left_sidedef,
            vertex_count,
            sidedef_count,
            strictness,
            warnings,
        )?;
        linedefs.push(MapLinedef {
            start,
            end,
            right,
            left,
            flags: u32::from(ld.flags),
            special: Special {
                special: i32::from(ld.special_type),
                args: [i32::from(ld.sector_tag), 0, 0, 0, 0],
            },
            id: 0,
        });
    }
    Ok(linedefs)
}

/// Resolves one BSP node child (`right_child`/`left_child`): bit 15 set
/// selects a subsector leaf (remaining 15 bits into `subsector_count`), clear
/// selects an internal node (into `node_count`). A small named helper rather
/// than an inline closure — a closure capturing `warnings` mutably fights the
/// borrow checker across the two sequential calls per node.
fn resolve_node_child(
    raw: u16,
    node_count: usize,
    subsector_count: usize,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<NodeChild, MapAssembleError> {
    if raw & 0x8000 == 0 {
        Ok(NodeChild::Node(NodeIdx(resolve_required(
            i32::from(raw),
            node_count,
            "node",
            "node",
            strictness,
            warnings,
        )?)))
    } else {
        Ok(NodeChild::Subsector(SubsectorIdx(resolve_required(
            i32::from(raw & 0x7fff),
            subsector_count,
            "subsector",
            "node",
            strictness,
            warnings,
        )?)))
    }
}

/// Assembles the classic-family BSP arenas (`SEGS`/`SSECTORS`/`NODES`) for a
/// binary map that is **not** `DeePBSP` v4 — the caller detects and routes
/// `xNd4` ahead of this helper.
///
/// This is the ZDoom-extended and classic dispatch, unchanged from its former
/// inline form: extended node encodings (ZDBSP/GL) live in a single
/// self-describing blob (zdbsp writes the non-GL `XNOD`/`ZNOD` into `NODES` and
/// the GL `XGL*`/`ZGL*` family into `SSECTORS`, gzdoom
/// `ML_ZNODES`/`ML_GLZNODES`), so at most one of the two lumps carries a 4-byte
/// signature. An uncompressed `X*` stream is decoded in place (ADR-0025, #326);
/// a zlib `Z*` inflates and decodes through the same parser (ADR-0025 §5, #327);
/// a still-gated `Z*` (feature off) or unrecognized signature keeps #199's
/// extended-encoding gate (strict refuses, lenient skips the BSP arenas and
/// warns). Absent any signature, the classic 12/4/28-byte decoders run.
///
/// `vertices` is `&mut` because an extended `X*`/`Z*` stream can append GL
/// vertices; the classic and `DeePBSP` paths append none.
///
/// # Errors
///
/// Propagates the record-decode and normalization errors of the underlying
/// classic/extended decoders, and (strict mode) `UnsupportedNodeEncoding` for a
/// gated signature.
#[allow(clippy::type_complexity)]
fn assemble_binary_bsp(
    wad: &Wad,
    group: &MapGroup,
    vertices: &mut Vec<MapVertex>,
    linedefs: &[MapLinedef],
    options: &ParseOptions,
    warnings: &mut Vec<MapWarning>,
) -> Result<(Vec<MapSeg>, Vec<MapSubsector>, Vec<MapNode>), MapAssembleError> {
    let s = options.strictness;
    let extended = ["NODES", "SSECTORS"].into_iter().find_map(|lump| {
        let bytes = lump_bytes(wad, group, lump)?;
        let head: [u8; 4] = bytes.get(..4)?.try_into().ok()?;
        EXTENDED_NODE_SIGNATURES
            .contains(&&head)
            .then_some((lump, head, bytes))
    });
    if let Some((lump, signature, bytes)) = extended {
        match classify_signature(signature) {
            Some((kind, NodeCompression::Uncompressed)) => {
                let decoded = decode_extended_nodes(bytes, kind, vertices, linedefs, s, warnings)?;
                vertices.extend(decoded.new_vertices);
                Ok((decoded.segs, decoded.subsectors, decoded.nodes))
            }
            // A zlib-compressed `Z*` stream inflates to its uncompressed twin's
            // body and decodes through the same parser, bounded by
            // `Limits::max_decoded_node_bytes` (ADR-0025 §5, #327).
            #[cfg(feature = "extended-nodes-zlib")]
            Some((kind, NodeCompression::Zlib)) => {
                let decoded = decode_compressed_extended_nodes(
                    bytes,
                    kind,
                    vertices,
                    linedefs,
                    options.limits.max_decoded_node_bytes,
                    s,
                    warnings,
                )?;
                vertices.extend(decoded.new_vertices);
                Ok((decoded.segs, decoded.subsectors, decoded.nodes))
            }
            // A `Z*` stream without the `extended-nodes-zlib` feature, and any
            // unrecognized signature, keep #199's extended-encoding gate: strict
            // refuses, lenient skips the BSP arenas and warns.
            _ => match s {
                Strictness::Strict => {
                    Err(MapAssembleError::UnsupportedNodeEncoding { lump, signature })
                }
                Strictness::Lenient => {
                    warnings.push(MapWarning::UnsupportedNodeEncoding { lump });
                    Ok((Vec::new(), Vec::new(), Vec::new()))
                }
            },
        }
    } else {
        let raw_segs = decode_optional::<common::Seg>(wad, group, "SEGS")?;
        let raw_subsectors = decode_optional::<common::Subsector>(wad, group, "SSECTORS")?;
        let raw_nodes = decode_optional::<common::Node>(wad, group, "NODES")?;
        normalize_bsp_or_degrade(
            &raw_segs,
            &raw_subsectors,
            &raw_nodes,
            vertices.len(),
            linedefs.len(),
            s,
            warnings,
        )
    }
}

/// Runs [`normalize_bsp`], degrading the whole BSP in lenient mode when a
/// reference cannot be recovered by clamping — a child pointing into an
/// **empty** arena has nothing to clamp to, and BSP data is optional
/// (ADR-0015 §5), so lenient assembly must not fail on it. The error's
/// details are preserved as a single [`MapWarning::DanglingReference`] — any
/// per-element warnings pushed for the now-dropped arenas are discarded — and
/// all three arenas come back empty (the same whole-BSP posture as the
/// extended-encoding gate). Strict mode propagates the error unchanged.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn normalize_bsp_or_degrade(
    raw_segs: &[common::Seg],
    raw_subsectors: &[common::Subsector],
    raw_nodes: &[common::Node],
    vertex_count: usize,
    linedef_count: usize,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<(Vec<MapSeg>, Vec<MapSubsector>, Vec<MapNode>), MapAssembleError> {
    // Snapshot so a degrade discards the per-element warnings that described
    // the dropped arenas — the caller sees exactly one warning for the whole
    // degrade, not a trail of diagnostics about data that no longer exists.
    let warning_watermark = warnings.len();
    match normalize_bsp(
        raw_segs,
        raw_subsectors,
        raw_nodes,
        vertex_count,
        linedef_count,
        strictness,
        warnings,
    ) {
        Err(MapAssembleError::DanglingReference {
            referent,
            index,
            from,
            count,
        }) if strictness == Strictness::Lenient => {
            warnings.truncate(warning_watermark);
            warnings.push(MapWarning::DanglingReference {
                referent,
                index,
                from,
                count,
            });
            Ok((Vec::new(), Vec::new(), Vec::new()))
        }
        other => other,
    }
}

/// Normalizes the engine-built BSP lumps into the graph arenas (ADR-0015 §1).
///
/// Cross-references resolve with the standard pattern: strict errors on the
/// first dangling reference; lenient clamps indices to `0` (or truncates a
/// subsector's seg run to the arena) and warns. Iterative throughout — the
/// tree is stored, not walked, so crafted cycles cannot recurse anything.
///
/// # Errors
/// Returns [`MapAssembleError::DanglingReference`] in strict mode if any seg's
/// vertex/linedef reference, subsector's seg run, or node's child reference is
/// out of range.
// The three-arena return tuple is the shared normalizer's whole point (Task 3
// reuses it for Doom 64); a named struct would only relocate, not reduce, it.
#[allow(clippy::type_complexity)]
fn normalize_bsp(
    raw_segs: &[common::Seg],
    raw_subsectors: &[common::Subsector],
    raw_nodes: &[common::Node],
    vertex_count: usize,
    linedef_count: usize,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<(Vec<MapSeg>, Vec<MapSubsector>, Vec<MapNode>), MapAssembleError> {
    let mut segs = Vec::with_capacity(raw_segs.len());
    for sg in raw_segs {
        segs.push(MapSeg {
            start: VertexIdx(resolve_required(
                i32::from(sg.start_vertex),
                vertex_count,
                "vertex",
                "seg",
                strictness,
                warnings,
            )?),
            end: VertexIdx(resolve_required(
                i32::from(sg.end_vertex),
                vertex_count,
                "vertex",
                "seg",
                strictness,
                warnings,
            )?),
            angle: sg.angle,
            linedef: Some(LinedefIdx(resolve_required(
                i32::from(sg.linedef),
                linedef_count,
                "linedef",
                "seg",
                strictness,
                warnings,
            )?)),
            direction: sg.direction,
            offset: i32::from(sg.offset),
        });
    }

    let mut subsectors = Vec::with_capacity(raw_subsectors.len());
    for ss in raw_subsectors {
        let first = usize::from(ss.first_seg);
        let end = first + usize::from(ss.seg_count);
        let range = if end <= segs.len() && first <= segs.len() {
            first..end
        } else {
            match strictness {
                Strictness::Strict => {
                    return Err(MapAssembleError::DanglingReference {
                        referent: "seg",
                        index: i32::try_from(end).unwrap_or(i32::MAX),
                        from: "subsector",
                        count: segs.len(),
                    });
                }
                Strictness::Lenient => {
                    warnings.push(MapWarning::DanglingReference {
                        referent: "seg",
                        index: i32::try_from(end).unwrap_or(i32::MAX),
                        from: "subsector",
                        count: segs.len(),
                    });
                    first.min(segs.len())..segs.len()
                }
            }
        };
        subsectors.push(MapSubsector {
            segs: range,
            leafs: 0..0,
        });
    }

    let node_count = raw_nodes.len();
    let subsector_count = subsectors.len();
    let mut nodes = Vec::with_capacity(node_count);
    for nd in raw_nodes {
        let right = resolve_node_child(
            nd.right_child,
            node_count,
            subsector_count,
            strictness,
            warnings,
        )?;
        let left = resolve_node_child(
            nd.left_child,
            node_count,
            subsector_count,
            strictness,
            warnings,
        )?;
        nodes.push(MapNode {
            x: i32::from(nd.x),
            y: i32::from(nd.y),
            dx: i32::from(nd.dx),
            dy: i32::from(nd.dy),
            right_bbox: nd.right_bbox.map(i32::from),
            left_bbox: nd.left_bbox.map(i32::from),
            right,
            left,
        });
    }

    Ok((segs, subsectors, nodes))
}

/// Translates a raw Hexen `THINGS` flag word into the graph's single
/// Doom/Boom-MBF thing-flag layout ([`MapThing::flags`], ADR-0019 §2).
///
/// Hexen's on-disk bits are *not* Doom's: its game-mode bits are **positive**
/// ("appears in X") and live at `0x0100`/`0x0200`/`0x0400`, where Doom's are
/// **negative** ("not in X") at bits 4/5/6; and Hexen spends bits 4–7 on
/// `dormant` plus the fighter/cleric/mage class filters. Translating here keeps
/// [`MapThing::flags`] meaning exactly one thing for every source format, so the
/// writers ([`write_udmf`](crate::map::write_udmf),
/// [`write_doom_map`](crate::map::write_doom_map)) can interpret it uniformly.
///
/// | Hexen (on disk) | Normalized |
/// |---|---|
/// | skill 1&2 / 3 / 4&5 (bits 0–2), ambush (bit 3) | copied unchanged |
/// | appears in single-player (`0x0100`) | bit 4 — *not* in single-player (inverted) |
/// | appears in deathmatch (`0x0400`) | bit 5 — *not* in deathmatch (inverted) |
/// | appears in co-op (`0x0200`) | bit 6 — *not* in co-op (inverted) |
/// | dormant (`0x0010`), class filters (`0x0020`/`0x0040`/`0x0080`) | dropped — no Doom equivalent |
/// | — | bit 7 (friend, MBF) is always `0`; Hexen has no equivalent |
///
/// Dropping the dormant and class bits is silent and unwarned, consistent with
/// how ADR-0017/ADR-0019 treat every other unmappable per-format boolean.
fn normalize_hexen_thing_flags(flags: u16) -> u32 {
    /// Hexen "appears in single-player games".
    const HEXEN_SINGLE: u16 = 0x0100;
    /// Hexen "appears in cooperative games".
    const HEXEN_COOP: u16 = 0x0200;
    /// Hexen "appears in deathmatch games".
    const HEXEN_DEATHMATCH: u16 = 0x0400;

    // Skills (bits 0-2) and ambush (bit 3) share Doom's meaning and position.
    let mut out = u32::from(flags & 0x000F);
    if flags & HEXEN_SINGLE == 0 {
        out |= 0x0010; // not in single-player
    }
    if flags & HEXEN_DEATHMATCH == 0 {
        out |= 0x0020; // not in deathmatch
    }
    if flags & HEXEN_COOP == 0 {
        out |= 0x0040; // not in co-op
    }
    out
}

/// Translates Doom 64 on-disk thing flags into the graph's normalized
/// Doom/Boom layout (ADR-0019 §2, ADR-0021 §2).
///
/// Verified against Doom64 EX `doomdef.h`: `MTF_EASY`/`MTF_NORMAL`/`MTF_HARD`/
/// `MTF_AMBUSH`/`MTF_MULTI` (1/2/4/8/16) already sit on the normalized bit
/// positions 0–4 (Doom 64's difficulty bits are positive per-skill spawn
/// flags, matching the normalized meaning); `MTF_NODEATHMATCH` (1024) maps to
/// bit 5 and `MTF_NONETGAME` (2048) to bit 6 (co-op). The Doom 64-only bits —
/// `MTF_SPAWN`/`MTF_ONTOUCH`/`MTF_ONDEATH`/`MTF_SECRET`/`MTF_NOINFIGHTING`/
/// `MTF_NIGHTMARE` — have no normalized slot and drop, exactly as Hexen's
/// dormant/class bits do. Bit 7 (friendly) is never set — Doom 64 has no such
/// flag. The raw word remains available via `Doom64Map`.
fn normalize_doom64_thing_flags(raw: i16) -> u32 {
    #[allow(clippy::cast_sign_loss)] // bit reinterpretation is intended
    let raw = raw as u16;
    let mut flags = u32::from(raw & 0b1_1111); // EASY|NORMAL|HARD|AMBUSH|MULTI
    if raw & 1024 != 0 {
        flags |= 0b10_0000; // not-in-deathmatch
    }
    if raw & 2048 != 0 {
        flags |= 0b100_0000; // not-in-co-op (Doom 64 "standard netgame")
    }
    flags
}

/// Widens raw Hexen `THINGS` records into normalized [`MapThing`]s, translating
/// the Hexen flag word into the graph's Doom/Boom-MBF layout (see
/// [`normalize_hexen_thing_flags`]).
fn normalize_things_hexen(raw: &[hexen::Thing]) -> Vec<MapThing> {
    raw.iter()
        .map(|t| MapThing {
            x: f64::from(t.x),
            y: f64::from(t.y),
            angle: t.angle,
            type_id: t.type_id,
            flags: normalize_hexen_thing_flags(t.flags),
            id: i32::from(t.tid),
            height: f64::from(t.z),
            special: Special {
                special: i32::from(t.special),
                args: t.args.map(i32::from),
            },
        })
        .collect()
}

/// Widens raw Hexen `LINEDEFS` records into normalized [`MapLinedef`]s, validating
/// each linedef's vertex and sidedef cross-references (via [`resolve_linedef_refs`]).
fn normalize_linedefs_hexen(
    raw: &[hexen::Linedef],
    vertex_count: usize,
    sidedef_count: usize,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<Vec<MapLinedef>, MapAssembleError> {
    let mut linedefs = Vec::with_capacity(raw.len());
    for ld in raw {
        let (start, end, right, left) = resolve_linedef_refs(
            ld.start_vertex,
            ld.end_vertex,
            ld.right_sidedef,
            ld.left_sidedef,
            vertex_count,
            sidedef_count,
            strictness,
            warnings,
        )?;
        linedefs.push(MapLinedef {
            start,
            end,
            right,
            left,
            flags: u32::from(ld.flags),
            special: Special {
                special: i32::from(ld.special),
                args: ld.args.map(i32::from),
            },
            id: 0,
        });
    }
    Ok(linedefs)
}

/// Narrows an `i32` UDMF value into `u16`: strict rejects out-of-range;
/// lenient clamps to `u16` bounds and records a [`MapWarning::FieldOutOfRange`].
fn coerce_u16(
    value: i32,
    field: &'static str,
    from: &'static str,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<u16, MapAssembleError> {
    if let Ok(v) = u16::try_from(value) {
        return Ok(v);
    }
    match strictness {
        Strictness::Strict => Err(MapAssembleError::FieldOutOfRange { field, from, value }),
        Strictness::Lenient => {
            warnings.push(MapWarning::FieldOutOfRange { field, from, value });
            Ok(if value < 0 { 0 } else { u16::MAX })
        }
    }
}

/// Widens raw UDMF `VERTEX` records into normalized [`MapVertex`]es.
fn normalize_udmf_vertices(raw: &[crate::map::udmf::UdmfVertex]) -> Vec<MapVertex> {
    raw.iter().map(|v| MapVertex { x: v.x, y: v.y }).collect()
}

/// Widens raw UDMF `SECTOR` records into normalized [`MapSector`]s.
fn normalize_udmf_sectors(raw: &[crate::map::udmf::UdmfSector]) -> Vec<MapSector> {
    raw.iter()
        .map(|s| MapSector {
            floor_height: s.heightfloor,
            ceiling_height: s.heightceiling,
            floor_flat: TextureRef::Name(s.texturefloor.clone()),
            ceiling_flat: TextureRef::Name(s.textureceiling.clone()),
            light: s.lightlevel,
            special: s.special,
            tag: s.id,
            colors: None,
            flags: 0,
        })
        .collect()
}

/// Widens raw UDMF `SIDEDEF` records into normalized [`MapSidedef`]s, validating
/// each sidedef's sector cross-reference.
fn normalize_udmf_sidedefs(
    raw: &[crate::map::udmf::UdmfSidedef],
    sector_count: usize,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<Vec<MapSidedef>, MapAssembleError> {
    let mut out = Vec::with_capacity(raw.len());
    for sd in raw {
        let sector = SectorIdx(resolve_required(
            sd.sector,
            sector_count,
            "sector",
            "sidedef",
            strictness,
            warnings,
        )?);
        out.push(MapSidedef {
            sector,
            x_offset: sd.offsetx,
            y_offset: sd.offsety,
            upper: TextureRef::Name(sd.texturetop.clone()),
            lower: TextureRef::Name(sd.texturebottom.clone()),
            middle: TextureRef::Name(sd.texturemiddle.clone()),
        });
    }
    Ok(out)
}

/// Widens raw UDMF `LINEDEF` records into normalized [`MapLinedef`]s, validating
/// each linedef's vertex and sidedef cross-references. Does not use the binary
/// `0xffff` sentinel for one-sided; instead routes `sideback: None` to `left: None`
/// and validates real `Some(idx)` values via [`resolve_optional`] (ADR-0017 §2).
fn normalize_udmf_linedefs(
    raw: &[crate::map::udmf::UdmfLinedef],
    vertex_count: usize,
    sidedef_count: usize,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<Vec<MapLinedef>, MapAssembleError> {
    let mut out = Vec::with_capacity(raw.len());
    for ld in raw {
        let start = VertexIdx(resolve_required(
            ld.v1,
            vertex_count,
            "vertex",
            "linedef",
            strictness,
            warnings,
        )?);
        let end = VertexIdx(resolve_required(
            ld.v2,
            vertex_count,
            "vertex",
            "linedef",
            strictness,
            warnings,
        )?);
        // `sidefront` is required by the UDMF parser (spec: no valid default);
        // a dangling or negative value here resolves like any optional sidedef
        // reference — strict error, lenient `None` + warning (ADR-0020 §3) —
        // rather than clamping to index 0, which fabricated a reference not
        // present in the source.
        let right = resolve_optional(ld.sidefront, sidedef_count, "linedef", strictness, warnings)?
            .map(SidedefIdx);
        let left = match ld.sideback {
            None => None,
            Some(idx) => resolve_optional(idx, sidedef_count, "linedef", strictness, warnings)?
                .map(SidedefIdx),
        };
        out.push(MapLinedef {
            start,
            end,
            right,
            left,
            flags: ld.flags,
            special: Special {
                special: ld.special,
                args: ld.args,
            },
            id: ld.id,
        });
    }
    Ok(out)
}

/// Widens raw UDMF `THING` records into normalized [`MapThing`]s, coercing
/// `type_id` to `u16`, wrapping `angle` modulo 360, and carrying the packed
/// Doom/Boom-MBF thing flags through (ADR-0019, amending ADR-0017 §1).
fn normalize_udmf_things(
    raw: &[crate::map::udmf::UdmfThing],
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<Vec<MapThing>, MapAssembleError> {
    let mut out = Vec::with_capacity(raw.len());
    for t in raw {
        let type_id = coerce_u16(t.type_id, "thing.type", "thing", strictness, warnings)?;
        // `rem_euclid(360)` yields 0..=359 for any i32, which always fits u16;
        // the conversion is infallible by construction.
        let angle = u16::try_from(t.angle.rem_euclid(360))
            .expect("rem_euclid(360) is in 0..=359, which always fits u16");
        out.push(MapThing {
            x: t.x,
            y: t.y,
            angle,
            type_id,
            flags: t.flags,
            id: t.id,
            height: t.height,
            special: Special {
                special: t.special,
                args: t.args,
            },
        });
    }
    Ok(out)
}

impl Map {
    /// Assembles a map from a WAD and one of its groups, using strict rules.
    ///
    /// This is a convenience wrapper over [`Map::assemble_with_options`] with
    /// [`ParseOptions::default()`], which is strict: the first out-of-range
    /// cross-reference or structural failure aborts assembly.
    ///
    /// # Errors
    /// Returns [`MapAssembleError`] if a required lump is missing, a record lump
    /// fails to decode, any cross-reference is out of range, a classic binary
    /// map's `NODES`/`SSECTORS` lump carries an unsupported extended node
    /// encoding (see issue #199), or — for a Doom 64 group (ADR-0021 §2) — the
    /// marker's nested WAD fails to read.
    pub fn assemble(wad: &Wad, group: &MapGroup) -> Result<Map, MapAssembleError> {
        Map::assemble_with_options(wad, group, ParseOptions::default())
    }

    /// Assembles a map under explicit options (ADR-0015 §3).
    ///
    /// Equivalent to [`Map::assemble_with_gl_source`] with `gl_wad: None`.
    ///
    /// # Errors
    /// See [`Map::assemble_with_gl_source`].
    pub fn assemble_with_options(
        wad: &Wad,
        group: &MapGroup,
        options: ParseOptions,
    ) -> Result<Map, MapAssembleError> {
        Self::assemble_with_gl_source(wad, group, None, options)
    }

    /// Assembles a map under explicit options, with an optional external
    /// source of classic GL-node lumps (ADR-0015 §3; `gl_wad` added for the
    /// `.gwa` sidecar reader, #342).
    ///
    /// `gl_wad` is an optional sibling [`Wad`] (e.g. a loaded `.gwa`)
    /// supplying the map's classic GL-node lumps; preferred over any in-WAD
    /// GL group, with in-WAD as fallback. `None` reads GL nodes from `wad`
    /// only (as [`Map::assemble_with_options`] does).
    ///
    /// A Doom 64 group (detected via the marker's nested `IWAD`/`PWAD` magic,
    /// ADR-0021 §1) is read via [`doom64::read_doom64_map`] and normalized
    /// separately from the classic binary/UDMF paths below (ADR-0021 §2).
    ///
    /// # Errors
    /// Returns [`MapAssembleError`] if a required lump is missing, a record lump
    /// fails to decode, or (in strict mode) a cross-reference is out of range.
    /// In lenient mode only structural failures (missing lump, undecodable
    /// records, an empty *required* target arena) return an error. A **classic
    /// binary** map's `NODES`/`SSECTORS` lump carrying an *uncompressed*
    /// `ZDoom` extended-node stream (`XNOD`/`XGLN`/`XGL2`/`XGL3`) is decoded in
    /// place into the BSP arenas (ADR-0025, #326); a still-unsupported extended
    /// encoding (the zlib-wrapped `Z*` twins, #327) is
    /// [`MapAssembleError::UnsupportedNodeEncoding`] in strict mode, or skipped
    /// in lenient mode with **all three** BSP arenas left empty plus one
    /// warning per gated lump. A **classic binary** map whose `NODES` lump
    /// begins with the 8-byte `DeePBSP` v4 signature (`xNd4`) is instead
    /// decoded as `DeePBSP` into the BSP arenas ahead of this gate (ADR-0025
    /// Stage 3, #328); a structurally malformed `DeePBSP` lump is a
    /// [`MapAssembleError::Records`] in **both** strictness modes (the
    /// framing-defect policy — `DeePBSP` mirrors the classic path it resembles,
    /// not the `ZDoom` readers' lenient degrade). Separately, a classic
    /// **binary** map carrying classic-GL node lumps — from `gl_wad` when
    /// supplied, or an in-WAD `GL_<mapname>` group otherwise (ADR-0025
    /// amendment, #324) — has that group decoded into its own, additive
    /// `gl_vertices`/`gl_segs`/`gl_subsectors`/`gl_nodes` arenas alongside
    /// (not instead of) the vanilla BSP above; a refused GL version (V1/V4) is
    /// [`MapAssembleError::UnsupportedGlNodeVersion`] in strict mode, or a
    /// warning with empty GL arenas in lenient mode. The gate does not apply
    /// to Doom 64 nested
    /// sub-lumps, whose records were already decoded by
    /// [`doom64::read_doom64_map`]. Lenient mode likewise degrades the whole
    /// BSP (empty arenas, one warning) when a BSP reference cannot be clamped,
    /// e.g. a node child pointing into an empty arena — optional BSP data
    /// never fails a lenient assembly. For a
    /// Doom 64 group, [`MapAssembleError::Doom64`] wraps a failure to read the
    /// marker's nested WAD (both modes) or a missing/undecodable sub-lump
    /// (strict mode; ADR-0021 §2).
    #[allow(clippy::too_many_lines)]
    pub fn assemble_with_gl_source(
        wad: &Wad,
        group: &MapGroup,
        gl_wad: Option<&Wad>,
        options: ParseOptions,
    ) -> Result<Map, MapAssembleError> {
        let mut warnings = Vec::new();

        match crate::map::detect_map_format(wad, group) {
            MapFormat::Udmf => assemble_udmf(wad, group, options, warnings),
            MapFormat::Doom64 => assemble_doom64(wad, group, options, warnings),
            format => {
                let s = options.strictness;

                // Records shared by both binary formats.
                let raw_verts = decode_required::<common::Vertex>(wad, group, "VERTEXES")?;
                let raw_sectors = decode_required::<common::Sector>(wad, group, "SECTORS")?;
                let raw_sides = decode_required::<common::Sidedef>(wad, group, "SIDEDEFS")?;

                let mut vertices = normalize_vertices(&raw_verts);
                let sectors = normalize_sectors(&raw_sectors);
                let sidedefs = normalize_sidedefs(&raw_sides, sectors.len(), s, &mut warnings)?;

                // Format-specific THINGS/LINEDEFS.
                let (things, linedefs) = match format {
                    MapFormat::Doom => {
                        let raw_lines = decode_required::<doom::Linedef>(wad, group, "LINEDEFS")?;
                        let raw_things = decode_required::<doom::Thing>(wad, group, "THINGS")?;
                        let linedefs = normalize_linedefs(
                            &raw_lines,
                            vertices.len(),
                            sidedefs.len(),
                            s,
                            &mut warnings,
                        )?;
                        (normalize_things(&raw_things), linedefs)
                    }
                    MapFormat::Hexen => {
                        let raw_lines = decode_required::<hexen::Linedef>(wad, group, "LINEDEFS")?;
                        let raw_things = decode_required::<hexen::Thing>(wad, group, "THINGS")?;
                        let linedefs = normalize_linedefs_hexen(
                            &raw_lines,
                            vertices.len(),
                            sidedefs.len(),
                            s,
                            &mut warnings,
                        )?;
                        (normalize_things_hexen(&raw_things), linedefs)
                    }
                    MapFormat::Udmf => unreachable!("Udmf is handled by the outer match arm"),
                    // Genuinely unreachable: the outer match already routes every
                    // Doom64-detected group (including a classic-named marker
                    // whose bytes carry nested IWAD/PWAD magic, ADR-0021 §1) to
                    // `assemble_doom64` before this binary-format fallback is
                    // ever entered. Delegating (rather than panicking) keeps
                    // this arm's behavior coherent with the outer arm's on the
                    // off chance the routing invariant above is ever violated.
                    MapFormat::Doom64 => return assemble_doom64(wad, group, options, warnings),
                };

                // `DeePBSP` v4 (`xNd4`) is a classic-widened node format: it keeps the
                // three separate `SEGS`/`SSECTORS`/`NODES` lumps but widens the records,
                // and heads its `NODES` lump with an 8-byte `xNd4\0\0\0\0` signature
                // (distinct from the 4-byte ZDoom signatures below). Detect it FIRST and
                // decode in place (ADR-0025 Stage 3, #328); it adds no new vertices. A
                // malformed `DeePBSP` lump is a hard `Records` error in both modes, like
                // the classic path it structurally resembles (ADR-0025 framing-defect
                // policy). A `NODES` lump without the `xNd4` signature falls through to
                // the 4-byte extended check, then classic — unchanged.
                let deepbsp_nodes = lump_bytes(wad, group, "NODES").filter(|b| is_deepbsp(b));
                let (segs, subsectors, nodes) = if let Some(nodes_bytes) = deepbsp_nodes {
                    decode_deepbsp(
                        lump_bytes(wad, group, "SEGS").unwrap_or_default(),
                        lump_bytes(wad, group, "SSECTORS").unwrap_or_default(),
                        nodes_bytes,
                        vertices.len(),
                        linedefs.len(),
                        s,
                        &mut warnings,
                    )?
                } else {
                    assemble_binary_bsp(
                        wad,
                        group,
                        &mut vertices,
                        &linedefs,
                        &options,
                        &mut warnings,
                    )?
                };

                let (reject, blockmap) = decode_reject_blockmap(
                    lump_bytes(wad, group, "REJECT"),
                    lump_bytes(wad, group, "BLOCKMAP"),
                    sectors.len(),
                    linedefs.len(),
                    s,
                    &mut warnings,
                )?;

                // Classic GL nodes (`GL_<mapname>`) are additive: they augment
                // the vanilla `SEGS`/`SSECTORS`/`NODES` graph above with glBSP's
                // higher-precision minisegs and BSP (#324, ADR-0025). Decode them
                // when present, leaving the arenas empty otherwise. In strict
                // mode a refusal (V1/V4) or framing defect propagates; in lenient
                // mode `decode_gl_group` returns empty arenas plus a warning.
                //
                // #342: prefer a GL group from the caller-supplied `.gwa`
                // (`gl_wad`), else fall back to an in-WAD group. The GL lump
                // BYTES come from whichever Wad won, but the normal-vertex/
                // linedef reference bounds are always the MAIN map's arenas.
                let gl_group = gl_wad
                    .and_then(|gw| {
                        crate::map::group::gl_group_in_gl_wad(gw, &group.name).map(|grp| (gw, grp))
                    })
                    .or_else(|| crate::map::group::gl_group_for(wad, group).map(|grp| (wad, grp)));
                let (gl_vertices, gl_segs, gl_subsectors, gl_nodes) =
                    if let Some((src, g)) = gl_group {
                        let decoded = crate::map::gl::decode_gl_group(
                            src.lump_bytes(g.vert).unwrap_or_default(),
                            src.lump_bytes(g.segs).unwrap_or_default(),
                            src.lump_bytes(g.ssect).unwrap_or_default(),
                            src.lump_bytes(g.nodes).unwrap_or_default(),
                            vertices.len(),
                            linedefs.len(),
                            s,
                            &mut warnings,
                        )?;
                        (
                            decoded.vertices,
                            decoded.segs,
                            decoded.subsectors,
                            decoded.nodes,
                        )
                    } else {
                        (Vec::new(), Vec::new(), Vec::new(), Vec::new())
                    };

                Ok(Map {
                    name: group.name.clone(),
                    format,
                    namespace: None,
                    vertices,
                    linedefs,
                    sidedefs,
                    sectors,
                    things,
                    lights: vec![],
                    segs,
                    subsectors,
                    nodes,
                    gl_vertices,
                    gl_segs,
                    gl_subsectors,
                    gl_nodes,
                    leafs: Vec::new(),
                    macros: Vec::new(),
                    reject,
                    blockmap,
                    warnings,
                })
            }
        }
    }
}

/// Widens raw Doom 64 `VERTEXES` (16.16 fixed-point) into normalized
/// [`MapVertex`]es.
fn normalize_doom64_vertices(raw: &[doom64::Vertex]) -> Vec<MapVertex> {
    raw.iter()
        .map(|v| MapVertex {
            x: f64::from(v.x) / 65536.0,
            y: f64::from(v.y) / 65536.0,
        })
        .collect()
}

/// Builds the map's light table the way the engine does (Doom64 EX
/// `P_LoadLights`): 256 implicit identity-grayscale entries (`r = g = b =
/// index`, `tag = 0`) followed by the map's `LIGHTS` lump records. A sector
/// color value below 256 therefore selects a grayscale light level, and a
/// value `>= 256` selects `LIGHTS` record `value - 256` (ADR-0021 §4).
fn normalize_doom64_lights(raw: &[doom64::Light]) -> Vec<MapLight> {
    let mut lights = Vec::with_capacity(256 + raw.len());
    for i in 0u8..=255u8 {
        lights.push(MapLight {
            r: i,
            g: i,
            b: i,
            tag: 0,
        });
    }
    lights.extend(raw.iter().map(|l| MapLight {
        r: l.r,
        g: l.g,
        b: l.b,
        tag: l.tag,
    }));
    lights
}

/// Resolves one on-disk Doom 64 texture hash against the outer WAD's
/// texture-name table (ADR-0022 §1/§4): a hit becomes a name; a miss with
/// a table present is a strict error / lenient keep-index-and-warn; no
/// table (no `Textures` section — a bare nested-map WAD) keeps the index
/// silently.
fn resolve_texture_ref(
    hash: u16,
    textures: Option<&Doom64TextureNames>,
    from: &'static str,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<TextureRef, MapAssembleError> {
    let Some(table) = textures else {
        return Ok(TextureRef::Index(hash));
    };
    if let Some(name) = table.get(hash) {
        return Ok(TextureRef::Name(name.to_owned()));
    }
    match strictness {
        Strictness::Strict => Err(MapAssembleError::UnresolvedTextureHash { hash, from }),
        Strictness::Lenient => {
            warnings.push(MapWarning::UnresolvedTextureHash { hash, from });
            Ok(TextureRef::Index(hash))
        }
    }
}

/// Widens raw Doom 64 `SECTORS` records into normalized [`MapSector`]s,
/// validating each of the five colored-lighting references against
/// `light_count` — the length of the engine-style combined light table (256
/// implicit grayscale entries plus the `LIGHTS` lump records; see
/// [`normalize_doom64_lights`]) — and resolving each flat hash against
/// `textures` (ADR-0022 §4; `None` keeps the index unresolved).
fn normalize_doom64_sectors(
    raw: &[doom64::Sector],
    light_count: usize,
    textures: Option<&Doom64TextureNames>,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<Vec<MapSector>, MapAssembleError> {
    let mut sectors = Vec::with_capacity(raw.len());
    for sec in raw {
        let mut colors = [LightIdx(0); 5];
        for (slot, &c) in colors.iter_mut().zip(&sec.colors) {
            *slot = LightIdx(resolve_required(
                i32::from(c),
                light_count,
                "light",
                "sector",
                strictness,
                warnings,
            )?);
        }
        sectors.push(MapSector {
            floor_height: i32::from(sec.floor_height),
            ceiling_height: i32::from(sec.ceiling_height),
            floor_flat: resolve_texture_ref(
                sec.floor_tex,
                textures,
                "sector",
                strictness,
                warnings,
            )?,
            ceiling_flat: resolve_texture_ref(
                sec.ceiling_tex,
                textures,
                "sector",
                strictness,
                warnings,
            )?,
            light: 0, // Doom 64 has no scalar light level (ADR-0021 §2)
            special: i32::from(sec.special),
            tag: i32::from(sec.tag),
            colors: Some(colors),
            flags: u32::from(sec.flags),
        });
    }
    Ok(sectors)
}

/// Widens raw Doom 64 `SIDEDEFS` records into normalized [`MapSidedef`]s,
/// validating each sidedef's sector cross-reference and resolving each
/// texture hash against `textures` (ADR-0022 §4; `None` keeps the index
/// unresolved).
fn normalize_doom64_sidedefs(
    raw: &[doom64::Sidedef],
    sector_count: usize,
    textures: Option<&Doom64TextureNames>,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<Vec<MapSidedef>, MapAssembleError> {
    let mut sidedefs = Vec::with_capacity(raw.len());
    for sd in raw {
        sidedefs.push(MapSidedef {
            sector: SectorIdx(resolve_required(
                i32::from(sd.sector),
                sector_count,
                "sector",
                "sidedef",
                strictness,
                warnings,
            )?),
            x_offset: i32::from(sd.x_offset),
            y_offset: i32::from(sd.y_offset),
            upper: resolve_texture_ref(sd.upper, textures, "sidedef", strictness, warnings)?,
            lower: resolve_texture_ref(sd.lower, textures, "sidedef", strictness, warnings)?,
            middle: resolve_texture_ref(sd.middle, textures, "sidedef", strictness, warnings)?,
        });
    }
    Ok(sidedefs)
}

/// Widens raw Doom 64 `LINEDEFS` records into normalized [`MapLinedef`]s,
/// validating each linedef's vertex and sidedef cross-references. The `tag`
/// field carries into `special.args[0]`, mirroring classic Doom's sector tag
/// (see [`normalize_linedefs`]).
fn normalize_doom64_linedefs(
    raw: &[doom64::Linedef],
    vertex_count: usize,
    sidedef_count: usize,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<Vec<MapLinedef>, MapAssembleError> {
    let mut linedefs = Vec::with_capacity(raw.len());
    for ld in raw {
        let (start, end, right, left) = resolve_linedef_refs(
            ld.v1,
            ld.v2,
            ld.sidefront,
            ld.sideback,
            vertex_count,
            sidedef_count,
            strictness,
            warnings,
        )?;
        linedefs.push(MapLinedef {
            start,
            end,
            right,
            left,
            flags: ld.flags,
            special: Special {
                special: i32::from(ld.special),
                args: [i32::from(ld.tag), 0, 0, 0, 0],
            },
            id: 0,
        });
    }
    Ok(linedefs)
}

/// Widens raw Doom 64 `THINGS` records into normalized [`MapThing`]s,
/// translating the on-disk flag word via [`normalize_doom64_thing_flags`] and
/// carrying `z`/`id` into [`MapThing::height`]/[`MapThing::id`].
fn normalize_doom64_things(
    raw: &[doom64::Thing],
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<Vec<MapThing>, MapAssembleError> {
    let mut things = Vec::with_capacity(raw.len());
    for t in raw {
        let type_id = coerce_u16(
            i32::from(t.type_id),
            "thing.type",
            "thing",
            strictness,
            warnings,
        )?;
        // `rem_euclid(360)` yields 0..=359 for any i32, which always fits u16;
        // the conversion is infallible by construction.
        let angle = u16::try_from(i32::from(t.angle).rem_euclid(360))
            .expect("rem_euclid(360) is in 0..=359, which always fits u16");
        things.push(MapThing {
            x: f64::from(t.x),
            y: f64::from(t.y),
            angle,
            type_id,
            flags: normalize_doom64_thing_flags(t.flags),
            id: i32::from(t.id),
            height: f64::from(t.z),
            special: Special {
                special: 0,
                args: [0; 5],
            },
        });
    }
    Ok(things)
}

/// Assembles a Doom 64 nested-WAD map into the graph (ADR-0021 §2).
///
/// Reads the group's marker lump bytes as a nested WAD (see
/// [`doom64::read_doom64_map`]) and widens every raw record into the graph's
/// normalized shapes: fixed-point vertices to `f64`, texture/flat `u16`
/// indices to [`TextureRef::Index`], each sector's five raw color IDs to
/// validated [`LightIdx`]es into [`Map::lights`], and the on-disk thing flag
/// word via [`normalize_doom64_thing_flags`]. Doom 64 has no scalar sector
/// light level, so [`MapSector::light`] is always `0`.
fn assemble_doom64(
    wad: &Wad,
    group: &MapGroup,
    options: ParseOptions,
    mut warnings: Vec<MapWarning>,
) -> Result<Map, MapAssembleError> {
    let bytes = wad.lump_data(&wad.lumps()[group.marker_index]);
    let raw = doom64::read_doom64_map(bytes, &options)
        .map_err(|source| MapAssembleError::Doom64 { source })?;
    let s = options.strictness;

    // ADR-0022 §4: build the texture-name table from the OUTER wad's
    // sections, bridging scan anomalies into this map's error/warning
    // stream (the Doom64Warning wrapping precedent).
    let texture_names = match wad.sections_with_options(options) {
        Ok(section_table) => {
            warnings.extend(
                section_table
                    .warnings()
                    .iter()
                    .cloned()
                    .map(MapWarning::TextureSection),
            );
            Doom64TextureNames::from_sections(wad, &section_table)
        }
        Err(source) => return Err(MapAssembleError::TextureSections { source }),
    };

    let vertices = normalize_doom64_vertices(&raw.vertexes);
    let lights = normalize_doom64_lights(&raw.lights);
    let sectors = normalize_doom64_sectors(
        &raw.sectors,
        lights.len(),
        texture_names.as_ref(),
        s,
        &mut warnings,
    )?;
    let sidedefs = normalize_doom64_sidedefs(
        &raw.sidedefs,
        sectors.len(),
        texture_names.as_ref(),
        s,
        &mut warnings,
    )?;
    let linedefs = normalize_doom64_linedefs(
        &raw.linedefs,
        vertices.len(),
        sidedefs.len(),
        s,
        &mut warnings,
    )?;
    let things = normalize_doom64_things(&raw.things, s, &mut warnings)?;

    // Doom 64 BSP records share the classic on-disk layout (ADR-0018), so
    // they normalize through the same shared path. The classic path's
    // extended-encoding gate is deliberately not replicated here: no Doom 64
    // toolchain emits ZDBSP/GL encodings into nested-WAD sub-lumps (the gate
    // exists for classic PWADs, where they are common), and a hypothetical
    // blob is still handled safely — `read_doom64_map` either rejects it
    // (`Records`/`TrailingBytes` when its length is not a whole multiple of
    // the record size) or decodes it into garbage records whose dangling
    // references the resolvers below then bound (strict error / lenient
    // clamp-or-degrade). Either way: no panic, no unbounded work. Real
    // support for extended encodings, anywhere, is #199.
    let (segs, mut subsectors, nodes) = normalize_bsp_or_degrade(
        &raw.segs,
        &raw.subsectors,
        &raw.nodes,
        vertices.len(),
        linedefs.len(),
        s,
        &mut warnings,
    )?;

    let (leaf_arena, leaf_ranges) = normalize_leafs(
        &raw.leafs,
        subsectors.len(),
        vertices.len(),
        segs.len(),
        s,
        &mut warnings,
    )?;
    // Empty ranges = no leaves (or a lenient degrade); subsectors keep 0..0.
    if !leaf_ranges.is_empty() {
        for (subsector, range) in subsectors.iter_mut().zip(leaf_ranges) {
            subsector.leafs = range;
        }
    }

    let macros = normalize_macros(&raw.macros, s, &mut warnings)?;

    // Doom64Warning values surface as MapWarning::Doom64 so the caller sees
    // one warning stream regardless of source format.
    warnings.extend(raw.warnings().iter().cloned().map(MapWarning::Doom64));

    // The nested-WAD reader hands back empty `Vec`s when the REJECT/BLOCKMAP
    // sub-lumps are missing, and empty means "not built" here just as it
    // does for the classic/UDMF paths (ADR-0019 §4).
    let (reject, blockmap) = decode_reject_blockmap(
        Some(&raw.reject),
        Some(&raw.blockmap),
        sectors.len(),
        linedefs.len(),
        s,
        &mut warnings,
    )?;

    Ok(Map {
        name: group.name.clone(),
        format: MapFormat::Doom64,
        namespace: None,
        vertices,
        linedefs,
        sidedefs,
        sectors,
        things,
        lights,
        segs,
        subsectors,
        nodes,
        gl_vertices: Vec::new(),
        gl_segs: Vec::new(),
        gl_subsectors: Vec::new(),
        gl_nodes: Vec::new(),
        leafs: leaf_arena,
        macros,
        reject,
        blockmap,
        warnings,
    })
}

/// Assembles a UDMF (`TEXTMAP`) map group into a [`Map`] (ADR-0017 §3).
#[allow(clippy::too_many_lines)]
fn assemble_udmf(
    wad: &Wad,
    group: &MapGroup,
    options: ParseOptions,
    mut warnings: Vec<MapWarning>,
) -> Result<Map, MapAssembleError> {
    let s = options.strictness;

    if !crate::map::group::group_has_lump(wad, group, "ENDMAP") {
        match s {
            Strictness::Strict => {
                return Err(MapAssembleError::UnterminatedUdmf {
                    name: group.name.clone(),
                });
            }
            Strictness::Lenient => warnings.push(MapWarning::UnterminatedUdmf {
                name: group.name.clone(),
            }),
        }
    }

    let bytes = lump_bytes(wad, group, "TEXTMAP")
        .ok_or(MapAssembleError::MissingLump { lump: "TEXTMAP" })?;
    let text = crate::map::udmf::decode_textmap(bytes)
        .map_err(|source| MapAssembleError::Udmf { source })?;
    let udmf = crate::map::udmf::parse_udmf(text, options.limits)
        .map_err(|source| MapAssembleError::Udmf { source })?;

    let mut vertices = normalize_udmf_vertices(&udmf.vertices);
    let sectors = normalize_udmf_sectors(&udmf.sectors);
    let sidedefs = normalize_udmf_sidedefs(&udmf.sidedefs, sectors.len(), s, &mut warnings)?;
    let linedefs = normalize_udmf_linedefs(
        &udmf.linedefs,
        vertices.len(),
        sidedefs.len(),
        s,
        &mut warnings,
    )?;
    let things = normalize_udmf_things(&udmf.things, s, &mut warnings)?;

    // UDMF BSP data lives in a `ZNODES` lump carrying an extended-node stream.
    // An uncompressed `X*` dialect decodes in place (ADR-0025, #326); a still-gated
    // `Z*` twin (#327) applies the same extended-encoding gate the binary path uses.
    let (segs, subsectors, nodes) = if let Some(bytes) = lump_bytes(wad, group, "ZNODES") {
        // Preserve whatever prefix is present (zero-padded) so a truncated
        // `ZNODES` lump reports the actual bytes in the gate error rather than
        // an all-zero signature.
        let mut signature = [0u8; 4];
        let head = &bytes[..bytes.len().min(4)];
        signature[..head.len()].copy_from_slice(head);
        match classify_signature(signature) {
            Some((kind, NodeCompression::Uncompressed)) => {
                let decoded =
                    decode_extended_nodes(bytes, kind, &vertices, &linedefs, s, &mut warnings)?;
                vertices.extend(decoded.new_vertices);
                (decoded.segs, decoded.subsectors, decoded.nodes)
            }
            // A zlib-compressed `Z*` stream inflates to its uncompressed twin's body
            // and decodes through the same parser, bounded by
            // `Limits::max_decoded_node_bytes` (ADR-0025 §5, #327).
            #[cfg(feature = "extended-nodes-zlib")]
            Some((kind, NodeCompression::Zlib)) => {
                let decoded = decode_compressed_extended_nodes(
                    bytes,
                    kind,
                    &vertices,
                    &linedefs,
                    options.limits.max_decoded_node_bytes,
                    s,
                    &mut warnings,
                )?;
                vertices.extend(decoded.new_vertices);
                (decoded.segs, decoded.subsectors, decoded.nodes)
            }
            // A `Z*` stream without the `extended-nodes-zlib` feature, and any
            // unrecognized signature, keep #199's extended-encoding gate.
            _ => match s {
                Strictness::Strict => {
                    return Err(MapAssembleError::UnsupportedNodeEncoding {
                        lump: "ZNODES",
                        signature,
                    });
                }
                Strictness::Lenient => {
                    warnings.push(MapWarning::UnsupportedNodeEncoding { lump: "ZNODES" });
                    (Vec::new(), Vec::new(), Vec::new())
                }
            },
        }
    } else {
        (Vec::new(), Vec::new(), Vec::new())
    };

    let (reject, blockmap) = decode_reject_blockmap(
        lump_bytes(wad, group, "REJECT"),
        lump_bytes(wad, group, "BLOCKMAP"),
        sectors.len(),
        linedefs.len(),
        s,
        &mut warnings,
    )?;

    Ok(Map {
        name: group.name.clone(),
        format: MapFormat::Udmf,
        namespace: Some(udmf.namespace),
        vertices,
        linedefs,
        sidedefs,
        sectors,
        things,
        lights: vec![],
        segs,
        subsectors,
        nodes,
        gl_vertices: Vec::new(),
        gl_segs: Vec::new(),
        gl_subsectors: Vec::new(),
        gl_nodes: Vec::new(),
        leafs: Vec::new(),
        macros: Vec::new(),
        reject,
        blockmap,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        Map, MapAssembleError, normalize_doom64_thing_flags, normalize_hexen_thing_flags,
        normalize_udmf_linedefs, normalize_udmf_sidedefs, normalize_udmf_things,
        normalize_udmf_vertices, resolve_binary_side, resolve_optional, resolve_required,
    };
    use crate::map::graph::{MapFormat, SidedefIdx, VertexIdx};
    use crate::map::udmf::{UdmfLinedef, UdmfSidedef, UdmfThing};
    use crate::{ParseOptions, Strictness, map::MapWarning};
    use proptest::prelude::*;

    fn encode_i32(value: usize) -> [u8; 4] {
        i32::try_from(value)
            .expect("test fixture values should fit within i32")
            .to_le_bytes()
    }

    /// Builds minimal PWAD bytes from `(name, data)` lump pairs, mirroring the
    /// on-disk layout used by `tests/common/mod.rs::build_wad` and
    /// `group.rs`'s test helper of the same name: a 12-byte header (`PWAD`,
    /// lump count, directory offset), lump payloads, then 16-byte directory
    /// entries (`filepos`, `size`, 8-byte name).
    fn build_pwad(lumps: &[(&str, &[u8])]) -> Vec<u8> {
        let mut payload = Vec::new();
        let mut directory = Vec::new();
        let directory_offset = 12 + lumps.iter().map(|(_, bytes)| bytes.len()).sum::<usize>();

        for (name, bytes) in lumps {
            let filepos = 12 + payload.len();
            payload.extend_from_slice(bytes);
            directory.extend_from_slice(&encode_i32(filepos));
            directory.extend_from_slice(&encode_i32(bytes.len()));
            let mut encoded = [0_u8; 8];
            for (slot, byte) in name.as_bytes().iter().take(8).enumerate() {
                encoded[slot] = *byte;
            }
            directory.extend_from_slice(&encoded);
        }

        let mut wad = Vec::new();
        wad.extend_from_slice(b"PWAD");
        wad.extend_from_slice(&encode_i32(lumps.len()));
        wad.extend_from_slice(&encode_i32(directory_offset));
        wad.extend_from_slice(&payload);
        wad.extend_from_slice(&directory);
        wad
    }

    #[test]
    fn assembles_a_minimal_udmf_map() {
        let text = concat!(
            "namespace = \"doom\";\n",
            "vertex { x = 0.0; y = 0.0; }\n",
            "vertex { x = 64.0; y = 0.0; }\n",
            "linedef { v1 = 0; v2 = 1; sidefront = 0; }\n",
            "sidedef { sector = 0; }\n",
            "sector { texturefloor = \"F\"; textureceiling = \"C\"; }\n",
            "thing { x = 0.0; y = 0.0; type = 1; }\n",
        );
        let wad = crate::Wad::from_bytes(build_pwad(&[
            ("MAP01", b"" as &[u8]),
            ("TEXTMAP", text.as_bytes()),
            ("ENDMAP", b""),
        ]))
        .unwrap();
        let g = crate::map::group::map_group(&wad, "MAP01").unwrap();
        assert_eq!(crate::map::detect_map_format(&wad, &g), MapFormat::Udmf);
        let map = Map::assemble_with_options(&wad, &g, ParseOptions::default()).unwrap();
        assert_eq!(map.namespace(), Some("doom"));
        assert_eq!(map.format(), MapFormat::Udmf);
        assert_eq!(map.vertices().len(), 2);
        assert_eq!(map.linedefs().len(), 1);
        assert_eq!(map.linedefs()[0].left, None);
    }

    #[test]
    fn resolve_required_negative_index_is_out_of_range() {
        let mut warnings = Vec::new();
        // Strict: a negative (UDMF-style) index is a dangling reference.
        assert!(
            resolve_required(
                -1,
                4,
                "vertex",
                "linedef",
                Strictness::Strict,
                &mut warnings
            )
            .is_err()
        );
        assert!(warnings.is_empty());
        // Lenient: clamps to 0 and records a warning.
        let idx = resolve_required(
            -1,
            4,
            "vertex",
            "linedef",
            Strictness::Lenient,
            &mut warnings,
        )
        .expect("lenient recovers");
        assert_eq!(idx, 0);
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn resolve_binary_side_sentinel_and_negative() {
        let mut warnings = Vec::new();
        // The binary 0xffff one-sided sentinel resolves to `None`.
        assert_eq!(
            resolve_binary_side(0xffff, 4, "linedef", Strictness::Strict, &mut warnings).unwrap(),
            None
        );
        assert!(warnings.is_empty());
        // A negative index (not the sentinel) is out of range → lenient `None` + warning.
        assert_eq!(
            resolve_binary_side(-2, 4, "linedef", Strictness::Lenient, &mut warnings).unwrap(),
            None
        );
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn resolve_optional_has_no_binary_sentinel() {
        let mut w = Vec::new();
        // 65535 is a VALID index when count is large enough (no 0xffff sentinel).
        assert_eq!(
            resolve_optional(0xffff, 70000, "linedef", Strictness::Strict, &mut w).unwrap(),
            Some(0xffff)
        );
        assert!(w.is_empty());
        // Out of range -> lenient None + warning.
        assert_eq!(
            resolve_optional(5, 4, "linedef", Strictness::Lenient, &mut w).unwrap(),
            None
        );
        assert_eq!(w.len(), 1);
    }

    #[test]
    fn normalize_udmf_linedef_sideback_none_and_valid_65535() {
        let mut w = Vec::new();
        // sideback None -> left None; a valid Some(1) with 2 sidedefs -> Some(1).
        let lines = [UdmfLinedef {
            v1: 0,
            v2: 1,
            sidefront: 0,
            sideback: Some(1),
            id: 7,
            special: 80,
            args: [1, 2, 0, 0, 0],
            flags: 0b101,
            extras: Vec::new(),
        }];
        let out = normalize_udmf_linedefs(&lines, 2, 2, Strictness::Strict, &mut w).unwrap();
        assert_eq!(out[0].start, VertexIdx(0));
        assert_eq!(out[0].end, VertexIdx(1));
        assert_eq!(out[0].right, Some(SidedefIdx(0)));
        assert_eq!(out[0].left, Some(SidedefIdx(1)));
        assert_eq!(out[0].id, 7);
        assert_eq!(out[0].flags, 0b101);
        assert_eq!(out[0].special.special, 80);
        assert_eq!(out[0].special.args, [1, 2, 0, 0, 0]);

        let one_sided = [UdmfLinedef {
            v1: 0,
            v2: 1,
            sidefront: 0,
            sideback: None,
            id: -1,
            special: 0,
            args: [0; 5],
            flags: 0,
            extras: Vec::new(),
        }];
        let out2 = normalize_udmf_linedefs(&one_sided, 2, 2, Strictness::Strict, &mut w).unwrap();
        assert_eq!(out2[0].left, None);
    }

    #[test]
    fn normalize_udmf_thing_narrows_type_and_wraps_angle() {
        let mut w = Vec::new();
        let things = [UdmfThing {
            x: 1.0,
            y: 2.0,
            height: 3.0,
            angle: 450,
            type_id: 1,
            id: 5,
            special: 0,
            args: [0; 5],
            flags: 0,
            extras: Vec::new(),
        }];
        let out = normalize_udmf_things(&things, Strictness::Strict, &mut w).unwrap();
        assert_eq!((out[0].x, out[0].y, out[0].height), (1.0, 2.0, 3.0));
        assert_eq!(out[0].angle, 90); // 450 rem_euclid 360
        assert_eq!(out[0].type_id, 1);
        assert_eq!(out[0].id, 5);
        assert_eq!(out[0].flags, 0);
    }

    #[test]
    fn thing_type_overflow_strict_errors_lenient_clamps() {
        let mut w = Vec::new();
        let bad = [UdmfThing {
            x: 0.0,
            y: 0.0,
            height: 0.0,
            angle: 0,
            type_id: 70000,
            id: 0,
            special: 0,
            args: [0; 5],
            flags: 0,
            extras: Vec::new(),
        }];
        assert!(normalize_udmf_things(&bad, Strictness::Strict, &mut w).is_err());
        let out = normalize_udmf_things(&bad, Strictness::Lenient, &mut w).unwrap();
        assert_eq!(out[0].type_id, u16::MAX);
        assert!(
            w.iter()
                .any(|x| matches!(x, MapWarning::FieldOutOfRange { .. }))
        );
    }

    #[test]
    fn normalize_udmf_sidedef_dangling_sector_strict_errors() {
        // Strict-mode error propagation on a UDMF sidedef's out-of-range sector.
        let mut w = Vec::new();
        let sides = [UdmfSidedef {
            offsetx: 0,
            offsety: 0,
            texturetop: "-".to_owned(),
            texturebottom: "-".to_owned(),
            texturemiddle: "-".to_owned(),
            sector: 99,
            extras: Vec::new(),
        }];
        let err = normalize_udmf_sidedefs(&sides, 1, Strictness::Strict, &mut w).unwrap_err();
        assert!(matches!(err, MapAssembleError::DanglingReference { .. }));
    }

    #[test]
    fn normalize_udmf_linedef_dangling_end_vertex_strict_errors() {
        // Strict-mode error on the second (end/`v2`) vertex reference — `v1` is
        // valid so resolution reaches `v2`.
        let mut w = Vec::new();
        let lines = [UdmfLinedef {
            v1: 0,
            v2: 99,
            sidefront: 0,
            sideback: None,
            id: 0,
            special: 0,
            args: [0; 5],
            flags: 0,
            extras: Vec::new(),
        }];
        let err = normalize_udmf_linedefs(&lines, 2, 1, Strictness::Strict, &mut w).unwrap_err();
        assert!(matches!(err, MapAssembleError::DanglingReference { .. }));
    }

    #[test]
    fn normalize_udmf_linedef_dangling_sidefront_strict_errors() {
        // Strict-mode error on the `sidefront` (right sidedef) reference — the
        // vertices resolve, so resolution reaches `sidefront`.
        let mut w = Vec::new();
        let lines = [UdmfLinedef {
            v1: 0,
            v2: 1,
            sidefront: 99,
            sideback: None,
            id: 0,
            special: 0,
            args: [0; 5],
            flags: 0,
            extras: Vec::new(),
        }];
        let err = normalize_udmf_linedefs(&lines, 2, 1, Strictness::Strict, &mut w).unwrap_err();
        assert!(matches!(err, MapAssembleError::DanglingReference { .. }));
    }

    /// A Hexen thing present in all three game modes (`0x0100` single |
    /// `0x0200` co-op | `0x0400` deathmatch) must normalize to Doom's *negative*
    /// bits 4/5/6 all **clear** — the graph says "not excluded from any mode".
    #[test]
    fn hexen_thing_in_all_game_modes_clears_the_negative_bits() {
        let normalized = normalize_hexen_thing_flags(0x0100 | 0x0200 | 0x0400);
        assert_eq!(normalized & 0x0070, 0, "bits 4/5/6 must all be clear");
        assert_eq!(normalized, 0x0000);
    }

    /// The converse: a Hexen thing naming no game mode appears nowhere, which in
    /// Doom's negative encoding is bits 4/5/6 all **set**.
    #[test]
    fn hexen_thing_in_no_game_mode_sets_the_negative_bits() {
        let normalized = normalize_hexen_thing_flags(0x0000);
        assert_eq!(normalized & 0x0070, 0x0070, "bits 4/5/6 must all be set");
        assert_eq!(normalized, 0x0070);
    }

    /// Each Hexen game-mode bit maps to its own Doom bit, inverted. Note the
    /// crossover: Hexen orders the bits single/co-op/deathmatch, Doom orders
    /// them single/deathmatch/co-op, so co-op and deathmatch swap positions.
    #[test]
    fn hexen_game_mode_bits_invert_into_their_doom_positions() {
        // Single-player only: DM (bit 5) and co-op (bit 6) excluded, SP not.
        assert_eq!(normalize_hexen_thing_flags(0x0100), 0x0060);
        // Co-op only (Hexen 0x0200) -> Doom bit 6 clear, bits 4 and 5 set.
        assert_eq!(normalize_hexen_thing_flags(0x0200), 0x0030);
        // Deathmatch only (Hexen 0x0400) -> Doom bit 5 clear, bits 4 and 6 set.
        assert_eq!(normalize_hexen_thing_flags(0x0400), 0x0050);
    }

    /// Skills (bits 0–2) and ambush (bit 3) share Doom's meaning *and* position,
    /// so they survive verbatim.
    #[test]
    fn hexen_skill_and_ambush_bits_are_preserved() {
        // All skills + ambush, no game modes: low nibble kept, bits 4/5/6 set.
        assert_eq!(normalize_hexen_thing_flags(0x000F), 0x007F);
        // Skill 3 only, in every game mode.
        assert_eq!(normalize_hexen_thing_flags(0x0002 | 0x0700), 0x0002);
    }

    /// `dormant` and the fighter/cleric/mage class filters have no Doom bit and
    /// are dropped — crucially, they must not leak into Doom's bits 4–7, which
    /// they collide with on disk.
    #[test]
    fn hexen_dormant_and_class_bits_are_dropped() {
        // dormant | fighter | cleric | mage, in all three game modes: every one
        // of those bits is unmappable, so nothing but 0 survives.
        let raw = 0x0010 | 0x0020 | 0x0040 | 0x0080 | 0x0100 | 0x0200 | 0x0400;
        assert_eq!(normalize_hexen_thing_flags(raw), 0x0000);
        // Bit 7 (friend, MBF) has no Hexen source and is never set — not even by
        // Hexen's `mage` bit, which occupies that same on-disk position.
        assert_eq!(normalize_hexen_thing_flags(0x0080) & 0x0080, 0);
    }

    #[test]
    fn doom64_thing_flags_translate_to_the_normalized_layout() {
        // Verified against Doom64 EX doomdef.h (ADR-0021 §2): EASY/NORMAL/HARD/
        // AMBUSH/MULTI (1/2/4/8/16) are value-identical to normalized bits 0-4;
        // NODEATHMATCH (1024) -> bit 5; NONETGAME (2048) -> bit 6 (co-op);
        // SPAWN/ONTOUCH/ONDEATH/SECRET/NOINFIGHTING/NIGHTMARE drop.
        assert_eq!(normalize_doom64_thing_flags(0), 0);
        assert_eq!(normalize_doom64_thing_flags(1 | 2 | 4), 0b111);
        assert_eq!(normalize_doom64_thing_flags(8), 0b1000);
        assert_eq!(normalize_doom64_thing_flags(16), 0b1_0000);
        assert_eq!(normalize_doom64_thing_flags(1024), 0b10_0000);
        assert_eq!(normalize_doom64_thing_flags(2048), 0b100_0000);
        // Doom 64-only bits drop; friend (bit 7) is never set.
        assert_eq!(
            normalize_doom64_thing_flags(32 | 64 | 128 | 256 | 512 | 4096),
            0
        );
    }

    proptest! {
        // Arbitrary UTF-8 text, wrapped as a TEXTMAP: whenever it happens to
        // parse as UDMF, normalization must neither panic nor create more
        // vertices than could possibly have been parsed from the input — the
        // O(input) allocation invariant (ADR-0016 item 1) applied to the UDMF
        // assembly surface.
        #[test]
        fn udmf_assembly_never_panics_and_is_bounded(text in ".*") {
            if let Ok(map) = crate::map::udmf::parse_udmf(&text, crate::Limits::default()) {
                // Normalizing cannot create more elements than were parsed.
                let mut w = Vec::new();
                let v = normalize_udmf_vertices(&map.vertices);
                prop_assert!(v.len() <= text.len());
                let _ = normalize_udmf_things(&map.things, Strictness::Lenient, &mut w);
            }
        }
    }
}
