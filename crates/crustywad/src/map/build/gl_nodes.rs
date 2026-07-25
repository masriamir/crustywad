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

use std::collections::{BTreeMap, HashMap, VecDeque};

use super::geom;
use super::nodes::{MAX_EXTENDED_INDEX, SAMPLE_BUDGET};
use super::{NodeBuildError, NodeBuildOptions, NodeBuildWarning, NodeStructureError};
use crate::Strictness;
use crate::map::doom::{Narrower, narrow_vertices};
use crate::map::graph::{
    GlNodeChild, GlNodeIdx, GlSeg, GlSegIdx, GlSubsector, GlSubsectorIdx, GlVertex, GlVertexIdx,
    GlVertexRef, LinedefIdx, Map, VertexIdx,
};

/// A working seg in the GL kernel. Mirrors the classic `WorkSeg` but adds the
/// GL partner link and drops `offset` (the GL formats derive it on read).
#[derive(Clone, Copy)]
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
struct GlPartition {
    /// Line start `x` (16.16).
    px: i32,
    /// Line start `y` (16.16).
    py: i32,
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
    fn new(px: i32, py: i32, dx: i32, dy: i32) -> Self {
        let (pxi, pyi, pdx, pdy) = (i64::from(px), i64::from(py), i64::from(dx), i64::from(dy));
        let len2 = i128::from(pdx) * i128::from(pdx) + i128::from(pdy) * i128::from(pdy);
        Self {
            px,
            py,
            pxi,
            pyi,
            pdx,
            pdy,
            len2,
        }
    }
}

/// How a seg sits relative to a GL partition line (§B.2) — the fixed-space twin
/// of the classic kernel's `Class`. Colinear segs (both endpoints on the line)
/// are tracked apart from segs with genuine off-line extent so the convexity
/// test can distinguish them; a straddler is [`GlClass::Split`] **only** when its
/// rounded intersection is strictly interior (the §C.3 endpoint-coincidence
/// collapse is already folded in), so a candidate whose "splits" all collapse to
/// one side is scored as leaving the other side empty. A single classification
/// decision is shared by [`GlBsp::select`]'s scoring and the split routing (via
/// [`GlBsp::classify_seg`]) so the two can never disagree.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum GlClass {
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
    /// `(x, y)` (16.16), guaranteed distinct from either endpoint.
    Split(i32, i32),
}

impl GlClass {
    /// Whether a non-splitting seg routes to the front side when partitioned.
    fn is_front(self) -> bool {
        matches!(self, GlClass::Front | GlClass::ColinearFront)
    }
}

/// A child slot in the GL internal tree arena, resolved to final indices at the
/// flatten step. A private local copy of the classic kernel's `TreeRef` (that
/// one is private to `nodes.rs`).
#[derive(Clone, Copy)]
enum TreeRef {
    /// A finished convex leaf: index into [`GlBsp::leaves`].
    Leaf(usize),
    /// An internal node: index into [`GlBsp::tree_nodes`].
    Node(usize),
}

/// One internal GL BSP node in the tree arena, built in post-order (children
/// first, root last) so its arena index *is* its final node index.
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

/// One on-partition vertex encountered while routing a set through a partition,
/// with the colinear segs that touch it. Task 5's loop-closing checks walk the
/// events in partition order and need, per vertex, which colinear front/back
/// segs enter it — so their working ids are retained here rather than just a
/// coverage count.
struct EventData {
    /// The combined-table id of the on-partition vertex.
    vertex: usize,
    /// Working ids of [`GlClass::ColinearFront`] segs incident to this vertex.
    colinear_front: Vec<usize>,
    /// Working ids of [`GlClass::ColinearBack`] segs incident to this vertex.
    colinear_back: Vec<usize>,
}

/// The per-`split_set` accumulator of on-partition events (§Q2, §Implications):
/// the vertices lying on the current partition, keyed by their **exact `i128`
/// dot product** along the partition direction. Integer keys replace ZDBSP's
/// `double` distances, so ordering along the line is exact and collision-free.
/// One accumulator is cleared and refilled per [`GlBsp::split_set`] call; Task
/// 5's `branch` consumes it immediately after routing (before the next call
/// overwrites it).
struct EventAccumulator {
    /// On-partition vertices keyed by their exact dot product along the
    /// partition direction, ordered by that key.
    events: BTreeMap<i128, EventData>,
}

impl EventAccumulator {
    /// A new, empty accumulator.
    fn new() -> Self {
        Self {
            events: BTreeMap::new(),
        }
    }

    /// Empties the accumulator for reuse by the next [`GlBsp::split_set`] call.
    fn clear(&mut self) {
        self.events.clear();
    }

    /// Records vertex `vertex` (already known on-partition) at exact dot `key`,
    /// merging into any existing event at that key. `colinear` optionally tags a
    /// colinear seg incident here: `Some((true, id))` for a front colinear seg,
    /// `Some((false, id))` for a back one; `None` for a split point or a
    /// non-colinear on-line endpoint.
    fn record(&mut self, key: i128, vertex: usize, colinear: Option<(bool, usize)>) {
        let entry = self.events.entry(key).or_insert_with(|| EventData {
            vertex,
            colinear_front: Vec::new(),
            colinear_back: Vec::new(),
        });
        // Distinct vertices cannot share an exact dot key on the same partition:
        // the key is the exact projection, so equal keys mean the same point.
        debug_assert_eq!(
            entry.vertex, vertex,
            "two distinct vertices share a partition dot key"
        );
        if let Some((front, id)) = colinear {
            if front {
                entry.colinear_front.push(id);
            } else {
                entry.colinear_back.push(id);
            }
        }
    }
}

/// The GL BSP builder's working state (ADR-0026 §2), mirroring the classic
/// [`Bsp`](super::nodes) structurally.
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
    /// Eager co-split fragments awaiting the set or leaf that still holds the
    /// seg they came from, keyed by that seg's working id (the Vec-set analog of
    /// ZDBSP's intrusive `Segs[p].next = p2` splice). When [`split_seg_at`] splits
    /// a seg it also co-splits the seg's partner; the partner's *new* fragment is
    /// not part of the set being routed right now, so it is parked here under the
    /// partner's id until a consumer reaches that partner. Consumers drain it
    /// transitively at exactly three points:
    /// (a) [`split_set`](Self::split_set)'s routing queue pushes a key's parked
    ///     fragments when it pops that key id;
    /// (b) the recursion driver expands a popped set before classification the
    ///     same way (Task 6);
    /// (c) `finish` expands each leaf's seg list before finalization (Task 6).
    /// Points (b)/(c) land with the driver in Task 6; only (a) is wired here. A
    /// partner sitting in an already-emitted leaf can still be co-split by a
    /// later partition in the sibling subtree — subdividing an edge of a convex
    /// polygon keeps it convex and closed, so a cross-subtree co-split is normal,
    /// not an error, and (c) is why an emitted leaf is still re-expanded.
    ///
    /// [`split_seg_at`]: Self::split_seg_at
    spawned: HashMap<usize, Vec<usize>>,
    /// Per-vertex incident segs whose current start (`v1`) is this vertex,
    /// indexed by combined-table vertex id (the Rust analog of ZDBSP's
    /// `nextforvert` lists). Grown by [`intern_vertex`](Self::intern_vertex) and
    /// maintained on every seg creation and in-place split. Consumed by Task 5.
    segs_starting_at: Vec<Vec<usize>>,
    /// Per-vertex incident segs whose current end (`v2`) is this vertex, the
    /// end-vertex twin of [`segs_starting_at`](Self::segs_starting_at). An
    /// in-place split that shortens a seg moves it here from its old end to the
    /// mid. Consumed by Task 5.
    segs_ending_at: Vec<Vec<usize>>,
    /// On-partition events for the current [`split_set`](Self::split_set) call,
    /// consumed by Task 5's `branch`.
    events: EventAccumulator,
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
            segs_starting_at: vec![Vec::new(); map_vertex_count],
            segs_ending_at: vec![Vec::new(); map_vertex_count],
            events: EventAccumulator::new(),
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
                let id = self.segs.len();
                self.segs.push(GlWorkSeg {
                    v1: a,
                    v2: b,
                    linedef: Some(li),
                    side: 0,
                    side_sector: sector,
                    // Partner is the back seg, pushed next (Notes §Q1).
                    partner: two_sided.then_some(right_id + 1),
                });
                self.register_seg(id, a, b);
            }
            if let Some(side) = ld.left {
                let sector = self.map.sidedefs()[side.0].sector.0;
                // Back seg swaps v1/v2 so the pair mirrors span. When two-sided,
                // its partner is the right seg at `right_id`; a left-only linedef
                // has no partner.
                let id = self.segs.len();
                self.segs.push(GlWorkSeg {
                    v1: b,
                    v2: a,
                    linedef: Some(li),
                    side: 1,
                    side_sector: sector,
                    partner: two_sided.then_some(right_id),
                });
                self.register_seg(id, b, a);
            }
        }
        self.live_segs = self.segs.len();
    }

    /// Resolves a combined-table vertex index to a [`GlVertexRef`]: an index
    /// below [`map_vertex_count`](Self::map_vertex_count) is a `Normal`
    /// (`VERTEXES`) vertex; at or above it, a `Gl` (`GL_VERT`) split vertex.
    fn vertex_ref(&self, idx: usize) -> GlVertexRef {
        if idx < self.map_vertex_count {
            GlVertexRef::Normal(VertexIdx(idx))
        } else {
            GlVertexRef::Gl(GlVertexIdx(idx - self.map_vertex_count))
        }
    }

    /// Whether every seg in `set` faces the same sector (§C.1). Minisegs carry a
    /// real `side_sector`, so no seg needs special-casing here.
    fn single_sector(&self, set: &[usize]) -> bool {
        let first = self.segs[set[0]].side_sector;
        set.iter().all(|&id| self.segs[id].side_sector == first)
    }

    /// The exact `i128` cross product of the partition direction with the vector
    /// from the line start to the 16.16 vertex `v`: `> 0` front, `< 0` back
    /// (§B.2, engine convention `R_PointOnSide`). Wide because a 16.16 delta can
    /// reach `2³²`, overflowing the classic `i64` cross.
    fn cross(&self, part: &GlPartition, v: usize) -> i128 {
        let (qx, qy) = self.verts[v];
        geom::cross_from_start_wide(
            i64::from(qx) - part.pxi,
            i64::from(qy) - part.pyi,
            part.pdx,
            part.pdy,
        )
    }

    /// Whether wide cross product `c` places its vertex **less than** 0.5 fixed
    /// units from the line (strict; a vertex exactly 0.5 units off counts as a
    /// side, not on the line).
    fn on_line(part: &GlPartition, c: i128) -> bool {
        geom::within_half_fixed_unit(c, part.len2)
    }

    /// Classifies seg `s` against `part` (§B.2) in fixed space, folding in the
    /// §C.3 endpoint-coincidence collapse so the result is **exactly** what the
    /// split routing will do — the single source of truth that keeps
    /// [`select`](Self::select) and the split pass in agreement. Colinear segs
    /// (both endpoints on-line) route by **orientation**: the dot of the seg
    /// direction with the partition direction `> 0` → [`GlClass::ColinearFront`],
    /// else [`GlClass::ColinearBack`] (source-verified ZDBSP rule, Notes §Q6).
    fn classify_seg(&self, part: &GlPartition, s: &GlWorkSeg) -> GlClass {
        let c1 = self.cross(part, s.v1);
        let c2 = self.cross(part, s.v2);
        let (on1, on2) = (Self::on_line(part, c1), Self::on_line(part, c2));
        let front = u8::from(!on1 && c1 > 0) + u8::from(!on2 && c2 > 0);
        let back = u8::from(!on1 && c1 < 0) + u8::from(!on2 && c2 < 0);

        if front > 0 && back > 0 {
            // Strict straddler. Compute where the split would actually land, on
            // the seg's own linedef geometry (§C.3), and collapse to a side if
            // the rounded point coincides with an endpoint.
            let Some((mx, my)) = self.intersection(s, part) else {
                // Parallel to its own canonical line — impossible for a genuine
                // straddler; never divide by zero (Global Constraint 9).
                debug_assert!(
                    false,
                    "a straddling seg cannot be parallel to the partition"
                );
                return GlClass::Front;
            };
            let ec1 = self.verts[s.v1];
            let ec2 = self.verts[s.v2];
            // A rounded split on an endpoint means the seg no longer straddles
            // after rounding — it collapses to the *other* endpoint's side.
            if (mx, my) == ec1 {
                return if c2 > 0 {
                    GlClass::Front
                } else {
                    GlClass::Back
                };
            }
            if (mx, my) == ec2 {
                return if c1 > 0 {
                    GlClass::Front
                } else {
                    GlClass::Back
                };
            }
            return GlClass::Split(mx, my);
        }
        if front > 0 {
            GlClass::Front
        } else if back > 0 {
            GlClass::Back
        } else {
            // Colinear (both endpoints on-line): assign by orientation (§Q6).
            let (sx1, sy1) = self.verts[s.v1];
            let (sx2, sy2) = self.verts[s.v2];
            let dot = i128::from(part.pdx) * (i128::from(sx2) - i128::from(sx1))
                + i128::from(part.pdy) * (i128::from(sy2) - i128::from(sy1));
            // A genuine colinear seg lies on the same nonzero-length line as the
            // partition, so its direction is (anti)parallel and the dot is
            // nonzero; a zero dot is impossible. Route Back if it somehow occurs.
            debug_assert!(
                dot != 0,
                "a colinear seg has a nonzero dot with the partition"
            );
            if dot > 0 {
                GlClass::ColinearFront
            } else {
                GlClass::ColinearBack
            }
        }
    }

    /// The rounded intersection of `part`'s line with seg `s`'s **canonical**
    /// geometry (§C.3), or `None` if they are parallel. For a real linedef the
    /// intersection is computed on the linedef's own start→end vertices (via
    /// [`verts`](Self::verts)), so both segs of a two-sided linedef split at the
    /// identical vertex (crack-freedom); a miniseg (`linedef: None`) uses its own
    /// `v1`→`v2`.
    ///
    /// The parametric solution is exact rational integer arithmetic — `num` and
    /// `den` are `i128` cross products, and each axis is
    /// `line_v1 + round_half_away_rational(num · delta_axis, den)`. This replaces
    /// ZDBSP's `double` + truncation with exact round-half-away integers (a
    /// documented divergence). `den == 0` (parallel) yields `None`.
    ///
    /// The result lies within the seg's bounding range by construction (an
    /// intersection point of a bounded seg with a line it straddles), so both
    /// axes fit `i32`; a hand-built out-of-range fixture trips the `debug_assert`
    /// and returns `None` (treated as parallel — a conservative non-split) rather
    /// than truncating silently.
    // `similar_names`: the `l*`/`p*` coordinate pairs mirror the classic
    // kernel's `intersection` naming; renaming obscures the parametric form.
    #[allow(clippy::similar_names)]
    fn intersection(&self, s: &GlWorkSeg, part: &GlPartition) -> Option<(i32, i32)> {
        // Canonical line endpoints: the source linedef's vertices for a real
        // seg, the seg's own endpoints for a miniseg.
        let (lv1, lv2) = match s.linedef {
            Some(li) => {
                let ld = &self.map.linedefs()[li];
                (self.verts[ld.start.0], self.verts[ld.end.0])
            }
            None => (self.verts[s.v1], self.verts[s.v2]),
        };
        let (lsx, lsy) = lv1;
        let (lex, ley) = lv2;
        let ldx = i128::from(lex) - i128::from(lsx);
        let ldy = i128::from(ley) - i128::from(lsy);
        // num = cross((partition start − line start), partition direction).
        let num = (i128::from(part.px) - i128::from(lsx)) * i128::from(part.pdy)
            - (i128::from(part.py) - i128::from(lsy)) * i128::from(part.pdx);
        // den = cross(line direction, partition direction).
        let den = ldx * i128::from(part.pdy) - ldy * i128::from(part.pdx);
        if den == 0 {
            return None; // parallel — no unique intersection
        }
        let mx = i128::from(lsx) + round_half_away_rational(num * ldx, den);
        let my = i128::from(lsy) + round_half_away_rational(num * ldy, den);
        if let (Ok(mx), Ok(my)) = (i32::try_from(mx), i32::try_from(my)) {
            Some((mx, my))
        } else {
            debug_assert!(
                false,
                "intersection of a bounded seg fits i32 by construction"
            );
            None
        }
    }

    /// Interns a split vertex, reusing any map or split vertex with the same
    /// **exact** 16.16 coordinate (§C.3): the combined `dedup` table is seeded
    /// with the map vertices, so interning an existing map vertex's coordinate
    /// returns its `Normal`-range index and grows nothing. A miss appends to the
    /// combined table, the split-vertex arena (`GlVertex { x: fixed / 65536.0 }`),
    /// `dedup`, and the two per-vertex incident lists. Returns the index.
    fn intern_vertex(&mut self, x: i32, y: i32) -> usize {
        if let Some(&idx) = self.dedup.get(&(x, y)) {
            return idx;
        }
        let idx = self.verts.len();
        self.verts.push((x, y));
        self.gl_vertices.push(GlVertex {
            x: f64::from(x) / 65536.0,
            y: f64::from(y) / 65536.0,
        });
        self.dedup.insert((x, y), idx);
        // Keep the incident lists index-aligned with `verts`.
        self.segs_starting_at.push(Vec::new());
        self.segs_ending_at.push(Vec::new());
        idx
    }

    /// Records a newly created seg `id` spanning `v1`→`v2` in the per-vertex
    /// incident lists.
    fn register_seg(&mut self, id: usize, v1: usize, v2: usize) {
        self.segs_starting_at[v1].push(id);
        self.segs_ending_at[v2].push(id);
    }

    /// Moves seg `id`'s recorded end vertex from `old` to `new` after an in-place
    /// split shortened it, preserving incident-list order (deterministic).
    fn move_seg_end(&mut self, id: usize, old: usize, new: usize) {
        if let Some(pos) = self.segs_ending_at[old].iter().position(|&s| s == id) {
            self.segs_ending_at[old].remove(pos);
        }
        self.segs_ending_at[new].push(id);
    }

    /// Records vertex `vertex` (known on-partition) into the current event
    /// accumulator, computing its exact `i128` dot key along `part`'s direction.
    /// `colinear` tags a colinear seg incident here (see
    /// [`EventAccumulator::record`]).
    fn record_event(&mut self, part: &GlPartition, vertex: usize, colinear: Option<(bool, usize)>) {
        let (vx, vy) = self.verts[vertex];
        let key = i128::from(part.pdx) * (i128::from(vx) - i128::from(part.px))
            + i128::from(part.pdy) * (i128::from(vy) - i128::from(part.py));
        self.events.record(key, vertex, colinear);
    }

    /// Splits straddling seg `sid` **in place** at the interior rounded
    /// intersection `(mx, my)` (mirroring ZDBSP `SplitSeg`, Notes §Q1): `sid`
    /// keeps the `v1→m` half, a freshly pushed id takes `m→v2`, and both fragments
    /// inherit linedef/side/sector. The mid vertex is interned **once** and reused
    /// by the partner co-split, so a two-sided linedef's two segs share the split
    /// vertex (crack-freedom). Returns `(v1→m fragment, m→v2 fragment)` — the
    /// caller re-classifies each to route it.
    ///
    /// # Eager partner co-split
    ///
    /// If `sid` has partner `p` (mirrored span `p = v2→v1`), `p` is split at the
    /// same interned mid into `pA = v2→m` (kept in `p`) and `pB = m→v1` (new).
    /// With `v1`-ordered fragments the mirrors are the **cross** pairing: `sA`
    /// (`v1→m`) mirrors `pB` (`m→v1`), and `sB` (`m→v2`) mirrors `pA` (`v2→m`) —
    /// so `partner(sid) = pB` and `partner(sB) = p`, *not* `sid ↔ p`. (`sid ↔ p`
    /// is ZDBSP's side-ordered convention, which this kernel does not use.) `pB`
    /// is not in the set being routed now, so it is parked in
    /// [`spawned`](Self::spawned) under `p`. The mirrored-span `debug_assert!`s
    /// below are the authoritative contract.
    ///
    /// # Errors
    ///
    /// [`NodeBuildError::TooManyElements`] (`kind: "segs"`) if the live-seg count
    /// exceeds [`MAX_EXTENDED_INDEX`] — the runaway backstop (Global Constraint 9);
    /// the GL formats are always extended, so the cap is uniform.
    fn split_seg_at(
        &mut self,
        sid: usize,
        mx: i32,
        my: i32,
    ) -> Result<(usize, usize), NodeBuildError> {
        let m = self.intern_vertex(mx, my);
        let s = self.segs[sid];
        let (v1, v2) = (s.v1, s.v2);
        debug_assert_ne!(m, v1, "split vertex coincides with the seg start");
        debug_assert_ne!(m, v2, "split vertex coincides with the seg end");

        // sB = m→v2 (new); `sid` becomes sA = v1→m in place.
        let s_b_id = self.segs.len();
        self.segs.push(GlWorkSeg { v1: m, v2, ..s });
        self.segs[sid].v2 = m;
        self.move_seg_end(sid, v2, m);
        self.register_seg(s_b_id, m, v2);
        let mut new_segs = 1usize;

        if let Some(p) = s.partner {
            let ps = self.segs[p];
            debug_assert!(
                ps.v1 == v2 && ps.v2 == v1,
                "partner span is the mirror of the seg's own span"
            );
            // pB = m→v1 (new); `p` becomes pA = v2→m in place.
            let p_b_id = self.segs.len();
            self.segs.push(GlWorkSeg {
                v1: m,
                v2: v1,
                ..ps
            });
            self.segs[p].v2 = m;
            self.move_seg_end(p, v1, m);
            self.register_seg(p_b_id, m, v1);
            new_segs += 1;

            // Cross re-link: sA↔pB and sB↔pA.
            self.segs[sid].partner = Some(p_b_id);
            self.segs[p_b_id].partner = Some(sid);
            self.segs[s_b_id].partner = Some(p);
            self.segs[p].partner = Some(s_b_id);

            // Park the partner's new fragment under the seg it came from (`p`).
            self.spawned.entry(p).or_default().push(p_b_id);

            // Authoritative involution + mirrored-span contract.
            debug_assert_eq!(
                self.segs[self.segs[sid].partner.unwrap()].partner,
                Some(sid)
            );
            debug_assert_eq!(self.segs[self.segs[p].partner.unwrap()].partner, Some(p));
            debug_assert!(
                self.segs[sid].v1 == self.segs[p_b_id].v2
                    && self.segs[sid].v2 == self.segs[p_b_id].v1,
                "sA and pB span-mirror"
            );
            debug_assert!(
                self.segs[s_b_id].v1 == self.segs[p].v2 && self.segs[s_b_id].v2 == self.segs[p].v1,
                "sB and pA span-mirror"
            );
        }

        // Each new fragment nets one live seg; the cap is the runaway backstop.
        self.live_segs += new_segs;
        if self.live_segs > MAX_EXTENDED_INDEX {
            return Err(NodeBuildError::TooManyElements {
                kind: "segs",
                count: self.live_segs,
                max: MAX_EXTENDED_INDEX,
            });
        }

        Ok((sid, s_b_id))
    }

    /// Partitions `set` by `part`, splitting straddlers (§C.3–C.4) and returning
    /// `(front, back)`. Routing is queue-based: a popped seg first drains any
    /// parked co-split fragments keyed by its id (spawn contract drain point (a),
    /// see [`spawned`](Self::spawned)) onto the queue, then classifies via the
    /// shared [`classify_seg`](Self::classify_seg). A `Split` runs
    /// [`split_seg_at`](Self::split_seg_at) and pushes **both** fragments back onto
    /// the queue: each now has the mid on the line, so it re-classifies cleanly to
    /// one side — re-classification is what keeps `select` and routing agreed.
    ///
    /// On-partition **events** are recorded for Task 5 (§Q2): each split point and
    /// on-line endpoint (recorded when a re-classified fragment reports its mid
    /// on-line), and both endpoints of every colinear seg (tagged front/back by
    /// its class). The accumulator is cleared on entry and left populated for the
    /// caller to consume before the next call.
    ///
    /// # Errors
    ///
    /// Propagates [`split_seg_at`](Self::split_seg_at)'s
    /// [`NodeBuildError::TooManyElements`].
    fn split_set(
        &mut self,
        set: Vec<usize>,
        part: &GlPartition,
    ) -> Result<(Vec<usize>, Vec<usize>), NodeBuildError> {
        self.events.clear();
        let mut front = Vec::new();
        let mut back = Vec::new();
        let mut queue: VecDeque<usize> = set.into();
        while let Some(sid) = queue.pop_front() {
            // Drain point (a): parked co-split fragments of this seg re-enter.
            self.drain_spawned(sid, &mut queue);
            let s = self.segs[sid];
            match self.classify_seg(part, &s) {
                GlClass::Split(mx, my) => {
                    let (a, b) = self.split_seg_at(sid, mx, my)?;
                    queue.push_back(a);
                    queue.push_back(b);
                }
                class @ (GlClass::ColinearFront | GlClass::ColinearBack) => {
                    let is_front = class.is_front();
                    self.record_event(part, s.v1, Some((is_front, sid)));
                    self.record_event(part, s.v2, Some((is_front, sid)));
                    if is_front {
                        front.push(sid);
                    } else {
                        back.push(sid);
                    }
                }
                class => {
                    // Genuine front/back extent; record whichever endpoint (if
                    // any) lies on the partition — a split point re-entering here.
                    let c1 = self.cross(part, s.v1);
                    let c2 = self.cross(part, s.v2);
                    if Self::on_line(part, c1) {
                        self.record_event(part, s.v1, None);
                    }
                    if Self::on_line(part, c2) {
                        self.record_event(part, s.v2, None);
                    }
                    if class.is_front() {
                        front.push(sid);
                    } else {
                        back.push(sid);
                    }
                }
            }
        }
        Ok((front, back))
    }

    /// Selects the best splitter in `set` (§B), or `None` if the set is convex
    /// (no valid partition). `relaxed` switches to the sector-separating validity
    /// rule (§C.2). Ties break toward the lowest seg id (determinism). Minisegs
    /// may serve as candidates (their line is a former partition).
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

    /// Scores the candidates at `positions` in `set`, updating `best`. Mirrors
    /// the classic kernel's `eval_candidates`: the same normal (§B.3) and relaxed
    /// (§C.2) validity rules, scored via [`geom::partition_score`] (counts are
    /// format-independent). `side_sector` is the facing sector for the
    /// sector-separation checks.
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
            let (px, py) = self.verts[s.v1];
            let (x2, y2) = self.verts[s.v2];
            let pdx = i64::from(x2) - i64::from(px);
            let pdy = i64::from(y2) - i64::from(py);
            // §B.1: only a seg whose deltas fit the on-disk `i32` node field can
            // be a splitter (it still participates as content).
            if !partition_delta_fits_gl(pdx, pdy) {
                continue;
            }
            #[allow(clippy::cast_possible_truncation)]
            let part = GlPartition::new(px, py, pdx as i32, pdy as i32);
            if part.len2 == 0 {
                continue; // a degenerate zero-length fragment cannot partition
            }
            // Full front/back counts (colinear included) drive scoring; the
            // non-colinear counts drive the normal convexity test. Because
            // `classify_seg` reports the *post-rounding* outcome, these counts
            // match the split routing exactly.
            let (mut nf, mut nb, mut nsp) = (0usize, 0usize, 0usize);
            let (mut front_solid, mut back_solid) = (0usize, 0usize);
            for &sid in set {
                match self.classify_seg(&part, &self.segs[sid]) {
                    GlClass::Front => {
                        nf += 1;
                        front_solid += 1;
                    }
                    GlClass::Back => {
                        nb += 1;
                        back_solid += 1;
                    }
                    GlClass::ColinearFront => nf += 1,
                    GlClass::ColinearBack => nb += 1,
                    GlClass::Split(..) => nsp += 1,
                }
            }
            let valid = if relaxed {
                // §C.2: a line separating segs of different sectors — both sides
                // non-empty, colinear segs counted (a two-sided shared line's
                // opposite colinear segs are what separate the sectors).
                (nf + nsp) > 0 && (nb + nsp) > 0
            } else {
                // §B.3: a line that genuinely partitions — a split, or
                // NON-colinear content on both sides. A splitter's own colinear
                // seg does not, alone, make its line a valid partition.
                nsp > 0 || (front_solid > 0 && back_solid > 0)
            };
            if !valid {
                continue;
            }
            let score = geom::partition_score(
                nf,
                nb,
                nsp,
                self.split_cost,
                self.aa_preference,
                pdx != 0 && pdy != 0,
            );
            let better = match *best {
                None => true,
                Some((bscore, bid)) => score < bscore || (score == bscore && cand < bid),
            };
            if better {
                *best = Some((score, cand));
            }
        }
    }

    /// The `i64` direction vector `verts[to] − verts[from]` in 16.16 space. A
    /// 16.16 delta can reach `2³²`, so the components are `i64` (they feed
    /// [`geom::clockwise_order`] / [`geom::counter_clockwise_order`], which widen
    /// to `i128` internally).
    fn dir_from(&self, from: usize, to: usize) -> (i64, i64) {
        let (fx, fy) = self.verts[from];
        let (tx, ty) = self.verts[to];
        (i64::from(tx) - i64::from(fx), i64::from(ty) - i64::from(fy))
    }

    /// Whether seg `sid` lies **entirely on** `part` (both endpoints within half a
    /// fixed unit of the partition line). The miniseg loop checks skip such segs
    /// when picking the loop-defining seg: a colinear seg is real geometry already
    /// covering the span, so counting it would make a miniseg duplicate it
    /// (Notes §Q2/§Q6, `nodebuild_gl.cpp:301`/`360`).
    fn seg_on_partition(&self, part: &GlPartition, sid: usize) -> bool {
        let s = self.segs[sid];
        Self::on_line(part, self.cross(part, s.v1)) && Self::on_line(part, self.cross(part, s.v2))
    }

    /// The exact `i128` event key of vertex `v` along `part`'s direction — the
    /// same projection [`record_event`](Self::record_event) keys events by, so a
    /// colinear seg's endpoints share keys with the events they bound.
    fn event_key(&self, part: &GlPartition, v: usize) -> i128 {
        let (vx, vy) = self.verts[v];
        i128::from(part.pdx) * (i128::from(vx) - i128::from(part.px))
            + i128::from(part.pdy) * (i128::from(vy) - i128::from(part.py))
    }

    /// The GL loop-start test (`CheckLoopStart`, Notes §Q2): the seg **ending** at
    /// `vertex` (the `segs2`/[`segs_ending_at`](Self::segs_ending_at) role) whose
    /// direction `verts[v1] − vertex` forms the **smallest clockwise angle** from
    /// the reference direction `(rdx, rdy)`, or `None` if the loop does not close
    /// on this side. Segs lying on the partition are skipped
    /// ([`seg_on_partition`](Self::seg_on_partition)) so a miniseg never duplicates
    /// real colinear geometry.
    ///
    /// The winner is rejected (`None`) when a seg **starting** at `vertex` (the
    /// opposite `segs`/[`segs_starting_at`](Self::segs_starting_at) list) either
    /// runs directly to `vertex2` — a real seg already spans this event pair — or
    /// forms a **strictly smaller** clockwise angle while not being the winner's
    /// partner (the interior lies on the wrong side). Ported line-for-line from
    /// `nodebuild_gl.cpp:282–339`, with [`geom::clockwise_order`] replacing BAM +
    /// `ANGLE_EPSILON` and [`seg_on_partition`](Self::seg_on_partition) replacing
    /// the `PointOnSide == 0 && diff < ANGLE_EPSILON` skip.
    fn check_loop_start(
        &self,
        part: &GlPartition,
        rdx: i64,
        rdy: i64,
        vertex: usize,
        vertex2: usize,
    ) -> Option<usize> {
        // Primary: segs ending at `vertex`, smallest clockwise angle from ref.
        let mut best: Option<usize> = None;
        for &sid in &self.segs_ending_at[vertex] {
            if self.seg_on_partition(part, sid) {
                continue; // a seg on the splitter never defines the loop
            }
            let (dx, dy) = self.dir_from(vertex, self.segs[sid].v1);
            best = Some(match best {
                None => sid,
                Some(b) => {
                    let (bx, by) = self.dir_from(vertex, self.segs[b].v1);
                    // `<=` matches ZDBSP's `diff <= bestang` (a later equal wins).
                    if geom::clockwise_order(rdx, rdy, dx, dy, bx, by).is_le() {
                        sid
                    } else {
                        b
                    }
                }
            });
        }
        let bestseg = best?;
        // Secondary: no seg starting at `vertex` may undercut the winner.
        let (bx, by) = self.dir_from(vertex, self.segs[bestseg].v1);
        for &sid in &self.segs_starting_at[vertex] {
            let seg = self.segs[sid];
            if seg.v2 == vertex2 {
                return None; // a real seg already spans prev→next
            }
            let (dx, dy) = self.dir_from(vertex, seg.v2);
            if geom::clockwise_order(rdx, rdy, dx, dy, bx, by).is_lt()
                && seg.partner != Some(bestseg)
            {
                return None; // interior is on the wrong side
            }
        }
        Some(bestseg)
    }

    /// The GL loop-end test (`CheckLoopEnd`, Notes §Q2): the mirror of
    /// [`check_loop_start`](Self::check_loop_start). It scans the seg **starting**
    /// at `vertex` (the `segs`/[`segs_starting_at`](Self::segs_starting_at) role)
    /// with the smallest angle, but from the **negated** reference `(−rdx, −rdy)`
    /// and in the **counter-clockwise** sense — ZDBSP's `CheckLoopEnd` minimizes
    /// `segAngle − (splitAngle + ANGLE_180)`, i.e. a CCW extremum from the
    /// 180°-rotated splitter (`nodebuild_gl.cpp:341–394`). The negation and the
    /// CCW sense are why this uses [`geom::counter_clockwise_order`] rather than
    /// the clockwise comparator; a single "smallest clockwise angle" rule would
    /// pick the seg 180° away (verified against source). The opposite (ending)
    /// list must not undercut the winner, same partner exemption as the mirror.
    // `similar_names`: `nrdx`/`nrdy` are the negated `rdx`/`rdy` reference deltas;
    // renaming them obscures the mirror-of-`check_loop_start` structure.
    #[allow(clippy::similar_names)]
    fn check_loop_end(
        &self,
        part: &GlPartition,
        rdx: i64,
        rdy: i64,
        vertex: usize,
    ) -> Option<usize> {
        // ZDBSP's `splitAngle + ANGLE_180`: the reference is the negated direction.
        let (nrdx, nrdy) = (-rdx, -rdy);
        // Primary: segs starting at `vertex`, smallest CCW angle from −ref.
        let mut best: Option<usize> = None;
        for &sid in &self.segs_starting_at[vertex] {
            if self.seg_on_partition(part, sid) {
                continue;
            }
            let (dx, dy) = self.dir_from(vertex, self.segs[sid].v2);
            best = Some(match best {
                None => sid,
                Some(b) => {
                    let (bx, by) = self.dir_from(vertex, self.segs[b].v2);
                    if geom::counter_clockwise_order(nrdx, nrdy, dx, dy, bx, by).is_le() {
                        sid
                    } else {
                        b
                    }
                }
            });
        }
        let bestseg = best?;
        // Secondary: no seg ending at `vertex` may undercut the winner.
        let (bx, by) = self.dir_from(vertex, self.segs[bestseg].v2);
        for &sid in &self.segs_ending_at[vertex] {
            let seg = self.segs[sid];
            let (dx, dy) = self.dir_from(vertex, seg.v1);
            if geom::counter_clockwise_order(nrdx, nrdy, dx, dy, bx, by).is_lt()
                && seg.partner != Some(bestseg)
            {
                return None;
            }
        }
        Some(bestseg)
    }

    /// Split-sharer repair (`FixSplitSharers`, Notes §Q1/§Q6): a colinear seg that
    /// spans **more than two** events (its endpoints' keys bracket one or more
    /// interior event keys) is force-split at each interior event vertex, its
    /// partner following in lockstep via [`split_seg_at`](Self::split_seg_at).
    /// Without it, minisegs would be added over overlapping colinear lines and
    /// partner linkage would be corrupted (`nodebuild_gl.cpp:64–160`). It runs
    /// **before** the interval walk so every colinear span is exactly one event
    /// wide by the time the loop checks run.
    ///
    /// Two-sided colinear segs are repaired through their front seg only (the
    /// back partner co-splits automatically); one-sided colinear segs are repaired
    /// on whichever side they routed to. Each new far fragment re-joins its
    /// parent's out-set and each parked partner fragment joins the opposite
    /// out-set — accumulated into locals first so the routed `front`/`back` are
    /// not aliased while `self` is mutated.
    ///
    /// # Errors
    ///
    /// Propagates [`split_seg_at`](Self::split_seg_at)'s
    /// [`NodeBuildError::TooManyElements`].
    fn fix_split_sharers(
        &mut self,
        part: &GlPartition,
        front: &mut Vec<usize>,
        back: &mut Vec<usize>,
    ) -> Result<(), NodeBuildError> {
        // Event vertices in exact partition order (snapshot: releases the borrow
        // of `self.events` before any mutation).
        let ordered: Vec<(i128, usize)> = self
            .events
            .events
            .iter()
            .map(|(&k, e)| (k, e.vertex))
            .collect();
        // Colinear segs to repair: every front colinear seg (its back partner
        // co-splits), plus one-sided back colinear segs (no partner to ride
        // along). Deduped and ordered for determinism (a colinear seg appears at
        // both of its endpoint events).
        let mut fronts: Vec<usize> = self
            .events
            .events
            .values()
            .flat_map(|e| e.colinear_front.iter().copied())
            .collect();
        fronts.sort_unstable();
        fronts.dedup();
        let mut backs: Vec<usize> = self
            .events
            .events
            .values()
            .flat_map(|e| e.colinear_back.iter().copied())
            .filter(|&sid| self.segs[sid].partner.is_none())
            .collect();
        backs.sort_unstable();
        backs.dedup();

        let mut add_front: Vec<usize> = Vec::new();
        let mut add_back: Vec<usize> = Vec::new();
        for sid in fronts {
            self.repair_colinear(part, sid, &ordered, true, &mut add_front, &mut add_back)?;
        }
        for sid in backs {
            self.repair_colinear(part, sid, &ordered, false, &mut add_front, &mut add_back)?;
        }
        front.extend(add_front);
        back.extend(add_back);
        Ok(())
    }

    /// Force-splits one colinear seg `sid` at every event strictly interior to its
    /// span, folding the fragments into `add_front`/`add_back` by side (see
    /// [`fix_split_sharers`](Self::fix_split_sharers)). `home_front` marks whether
    /// `sid` routed to the front out-set. New far fragments join the home side;
    /// each co-split partner's parked fragment (from
    /// [`split_seg_at`](Self::split_seg_at)) joins the opposite side.
    ///
    /// # Errors
    ///
    /// Propagates [`split_seg_at`](Self::split_seg_at)'s
    /// [`NodeBuildError::TooManyElements`].
    fn repair_colinear(
        &mut self,
        part: &GlPartition,
        sid: usize,
        ordered: &[(i128, usize)],
        home_front: bool,
        add_front: &mut Vec<usize>,
        add_back: &mut Vec<usize>,
    ) -> Result<(), NodeBuildError> {
        let s = self.segs[sid];
        let k1 = self.event_key(part, s.v1);
        let k2 = self.event_key(part, s.v2);
        let (lo, hi) = (k1.min(k2), k1.max(k2));
        // Interior event vertices, ordered from v1 toward v2 so each split keeps
        // the near half in place and continues on the far half.
        let mut interior: Vec<usize> = ordered
            .iter()
            .filter(|(k, _)| *k > lo && *k < hi)
            .map(|&(_, v)| v)
            .collect();
        if k1 > k2 {
            interior.reverse();
        }
        let mut cur = sid;
        for v in interior {
            let (mx, my) = self.verts[v];
            let partner_before = self.segs[cur].partner;
            let (_a, b) = self.split_seg_at(cur, mx, my)?;
            if home_front {
                add_front.push(b);
            } else {
                add_back.push(b);
            }
            // The partner's new co-split fragment was parked under the partner id;
            // drain it into the opposite out-set.
            if let Some(p) = partner_before
                && let Some(frags) = self.spawned.remove(&p)
            {
                if home_front {
                    add_back.extend(frags);
                } else {
                    add_front.extend(frags);
                }
            }
            cur = b;
        }
        Ok(())
    }

    /// Adds mirrored miniseg pairs across the partition (`AddMinisegs`, Notes §Q2),
    /// consuming the on-partition events left in [`self.events`](Self::events) by
    /// the immediately preceding [`split_set`](Self::split_set) call — the
    /// accumulator is not passed explicitly because it lives on `self` (Task 4's
    /// storage choice); callers must invoke `add_minisegs` before the next
    /// `split_set` overwrites it. `front`/`back` are the just-routed out-sets.
    ///
    /// First runs [`fix_split_sharers`](Self::fix_split_sharers), then walks the
    /// events in exact partition order. For each consecutive pair `(prev, next)` a
    /// mirrored pair is created **iff all four** loop checks return a seg (Notes
    /// §Q2's exact calls, with the back checks negating the partition direction):
    /// front loop-start at `prev`, back loop-start at `next`, front loop-end at
    /// `next`, back loop-end at `prev`. This is the interval-occupancy rule: the
    /// span is interior only when a real loop closes at both endpoints on both
    /// sides, so minisegs are never created in void space.
    ///
    /// On success the front miniseg `prev→next` (`linedef: None`, `side: 0`,
    /// `side_sector` from the front loop-start seg) and the back miniseg `next→prev`
    /// (`side: 1`, sector from the back loop-start seg) are created as mutual
    /// partners, registered in the incident lists, counted against the live-seg
    /// cap, and pushed to `front`/`back` respectively. The `side` field records
    /// facing only; the GL emission-side mapping (minisegs emit `side 0`) is Task
    /// 6's concern. Later spans see earlier minisegs in the incident lists, exactly
    /// as ZDBSP's `AddMiniseg` updates the per-vertex lists in place.
    ///
    /// # Errors
    ///
    /// [`NodeBuildError::TooManyElements`] (`kind: "segs"`) when the live-seg count
    /// exceeds [`MAX_EXTENDED_INDEX`], and any error from
    /// [`fix_split_sharers`](Self::fix_split_sharers).
    fn add_minisegs(
        &mut self,
        part: &GlPartition,
        front: &mut Vec<usize>,
        back: &mut Vec<usize>,
    ) -> Result<(), NodeBuildError> {
        // Overlapping colinear segs must be one event wide before the walk.
        self.fix_split_sharers(part, front, back)?;

        // Event vertices in exact partition order (BTreeMap iterates by key).
        let ordered: Vec<usize> = self.events.events.values().map(|e| e.vertex).collect();
        let (pdx, pdy) = (part.pdx, part.pdy);
        for pair in ordered.windows(2) {
            let (prev, next) = (pair[0], pair[1]);
            // All four loop checks must pass; the back checks use −partition dir.
            let Some(fseg1) = self.check_loop_start(part, pdx, pdy, prev, next) else {
                continue;
            };
            let Some(bseg1) = self.check_loop_start(part, -pdx, -pdy, next, prev) else {
                continue;
            };
            if self.check_loop_end(part, pdx, pdy, next).is_none() {
                continue; // front loop-end at next
            }
            if self.check_loop_end(part, -pdx, -pdy, prev).is_none() {
                continue; // back loop-end at prev
            }

            // Sectors copied from the loop-start segs (ZDBSP `AddMinisegs:195`).
            let f_sector = self.segs[fseg1].side_sector;
            let b_sector = self.segs[bseg1].side_sector;
            let front_id = self.segs.len();
            let back_id = front_id + 1;
            self.segs.push(GlWorkSeg {
                v1: prev,
                v2: next,
                linedef: None,
                side: 0,
                side_sector: f_sector,
                partner: Some(back_id),
            });
            self.segs.push(GlWorkSeg {
                v1: next,
                v2: prev,
                linedef: None,
                side: 1,
                side_sector: b_sector,
                partner: Some(front_id),
            });
            self.register_seg(front_id, prev, next);
            self.register_seg(back_id, next, prev);
            // Two new live segs; the cap is the runaway backstop.
            self.live_segs += 2;
            if self.live_segs > MAX_EXTENDED_INDEX {
                return Err(NodeBuildError::TooManyElements {
                    kind: "segs",
                    count: self.live_segs,
                    max: MAX_EXTENDED_INDEX,
                });
            }
            front.push(front_id);
            back.push(back_id);
        }
        Ok(())
    }

    /// Drains any co-split fragments parked under `sid` in
    /// [`spawned`](Self::spawned) onto `queue`. The single drain step shared by
    /// [`split_set`](Self::split_set)'s routing queue (drain point (a)) and
    /// [`expand_set`](Self::expand_set) (drain points (b)/(c)), so the three
    /// consumers can never diverge.
    fn drain_spawned(&mut self, sid: usize, queue: &mut VecDeque<usize>) {
        if let Some(parked) = self.spawned.remove(&sid) {
            queue.extend(parked);
        }
    }

    /// Transitively expands `set` by draining every parked co-split fragment it
    /// (directly or through a newly drained fragment) unlocks — spawn-table drain
    /// points (b) and (c) (see [`spawned`](Self::spawned)). Each id is emitted
    /// once, in a deterministic order; the drain uses the same
    /// [`drain_spawned`](Self::drain_spawned) step as `split_set`'s queue.
    ///
    /// Point (b) runs in [`process_split`](Self::process_split) before
    /// classification; point (c) runs in [`finish`](Self::finish) on each leaf, so
    /// an emitted leaf still picks up fragments a later sibling-subtree partition
    /// co-split into one of its segs.
    fn expand_set(&mut self, set: Vec<usize>) -> Vec<usize> {
        let mut out = Vec::with_capacity(set.len());
        let mut queue: VecDeque<usize> = set.into();
        while let Some(sid) = queue.pop_front() {
            self.drain_spawned(sid, &mut queue);
            out.push(sid);
        }
        out
    }

    /// Drives the explicit GL work stack (§C.6), mirroring the classic
    /// `Bsp::partition` (`nodes.rs`): each pop either makes a convex leaf or emits
    /// a node frame plus two child sets. No call recursion (Global Constraint 9).
    ///
    /// # Errors
    ///
    /// Propagates [`branch`](Self::branch) / [`process_split`](Self::process_split)
    /// errors.
    fn partition(&mut self) -> Result<(), NodeBuildError> {
        let root_set: Vec<usize> = (0..self.segs.len()).collect();
        let mut work: Vec<GlTask> = vec![GlTask::Split(root_set)];
        let mut done: Vec<TreeRef> = Vec::new();

        while let Some(task) = work.pop() {
            match task {
                GlTask::Split(set) => self.process_split(set, &mut work, &mut done)?,
                GlTask::Merge { px, py, dx, dy } => {
                    // Front was pushed last, so it completed first and sits beneath
                    // back on `done`. unreachable panic: every `Merge` is pushed
                    // with its two child `Split`s, each pushing one `done` entry.
                    let back = done.pop().expect("merge back child present");
                    let front = done.pop().expect("merge front child present");
                    self.tree_nodes.push(GlTreeNode {
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

    /// Processes one `Split` task: expand the parked co-split fragments (drain
    /// point (b)), then select a partition (§B) or make a convex leaf, honoring
    /// the single-sector / mixed-sector rule (§C.1–C.2) — the same semantics as
    /// the classic `process_split`.
    ///
    /// # Errors
    ///
    /// [`NodeBuildError::MixedSectorSubsector`] (strict) for a multi-sector fan
    /// with no separating line, or [`branch`](Self::branch)'s errors.
    fn process_split(
        &mut self,
        set: Vec<usize>,
        work: &mut Vec<GlTask>,
        done: &mut Vec<TreeRef>,
    ) -> Result<(), NodeBuildError> {
        // Drain point (b): expand parked co-split fragments before classifying.
        let set = self.expand_set(set);
        if let Some(cand) = self.select(&set, false) {
            return self.branch(cand, &set, work);
        }
        if self.single_sector(&set) {
            self.push_leaf(set, done);
            return Ok(());
        }
        // §C.2: a multi-sector convex region — retry with the sector-separating
        // relaxation; a separating line is a normal branch.
        if let Some(cand) = self.select(&set, true) {
            return self.branch(cand, &set, work);
        }
        // A mixed-sector fan: strict rejects, lenient accepts the leaf and warns.
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

    /// Emits a node for splitter `cand`: builds the partition, routes the set
    /// ([`split_set`](Self::split_set)), adds minisegs
    /// ([`add_minisegs`](Self::add_minisegs)) into the child sets, guards against
    /// an empty side, then pushes a `Merge` frame plus the two child `Split`s
    /// (front last, so it is processed first). Minisegs enter the child sets
    /// **before** the empty-side guard — a miniseg may legitimately populate an
    /// otherwise-empty side (ADR-0026 §2).
    ///
    /// # Errors
    ///
    /// [`split_set`](Self::split_set) / [`add_minisegs`](Self::add_minisegs)
    /// overflow errors, or [`NodeBuildError::DegeneratePartition`] when a side is
    /// still empty after minisegs (a fuzz-safe backstop, Global Constraint 9;
    /// well-formed geometry never trips it).
    fn branch(
        &mut self,
        cand: usize,
        set: &[usize],
        work: &mut Vec<GlTask>,
    ) -> Result<(), NodeBuildError> {
        let s = self.segs[cand];
        let (px, py) = self.verts[s.v1];
        let (x2, y2) = self.verts[s.v2];
        // `select` only returns a candidate whose 16.16 deltas fit `i32` (§B.1),
        // so the true delta fits `i32` and this widened subtraction never
        // overflows; the cast back is therefore lossless.
        let pdx = i64::from(x2) - i64::from(px);
        let pdy = i64::from(y2) - i64::from(py);
        debug_assert!(partition_delta_fits_gl(pdx, pdy));
        #[allow(clippy::cast_possible_truncation)]
        let (dx, dy) = (pdx as i32, pdy as i32);
        let part = GlPartition::new(px, py, dx, dy);

        let (mut front, mut back) = self.split_set(set.to_vec(), &part)?;
        // Minisegs may make a previously-empty side non-empty; the guard runs
        // after (ADR-0026 §2, controller resolution 2).
        self.add_minisegs(&part, &mut front, &mut back)?;
        if front.is_empty() || back.is_empty() {
            return Err(NodeBuildError::DegeneratePartition {
                set_segs: set.len(),
            });
        }

        work.push(GlTask::Merge { px, py, dx, dy });
        work.push(GlTask::Split(back));
        work.push(GlTask::Split(front));
        Ok(())
    }

    /// Records `set` as the next convex leaf (= final subsector) and pushes its
    /// ref onto `done`.
    fn push_leaf(&mut self, set: Vec<usize>, done: &mut Vec<TreeRef>) {
        self.leaves.push(set);
        done.push(TreeRef::Leaf(self.leaves.len() - 1));
    }

    /// The `i128` direction of vertex `v` about the leaf midpoint, scaled by the
    /// endpoint count `n` to stay exact without division (controller resolution
    /// 3): `(v.x·n − sum_x, v.y·n − sum_y)`, where `(sum_x, sum_y)` are the `i64`
    /// coordinate sums of the leaf's endpoints. Scaling by the positive `n`
    /// preserves the angle, so [`clockwise_from`] orders these exactly.
    fn midpoint_dir(&self, v: usize, n: i64, sum_x: i64, sum_y: i64) -> (i128, i128) {
        let (vx, vy) = self.verts[v];
        (
            i128::from(vx) * i128::from(n) - i128::from(sum_x),
            i128::from(vy) * i128::from(n) - i128::from(sum_y),
        )
    }

    /// Appends a partner-less **connecting miniseg** `from → to` to the working
    /// seg arena (ADR-0026 §2 leaf closing, Notes §Q5 `PushConnectingGLSeg`):
    /// `linedef: None`, `partner: None`, `side: 0`, `side_sector` from the leaf's
    /// sector. It becomes a real seg in the output subsector loop, so it
    /// is cap-checked and registered in the incident lists like any other seg.
    /// Returns its working id.
    ///
    /// # Errors
    ///
    /// [`NodeBuildError::TooManyElements`] (`kind: "segs"`) if the live-seg count
    /// exceeds [`MAX_EXTENDED_INDEX`].
    fn push_connecting_miniseg(
        &mut self,
        from: usize,
        to: usize,
        side_sector: usize,
    ) -> Result<usize, NodeBuildError> {
        let id = self.segs.len();
        self.segs.push(GlWorkSeg {
            v1: from,
            v2: to,
            linedef: None,
            side: 0,
            side_sector,
            partner: None,
        });
        self.register_seg(id, from, to);
        self.live_segs += 1;
        if self.live_segs > MAX_EXTENDED_INDEX {
            return Err(NodeBuildError::TooManyElements {
                kind: "segs",
                count: self.live_segs,
                max: MAX_EXTENDED_INDEX,
            });
        }
        Ok(id)
    }

    /// Orders one convex leaf's segs into a single closed loop (Notes §Q5
    /// `CloseSubsector`, ADR-0026 §2): a greedy continuation-first walk — the next
    /// seg is the one whose `v1` equals the previous seg's `v2`, else the one
    /// whose `v1` forms the smallest clockwise angular step about the leaf
    /// midpoint from the previous `v2`'s direction. When no continuation exists a
    /// partner-less connecting miniseg bridges the gap
    /// ([`push_connecting_miniseg`](Self::push_connecting_miniseg)); the final gap
    /// back to the first vertex is closed the same way. Connecting minisegs are
    /// **normal operation in both modes** (ZDBSP does this unconditionally).
    ///
    /// A leaf with fewer than 3 distinct vertices after expansion cannot form a
    /// loop: strict mode returns [`NodeBuildError::DegenerateLeaf`], lenient warns
    /// ([`NodeBuildWarning::DegenerateLeaf`]) and returns the segs unordered.
    ///
    /// # Errors
    ///
    /// [`NodeBuildError::DegenerateLeaf`] (strict) for a sub-3-vertex leaf, and
    /// [`push_connecting_miniseg`](Self::push_connecting_miniseg)'s overflow.
    fn close_leaf(&mut self, segs: Vec<usize>) -> Result<Vec<usize>, NodeBuildError> {
        // Distinct-vertex guard: a loop needs at least a triangle.
        let mut distinct: Vec<usize> = segs
            .iter()
            .flat_map(|&s| [self.segs[s].v1, self.segs[s].v2])
            .collect();
        distinct.sort_unstable();
        distinct.dedup();
        if distinct.len() < 3 {
            return match self.strictness {
                Strictness::Strict => Err(NodeBuildError::DegenerateLeaf {
                    subsector_segs: segs.len(),
                }),
                Strictness::Lenient => {
                    self.warnings.push(NodeBuildWarning::DegenerateLeaf {
                        subsector_segs: segs.len(),
                    });
                    Ok(segs)
                }
            };
        }

        // Midpoint pivot (exact, no division): endpoint coordinate sums and count.
        let n = i64::try_from(segs.len())
            .unwrap_or(i64::MAX)
            .saturating_mul(2);
        let (mut sum_x, mut sum_y) = (0i64, 0i64);
        for &s in &segs {
            for e in [self.segs[s].v1, self.segs[s].v2] {
                let (x, y) = self.verts[e];
                sum_x += i64::from(x);
                sum_y += i64::from(y);
            }
        }
        // The sector for connecting minisegs — the leaf's first seg's sector (all
        // equal in a single-sector leaf; the render-convention sector for a
        // lenient mixed-sector leaf).
        let leaf_sector = self.segs[segs[0]].side_sector;

        let mut remaining = segs;
        let start = remaining.remove(0);
        let start_v1 = self.segs[start].v1;
        let mut order = vec![start];
        let mut prev = start;
        while !remaining.is_empty() {
            let target = self.segs[prev].v2;
            if let Some(pos) = remaining.iter().position(|&c| self.segs[c].v1 == target) {
                // Immediate continuation: the loop already connects here.
                let c = remaining.remove(pos);
                order.push(c);
                prev = c;
                continue;
            }
            // No continuation: pick the smallest clockwise step about the midpoint
            // and bridge the gap with a connecting miniseg.
            let reference = self.midpoint_dir(target, n, sum_x, sum_y);
            let mut best = 0usize;
            for i in 1..remaining.len() {
                let a = self.midpoint_dir(self.segs[remaining[i]].v1, n, sum_x, sum_y);
                let b = self.midpoint_dir(self.segs[remaining[best]].v1, n, sum_x, sum_y);
                if clockwise_from(reference, a, b).is_lt() {
                    best = i;
                }
            }
            let c = remaining.remove(best);
            let mini = self.push_connecting_miniseg(target, self.segs[c].v1, leaf_sector)?;
            order.push(mini);
            order.push(c);
            prev = c;
        }
        // Close the final gap back to the first vertex.
        if self.segs[prev].v2 != start_v1 {
            let mini = self.push_connecting_miniseg(self.segs[prev].v2, start_v1, leaf_sector)?;
            order.push(mini);
        }
        Ok(order)
    }

    /// The whole-unit bbox `[top, bottom, left, right]` of a leaf's seg
    /// endpoints, rounded **outward** from 16.16 space (controller resolution 6):
    /// `top`/`right` ceil, `bottom`/`left` floor, so the box never clips a seg.
    fn leaf_bbox_whole(&self, order: &[usize]) -> [i32; 4] {
        let (mut max_y, mut min_y, mut min_x, mut max_x) = (i32::MIN, i32::MAX, i32::MAX, i32::MIN);
        for &sid in order {
            let s = self.segs[sid];
            for e in [s.v1, s.v2] {
                let (x, y) = self.verts[e];
                max_y = max_y.max(y);
                min_y = min_y.min(y);
                min_x = min_x.min(x);
                max_x = max_x.max(x);
            }
        }
        [
            ceil_fixed(max_y),
            floor_fixed(min_y),
            floor_fixed(min_x),
            ceil_fixed(max_x),
        ]
    }

    /// Flattens the tree arena into [`BuiltGlNodes`] (ADR-0026 §2): closes every
    /// leaf into a subsector loop (spawn drain point (c) first), emits segs as
    /// contiguous per-subsector runs with working ids remapped to final
    /// [`GlSegIdx`], builds subsectors and post-order nodes (root last) with
    /// outward-rounded bboxes unioned bottom-up, and enforces the extended arena
    /// ceilings.
    ///
    /// # Errors
    ///
    /// [`close_leaf`](Self::close_leaf)'s errors, and
    /// [`NodeBuildError::TooManyElements`] when the GL vertex, seg, subsector, or
    /// node arena exceeds [`MAX_EXTENDED_INDEX`] (GL is always extended, so the
    /// classic vanilla soft ceilings and the `u16` linedef gate do not apply).
    fn finish(mut self) -> Result<(BuiltGlNodes, Vec<NodeBuildWarning>), NodeBuildError> {
        // (c) Re-expand and close each leaf into an ordered loop. Taking `leaves`
        // out first frees `self` for the mutation `close_leaf` needs.
        let leaves = std::mem::take(&mut self.leaves);
        let mut leaf_orders: Vec<Vec<usize>> = Vec::with_capacity(leaves.len());
        for leaf in leaves {
            let expanded = self.expand_set(leaf);
            leaf_orders.push(self.close_leaf(expanded)?);
        }

        // Emission-order remap: working seg id → final `GlSegIdx` (Notes §Q1 — a
        // pure index translation of already-maintained partner linkage).
        let total: usize = leaf_orders.iter().map(Vec::len).sum();
        let mut remap = vec![usize::MAX; self.segs.len()];
        let mut ordered_ids: Vec<usize> = Vec::with_capacity(total);
        for order in &leaf_orders {
            for &sid in order {
                remap[sid] = ordered_ids.len();
                ordered_ids.push(sid);
            }
        }

        // Final segs, in subsector loop order.
        let mut segs: Vec<GlSeg> = Vec::with_capacity(total);
        for &sid in &ordered_ids {
            let s = self.segs[sid];
            segs.push(GlSeg {
                start: self.vertex_ref(s.v1),
                end: self.vertex_ref(s.v2),
                linedef: s.linedef.map(LinedefIdx),
                side: s.side,
                partner: s.partner.map(|p| {
                    debug_assert!(remap[p] != usize::MAX, "every partner is emitted");
                    GlSegIdx(remap[p])
                }),
            });
        }
        // The involution survives the pure index remap.
        debug_assert!(segs.iter().enumerate().all(|(i, s)| match s.partner {
            Some(p) => segs[p.0].partner == Some(GlSegIdx(i)),
            None => true,
        }));

        // Subsectors own contiguous runs in emission order.
        let mut subsectors: Vec<GlSubsector> = Vec::with_capacity(leaf_orders.len());
        let mut start = 0usize;
        for order in &leaf_orders {
            subsectors.push(GlSubsector {
                segs: start..start + order.len(),
            });
            start += order.len();
        }

        // Leaf bboxes (fixed → outward whole units), then node bboxes bottom-up.
        let leaf_bboxes: Vec<[i32; 4]> = leaf_orders
            .iter()
            .map(|order| self.leaf_bbox_whole(order))
            .collect();
        let mut node_bboxes: Vec<[i32; 4]> = Vec::with_capacity(self.tree_nodes.len());
        for tn in &self.tree_nodes {
            let fb = bbox_of_gl_ref(tn.front, &leaf_bboxes, &node_bboxes);
            let bb = bbox_of_gl_ref(tn.back, &leaf_bboxes, &node_bboxes);
            node_bboxes.push(geom::bbox_union(fb, bb));
        }

        // Final nodes: front = right child, back = left child (root last).
        let mut nodes: Vec<BuiltGlNode> = Vec::with_capacity(self.tree_nodes.len());
        for tn in &self.tree_nodes {
            nodes.push(BuiltGlNode {
                x: tn.px,
                y: tn.py,
                dx: tn.dx,
                dy: tn.dy,
                right_bbox: bbox_of_gl_ref(tn.front, &leaf_bboxes, &node_bboxes),
                left_bbox: bbox_of_gl_ref(tn.back, &leaf_bboxes, &node_bboxes),
                right: child_of_gl_ref(tn.front),
                left: child_of_gl_ref(tn.back),
            });
        }

        // Extended arena ceilings (GL is always extended — no vanilla soft cap).
        let vertex_count = self.map_vertex_count + self.gl_vertices.len();
        check_gl_ceiling("vertices", vertex_count)?;
        check_gl_ceiling("segs", segs.len())?;
        check_gl_ceiling("subsectors", subsectors.len())?;
        check_gl_ceiling("nodes", nodes.len())?;

        let built = BuiltGlNodes {
            gl_vertices: self.gl_vertices,
            segs,
            subsectors,
            nodes,
        };
        // Post-order flatten places the root last (or leaves `nodes` empty for a
        // single-subsector map).
        debug_assert!(match self.root {
            Some(TreeRef::Node(k)) => k + 1 == built.nodes.len(),
            Some(TreeRef::Leaf(_)) => built.nodes.is_empty(),
            None => built.subsectors.is_empty(),
        });
        Ok((built, self.warnings))
    }
}

/// Whether a partition delta fits the XGL3 on-disk `i32` node `dx`/`dy` field
/// (§B.1): a seg can serve as a splitter only if its 16.16 `v2 - v1` fits `i32`
/// on both axes. The fixed-space analog of the classic `partition_delta_fits`
/// (whose ceiling is the on-disk `i16`); the range is the full signed `i32`.
fn partition_delta_fits_gl(pdx: i64, pdy: i64) -> bool {
    i32::try_from(pdx).is_ok() && i32::try_from(pdy).is_ok()
}

/// Integer round-half-away-from-zero division of `a / b` (`b != 0`): the exact
/// rounding used by [`GlBsp::intersection`] in place of ZDBSP's `double` +
/// truncation. Ties (`|remainder| · 2 == |b|`) round away from zero; exact
/// quotients are returned unchanged. Correct in every sign quadrant.
fn round_half_away_rational(a: i128, b: i128) -> i128 {
    debug_assert!(
        b != 0,
        "round_half_away_rational requires a nonzero denominator"
    );
    // Normalize the denominator positive so the truncated remainder's sign
    // tracks the numerator's, making the tie test symmetric across quadrants.
    let (a, b) = if b < 0 { (-a, -b) } else { (a, b) };
    let q = a / b;
    let r = a % b;
    if 2 * r.abs() >= b { q + a.signum() } else { q }
}

/// One step on the GL build stack (Global Constraint 9: no call recursion), the
/// GL twin of the classic `Task`.
enum GlTask {
    /// Partition (or make a leaf of) this set of seg ids.
    Split(Vec<usize>),
    /// Combine the two child results on the `done` stack into a node.
    Merge {
        /// Partition-line start `x` (16.16).
        px: i32,
        /// Partition-line start `y` (16.16).
        py: i32,
        /// Partition-line `dx` (16.16).
        dx: i32,
        /// Partition-line `dy` (16.16).
        dy: i32,
    },
}

/// Orders directions `a` and `b` by clockwise angle from reference `r` (all
/// exact `i128`), returning [`Ordering::Less`](core::cmp::Ordering::Less) when
/// `a` is the smaller clockwise step. The `i128`-width twin of
/// [`geom::clockwise_order`](super::geom) for the leaf-midpoint pivot (controller
/// resolution 3): the scaled midpoint directions `v·n − sum` exceed `i64`, and —
/// crucially — the within-rank tiebreak is computed as the **original** cross
/// `a×b` (one product level), where `geom`'s frame-space cross double-scales by
/// `|r|²` and would overflow `i128` for these magnitudes. Ranks bucket by the
/// sign of `r×d` (one product level), monotone in clockwise angle from `r`.
fn clockwise_from(r: (i128, i128), a: (i128, i128), b: (i128, i128)) -> core::cmp::Ordering {
    use core::cmp::Ordering;
    // Clockwise half-plane rank of `d` about `r`: 0 = on `r` (forward), 1 = right
    // (clockwise) half, 2 = anti-`r`, 3 = left (CCW) half — monotone in the
    // clockwise angle, so a smaller rank orders first.
    let rank = |d: (i128, i128)| -> u8 {
        let cross = r.0 * d.1 - r.1 * d.0; // r × d
        match cross.cmp(&0) {
            Ordering::Equal => {
                let dot = r.0 * d.0 + r.1 * d.1; // r · d
                if dot > 0 { 0 } else { 2 }
            }
            Ordering::Less => 1,
            Ordering::Greater => 3,
        }
    };
    match rank(a).cmp(&rank(b)) {
        // Same rank: `a` is clockwise-before `b` iff `a × b < 0`.
        Ordering::Equal => (a.0 * b.1 - a.1 * b.0).cmp(&0),
        other => other,
    }
}

/// The floor of a 16.16 fixed-point value in whole map units. Arithmetic right
/// shift is floor for every sign (it rounds toward negative infinity).
fn floor_fixed(v: i32) -> i32 {
    v >> 16
}

/// The ceil of a 16.16 fixed-point value in whole map units: the floor plus one
/// when a fractional part is present. Correct for every sign (the fractional bits
/// are nonzero exactly when the value is not a whole unit).
fn ceil_fixed(v: i32) -> i32 {
    floor_fixed(v) + i32::from(v & 0xFFFF != 0)
}

/// The whole-unit bbox of a GL tree child, from the already-computed leaf/node
/// bbox tables (the GL twin of the classic `bbox_of_ref`).
fn bbox_of_gl_ref(child: TreeRef, leaf_bboxes: &[[i32; 4]], node_bboxes: &[[i32; 4]]) -> [i32; 4] {
    match child {
        TreeRef::Leaf(i) => leaf_bboxes[i],
        TreeRef::Node(k) => node_bboxes[k],
    }
}

/// The [`GlNodeChild`] a GL tree ref resolves to.
fn child_of_gl_ref(child: TreeRef) -> GlNodeChild {
    match child {
        TreeRef::Leaf(i) => GlNodeChild::Subsector(GlSubsectorIdx(i)),
        TreeRef::Node(k) => GlNodeChild::Node(GlNodeIdx(k)),
    }
}

/// The extended-arena ceiling (controller resolution 8): GL is always an extended
/// format, so every arena is checked against [`MAX_EXTENDED_INDEX`] in **both**
/// strictness modes — there is no vanilla `u16` soft ceiling, and no `u16`
/// linedef gate (GL seg linedef refs are 32-bit capable; emission ceilings are
/// #364's concern).
fn check_gl_ceiling(kind: &'static str, count: usize) -> Result<(), NodeBuildError> {
    if count > MAX_EXTENDED_INDEX {
        return Err(NodeBuildError::TooManyElements {
            kind,
            count,
            max: MAX_EXTENDED_INDEX,
        });
    }
    Ok(())
}

/// One internal GL BSP node produced by [`build_gl_nodes`] (ADR-0026 §1).
///
/// Partition line `(x, y)` + `(dx, dy)` are 16.16 fixed-point (the GL formats
/// carry sub-unit partition geometry); the child bboxes are whole map units,
/// `[top, bottom, left, right]`, rounded outward from the fixed-space leaf
/// extents. `right`/`left` are the front/back children (front = right, Global
/// Constraint 7). The arena is post-order, so the root is the last element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct BuiltGlNode {
    /// Partition-line start `x` (16.16 fixed-point).
    pub x: i32,
    /// Partition-line start `y` (16.16 fixed-point).
    pub y: i32,
    /// Partition-line `dx` from `(x, y)` (16.16 fixed-point).
    pub dx: i32,
    /// Partition-line `dy` from `(x, y)` (16.16 fixed-point).
    pub dy: i32,
    /// Right (front) child bbox `[top, bottom, left, right]`, whole map units.
    pub right_bbox: [i32; 4],
    /// Left (back) child bbox `[top, bottom, left, right]`, whole map units.
    pub left_bbox: [i32; 4],
    /// The right (front) child: a GL node or a GL subsector leaf.
    pub right: GlNodeChild,
    /// The left (back) child: a GL node or a GL subsector leaf.
    pub left: GlNodeChild,
}

/// A built GL BSP tree (ADR-0026 §1, #363): the arenas a writer serializes to the
/// `GL_VERT`/`GL_SEGS`/`GL_SSECT`/`GL_NODES` (or `XGL*`) lumps.
///
/// # Index-domain invariants
///
/// [`validate`](Self::validate) (run by [`new`](Self::new)) upholds all of:
///
/// - [`subsectors`](Self::subsectors) own **contiguous** [`segs`](Self::segs)
///   runs partitioning the arena exactly from `0`.
/// - Every [`GlVertexRef`] is in range: a `Normal` below `orig_vertex_count`, a
///   `Gl` below [`gl_vertices`](Self::gl_vertices)`.len()`.
/// - Every [`nodes`](Self::nodes) child index is in range for its arena.
/// - [`segs`](Self::segs) partner links form a mirrored involution
///   (`partner[partner[i]] == i`, never self, and spans mirror).
/// - Each subsector's seg run is a **closed loop** (`seg.end == next.start`
///   cyclically).
///
/// [`build_gl_nodes`] output upholds them by construction (with one lenient-mode
/// exception for loop closure — see [`validate`](Self::validate)).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct BuiltGlNodes {
    /// The `GL_VERT` split vertices created while partitioning, in creation
    /// order. Seg refs of kind [`GlVertexRef::Gl`] index this arena.
    pub gl_vertices: Vec<GlVertex>,
    /// The `GL_SEGS` arena, ordered so each subsector owns a contiguous run.
    pub segs: Vec<GlSeg>,
    /// The `GL_SSECT` arena: one leaf per convex region, each a contiguous run
    /// into [`segs`](Self::segs).
    pub subsectors: Vec<GlSubsector>,
    /// The `GL_NODES` arena: the internal tree in post-order (root last).
    pub nodes: Vec<BuiltGlNode>,
}

impl BuiltGlNodes {
    /// Assembles a `BuiltGlNodes` from its arenas and validates the structural
    /// invariants via [`validate`](Self::validate).
    ///
    /// `#[non_exhaustive]` blocks struct-literal construction from outside this
    /// crate — this is the public constructor a downstream crate (a hand-built
    /// fixture, or an alternative GL nodebuilder) uses. [`build_gl_nodes`] itself
    /// constructs the type in-crate and is correct by construction, so it bypasses
    /// this check.
    ///
    /// `orig_vertex_count` is the owning map's `VERTEXES` record count.
    ///
    /// # Errors
    ///
    /// [`NodeBuildError::InvalidStructure`] if the arenas violate an invariant
    /// (both strictness modes).
    pub fn new(
        gl_vertices: Vec<GlVertex>,
        segs: Vec<GlSeg>,
        subsectors: Vec<GlSubsector>,
        nodes: Vec<BuiltGlNode>,
        orig_vertex_count: usize,
    ) -> Result<Self, NodeBuildError> {
        let built = Self {
            gl_vertices,
            segs,
            subsectors,
            nodes,
        };
        built.validate(orig_vertex_count)?;
        Ok(built)
    }

    /// Checks this `BuiltGlNodes` against the type's [index-domain
    /// invariants](Self#index-domain-invariants). O(n) over the arenas, iterative,
    /// identical in both strictness modes. [`build_gl_nodes`] strict-mode output
    /// always passes; lenient output passes too **except** that a recovered
    /// degenerate leaf ([`NodeBuildWarning::DegenerateLeaf`]) is emitted as-is and
    /// may violate loop closure — the warning is the signal. Like the classic
    /// [`BuiltNodes::validate`](super::BuiltNodes), it checks only index-domain
    /// structure, not BSP semantics (node acyclicity, reachability, or whether a
    /// loop is geometrically convex).
    ///
    /// `orig_vertex_count` is the owning map's `VERTEXES` record count; the
    /// combined vertex domain is `Normal` refs below it and `Gl` refs below
    /// [`gl_vertices`](Self::gl_vertices)`.len()`.
    ///
    /// # Errors
    ///
    /// [`NodeBuildError::InvalidStructure`] on the first violated invariant,
    /// naming the offending element and bound.
    pub fn validate(&self, orig_vertex_count: usize) -> Result<(), NodeBuildError> {
        // (1) Subsector seg ranges partition `segs` exactly, contiguous from 0.
        let mut expected_start = 0usize;
        for (i, ss) in self.subsectors.iter().enumerate() {
            if ss.segs.start != expected_start || ss.segs.end < ss.segs.start {
                return Err(NodeStructureError::SubsectorRange {
                    subsector: i,
                    start: ss.segs.start,
                    end: ss.segs.end,
                    expected_start,
                }
                .into());
            }
            expected_start = ss.segs.end;
        }
        if expected_start != self.segs.len() {
            return Err(NodeStructureError::SubsectorPartition {
                covered: expected_start,
                segs: self.segs.len(),
            }
            .into());
        }

        // (2) Every GL vertex ref is in range for the arena it addresses.
        for (i, s) in self.segs.iter().enumerate() {
            for r in [s.start, s.end] {
                let (idx, bound) = match r {
                    GlVertexRef::Normal(v) => (v.0, orig_vertex_count),
                    GlVertexRef::Gl(v) => (v.0, self.gl_vertices.len()),
                };
                if idx >= bound {
                    return Err(NodeStructureError::GlVertexRef { seg: i, bound }.into());
                }
            }
        }

        // (3) Node child indices in range for their arena.
        for (i, n) in self.nodes.iter().enumerate() {
            for child in [n.right, n.left] {
                match child {
                    GlNodeChild::Node(k) if k.0 >= self.nodes.len() => {
                        return Err(NodeStructureError::NodeChild {
                            node: i,
                            arena: "node",
                            child: k.0,
                            bound: self.nodes.len(),
                        }
                        .into());
                    }
                    GlNodeChild::Subsector(k) if k.0 >= self.subsectors.len() => {
                        return Err(NodeStructureError::NodeChild {
                            node: i,
                            arena: "subsector",
                            child: k.0,
                            bound: self.subsectors.len(),
                        }
                        .into());
                    }
                    _ => {}
                }
            }
        }

        // (4) Partner links form a mirrored involution (never self).
        for (i, s) in self.segs.iter().enumerate() {
            if let Some(p) = s.partner {
                let ok = p.0 != i
                    && p.0 < self.segs.len()
                    && self.segs[p.0].partner == Some(GlSegIdx(i))
                    && self.segs[p.0].start == s.end
                    && self.segs[p.0].end == s.start;
                if !ok {
                    return Err(NodeStructureError::PartnerAsymmetry {
                        seg: i,
                        partner: p.0,
                    }
                    .into());
                }
            }
        }

        // (5) Each subsector's seg run is a closed loop (cyclic end == next start).
        for (si, ss) in self.subsectors.iter().enumerate() {
            let run = ss.segs.clone();
            let len = run.end - run.start;
            for k in 0..len {
                let cur = run.start + k;
                let next = run.start + (k + 1) % len;
                if self.segs[cur].end != self.segs[next].start {
                    return Err(NodeStructureError::OpenLoop {
                        subsector: si,
                        seg: cur,
                    }
                    .into());
                }
            }
        }

        Ok(())
    }
}

/// Builds a map's GL BSP nodes (ADR-0026, #363): the clean-room GL kernel — it
/// narrows the vertex arena through the shared write path, widens to 16.16, forms
/// one seg per present linedef side (partnered across two-sided lines), then
/// recursively partitions with an explicit work stack, adding minisegs across
/// each partition and closing every convex leaf into a GL subsector loop.
///
/// # Engine conventions (Global Constraint 7)
///
/// - **Side of a partition.** `cross > 0` is front, `cross < 0` is back; the
///   front child is the node's right child (as in the classic
///   [`build_nodes`](super::build_nodes)).
/// - **GL is always an extended format.** Counts and vertex/seg refs are 32-bit,
///   so the arenas are checked only against the extended `MAX_EXTENDED_INDEX`
///   ceiling (2³¹) — the classic
///   `u16` vertex/seg soft ceiling and the `u16` linedef-count gate are
///   **intentionally absent** (GL never targets vanilla; a GL seg's linedef ref
///   is 32-bit capable, and any emission-side ceiling is #364's concern).
///
/// # Errors
///
/// - [`NodeBuildError::EmptyGeometry`] (both modes) when the map has no vertices,
///   linedefs, sidedefs, or sectors — or when no linedef yields a seg.
/// - [`NodeBuildError::Write`] wrapping a [`DoomWriteError`](crate::map::DoomWriteError)
///   from the shared narrowing pass (strict-mode out-of-range coordinate).
/// - [`NodeBuildError::MixedSectorSubsector`] (strict) for a convex region
///   spanning multiple sectors with no separating seg line; lenient warns and
///   accepts the leaf.
/// - [`NodeBuildError::DegenerateLeaf`] (strict) for a convex leaf of fewer than
///   3 distinct vertices; lenient warns and emits it as-is.
/// - [`NodeBuildError::DegeneratePartition`] (both modes) — a fuzz-safe backstop
///   — when a selected partition leaves a side empty even after minisegs.
/// - [`NodeBuildError::TooManyElements`] (both modes) when the GL vertex, seg,
///   subsector, or node arena exceeds the extended `MAX_EXTENDED_INDEX` ceiling
///   (2³¹).
pub fn build_gl_nodes(
    map: &Map,
    opts: &NodeBuildOptions,
) -> Result<(BuiltGlNodes, Vec<NodeBuildWarning>), NodeBuildError> {
    // Empty-geometry gate — identical to `build_nodes`.
    if map.vertices().is_empty()
        || map.linedefs().is_empty()
        || map.sidedefs().is_empty()
        || map.sectors().is_empty()
    {
        return Err(NodeBuildError::EmptyGeometry);
    }
    // No `check_linedef_count` u16 gate: GL seg linedef refs are format-wide
    // 32-bit, so a >65,535-linedef map is representable here; emission ceilings
    // are #364's concern (see the engine-conventions block above).

    let mut bsp = GlBsp::new(map, opts)?;
    bsp.build_initial_segs();
    if bsp.segs.is_empty() {
        // Every linedef was zero-length or sideless: nothing to partition.
        return Err(NodeBuildError::EmptyGeometry);
    }
    bsp.partition()?;
    bsp.finish()
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

    /// Integer round-half-away-from-zero division, exact in every quadrant.
    #[test]
    fn round_half_away_rational_rounds_half_away_from_zero() {
        assert_eq!(round_half_away_rational(1, 2), 1);
        assert_eq!(round_half_away_rational(-1, 2), -1);
        assert_eq!(round_half_away_rational(3, 4), 1);
        assert_eq!(round_half_away_rational(7, 2), 4);
        // Exact division is returned unchanged.
        assert_eq!(round_half_away_rational(4, 2), 2);
        assert_eq!(round_half_away_rational(-6, 3), -2);
        // Sign combinations: the denominator's sign must not skew the rounding.
        assert_eq!(round_half_away_rational(1, -2), -1);
        assert_eq!(round_half_away_rational(-1, -2), 1);
        assert_eq!(round_half_away_rational(-3, 4), -1);
    }

    /// A candidate's fixed-space deltas may serve as a splitter only when both
    /// fit the XGL3 on-disk `i32` `dx`/`dy` field.
    #[test]
    fn partition_delta_fits_gl_bounds() {
        // The widest 16.16 delta a Doom `i16` coordinate can span — `i16::MIN`
        // widened by 16 bits — is exactly `i32::MIN`, and still fits.
        let widest = i64::from(i32::from(i16::MIN) << 16);
        assert!(partition_delta_fits_gl(widest, 0));
        assert!(partition_delta_fits_gl(0, widest));
        // One past `i32::MAX` does not.
        assert!(!partition_delta_fits_gl(1_i64 << 31, 0));
        assert!(!partition_delta_fits_gl(0, 1_i64 << 31));
    }

    /// A seg lying on the partition routes by orientation: same direction as the
    /// partition → `ColinearFront`, reversed → `ColinearBack` (Notes §Q6).
    #[test]
    fn colinear_segs_route_by_orientation() {
        let map = build_map(&[(0.0, 0.0), (64.0, 0.0)], &[(0, 1, Some(0), None)]);
        let mut bsp = GlBsp::new(&map, &NodeBuildOptions::strict()).unwrap();
        bsp.verts = vec![(0, 0), (64, 0)];
        let part = GlPartition::new(0, 0, 64, 0);

        let forward = GlWorkSeg {
            v1: 0,
            v2: 1,
            linedef: None,
            side: 0,
            side_sector: 0,
            partner: None,
        };
        let reversed = GlWorkSeg {
            v1: 1,
            v2: 0,
            linedef: None,
            side: 0,
            side_sector: 0,
            partner: None,
        };
        assert_eq!(bsp.classify_seg(&part, &forward), GlClass::ColinearFront);
        assert_eq!(bsp.classify_seg(&part, &reversed), GlClass::ColinearBack);
    }

    /// A genuine pre-rounding straddler whose rounded intersection lands exactly
    /// on an endpoint collapses to the *other* endpoint's side (§C.3), never
    /// `Split`. Fixture: partition `(0,0)+(3,2)`, seg `(1,0)→(-1,3)` — both
    /// endpoints strictly off-line and on opposite sides, yet the exact rounded
    /// intersection is `(1,0)` (the seg's own `v1`), so the seg classifies to the
    /// side of the far endpoint (`Back`, since `v2`'s cross is negative).
    #[test]
    fn classify_agrees_with_endpoint_collapse() {
        let map = build_map(&[(1.0, 0.0), (-1.0, 3.0)], &[(0, 1, Some(0), None)]);
        let mut bsp = GlBsp::new(&map, &NodeBuildOptions::strict()).unwrap();
        bsp.verts = vec![(1, 0), (-1, 3)];
        let part = GlPartition::new(0, 0, 3, 2);
        let seg = GlWorkSeg {
            v1: 0,
            v2: 1,
            linedef: Some(0),
            side: 0,
            side_sector: 0,
            partner: None,
        };

        assert_eq!(bsp.intersection(&seg, &part), Some((1, 0)));
        assert_eq!(bsp.classify_seg(&part, &seg), GlClass::Back);
    }

    /// The right seg of the two-sided wall (linedef 1), by construction the
    /// `side == 0` seg.
    fn wall_front_seg(bsp: &GlBsp) -> usize {
        bsp.segs
            .iter()
            .position(|s| s.linedef == Some(1) && s.side == 0)
            .expect("the two-sided wall has a front seg")
    }

    /// Splitting a two-sided pair's front seg yields four fragments forming two
    /// mirrored partner pairs, and the partner involution plus the mirrored-span
    /// invariant hold on both. The eager co-split derivation: with `sid` keeping
    /// `v1→m` and the new id taking `m→v2`, the mirror pairing is the *cross*
    /// pairing `sA↔pB` and `sB↔pA` (not `sid↔p`), because under v1-ordering the
    /// halves that share a span mirror are the ones on opposite fragment ends.
    #[test]
    fn split_preserves_partner_involution_two_sided() {
        let map = two_room_map();
        let mut bsp = GlBsp::new(&map, &NodeBuildOptions::strict()).unwrap();
        bsp.build_initial_segs();

        let front = wall_front_seg(&bsp);
        let p = bsp.segs[front]
            .partner
            .expect("wall front seg has a partner");
        let (v1, v2) = (bsp.segs[front].v1, bsp.segs[front].v2);

        // Split the wall at its midpoint (64, 32) in 16.16 — a fresh interior
        // vertex, so a genuine co-split with a newly interned mid.
        let (a, b) = bsp.split_seg_at(front, 64 << 16, 32 << 16).unwrap();
        assert_eq!(a, front, "the v1→m fragment is kept in `sid` in place");
        let m = bsp.segs[front].v2;
        assert!(
            m >= bsp.map_vertex_count,
            "the mid vertex is a split vertex"
        );

        // Four distinct fragments: front(sA), b(sB), p(pA), and pB.
        let p_b = bsp.segs[front].partner.expect("sA keeps a partner");
        let ids = [a, b, p, p_b];
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                assert_ne!(ids[i], ids[j], "the four fragments are distinct");
            }
        }

        // Pair 1: sA (v1→m) ↔ pB (m→v1).
        assert_eq!(bsp.segs[front].partner, Some(p_b));
        assert_eq!(bsp.segs[p_b].partner, Some(front));
        assert_eq!(bsp.segs[front].v1, bsp.segs[p_b].v2);
        assert_eq!(bsp.segs[front].v2, bsp.segs[p_b].v1);

        // Pair 2: sB (m→v2) ↔ pA (v2→m).
        assert_eq!(bsp.segs[b].partner, Some(p));
        assert_eq!(bsp.segs[p].partner, Some(b));
        assert_eq!(bsp.segs[b].v1, bsp.segs[p].v2);
        assert_eq!(bsp.segs[b].v2, bsp.segs[p].v1);

        // The fragments span the original edge: sA=v1→m, sB=m→v2.
        assert_eq!((bsp.segs[front].v1, bsp.segs[front].v2), (v1, m));
        assert_eq!((bsp.segs[b].v1, bsp.segs[b].v2), (m, v2));
    }

    /// When a seg is split but its partner is *not* being routed by the current
    /// `split_set`, the partner's new co-split fragment is parked in the spawn
    /// table under the partner's id, awaiting the container that still holds the
    /// partner (drain points (b)/(c), Task 6).
    #[test]
    fn co_split_fragment_lands_in_spawn_table_for_foreign_container() {
        let map = two_room_map();
        let mut bsp = GlBsp::new(&map, &NodeBuildOptions::strict()).unwrap();
        bsp.build_initial_segs();

        let front = wall_front_seg(&bsp);
        let p = bsp.segs[front]
            .partner
            .expect("wall front seg has a partner");
        let v1 = bsp.segs[front].v1;

        // Route only the front seg; the partner lives in a foreign container.
        let part = GlPartition::new(0, 32 << 16, 64 << 16, 0);
        let (f, b) = bsp.split_set(vec![front], &part).unwrap();

        // The straddler's own fragments routed to both sides.
        assert_eq!(f.len(), 1, "one fragment on the front");
        assert_eq!(b.len(), 1, "one fragment on the back");

        // The partner's new fragment (m→v1) is parked under `spawned[p]`, not
        // drained (p was not in the routed set).
        let parked = bsp.spawned.get(&p).expect("partner fragment is parked");
        assert_eq!(parked.len(), 1, "exactly one parked co-split fragment");
        let p_b = parked[0];
        let m = bsp.segs[front].v2;
        assert_eq!((bsp.segs[p_b].v1, bsp.segs[p_b].v2), (m, v1));
    }

    /// `split_set` routes a straddler's two fragments to opposite sides and
    /// records the split point as an on-partition event keyed by its exact
    /// `i128` dot product along the partition direction.
    #[test]
    fn split_set_routes_fragments_both_sides_and_records_split_event() {
        let map = two_room_map();
        let mut bsp = GlBsp::new(&map, &NodeBuildOptions::strict()).unwrap();
        bsp.build_initial_segs();

        let front = wall_front_seg(&bsp);
        // A horizontal partition through y = 32 splits the vertical wall.
        let part = GlPartition::new(0, 32 << 16, 64 << 16, 0);
        let (f, b) = bsp.split_set(vec![front], &part).unwrap();

        assert_eq!(f.len(), 1);
        assert_eq!(b.len(), 1);
        assert_ne!(f[0], b[0], "the fragments are distinct segs");

        // The split point (64, 32) sits on the partition; its event key is the
        // dot of (m − start) with the partition direction.
        let m = bsp.segs[front].v2;
        assert!(m >= bsp.map_vertex_count, "the mid is a split vertex");
        let (mx, my) = bsp.verts[m];
        let key = i128::from(part.pdx) * (i128::from(mx) - i128::from(part.px))
            + i128::from(part.pdy) * (i128::from(my) - i128::from(part.py));
        let event = bsp
            .events
            .events
            .get(&key)
            .expect("the split point is recorded as an event");
        assert_eq!(event.vertex, m, "the event carries the split vertex id");
    }

    /// Interning the exact coordinates of an existing map vertex returns its
    /// `Normal`-range index and grows neither the split-vertex arena nor the
    /// combined table.
    #[test]
    fn intern_vertex_dedups_exactly_and_reuses_normal_verts() {
        let map = two_room_map();
        let mut bsp = GlBsp::new(&map, &NodeBuildOptions::strict()).unwrap();

        let verts_before = bsp.verts.len();
        let gl_before = bsp.gl_vertices.len();
        // Map vertex 1 is (64, 0); its 16.16 coordinates are (64<<16, 0).
        let idx = bsp.intern_vertex(64 << 16, 0);
        assert_eq!(idx, 1, "reuses the existing map vertex");
        assert!(idx < bsp.map_vertex_count, "index is in the Normal range");
        assert_eq!(
            bsp.verts.len(),
            verts_before,
            "no new combined-table vertex"
        );
        assert_eq!(bsp.gl_vertices.len(), gl_before, "no new split vertex");

        // A genuinely new coordinate does push a split vertex and grows both.
        let fresh = bsp.intern_vertex(64 << 16, 32 << 16);
        assert_eq!(fresh, verts_before, "new split vertex appended at the tail");
        assert_eq!(bsp.gl_vertices.len(), gl_before + 1);
        assert!((bsp.gl_vertices[gl_before].y - 32.0).abs() < 1e-9);
    }

    /// A single convex square room bisected by a vertical partition through its
    /// open interior — the minimal loop-closing "gap" a miniseg seals. Hand-
    /// derived (Notes §Q2): two events, `(32,0)` and `(32,64)`, bound one span;
    /// with the room wound sector-on-the-right, every one of the four loop checks
    /// finds a real seg turning into the interior (front loop-start at `(32,0)`
    /// picks the front bottom fragment; the back and end mirrors likewise), so
    /// exactly one mirrored miniseg pair is created — front `prev→next` into the
    /// front out-set, back `next→prev` into the back, mutual partners with
    /// `linedef: None` and mirrored spans.
    #[test]
    fn doorway_gap_gets_mirrored_miniseg_pair() {
        let map = build_map(
            &[(0.0, 0.0), (64.0, 0.0), (64.0, 64.0), (0.0, 64.0)],
            &[
                (1, 0, Some(0), None), // bottom  B→A (west, sector on right)
                (0, 3, Some(0), None), // left    A→D (north)
                (3, 2, Some(0), None), // top     D→C (east)
                (2, 1, Some(0), None), // right   C→B (south)
            ],
        );
        let mut bsp = GlBsp::new(&map, &NodeBuildOptions::strict()).unwrap();
        bsp.build_initial_segs();
        let all: Vec<usize> = (0..bsp.segs.len()).collect();
        // Vertical partition x = 32, pointing north.
        let part = GlPartition::new(32 << 16, 0, 0, 64 << 16);
        let (mut front, mut back) = bsp.split_set(all, &part).unwrap();
        let (fb, bb) = (front.len(), back.len());

        bsp.add_minisegs(&part, &mut front, &mut back).unwrap();

        assert_eq!(
            front.len(),
            fb + 1,
            "exactly one miniseg added to the front"
        );
        assert_eq!(back.len(), bb + 1, "exactly one miniseg added to the back");
        let f = *front.last().unwrap();
        let b = *back.last().unwrap();
        assert_eq!(bsp.segs[f].linedef, None, "front miniseg has no linedef");
        assert_eq!(bsp.segs[b].linedef, None, "back miniseg has no linedef");
        assert_eq!(bsp.segs[f].partner, Some(b), "mutual partners");
        assert_eq!(bsp.segs[b].partner, Some(f), "mutual partners");
        assert_eq!(bsp.segs[f].v1, bsp.segs[b].v2, "mirrored span");
        assert_eq!(bsp.segs[f].v2, bsp.segs[b].v1, "mirrored span");
        assert_eq!(bsp.segs[f].side, 0, "front miniseg records side 0");
        assert_eq!(bsp.segs[b].side, 1, "back miniseg records side 1");
    }

    /// Partitioning along a fully-segged two-sided wall creates no minisegs: the
    /// span between the wall's two events is covered by colinear geometry, so the
    /// front loop-start's secondary scan sees a real seg running straight from
    /// `prev` to `next` (`v2 == vertex2`) and returns `None` (Notes §Q2/§Q6). The
    /// out-sets are unchanged.
    #[test]
    fn solid_colinear_wall_produces_no_minisegs() {
        let map = two_room_map();
        let mut bsp = GlBsp::new(&map, &NodeBuildOptions::strict()).unwrap();
        bsp.build_initial_segs();
        let all: Vec<usize> = (0..bsp.segs.len()).collect();
        // Partition ALONG the shared two-sided wall at x = 64.
        let part = GlPartition::new(64 << 16, 0, 0, 64 << 16);
        let (mut front, mut back) = bsp.split_set(all, &part).unwrap();
        let (fb, bb) = (front.len(), back.len());

        bsp.add_minisegs(&part, &mut front, &mut back).unwrap();

        assert_eq!(front.len(), fb, "no minisegs added to the front");
        assert_eq!(back.len(), bb, "no minisegs added to the back");
    }

    /// A span with no loop-closing geometry on one side yields no miniseg ("don't
    /// create subsectors in void space", Notes §Q2). The bottom wall touches the
    /// partition at `(64,0)` but only extends into the back half, so nothing
    /// *ends* at `(64,0)` — the front loop-start's primary scan is empty and
    /// returns `None`. A second crossing wall supplies the upper event `(64,64)`
    /// so a full span exists, yet still no pair is created.
    #[test]
    fn void_interval_produces_no_minisegs() {
        let map = build_map(
            &[(64.0, 0.0), (0.0, 0.0), (0.0, 64.0), (128.0, 64.0)],
            &[
                (0, 1, Some(0), None), // (64,0)→(0,0): starts on the line, into back
                (2, 3, Some(0), None), // (0,64)→(128,64): crosses at (64,64)
            ],
        );
        let mut bsp = GlBsp::new(&map, &NodeBuildOptions::strict()).unwrap();
        bsp.build_initial_segs();
        let all: Vec<usize> = (0..bsp.segs.len()).collect();
        let part = GlPartition::new(64 << 16, 0, 0, 64 << 16);
        let (mut front, mut back) = bsp.split_set(all, &part).unwrap();
        let (fb, bb) = (front.len(), back.len());

        bsp.add_minisegs(&part, &mut front, &mut back).unwrap();

        assert_eq!(front.len(), fb, "front out-set unchanged (void span)");
        assert_eq!(back.len(), bb, "back out-set unchanged (void span)");
    }

    /// A two-sided colinear wall spanning three events is force-split at the
    /// interior event before the interval walk, and its partner follows in
    /// lockstep (Notes §Q1 `FixSplitSharers`). Fixture: a shared wall along `x=0`
    /// from `(0,-32)` to `(0,32)`, crossed by a perpendicular wall at `(0,0)` that
    /// interns the interior event. After the repair the wall and its partner are
    /// each two fragments meeting at `(0,0)`, and the partner involution + mirrored
    /// span hold over every seg.
    #[test]
    fn overlapping_colinear_seg_is_split_at_interior_events() {
        let map = build_map(
            &[(0.0, -32.0), (0.0, 32.0), (-16.0, 0.0), (16.0, 0.0)],
            &[
                (0, 1, Some(0), Some(1)), // shared colinear wall along x = 0
                (2, 3, Some(0), None),    // perpendicular wall crossing at (0,0)
            ],
        );
        let mut bsp = GlBsp::new(&map, &NodeBuildOptions::strict()).unwrap();
        bsp.build_initial_segs();
        assert_eq!(
            bsp.segs.iter().filter(|s| s.linedef == Some(0)).count(),
            2,
            "the two-sided colinear wall starts as two segs"
        );
        let all: Vec<usize> = (0..bsp.segs.len()).collect();
        // Partition along x = 0, from (0,-32) pointing north.
        let part = GlPartition::new(0, -32 << 16, 0, 64 << 16);
        let (mut front, mut back) = bsp.split_set(all, &part).unwrap();

        bsp.add_minisegs(&part, &mut front, &mut back).unwrap();

        // The colinear wall and its partner were each force-split at (0,0).
        assert_eq!(
            bsp.segs.iter().filter(|s| s.linedef == Some(0)).count(),
            4,
            "colinear wall + partner each split at the interior event"
        );
        // Involution and mirrored span hold over every seg after the co-split.
        for (i, s) in bsp.segs.iter().enumerate() {
            if let Some(p) = s.partner {
                assert_eq!(bsp.segs[p].partner, Some(i), "partner involution holds");
                assert_eq!(bsp.segs[i].v1, bsp.segs[p].v2, "mirrored span");
                assert_eq!(bsp.segs[i].v2, bsp.segs[p].v1, "mirrored span");
            }
        }
        // The interior split vertex (0,0) is shared by fragments on both sides.
        let mid = bsp.dedup[&(0, 0)];
        assert!(
            bsp.segs.iter().any(|s| s.linedef == Some(0) && s.v2 == mid),
            "a colinear fragment ends at the interior vertex"
        );
        assert!(
            bsp.segs.iter().any(|s| s.linedef == Some(0) && s.v1 == mid),
            "a colinear fragment starts at the interior vertex"
        );
    }

    // --- Task 6: driver, leaf closing, BuiltGlNodes + validate ---------------

    use crate::map::build::NodeStructureError;
    use crate::map::graph::{GlSeg, GlSegIdx, GlSubsector};

    /// A one-sided square room: four walls facing sector 0, wound Doom-correct
    /// (sector on the right) as a closed v0→v1→v2→v3→v0 loop.
    fn square_room() -> Map {
        build_map(
            &[(0.0, 0.0), (0.0, 64.0), (64.0, 64.0), (64.0, 0.0)],
            &[
                (0, 1, Some(0), None),
                (1, 2, Some(0), None),
                (2, 3, Some(0), None),
                (3, 0, Some(0), None),
            ],
        )
    }

    /// Two square rooms sharing the vertical wall at x = 64, wound Doom-correct
    /// (each one-sided wall has its sector on the right, so the GL leaf-closing
    /// pass routes the shared two-sided wall's segs into the matching cells).
    /// Sector 0 is the west room, sector 1 the east; linedef 2 is the only
    /// two-sided line.
    fn two_room_correct() -> Map {
        build_map(
            &[
                (0.0, 0.0),    // 0
                (0.0, 64.0),   // 1
                (64.0, 64.0),  // 2
                (64.0, 0.0),   // 3
                (128.0, 0.0),  // 4
                (128.0, 64.0), // 5
            ],
            &[
                (0, 1, Some(0), None),    // west  room 0
                (1, 2, Some(0), None),    // north room 0
                (2, 3, Some(0), Some(1)), // SHARED wall (right=room 0, left=room 1)
                (3, 0, Some(0), None),    // south room 0
                (2, 5, Some(1), None),    // north room 1
                (5, 4, Some(1), None),    // east  room 1
                (4, 3, Some(1), None),    // south room 1
            ],
        )
    }

    #[test]
    fn square_room_builds_one_closed_subsector() {
        let map = square_room();
        let (built, warnings) = build_gl_nodes(&map, &NodeBuildOptions::strict()).unwrap();

        assert!(warnings.is_empty(), "a bare convex room warns nothing");
        assert_eq!(built.subsectors.len(), 1, "one convex subsector");
        assert_eq!(built.nodes.len(), 0, "no internal nodes");
        assert_eq!(built.segs.len(), 4, "four boundary segs");
        assert_eq!(
            built.subsectors[0].segs,
            0..4,
            "subsector owns all four segs"
        );
        assert!(built.gl_vertices.is_empty(), "no splits, no GL vertices");

        for s in &built.segs {
            assert!(s.linedef.is_some(), "no minisegs in a bare convex room");
            assert!(s.partner.is_none(), "one-sided walls have no partner");
        }
        // The seg run is a closed loop.
        for i in 0..4 {
            let next = (i + 1) % 4;
            assert_eq!(built.segs[i].end, built.segs[next].start, "loop closes");
        }
        assert!(built.validate(4).is_ok(), "output is structurally valid");
    }

    #[test]
    fn two_room_map_builds_closed_convex_subsectors() {
        let map = two_room_correct();
        let orig = map.vertices().len();
        let (built, warnings) = build_gl_nodes(&map, &NodeBuildOptions::strict()).unwrap();

        assert!(warnings.is_empty(), "two single-sector rooms warn nothing");
        assert!(
            !built.nodes.is_empty(),
            "the shared wall forces at least one node"
        );
        assert!(built.validate(orig).is_ok(), "output is structurally valid");

        // Every subsector is a closed loop.
        for (si, ss) in built.subsectors.iter().enumerate() {
            let run = &built.segs[ss.segs.clone()];
            for i in 0..run.len() {
                let next = (i + 1) % run.len();
                assert_eq!(run[i].end, run[next].start, "subsector {si} closes");
            }
        }
        // The partner involution holds over the whole seg arena.
        for (i, s) in built.segs.iter().enumerate() {
            if let Some(p) = s.partner {
                assert_ne!(p.0, i, "a seg is never its own partner");
                assert_eq!(
                    built.segs[p.0].partner,
                    Some(GlSegIdx(i)),
                    "partner involution"
                );
                assert_eq!(built.segs[p.0].start, s.end, "mirrored span");
                assert_eq!(built.segs[p.0].end, s.start, "mirrored span");
            }
        }
    }

    /// A one-sided GL seg from map vertex `sx` to map vertex `ex`.
    fn gl_seg(sx: usize, ex: usize, partner: Option<usize>) -> GlSeg {
        GlSeg {
            start: GlVertexRef::Normal(VertexIdx(sx)),
            end: GlVertexRef::Normal(VertexIdx(ex)),
            linedef: Some(LinedefIdx(0)),
            side: 0,
            partner: partner.map(GlSegIdx),
        }
    }

    /// A valid single-subsector GL square over map vertices 0..=3.
    fn gl_square() -> BuiltGlNodes {
        BuiltGlNodes {
            gl_vertices: Vec::new(),
            segs: vec![
                gl_seg(0, 1, None),
                gl_seg(1, 2, None),
                gl_seg(2, 3, None),
                gl_seg(3, 0, None),
            ],
            subsectors: vec![GlSubsector { segs: 0..4 }],
            nodes: Vec::new(),
        }
    }

    #[test]
    fn validate_accepts_a_well_formed_gl_square() {
        assert!(gl_square().validate(4).is_ok());
    }

    #[test]
    fn validate_rejects_partner_asymmetry() {
        let mut built = gl_square();
        // Seg 0 names seg 2 as partner, but seg 2 has no partner back.
        built.segs[0].partner = Some(GlSegIdx(2));
        assert_eq!(
            built.validate(4).unwrap_err(),
            NodeBuildError::InvalidStructure(NodeStructureError::PartnerAsymmetry {
                seg: 0,
                partner: 2,
            }),
        );
    }

    #[test]
    fn validate_rejects_open_loop() {
        let mut built = gl_square();
        // Break the chain: seg 0's end (v1) no longer meets seg 1's start.
        built.segs[1].start = GlVertexRef::Normal(VertexIdx(2));
        assert_eq!(
            built.validate(4).unwrap_err(),
            NodeBuildError::InvalidStructure(NodeStructureError::OpenLoop {
                subsector: 0,
                seg: 0,
            }),
        );
    }

    #[test]
    fn validate_rejects_out_of_range_gl_ref() {
        let mut built = gl_square();
        // The GL-vertex arena is empty, so any `Gl` reference is out of range.
        built.segs[2].end = GlVertexRef::Gl(GlVertexIdx(0));
        assert_eq!(
            built.validate(4).unwrap_err(),
            NodeBuildError::InvalidStructure(NodeStructureError::GlVertexRef { seg: 2, bound: 0 }),
        );
    }

    #[test]
    fn degenerate_leaf_strict_errors_lenient_warns() {
        // A single two-sided linedef yields two segs over just two vertices — a
        // leaf of fewer than 3 distinct vertices.
        let map = build_map(&[(0.0, 0.0), (64.0, 0.0)], &[(0, 1, Some(0), Some(1))]);

        let mut strict = GlBsp::new(&map, &NodeBuildOptions::strict()).unwrap();
        strict.build_initial_segs();
        let leaf: Vec<usize> = (0..strict.segs.len()).collect();
        assert_eq!(
            strict.close_leaf(leaf).unwrap_err(),
            NodeBuildError::DegenerateLeaf { subsector_segs: 2 },
        );

        let mut lenient = GlBsp::new(&map, &NodeBuildOptions::lenient()).unwrap();
        lenient.build_initial_segs();
        let leaf: Vec<usize> = (0..lenient.segs.len()).collect();
        let order = lenient.close_leaf(leaf).unwrap();
        assert_eq!(order.len(), 2, "the degenerate leaf is emitted as-is");
        assert!(
            lenient
                .warnings
                .iter()
                .any(|w| matches!(w, NodeBuildWarning::DegenerateLeaf { subsector_segs: 2 })),
            "lenient mode warns once for the degenerate leaf"
        );
    }
}
