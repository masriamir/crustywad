//! The classic BSP pass's output types and their lump serialization
//! (ADR-0024 §2, staging §9.2, issue #315).
//!
//! [`BuiltNodes`] is what the (Task 2) kernel `build_nodes` produces: the split
//! vertices it created, plus the `SEGS`, `SSECTORS`, and `NODES` arenas of a
//! finished classic BSP tree, expressed in the same normalized graph types the
//! reader assembles into ([`MapSeg`], [`MapSubsector`], [`MapNode`]).
//! [`BuiltNodes::to_lump_bytes`] renders those arenas to the four on-disk lumps
//! the engine reads, reusing the [`common`](crate::map::common) record structs
//! so the byte layout is declared exactly once.
//!
//! This module deliberately carries **only** the output types and their
//! serializer: it can be — and is — unit-tested against hand-constructed
//! [`BuiltNodes`] values before the partitioning kernel exists.
//!
//! [`Map`]: crate::map::Map

use std::io::Cursor;

use binrw::BinWriterExt;

use crate::map::DoomWriteError;
use crate::map::build::NodeBuildError;
use crate::map::common::{Node, Seg, Subsector, Vertex};
use crate::map::graph::{MapNode, MapSeg, MapSubsector, MapVertex, NodeChild};

/// The BSP child-reference leaf flag (`NF_SUBSECTOR`): with bit 15 set the
/// remaining 15 bits are a subsector index, otherwise a node index (Chocolate
/// Doom `doomdata.h`).
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
    ///   [`NodeBuildError::Write`] for a vertex/linedef/seg index or node
    ///   coordinate that does not fit its `u16`/`i16` on-disk field. `build_nodes`
    ///   narrows coordinates and bounds indices before constructing a
    ///   [`BuiltNodes`], so these guard only hand-constructed values.
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

        let vertexes: Vec<Vertex> = self
            .split_vertices
            .iter()
            .map(|v| Vertex {
                x: narrow_vertex_coord(v.x),
                y: narrow_vertex_coord(v.y),
            })
            .collect();

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
                seg_count: encode_index(count, "segs")?,
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

/// Narrows a split-vertex `f64` coordinate to the on-disk `i16`. `build_nodes`
/// stores whole, in-range map units, so this rounds (half away from zero) and
/// clamps defensively — non-panicking on any hand-constructed value.
fn narrow_vertex_coord(value: f64) -> i16 {
    let clamped = value
        .round()
        .clamp(f64::from(i16::MIN), f64::from(i16::MAX));
    // Finite and within `i16` range by the clamp above.
    #[allow(clippy::cast_possible_truncation)]
    {
        clamped as i16
    }
}

/// Converts an arena index to its `u16` on-disk form, or a defensive
/// [`NodeBuildError::TooManyElements`] naming `kind` if it does not fit.
fn encode_index(idx: usize, kind: &'static str) -> Result<u16, NodeBuildError> {
    u16::try_from(idx).map_err(|_| NodeBuildError::TooManyElements {
        kind,
        count: idx.saturating_add(1),
        max: MAX_U16_INDEXED,
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
}
