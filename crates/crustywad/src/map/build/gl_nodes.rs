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

use super::geom;
use super::nodes::SAMPLE_BUDGET;
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
#[allow(dead_code)] // Consumed by the partition/split passes (Tasks 4, 6).
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
    #[allow(dead_code)] // Consumed by the split pass (Task 4).
    fn is_front(self) -> bool {
        matches!(self, GlClass::Front | GlClass::ColinearFront)
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

    /// Whether every seg in `set` faces the same sector (§C.1). Minisegs carry a
    /// real `side_sector`, so no seg needs special-casing here.
    #[allow(dead_code)] // Consumed by the partition pass (Task 6).
    fn single_sector(&self, set: &[usize]) -> bool {
        let first = self.segs[set[0]].side_sector;
        set.iter().all(|&id| self.segs[id].side_sector == first)
    }

    /// The exact `i128` cross product of the partition direction with the vector
    /// from the line start to the 16.16 vertex `v`: `> 0` front, `< 0` back
    /// (§B.2, engine convention `R_PointOnSide`). Wide because a 16.16 delta can
    /// reach `2³²`, overflowing the classic `i64` cross.
    #[allow(dead_code)] // Consumed by the partition/split passes (Tasks 4, 6).
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
    #[allow(dead_code)] // Consumed by the partition/split passes (Tasks 4, 6).
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
    #[allow(dead_code)] // Consumed by the partition/split passes (Tasks 4, 6).
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
    #[allow(dead_code, clippy::similar_names)] // Consumed by the split pass (Task 4).
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

    /// Selects the best splitter in `set` (§B), or `None` if the set is convex
    /// (no valid partition). `relaxed` switches to the sector-separating validity
    /// rule (§C.2). Ties break toward the lowest seg id (determinism). Minisegs
    /// may serve as candidates (their line is a former partition).
    #[allow(dead_code)] // Consumed by the partition pass (Task 6).
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
    #[allow(dead_code)] // Consumed by the partition pass (Task 6).
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
}

/// Whether a partition delta fits the XGL3 on-disk `i32` node `dx`/`dy` field
/// (§B.1): a seg can serve as a splitter only if its 16.16 `v2 - v1` fits `i32`
/// on both axes. The fixed-space analog of the classic `partition_delta_fits`
/// (whose ceiling is the on-disk `i16`); the range is the full signed `i32`.
#[allow(dead_code)] // Consumed by the partition pass (Task 6).
fn partition_delta_fits_gl(pdx: i64, pdy: i64) -> bool {
    i32::try_from(pdx).is_ok() && i32::try_from(pdy).is_ok()
}

/// Integer round-half-away-from-zero division of `a / b` (`b != 0`): the exact
/// rounding used by [`GlBsp::intersection`] in place of ZDBSP's `double` +
/// truncation. Ties (`|remainder| · 2 == |b|`) round away from zero; exact
/// quotients are returned unchanged. Correct in every sign quadrant.
#[allow(dead_code)] // Consumed by the split pass (Task 4).
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
}
