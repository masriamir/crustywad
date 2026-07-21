//! Decoding uncompressed ZDoom extended BSP node streams (ADR-0025, #326).
//!
//! ZDoom's node builder can emit the `SEGS`/`SSECTORS`/`NODES` data as a single
//! self-describing blob in the `NODES` lump instead of the three classic
//! fixed-size record lumps. This module decodes the four **uncompressed**
//! dialects — `XNOD` (the non-GL extended layout), and the GL layouts `XGLN`,
//! `XGL2`, and `XGL3` — into the graph's [`MapSeg`]/[`MapSubsector`]/[`MapNode`]
//! arenas. The zlib-wrapped `Z*` twins (#327) and DeePBSP `xNd4` (#328) are out
//! of scope here.
//!
//! The byte layout is source-verified against the gzdoom loader
//! (`P_LoadZNodes`/`LoadZSegs`/`LoadGLZSegs`) and the zdbsp writer. Framing,
//! after the 4-byte ASCII tag: a vertex header (original + node-builder-added
//! vertices), a subsector block (per-subsector seg counts, assigned as
//! consecutive runs), a seg block, then a node block. All fields are
//! little-endian.
//!
//! Neither the seg `angle` nor its `offset` is stored on disk for the extended
//! formats; both are derived from geometry (ADR-0025) — the angle from the
//! endpoints' `atan2`, encoded as a 32-bit binary angle then narrowed to the
//! graph's 16-bit BAM, and the offset as the Euclidean distance from the seg's
//! start vertex to the appropriate endpoint of its backing linedef.
//!
//! Hardening (ADR-0016): every stream count is validated against the remaining
//! byte budget before any allocation, so a hostile header cannot force a large
//! reservation; decoding is a single forward pass with no recursion; and both
//! [`Strictness`] modes return without panicking on any input — strict fails
//! with a [`MapAssembleError`], lenient degrades the whole BSP to empty arenas
//! (or clamps an individual reference) and records a [`MapWarning`].

use crate::Strictness;
use crate::map::assemble::MapAssembleError;
use crate::map::graph::{
    LinedefIdx, MapLinedef, MapNode, MapSeg, MapSubsector, MapVertex, MapWarning, NodeChild,
    NodeIdx, SubsectorIdx, VertexIdx,
};

/// A structural fault in an uncompressed extended-node stream — a defect in the
/// stream's own framing, as opposed to an out-of-range cross-reference (which
/// reuses [`MapAssembleError::DanglingReference`], the classic path's
/// vocabulary).
///
/// Carried by [`MapAssembleError::ExtendedNode`] (strict mode) and
/// [`MapWarning::ExtendedNode`] (lenient recovery).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ExtendedNodeError {
    /// The stream ended in the middle of a fixed-size field or record.
    #[error("truncated extended node stream while reading the {section}")]
    Truncated {
        /// The part of the stream being read when it ran out (e.g.
        /// `"vertex header"`, `"seg"`, `"node"`).
        section: &'static str,
    },
    /// A stream count multiplied by its record size overflowed, or exceeded the
    /// number of bytes remaining in the lump — a hostile or corrupt count that
    /// would demand more data (and allocation) than the lump can hold.
    #[error("extended node {section} count exceeds the remaining stream length")]
    CountOverflow {
        /// The block whose count was rejected (e.g. `"vertex"`, `"subsector"`,
        /// `"seg"`, `"node"`).
        section: &'static str,
    },
    /// The subsectors' seg counts did not sum to the seg block's own count — a
    /// hard consistency invariant (gzdoom `P_LoadZNodes` fatal-errors on it).
    #[error("subsector seg total {seg_total} does not match the seg count {num_segs}")]
    SegCountMismatch {
        /// The sum of every subsector's `segCount`.
        seg_total: u64,
        /// The seg block's declared `numSegs`.
        num_segs: u64,
    },
    /// The vertex header's `origVerts` did not match the map's actual vertex
    /// count. Larger than the map's count is unusable (gzdoom rejects it);
    /// smaller is a recoverable mismatch (lenient proceeds using the map's
    /// count as the split base between existing and node-builder vertices).
    #[error(
        "vertex header origVerts {orig_verts} does not match the map's {existing} existing vertices"
    )]
    VertexHeaderMismatch {
        /// The stream's declared original-vertex count.
        orig_verts: usize,
        /// The map's actual `VERTEXES`-derived vertex count.
        existing: usize,
    },
    /// A GL seg's partner-seg index pointed outside the seg arena. The partner
    /// link is a GL-render detail not carried into the graph; it is validated
    /// only (gzdoom `LoadGLZSegs` fatal-errors on it).
    #[error("partner seg index {partner} on seg {seg} is out of range ({num_segs} segs)")]
    PartnerOutOfRange {
        /// The 0-based index of the seg carrying the bad partner link.
        seg: usize,
        /// The out-of-range partner index.
        partner: u32,
        /// The seg arena length.
        num_segs: usize,
    },
}

/// Which uncompressed extended-node dialect a stream is, selecting the seg and
/// node record layouts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExtendedNodeKind {
    /// `XNOD`: the non-GL extended layout — explicit seg `v2`, no partner, a
    /// 16-bit linedef on every seg (no minisegs), and 16-bit node partitions.
    Xnod,
    /// `XGLN`: GL layout with an implicit `v2`, a 32-bit partner, a 16-bit
    /// linedef (`0xFFFF` = miniseg), and 16-bit node partitions.
    Xgln,
    /// `XGL2`: like `XGLN` but with a 32-bit linedef (`0xFFFFFFFF` = miniseg).
    Xgl2,
    /// `XGL3`: like `XGL2` for segs, but node partitions are 32-bit 16.16
    /// fixed-point.
    Xgl3,
}

impl ExtendedNodeKind {
    /// Maps a 4-byte lump-head signature to the decodable dialect it names, or
    /// `None` when the signature is recognized but this build cannot decode it
    /// (the zlib-wrapped `Z*` twins, #327) or is not an extended-node signature
    /// at all. `DeePBSP`'s `xNd4` falls into the latter case: it is not yet
    /// detected as an extended encoding by the caller at all (#328), so this
    /// function never even sees it gated — it never reaches this match.
    /// `Some` means "decode with this kind"; `None` means "keep the
    /// extended-encoding gate".
    pub(crate) fn from_signature(sig: [u8; 4]) -> Option<ExtendedNodeKind> {
        match &sig {
            b"XNOD" => Some(ExtendedNodeKind::Xnod),
            b"XGLN" => Some(ExtendedNodeKind::Xgln),
            b"XGL2" => Some(ExtendedNodeKind::Xgl2),
            b"XGL3" => Some(ExtendedNodeKind::Xgl3),
            _ => None,
        }
    }

    /// The 4-byte ASCII tag naming this dialect, for diagnostics.
    fn lump_name(self) -> &'static str {
        match self {
            ExtendedNodeKind::Xnod => "XNOD",
            ExtendedNodeKind::Xgln => "XGLN",
            ExtendedNodeKind::Xgl2 => "XGL2",
            ExtendedNodeKind::Xgl3 => "XGL3",
        }
    }

    /// Whether this is a GL dialect (implicit `v2`, partner links, minisegs).
    fn is_gl(self) -> bool {
        !matches!(self, ExtendedNodeKind::Xnod)
    }

    /// Whether node partitions are 32-bit 16.16 fixed-point (`XGL3` only).
    fn is_xgl3(self) -> bool {
        matches!(self, ExtendedNodeKind::Xgl3)
    }

    /// The on-disk byte size of one seg record in this dialect: `XNOD`
    /// `u32 v1, u32 v2, u16 line, u8 side` = 11; `XGLN`
    /// `u32 v1, u32 partner, u16 line, u8 side` = 11; `XGL2`/`XGL3`
    /// `u32 v1, u32 partner, u32 line, u8 side` = 13.
    fn seg_size(self) -> usize {
        match self {
            ExtendedNodeKind::Xnod | ExtendedNodeKind::Xgln => 11,
            ExtendedNodeKind::Xgl2 | ExtendedNodeKind::Xgl3 => 13,
        }
    }

    /// The on-disk byte size of one node record: 40 for `XGL3` (`i32` partition
    /// ×4 + `i16` bbox ×8 + `u32` children ×2), 32 for the others (`i16`
    /// partition ×4 + `i16` bbox ×8 + `u32` children ×2).
    fn node_size(self) -> usize {
        if self.is_xgl3() { 40 } else { 32 }
    }
}

/// The decoded BSP arenas from an uncompressed extended-node stream, ready to be
/// spliced into an assembling [`Map`](crate::map::Map).
///
/// `new_vertices` are the node-builder-added vertices; the caller appends them
/// after the `VERTEXES`-derived vertices (the seg endpoint indices in `segs` are
/// already resolved against the combined arena).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DecodedExtendedBsp {
    /// Node-builder-added vertices, to append after the existing vertex arena.
    pub(crate) new_vertices: Vec<MapVertex>,
    /// The decoded segs, with derived `angle`/`offset` and resolved endpoints.
    pub(crate) segs: Vec<MapSeg>,
    /// The decoded subsectors (consecutive seg runs).
    pub(crate) subsectors: Vec<MapSubsector>,
    /// The decoded BSP nodes; the root is the last, per the crate convention.
    pub(crate) nodes: Vec<MapNode>,
}

impl DecodedExtendedBsp {
    /// The empty result — every arena cleared — produced by a lenient whole-BSP
    /// degrade.
    fn empty() -> Self {
        Self {
            new_vertices: Vec::new(),
            segs: Vec::new(),
            subsectors: Vec::new(),
            nodes: Vec::new(),
        }
    }
}

/// A forward, bounds-checked little-endian reader over the stream bytes. Every
/// read returns `None` at end-of-input rather than panicking.
struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    /// A reader positioned at `pos` (past the already-consumed 4-byte tag).
    fn new(bytes: &'a [u8], pos: usize) -> Self {
        Self { bytes, pos }
    }

    /// Bytes not yet consumed. `pos` never exceeds `bytes.len()` (reads only
    /// advance on success), so this never underflows.
    fn remaining(&self) -> usize {
        self.bytes.len() - self.pos
    }

    /// Reads a slice of `n` bytes, advancing past them, or `None` at EOF.
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let end = self.pos.checked_add(n)?;
        let slice = self.bytes.get(self.pos..end)?;
        self.pos = end;
        Some(slice)
    }

    /// Reads a little-endian `u32`.
    fn u32(&mut self) -> Option<u32> {
        self.take(4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// Reads a little-endian `i32`.
    fn i32(&mut self) -> Option<i32> {
        self.take(4)
            .map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// Reads a little-endian `u16`.
    fn u16(&mut self) -> Option<u16> {
        self.take(2).map(|b| u16::from_le_bytes([b[0], b[1]]))
    }

    /// Reads a little-endian `i16` widened to `i32` (the graph's coordinate
    /// width).
    fn i16_as_i32(&mut self) -> Option<i32> {
        self.take(2)
            .map(|b| i32::from(i16::from_le_bytes([b[0], b[1]])))
    }

    /// Reads a `u8`.
    fn u8(&mut self) -> Option<u8> {
        self.take(1).map(|b| b[0])
    }
}

/// Whether `count` records of `record_size` bytes fit within `remaining` bytes,
/// with no arithmetic overflow — the ADR-0016 bounded-allocation guard, applied
/// before any `Vec::with_capacity` from an untrusted stream count.
fn fits(count: u32, record_size: usize, remaining: usize) -> bool {
    u64::from(count)
        .checked_mul(record_size as u64)
        .is_some_and(|need| need <= remaining as u64)
}

/// Resolves an arena reference, mirroring the assembler's `resolve_required`:
/// in range yields the index; an empty target arena is always fatal (nothing to
/// clamp to); otherwise strict errors and lenient clamps to `0` with a
/// [`MapWarning::DanglingReference`]. The raw index is preserved in the
/// diagnostic (saturated into `i32` for the shared warning field).
fn resolve_ref(
    index: usize,
    count: usize,
    referent: &'static str,
    from: &'static str,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<usize, MapAssembleError> {
    if index < count {
        return Ok(index);
    }
    let diag = i32::try_from(index).unwrap_or(i32::MAX);
    if count == 0 {
        return Err(MapAssembleError::DanglingReference {
            referent,
            index: diag,
            from,
            count: 0,
        });
    }
    match strictness {
        Strictness::Strict => Err(MapAssembleError::DanglingReference {
            referent,
            index: diag,
            from,
            count,
        }),
        Strictness::Lenient => {
            warnings.push(MapWarning::DanglingReference {
                referent,
                index: diag,
                from,
                count,
            });
            Ok(0)
        }
    }
}

/// Resolves one BSP node child, mirroring the assembler's `resolve_node_child`
/// but with the extended path's 32-bit leaf flag: bit 31 (`0x8000_0000`) set
/// selects a subsector leaf (remaining 31 bits into `subsector_count`), clear
/// selects an internal node (into `node_count`).
fn resolve_child(
    raw: u32,
    node_count: usize,
    subsector_count: usize,
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<NodeChild, MapAssembleError> {
    if raw & 0x8000_0000 == 0 {
        Ok(NodeChild::Node(NodeIdx(resolve_ref(
            raw as usize,
            node_count,
            "node",
            "node",
            strictness,
            warnings,
        )?)))
    } else {
        Ok(NodeChild::Subsector(SubsectorIdx(resolve_ref(
            (raw & 0x7FFF_FFFF) as usize,
            subsector_count,
            "subsector",
            "node",
            strictness,
            warnings,
        )?)))
    }
}

/// Encodes the angle of the vector `v1 -> v2` as the graph's 16-bit binary angle
/// (BAM): `atan2(dy, dx)` scaled to a full 32-bit turn, wrapped, then narrowed
/// to the high 16 bits. Due-east is `0x0000`, due-north `0x4000`, due-west
/// `0x8000`, due-south `0xC000`. Valid for minisegs (endpoints only). Coincident
/// endpoints give `atan2(0, 0) == 0` — a deterministic `0x0000`, never a panic.
fn derive_angle(v1: MapVertex, v2: MapVertex) -> u16 {
    let rad = (v2.y - v1.y).atan2(v2.x - v1.x);
    // Scale the [-PI, PI] angle to a signed 32-bit turn, then wrap into u32.
    // `as u32` on an f64 saturates (wrong for the negative half), so route the
    // value through `i64` first, which truncates modularly — the intended BAM
    // wrap. The scaled value is in (-2^31, 2^31], always exact in i64.
    let scaled = rad / (2.0 * std::f64::consts::PI) * 2f64.powi(32);
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_possible_wrap
    )]
    let bam = scaled as i64 as u32;
    #[allow(clippy::cast_possible_truncation)]
    let angle = (bam >> 16) as u16;
    angle
}

/// The seg's distance-along-linedef offset (ADR-0025): the Euclidean distance
/// from `start` (the seg's start vertex) to `reference` (the linedef's start
/// vertex for a front seg, its end vertex for a back seg), rounded to whole map
/// units. Minisegs have no backing linedef and use `0`.
fn derive_offset(start: MapVertex, reference: MapVertex) -> i32 {
    let dx = reference.x - start.x;
    let dy = reference.y - start.y;
    let dist = dx.mul_add(dx, dy * dy).sqrt();
    // Non-negative and, for any real map, well within i32; `round() as i32`
    // saturates rather than wrapping on a pathological coordinate.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let offset = dist.round() as i32;
    offset
}

/// A raw seg record, before endpoint resolution and `angle`/`offset` derivation.
struct RawSeg {
    /// The start vertex index into the combined vertex arena.
    v1: u32,
    /// The end vertex index — explicit for `XNOD`, filled from the subsector run
    /// for the GL dialects.
    v2: u32,
    /// The backing linedef index, or `None` for a GL miniseg.
    linedef: Option<u32>,
    /// The on-disk side bit (0 = front/right, 1 = back/left).
    side: u8,
}

/// Decodes an uncompressed extended-node stream into BSP arenas (ADR-0025).
///
/// `bytes` is the whole `NODES` lump **including** its 4-byte ASCII tag.
/// `existing_vertices` is the map's `VERTEXES`-derived vertex arena — it both
/// supplies the split base for seg vertex indices (indices below its length are
/// existing vertices, the rest are `new_vertices`) and the coordinates the
/// derived `angle`/`offset` need. `linedefs` is the map's assembled linedef
/// arena, used to bound seg linedef references and to locate the offset's
/// reference endpoint.
///
/// # Errors
///
/// In [`Strictness::Strict`], returns [`MapAssembleError::ExtendedNode`] for a
/// structural fault (truncation, an overflowing count, a subsector/seg-count
/// mismatch, a vertex-header mismatch, or a partner index out of range), and
/// [`MapAssembleError::DanglingReference`] for an out-of-range vertex, linedef,
/// or child reference. In [`Strictness::Lenient`], an individual out-of-range
/// reference is clamped with a warning; any other fault degrades the whole BSP
/// to empty arenas with a single [`MapWarning`] (mirroring the assembler's
/// `normalize_bsp_or_degrade`).
pub(crate) fn decode_extended_nodes(
    bytes: &[u8],
    kind: ExtendedNodeKind,
    existing_vertices: &[MapVertex],
    linedefs: &[MapLinedef],
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<DecodedExtendedBsp, MapAssembleError> {
    let watermark = warnings.len();
    match decode_inner(
        bytes,
        kind,
        existing_vertices,
        linedefs,
        strictness,
        warnings,
    ) {
        Ok(bsp) => Ok(bsp),
        Err(err) if strictness == Strictness::Lenient => {
            // Whole-BSP degrade: drop any per-element warnings describing the
            // arenas we are discarding, and surface exactly one warning for the
            // degrade (the same posture as the classic extended-encoding gate).
            warnings.truncate(watermark);
            warnings.push(degrade_warning(&err, kind.lump_name()));
            Ok(DecodedExtendedBsp::empty())
        }
        Err(err) => Err(err),
    }
}

/// Maps a fatal decode error to the single [`MapWarning`] recorded for a lenient
/// whole-BSP degrade.
fn degrade_warning(err: &MapAssembleError, lump: &'static str) -> MapWarning {
    match *err {
        MapAssembleError::DanglingReference {
            referent,
            index,
            from,
            count,
        } => MapWarning::DanglingReference {
            referent,
            index,
            from,
            count,
        },
        MapAssembleError::ExtendedNode { lump, reason } => {
            MapWarning::ExtendedNode { lump, reason }
        }
        // decode_inner only produces the two kinds above; keep the mapping total
        // without inventing a warning for an impossible variant.
        _ => MapWarning::ExtendedNode {
            lump,
            reason: ExtendedNodeError::Truncated { section: "stream" },
        },
    }
}

/// The strict-style decode: structural faults and empty-arena references return
/// `Err`; individual out-of-range references clamp with a warning in lenient
/// mode; the two "recover and continue" mismatches (a smaller-than-map
/// `origVerts`, a partner out of range) warn in lenient mode and proceed.
#[allow(clippy::too_many_lines)]
fn decode_inner(
    bytes: &[u8],
    kind: ExtendedNodeKind,
    existing_vertices: &[MapVertex],
    linedefs: &[MapLinedef],
    strictness: Strictness,
    warnings: &mut Vec<MapWarning>,
) -> Result<DecodedExtendedBsp, MapAssembleError> {
    let lump = kind.lump_name();
    let truncated = |section: &'static str| MapAssembleError::ExtendedNode {
        lump,
        reason: ExtendedNodeError::Truncated { section },
    };
    let overflow = |section: &'static str| MapAssembleError::ExtendedNode {
        lump,
        reason: ExtendedNodeError::CountOverflow { section },
    };

    // The 4-byte tag is part of the lump; skip it.
    if bytes.len() < 4 {
        return Err(truncated("tag"));
    }
    let mut reader = Reader::new(bytes, 4);

    // --- 1a. Vertex header ---
    let orig_verts = reader.u32().ok_or_else(|| truncated("vertex header"))?;
    let new_verts = reader.u32().ok_or_else(|| truncated("vertex header"))?;
    let existing_len = existing_vertices.len();
    let orig_verts_usize = orig_verts as usize;
    if orig_verts_usize > existing_len {
        // More original vertices than the map has: unusable (gzdoom rejects).
        return Err(MapAssembleError::ExtendedNode {
            lump,
            reason: ExtendedNodeError::VertexHeaderMismatch {
                orig_verts: orig_verts_usize,
                existing: existing_len,
            },
        });
    }
    if orig_verts_usize != existing_len {
        // Fewer than the map has: recoverable. Proceed using the map's count as
        // the split base between existing and node-builder vertices.
        let reason = ExtendedNodeError::VertexHeaderMismatch {
            orig_verts: orig_verts_usize,
            existing: existing_len,
        };
        match strictness {
            Strictness::Strict => return Err(MapAssembleError::ExtendedNode { lump, reason }),
            Strictness::Lenient => warnings.push(MapWarning::ExtendedNode { lump, reason }),
        }
    }
    if !fits(new_verts, 8, reader.remaining()) {
        return Err(overflow("vertex"));
    }
    let mut new_vertices = Vec::with_capacity(new_verts as usize);
    for _ in 0..new_verts {
        let x = reader.i32().ok_or_else(|| truncated("vertex"))?;
        let y = reader.i32().ok_or_else(|| truncated("vertex"))?;
        new_vertices.push(MapVertex {
            x: f64::from(x) / 65536.0,
            y: f64::from(y) / 65536.0,
        });
    }
    let combined_count = existing_len + new_vertices.len();

    // --- 1b. Subsector block ---
    let num_subs = reader.u32().ok_or_else(|| truncated("subsector block"))?;
    if !fits(num_subs, 4, reader.remaining()) {
        return Err(overflow("subsector"));
    }
    let mut seg_counts = Vec::with_capacity(num_subs as usize);
    let mut seg_total: u64 = 0;
    for _ in 0..num_subs {
        let c = reader.u32().ok_or_else(|| truncated("subsector"))?;
        seg_total += u64::from(c);
        seg_counts.push(c);
    }

    // --- 1c. Seg block ---
    let num_segs = reader.u32().ok_or_else(|| truncated("seg block"))?;
    if seg_total != u64::from(num_segs) {
        return Err(MapAssembleError::ExtendedNode {
            lump,
            reason: ExtendedNodeError::SegCountMismatch {
                seg_total,
                num_segs: u64::from(num_segs),
            },
        });
    }
    if !fits(num_segs, kind.seg_size(), reader.remaining()) {
        return Err(overflow("seg"));
    }
    let mut raw_segs = Vec::with_capacity(num_segs as usize);
    for i in 0..num_segs {
        let v1 = reader.u32().ok_or_else(|| truncated("seg"))?;
        let (v2, linedef) = if kind.is_gl() {
            let partner = reader.u32().ok_or_else(|| truncated("seg"))?;
            if partner != 0xFFFF_FFFF && partner as usize >= num_segs as usize {
                let reason = ExtendedNodeError::PartnerOutOfRange {
                    seg: i as usize,
                    partner,
                    num_segs: num_segs as usize,
                };
                match strictness {
                    Strictness::Strict => {
                        return Err(MapAssembleError::ExtendedNode { lump, reason });
                    }
                    Strictness::Lenient => warnings.push(MapWarning::ExtendedNode { lump, reason }),
                }
            }
            // v2 is implicit for GL; filled from the subsector run below.
            let linedef = if kind.is_xgl3() || matches!(kind, ExtendedNodeKind::Xgl2) {
                let line = reader.u32().ok_or_else(|| truncated("seg"))?;
                (line != 0xFFFF_FFFF).then_some(line)
            } else {
                // XGLN: 16-bit linedef, 0xFFFF = miniseg.
                let line = reader.u16().ok_or_else(|| truncated("seg"))?;
                (line != 0xFFFF).then_some(u32::from(line))
            };
            (0, linedef)
        } else {
            // XNOD: explicit v2, 16-bit linedef, no minisegs.
            let v2 = reader.u32().ok_or_else(|| truncated("seg"))?;
            let line = reader.u16().ok_or_else(|| truncated("seg"))?;
            (v2, Some(u32::from(line)))
        };
        let side = reader.u8().ok_or_else(|| truncated("seg"))?;
        raw_segs.push(RawSeg {
            v1,
            v2,
            linedef,
            side,
        });
    }

    // Fill each GL seg's implicit v2 from the next seg's v1 within its
    // subsector's consecutive run, wrapping the last seg back to the first.
    if kind.is_gl() {
        let mut cursor = 0usize;
        for &count in &seg_counts {
            let count = count as usize;
            for j in 0..count {
                let cur = cursor + j;
                let next = cursor + (j + 1) % count;
                raw_segs[cur].v2 = raw_segs[next].v1;
            }
            cursor += count;
        }
    }

    // Resolve endpoints, derive angle/offset, resolve linedefs.
    let vcoord = |idx: usize| -> MapVertex {
        if idx < existing_len {
            existing_vertices[idx]
        } else {
            new_vertices[idx - existing_len]
        }
    };
    let mut segs = Vec::with_capacity(raw_segs.len());
    for raw in &raw_segs {
        let start_idx = resolve_ref(
            raw.v1 as usize,
            combined_count,
            "vertex",
            "seg",
            strictness,
            warnings,
        )?;
        let end_idx = resolve_ref(
            raw.v2 as usize,
            combined_count,
            "vertex",
            "seg",
            strictness,
            warnings,
        )?;
        let start_coord = vcoord(start_idx);
        let end_coord = vcoord(end_idx);
        let angle = derive_angle(start_coord, end_coord);
        let linedef = match raw.linedef {
            None => None,
            Some(line) => Some(LinedefIdx(resolve_ref(
                line as usize,
                linedefs.len(),
                "linedef",
                "seg",
                strictness,
                warnings,
            )?)),
        };
        let offset = match linedef {
            None => 0,
            Some(idx) => {
                let ld = &linedefs[idx.0];
                let ref_vertex = if raw.side == 0 { ld.start } else { ld.end };
                // Linedef endpoints reference existing (original) vertices; a
                // pre-assembled linedef's indices are in range, but guard anyway
                // so no input can panic.
                let ref_coord = existing_vertices
                    .get(ref_vertex.0)
                    .copied()
                    .unwrap_or(start_coord);
                derive_offset(start_coord, ref_coord)
            }
        };
        segs.push(MapSeg {
            start: VertexIdx(start_idx),
            end: VertexIdx(end_idx),
            angle,
            linedef,
            // `direction` is a 0/1 flag (`MapSeg::direction`); the on-disk `side`
            // is a `u8`, so clamp any nonzero to 1 rather than propagate a
            // malformed value. Matches how `offset` above already treats `side`
            // (the `side == 0` reference-vertex choice).
            direction: u16::from(raw.side != 0),
            offset,
        });
    }

    // Subsectors: consecutive seg runs. `seg_total == num_segs == segs.len()`,
    // so every run lands inside the seg arena by construction.
    let mut subsectors = Vec::with_capacity(seg_counts.len());
    let mut cursor = 0usize;
    for &count in &seg_counts {
        let count = count as usize;
        subsectors.push(MapSubsector {
            segs: cursor..cursor + count,
            leafs: 0..0,
        });
        cursor += count;
    }

    // --- 1d. Node block ---
    let num_nodes = reader.u32().ok_or_else(|| truncated("node block"))?;
    if !fits(num_nodes, kind.node_size(), reader.remaining()) {
        return Err(overflow("node"));
    }
    let node_count = num_nodes as usize;
    let subsector_count = subsectors.len();
    let mut nodes = Vec::with_capacity(node_count);
    for _ in 0..num_nodes {
        let (x, y, dx, dy) = if kind.is_xgl3() {
            // 16.16 fixed-point partition -> whole map units (arithmetic shift).
            let x = reader.i32().ok_or_else(|| truncated("node"))? >> 16;
            let y = reader.i32().ok_or_else(|| truncated("node"))? >> 16;
            let dx = reader.i32().ok_or_else(|| truncated("node"))? >> 16;
            let dy = reader.i32().ok_or_else(|| truncated("node"))? >> 16;
            (x, y, dx, dy)
        } else {
            let x = reader.i16_as_i32().ok_or_else(|| truncated("node"))?;
            let y = reader.i16_as_i32().ok_or_else(|| truncated("node"))?;
            let dx = reader.i16_as_i32().ok_or_else(|| truncated("node"))?;
            let dy = reader.i16_as_i32().ok_or_else(|| truncated("node"))?;
            (x, y, dx, dy)
        };
        let mut bbox = || -> Result<[i32; 4], MapAssembleError> {
            Ok([
                reader.i16_as_i32().ok_or_else(|| truncated("node"))?,
                reader.i16_as_i32().ok_or_else(|| truncated("node"))?,
                reader.i16_as_i32().ok_or_else(|| truncated("node"))?,
                reader.i16_as_i32().ok_or_else(|| truncated("node"))?,
            ])
        };
        let right_bbox = bbox()?;
        let left_bbox = bbox()?;
        let right_raw = reader.u32().ok_or_else(|| truncated("node"))?;
        let left_raw = reader.u32().ok_or_else(|| truncated("node"))?;
        let right = resolve_child(right_raw, node_count, subsector_count, strictness, warnings)?;
        let left = resolve_child(left_raw, node_count, subsector_count, strictness, warnings)?;
        nodes.push(MapNode {
            x,
            y,
            dx,
            dy,
            right_bbox,
            left_bbox,
            right,
            left,
        });
    }

    Ok(DecodedExtendedBsp {
        new_vertices,
        segs,
        subsectors,
        nodes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::graph::Special;

    /// A chainable little-endian byte-stream builder for hand-crafting fixtures.
    #[derive(Default)]
    struct Buf(Vec<u8>);

    impl Buf {
        fn tag(mut self, t: [u8; 4]) -> Self {
            self.0.extend_from_slice(&t);
            self
        }
        fn u32(mut self, v: u32) -> Self {
            self.0.extend_from_slice(&v.to_le_bytes());
            self
        }
        fn i32(mut self, v: i32) -> Self {
            self.0.extend_from_slice(&v.to_le_bytes());
            self
        }
        fn u16(mut self, v: u16) -> Self {
            self.0.extend_from_slice(&v.to_le_bytes());
            self
        }
        fn i16(mut self, v: i16) -> Self {
            self.0.extend_from_slice(&v.to_le_bytes());
            self
        }
        fn u8(mut self, v: u8) -> Self {
            self.0.push(v);
            self
        }
        fn build(self) -> Vec<u8> {
            self.0
        }
    }

    /// The four corners of a 64×64 axis-aligned square, in CCW order.
    fn square() -> Vec<MapVertex> {
        vec![
            MapVertex { x: 0.0, y: 0.0 },
            MapVertex { x: 64.0, y: 0.0 },
            MapVertex { x: 64.0, y: 64.0 },
            MapVertex { x: 0.0, y: 64.0 },
        ]
    }

    /// A minimal linedef between two vertex indices.
    fn linedef(start: usize, end: usize) -> MapLinedef {
        MapLinedef {
            start: VertexIdx(start),
            end: VertexIdx(end),
            right: None,
            left: None,
            flags: 0,
            special: Special {
                special: 0,
                args: [0; 5],
            },
            id: 0,
        }
    }

    /// Four edge linedefs of the square, each starting at vertex `i` — so a
    /// front seg whose start vertex is `i` derives offset `0`.
    fn square_linedefs() -> Vec<MapLinedef> {
        (0..4).map(|i| linedef(i, (i + 1) % 4)).collect()
    }

    fn decode(
        bytes: &[u8],
        kind: ExtendedNodeKind,
        verts: &[MapVertex],
        lds: &[MapLinedef],
        strictness: Strictness,
    ) -> (
        Result<DecodedExtendedBsp, MapAssembleError>,
        Vec<MapWarning>,
    ) {
        let mut warnings = Vec::new();
        let out = decode_extended_nodes(bytes, kind, verts, lds, strictness, &mut warnings);
        (out, warnings)
    }

    #[test]
    fn bam_encoding_matches_known_cardinal_angles() {
        let o = MapVertex { x: 0.0, y: 0.0 };
        assert_eq!(derive_angle(o, MapVertex { x: 1.0, y: 0.0 }), 0x0000); // east
        assert_eq!(derive_angle(o, MapVertex { x: 0.0, y: 1.0 }), 0x4000); // north
        assert_eq!(derive_angle(o, MapVertex { x: -1.0, y: 0.0 }), 0x8000); // west
        assert_eq!(derive_angle(o, MapVertex { x: 0.0, y: -1.0 }), 0xC000); // south
        // Coincident endpoints are deterministic, not a panic: atan2(0,0) == 0.
        assert_eq!(derive_angle(o, o), 0x0000);
    }

    /// A 4-seg square XGLN stream (formatspec §5), 1 subsector, 0 nodes.
    fn xgln_square(line_overrides: [u16; 4]) -> Vec<u8> {
        let mut b = Buf::default()
            .tag(*b"XGLN")
            .u32(4) // origVerts
            .u32(0) // newVerts
            .u32(1) // numSubsectors
            .u32(4) // ss0 segCount
            .u32(4); // numSegs
        for (i, line) in line_overrides.iter().enumerate() {
            b = b
                .u32(u32::try_from(i).unwrap()) // v1
                .u32(0xFFFF_FFFF) // partner = none
                .u16(*line) // linedef
                .u8(0); // side
        }
        b.u32(0).build() // numNodes = 0
    }

    #[test]
    fn xgln_square_implicit_v2_and_derivations() {
        let bytes = xgln_square([0, 1, 2, 3]);
        let (out, warnings) = decode(
            &bytes,
            ExtendedNodeKind::Xgln,
            &square(),
            &square_linedefs(),
            Strictness::Strict,
        );
        let bsp = out.expect("valid stream");
        assert!(warnings.is_empty());
        assert!(bsp.new_vertices.is_empty());
        assert!(bsp.nodes.is_empty());
        assert_eq!(bsp.subsectors.len(), 1);
        assert_eq!(bsp.subsectors[0].segs, 0..4);
        assert_eq!(bsp.segs.len(), 4);

        // Implicit v2 wrap 0->1->2->3->0, with axis-aligned cardinal angles.
        let expected = [
            (0usize, 1usize, 0x0000u16), // east
            (1, 2, 0x4000),              // north
            (2, 3, 0x8000),              // west
            (3, 0, 0xC000),              // south
        ];
        for (i, seg) in bsp.segs.iter().enumerate() {
            let (s, e, angle) = expected[i];
            assert_eq!(seg.start, VertexIdx(s), "seg {i} start");
            assert_eq!(seg.end, VertexIdx(e), "seg {i} end");
            assert_eq!(seg.angle, angle, "seg {i} angle");
            assert_eq!(seg.linedef, Some(LinedefIdx(i)), "seg {i} linedef");
            assert_eq!(seg.direction, 0, "seg {i} direction");
            assert_eq!(seg.offset, 0, "seg {i} offset");
        }
    }

    #[test]
    fn xgln_miniseg_has_no_linedef_and_zero_offset() {
        // seg 2's linedef field is the XGLN sentinel 0xFFFF.
        let bytes = xgln_square([0, 1, 0xFFFF, 3]);
        let (out, _) = decode(
            &bytes,
            ExtendedNodeKind::Xgln,
            &square(),
            &square_linedefs(),
            Strictness::Strict,
        );
        let bsp = out.expect("valid stream");
        assert_eq!(bsp.segs[2].linedef, None);
        assert_eq!(bsp.segs[2].offset, 0);
        // Its geometry (angle) is still derived from the endpoints.
        assert_eq!(bsp.segs[2].angle, 0x8000);
        // Neighbors remain linedef-backed.
        assert_eq!(bsp.segs[1].linedef, Some(LinedefIdx(1)));
    }

    #[test]
    fn xgl2_u32_linedef_and_miniseg_sentinel() {
        // XGL2: 13-byte segs (u32 linedef); sentinel is 0xFFFFFFFF.
        let mut b = Buf::default()
            .tag(*b"XGL2")
            .u32(4)
            .u32(0)
            .u32(1)
            .u32(4)
            .u32(4);
        let lines = [0u32, 0xFFFF_FFFF, 2, 3];
        for (i, line) in lines.iter().enumerate() {
            b = b
                .u32(u32::try_from(i).unwrap())
                .u32(0xFFFF_FFFF)
                .u32(*line)
                .u8(0);
        }
        let bytes = b.u32(0).build();
        let (out, _) = decode(
            &bytes,
            ExtendedNodeKind::Xgl2,
            &square(),
            &square_linedefs(),
            Strictness::Strict,
        );
        let bsp = out.expect("valid stream");
        assert_eq!(bsp.segs[0].linedef, Some(LinedefIdx(0)));
        assert_eq!(bsp.segs[1].linedef, None); // miniseg
        assert_eq!(bsp.segs[3].linedef, Some(LinedefIdx(3)));
        assert_eq!(bsp.segs[0].angle, 0x0000);
    }

    #[test]
    fn xgl3_node_fixed_point_partition_bbox_and_children() {
        // 2 empty subsectors, 0 segs, 1 XGL3 node (40 bytes).
        let bytes = Buf::default()
            .tag(*b"XGL3")
            .u32(4) // origVerts
            .u32(0) // newVerts
            .u32(2) // numSubsectors
            .u32(0) // ss0 segCount
            .u32(0) // ss1 segCount
            .u32(0) // numSegs
            .u32(1) // numNodes
            // partition: i32 16.16 fixed-point
            .i32(64 * 65536)
            .i32(128 * 65536)
            .i32(-16 * 65536)
            .i32(32 * 65536)
            // right bbox [top,bottom,left,right] i16
            .i16(100)
            .i16(-100)
            .i16(-50)
            .i16(50)
            // left bbox
            .i16(10)
            .i16(-10)
            .i16(-5)
            .i16(5)
            // children: bit-31 set => subsector
            .u32(0x8000_0000)
            .u32(0x8000_0001)
            .build();
        let (out, _) = decode(
            &bytes,
            ExtendedNodeKind::Xgl3,
            &square(),
            &square_linedefs(),
            Strictness::Strict,
        );
        let bsp = out.expect("valid stream");
        assert_eq!(bsp.nodes.len(), 1);
        let n = bsp.nodes[0];
        // Fixed-point partition arithmetic-shifted to whole map units.
        assert_eq!((n.x, n.y, n.dx, n.dy), (64, 128, -16, 32));
        assert_eq!(n.right_bbox, [100, -100, -50, 50]);
        assert_eq!(n.left_bbox, [10, -10, -5, 5]);
        assert_eq!(n.right, NodeChild::Subsector(SubsectorIdx(0)));
        assert_eq!(n.left, NodeChild::Subsector(SubsectorIdx(1)));
    }

    #[test]
    fn xnod_explicit_v2_and_i16_node() {
        // XNOD: 13-byte segs with explicit v2, no partner, no minisegs; a
        // 32-byte i16 node.
        let lds = vec![linedef(1, 0), linedef(3, 2)];
        let bytes = Buf::default()
            .tag(*b"XNOD")
            .u32(4) // origVerts
            .u32(0) // newVerts
            .u32(1) // numSubsectors
            .u32(2) // segCount
            .u32(2) // numSegs
            // seg0: v1=0, v2=1, line=0, side=0
            .u32(0)
            .u32(1)
            .u16(0)
            .u8(0)
            // seg1: v1=2, v2=3, line=1, side=1
            .u32(2)
            .u32(3)
            .u16(1)
            .u8(1)
            .u32(1) // numNodes
            // i16 partition
            .i16(8)
            .i16(16)
            .i16(-4)
            .i16(2)
            // right bbox
            .i16(50)
            .i16(-50)
            .i16(-25)
            .i16(25)
            // left bbox
            .i16(5)
            .i16(-5)
            .i16(-2)
            .i16(2)
            // children: both subsector 0
            .u32(0x8000_0000)
            .u32(0x8000_0000)
            .build();
        let (out, warnings) = decode(
            &bytes,
            ExtendedNodeKind::Xnod,
            &square(),
            &lds,
            Strictness::Strict,
        );
        let bsp = out.expect("valid stream");
        assert!(warnings.is_empty());
        assert_eq!(bsp.segs.len(), 2);
        // seg0: explicit v2 = 1, front side, linedef start vertex 1 => Dist 64.
        assert_eq!(bsp.segs[0].start, VertexIdx(0));
        assert_eq!(bsp.segs[0].end, VertexIdx(1));
        assert_eq!(bsp.segs[0].linedef, Some(LinedefIdx(0)));
        assert_eq!(bsp.segs[0].direction, 0);
        assert_eq!(bsp.segs[0].angle, 0x0000); // east
        assert_eq!(bsp.segs[0].offset, 64);
        // seg1: back side => linedef end vertex 2 == seg start => Dist 0.
        assert_eq!(bsp.segs[1].direction, 1);
        assert_eq!(bsp.segs[1].angle, 0x8000); // west
        assert_eq!(bsp.segs[1].offset, 0);
        // i16 node partition widened directly.
        let n = bsp.nodes[0];
        assert_eq!((n.x, n.y, n.dx, n.dy), (8, 16, -4, 2));
        assert_eq!(n.right_bbox, [50, -50, -25, 25]);
        assert_eq!(n.right, NodeChild::Subsector(SubsectorIdx(0)));
        assert_eq!(n.left, NodeChild::Subsector(SubsectorIdx(0)));
    }

    #[test]
    fn xnod_seg_block_at_lump_end_is_not_overflowed_i1_regression() {
        // Regression for I1: XNOD segs are 11 bytes each (u32 v1 + u32 v2 + u16
        // line + u8 side), not 13. A 72-byte XNOD lump: 24-byte header (tag +
        // origVerts + newVerts + numSubsectors + segCount + numSegs), 4 segs ×
        // 11 bytes = 44, then numNodes = 0 (4 bytes). Before the fix,
        // `seg_size()` returning 13 made `fits()` demand 52 bytes for the seg
        // block when only 48 remained (44 segs + the trailing numNodes field),
        // so this valid, minimal stream was spuriously rejected as
        // `CountOverflow` even though the DECODE read path (11 bytes/seg) would
        // have consumed it correctly.
        let mut b = Buf::default()
            .tag(*b"XNOD")
            .u32(4) // origVerts
            .u32(0) // newVerts
            .u32(1) // numSubsectors
            .u32(4) // ss0 segCount
            .u32(4); // numSegs
        for i in 0..4u32 {
            b = b
                .u32(i) // v1
                .u32((i + 1) % 4) // v2
                .u16(u16::try_from(i).unwrap()) // linedef
                .u8(0); // side
        }
        let bytes = b.u32(0).build(); // numNodes = 0
        assert_eq!(
            bytes.len(),
            72,
            "fixture is the exact 72-byte regression case"
        );

        let (out, warnings) = decode(
            &bytes,
            ExtendedNodeKind::Xnod,
            &square(),
            &square_linedefs(),
            Strictness::Strict,
        );
        let bsp = out.expect("valid XNOD stream with seg block at lump end must decode");
        assert!(warnings.is_empty());
        assert!(bsp.nodes.is_empty());
        assert_eq!(bsp.subsectors.len(), 1);
        assert_eq!(bsp.subsectors[0].segs, 0..4);
        assert_eq!(bsp.segs.len(), 4);
        for (i, seg) in bsp.segs.iter().enumerate() {
            assert_eq!(seg.start, VertexIdx(i), "seg {i} start");
            assert_eq!(seg.end, VertexIdx((i + 1) % 4), "seg {i} end");
            assert_eq!(seg.linedef, Some(LinedefIdx(i)), "seg {i} linedef");
        }

        // Lenient mode must also decode successfully, not degrade to empty.
        let (lenient, lenient_warnings) = decode(
            &bytes,
            ExtendedNodeKind::Xnod,
            &square(),
            &square_linedefs(),
            Strictness::Lenient,
        );
        let lenient_bsp = lenient.expect("lenient must also decode, not degrade");
        assert_eq!(lenient_bsp.segs.len(), 4);
        assert!(lenient_warnings.is_empty());
    }

    #[test]
    fn node_with_two_subsector_children_and_root_last() {
        // 2 empty subsectors, 1 node whose two children are subsectors 0 and 1.
        let bytes = Buf::default()
            .tag(*b"XGLN")
            .u32(4)
            .u32(0)
            .u32(2) // numSubsectors
            .u32(0)
            .u32(0)
            .u32(0) // numSegs
            .u32(1) // numNodes
            .i16(0)
            .i16(0)
            .i16(0)
            .i16(0)
            .i16(0)
            .i16(0)
            .i16(0)
            .i16(0)
            .i16(0)
            .i16(0)
            .i16(0)
            .i16(0)
            .u32(0x8000_0000)
            .u32(0x8000_0000 | 1)
            .build();
        let (out, _) = decode(
            &bytes,
            ExtendedNodeKind::Xgln,
            &square(),
            &square_linedefs(),
            Strictness::Strict,
        );
        let bsp = out.expect("valid stream");
        assert_eq!(bsp.nodes.len(), 1);
        // Root convention is the last node (index len-1); here node 0.
        let root = &bsp.nodes[bsp.nodes.len() - 1];
        assert_eq!(root.right, NodeChild::Subsector(SubsectorIdx(0)));
        assert_eq!(root.left, NodeChild::Subsector(SubsectorIdx(1)));
    }

    #[test]
    fn truncated_mid_seg_strict_errors_lenient_degrades() {
        let full = xgln_square([0, 1, 2, 3]);
        // Drop the trailing numNodes field: the seg block reads fine, then the
        // node-count read hits EOF. (A cut *inside* the seg block is instead
        // caught by the bounded-allocation guard as CountOverflow.)
        let truncated = &full[..full.len() - 4];
        let (strict, _) = decode(
            truncated,
            ExtendedNodeKind::Xgln,
            &square(),
            &square_linedefs(),
            Strictness::Strict,
        );
        assert!(matches!(
            strict,
            Err(MapAssembleError::ExtendedNode {
                reason: ExtendedNodeError::Truncated { .. },
                ..
            })
        ));
        let (lenient, warnings) = decode(
            truncated,
            ExtendedNodeKind::Xgln,
            &square(),
            &square_linedefs(),
            Strictness::Lenient,
        );
        let bsp = lenient.expect("lenient degrades");
        assert!(bsp.segs.is_empty() && bsp.subsectors.is_empty() && bsp.nodes.is_empty());
        assert_eq!(warnings.len(), 1);
        assert!(matches!(warnings[0], MapWarning::ExtendedNode { .. }));
    }

    #[test]
    fn seg_count_overflows_lump_strict_errors_lenient_degrades() {
        // sum(segCount) == numSegs, but numSegs is far larger than the lump.
        let bytes = Buf::default()
            .tag(*b"XGLN")
            .u32(4)
            .u32(0)
            .u32(1)
            .u32(0x1000_0000) // ss0 segCount
            .u32(0x1000_0000) // numSegs (matches sum, but overflows the lump)
            .build();
        let (strict, _) = decode(
            &bytes,
            ExtendedNodeKind::Xgln,
            &square(),
            &square_linedefs(),
            Strictness::Strict,
        );
        assert!(matches!(
            strict,
            Err(MapAssembleError::ExtendedNode {
                reason: ExtendedNodeError::CountOverflow { .. },
                ..
            })
        ));
        let (lenient, warnings) = decode(
            &bytes,
            ExtendedNodeKind::Xgln,
            &square(),
            &square_linedefs(),
            Strictness::Lenient,
        );
        assert!(lenient.expect("degrades").segs.is_empty());
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn seg_count_mismatch_strict_errors_lenient_degrades() {
        // segCount sum (5) != numSegs (4).
        let mut b = Buf::default()
            .tag(*b"XGLN")
            .u32(4)
            .u32(0)
            .u32(1)
            .u32(5) // ss0 segCount
            .u32(4); // numSegs
        for i in 0..4u16 {
            b = b.u32(u32::from(i)).u32(0xFFFF_FFFF).u16(i).u8(0);
        }
        let bytes = b.u32(0).build();
        let (strict, _) = decode(
            &bytes,
            ExtendedNodeKind::Xgln,
            &square(),
            &square_linedefs(),
            Strictness::Strict,
        );
        assert!(matches!(
            strict,
            Err(MapAssembleError::ExtendedNode {
                reason: ExtendedNodeError::SegCountMismatch { .. },
                ..
            })
        ));
        let (lenient, warnings) = decode(
            &bytes,
            ExtendedNodeKind::Xgln,
            &square(),
            &square_linedefs(),
            Strictness::Lenient,
        );
        assert!(lenient.expect("degrades").nodes.is_empty());
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn orig_verts_smaller_than_existing_warns_but_proceeds() {
        // origVerts = 2, but the map has 4 vertices: recoverable mismatch.
        let mut b = Buf::default()
            .tag(*b"XGLN")
            .u32(2) // origVerts (< existing 4)
            .u32(0)
            .u32(1)
            .u32(4)
            .u32(4);
        for i in 0..4u16 {
            b = b.u32(u32::from(i)).u32(0xFFFF_FFFF).u16(i).u8(0);
        }
        let bytes = b.u32(0).build();
        let (strict, _) = decode(
            &bytes,
            ExtendedNodeKind::Xgln,
            &square(),
            &square_linedefs(),
            Strictness::Strict,
        );
        assert!(matches!(
            strict,
            Err(MapAssembleError::ExtendedNode {
                reason: ExtendedNodeError::VertexHeaderMismatch { .. },
                ..
            })
        ));
        // Lenient proceeds using the map's count as the split base.
        let (lenient, warnings) = decode(
            &bytes,
            ExtendedNodeKind::Xgln,
            &square(),
            &square_linedefs(),
            Strictness::Lenient,
        );
        assert_eq!(lenient.expect("proceeds").segs.len(), 4);
        assert_eq!(warnings.len(), 1);
        assert!(matches!(
            warnings[0],
            MapWarning::ExtendedNode {
                reason: ExtendedNodeError::VertexHeaderMismatch { .. },
                ..
            }
        ));
    }

    #[test]
    fn orig_verts_larger_than_existing_strict_errors_lenient_degrades() {
        let bytes = Buf::default()
            .tag(*b"XGLN")
            .u32(9) // origVerts > existing 4
            .u32(0)
            .u32(0)
            .u32(0)
            .u32(0)
            .build();
        let (strict, _) = decode(
            &bytes,
            ExtendedNodeKind::Xgln,
            &square(),
            &square_linedefs(),
            Strictness::Strict,
        );
        assert!(matches!(
            strict,
            Err(MapAssembleError::ExtendedNode {
                reason: ExtendedNodeError::VertexHeaderMismatch { .. },
                ..
            })
        ));
        let (lenient, warnings) = decode(
            &bytes,
            ExtendedNodeKind::Xgln,
            &square(),
            &square_linedefs(),
            Strictness::Lenient,
        );
        assert!(lenient.expect("degrades").segs.is_empty());
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn child_index_out_of_range_strict_errors_lenient_clamps() {
        // One subsector, a node child pointing at subsector 5.
        let bytes = Buf::default()
            .tag(*b"XGLN")
            .u32(4)
            .u32(0)
            .u32(1) // numSubsectors
            .u32(0)
            .u32(0) // numSegs
            .u32(1) // numNodes
            .i16(0)
            .i16(0)
            .i16(0)
            .i16(0)
            .i16(0)
            .i16(0)
            .i16(0)
            .i16(0)
            .i16(0)
            .i16(0)
            .i16(0)
            .i16(0)
            .u32(0x8000_0000 | 5) // subsector 5 (out of range)
            .u32(0x8000_0000)
            .build();
        let (strict, _) = decode(
            &bytes,
            ExtendedNodeKind::Xgln,
            &square(),
            &square_linedefs(),
            Strictness::Strict,
        );
        assert!(matches!(
            strict,
            Err(MapAssembleError::DanglingReference {
                referent: "subsector",
                ..
            })
        ));
        // Lenient clamps the child to subsector 0 and continues (no degrade).
        let (lenient, warnings) = decode(
            &bytes,
            ExtendedNodeKind::Xgln,
            &square(),
            &square_linedefs(),
            Strictness::Lenient,
        );
        let bsp = lenient.expect("clamps");
        assert_eq!(bsp.nodes.len(), 1);
        assert_eq!(bsp.nodes[0].right, NodeChild::Subsector(SubsectorIdx(0)));
        assert!(matches!(
            warnings[0],
            MapWarning::DanglingReference {
                referent: "subsector",
                ..
            }
        ));
    }

    #[test]
    fn partner_out_of_range_strict_errors_lenient_warns_and_continues() {
        // A single-seg subsector whose partner index (7) exceeds numSegs (1).
        let bytes = Buf::default()
            .tag(*b"XGLN")
            .u32(4)
            .u32(0)
            .u32(1)
            .u32(1) // segCount
            .u32(1) // numSegs
            .u32(0) // v1
            .u32(7) // partner (out of range, != 0xFFFFFFFF)
            .u16(0) // linedef
            .u8(0)
            .u32(0) // numNodes
            .build();
        let (strict, _) = decode(
            &bytes,
            ExtendedNodeKind::Xgln,
            &square(),
            &square_linedefs(),
            Strictness::Strict,
        );
        assert!(matches!(
            strict,
            Err(MapAssembleError::ExtendedNode {
                reason: ExtendedNodeError::PartnerOutOfRange { .. },
                ..
            })
        ));
        let (lenient, warnings) = decode(
            &bytes,
            ExtendedNodeKind::Xgln,
            &square(),
            &square_linedefs(),
            Strictness::Lenient,
        );
        // Partner is validation-only, so lenient warns and still decodes the seg.
        assert_eq!(lenient.expect("continues").segs.len(), 1);
        assert!(matches!(
            warnings[0],
            MapWarning::ExtendedNode {
                reason: ExtendedNodeError::PartnerOutOfRange { .. },
                ..
            }
        ));
    }

    proptest::proptest! {
        #[test]
        fn random_streams_never_panic(data in proptest::collection::vec(proptest::prelude::any::<u8>(), 0..300)) {
            let verts = square();
            let lds = square_linedefs();
            for kind in [
                ExtendedNodeKind::Xnod,
                ExtendedNodeKind::Xgln,
                ExtendedNodeKind::Xgl2,
                ExtendedNodeKind::Xgl3,
            ] {
                for strictness in [Strictness::Strict, Strictness::Lenient] {
                    let mut warnings = Vec::new();
                    let _ = decode_extended_nodes(&data, kind, &verts, &lds, strictness, &mut warnings);
                }
            }
        }
    }
}
