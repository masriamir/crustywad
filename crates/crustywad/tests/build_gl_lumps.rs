//! Public-API tests for the `nodebuild` GL BSP builder (ADR-0026, #363).
//!
//! Covers the GL kernel `build_gl_nodes`: two oracles — `validate_gl_structure`
//! (the format-domain invariants that hold on every output) and `validate_gl_bsp`
//! (the former plus geometric convexity, for well-formed geometry only) — the
//! fixture set (square room, L-room, two-room doorway, corridor room, fractional
//! split, determinism), a random-geometry proptest, and the retail sweep. Mirrors
//! the organization of `build_lumps.rs` (the classic BSP pass), reusing its
//! WAD-building helpers and oracle pattern.
//!
//! Convexity is asserted only on the designed, non-overlapping fixtures. The
//! proptest and the retail sweep exercise arbitrary/self-referencing geometry,
//! which legitimately yields valid-but-non-convex subsector loops (segs interior
//! to a convex leaf); those assert `validate_gl_structure` alone. See
//! `assert_subsectors_convex` for the full rationale.
#![cfg(feature = "nodebuild")]

mod common;

use crustywad::map::build::{
    BuiltGlNodes, NodeBuildError, NodeBuildOptions, NodeBuildWarning, NodeStructureError,
    build_gl_nodes,
};
use crustywad::map::{GlNodeChild, GlVertexRef, Map};
use crustywad::{ParseOptions, Wad};
use proptest::prelude::*;

// --- WAD-building helpers (mirrors build_lumps.rs) ---------------------------

/// Encodes a Doom 8-byte name field, NUL-padded on the right.
fn name8(name: &str) -> [u8; 8] {
    let mut out = [0u8; 8];
    for (slot, byte) in out.iter_mut().zip(name.as_bytes()) {
        *slot = *byte;
    }
    out
}

/// One classic `LINEDEFS` record (14 bytes, all `u16` fields).
fn linedef_bytes(
    start_vertex: u16,
    end_vertex: u16,
    flags: u16,
    special_type: u16,
    sector_tag: u16,
    right_sidedef: u16,
    left_sidedef: u16,
) -> Vec<u8> {
    [
        start_vertex,
        end_vertex,
        flags,
        special_type,
        sector_tag,
        right_sidedef,
        left_sidedef,
    ]
    .iter()
    .flat_map(|v| v.to_le_bytes())
    .collect()
}

/// One `SIDEDEFS` record (30 bytes): offsets, three 8-byte texture names, then
/// the sector index.
fn sidedef_bytes(upper: &str, lower: &str, middle: &str, sector: u16) -> Vec<u8> {
    [
        &0i16.to_le_bytes()[..],
        &0i16.to_le_bytes(),
        &name8(upper),
        &name8(lower),
        &name8(middle),
        &sector.to_le_bytes(),
    ]
    .concat()
}

/// One `THINGS` record (10 bytes): x, y (`i16`), angle/type/flags (`u16`).
fn thing_bytes(x: i16, y: i16, angle: u16, type_id: u16, flags: u16) -> Vec<u8> {
    [
        &x.to_le_bytes()[..],
        &y.to_le_bytes(),
        &angle.to_le_bytes(),
        &type_id.to_le_bytes(),
        &flags.to_le_bytes(),
    ]
    .concat()
}

/// `VERTEXES` records (4 bytes each) from `(x, y)` pairs.
fn vertexes_bytes(points: &[(i16, i16)]) -> Vec<u8> {
    points
        .iter()
        .flat_map(|(x, y)| [x.to_le_bytes(), y.to_le_bytes()].concat())
        .collect()
}

/// One `SECTORS` record (26 bytes): heights, two 8-byte flat names, light,
/// special, tag.
fn sector_bytes(tag: i16) -> Vec<u8> {
    [
        &0i16.to_le_bytes()[..],
        &128i16.to_le_bytes(),
        &name8("FLOOR4_8"),
        &name8("CEIL3_5"),
        &160i16.to_le_bytes(),
        &0i16.to_le_bytes(),
        &tag.to_le_bytes(),
    ]
    .concat()
}

/// Assembles a Doom map from vertices and general linedefs. Each linedef is
/// `(start, end, right_sector, left_sector)`: a `Some(sector)` side gets a fresh
/// sidedef facing that sector; a `None` side is the `0xffff` "no sidedef"
/// sentinel. Two-sided (both sides present) linedefs get the two-sided flag.
/// Enough sectors are emitted to cover the highest referenced index. Copied from
/// `build_lumps.rs` so the GL fixtures build through the same public path.
fn assemble_general(points: &[(i16, i16)], lines: &[(u16, u16, Option<u16>, Option<u16>)]) -> Map {
    let mut linedefs = Vec::new();
    let mut sidedefs = Vec::new();
    let mut next_side: u16 = 0;
    let mut max_sector: u16 = 0;
    let side_for = |sector: u16, sidedefs: &mut Vec<u8>, next_side: &mut u16| -> u16 {
        sidedefs.extend(sidedef_bytes("-", "-", "STARTAN3", sector));
        let idx = *next_side;
        *next_side += 1;
        idx
    };
    for &(s, e, rs, ls) in lines {
        let right = match rs {
            Some(sec) => {
                max_sector = max_sector.max(sec);
                side_for(sec, &mut sidedefs, &mut next_side)
            }
            None => 0xffff,
        };
        let left = match ls {
            Some(sec) => {
                max_sector = max_sector.max(sec);
                side_for(sec, &mut sidedefs, &mut next_side)
            }
            None => 0xffff,
        };
        let flags: u16 = if rs.is_some() && ls.is_some() {
            0x0004
        } else {
            0x0001
        };
        linedefs.extend(linedef_bytes(s, e, flags, 0, 0, right, left));
    }
    let mut sectors = Vec::new();
    for i in 0..=max_sector {
        sectors.extend(sector_bytes(i16::try_from(i).unwrap()));
    }
    let bytes = common::build_doom_map_wad(
        "MAP01",
        thing_bytes(0, 0, 0, 1, 7),
        linedefs,
        sidedefs,
        vertexes_bytes(points),
        sectors,
    );
    let wad = Wad::from_bytes(bytes).expect("fixture WAD parses");
    let group = wad.map_group("MAP01").expect("map group present");
    Map::assemble(&wad, &group).expect("map assembles")
}

// --- Oracle -----------------------------------------------------------------

/// Resolves a [`GlVertexRef`] to its exact 16.16 fixed-point coordinates as
/// `(i64, i64)`. A `Normal` ref indexes the map's own whole-unit vertices, so
/// `x * 65536` is exact; a `Gl` ref indexes the built split vertices, whose
/// `f64` was created as `fixed / 65536.0` (a power-of-two divide, exact), so
/// `(x * 65536).round()` recovers the fixed value exactly. Both are therefore
/// lossless.
fn gl_ref_fixed(map: &Map, built: &BuiltGlNodes, r: GlVertexRef) -> (i64, i64) {
    let (x, y) = match r {
        GlVertexRef::Normal(v) => {
            let mv = map.vertices()[v.0];
            (mv.x, mv.y)
        }
        GlVertexRef::Gl(v) => {
            let gv = built.gl_vertices[v.0];
            (gv.x, gv.y)
        }
    };
    #[allow(clippy::cast_possible_truncation)]
    let fx = (x * 65536.0).round() as i64;
    #[allow(clippy::cast_possible_truncation)]
    let fy = (y * 65536.0).round() as i64;
    (fx, fy)
}

/// The **structural** GL oracle — the format-domain invariants that hold on
/// **every** build output, well-formed or adversarial. Shared by the designed
/// fixtures (via [`validate_gl_bsp`]), the random-geometry proptest, and the
/// retail sweep. It asserts:
///
/// - Warning set: every emitted warning is one of the four allowed GL variants
///   (`Write | VanillaCeilingExceeded | MixedSectorSubsector | DegenerateLeaf`).
/// - Structural [`BuiltGlNodes::validate`] passes (subsector partition, vertex
///   ranges, node-child ranges, partner involution + mirrored spans, closed
///   loops). The one documented exception: a lenient build carrying a
///   [`NodeBuildWarning::DegenerateLeaf`] may emit a leaf whose loop does not
///   close — an `OpenLoop` failure is accepted **iff** such a warning is present.
/// - Root last: the last node is referenced by no other node; every other node
///   is referenced exactly once (a tree with the root at the end).
/// - Subsector/node count: `subsectors == nodes + 1` when nodes exist, else
///   exactly one subsector.
/// - Linedef/side consistency: every non-miniseg seg's linedef is in range and
///   its `side` (0 = right/front, 1 = left/back) is present on that linedef.
/// - Ancestor-side containment: every seg endpoint of every subsector lies on —
///   or within a (scaled) 1.5-map-unit tolerance of — the correct side of every
///   ancestor partition on the path from the root to that leaf (front child ⇒
///   front side, back child ⇒ back side). The fixed-space analogue of the
///   classic pass's geometric containment check.
///
/// Geometric **convexity is deliberately not asserted here** — see
/// [`validate_gl_bsp`] for why it holds only for well-formed geometry.
fn validate_gl_structure(map: &Map, built: &BuiltGlNodes, warnings: &[NodeBuildWarning]) {
    let has_degenerate = warnings
        .iter()
        .any(|w| matches!(w, NodeBuildWarning::DegenerateLeaf { .. }));

    // (0) Warning set: only the four GL-permitted variants may appear.
    for w in warnings {
        assert!(
            matches!(
                w,
                NodeBuildWarning::Write(_)
                    | NodeBuildWarning::VanillaCeilingExceeded { .. }
                    | NodeBuildWarning::MixedSectorSubsector { .. }
                    | NodeBuildWarning::DegenerateLeaf { .. }
            ),
            "unexpected GL node-build warning: {w:?}"
        );
    }

    // (1) Structural validation, with the lenient degenerate-leaf loop-closure
    //     exemption folded in.
    match built.validate(map.vertices().len()) {
        Ok(()) => {}
        Err(NodeBuildError::InvalidStructure(NodeStructureError::OpenLoop { .. })) => {
            assert!(
                has_degenerate,
                "validate returned OpenLoop but no DegenerateLeaf warning is present"
            );
        }
        Err(e) => panic!("BuiltGlNodes::validate failed: {e:?}"),
    }

    // (2) Subsector/node count relationship.
    if built.nodes.is_empty() {
        assert_eq!(
            built.subsectors.len(),
            1,
            "no nodes => exactly one convex subsector"
        );
    } else {
        assert_eq!(
            built.subsectors.len(),
            built.nodes.len() + 1,
            "a full binary tree of leaves: subsectors == nodes + 1"
        );
    }

    // (3) Root last: the last node is unreferenced as a child; every other node
    //     is referenced exactly once (a tree rooted at the final arena slot).
    if !built.nodes.is_empty() {
        let mut refcount = vec![0usize; built.nodes.len()];
        for n in &built.nodes {
            for child in [n.right, n.left] {
                if let GlNodeChild::Node(k) = child {
                    assert!(k.0 < built.nodes.len(), "node child index in range");
                    refcount[k.0] += 1;
                }
            }
        }
        let root = built.nodes.len() - 1;
        assert_eq!(refcount[root], 0, "the root (last node) is not a child");
        for (i, &c) in refcount.iter().enumerate() {
            if i != root {
                assert_eq!(c, 1, "non-root node {i} is referenced exactly once");
            }
        }
    }

    // (4) Linedef/side consistency for every real (non-miniseg) seg.
    for (i, s) in built.segs.iter().enumerate() {
        if let Some(li) = s.linedef {
            assert!(
                li.0 < map.linedefs().len(),
                "seg {i} linedef {} in range",
                li.0
            );
            assert!(s.side == 0 || s.side == 1, "seg {i} side is 0 or 1");
            let ld = &map.linedefs()[li.0];
            let side_present = if s.side == 0 {
                ld.right.is_some()
            } else {
                ld.left.is_some()
            };
            assert!(
                side_present,
                "seg {i} side {} has a present sidedef on its linedef",
                s.side
            );
        }
    }

    // (5) Ancestor-side containment (see `assert_ancestor_side_containment`).
    assert_ancestor_side_containment(map, built);
}

/// Ancestor-side containment: every seg endpoint of every subsector lies on — or
/// within tolerance of — the correct side of every ancestor partition on the path
/// from the root to that leaf (front child ⇒ front side, back child ⇒ back side).
/// The fixed-space analogue of the classic oracle's geometric containment check
/// (`build_lumps.rs`), walked from the root (`built.nodes.last()`) carrying the
/// ancestor partition stack.
///
/// Tolerance: the classic pass allows 1.5 **map units** of slack; scaled to 16.16
/// fixed space that is `3 * 65536 / 2 = 98304` fixed units. GL split vertices land
/// within 0.5 fixed units of their partition, so this bound is generous; keeping
/// the classic's semantic value (scaled) preserves cross-oracle comparability.
///
/// Side test: an exact `i128` cross product (`> 0` is the front side, mirroring
/// the classic formula). A point is "within tolerance" of the line when
/// `distance² ≤ tol²`, i.e. `cross² ≤ tol²·len2` (no `sqrt`). A violation is an
/// endpoint on the *wrong* side *and* farther than the tolerance.
///
/// Overflow guard (mirrors `geom::within_half_fixed_unit`): partition deltas are
/// `i32` `fixed_t`, so `len2 = dx²+dy² ≤ 2·(2³¹)² = 2⁶³`, and `tol² < 2³⁴`, hence
/// the pass threshold `tol²·len2 < 2⁹⁷` (fits `i128`). A pass therefore needs
/// `|cross| < 2⁴⁸·⁵ < 2⁴⁹`; any `|cross| ≥ 2⁴⁹` on the wrong side is simply "far
/// on the wrong side" — a failure we take without squaring (`cross` reaches ~2⁶⁴,
/// whose square overflows `i128`). Below the guard, `cross² < 2⁹⁸` is in range.
fn assert_ancestor_side_containment(map: &Map, built: &BuiltGlNodes) {
    const TOL: i128 = 3 * 65536 / 2; // 1.5 map units in 16.16 fixed
    const TOL_SQ: i128 = TOL * TOL;
    const CROSS_GUARD: u128 = 1 << 49;
    if built.nodes.is_empty() {
        return; // a single convex leaf has no ancestor partitions
    }
    let root = GlNodeChild::Node(crustywad::map::GlNodeIdx(built.nodes.len() - 1));
    // Each stack entry carries its child and the ancestor partition path as
    // (node_index, is_front) pairs (front = right/front child taken).
    let mut stack: Vec<(GlNodeChild, Vec<(usize, bool)>)> = vec![(root, Vec::new())];
    while let Some((child, path)) = stack.pop() {
        match child {
            GlNodeChild::Node(k) => {
                let n = &built.nodes[k.0];
                let mut front = path.clone();
                front.push((k.0, true));
                let mut back = path.clone();
                back.push((k.0, false));
                stack.push((n.right, front));
                stack.push((n.left, back));
            }
            GlNodeChild::Subsector(i) => {
                for &(node_idx, is_front) in &path {
                    let n = &built.nodes[node_idx];
                    let nx = i128::from(n.x);
                    let ny = i128::from(n.y);
                    let ndx = i128::from(n.dx);
                    let ndy = i128::from(n.dy);
                    let len2 = ndx * ndx + ndy * ndy;
                    for seg in &built.segs[built.subsectors[i.0].segs.clone()] {
                        for endpoint in [seg.start, seg.end] {
                            let (qx, qy) = gl_ref_fixed(map, built, endpoint);
                            // Engine cross: > 0 is the front side.
                            let cross = (i128::from(qx) - nx) * ndy - (i128::from(qy) - ny) * ndx;
                            // Correct side? front ⇒ cross ≥ 0, back ⇒ cross ≤ 0.
                            let on_wrong_side = if is_front { cross < 0 } else { cross > 0 };
                            if !on_wrong_side {
                                continue;
                            }
                            // On the wrong side: tolerated only within `tol`. The
                            // guard short-circuits the square when |cross| is
                            // beyond any tolerable magnitude (and would overflow
                            // i128), classifying it as a failure.
                            assert!(
                                cross.unsigned_abs() < CROSS_GUARD
                                    && cross * cross <= TOL_SQ * len2,
                                "subsector {} seg endpoint within tolerance of ancestor partition {node_idx} (is_front={is_front})",
                                i.0
                            );
                        }
                    }
                }
            }
        }
    }
}

/// Whether every subsector is a **convex** loop: walking each subsector's segs
/// in loop order, every consecutive edge-pair turn shares one orientation sign
/// (exact `i128` cross products on the seg-start coordinates; a zero cross for
/// colinear points is allowed). Runs are shorter than 3 segs, and every
/// subsector under a `DegenerateLeaf` warning, are skipped (an unordered
/// degenerate leaf is not a loop).
///
/// This is a **well-formed-geometry** invariant, not a universal one. A GL BSP
/// leaf is the intersection of its ancestor half-planes, hence a convex region;
/// but the leaf's seg *loop* traces that convex boundary only when every seg in
/// the leaf lies on the boundary. Self-referencing lines (both sides one sector)
/// and crossing/overlapping walls place real segs in a leaf's *interior*, so the
/// closed loop then weaves around them and is not convex — an inherent property
/// of GL building over such input, not a builder defect. This matches the
/// kernel's own tests (which assert closed loops, never geometric convexity) and
/// [`BuiltGlNodes::validate`] (which excludes convexity by design). So convexity
/// is checked only on the designed, non-overlapping fixtures; the random-geometry
/// proptest and the retail sweep (both of which routinely exercise crossing and
/// self-referencing lines) assert [`validate_gl_structure`] alone.
fn assert_subsectors_convex(map: &Map, built: &BuiltGlNodes, warnings: &[NodeBuildWarning]) {
    if warnings
        .iter()
        .any(|w| matches!(w, NodeBuildWarning::DegenerateLeaf { .. }))
    {
        return;
    }
    for (si, ss) in built.subsectors.iter().enumerate() {
        let run = ss.segs.clone();
        let n = run.len();
        if n < 3 {
            continue; // fewer than 3 vertices: no turn to test
        }
        let coords: Vec<(i64, i64)> = (run.start..run.end)
            .map(|i| gl_ref_fixed(map, built, built.segs[i].start))
            .collect();
        let mut sign = 0i8;
        for k in 0..n {
            let prev = coords[k];
            let cur = coords[(k + 1) % n];
            let next = coords[(k + 2) % n];
            // Turn from edge prev->cur to edge cur->next: cross of the two.
            let cross = i128::from(cur.0 - prev.0) * i128::from(next.1 - cur.1)
                - i128::from(cur.1 - prev.1) * i128::from(next.0 - cur.0);
            if cross != 0 {
                let turn = if cross > 0 { 1i8 } else { -1 };
                if sign == 0 {
                    sign = turn;
                } else {
                    assert_eq!(
                        sign, turn,
                        "subsector {si} is convex (consistent turn orientation)"
                    );
                }
            }
        }
    }
}

/// The full GL oracle for a **well-formed** (non-overlapping, properly-sectored)
/// fixture: [`validate_gl_structure`] plus geometric convexity of every
/// subsector ([`assert_subsectors_convex`]). Used by the designed fixtures. The
/// proptest and retail sweep, whose geometry is arbitrary, assert
/// [`validate_gl_structure`] only.
fn validate_gl_bsp(map: &Map, built: &BuiltGlNodes, warnings: &[NodeBuildWarning]) {
    validate_gl_structure(map, built, warnings);
    assert_subsectors_convex(map, built, warnings);
}

// --- Fixtures ----------------------------------------------------------------

/// A single convex square room: four one-sided linedefs, one sector. It is
/// already convex, so the GL build produces exactly one subsector, no nodes,
/// four real segs in a closed loop, no split vertices, no minisegs, and no
/// partners.
#[test]
fn build_gl_nodes_square_room_is_one_convex_subsector() {
    let points = [(0i16, 0i16), (128, 0), (128, 128), (0, 128)];
    let map = assemble_general(
        &points,
        &[
            (0, 1, Some(0), None),
            (1, 2, Some(0), None),
            (2, 3, Some(0), None),
            (3, 0, Some(0), None),
        ],
    );
    let (built, warnings) = build_gl_nodes(&map, &NodeBuildOptions::strict()).expect("builds");
    assert!(warnings.is_empty(), "square room builds strict-clean");

    assert_eq!(built.subsectors.len(), 1, "one convex subsector");
    assert!(built.nodes.is_empty(), "a convex room needs no nodes");
    assert_eq!(built.segs.len(), 4, "four walls => four segs");
    assert_eq!(built.subsectors[0].segs, 0..4);
    assert!(built.gl_vertices.is_empty(), "a convex room splits nothing");
    assert!(
        built.segs.iter().all(|s| s.linedef.is_some()),
        "no minisegs in a convex room"
    );
    assert!(
        built.segs.iter().all(|s| s.partner.is_none()),
        "one-sided walls have no partners"
    );

    validate_gl_bsp(&map, &built, &warnings);
}

/// The L-shaped (concave) single-sector room. The reflex corner forces at least
/// one interior partition, so the build yields >= 1 node; the interior cut is
/// closed with minisegs, so the full oracle must still hold.
#[test]
fn build_gl_nodes_l_room_has_nodes_and_passes_the_oracle() {
    let map = l_room_map();
    let (built, warnings) = build_gl_nodes(&map, &NodeBuildOptions::strict()).expect("builds");
    assert!(warnings.is_empty(), "L-room builds strict-clean");

    assert!(
        !built.nodes.is_empty(),
        "a concave room needs at least one node"
    );
    assert_eq!(built.subsectors.len(), built.nodes.len() + 1);
    validate_gl_bsp(&map, &built, &warnings);
}

/// The L-room geometry: outer corners (0,0) (256,0) (256,128) (128,128)
/// (128,256) (0,256), chained one-sided against sector 0. Concave at (128,128).
fn l_room_map() -> Map {
    let points = [
        (0i16, 0i16),
        (256, 0),
        (256, 128),
        (128, 128),
        (128, 256),
        (0, 256),
    ];
    let lines = [
        (0u16, 1u16, Some(0u16), None),
        (1, 2, Some(0), None),
        (2, 3, Some(0), None),
        (3, 4, Some(0), None),
        (4, 5, Some(0), None),
        (5, 0, Some(0), None),
    ];
    assemble_general(&points, &lines)
}

/// Two rooms joined by a doorway — the shared boundary is one two-sided wall.
/// The wall forces at least one BSP node, and its two facing segs are emitted as
/// a mutual-partner pair (the GL involution): partner links point back at each
/// other, spans mirror, and neither is its own partner. Both rooms are convex and
/// fully walled, so the full oracle (including convexity) holds.
///
/// This is the fixture that exercises **real (linedef-backed) partnered segs**.
/// The separate [`build_gl_nodes_corridor_room_produces_minisegs`] fixture
/// exercises minisegs.
#[test]
fn build_gl_nodes_two_room_doorway_partners_the_shared_wall() {
    let map = two_room_doorway_map();
    let (built, warnings) = build_gl_nodes(&map, &NodeBuildOptions::strict()).expect("builds");
    assert!(warnings.is_empty(), "two proper rooms build strict-clean");
    assert!(
        !built.nodes.is_empty(),
        "the shared wall forces at least one node"
    );

    validate_gl_bsp(&map, &built, &warnings);

    // The two-sided shared wall yields at least one mutual-partner seg pair.
    let mut found_pair = false;
    for (i, s) in built.segs.iter().enumerate() {
        if let Some(p) = s.partner {
            assert_ne!(p.0, i, "a seg is never its own partner");
            assert_eq!(
                built.segs[p.0].partner.map(|q| q.0),
                Some(i),
                "partner involution"
            );
            assert_eq!(built.segs[i].start, built.segs[p.0].end, "mirrored span");
            assert_eq!(built.segs[i].end, built.segs[p.0].start, "mirrored span");
            found_pair = true;
        }
    }
    assert!(
        found_pair,
        "the two-sided shared wall produces a partnered seg pair"
    );
}

/// Two proper convex rooms (sector 0 west, sector 1 east) sharing the vertical
/// wall at x = 64, wound Doom-correct (each one-sided wall has its sector on the
/// right). Linedef 2 is the only two-sided line — the doorway. Mirrors the
/// kernel's own `two_room_correct` fixture.
fn two_room_doorway_map() -> Map {
    let points = [
        (0i16, 0i16), // 0
        (0, 64),      // 1
        (64, 64),     // 2
        (64, 0),      // 3
        (128, 0),     // 4
        (128, 64),    // 5
    ];
    let lines = [
        (0u16, 1u16, Some(0u16), None), // west, room 0
        (1, 2, Some(0), None),          // north, room 0
        (2, 3, Some(0), Some(1)),       // SHARED wall (right = room 0, left = room 1)
        (3, 0, Some(0), None),          // south, room 0
        (2, 5, Some(1), None),          // north, room 1
        (5, 4, Some(1), None),          // east, room 1
        (4, 3, Some(1), None),          // south, room 1
    ];
    assemble_general(&points, &lines)
}

/// A single-sector "dumbbell": two square rooms joined by a narrow corridor, all
/// one-sided walls facing sector 0. The corridor makes the footprint concave, so
/// the BSP must cut through open floor; each such cut is sealed with a
/// **connecting miniseg** (`linedef: None`, `partner: None` — normal GL
/// leaf-closing operation). This fixture asserts that minisegs appear and that
/// the full oracle (including convexity) holds.
///
/// Connecting minisegs are unpartnered by construction (they bridge a convex
/// leaf's boundary gap); the mutual-partner *partition* miniseg only arises when a
/// partition crosses a floor-on-both-sides gap, which this open-corridor geometry
/// does not force. Partnered segs are exercised by
/// [`build_gl_nodes_two_room_doorway_partners_the_shared_wall`].
///
/// # Known integration gap: partnered partition-minisegs
///
/// No fixture here exercises builder-created partitioned miniseg **pairs** (two
/// minisegs that are mutual partners) end-to-end through the public API: the
/// corridor's connecting minisegs are unpartnered by design, and the doorway's
/// partners are real (linedef-backed) segs, not minisegs. Partnered
/// partition-minisegs *are* covered — at the unit level by the kernel's Task-5
/// tests in `gl_nodes.rs`, and by the retail sweep, where two-sided walls force
/// them across real geometry. An integration fixture that forces a partnered
/// partition-miniseg pair *without* also tripping a `DegenerateLeaf` has not been
/// found; rather than fabricate one that only passes by luck, this gap is left
/// explicit — revisit it with the sweep run.
#[test]
fn build_gl_nodes_corridor_room_produces_minisegs() {
    let map = corridor_room_map();
    let (built, warnings) = build_gl_nodes(&map, &NodeBuildOptions::strict()).expect("builds");
    assert!(warnings.is_empty(), "the corridor room builds strict-clean");

    validate_gl_bsp(&map, &built, &warnings);

    let minisegs = built.segs.iter().filter(|s| s.linedef.is_none()).count();
    assert!(
        minisegs > 0,
        "cutting through open corridor floor produces minisegs"
    );
}

/// The dumbbell corridor room: a left room (x 0..64), a right room (x 192..256),
/// and a narrow corridor (y 16..48) joining them, all y 0..64. The twelve
/// boundary vertices are chained one-sided against sector 0; the corridor
/// junctions (vertices 2, 3, 8, 9) are the reflex corners that force interior
/// cuts.
fn corridor_room_map() -> Map {
    let points = [
        (0i16, 0i16), // 0
        (64, 0),      // 1
        (64, 16),     // 2 corridor bottom-left (reflex)
        (192, 16),    // 3 corridor bottom-right (reflex)
        (192, 0),     // 4
        (256, 0),     // 5
        (256, 64),    // 6
        (192, 64),    // 7
        (192, 48),    // 8 corridor top-right (reflex)
        (64, 48),     // 9 corridor top-left (reflex)
        (64, 64),     // 10
        (0, 64),      // 11
    ];
    let mut lines = Vec::new();
    for i in 0u16..12 {
        lines.push((i, (i + 1) % 12, Some(0u16), None));
    }
    assemble_general(&points, &lines)
}

/// A fractional split: a concave single-sector "chevron" room whose diagonal
/// notch walls are crossed by the BSP partition at an off-lattice point. All
/// walls are one-sided against sector 0 (so the leaves stay properly convex), yet
/// the interior partition splits a diagonal wall at a coordinate with a nonzero
/// fractional part — proving at least one `GlVertexRef::Gl` split vertex carries
/// a non-integral `f64` coordinate, i.e. that 16.16 sub-unit precision survives.
#[test]
fn build_gl_nodes_fractional_split_makes_off_lattice_vertex() {
    // A rectangle-ish outline with a downward "V" notch bitten out of the right
    // edge: the notch tip at (90,80) and the two diagonal notch walls
    // (200,150)->(90,80) and (90,80)->(200,151) are what a partition slices at a
    // fractional point.
    let points = [
        (0i16, 0i16),
        (200, 0),
        (200, 150),
        (90, 80), // notch tip (reflex)
        (200, 151),
        (200, 300),
        (0, 300),
    ];
    let mut lines = Vec::new();
    let n = u16::try_from(points.len()).unwrap();
    for i in 0..n {
        lines.push((i, (i + 1) % n, Some(0u16), None));
    }
    let map = assemble_general(&points, &lines);
    let (built, warnings) = build_gl_nodes(&map, &NodeBuildOptions::strict()).expect("builds");
    assert!(
        warnings.is_empty(),
        "fractional-split map builds strict-clean"
    );
    validate_gl_bsp(&map, &built, &warnings);

    // At least one split vertex is off the integer lattice.
    let off_lattice = built
        .gl_vertices
        .iter()
        .any(|v| v.x.fract() != 0.0 || v.y.fract() != 0.0);
    assert!(
        off_lattice,
        "an off-lattice partition intersection yields a fractional GL vertex: {:?}",
        built.gl_vertices
    );
}

/// Determinism: the same map built twice yields byte-identical arenas (the full
/// [`BuiltGlNodes`] compares equal via its derived `PartialEq`).
#[test]
fn build_gl_nodes_is_deterministic() {
    let map = l_room_map();
    let a = build_gl_nodes(&map, &NodeBuildOptions::strict())
        .expect("builds")
        .0;
    let b = build_gl_nodes(&map, &NodeBuildOptions::strict())
        .expect("builds")
        .0;
    assert_eq!(a, b, "GL node building is deterministic");
}

// --- Proptest ----------------------------------------------------------------

/// Whether a `build_gl_nodes` error is one the plan permits (never a panic): the
/// shared narrowing/format errors plus the GL-specific structural guards. Mirrors
/// `build_lumps.rs`'s `is_plan_known_error`, extended with the GL
/// [`NodeBuildError::DegenerateLeaf`].
fn is_plan_known_error(err: &NodeBuildError) -> bool {
    matches!(
        err,
        NodeBuildError::EmptyGeometry
            | NodeBuildError::Write(_)
            | NodeBuildError::TooManyElements { .. }
            | NodeBuildError::MixedSectorSubsector { .. }
            | NodeBuildError::DegeneratePartition { .. }
            | NodeBuildError::DegenerateLeaf { .. }
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    /// The structural GL oracle holds on every generated single-sector map that
    /// builds `Ok`, and any build that errors does so with a plan-known variant —
    /// never a panic — in both strictness modes. Reuses the classic pass's
    /// random-geometry shape (small integer coordinates, index-wrapped linedefs,
    /// optional two-sided lines). Every side faces sector 0, so a produced
    /// subsector is single-sector.
    ///
    /// This asserts [`validate_gl_structure`], not the full [`validate_gl_bsp`]:
    /// the generator freely crosses and overlaps walls (and two-sided sector-0
    /// lines are self-referencing), which legitimately yields valid-but-non-convex
    /// subsector loops (see [`assert_subsectors_convex`]). Convexity is asserted
    /// on the designed, non-overlapping fixtures instead.
    #[test]
    fn build_gl_nodes_random_single_sector_maps_hold_the_oracle(
        coords in prop::collection::vec((-1024i16..=1024, -1024i16..=1024), 2..=10),
        raw_lines in prop::collection::vec((0usize..10, 0usize..10, any::<bool>()), 1..=12),
    ) {
        let n = coords.len();
        let lines: Vec<(u16, u16, Option<u16>, Option<u16>)> = raw_lines
            .iter()
            .map(|&(s, e, two_sided)| {
                let start = u16::try_from(s % n).unwrap();
                let end = u16::try_from(e % n).unwrap();
                (start, end, Some(0u16), two_sided.then_some(0u16))
            })
            .collect();
        let map = assemble_general(&coords, &lines);

        for opts in [NodeBuildOptions::strict(), NodeBuildOptions::lenient()] {
            match build_gl_nodes(&map, &opts) {
                Ok((built, warnings)) => validate_gl_structure(&map, &built, &warnings),
                Err(e) => prop_assert!(
                    is_plan_known_error(&e),
                    "build_gl_nodes returned an unexpected error variant: {e:?}"
                ),
            }
        }
    }
}

// --- Retail sweep ------------------------------------------------------------

/// Optional retail-WAD sweep for the GL BSP builder (ADR-0026): build GL nodes
/// for every assemblable classic-format map in a local collection and run the
/// structural GL oracle over real geometry.
///
/// Same gating/skip pattern as the classic sweep in `build_lumps.rs`: point
/// `CRUSTYWAD_SWEEP_DIR` at a directory of WAD files (**absolute path** — cargo
/// runs the test binary with its CWD at the package root, so a relative path
/// resolves against the wrong directory and the sweep silently skips). Doom 64
/// maps ship pre-built nodes and are not a build target, so they are skipped.
///
/// The sweep runs `build_gl_nodes` in **lenient** mode. Its warning set is pinned
/// to the four GL-tolerated classes (`Write | VanillaCeilingExceeded |
/// MixedSectorSubsector | DegenerateLeaf`), enforced inside
/// [`validate_gl_structure`]; every other warning, and every error, fails the
/// sweep. Geometric convexity is **not** asserted here: retail maps routinely
/// carry self-referencing and crossing lines, which yield valid-but-non-convex
/// GL subsector loops (see [`assert_subsectors_convex`]).
#[cfg(feature = "sweep-tests")]
#[test]
#[allow(clippy::too_many_lines)]
fn sweep_builds_gl_nodes_for_every_classic_map() {
    use crustywad::map::{MapFormat, detect_map_format};

    let paths = common::wad_files("CRUSTYWAD_SWEEP_DIR");
    if paths.is_empty() {
        return; // skip note already printed by the helper
    }

    let mut maps_built = 0usize;
    let mut doom64_skipped = 0usize;
    let mut total_segs = 0usize;
    let mut total_minisegs = 0usize;
    let mut degenerate_leaf_maps = 0usize;

    for path in &paths {
        let wad = Wad::from_path_with_options(path, ParseOptions::strict())
            .unwrap_or_else(|e| panic!("{}: container failed strict parse: {e}", path.display()));

        for group in wad.map_groups() {
            if detect_map_format(&wad, &group) == MapFormat::Doom64 {
                doom64_skipped += 1;
                continue;
            }

            // Assemble leniently so the sweep exercises the builder over every
            // classic map, matching the classic sweeps.
            let map = Map::assemble_with_options(&wad, &group, ParseOptions::lenient())
                .unwrap_or_else(|e| {
                    panic!(
                        "{}: map {} failed lenient assembly: {e}",
                        path.display(),
                        group.name
                    )
                });

            let (built, warnings) = build_gl_nodes(&map, &NodeBuildOptions::lenient())
                .unwrap_or_else(|e| {
                    panic!(
                        "{}: map {} lenient build_gl_nodes failed: {e:?}",
                        path.display(),
                        group.name
                    )
                });

            // The structural oracle enforces the allowed warning set and every
            // format-domain invariant over real geometry (convexity excluded —
            // real maps carry self-referencing/crossing lines).
            validate_gl_structure(&map, &built, &warnings);

            if warnings
                .iter()
                .any(|w| matches!(w, NodeBuildWarning::DegenerateLeaf { .. }))
            {
                degenerate_leaf_maps += 1;
            }
            total_segs += built.segs.len();
            total_minisegs += built.segs.iter().filter(|s| s.linedef.is_none()).count();
            maps_built += 1;
        }
    }

    assert!(
        maps_built > 0,
        "CRUSTYWAD_SWEEP_DIR contained {} WAD file(s) but no classic maps were built",
        paths.len()
    );
    eprintln!(
        "built GL nodes for {} WAD(s): {maps_built} classic map(s), {doom64_skipped} Doom 64 skipped, {total_segs} total GL segs ({total_minisegs} minisegs); {degenerate_leaf_maps} map(s) carried a tolerated degenerate leaf",
        paths.len()
    );
}
