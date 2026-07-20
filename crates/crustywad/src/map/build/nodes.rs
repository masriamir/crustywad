//! The classic BSP pass — the `build_nodes` kernel, its output types, and
//! their lump serialization (ADR-0024 §2, staging §9.2, issue #315).
//!
//! [`build_nodes`] partitions an assembled [`Map`]'s segs into a classic BSP
//! tree on seg lines only (no synthesized partitions), producing [`BuiltNodes`]:
//! the split vertices it created, plus the `SEGS`, `SSECTORS`, and `NODES`
//! arenas, expressed in the same normalized graph types the reader assembles
//! into ([`MapSeg`], [`MapSubsector`], [`MapNode`]).
//! [`BuiltNodes::to_lump_bytes`] renders those arenas to the on-disk lumps the
//! engine reads, reusing the [`common`](crate::map::common) record structs so
//! the byte layout is declared exactly once. A convex leaf spanning more than
//! one sector with no separating seg line is the tolerated mixed-sector fan
//! (ADR-0024 §7 amendment): strict rejects it, lenient warns and emits it.
//!
//! [`Map`]: crate::map::Map

use std::collections::HashMap;
use std::io::Cursor;

use binrw::BinWriterExt;

use crate::Strictness;
use crate::map::DoomWriteError;
use crate::map::build::{NodeBuildError, NodeBuildOptions, NodeBuildWarning};
use crate::map::common::{Node, Seg, Subsector, Vertex};
use crate::map::doom::{DoomWriteWarning, Narrower, narrow_vertices};
use crate::map::graph::{
    LinedefIdx, Map, MapNode, MapSeg, MapSubsector, MapVertex, NodeChild, NodeIdx, SubsectorIdx,
    VertexIdx,
};

/// The BSP child-reference leaf flag (`NF_SUBSECTOR`): with bit 15 set the
/// remaining 15 bits are a subsector index, otherwise a node index (Chocolate
/// Doom `src/doom/doomdata.h:175`, `#define NF_SUBSECTOR 0x8000`).
const NF_SUBSECTOR: u16 = 0x8000;

/// The subsector/node count ceiling (ADR-0024 §5, Global Constraint 6). A BSP
/// child reference reserves bit 15 ([`NF_SUBSECTOR`]) as the leaf flag, so an
/// index must fit the low 15 bits (`0..=0x7FFF`); the largest legal arena
/// therefore holds `0x8000` (32,768) elements.
const MAX_BSP_INDEX: usize = 0x8000;

/// The number of distinct values a `u16` index can address (65,536). Vertex,
/// linedef, and seg references narrow to `u16`, so their indices must be below
/// this (`0..=0xFFFF`).
const MAX_U16_INDEXED: usize = 0x1_0000;

/// The assembled output of the classic BSP pass (ADR-0024 §2): the vertices the
/// pass created by splitting straddling segs, plus the finished `SEGS`,
/// `SSECTORS`, and `NODES` arenas.
///
/// # Index domains (ADR-0024 §2, Global Constraint 5)
///
/// The arenas reference each other — and the owning [`Map`](crate::map::Map) —
/// by index, under these conventions:
///
/// - A seg's [`start`](MapSeg::start)/[`end`](MapSeg::end) index the **map's
///   own vertices first, then [`split_vertices`](Self::split_vertices)**: an
///   index `i < map.vertices().len()` is a map vertex; `i` at or above that is
///   `split_vertices[i - map.vertices().len()]`. (This module does not carry
///   the map's vertices; the offset is applied by the kernel that fills these
///   arenas.)
/// - A subsector's [`segs`](MapSubsector::segs) range indexes
///   [`segs`](Self::segs). These ranges are contiguous and partition `segs`
///   exactly: every seg belongs to exactly one subsector, in subsector order.
/// - A node's [`right`](MapNode::right)/[`left`](MapNode::left) child indexes
///   either [`nodes`](Self::nodes) (an internal node) or
///   [`subsectors`](Self::subsectors) (a leaf), per [`NodeChild`]. The BSP root
///   is the **last** node, matching [`Map::bsp_root`](crate::map::Map::bsp_root).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct BuiltNodes {
    /// The vertices created by splitting straddling segs, in creation order.
    /// Empty when no linedef needed splitting (a fully convex map). Each
    /// coordinate is a whole map unit in `i16` range (the kernel rounds at
    /// creation); seg indices at or above `map.vertices().len()` address this
    /// arena (see the type's index-domain notes).
    pub split_vertices: Vec<MapVertex>,
    /// The `SEGS` arena: every BSP seg, ordered so each subsector owns a
    /// contiguous run (see [`subsectors`](Self::subsectors)).
    pub segs: Vec<MapSeg>,
    /// The `SSECTORS` arena: one leaf per convex region, each a contiguous run
    /// into [`segs`](Self::segs).
    pub subsectors: Vec<MapSubsector>,
    /// The `NODES` arena: the internal BSP tree in post-order, so every child
    /// index is already assigned and the root lands last.
    pub nodes: Vec<MapNode>,
}

/// The four serialized node lumps produced by [`BuiltNodes::to_lump_bytes`], in
/// canonical order (ADR-0024 §2).
///
/// [`split_vertexes`](Self::split_vertexes) holds only the pass's *new*
/// vertices; a writer appends it to the map's existing `VERTEXES` lump so the
/// seg vertex indices resolve (see [`BuiltNodes`]'s index-domain notes).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct BuiltNodeLumps {
    /// The serialized split vertices, as `VERTEXES` records (4 bytes each) —
    /// the tail to append to the map's existing `VERTEXES` lump.
    pub split_vertexes: Vec<u8>,
    /// The serialized `SEGS` lump (12 bytes per record).
    pub segs: Vec<u8>,
    /// The serialized `SSECTORS` lump (4 bytes per record).
    pub ssectors: Vec<u8>,
    /// The serialized `NODES` lump (28 bytes per record); empty for a map that
    /// is a single convex subsector (the engine's `numnodes == 0` path).
    pub nodes: Vec<u8>,
}

impl BuiltNodes {
    /// Serializes this BSP tree to its four on-disk lumps (ADR-0024 §2, §D).
    ///
    /// Each arena is mapped into the matching [`common`](crate::map::common)
    /// record — [`Vertex`], [`Seg`], [`Subsector`], [`Node`] — and dumped
    /// little-endian, so the on-disk layout is declared once (by those structs)
    /// and never restated here. A node child is encoded `NF_SUBSECTOR (0x8000)
    /// | subsector_index` for a leaf and the bare node index for an internal
    /// child.
    ///
    /// # Errors
    ///
    /// - [`NodeBuildError::TooManyElements`] (**both** strictness modes; ADR-0024
    ///   §5, Global Constraint 6) when [`subsectors`](Self::subsectors) or
    ///   [`nodes`](Self::nodes) exceeds 32,768 — a BSP child reference reserves
    ///   bit 15 as the `NF_SUBSECTOR` leaf flag, so those indices must fit 15
    ///   bits. This structural ceiling is mode-independent.
    /// - [`NodeBuildError::Write`] wrapping
    ///   [`DoomWriteError::ValueOutOfRange`] with `field: "offset"` when a seg's
    ///   [`offset`](MapSeg::offset) exceeds the `i16` on-disk range. This is the
    ///   **strict** half of the offset rule (ADR-0024 §D): the serializer takes
    ///   no strictness, so the *lenient* clamp-and-warn
    ///   ([`DoomWriteWarning::ValueClamped`](crate::map::DoomWriteWarning::ValueClamped))
    ///   is applied upstream by `build_nodes`, which clamps
    ///   [`offset`](MapSeg::offset) into `i16` range before it reaches here — a
    ///   well-formed [`BuiltNodes`] never trips this.
    /// - A defensive [`NodeBuildError::TooManyElements`] /
    ///   [`NodeBuildError::Write`] for a vertex/linedef/seg index, a node
    ///   coordinate, or a split-vertex coordinate (rounded half away from
    ///   zero, then range-checked —
    ///   [`DoomWriteError::ValueOutOfRange`] with `block: "vertex"` when the
    ///   result does not fit `i16`) that does not fit its `u16`/`i16` on-disk
    ///   field. `build_nodes` narrows coordinates and bounds indices before
    ///   constructing a [`BuiltNodes`], so these guard only hand-constructed
    ///   values.
    pub fn to_lump_bytes(&self) -> Result<BuiltNodeLumps, NodeBuildError> {
        // Structural count ceilings (both modes): the leaf flag occupies bit 15
        // of every child reference, so these indices must fit 15 bits.
        if self.subsectors.len() > MAX_BSP_INDEX {
            return Err(NodeBuildError::TooManyElements {
                kind: "subsectors",
                count: self.subsectors.len(),
                max: MAX_BSP_INDEX,
            });
        }
        if self.nodes.len() > MAX_BSP_INDEX {
            return Err(NodeBuildError::TooManyElements {
                kind: "nodes",
                count: self.nodes.len(),
                max: MAX_BSP_INDEX,
            });
        }

        let mut vertexes = Vec::with_capacity(self.split_vertices.len());
        for (i, v) in self.split_vertices.iter().enumerate() {
            vertexes.push(Vertex {
                x: narrow_vertex_coord(v.x, "x", i)?,
                y: narrow_vertex_coord(v.y, "y", i)?,
            });
        }

        let mut segs = Vec::with_capacity(self.segs.len());
        for (i, s) in self.segs.iter().enumerate() {
            segs.push(Seg {
                start_vertex: encode_index(s.start.0, "vertices")?,
                end_vertex: encode_index(s.end.0, "vertices")?,
                angle: s.angle,
                linedef: encode_index(s.linedef.0, "linedefs")?,
                direction: s.direction,
                offset: narrow_offset(s.offset, i)?,
            });
        }

        let mut subsectors = Vec::with_capacity(self.subsectors.len());
        for ss in &self.subsectors {
            let count = ss.segs.end.saturating_sub(ss.segs.start);
            subsectors.push(Subsector {
                seg_count: encode_count(count, "subsector segs")?,
                first_seg: encode_index(ss.segs.start, "segs")?,
            });
        }

        let mut nodes = Vec::with_capacity(self.nodes.len());
        for (i, n) in self.nodes.iter().enumerate() {
            nodes.push(Node {
                x: narrow_coord(n.x, "x", i)?,
                y: narrow_coord(n.y, "y", i)?,
                dx: narrow_coord(n.dx, "dx", i)?,
                dy: narrow_coord(n.dy, "dy", i)?,
                right_bbox: narrow_bbox(n.right_bbox, "right_bbox", i)?,
                left_bbox: narrow_bbox(n.left_bbox, "left_bbox", i)?,
                right_child: encode_child(n.right)?,
                left_child: encode_child(n.left)?,
            });
        }

        Ok(BuiltNodeLumps {
            split_vertexes: encode(&vertexes),
            segs: encode(&segs),
            ssectors: encode(&subsectors),
            nodes: encode(&nodes),
        })
    }
}

/// Narrows a split-vertex `f64` coordinate to the on-disk `i16`, rounding half
/// away from zero (the write path's rounding). A rounded value outside `i16`
/// range — or a non-finite one — is a [`DoomWriteError::ValueOutOfRange`] via
/// the [`Write`](NodeBuildError::Write) wrapper, the same shape as every
/// sibling narrowing helper. Defensive only: `build_nodes` creates split
/// vertices as whole, finite, in-`i16`-range map units, so this path is
/// unreachable for kernel-produced values and guards hand-constructed ones.
// Casts are guarded: the error path saturates deliberately, the success path is
// range-checked.
#[allow(clippy::cast_possible_truncation)]
fn narrow_vertex_coord(
    value: f64,
    field: &'static str,
    index: usize,
) -> Result<i16, NodeBuildError> {
    let rounded = value.round();
    if !(f64::from(i16::MIN)..=f64::from(i16::MAX)).contains(&rounded) {
        // Out of range or NaN (which fails the range test). The reported value
        // saturates into i64 (NaN reports 0) — precise enough for a
        // hand-constructed-only diagnostic.
        return Err(NodeBuildError::Write(DoomWriteError::ValueOutOfRange {
            block: "vertex",
            field,
            index,
            value: rounded as i64,
        }));
    }
    // Finite and within `i16` range by the check above.
    Ok(rounded as i16)
}

/// Converts an arena index to its `u16` on-disk form, or a defensive
/// [`NodeBuildError::TooManyElements`] naming `kind` if it does not fit. The
/// reported `count` is `idx + 1` — the smallest arena that could hold the
/// index. For a field that is itself a count (`Subsector::seg_count`), use
/// [`encode_count`] instead, which reports the value verbatim.
fn encode_index(idx: usize, kind: &'static str) -> Result<u16, NodeBuildError> {
    u16::try_from(idx).map_err(|_| NodeBuildError::TooManyElements {
        kind,
        count: idx.saturating_add(1),
        max: MAX_U16_INDEXED,
    })
}

/// Converts an element *count* (`Subsector::seg_count`) to its `u16` on-disk
/// form, or a defensive [`NodeBuildError::TooManyElements`] naming `kind` if
/// it does not fit. Unlike [`encode_index`], the reported `count` is the value
/// itself — no `+ 1` adjustment — and the ceiling is `u16::MAX` (the largest
/// storable count), not the index-domain size.
fn encode_count(count: usize, kind: &'static str) -> Result<u16, NodeBuildError> {
    u16::try_from(count).map_err(|_| NodeBuildError::TooManyElements {
        kind,
        count,
        max: usize::from(u16::MAX),
    })
}

/// Narrows a seg `offset` (`i32`) to the on-disk `i16`. Overflow is the strict
/// half of the ADR-0024 §D offset rule: a [`DoomWriteError::ValueOutOfRange`]
/// with `field: "offset"`.
fn narrow_offset(offset: i32, index: usize) -> Result<i16, NodeBuildError> {
    i16::try_from(offset).map_err(|_| {
        NodeBuildError::Write(DoomWriteError::ValueOutOfRange {
            block: "seg",
            field: "offset",
            index,
            value: i64::from(offset),
        })
    })
}

/// Narrows a node partition coordinate (`i32`) to the on-disk `i16`, or a
/// defensive [`DoomWriteError::ValueOutOfRange`] naming `field`.
fn narrow_coord(value: i32, field: &'static str, index: usize) -> Result<i16, NodeBuildError> {
    i16::try_from(value).map_err(|_| {
        NodeBuildError::Write(DoomWriteError::ValueOutOfRange {
            block: "node",
            field,
            index,
            value: i64::from(value),
        })
    })
}

/// Narrows a node child bounding box (four `i32`) to the on-disk `[i16; 4]`.
fn narrow_bbox(
    bbox: [i32; 4],
    field: &'static str,
    index: usize,
) -> Result<[i16; 4], NodeBuildError> {
    let mut out = [0i16; 4];
    for (slot, &value) in out.iter_mut().zip(bbox.iter()) {
        *slot = narrow_coord(value, field, index)?;
    }
    Ok(out)
}

/// Encodes a [`NodeChild`] as its on-disk `u16`: a leaf sets [`NF_SUBSECTOR`]
/// (bit 15) over the subsector index, an internal node stores the bare node
/// index. The index must leave bit 15 free (Global Constraint 6); a value that
/// does not is a defensive [`NodeBuildError::TooManyElements`].
fn encode_child(child: NodeChild) -> Result<u16, NodeBuildError> {
    let (idx, leaf) = match child {
        NodeChild::Node(n) => (n.0, false),
        NodeChild::Subsector(s) => (s.0, true),
    };
    let kind = if leaf { "subsectors" } else { "nodes" };
    let too_many = || NodeBuildError::TooManyElements {
        kind,
        count: idx.saturating_add(1),
        max: MAX_BSP_INDEX,
    };
    let value = u16::try_from(idx).map_err(|_| too_many())?;
    if value & NF_SUBSECTOR != 0 {
        return Err(too_many());
    }
    Ok(if leaf { NF_SUBSECTOR | value } else { value })
}

/// Serializes a slice of [`binrw::BinWrite`] records into a lump byte buffer,
/// little-endian. Writing into an in-memory `Vec` cannot fail.
fn encode<T>(records: &[T]) -> Vec<u8>
where
    T: for<'a> binrw::BinWrite<Args<'a> = ()>,
{
    let mut cursor = Cursor::new(Vec::new());
    for record in records {
        cursor
            .write_le(record)
            .expect("writing to an in-memory Vec is infallible");
    }
    cursor.into_inner()
}

/// The vanilla vertex/seg *soft* ceiling (ADR-0024 §5, Global Constraint 6):
/// counts above this exceed the vanilla engine's fixed arrays but still fit the
/// format's 16-bit indices. Numerically equal to [`MAX_BSP_INDEX`], but a
/// distinct concept — this one warns (lenient) rather than always erroring.
const VANILLA_CEILING: usize = 0x8000;

/// Whether a partition delta fits the on-disk `i16` node `dx`/`dy` field
/// (§B.1): a seg can serve as a splitter only if its `v2 - v1` fits the **full
/// signed** `i16` range `[-32_768, 32_767]` on both axes. The range is
/// asymmetric — `i16::MIN` is a valid delta, `+32_768` is not.
fn partition_delta_fits(pdx: i64, pdy: i64) -> bool {
    let fits = |v: i64| i64::from(i16::MIN) <= v && v <= i64::from(i16::MAX);
    fits(pdx) && fits(pdy)
}

/// Above this working-set size the partition search evaluates every
/// `ceil(n / SAMPLE_BUDGET)`-th candidate first, falling back to all candidates
/// only if the sample finds none (§B.1). Correctness never depends on it.
const SAMPLE_BUDGET: usize = 512;

/// Builds the classic BSP tree for `map` (ADR-0024 §2, spec §A–D, issue #315).
///
/// Produces the [`BuiltNodes`] arenas — split vertices, `SEGS`, `SSECTORS`, and
/// `NODES` — a clean-room classic nodebuilder: it narrows the vertex arena
/// through the shared write-path pass (`opts.strictness` drives it, exactly as
/// [`build_blockmap`](crate::map::build::build_blockmap)), forms one seg per
/// present linedef side, then recursively partitions the seg set with an
/// **explicit work stack** (no call recursion; Global Constraint 9) until every
/// region is convex and single-sector.
///
/// # Engine conventions (Global Constraint 7)
///
/// Derived by reading Chocolate Doom (the permitted *consumer* source; ADR-0024
/// §1) and cited where load-bearing:
///
/// - **Side of a partition.** `R_PointOnSide` (`src/doom/r_main.c:145`) returns
///   side 0 (front) when `node_dx*(y - node_y) - node_dy*(x - node_x) < 0`. The
///   spec's cross `(q.x - p.x)*dy - (q.y - p.y)*dx` is the negation of that, so
///   **`cross > 0` is front, `cross < 0` is back**. `R_RenderBSPNode`
///   (`src/doom/r_bsp.c`) recurses `children[side]` front-first, so the **front
///   child is `child[0]` = the node's right child**.
/// - **`NF_SUBSECTOR = 0x8000`** (`src/doom/doomdata.h:175`): a child reference
///   with bit 15 set is a subsector leaf. Encoded in [`BuiltNodes::to_lump_bytes`].
/// - **Node bbox order `[top, bottom, left, right]`** = `[max_y, min_y, min_x,
///   max_x]` (`BOXTOP/BOXBOTTOM/BOXLEFT/BOXRIGHT` = 0/1/2/3,
///   `src/m_bbox.h:31`; `mapnode_t.bbox[2][4]`, `src/doom/doomdata.h:187`).
/// - **Seg `offset`** is consumed verbatim by the engine (`P_LoadSegs`,
///   `src/doom/p_setup.c:198`; used for texture alignment in `R_StoreWallRange`,
///   `src/doom/r_segs.c`), so this builder computes it as the distance along the
///   linedef from the seg's own start reference — the linedef start vertex for a
///   direction-0 seg, the linedef **end** vertex for a direction-1 seg.
///
/// # Errors
///
/// - [`NodeBuildError::EmptyGeometry`] (both modes) when the map has no
///   vertices, linedefs, sidedefs, or sectors — or when no linedef yields a seg
///   (every linedef is zero-length or sideless), leaving nothing to partition.
/// - [`NodeBuildError::Write`] wrapping a [`DoomWriteError`] from the shared
///   narrowing pass (strict-mode non-finite/fractional/out-of-range coordinate),
///   or a seg `offset` that overflows `i16` on a linedef longer than 32,767
///   units (strict `ValueOutOfRange`; lenient clamps and warns —
///   [`DoomWriteWarning::ValueClamped`], `field: "offset"`).
/// - [`NodeBuildError::TooManyElements`] (both modes) when vertices or segs
///   exceed 65,536, or subsectors or nodes exceed 32,768 (the structural 15-bit
///   child-reference ceiling); the > 32,768 vertex/seg *soft* ceiling is a
///   strict error and a lenient [`NodeBuildWarning::VanillaCeilingExceeded`].
/// - [`NodeBuildError::MixedSectorSubsector`] (strict) when a convex region
///   spans multiple sectors with no separating seg line; lenient accepts the
///   leaf and emits one [`NodeBuildWarning::MixedSectorSubsector`] per such leaf
///   — the engine-tolerated output the retail masters ship (ADR-0024 §7
///   amendment 2026-07-19).
/// - [`NodeBuildError::DegeneratePartition`] (both modes) — a hardening guard —
///   when a selected partition fails to separate its seg set into two non-empty
///   sides (only reachable for adversarial geometry via the §C.3 endpoint
///   fallback; well-formed maps never trip it).
pub fn build_nodes(
    map: &Map,
    opts: &NodeBuildOptions,
) -> Result<(BuiltNodes, Vec<NodeBuildWarning>), NodeBuildError> {
    // §A.1 empty-geometry gate — identical to `build_blockmap` (Global
    // Constraint 3 keeps the two in lockstep).
    if map.vertices().is_empty()
        || map.linedefs().is_empty()
        || map.sidedefs().is_empty()
        || map.sectors().is_empty()
    {
        return Err(NodeBuildError::EmptyGeometry);
    }
    // A seg stores its source linedef in a `u16`; an arena too large to index
    // cannot be serialized, so reject it up front (both modes) rather than
    // succeed here and fail later in `to_lump_bytes` (PR #319).
    check_linedef_count(map.linedefs().len())?;

    let mut bsp = Bsp::new(map, opts)?;
    bsp.build_initial_segs();
    if bsp.segs.is_empty() {
        // Every linedef was zero-length or sideless: no walls to partition and
        // hence no subsector to emit. Vanilla needs at least one, so this is the
        // same "nothing to build" condition as an empty arena (both modes).
        return Err(NodeBuildError::EmptyGeometry);
    }
    bsp.partition()?;
    bsp.finish()
}

/// A seg under construction. Endpoints index the *combined* vertex table
/// (`Bsp::verts`): the map's narrowed vertices first, then the split vertices
/// the pass creates, matching [`BuiltNodes`]'s index-domain contract.
#[derive(Clone, Copy)]
struct WorkSeg {
    /// Start-vertex index into the combined vertex table.
    v1: usize,
    /// End-vertex index into the combined vertex table.
    v2: usize,
    /// The source linedef.
    linedef: usize,
    /// `0` if the seg runs with the linedef (right side), `1` if reversed (left).
    direction: u16,
    /// The sector this seg's own side faces (for the single-sector rule, §C).
    side_sector: usize,
    /// The **raw** (un-narrowed) Euclidean distance along the linedef from the
    /// seg's start reference to `v1`, in map units (§D). Kept as `f64` and
    /// narrowed to `i16` range only at flatten time, where the seg's real final
    /// index is known for the [`DoomWriteWarning::ValueClamped`] / `ValueOutOfRange`
    /// diagnostic (Finding 2, PR #319).
    offset: f64,
}

/// How a seg sits relative to a partition line (§B.2).
///
/// Colinear segs (both endpoints on the line) are tracked separately from segs
/// with genuine off-line extent: a candidate is *convex* — no valid splitter —
/// when all **non-colinear** content lies on one side (the controller's
/// square-room fixture: a splitter's own colinear seg does not, by itself, make
/// its line a valid partition). The relaxed sector-separating rule (§C.2) does
/// count colinear segs, which is how a two-sided shared line separates two
/// sectors in an otherwise convex region.
/// A single classification decision is shared verbatim by [`Bsp::select`]'s
/// scoring and [`Bsp::split_set`]'s routing (via [`Bsp::classify_seg`]) so the
/// two can **never** disagree — the invariant that keeps
/// [`NodeBuildError::DegeneratePartition`] unreachable on well-formed geometry.
/// A straddler is [`Class::Split`] **only** when its rounded intersection is
/// strictly interior (§C.3 endpoint-coincidence collapse already folded in), so
/// a candidate whose "splits" all collapse to one side is correctly scored as
/// leaving the other side empty.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Class {
    /// Genuine extent on the front (right) half-plane (includes a straddler that
    /// collapsed to the front by the §C.3 endpoint rule).
    Front,
    /// Genuine extent on the back (left) half-plane (includes a §C.3 collapse).
    Back,
    /// Both endpoints on the line, same direction as the partition → front side.
    ColinearFront,
    /// Both endpoints on the line, opposite direction → back side.
    ColinearBack,
    /// Strictly straddling with a strictly-interior rounded intersection at
    /// `(x, y)` — the two fragments are guaranteed distinct and land on opposite
    /// sides.
    Split(i32, i32),
}

impl Class {
    /// Whether a non-splitting seg routes to the front side when partitioned.
    fn is_front(self) -> bool {
        matches!(self, Class::Front | Class::ColinearFront)
    }
}

/// A partition line and its precomputed integer/`i128` forms, built once per
/// candidate so [`Bsp::classify_seg`] does no redundant widening.
#[derive(Clone, Copy)]
struct Partition {
    /// Line start `x` (the splitter seg's `v1`), map units.
    px: i32,
    /// Line start `y`.
    py: i32,
    /// Line direction `dx` (fits `i16` by §B.1).
    dx: i32,
    /// Line direction `dy`.
    dy: i32,
    /// `px` as `i64`.
    pxi: i64,
    /// `py` as `i64`.
    pyi: i64,
    /// `dx` as `i64`.
    pdx: i64,
    /// `dy` as `i64`.
    pdy: i64,
    /// `dx² + dy²` as `i128` (the on-line threshold denominator).
    len2: i128,
}

impl Partition {
    /// Builds a partition from integer line start `(px, py)` and direction
    /// `(dx, dy)`.
    fn new(px: i32, py: i32, dx: i32, dy: i32) -> Self {
        let (pxi, pyi, pdx, pdy) = (i64::from(px), i64::from(py), i64::from(dx), i64::from(dy));
        let len2 = i128::from(pdx) * i128::from(pdx) + i128::from(pdy) * i128::from(pdy);
        Self {
            px,
            py,
            dx,
            dy,
            pxi,
            pyi,
            pdx,
            pdy,
            len2,
        }
    }
}

/// A child slot in the internal tree arena, resolved to final indices at the
/// flatten step.
#[derive(Clone, Copy)]
enum TreeRef {
    /// A finished convex leaf: index into [`Bsp::leaves`] (= final subsector index).
    Leaf(usize),
    /// An internal node: index into [`Bsp::tree_nodes`] (= final node index).
    Node(usize),
}

/// One internal BSP node in the tree arena, built in post-order (children first,
/// root last), so its arena index *is* its final `NODES` index.
struct TreeNode {
    /// Partition-line start `x` (the splitter seg's `v1`), map units.
    px: i32,
    /// Partition-line start `y`.
    py: i32,
    /// Partition-line `dx` = splitter `v2.x - v1.x` (fits `i16` by §B.1).
    dx: i32,
    /// Partition-line `dy`.
    dy: i32,
    /// The front (right) child.
    front: TreeRef,
    /// The back (left) child.
    back: TreeRef,
}

/// One step on the explicit build stack (Global Constraint 9: no call recursion).
enum Task {
    /// Partition (or make a leaf of) this set of seg ids.
    Split(Vec<usize>),
    /// Combine the two child results on the `done` stack into a node.
    Merge {
        /// Partition-line start `x`.
        px: i32,
        /// Partition-line start `y`.
        py: i32,
        /// Partition-line `dx`.
        dx: i32,
        /// Partition-line `dy`.
        dy: i32,
    },
}

/// The classic BSP builder's working state (spec §A–D).
struct Bsp<'a> {
    /// The source map (linedef/sidedef references resolve against it).
    map: &'a Map,
    /// Combined vertex coordinates: map vertices (narrowed `i16`, widened to
    /// `i32`) first, then split vertices — all whole map units.
    verts: Vec<(i32, i32)>,
    /// Count of leading map vertices in [`verts`](Self::verts); split-vertex
    /// indices start here (the [`BuiltNodes`] index-domain offset).
    map_vertex_count: usize,
    /// The split vertices created, in creation order (the output arena).
    split_vertices: Vec<MapVertex>,
    /// Exact-coordinate dedup over map *and* split vertices, seeded with the map
    /// vertices so a split landing on existing geometry reuses it (§C.3). Used
    /// for lookup only; never iterated, so it never touches an output ordering.
    dedup: HashMap<(i32, i32), usize>,
    /// The seg arena. Fragments are appended; abandoned split parents stay but
    /// are unreferenced.
    segs: Vec<WorkSeg>,
    /// Live seg count (segs reachable in some pending set or leaf). Each split
    /// nets `+1`; doubles as the termination backstop (Global Constraint 9).
    live_segs: usize,
    /// Strict or lenient (drives narrowing, ceilings, and the mixed-sector rule).
    strictness: Strictness,
    /// Partition heuristic: weight per straddling split (§B.3).
    split_cost: u32,
    /// Partition heuristic: axis-aligned preference divisor (`0` = no penalty).
    aa_preference: u32,
    /// Recovered lenient-mode warnings.
    warnings: Vec<NodeBuildWarning>,
    /// Convex leaves, in creation order = final subsector order. Each is a list
    /// of seg ids.
    leaves: Vec<Vec<usize>>,
    /// Internal nodes, in post-order (root last) = final `NODES` order.
    tree_nodes: Vec<TreeNode>,
    /// The tree root, set by [`partition`](Self::partition).
    root: Option<TreeRef>,
}

impl<'a> Bsp<'a> {
    /// Narrows the map's vertices through the shared write path (§A.2) and seeds
    /// the combined vertex table and dedup index.
    fn new(map: &'a Map, opts: &NodeBuildOptions) -> Result<Self, NodeBuildError> {
        // ADR-0024 §3: the identical narrowing pass the write path and BLOCKMAP
        // builder use. Strict failures surface as `Write(..)`; recoveries become
        // `NodeBuildWarning::Write`, never `NodesNotBuilt` (this narrower is not
        // seeded with it).
        let mut narrower = Narrower::new(opts.strictness);
        let arena = narrow_vertices(&mut narrower, map.vertices())?;
        let warnings: Vec<NodeBuildWarning> = narrower
            .warnings
            .into_iter()
            .map(NodeBuildWarning::Write)
            .collect();

        let mut verts = Vec::with_capacity(arena.len());
        let mut dedup: HashMap<(i32, i32), usize> = HashMap::with_capacity(arena.len());
        for (i, v) in arena.iter().enumerate() {
            let coord = (i32::from(v.x), i32::from(v.y));
            verts.push(coord);
            // First writer wins; a later duplicate map vertex simply never
            // becomes a dedup target. Deterministic (index order) and harmless:
            // dedup only ever redirects a *new* split vertex.
            dedup.entry(coord).or_insert(i);
        }
        let map_vertex_count = verts.len();

        Ok(Self {
            map,
            verts,
            map_vertex_count,
            split_vertices: Vec::new(),
            dedup,
            segs: Vec::new(),
            live_segs: 0,
            strictness: opts.strictness,
            split_cost: opts.split_cost,
            aa_preference: opts.aa_preference,
            warnings,
            leaves: Vec::new(),
            tree_nodes: Vec::new(),
            root: None,
        })
    }

    /// The combined-table coordinate of vertex `idx`.
    fn coord(&self, idx: usize) -> (i32, i32) {
        self.verts[idx]
    }

    /// §A.4: one seg per present linedef side. Right side → direction 0 (v1 =
    /// linedef start), left side → direction 1 (v1 = linedef end). Zero-length
    /// and fully-sideless linedefs contribute no segs (documented drop, not a
    /// warning). Initial segs have `offset == 0` (v1 *is* the start reference).
    fn build_initial_segs(&mut self) {
        for (li, ld) in self.map.linedefs().iter().enumerate() {
            let (a, b) = (ld.start.0, ld.end.0);
            // Zero-length after narrowing: no direction, engine derives nothing.
            if self.coord(a) == self.coord(b) {
                continue;
            }
            if let Some(side) = ld.right {
                let sector = self.map.sidedefs()[side.0].sector.0;
                self.segs.push(WorkSeg {
                    v1: a,
                    v2: b,
                    linedef: li,
                    direction: 0,
                    side_sector: sector,
                    offset: 0.0,
                });
            }
            if let Some(side) = ld.left {
                let sector = self.map.sidedefs()[side.0].sector.0;
                self.segs.push(WorkSeg {
                    v1: b,
                    v2: a,
                    linedef: li,
                    direction: 1,
                    side_sector: sector,
                    offset: 0.0,
                });
            }
        }
        self.live_segs = self.segs.len();
    }

    /// Drives the explicit work stack (§C.6): each pop either makes a convex leaf
    /// or emits a node frame plus two child sets. No call recursion anywhere.
    fn partition(&mut self) -> Result<(), NodeBuildError> {
        let root_set: Vec<usize> = (0..self.segs.len()).collect();
        let mut work: Vec<Task> = vec![Task::Split(root_set)];
        let mut done: Vec<TreeRef> = Vec::new();

        while let Some(task) = work.pop() {
            match task {
                Task::Split(set) => self.process_split(set, &mut work, &mut done)?,
                Task::Merge { px, py, dx, dy } => {
                    // Front was pushed last, so it completed first and sits
                    // beneath back on the `done` stack. unreachable panic: every
                    // `Merge` is pushed together with its two child `Split`s, each
                    // of which pushes exactly one `done` entry before this pops.
                    let back = done.pop().expect("merge back child present");
                    let front = done.pop().expect("merge front child present");
                    self.tree_nodes.push(TreeNode {
                        px,
                        py,
                        dx,
                        dy,
                        front,
                        back,
                    });
                    done.push(TreeRef::Node(self.tree_nodes.len() - 1));
                }
            }
        }

        self.root = done.pop();
        debug_assert!(done.is_empty(), "the build stack resolves to one root");
        Ok(())
    }

    /// Processes one `Split` task: select a partition (§B), else make a leaf,
    /// honoring the single-sector / mixed-sector rule (§C.1–C.2).
    fn process_split(
        &mut self,
        set: Vec<usize>,
        work: &mut Vec<Task>,
        done: &mut Vec<TreeRef>,
    ) -> Result<(), NodeBuildError> {
        if let Some(cand) = self.select(&set, false) {
            return self.branch(cand, &set, work);
        }
        // Convex: no seg's line has content on both sides (§B.4).
        if self.single_sector(&set) {
            self.push_leaf(set, done);
            return Ok(());
        }
        // §C.2: a multi-sector convex region. Retry with the sector-separating
        // relaxation; a separating line is a normal branch.
        if let Some(cand) = self.select(&set, true) {
            return self.branch(cand, &set, work);
        }
        // A mixed-sector fan: convex, multi-sector, and no seg line separates
        // the sectors (§C.2 relaxed retry above found none). Strict rejects it;
        // lenient accepts the leaf and warns once for it — the engine-tolerated
        // output the retail masters ship (ADR-0024 §7 amendment 2026-07-19).
        match self.strictness {
            Strictness::Strict => Err(NodeBuildError::MixedSectorSubsector {
                subsector_segs: set.len(),
            }),
            Strictness::Lenient => {
                self.warnings.push(NodeBuildWarning::MixedSectorSubsector {
                    subsector_segs: set.len(),
                });
                self.push_leaf(set, done);
                Ok(())
            }
        }
    }

    /// Emits a node for splitter `cand`: partitions `set` and pushes a `Merge`
    /// frame plus the two child `Split`s (front last, so it is processed first).
    fn branch(
        &mut self,
        cand: usize,
        set: &[usize],
        work: &mut Vec<Task>,
    ) -> Result<(), NodeBuildError> {
        let s = self.segs[cand];
        let (px, py) = self.coord(s.v1);
        let (x2, y2) = self.coord(s.v2);
        let (dx, dy) = (x2 - px, y2 - py);
        let part = Partition::new(px, py, dx, dy);

        let (front, back) = self.split_set(set, &part)?;
        if front.is_empty() || back.is_empty() {
            // `select` returns only a candidate whose `classify_seg` counts place
            // content on both sides, and `split_set` routes by that *same*
            // classification, so a valid selection cannot empty a side. This
            // guard therefore stays as a fuzz-safe backstop (Global Constraint 9):
            // rather than emit a degenerate node or loop on adversarial geometry
            // that somehow reaches here, fail cleanly in both modes. Well-formed
            // geometry — including the full retail collection — never trips it.
            return Err(NodeBuildError::DegeneratePartition {
                set_segs: set.len(),
            });
        }

        work.push(Task::Merge { px, py, dx, dy });
        work.push(Task::Split(back));
        work.push(Task::Split(front));
        Ok(())
    }

    /// Records `set` as the next convex subsector (leaf) and pushes its ref.
    fn push_leaf(&mut self, set: Vec<usize>, done: &mut Vec<TreeRef>) {
        self.leaves.push(set);
        done.push(TreeRef::Leaf(self.leaves.len() - 1));
    }

    /// Whether every seg in `set` faces the same sector (§C.1).
    fn single_sector(&self, set: &[usize]) -> bool {
        let first = self.segs[set[0]].side_sector;
        set.iter().all(|&id| self.segs[id].side_sector == first)
    }

    /// The exact `i64` cross product of the partition direction with the vector
    /// from the line start to vertex `e`: `> 0` front, `< 0` back (§B.2, engine
    /// convention `R_PointOnSide`, `src/doom/r_main.c:145`).
    fn cross(&self, part: &Partition, e: usize) -> i64 {
        let (qx, qy) = self.coord(e);
        (i64::from(qx) - part.pxi) * part.pdy - (i64::from(qy) - part.pyi) * part.pdx
    }

    /// Whether cross product `c` places its vertex **less than** 0.5 map units
    /// from the line (`distance² < 1/4 ⇔ cross² < len²/4 ⇔ 4·cross² < len²`;
    /// exact in `i128`). The inequality is strict: a vertex exactly 0.5 units
    /// off counts as front or back, not on the line.
    fn on_line(part: &Partition, c: i64) -> bool {
        i128::from(c) * i128::from(c) * 4 < part.len2
    }

    /// Classifies seg `s` against `part` (§B.2), folding in the §C.3
    /// endpoint-coincidence collapse so the result is **exactly** what
    /// [`split_set`](Self::split_set) will do: a straddler is [`Class::Split`]
    /// only when its rounded intersection is strictly interior, otherwise it
    /// classifies to the side it actually collapses to. This single source of
    /// truth is what keeps `select` and `split_set` in agreement.
    fn classify_seg(&self, part: &Partition, s: &WorkSeg) -> Class {
        let c1 = self.cross(part, s.v1);
        let c2 = self.cross(part, s.v2);
        let (on1, on2) = (Self::on_line(part, c1), Self::on_line(part, c2));
        let front = u8::from(!on1 && c1 > 0) + u8::from(!on2 && c2 > 0);
        let back = u8::from(!on1 && c1 < 0) + u8::from(!on2 && c2 < 0);

        if front > 0 && back > 0 {
            // Strict straddler. Compute where the split would actually land, on
            // the seg's own linedef geometry (§C.3).
            let Some((mx, my)) = self.intersection(s, part) else {
                // Parallel to its own linedef — impossible for a straddler; do
                // not divide by zero (Global Constraint 9).
                debug_assert!(
                    false,
                    "a straddling seg cannot be parallel to the partition"
                );
                return Class::Front;
            };
            let ec1 = self.coord(s.v1);
            let ec2 = self.coord(s.v2);
            // If the rounded split coincides with an endpoint, the seg does not
            // actually straddle after rounding — it collapses to the *other*
            // (non-coincident) endpoint's side.
            if (mx, my) == ec1 {
                return if c2 > 0 { Class::Front } else { Class::Back };
            }
            if (mx, my) == ec2 {
                return if c1 > 0 { Class::Front } else { Class::Back };
            }
            return Class::Split(mx, my);
        }
        if front > 0 {
            Class::Front
        } else if back > 0 {
            Class::Back
        } else {
            // Colinear: assign by direction (§B.2) — same direction as the
            // partition → front (the engine draws the front side first).
            let (sx1, sy1) = self.coord(s.v1);
            let (sx2, sy2) = self.coord(s.v2);
            let dot = part.pdx * (i64::from(sx2) - i64::from(sx1))
                + part.pdy * (i64::from(sy2) - i64::from(sy1));
            if dot > 0 {
                Class::ColinearFront
            } else {
                Class::ColinearBack
            }
        }
    }

    /// The rounded intersection of `part`'s line with seg `s`'s own linedef's
    /// canonical geometry (§C.3), or `None` if they are parallel. Computing on
    /// the linedef (not the seg's possibly-reversed `v1`/`v2`) is what makes a
    /// two-sided linedef's front and back segs split at the identical vertex.
    // The per-axis `f64` component pairs are conventional; renaming obscures them.
    #[allow(clippy::similar_names)]
    fn intersection(&self, s: &WorkSeg, part: &Partition) -> Option<(i32, i32)> {
        let ld = &self.map.linedefs()[s.linedef];
        let (lsx, lsy) = self.coord(ld.start.0);
        let (lex, ley) = self.coord(ld.end.0);
        let (ldx, ldy) = (f64::from(lex - lsx), f64::from(ley - lsy));
        let (pdxf, pdyf) = (f64::from(part.dx), f64::from(part.dy));
        let (lsxf, lsyf) = (f64::from(lsx), f64::from(lsy));
        let (pxf, pyf) = (f64::from(part.px), f64::from(part.py));

        // t = cross(part_dir, line_start - part_point) / -cross(part_dir, line_dir).
        let num = pdxf * (lsyf - pyf) - pdyf * (lsxf - pxf);
        let denom = -(pdxf * ldy - pdyf * ldx);
        if denom.abs() < 1e-9 {
            return None;
        }
        let t = num / denom;
        Some((
            round_half_away(lsxf + t * ldx),
            round_half_away(lsyf + t * ldy),
        ))
    }

    /// Selects the best splitter in `set` (§B), or `None` if the set is convex
    /// (no valid partition). `relaxed` switches to the sector-separating validity
    /// rule (§C.2). Ties break toward the lowest seg id (determinism).
    fn select(&self, set: &[usize], relaxed: bool) -> Option<usize> {
        let n = set.len();
        let stride = if n > SAMPLE_BUDGET {
            n.div_ceil(SAMPLE_BUDGET)
        } else {
            1
        };
        let mut best: Option<(u64, usize)> = None;
        self.eval_candidates(set, relaxed, (0..n).step_by(stride), &mut best);
        if best.is_none() && stride > 1 {
            // The sample found nothing; correctness requires the full pass.
            self.eval_candidates(set, relaxed, 0..n, &mut best);
        }
        best.map(|(_, id)| id)
    }

    /// Scores the candidates at `positions` in `set`, updating `best`.
    fn eval_candidates<I: IntoIterator<Item = usize>>(
        &self,
        set: &[usize],
        relaxed: bool,
        positions: I,
        best: &mut Option<(u64, usize)>,
    ) {
        for pos in positions {
            let cand = set[pos];
            let s = self.segs[cand];
            let (px, py) = self.coord(s.v1);
            let (x2, y2) = self.coord(s.v2);
            let pdx = i64::from(x2) - i64::from(px);
            let pdy = i64::from(y2) - i64::from(py);
            // §B.1: only a seg whose deltas fit the on-disk `i16` node field can
            // be a splitter (it still participates as content).
            if !partition_delta_fits(pdx, pdy) {
                continue;
            }
            #[allow(clippy::cast_possible_truncation)]
            let part = Partition::new(px, py, pdx as i32, pdy as i32);
            if part.len2 == 0 {
                continue; // a degenerate zero-length fragment cannot partition
            }
            // Full front/back counts (colinear included) drive scoring; the
            // non-colinear counts (`front_solid`/`back_solid`) drive the normal
            // convexity test (see `Class`). Because `classify_seg` reports the
            // *post-rounding* outcome, these counts match `split_set` exactly.
            let (mut nf, mut nb, mut nsp) = (0usize, 0usize, 0usize);
            let (mut front_solid, mut back_solid) = (0usize, 0usize);
            for &sid in set {
                match self.classify_seg(&part, &self.segs[sid]) {
                    Class::Front => {
                        nf += 1;
                        front_solid += 1;
                    }
                    Class::Back => {
                        nb += 1;
                        back_solid += 1;
                    }
                    Class::ColinearFront => nf += 1,
                    Class::ColinearBack => nb += 1,
                    Class::Split(..) => nsp += 1,
                }
            }
            let valid = if relaxed {
                // §C.2: a line that separates segs of different sectors — both
                // resulting sides non-empty, colinear segs counted (a two-sided
                // shared line's opposite colinear segs are what separate the
                // sectors).
                (nf + nsp) > 0 && (nb + nsp) > 0
            } else {
                // §B.3 (reconciled with the square-room fixture): a line that
                // genuinely partitions the set — a split, or NON-colinear content
                // on both sides. A splitter's own colinear seg does not, alone,
                // make its line a valid partition.
                nsp > 0 || (front_solid > 0 && back_solid > 0)
            };
            if !valid {
                continue;
            }
            let mut score = u64::from(self.split_cost) * nsp as u64 + nf.abs_diff(nb) as u64;
            if pdx != 0 && pdy != 0 && self.aa_preference > 0 {
                // Diagonal penalty (§B.3): a larger `aa_preference` is a weaker
                // penalty. Guarded against divide-by-zero.
                score += (nf + nb + nsp) as u64 / u64::from(self.aa_preference);
            }
            let better = match *best {
                None => true,
                Some((bscore, bid)) => score < bscore || (score == bscore && cand < bid),
            };
            if better {
                *best = Some((score, cand));
            }
        }
    }

    /// Partitions `set` by `part`, splitting straddling segs (§C.3–C.4).
    /// Routing uses the same [`classify_seg`](Self::classify_seg) that
    /// [`select`](Self::select) scored with, so a non-empty side here is
    /// guaranteed whenever the selector deemed the candidate valid. Returns
    /// `(front, back)`.
    fn split_set(
        &mut self,
        set: &[usize],
        part: &Partition,
    ) -> Result<(Vec<usize>, Vec<usize>), NodeBuildError> {
        let mut front = Vec::new();
        let mut back = Vec::new();
        for &sid in set {
            let s = self.segs[sid];
            match self.classify_seg(part, &s) {
                Class::Split(mx, my) => {
                    let (f, b) = self.split_seg_at(part, sid, mx, my)?;
                    front.push(f);
                    back.push(b);
                }
                class if class.is_front() => front.push(sid),
                _ => back.push(sid),
            }
        }
        Ok((front, back))
    }

    /// Splits straddling seg `sid` at the strictly-interior rounded intersection
    /// `(mx, my)` that [`classify_seg`](Self::classify_seg) already computed and
    /// vetted (§C.3–C.4), returning `(front_fragment, back_fragment)`. The mid
    /// vertex is interned (deduplicated), so a two-sided linedef's two segs share
    /// one split vertex — no crack.
    fn split_seg_at(
        &mut self,
        part: &Partition,
        sid: usize,
        mx: i32,
        my: i32,
    ) -> Result<(usize, usize), NodeBuildError> {
        let s = self.segs[sid];
        let ld = &self.map.linedefs()[s.linedef];
        // Offset reference: linedef start (dir 0) or end (dir 1) — §D.
        let ref_idx = if s.direction == 0 {
            ld.start.0
        } else {
            ld.end.0
        };
        let (rx, ry) = self.coord(ref_idx);
        let c1 = self.coord(s.v1);
        let mid = self.intern_vertex(mx, my);

        // Raw distances; narrowing to i16 is deferred to flatten time, where the
        // final seg index is known for the diagnostic (Finding 2, PR #319).
        let off_front = distance(rx, ry, c1.0, c1.1);
        let off_mid = distance(rx, ry, mx, my);

        // Fragment A spans s.v1 → mid; fragment B spans mid → s.v2. Each inherits
        // linedef/direction/sector (§C.4).
        let frag_a = WorkSeg {
            v1: s.v1,
            v2: mid,
            offset: off_front,
            ..s
        };
        let frag_b = WorkSeg {
            v1: mid,
            v2: s.v2,
            offset: off_mid,
            ..s
        };
        let id_a = self.segs.len();
        self.segs.push(frag_a);
        let id_b = self.segs.len();
        self.segs.push(frag_b);

        // Each split nets one live seg (parent replaced by two fragments); this
        // is the runaway backstop (Global Constraint 9).
        self.live_segs += 1;
        if self.live_segs > MAX_U16_INDEXED {
            return Err(NodeBuildError::TooManyElements {
                kind: "segs",
                count: self.live_segs,
                max: MAX_U16_INDEXED,
            });
        }

        // Fragment A takes the side of s.v1; B takes the side of s.v2. As a
        // straddler they are on opposite sides.
        if self.cross(part, s.v1) > 0 {
            Ok((id_a, id_b))
        } else {
            Ok((id_b, id_a))
        }
    }

    /// Interns a split vertex, reusing an existing map or split vertex with the
    /// same exact integer coordinate (§C.3). Returns its combined-table index.
    fn intern_vertex(&mut self, x: i32, y: i32) -> usize {
        if let Some(&idx) = self.dedup.get(&(x, y)) {
            return idx;
        }
        let idx = self.verts.len();
        self.verts.push((x, y));
        self.split_vertices.push(MapVertex {
            x: f64::from(x),
            y: f64::from(y),
        });
        self.dedup.insert((x, y), idx);
        idx
    }

    /// Flattens the tree arena into [`BuiltNodes`]: final segs (contiguous per
    /// subsector), subsectors (creation order), nodes (post-order, root last),
    /// with child bboxes computed bottom-up (§C.4, §D). Enforces the arena
    /// ceilings (Global Constraint 6).
    fn finish(mut self) -> Result<(BuiltNodes, Vec<NodeBuildWarning>), NodeBuildError> {
        // Per-leaf bboxes [top, bottom, left, right] = [max_y, min_y, min_x, max_x].
        let leaf_bboxes: Vec<[i32; 4]> = self
            .leaves
            .iter()
            .map(|leaf| self.bbox_of_segs(leaf))
            .collect();

        // Node bboxes bottom-up: post-order guarantees children precede parents.
        let mut node_bboxes: Vec<[i32; 4]> = Vec::with_capacity(self.tree_nodes.len());
        for tn in &self.tree_nodes {
            let fb = bbox_of_ref(tn.front, &leaf_bboxes, &node_bboxes);
            let bb = bbox_of_ref(tn.back, &leaf_bboxes, &node_bboxes);
            node_bboxes.push(bbox_union(fb, bb));
        }

        // Final segs, subsectors owning contiguous runs (Global Constraint 5).
        let mut segs: Vec<MapSeg> = Vec::with_capacity(self.live_segs);
        let mut subsectors: Vec<MapSubsector> = Vec::with_capacity(self.leaves.len());
        let mut offset_warnings: Vec<NodeBuildWarning> = Vec::new();
        for leaf in &self.leaves {
            let start = segs.len();
            for &sid in leaf {
                let s = self.segs[sid];
                let (x1, y1) = self.coord(s.v1);
                let (x2, y2) = self.coord(s.v2);
                // Narrow the raw offset now that the seg's real final index is
                // known (Finding 2, PR #319): the diagnostic names *this* seg.
                let (offset, warning) = finalize_offset(s.offset, segs.len(), self.strictness)?;
                if let Some(warning) = warning {
                    offset_warnings.push(warning);
                }
                segs.push(MapSeg {
                    start: VertexIdx(s.v1),
                    end: VertexIdx(s.v2),
                    angle: bam_angle(x2 - x1, y2 - y1),
                    linedef: LinedefIdx(s.linedef),
                    direction: s.direction,
                    offset,
                });
            }
            // Finding 1 (PR #319): a subsector's `seg_count` is a `u16` on disk,
            // so a convex leaf of > 65,535 segs is structurally unencodable —
            // reject here (both modes), not only later in `to_lump_bytes`.
            check_subsector_seg_count(segs.len() - start)?;
            subsectors.push(MapSubsector {
                segs: start..segs.len(),
                leafs: 0..0,
            });
        }
        self.warnings.extend(offset_warnings);

        // Final nodes: front = child[0] = right; back = left (Global Constraint 7).
        let mut nodes: Vec<MapNode> = Vec::with_capacity(self.tree_nodes.len());
        for (j, tn) in self.tree_nodes.iter().enumerate() {
            nodes.push(MapNode {
                x: tn.px,
                y: tn.py,
                dx: tn.dx,
                dy: tn.dy,
                right_bbox: bbox_of_ref(tn.front, &leaf_bboxes, &node_bboxes),
                left_bbox: bbox_of_ref(tn.back, &leaf_bboxes, &node_bboxes),
                right: child_of_ref(tn.front),
                left: child_of_ref(tn.back),
            });
            debug_assert!(j < self.tree_nodes.len());
        }

        // Global Constraint 6 ceilings, enforced in `build_nodes` too (not only
        // the serializer). Vertex/seg soft ceiling warns in lenient mode.
        let vertex_count = self.map_vertex_count + self.split_vertices.len();
        self.check_soft_ceiling("vertices", vertex_count)?;
        self.check_soft_ceiling("segs", segs.len())?;
        check_hard_index_ceiling("subsectors", subsectors.len())?;
        check_hard_index_ceiling("nodes", nodes.len())?;

        let built = BuiltNodes {
            split_vertices: self.split_vertices,
            segs,
            subsectors,
            nodes,
        };
        // The post-order flatten must place the root last (or, for a single
        // convex subsector, leave `nodes` empty).
        debug_assert!(match self.root {
            Some(TreeRef::Node(k)) => k + 1 == built.nodes.len(),
            Some(TreeRef::Leaf(_)) => built.nodes.is_empty(),
            None => built.subsectors.is_empty(),
        });
        Ok((built, self.warnings))
    }

    /// The bbox `[top, bottom, left, right]` enclosing every endpoint of `seg_ids`.
    fn bbox_of_segs(&self, seg_ids: &[usize]) -> [i32; 4] {
        let mut bbox = [i32::MIN, i32::MAX, i32::MAX, i32::MIN];
        for &sid in seg_ids {
            let s = self.segs[sid];
            for &e in &[s.v1, s.v2] {
                let (x, y) = self.coord(e);
                bbox[0] = bbox[0].max(y); // top    = max_y
                bbox[1] = bbox[1].min(y); // bottom = min_y
                bbox[2] = bbox[2].min(x); // left   = min_x
                bbox[3] = bbox[3].max(x); // right  = max_x
            }
        }
        bbox
    }

    /// Vertex/seg soft ceiling (Global Constraint 6), pushing the lenient warning
    /// if any; see [`soft_ceiling`] for the rule.
    fn check_soft_ceiling(
        &mut self,
        kind: &'static str,
        count: usize,
    ) -> Result<(), NodeBuildError> {
        if let Some(warning) = soft_ceiling(kind, count, self.strictness)? {
            self.warnings.push(warning);
        }
        Ok(())
    }
}

/// The vertex/seg soft ceiling rule (Global Constraint 6): a count above 65,536
/// is a structural [`NodeBuildError::TooManyElements`] in **both** modes; above
/// 32,768 (the vanilla array limit) is a strict [`NodeBuildError::TooManyElements`]
/// and a lenient [`NodeBuildWarning::VanillaCeilingExceeded`] (returned for the
/// caller to record). Below both, `Ok(None)`.
fn soft_ceiling(
    kind: &'static str,
    count: usize,
    strictness: Strictness,
) -> Result<Option<NodeBuildWarning>, NodeBuildError> {
    if count > MAX_U16_INDEXED {
        return Err(NodeBuildError::TooManyElements {
            kind,
            count,
            max: MAX_U16_INDEXED,
        });
    }
    if count > VANILLA_CEILING {
        return match strictness {
            Strictness::Strict => Err(NodeBuildError::TooManyElements {
                kind,
                count,
                max: VANILLA_CEILING,
            }),
            Strictness::Lenient => Ok(Some(NodeBuildWarning::VanillaCeilingExceeded {
                kind,
                count,
                max: VANILLA_CEILING,
            })),
        };
    }
    Ok(None)
}

/// Rounds half away from zero to the nearest whole map unit (the write path's
/// rounding), returning `i32`. Inputs are bounded map coordinates, so the cast
/// cannot overflow.
#[allow(clippy::cast_possible_truncation)]
fn round_half_away(value: f64) -> i32 {
    value.round() as i32
}

/// Euclidean distance between two integer points as `f64` (IEEE `sqrt` is
/// correctly rounded and deterministic — Global Constraint 8).
fn distance(ax: i32, ay: i32, bx: i32, by: i32) -> f64 {
    f64::from(ax - bx).hypot(f64::from(ay - by))
}

/// The BAM angle of the vector `(dx, dy)` (§D): `atan2(dy, dx) / TAU * 65536`,
/// rounded and wrapped into `u16`. Axis-aligned and 45° directions are exact.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn bam_angle(dx: i32, dy: i32) -> u16 {
    let radians = f64::from(dy).atan2(f64::from(dx));
    let scaled = radians / std::f64::consts::TAU * 65536.0;
    // `scaled` is within (-32768, 32768]; round then wrap into 0..65536.
    (scaled.round() as i64).rem_euclid(65536) as u16
}

/// The bbox of a child ref, from the already-computed leaf/node bbox tables.
fn bbox_of_ref(child: TreeRef, leaf_bboxes: &[[i32; 4]], node_bboxes: &[[i32; 4]]) -> [i32; 4] {
    match child {
        TreeRef::Leaf(i) => leaf_bboxes[i],
        TreeRef::Node(k) => node_bboxes[k],
    }
}

/// The `[top, bottom, left, right]` union of two bboxes.
fn bbox_union(a: [i32; 4], b: [i32; 4]) -> [i32; 4] {
    [
        a[0].max(b[0]),
        a[1].min(b[1]),
        a[2].min(b[2]),
        a[3].max(b[3]),
    ]
}

/// The [`NodeChild`] a tree ref resolves to.
fn child_of_ref(child: TreeRef) -> NodeChild {
    match child {
        TreeRef::Leaf(i) => NodeChild::Subsector(SubsectorIdx(i)),
        TreeRef::Node(k) => NodeChild::Node(NodeIdx(k)),
    }
}

/// The structural subsector/node index ceiling (Global Constraint 6): a BSP
/// child reference reserves bit 15, so at most 32,768 elements are addressable —
/// a hard [`NodeBuildError::TooManyElements`] in **both** modes.
fn check_hard_index_ceiling(kind: &'static str, count: usize) -> Result<(), NodeBuildError> {
    if count > MAX_BSP_INDEX {
        return Err(NodeBuildError::TooManyElements {
            kind,
            count,
            max: MAX_BSP_INDEX,
        });
    }
    Ok(())
}

/// A subsector's on-disk `seg_count` is a `u16`, so a convex leaf of more than
/// 65,535 segs is structurally unencodable — a [`NodeBuildError::TooManyElements`]
/// (`kind: "subsector segs"`) in **both** modes (Finding 1, PR #319). Reachable
/// only at the seg-ceiling boundary: up to 65,536 segs collapsed into a single
/// convex leaf in lenient mode.
fn check_subsector_seg_count(count: usize) -> Result<(), NodeBuildError> {
    if count > usize::from(u16::MAX) {
        return Err(NodeBuildError::TooManyElements {
            kind: "subsector segs",
            count,
            max: usize::from(u16::MAX),
        });
    }
    Ok(())
}

/// The map's linedef count must fit the `Seg.linedef` on-disk `u16`: a seg
/// referencing a linedef index past `u16::MAX` (i.e. a `> MAX_U16_INDEXED`
/// arena) is unencodable, so a successful `build_nodes` would otherwise fail
/// only later in [`BuiltNodes::to_lump_bytes`]. Rejected up front in **both**
/// modes (structural — no lenient recovery). No retail map approaches this
/// (the maximum is ~7,245 linedefs); PR #319.
fn check_linedef_count(count: usize) -> Result<(), NodeBuildError> {
    if count > MAX_U16_INDEXED {
        return Err(NodeBuildError::TooManyElements {
            kind: "linedefs",
            count,
            max: MAX_U16_INDEXED,
        });
    }
    Ok(())
}

/// Narrows a computed seg `offset` to the on-disk `i16` at flatten time (§D,
/// Finding 2 PR #319). `index` is the seg's **final** index (not its linedef),
/// so the diagnostic names the exact seg. Strict returns a write-path
/// [`DoomWriteError::ValueOutOfRange`]; lenient clamps and returns a
/// [`DoomWriteWarning::ValueClamped`] warning for the caller to record — so a
/// `build_nodes` product never trips [`BuiltNodes::to_lump_bytes`]'s defensive
/// offset check. A well-formed offset (≤ the linedef length) overflows only on a
/// linedef longer than 32,767 units.
fn finalize_offset(
    raw: f64,
    index: usize,
    strictness: Strictness,
) -> Result<(i32, Option<NodeBuildWarning>), NodeBuildError> {
    let rounded = round_half_away(raw);
    if let Ok(v) = i16::try_from(rounded) {
        return Ok((i32::from(v), None));
    }
    match strictness {
        Strictness::Strict => Err(NodeBuildError::Write(DoomWriteError::ValueOutOfRange {
            block: "seg",
            field: "offset",
            index,
            value: i64::from(rounded),
        })),
        Strictness::Lenient => {
            let clamped = rounded.clamp(i32::from(i16::MIN), i32::from(i16::MAX));
            let warning = NodeBuildWarning::Write(DoomWriteWarning::ValueClamped {
                block: "seg",
                field: "offset",
                index,
                from: i64::from(rounded),
                to: i64::from(clamped),
            });
            Ok((clamped, Some(warning)))
        }
    }
}

// These are public-API tests that would ordinarily live in
// `tests/build_lumps.rs` (the house unit-vs-integration convention), but
// `BuiltNodes` is `#[non_exhaustive]`: an integration test is a separate
// crate, where struct-literal construction is rejected (E0639), and no public
// producer exists until stage 2's `build_nodes` lands. They are therefore
// unit tests by necessity, not by preference — once `build_nodes` exists,
// integration tests can exercise `to_lump_bytes` through its output.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::graph::{LinedefIdx, NodeIdx, SubsectorIdx, VertexIdx};
    use crate::map::{Seg as SegRecord, Subsector as SubsectorRecord, parse_records};

    /// A one-sided square-room seg (direction 0, offset 0, axis-aligned angle).
    fn seg(start: usize, end: usize, angle: u16, linedef: usize) -> MapSeg {
        MapSeg {
            start: VertexIdx(start),
            end: VertexIdx(end),
            angle,
            linedef: LinedefIdx(linedef),
            direction: 0,
            offset: 0,
        }
    }

    /// The controller-verified square-room fixture: four one-sided segs, one
    /// convex subsector, no split vertices, no nodes.
    fn square_room() -> BuiltNodes {
        BuiltNodes {
            split_vertices: Vec::new(),
            // (0,0)->(128,0) east 0x0000, (128,0)->(128,128) north 0x4000,
            // (128,128)->(0,128) west 0x8000, (0,128)->(0,0) south 0xC000.
            segs: vec![
                seg(0, 1, 0x0000, 0),
                seg(1, 2, 0x4000, 1),
                seg(2, 3, 0x8000, 2),
                seg(3, 0, 0xC000, 3),
            ],
            subsectors: vec![MapSubsector {
                segs: 0..4,
                leafs: 0..0,
            }],
            nodes: Vec::new(),
        }
    }

    #[test]
    fn square_room_matches_controller_bytes() {
        let lumps = square_room().to_lump_bytes().expect("serializes");

        // split_vertexes and nodes empty; single-convex-subsector => numnodes 0.
        assert!(lumps.split_vertexes.is_empty());
        assert!(lumps.nodes.is_empty());

        // ssectors: one record, seg_count 4, first_seg 0.
        assert_eq!(lumps.ssectors, vec![0x04, 0x00, 0x00, 0x00]);

        // segs: four 12-byte records; seg 0 verified byte-for-byte.
        assert_eq!(lumps.segs.len(), 4 * 12);
        assert_eq!(
            &lumps.segs[..12],
            &[
                0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
            ],
        );
    }

    #[test]
    fn square_room_round_trips_through_parse_records() {
        let lumps = square_room().to_lump_bytes().expect("serializes");

        let segs: Vec<SegRecord> = parse_records(&lumps.segs).expect("segs parse");
        assert_eq!(segs.len(), 4);
        assert_eq!(segs[0].start_vertex, 0);
        assert_eq!(segs[0].end_vertex, 1);
        assert_eq!(segs[0].angle, 0x0000);
        assert_eq!(segs[1].angle, 0x4000);
        assert_eq!(segs[0].linedef, 0);
        assert_eq!(segs[0].direction, 0);
        assert_eq!(segs[0].offset, 0);

        let ssectors: Vec<SubsectorRecord> =
            parse_records(&lumps.ssectors).expect("ssectors parse");
        assert_eq!(ssectors.len(), 1);
        assert_eq!(ssectors[0].seg_count, 4);
        assert_eq!(ssectors[0].first_seg, 0);
    }

    #[test]
    fn leaf_child_encodes_the_subsector_flag() {
        // One node whose right child is subsector 3 and left child is node 0.
        let nodes = BuiltNodes {
            split_vertices: Vec::new(),
            segs: vec![seg(0, 1, 0x0000, 0)],
            subsectors: vec![
                MapSubsector {
                    segs: 0..1,
                    leafs: 0..0,
                };
                4
            ],
            nodes: vec![
                MapNode {
                    x: 0,
                    y: 0,
                    dx: 128,
                    dy: 0,
                    right_bbox: [1, 2, 3, 4],
                    left_bbox: [5, 6, 7, 8],
                    right: NodeChild::Subsector(SubsectorIdx(3)),
                    left: NodeChild::Node(NodeIdx(0)),
                };
                1
            ],
        };
        let lumps = nodes.to_lump_bytes().expect("serializes");

        // NODES record is 28 bytes; the two child words are the last 4 bytes.
        assert_eq!(lumps.nodes.len(), 28);
        let right_child = u16::from_le_bytes([lumps.nodes[24], lumps.nodes[25]]);
        let left_child = u16::from_le_bytes([lumps.nodes[26], lumps.nodes[27]]);
        assert_eq!(right_child, NF_SUBSECTOR | 3, "leaf child sets bit 15");
        assert_eq!(left_child, 0, "internal-node child is the bare index");
    }

    #[test]
    fn too_many_subsectors_errors_in_both_modes() {
        // 32,769 empty-range subsectors exceed the 32,768 structural ceiling.
        let over = MAX_BSP_INDEX + 1;
        let built = BuiltNodes {
            split_vertices: Vec::new(),
            segs: Vec::new(),
            subsectors: vec![
                MapSubsector {
                    segs: 0..0,
                    leafs: 0..0,
                };
                over
            ],
            nodes: Vec::new(),
        };
        assert_eq!(
            built.to_lump_bytes().unwrap_err(),
            NodeBuildError::TooManyElements {
                kind: "subsectors",
                count: over,
                max: MAX_BSP_INDEX,
            },
        );
    }

    #[test]
    fn offset_out_of_i16_range_is_a_value_out_of_range_error() {
        let mut s = seg(0, 1, 0x0000, 0);
        s.offset = i32::from(i16::MAX) + 1;
        let built = BuiltNodes {
            split_vertices: Vec::new(),
            segs: vec![s],
            subsectors: vec![MapSubsector {
                segs: 0..1,
                leafs: 0..0,
            }],
            nodes: Vec::new(),
        };
        assert_eq!(
            built.to_lump_bytes().unwrap_err(),
            NodeBuildError::Write(DoomWriteError::ValueOutOfRange {
                block: "seg",
                field: "offset",
                index: 0,
                value: i64::from(i16::MAX) + 1,
            }),
        );
    }

    #[test]
    fn split_vertex_out_of_i16_range_is_a_value_out_of_range_error() {
        let built = BuiltNodes {
            split_vertices: vec![MapVertex {
                x: 40_000.0,
                y: 0.0,
            }],
            segs: Vec::new(),
            subsectors: Vec::new(),
            nodes: Vec::new(),
        };
        assert_eq!(
            built.to_lump_bytes().unwrap_err(),
            NodeBuildError::Write(DoomWriteError::ValueOutOfRange {
                block: "vertex",
                field: "x",
                index: 0,
                value: 40_000,
            }),
        );
    }

    #[test]
    fn split_vertices_serialize_as_vertexes_records() {
        use crate::map::Vertex as VertexRecord;

        let built = BuiltNodes {
            split_vertices: vec![MapVertex { x: 64.0, y: -32.0 }],
            segs: Vec::new(),
            subsectors: Vec::new(),
            nodes: Vec::new(),
        };
        let lumps = built.to_lump_bytes().expect("serializes");
        assert_eq!(lumps.split_vertexes.len(), 4);
        let verts: Vec<VertexRecord> =
            parse_records(&lumps.split_vertexes).expect("vertexes parse");
        assert_eq!(verts, vec![VertexRecord { x: 64, y: -32 }]);
    }

    #[test]
    fn round_half_away_matches_the_write_path() {
        assert_eq!(round_half_away(0.5), 1);
        assert_eq!(round_half_away(-0.5), -1);
        assert_eq!(round_half_away(2.4), 2);
        assert_eq!(round_half_away(-2.6), -3);
        assert_eq!(round_half_away(0.0), 0);
    }

    #[test]
    fn distance_is_euclidean() {
        assert!((distance(0, 0, 3, 4) - 5.0).abs() < 1e-9);
        assert!(distance(10, 10, 10, 10).abs() < 1e-9);
        // 64 units straight east: exact.
        assert!((distance(0, 0, 64, 0) - 64.0).abs() < 1e-9);
    }

    #[test]
    fn bam_angle_is_exact_for_axis_aligned_and_45() {
        // The controller square-room angles.
        assert_eq!(bam_angle(1, 0), 0x0000); // east
        assert_eq!(bam_angle(0, 1), 0x4000); // north
        assert_eq!(bam_angle(-1, 0), 0x8000); // west
        assert_eq!(bam_angle(0, -1), 0xC000); // south
        // 45° diagonals are exact too (Global Constraint 8).
        assert_eq!(bam_angle(1, 1), 0x2000);
        assert_eq!(bam_angle(-1, 1), 0x6000);
        assert_eq!(bam_angle(-1, -1), 0xA000);
        assert_eq!(bam_angle(1, -1), 0xE000);
    }

    /// The vertex/seg soft ceiling is the sanctioned unit seam for the
    /// over-32,768 / over-65,536 tests: a live 33k-seg *convex* map would make
    /// the partition search O(n²) and blow the time budget, so the threshold
    /// logic is exercised here directly.
    #[test]
    fn soft_ceiling_thresholds() {
        // Under both ceilings: clean.
        assert_eq!(soft_ceiling("segs", 32_768, Strictness::Strict), Ok(None));

        // Over the vanilla (32,768) ceiling: strict errors, lenient warns.
        assert_eq!(
            soft_ceiling("segs", 32_769, Strictness::Strict),
            Err(NodeBuildError::TooManyElements {
                kind: "segs",
                count: 32_769,
                max: VANILLA_CEILING,
            })
        );
        assert_eq!(
            soft_ceiling("vertices", 40_000, Strictness::Lenient),
            Ok(Some(NodeBuildWarning::VanillaCeilingExceeded {
                kind: "vertices",
                count: 40_000,
                max: VANILLA_CEILING,
            }))
        );

        // Over the structural (65,536) ceiling: TooManyElements in BOTH modes.
        for strictness in [Strictness::Strict, Strictness::Lenient] {
            assert_eq!(
                soft_ceiling("segs", 65_537, strictness),
                Err(NodeBuildError::TooManyElements {
                    kind: "segs",
                    count: 65_537,
                    max: MAX_U16_INDEXED,
                })
            );
        }
    }

    #[test]
    fn hard_index_ceiling_rejects_over_32768_in_both_conceptual_modes() {
        assert_eq!(check_hard_index_ceiling("nodes", MAX_BSP_INDEX), Ok(()));
        assert_eq!(
            check_hard_index_ceiling("subsectors", MAX_BSP_INDEX + 1),
            Err(NodeBuildError::TooManyElements {
                kind: "subsectors",
                count: MAX_BSP_INDEX + 1,
                max: MAX_BSP_INDEX,
            })
        );
    }

    #[test]
    fn subsector_seg_count_ceiling_rejects_over_u16_max() {
        // The exact boundary: 65,535 is encodable, 65,536 is not (Finding 1).
        let max = usize::from(u16::MAX);
        assert_eq!(check_subsector_seg_count(max), Ok(()));
        assert_eq!(
            check_subsector_seg_count(max + 1),
            Err(NodeBuildError::TooManyElements {
                kind: "subsector segs",
                count: max + 1,
                max,
            })
        );
    }

    #[test]
    fn partition_delta_fits_the_full_signed_i16_range() {
        // i16::MIN (-32,768) is a valid on-disk delta; +32,768 is not.
        assert!(partition_delta_fits(-32_768, 0));
        assert!(partition_delta_fits(0, -32_768));
        assert!(partition_delta_fits(32_767, -32_768));
        assert!(!partition_delta_fits(-32_769, 0));
        assert!(!partition_delta_fits(0, 32_768));
    }

    #[test]
    fn linedef_count_ceiling_rejects_over_u16_indexable() {
        // 65,536 linedefs index 0..=65,535 (all fit the u16 seg field); 65,537
        // would need index 65,536, which does not (PR #319).
        assert_eq!(check_linedef_count(MAX_U16_INDEXED), Ok(()));
        assert_eq!(
            check_linedef_count(MAX_U16_INDEXED + 1),
            Err(NodeBuildError::TooManyElements {
                kind: "linedefs",
                count: MAX_U16_INDEXED + 1,
                max: MAX_U16_INDEXED,
            })
        );
    }

    #[test]
    fn finalize_offset_narrows_at_the_i16_boundary() {
        // In range: no warning, value passes through (rounded half away).
        assert_eq!(
            finalize_offset(100.5, 7, Strictness::Strict).unwrap(),
            (101, None)
        );

        // Strict overflow: a write-path ValueOutOfRange naming the SEG index.
        let over = f64::from(i16::MAX) + 1.0;
        assert_eq!(
            finalize_offset(over, 42, Strictness::Strict).unwrap_err(),
            NodeBuildError::Write(DoomWriteError::ValueOutOfRange {
                block: "seg",
                field: "offset",
                index: 42,
                value: i64::from(i16::MAX) + 1,
            })
        );

        // Lenient overflow: clamp to i16::MAX and a ValueClamped warning on the
        // same seg index.
        let (value, warning) = finalize_offset(over, 42, Strictness::Lenient).unwrap();
        assert_eq!(value, i32::from(i16::MAX));
        assert_eq!(
            warning,
            Some(NodeBuildWarning::Write(DoomWriteWarning::ValueClamped {
                block: "seg",
                field: "offset",
                index: 42,
                from: i64::from(i16::MAX) + 1,
                to: i64::from(i16::MAX),
            }))
        );
    }

    #[test]
    fn encode_index_rejects_over_u16() {
        assert_eq!(encode_index(0, "vertices").unwrap(), 0);
        assert_eq!(encode_index(0xFFFF, "vertices").unwrap(), 0xFFFF);
        // 0x1_0000 does not fit u16; the reported count is idx + 1.
        assert_eq!(
            encode_index(0x1_0000, "vertices").unwrap_err(),
            NodeBuildError::TooManyElements {
                kind: "vertices",
                count: 0x1_0001,
                max: MAX_U16_INDEXED,
            }
        );
    }

    #[test]
    fn encode_count_rejects_over_u16() {
        assert_eq!(encode_count(0xFFFF, "subsector segs").unwrap(), 0xFFFF);
        // A count (not an index) reports itself, ceiling u16::MAX.
        assert_eq!(
            encode_count(0x1_0000, "subsector segs").unwrap_err(),
            NodeBuildError::TooManyElements {
                kind: "subsector segs",
                count: 0x1_0000,
                max: usize::from(u16::MAX),
            }
        );
    }

    #[test]
    fn to_lump_bytes_rejects_out_of_range_node_fields() {
        // Defensive serializer guards for a hand-constructed BuiltNodes:
        // `build_nodes` never produces these (its node coords/bboxes are narrowed
        // and its child indices bounded), so they guard only in-crate callers.
        let base = MapNode {
            x: 0,
            y: 0,
            dx: 1,
            dy: 0,
            right_bbox: [0; 4],
            left_bbox: [0; 4],
            right: NodeChild::Subsector(SubsectorIdx(0)),
            left: NodeChild::Subsector(SubsectorIdx(0)),
        };
        let one_sub = vec![MapSubsector {
            segs: 0..0,
            leafs: 0..0,
        }];
        let build = |node: MapNode| BuiltNodes {
            split_vertices: Vec::new(),
            segs: Vec::new(),
            subsectors: one_sub.clone(),
            nodes: vec![node],
        };

        // A node partition coordinate out of i16 range.
        let mut node = base;
        node.x = 40_000;
        assert_eq!(
            build(node).to_lump_bytes().unwrap_err(),
            NodeBuildError::Write(DoomWriteError::ValueOutOfRange {
                block: "node",
                field: "x",
                index: 0,
                value: 40_000,
            })
        );

        // A child bounding-box component out of i16 range.
        let mut node = base;
        node.right_bbox = [40_000, 0, 0, 0];
        assert_eq!(
            build(node).to_lump_bytes().unwrap_err(),
            NodeBuildError::Write(DoomWriteError::ValueOutOfRange {
                block: "node",
                field: "right_bbox",
                index: 0,
                value: 40_000,
            })
        );

        // A child index that leaves no room for the NF_SUBSECTOR flag bit.
        let mut node = base;
        node.right = NodeChild::Subsector(SubsectorIdx(MAX_BSP_INDEX));
        assert_eq!(
            build(node).to_lump_bytes().unwrap_err(),
            NodeBuildError::TooManyElements {
                kind: "subsectors",
                count: MAX_BSP_INDEX + 1,
                max: MAX_BSP_INDEX,
            }
        );
    }

    #[test]
    fn to_lump_bytes_rejects_seg_vertex_index_over_u16() {
        // A seg whose start vertex index does not fit u16 — the defensive
        // `encode_index` guard reached through the serializer.
        let mut s = seg(0, 1, 0x0000, 0);
        s.start = VertexIdx(0x1_0000);
        let built = BuiltNodes {
            split_vertices: Vec::new(),
            segs: vec![s],
            subsectors: vec![MapSubsector {
                segs: 0..1,
                leafs: 0..0,
            }],
            nodes: Vec::new(),
        };
        assert_eq!(
            built.to_lump_bytes().unwrap_err(),
            NodeBuildError::TooManyElements {
                kind: "vertices",
                count: 0x1_0001,
                max: MAX_U16_INDEXED,
            }
        );
    }

    #[test]
    fn too_many_nodes_errors_in_both_modes() {
        // 32,769 nodes exceed the 15-bit child-reference ceiling; the nodes arm
        // of `to_lump_bytes` is reached only when subsectors are within bounds.
        let over = MAX_BSP_INDEX + 1;
        let built = BuiltNodes {
            split_vertices: Vec::new(),
            segs: Vec::new(),
            subsectors: vec![MapSubsector {
                segs: 0..0,
                leafs: 0..0,
            }],
            nodes: vec![
                MapNode {
                    x: 0,
                    y: 0,
                    dx: 1,
                    dy: 0,
                    right_bbox: [0; 4],
                    left_bbox: [0; 4],
                    right: NodeChild::Subsector(SubsectorIdx(0)),
                    left: NodeChild::Subsector(SubsectorIdx(0)),
                };
                over
            ],
        };
        assert_eq!(
            built.to_lump_bytes().unwrap_err(),
            NodeBuildError::TooManyElements {
                kind: "nodes",
                count: over,
                max: MAX_BSP_INDEX,
            }
        );
    }
}
