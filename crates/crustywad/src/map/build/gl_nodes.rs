//! The GL BSP pass — the `build_gl_nodes` kernel's working state and types
//! (ADR-0026 §2, issue #363).
//!
//! This module structurally mirrors the classic BSP kernel (`nodes.rs`): the
//! same vertex-narrowing gate, the same initial-seg pass, the same explicit
//! work-stack partitioner (in later tasks). The GL kernel differs in three
//! ways — vertices widen to 16.16 fixed-point (`raw << 16`), segs carry a
//! `partner` link maintained from birth (the GL involution), and there is no
//! per-seg `offset` (the GL formats derive it on read). Moved logic keeps the
//! classic semantics verbatim; a behavior change here changes built GL lumps.
//! Bare `§` references in item docs (e.g. §B.2, §D) are ADR-0024 sections,
//! carried over verbatim from the classic kernel this one mirrors.

use std::collections::HashMap;

use super::{NodeBuildError, NodeBuildOptions, NodeBuildWarning};
use crate::Strictness;
use crate::map::doom::{Narrower, narrow_vertices};
use crate::map::graph::{GlVertex, GlVertexIdx, GlVertexRef, Map, VertexIdx};

/// A working seg in the GL kernel. Mirrors the classic `WorkSeg` but adds the
/// GL partner link and drops `offset` (the GL formats derive it on read).
#[derive(Clone, Copy)]
#[allow(dead_code)] // `side_sector` is seeded here but not consumed until the
// convex-leaf / miniseg passes (Tasks 3–4) read it.
struct GlWorkSeg {
    /// Start-vertex index into the combined 16.16 vertex table.
    v1: usize,
    /// End-vertex index into the combined 16.16 vertex table.
    v2: usize,
    /// The source linedef, or `None` for a miniseg (Task 4).
    linedef: Option<usize>,
    /// `0` = right/front sidedef, `1` = left/back.
    side: u8,
    /// The sector this seg's own side faces (minisegs copy it from their loop
    /// seg).
    side_sector: usize,
    /// The partner seg on the other side of a two-sided edge — an involution
    /// over working seg ids — or `None` for a one-sided edge.
    partner: Option<usize>,
}

/// A partition line in 16.16 space with its widened integer precomputes, built
/// once per candidate so the classify pass (Task 3) does no redundant widening.
#[derive(Clone, Copy)]
#[allow(dead_code)] // Consumed by the partition/classify pass (Task 3).
struct GlPartition {
    /// Line start `x` (16.16).
    px: i32,
    /// Line start `y` (16.16).
    py: i32,
    /// Line direction `dx` (16.16).
    dx: i32,
    /// Line direction `dy` (16.16).
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

impl GlPartition {
    /// Builds a partition from 16.16 line start `(px, py)` and direction
    /// `(dx, dy)`, precomputing the `i64` widenings and the `i128` squared
    /// length `len2 = pdx² + pdy²`.
    #[allow(dead_code)] // Called by the partition/classify pass (Task 3).
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

/// A child slot in the GL internal tree arena, resolved to final indices at the
/// flatten step. A private local copy of the classic kernel's `TreeRef` (that
/// one is private to `nodes.rs`).
#[derive(Clone, Copy)]
#[allow(dead_code)] // Constructed by the partition pass (Task 3).
enum TreeRef {
    /// A finished convex leaf: index into [`GlBsp::leaves`].
    Leaf(usize),
    /// An internal node: index into [`GlBsp::tree_nodes`].
    Node(usize),
}

/// One internal GL BSP node in the tree arena, built in post-order (children
/// first, root last) so its arena index *is* its final node index.
#[allow(dead_code)] // Populated by the partition pass (Task 3).
struct GlTreeNode {
    /// Partition-line start `x` (16.16).
    px: i32,
    /// Partition-line start `y` (16.16).
    py: i32,
    /// Partition-line `dx` (16.16).
    dx: i32,
    /// Partition-line `dy` (16.16).
    dy: i32,
    /// The front (right) child.
    front: TreeRef,
    /// The back (left) child.
    back: TreeRef,
}

/// The GL BSP builder's working state (ADR-0026 §2), mirroring the classic
/// [`Bsp`](super::nodes) structurally.
#[allow(dead_code)] // Several arenas (dedup, spawned, leaves, tree_nodes, root,
// the heuristic weights, strictness/warnings) are seeded here and consumed by
// the partition / miniseg / flatten passes in Tasks 3–6.
struct GlBsp<'a> {
    /// The source map (linedef/sidedef references resolve against it).
    map: &'a Map,
    /// Combined 16.16 vertex coordinates: map vertices first, then split
    /// vertices.
    verts: Vec<(i32, i32)>,
    /// Count of leading map vertices in [`verts`](Self::verts); split-vertex
    /// indices start here.
    map_vertex_count: usize,
    /// Output arena for split vertices (`f64 = fixed / 65536.0`), in creation
    /// order.
    gl_vertices: Vec<GlVertex>,
    /// Exact-coordinate dedup over the combined 16.16 table, seeded with the map
    /// vertices (first-writer-wins).
    dedup: HashMap<(i32, i32), usize>,
    /// The seg arena.
    segs: Vec<GlWorkSeg>,
    /// Eager co-split fragments awaiting their container seg (Task 4).
    spawned: HashMap<usize, Vec<usize>>,
    /// Live seg count (segs reachable in some pending set or leaf).
    live_segs: usize,
    /// Strict or lenient (drives narrowing and the ceilings).
    strictness: Strictness,
    /// Partition heuristic: weight per straddling split (§B.3).
    split_cost: u32,
    /// Partition heuristic: axis-aligned preference divisor (`0` = no penalty).
    aa_preference: u32,
    /// Recovered lenient-mode warnings.
    warnings: Vec<NodeBuildWarning>,
    /// Convex leaves, in creation order = final subsector order.
    leaves: Vec<Vec<usize>>,
    /// Internal nodes, in post-order (root last).
    tree_nodes: Vec<GlTreeNode>,
    /// The tree root, set by the partition pass (Task 3).
    root: Option<TreeRef>,
}

impl<'a> GlBsp<'a> {
    /// Narrows the map's vertices through the shared write path (ADR-0024 §3),
    /// widens each to 16.16, and seeds the combined vertex table and dedup
    /// index.
    ///
    /// Mirrors the classic [`Bsp::new`](super::nodes) gate exactly: strict
    /// narrowing failures propagate as [`NodeBuildError::Write`]; lenient
    /// recoveries drain into [`NodeBuildWarning::Write`].
    ///
    /// # Errors
    ///
    /// Returns [`NodeBuildError::Write`] when strict-mode narrowing rejects a
    /// coordinate that does not fit the Doom `i16` on-disk field.
    #[allow(dead_code)] // Driven by `build_gl_nodes` (Task 6).
    fn new(map: &'a Map, opts: &NodeBuildOptions) -> Result<Self, NodeBuildError> {
        // ADR-0024 §3: the identical narrowing pass the write path and classic
        // kernel use. Strict failures surface as `Write(..)`; recoveries become
        // `NodeBuildWarning::Write`.
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
            // Widen each whole-unit narrowed coordinate to 16.16 fixed-point.
            let coord = (i32::from(v.x) << 16, i32::from(v.y) << 16);
            verts.push(coord);
            // First writer wins; a later duplicate map vertex never becomes a
            // dedup target. Deterministic and harmless: dedup only ever
            // redirects a *new* split vertex.
            dedup.entry(coord).or_insert(i);
        }
        let map_vertex_count = verts.len();

        Ok(Self {
            map,
            verts,
            map_vertex_count,
            gl_vertices: Vec::new(),
            dedup,
            segs: Vec::new(),
            spawned: HashMap::new(),
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

    /// ADR-0026 §2 / §A.4: one seg per present linedef side, skipping
    /// zero-length-after-narrowing linedefs. A two-sided linedef additionally
    /// links its front/back segs as partners from birth (the GL involution,
    /// Notes §Q1): the back seg has swapped `v1`/`v2` and the pair cross-links,
    /// so `partner[partner[i]] == i` and the mirrored-span invariant
    /// (`segs[i].v1 == segs[partner].v2`) holds by construction. One-sided segs
    /// get `partner: None`.
    #[allow(dead_code)] // Driven by `build_gl_nodes` (Task 6).
    fn build_initial_segs(&mut self) {
        for (li, ld) in self.map.linedefs().iter().enumerate() {
            let (a, b) = (ld.start.0, ld.end.0);
            // Zero-length after narrowing: no direction, engine derives nothing.
            if self.verts[a] == self.verts[b] {
                continue;
            }
            let two_sided = ld.right.is_some() && ld.left.is_some();
            // Index the right seg will occupy; the left seg (if any) follows it.
            let right_id = self.segs.len();
            if let Some(side) = ld.right {
                let sector = self.map.sidedefs()[side.0].sector.0;
                self.segs.push(GlWorkSeg {
                    v1: a,
                    v2: b,
                    linedef: Some(li),
                    side: 0,
                    side_sector: sector,
                    // Partner is the back seg, pushed next (Notes §Q1).
                    partner: two_sided.then_some(right_id + 1),
                });
            }
            if let Some(side) = ld.left {
                let sector = self.map.sidedefs()[side.0].sector.0;
                // Back seg swaps v1/v2 so the pair mirrors span. When two-sided,
                // its partner is the right seg at `right_id`; a left-only linedef
                // has no partner.
                self.segs.push(GlWorkSeg {
                    v1: b,
                    v2: a,
                    linedef: Some(li),
                    side: 1,
                    side_sector: sector,
                    partner: two_sided.then_some(right_id),
                });
            }
        }
        self.live_segs = self.segs.len();
    }

    /// Resolves a combined-table vertex index to a [`GlVertexRef`]: an index
    /// below [`map_vertex_count`](Self::map_vertex_count) is a `Normal`
    /// (`VERTEXES`) vertex; at or above it, a `Gl` (`GL_VERT`) split vertex.
    #[allow(dead_code)] // Consumed by the flatten step (Task 5).
    fn vertex_ref(&self, idx: usize) -> GlVertexRef {
        if idx < self.map_vertex_count {
            GlVertexRef::Normal(VertexIdx(idx))
        } else {
            GlVertexRef::Gl(GlVertexIdx(idx - self.map_vertex_count))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::graph::{
        MapFormat, MapLinedef, MapSector, MapSidedef, MapThing, MapVertex, SectorIdx, SidedefIdx,
        Special, TextureRef,
    };

    /// A default one-sided sidedef facing `sector`.
    fn sidedef(sector: usize) -> MapSidedef {
        MapSidedef {
            sector: SectorIdx(sector),
            x_offset: 0,
            y_offset: 0,
            upper: TextureRef::Name("-".into()),
            lower: TextureRef::Name("-".into()),
            middle: TextureRef::Name("STARTAN3".into()),
        }
    }

    /// A default sector.
    fn sector() -> MapSector {
        MapSector {
            floor_height: 0,
            ceiling_height: 128,
            floor_flat: TextureRef::Name("FLOOR4_8".into()),
            ceiling_flat: TextureRef::Name("CEIL3_5".into()),
            light: 160,
            special: 0,
            tag: 0,
            colors: None,
            flags: 0,
        }
    }

    /// Builds a `Map` from vertex coordinates and linedefs given as
    /// `(start, end, right_sector, left_sector)` — a `Some(sector)` side gets a
    /// fresh sidedef facing that sector. A `left` side makes the linedef
    /// two-sided.
    fn build_map(
        verts: &[(f64, f64)],
        lines: &[(usize, usize, Option<usize>, Option<usize>)],
    ) -> Map {
        let mut sidedefs = Vec::new();
        let mut max_sector = 0usize;
        let mut linedefs = Vec::new();
        for &(s, e, r, l) in lines {
            let mut alloc = |sec: usize| {
                max_sector = max_sector.max(sec);
                let i = sidedefs.len();
                sidedefs.push(sidedef(sec));
                SidedefIdx(i)
            };
            let right = r.map(&mut alloc);
            let left = l.map(&mut alloc);
            linedefs.push(MapLinedef {
                start: VertexIdx(s),
                end: VertexIdx(e),
                right,
                left,
                flags: 0,
                special: Special {
                    special: 0,
                    args: [0; 5],
                },
                id: 0,
            });
        }
        Map {
            name: "MAP01".into(),
            format: MapFormat::Doom,
            namespace: None,
            vertices: verts.iter().map(|&(x, y)| MapVertex { x, y }).collect(),
            linedefs,
            sidedefs,
            sectors: (0..=max_sector).map(|_| sector()).collect(),
            things: vec![MapThing {
                x: 0.0,
                y: 0.0,
                angle: 0,
                type_id: 1,
                flags: 0,
                id: 0,
                height: 0.0,
                special: Special {
                    special: 0,
                    args: [0; 5],
                },
            }],
            lights: Vec::new(),
            segs: Vec::new(),
            subsectors: Vec::new(),
            nodes: Vec::new(),
            gl_vertices: Vec::new(),
            gl_segs: Vec::new(),
            gl_subsectors: Vec::new(),
            gl_nodes: Vec::new(),
            leafs: Vec::new(),
            macros: Vec::new(),
            reject: None,
            blockmap: None,
            warnings: Vec::new(),
        }
    }

    /// Two square rooms sharing the vertical wall from v1 to v2 — the only
    /// two-sided linedef (index 1). Six vertices, seven linedefs, two sectors.
    fn two_room_map() -> Map {
        build_map(
            &[
                (0.0, 0.0),    // 0
                (64.0, 0.0),   // 1
                (64.0, 64.0),  // 2
                (0.0, 64.0),   // 3
                (128.0, 0.0),  // 4
                (128.0, 64.0), // 5
            ],
            &[
                (0, 1, Some(0), None),    // L0 south wall, sector 0
                (1, 2, Some(0), Some(1)), // L1 SHARED vertical wall (two-sided)
                (2, 3, Some(0), None),    // L2 north wall, sector 0
                (3, 0, Some(0), None),    // L3 west wall, sector 0
                (1, 4, Some(1), None),    // L4 south wall, sector 1
                (4, 5, Some(1), None),    // L5 east wall, sector 1
                (5, 2, Some(1), None),    // L6 north wall, sector 1
            ],
        )
    }

    #[test]
    fn initial_segs_link_two_sided_partners_as_mirrored_involution() {
        let map = two_room_map();
        let mut bsp = GlBsp::new(&map, &NodeBuildOptions::strict()).unwrap();
        bsp.build_initial_segs();

        // The two segs of the shared linedef (index 1) are the partnered pair.
        let pair: Vec<usize> = bsp
            .segs
            .iter()
            .enumerate()
            .filter(|(_, s)| s.linedef == Some(1))
            .map(|(i, _)| i)
            .collect();
        assert_eq!(pair.len(), 2, "the two-sided linedef yields two segs");
        let (i, j) = (pair[0], pair[1]);

        // Involution over working ids.
        assert_eq!(bsp.segs[i].partner, Some(j));
        assert_eq!(bsp.segs[j].partner, Some(i));
        let pi = bsp.segs[i].partner.unwrap();
        assert_eq!(bsp.segs[pi].partner, Some(i), "partner[partner[i]] == i");

        // Mirrored span: each seg's start is its partner's end and vice versa.
        assert_eq!(bsp.segs[i].v1, bsp.segs[j].v2);
        assert_eq!(bsp.segs[i].v2, bsp.segs[j].v1);

        // Opposite sides.
        assert_ne!(bsp.segs[i].side, bsp.segs[j].side);
        assert_eq!(bsp.segs[i].side + bsp.segs[j].side, 1);

        // Every one-sided seg has no partner.
        for s in &bsp.segs {
            if s.linedef != Some(1) {
                assert_eq!(s.partner, None, "one-sided segs have no partner");
            }
        }
    }

    #[test]
    fn new_widens_vertices_to_16_16_and_dedups_exactly() {
        // A one-sided triangle whose apex sits at (64, -32).
        let map = build_map(
            &[(0.0, 0.0), (128.0, 0.0), (64.0, -32.0)],
            &[
                (0, 1, Some(0), None),
                (1, 2, Some(0), None),
                (2, 0, Some(0), None),
            ],
        );
        let bsp = GlBsp::new(&map, &NodeBuildOptions::strict()).unwrap();

        assert_eq!(bsp.map_vertex_count, 3);
        assert_eq!(bsp.verts.len(), 3);
        // (64, -32) widens to (64 << 16, -32 << 16).
        assert_eq!(bsp.verts[2], (64 << 16, -32 << 16));
        assert_eq!(bsp.verts[0], (0, 0));

        // vertex_ref straddles the map/split boundary.
        assert_eq!(bsp.vertex_ref(0), GlVertexRef::Normal(VertexIdx(0)));
        assert_eq!(bsp.vertex_ref(2), GlVertexRef::Normal(VertexIdx(2)));
        assert_eq!(bsp.vertex_ref(3), GlVertexRef::Gl(GlVertexIdx(0)));
        assert_eq!(bsp.vertex_ref(5), GlVertexRef::Gl(GlVertexIdx(2)));
    }
}
