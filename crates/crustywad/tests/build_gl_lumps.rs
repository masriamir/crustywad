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
    BuiltGlNodes, NodeBuildError, NodeBuildOptions, NodeBuildWarning, NodeFormat,
    add_doom_map_with_nodes, add_udmf_map_with_nodes, build_gl_nodes, build_nodes,
};
use crustywad::map::{GlNodeChild, GlVertexRef, Map, NodeChild};
use crustywad::{ParseOptions, Wad, WadBuilder, WadKind, WriteOptions};
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

/// One linedef of a general fixture: `(start, end, right_sector, left_sector)` —
/// a `Some(sector)` side gets a sidedef facing that sector, a `None` side is the
/// `0xffff` "no sidedef" sentinel.
type Line = (u16, u16, Option<u16>, Option<u16>);

/// The five classic map lumps (raw on-disk bytes) for a fixture, in WAD order —
/// the geometry an extended-node round-trip re-synthesizes around a rebuilt
/// `SEGS`/`SSECTORS`/`NODES` triple (see [`reread_via_ssectors`]).
struct MapLumps {
    things: Vec<u8>,
    linedefs: Vec<u8>,
    sidedefs: Vec<u8>,
    vertexes: Vec<u8>,
    sectors: Vec<u8>,
}

/// Builds the five classic map lumps from vertices and general linedefs. Each
/// linedef is `(start, end, right_sector, left_sector)`: a `Some(sector)` side
/// gets a fresh sidedef facing that sector; a `None` side is the `0xffff` "no
/// sidedef" sentinel. Two-sided (both sides present) linedefs get the two-sided
/// flag. Enough sectors are emitted to cover the highest referenced index.
fn map_lumps_general(points: &[(i16, i16)], lines: &[Line]) -> MapLumps {
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
    MapLumps {
        things: thing_bytes(0, 0, 0, 1, 7),
        linedefs,
        sidedefs,
        vertexes: vertexes_bytes(points),
        sectors,
    }
}

/// Assembles a classic Doom `Map` from the five map lumps.
fn assemble_from_lumps(lumps: &MapLumps) -> Map {
    let bytes = common::build_doom_map_wad(
        "MAP01",
        lumps.things.clone(),
        lumps.linedefs.clone(),
        lumps.sidedefs.clone(),
        lumps.vertexes.clone(),
        lumps.sectors.clone(),
    );
    let wad = Wad::from_bytes(bytes).expect("fixture WAD parses");
    let group = wad.map_group("MAP01").expect("map group present");
    Map::assemble(&wad, &group).expect("map assembles")
}

/// Assembles a Doom map from vertices and general linedefs. Copied from
/// `build_lumps.rs` so the GL fixtures build through the same public path.
fn assemble_general(points: &[(i16, i16)], lines: &[Line]) -> Map {
    assemble_from_lumps(&map_lumps_general(points, lines))
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
/// - Warning set: every emitted warning is one of the three allowed GL variants
///   (`Write | VanillaCeilingExceeded | MixedSectorSubsector`).
/// - Structural [`BuiltGlNodes::validate`] passes (subsector partition, vertex
///   ranges, node-child ranges, partner involution + mirrored spans, closed
///   loops) unconditionally, in both modes — every leaf, degenerate or not, is
///   closed into a cyclic loop by the connecting-miniseg path (a degenerate
///   1-seg leaf becomes a closed 2-vertex loop).
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
    // (0) Warning set: only the three GL-permitted variants may appear.
    for w in warnings {
        assert!(
            matches!(
                w,
                NodeBuildWarning::Write(_)
                    | NodeBuildWarning::VanillaCeilingExceeded { .. }
                    | NodeBuildWarning::MixedSectorSubsector { .. }
            ),
            "unexpected GL node-build warning: {w:?}"
        );
    }

    // (1) Structural validation: passes unconditionally in both modes — every
    //     leaf, degenerate or not, is closed into a cyclic loop.
    if let Err(e) = built.validate(map.vertices().len()) {
        panic!("BuiltGlNodes::validate failed: {e:?}");
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
/// colinear points is allowed). Runs shorter than 3 segs are skipped (a
/// degenerate ≤2-vertex loop has no turn to test).
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
fn assert_subsectors_convex(map: &Map, built: &BuiltGlNodes) {
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
        // Intentional no-assert: if every turn was collinear (`sign` still 0), the
        // subsector is a fully degenerate line with no orientation to test — a
        // valid convex extreme, not a failure. There is nothing to assert here.
    }
}

/// The full GL oracle for a **well-formed** (non-overlapping, properly-sectored)
/// fixture: [`validate_gl_structure`] plus geometric convexity of every
/// subsector ([`assert_subsectors_convex`]). Used by the designed fixtures. The
/// proptest and retail sweep, whose geometry is arbitrary, assert
/// [`validate_gl_structure`] only.
fn validate_gl_bsp(map: &Map, built: &BuiltGlNodes, warnings: &[NodeBuildWarning]) {
    validate_gl_structure(map, built, warnings);
    assert_subsectors_convex(map, built);
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

    // Explicit root-last check on this driver fixture: the last node is the tree
    // root (referenced by no other node), and every non-root node is referenced
    // exactly once as a child.
    let mut refcount = vec![0usize; built.nodes.len()];
    for n in &built.nodes {
        for child in [n.right, n.left] {
            if let GlNodeChild::Node(k) = child {
                refcount[k.0] += 1;
            }
        }
    }
    let root = built.nodes.len() - 1;
    assert_eq!(
        refcount[root], 0,
        "the root (last node) is referenced by no node"
    );
    for (i, &c) in refcount.iter().enumerate() {
        if i != root {
            assert_eq!(c, 1, "non-root node {i} is referenced exactly once");
        }
    }

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
    let (points, lines) = two_room_doorway_geometry();
    assemble_general(&points, &lines)
}

/// The two-room-doorway geometry as `(points, lines)` — factored out so the
/// write→read round-trip fixture can rebuild the same map lumps around a
/// re-synthesized extended-node stream.
fn two_room_doorway_geometry() -> (Vec<(i16, i16)>, Vec<Line>) {
    let points = vec![
        (0i16, 0i16), // 0
        (0, 64),      // 1
        (64, 64),     // 2
        (64, 0),      // 3
        (128, 0),     // 4
        (128, 64),    // 5
    ];
    let lines = vec![
        (0u16, 1u16, Some(0u16), None), // west, room 0
        (1, 2, Some(0), None),          // north, room 0
        (2, 3, Some(0), Some(1)),       // SHARED wall (right = room 0, left = room 1)
        (3, 0, Some(0), None),          // south, room 0
        (2, 5, Some(1), None),          // north, room 1
        (5, 4, Some(1), None),          // east, room 1
        (4, 3, Some(1), None),          // south, room 1
    ];
    (points, lines)
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
/// partition-miniseg pair has not been
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
    let (points, lines) = fractional_chevron_geometry();
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

    // Stronger: the off-lattice vertex is actually *wired into the geometry* — at
    // least one emitted seg endpoint resolves to a `GlVertexRef::Gl` whose GL
    // vertex carries non-integral coordinates (not merely present in the arena).
    let endpoint_off_lattice = built.segs.iter().any(|s| {
        [s.start, s.end].into_iter().any(|r| match r {
            GlVertexRef::Gl(v) => {
                let gv = built.gl_vertices[v.0];
                gv.x.fract() != 0.0 || gv.y.fract() != 0.0
            }
            GlVertexRef::Normal(_) => false,
        })
    });
    assert!(
        endpoint_off_lattice,
        "a seg endpoint references a fractional GL vertex: {:?}",
        built.gl_vertices
    );
}

/// The fractional-chevron geometry as `(points, lines)`: a rectangle-ish outline
/// with a downward "V" notch bitten out of the right edge — the notch tip at
/// (90,80) and the two diagonal notch walls (200,150)->(90,80) and
/// (90,80)->(200,151) are what a partition slices at a fractional point. Factored
/// out so the round-trip fixture can rebuild the same lumps.
fn fractional_chevron_geometry() -> (Vec<(i16, i16)>, Vec<Line>) {
    let points = vec![
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
    (points, lines)
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

// --- Write→read round-trip through the SSECTORS carrier ----------------------

/// Re-synthesizes a WAD around `lumps` — the original THINGS/LINEDEFS/SIDEDEFS/
/// VERTEXES/SECTORS — plus a rebuilt node triple (`SEGS = b""`, `SSECTORS =
/// stream`, `NODES = b""`), then assembles it strict. The empty `NODES` makes the
/// assembler's binary-path dispatch fall through to `SSECTORS`, where the extended
/// `XGL3`/`ZGL3` stream is probed and decoded into the graph's
/// `MapSeg`/`MapSubsector`/`MapNode` arenas.
fn reread_via_ssectors(lumps: &MapLumps, stream: &[u8]) -> Map {
    let bytes = common::build_doom_map_wad_with_lumps(
        "MAP01",
        lumps.things.clone(),
        lumps.linedefs.clone(),
        lumps.sidedefs.clone(),
        lumps.vertexes.clone(),
        lumps.sectors.clone(),
        &[("SEGS", b""), ("SSECTORS", stream), ("NODES", b"")],
    );
    let wad = Wad::from_bytes(bytes).expect("reread WAD parses");
    let group = wad.map_group("MAP01").expect("reread map group present");
    Map::assemble_with_options(&wad, &group, ParseOptions::strict())
        .expect("extended-node stream decodes")
}

/// The 16.16 fixed-point coordinates of a reader seg endpoint, resolved through
/// the reread map's combined vertex table (the original `VERTEXES` first, then the
/// stream's appended GL vertices). The recovered value is exact: an original
/// vertex is a whole-unit `f64` and an appended GL vertex was decoded as
/// `fixed / 65536.0` (a power-of-two divide), so `* 65536` inverts both losslessly
/// — the exact counterpart of [`gl_ref_fixed`] on the build side.
fn reader_vertex_fixed(map: &Map, v: usize) -> (i64, i64) {
    let mv = map.vertices()[v];
    #[allow(clippy::cast_possible_truncation)]
    let fx = (mv.x * 65536.0).round() as i64;
    #[allow(clippy::cast_possible_truncation)]
    let fy = (mv.y * 65536.0).round() as i64;
    (fx, fy)
}

/// Asserts everything the writer→reader pair must preserve, excluding the two
/// documented reader losses (partner links, dropped; fractional node partitions,
/// truncated `>> 16`). The seg and subsector arenas are index-aligned — the writer
/// emits them in build order and the reader reads them back in the same order — so
/// `built.*[i]` corresponds to `reread.*()[i]`.
fn assert_round_trip(src_map: &Map, built: &BuiltGlNodes, reread: &Map) {
    // Seg count round-trips exactly.
    assert_eq!(
        reread.segs().len(),
        built.segs.len(),
        "seg count round-trips"
    );
    for (i, (bs, rs)) in built.segs.iter().zip(reread.segs()).enumerate() {
        // Per-seg linedef Option: a miniseg's `None` survives.
        assert_eq!(rs.linedef, bs.linedef, "seg {i} linedef option round-trips");
        // Direction is derived from the on-disk side bit.
        assert_eq!(
            rs.direction,
            u16::from(bs.side != 0),
            "seg {i} direction == u16::from(side != 0)"
        );
        // Endpoints via implicit-v2 reconstruction: the reader's start/end resolve
        // (through the combined vertex table) to the built seg's flattened refs.
        assert_eq!(
            reader_vertex_fixed(reread, rs.start.0),
            gl_ref_fixed(src_map, built, bs.start),
            "seg {i} start endpoint reconstructs"
        );
        assert_eq!(
            reader_vertex_fixed(reread, rs.end.0),
            gl_ref_fixed(src_map, built, bs.end),
            "seg {i} end endpoint reconstructs (implicit v2)"
        );
    }

    // Subsector seg ranges round-trip exactly.
    assert_eq!(
        reread.subsectors().len(),
        built.subsectors.len(),
        "subsector count round-trips"
    );
    for (i, (bss, rss)) in built.subsectors.iter().zip(reread.subsectors()).enumerate() {
        assert_eq!(rss.segs, bss.segs, "subsector {i} seg range round-trips");
    }

    // Node partitions: the reader truncates the 16.16 partition to whole units
    // (`>> 16`) — a documented loss, so compare against the shifted built value.
    assert_eq!(
        reread.nodes().len(),
        built.nodes.len(),
        "node count round-trips"
    );
    for (i, (bn, rn)) in built.nodes.iter().zip(reread.nodes()).enumerate() {
        assert_eq!(rn.x, bn.x >> 16, "node {i} partition x truncates to whole");
        assert_eq!(rn.y, bn.y >> 16, "node {i} partition y truncates to whole");
        assert_eq!(
            rn.dx,
            bn.dx >> 16,
            "node {i} partition dx truncates to whole"
        );
        assert_eq!(
            rn.dy,
            bn.dy >> 16,
            "node {i} partition dy truncates to whole"
        );
        // Bounding boxes are whole map units on both sides — no truncation, so
        // the reader's `[i32; 4]` equals the built `[i32; 4]` exactly.
        assert_eq!(
            rn.right_bbox, bn.right_bbox,
            "node {i} right bbox round-trips exactly"
        );
        assert_eq!(
            rn.left_bbox, bn.left_bbox,
            "node {i} left bbox round-trips exactly"
        );
        // Children map across the two structurally-identical child enums.
        assert!(
            child_matches(rn.right, bn.right),
            "node {i} right child round-trips ({:?} vs {:?})",
            rn.right,
            bn.right
        );
        assert!(
            child_matches(rn.left, bn.left),
            "node {i} left child round-trips ({:?} vs {:?})",
            rn.left,
            bn.left
        );
    }
}

/// Whether a reader [`NodeChild`] denotes the same child as a built
/// [`GlNodeChild`]: same variant, same index. The two enums are structurally
/// identical but distinct types (reader `NodeIdx`/`SubsectorIdx` vs built
/// `GlNodeIdx`/`GlSubsectorIdx`), so a small local mapping bridges them.
fn child_matches(reader: NodeChild, built: GlNodeChild) -> bool {
    use crustywad::map::{GlNodeIdx, GlSubsectorIdx, NodeIdx, SubsectorIdx};
    match (reader, built) {
        (NodeChild::Node(NodeIdx(a)), GlNodeChild::Node(GlNodeIdx(b))) => a == b,
        (NodeChild::Subsector(SubsectorIdx(a)), GlNodeChild::Subsector(GlSubsectorIdx(b))) => {
            a == b
        }
        _ => false,
    }
}

/// The doorway map (a two-sided shared wall forcing a node and a partnered seg
/// pair) survives a full XGL3 write→read round-trip through the SSECTORS carrier:
/// every preserved field matches, with the two documented losses excluded.
#[test]
fn xgl3_round_trips_through_the_reader_on_the_doorway_map() {
    let (points, lines) = two_room_doorway_geometry();
    let lumps = map_lumps_general(&points, &lines);
    let map = assemble_from_lumps(&lumps);
    let (built, warnings) = build_gl_nodes(&map, &NodeBuildOptions::strict()).expect("builds");
    assert!(warnings.is_empty(), "doorway builds strict-clean");
    assert!(!built.nodes.is_empty(), "the shared wall forces a node");

    let orig = map.vertices().len();
    let stream = built
        .to_extended_lump_bytes(orig, NodeFormat::Xgl3)
        .expect("XGL3 stream serializes");
    let reread = reread_via_ssectors(&lumps, &stream);
    assert_round_trip(&map, &built, &reread);
}

/// The same doorway map round-trips through the zlib-compressed `ZGL3` container
/// (feature-gated), yielding the identical decoded arenas its `XGL3` twin does.
#[cfg(feature = "extended-nodes-zlib")]
#[test]
fn zgl3_round_trips_the_same_map() {
    let (points, lines) = two_room_doorway_geometry();
    let lumps = map_lumps_general(&points, &lines);
    let map = assemble_from_lumps(&lumps);
    let (built, warnings) = build_gl_nodes(&map, &NodeBuildOptions::strict()).expect("builds");
    assert!(warnings.is_empty(), "doorway builds strict-clean");

    let orig = map.vertices().len();
    let stream = built
        .to_extended_lump_bytes(orig, NodeFormat::Zgl3)
        .expect("ZGL3 stream serializes");
    let reread = reread_via_ssectors(&lumps, &stream);
    assert_round_trip(&map, &built, &reread);
}

/// `NodeFormat::Gl` auto-resolution round-trips end to end on the doorway map:
/// its whole-unit coordinates never force a fractional partition, so
/// `resolve_gl_dialect` picks the minimal dialect, `Xgln`. The reader widens
/// that stream's `i16` partitions to whole `i32`, which is exactly what
/// [`assert_round_trip`]'s `built >> 16` expectation already checks — the same
/// comparator serves both the explicit-`Xgl3` fixture above and this
/// auto-resolved one, since a whole-unit `i32` shifted right 16 bits equals the
/// `i16` the `Xgln` reader widened back up.
#[test]
fn gl_auto_round_trips_the_doorway_map_as_xgln() {
    let (points, lines) = two_room_doorway_geometry();
    let lumps = map_lumps_general(&points, &lines);
    let map = assemble_from_lumps(&lumps);
    let (built, warnings) = build_gl_nodes(&map, &NodeBuildOptions::strict()).expect("builds");
    assert!(warnings.is_empty(), "doorway builds strict-clean");
    assert!(!built.nodes.is_empty(), "the shared wall forces a node");

    let orig = map.vertices().len();
    let stream = built
        .to_extended_lump_bytes(orig, NodeFormat::Gl)
        .expect("Gl auto-resolution serializes");
    assert_eq!(
        &stream[..4],
        b"XGLN",
        "whole-unit doorway partitions resolve to the minimal XGLN dialect"
    );
    let reread = reread_via_ssectors(&lumps, &stream);
    assert_round_trip(&map, &built, &reread);
}

/// `NodeFormat::Gl` auto-resolution escalates all the way to `Xgl3` when the
/// BSP partition itself is genuinely fractional (not merely a fractional
/// **split vertex**, which [`fractional_chevron_geometry`]'s notch produces —
/// per `resolve_gl_dialect`'s own contract, "GL vertices never force
/// escalation": a partition candidate is always a piece of an original
/// linedef, so its anchor is only fractional when that piece itself was
/// produced by an earlier off-lattice split, which the single-level chevron
/// notch does not trigger). [`fractional_partition_pentagon_geometry`] is a
/// small single-sector fixture (found by sweeping tiny single-sector polygons
/// for one whose *own* interior partition anchors on a non-integral 16.16
/// coordinate) that does trigger it, confirmed by inspecting
/// `BuiltGlNodes::nodes` directly before asserting on the serialized header.
/// This proves auto-resolution never silently truncates a fractional
/// partition down to a coarser dialect that can't hold it.
#[test]
fn gl_auto_round_trips_the_fractional_partition_pentagon_as_xgl3() {
    let (points, lines) = fractional_partition_pentagon_geometry();
    let lumps = map_lumps_general(&points, &lines);
    let map = assemble_from_lumps(&lumps);
    let (built, warnings) = build_gl_nodes(&map, &NodeBuildOptions::strict()).expect("builds");
    assert!(warnings.is_empty(), "pentagon builds strict-clean");
    assert!(
        built.nodes.iter().any(|node| {
            [node.x, node.y, node.dx, node.dy]
                .into_iter()
                .any(|v| v & 0xffff != 0)
        }),
        "fixture precondition: at least one node partition is genuinely fractional"
    );

    let orig = map.vertices().len();
    let stream = built
        .to_extended_lump_bytes(orig, NodeFormat::Gl)
        .expect("Gl auto-resolution serializes");
    assert_eq!(
        &stream[..4],
        b"XGL3",
        "a fractional partition forces escalation to the XGL3 dialect"
    );
    let reread = reread_via_ssectors(&lumps, &stream);
    assert_round_trip(&map, &built, &reread);
}

/// The fractional-partition pentagon geometry as `(points, lines)`: a small
/// single-sector, all-one-sided closed pentagon whose interior BSP partition
/// lands on a genuinely fractional 16.16 anchor point (verified above, not
/// merely asserted) — unlike [`fractional_chevron_geometry`], whose fractional
/// coordinate lives only on a split *vertex*, not a partition anchor.
fn fractional_partition_pentagon_geometry() -> (Vec<(i16, i16)>, Vec<Line>) {
    let points = vec![(23i16, 23i16), (0, -3), (-9, 17), (37, 7), (18, 38)];
    let n = u16::try_from(points.len()).unwrap();
    let lines = (0..n).map(|i| (i, (i + 1) % n, Some(0u16), None)).collect();
    (points, lines)
}

/// Fractional GL vertices survive the 16.16 vertex header: the chevron fixture's
/// off-lattice split vertices re-read (beyond `orig_vertex_count`) with the exact
/// non-integral coordinates the builder produced.
#[test]
fn xgl3_preserves_fractional_gl_vertices_through_the_header() {
    let (points, lines) = fractional_chevron_geometry();
    let lumps = map_lumps_general(&points, &lines);
    let map = assemble_from_lumps(&lumps);
    let (built, warnings) = build_gl_nodes(&map, &NodeBuildOptions::strict()).expect("builds");
    assert!(warnings.is_empty(), "chevron builds strict-clean");
    assert!(
        !built.gl_vertices.is_empty(),
        "the fractional split produces GL vertices"
    );

    let orig = map.vertices().len();
    let stream = built
        .to_extended_lump_bytes(orig, NodeFormat::Xgl3)
        .expect("XGL3 stream serializes");
    let reread = reread_via_ssectors(&lumps, &stream);

    // Every appended reader vertex (beyond the originals) equals the built GL
    // vertex it came from — compared in 16.16 fixed space, where both sides
    // recover their exact on-disk value (`fixed / 65536.0` inverts losslessly),
    // so the comparison is integral and exact.
    assert_eq!(
        reread.vertices().len(),
        orig + built.gl_vertices.len(),
        "the reader appends exactly the built GL vertices"
    );
    let mut saw_fractional = false;
    for (k, gv) in built.gl_vertices.iter().enumerate() {
        assert_eq!(
            reader_vertex_fixed(&reread, orig + k),
            gl_ref_fixed(
                &map,
                &built,
                GlVertexRef::Gl(crustywad::map::GlVertexIdx(k))
            ),
            "GL vertex {k} coordinates survive the 16.16 header"
        );
        if gv.x.fract() != 0.0 || gv.y.fract() != 0.0 {
            saw_fractional = true;
        }
    }
    assert!(
        saw_fractional,
        "at least one appended GL vertex is non-integral: {:?}",
        built.gl_vertices
    );
}

// --- One-shot GL arm (add_doom_map_with_nodes) -------------------------------

/// `add_doom_map_with_nodes` with a GL [`NodeFormat`] (`Xgl3`) emits the GL
/// stream in the `SSECTORS` carrier — the inverse of the reader's
/// NODES-then-SSECTORS probe (an empty `NODES` lump makes the assembler fall
/// through to `SSECTORS`, where a GL stream signature is recognized). `SEGS`
/// and `NODES` are both empty; `VERTEXES` is untouched (byte-identical to the
/// input map's own vertices — GL split vertices live in the stream header, not
/// appended to the classic lump, exactly like the non-GL extended arm). The
/// canonical eleven-lump order still holds, and the resulting WAD re-assembles
/// strict with a populated seg/subsector arena, proving the reader's dispatch
/// actually found the stream.
#[test]
fn oneshot_emits_xgl3_in_ssectors_with_empty_segs_and_nodes() {
    let map = two_room_doorway_map();
    let mut opts = NodeBuildOptions::strict();
    opts.format = NodeFormat::Xgl3;

    let mut builder = WadBuilder::new(WadKind::Pwad);
    let warnings =
        add_doom_map_with_nodes(&mut builder, "MAP01", &map, &WriteOptions::strict(), &opts)
            .expect("one-shot builds");
    assert!(
        warnings.is_empty(),
        "a clean map builds warning-free: {warnings:?}"
    );

    let bytes = builder.build().expect("WAD serializes");
    let wad = Wad::from_bytes(bytes).expect("built WAD parses");

    let names: Vec<&str> = wad.lumps().iter().map(crustywad::Lump::name).collect();
    assert_eq!(
        names,
        [
            "MAP01", "THINGS", "LINEDEFS", "SIDEDEFS", "VERTEXES", "SEGS", "SSECTORS", "NODES",
            "SECTORS", "REJECT", "BLOCKMAP",
        ],
        "canonical lump order still holds for the GL layout (Global Constraint 5)"
    );

    let lump_bytes = |name: &str| {
        let idx = wad
            .lumps()
            .iter()
            .position(|l| l.name() == name)
            .expect("lump present");
        wad.lump_bytes(idx).expect("lump bytes present")
    };

    // SSECTORS carries the XGL3 stream; SEGS/NODES are both empty (contrast
    // with the non-GL extended arm, which carries its stream in NODES).
    let ssectors_bytes = lump_bytes("SSECTORS");
    assert_eq!(
        &ssectors_bytes[..4],
        b"XGL3",
        "SSECTORS carries the XGL3 stream signature"
    );
    assert!(
        lump_bytes("SEGS").is_empty(),
        "SEGS is empty for the GL layout"
    );
    assert!(
        lump_bytes("NODES").is_empty(),
        "NODES is empty for the GL layout"
    );

    // VERTEXES holds only the map's own vertices, byte-identical to the input.
    assert_eq!(
        lump_bytes("VERTEXES"),
        vertexes_bytes(&two_room_doorway_geometry().0),
        "VERTEXES is untouched for the GL layout — split GL verts live in the stream header"
    );

    // The map re-assembles strict with a populated seg/subsector arena, proving
    // the reader's dispatch found the stream in SSECTORS.
    let group = wad.map_group("MAP01").expect("map group present");
    let assembled = Map::assemble(&wad, &group).expect("strict assembly");
    assert!(
        assembled.warnings().is_empty(),
        "the playable WAD assembles strict-clean: {:?}",
        assembled.warnings()
    );
    assert!(
        !assembled.segs().is_empty(),
        "the doorway map yields at least one seg"
    );
    assert!(
        !assembled.subsectors().is_empty(),
        "the doorway map yields at least one subsector"
    );
}

/// Proves the one-shot/auto-resolution composition needs no code change: the
/// GL arm in `add_doom_map_with_nodes` gates on the crate's private
/// `NodeFormat::is_gl` (which already covers `Gl`), and
/// `BuiltGlNodes::to_extended_lump_bytes` resolves `Gl` internally — so
/// feeding `NodeFormat::Gl` straight into the one-shot builder on the
/// (whole-unit) doorway map emits the auto-resolved `XGLN` stream in
/// `SSECTORS`, with `SEGS`/`NODES` both empty exactly like the explicit-dialect
/// arm, and the resulting WAD still re-assembles strict with a populated
/// seg/subsector arena.
#[test]
fn oneshot_emits_the_auto_resolved_gl_stream() {
    let map = two_room_doorway_map();
    let mut opts = NodeBuildOptions::strict();
    opts.format = NodeFormat::Gl;

    let mut builder = WadBuilder::new(WadKind::Pwad);
    let warnings =
        add_doom_map_with_nodes(&mut builder, "MAP01", &map, &WriteOptions::strict(), &opts)
            .expect("one-shot builds");
    assert!(
        warnings.is_empty(),
        "a clean map builds warning-free: {warnings:?}"
    );

    let bytes = builder.build().expect("WAD serializes");
    let wad = Wad::from_bytes(bytes).expect("built WAD parses");

    let lump_bytes = |name: &str| {
        let idx = wad
            .lumps()
            .iter()
            .position(|l| l.name() == name)
            .expect("lump present");
        wad.lump_bytes(idx).expect("lump bytes present")
    };

    // SSECTORS carries the auto-resolved stream; the whole-unit doorway
    // geometry resolves to the minimal XGLN dialect, same as the direct
    // `to_extended_lump_bytes(.., NodeFormat::Gl)` call above.
    let ssectors_bytes = lump_bytes("SSECTORS");
    assert_eq!(
        &ssectors_bytes[..4],
        b"XGLN",
        "SSECTORS carries the auto-resolved XGLN stream signature"
    );
    assert!(
        lump_bytes("SEGS").is_empty(),
        "SEGS is empty for the GL layout"
    );
    assert!(
        lump_bytes("NODES").is_empty(),
        "NODES is empty for the GL layout"
    );

    // The map re-assembles strict with a populated seg/subsector arena,
    // proving the reader's dispatch found the auto-resolved stream in
    // SSECTORS.
    let group = wad.map_group("MAP01").expect("map group present");
    let assembled = Map::assemble(&wad, &group).expect("strict assembly");
    assert!(
        assembled.warnings().is_empty(),
        "the playable WAD assembles strict-clean: {:?}",
        assembled.warnings()
    );
    assert!(
        !assembled.segs().is_empty(),
        "the doorway map yields at least one seg"
    );
    assert!(
        !assembled.subsectors().is_empty(),
        "the doorway map yields at least one subsector"
    );
}

/// The zlib-compressed `ZGL3` twin lands in the same `SSECTORS` carrier
/// (feature-gated).
#[cfg(feature = "extended-nodes-zlib")]
#[test]
fn oneshot_emits_zgl3_in_ssectors() {
    let map = two_room_doorway_map();
    let mut opts = NodeBuildOptions::strict();
    opts.format = NodeFormat::Zgl3;

    let mut builder = WadBuilder::new(WadKind::Pwad);
    let warnings =
        add_doom_map_with_nodes(&mut builder, "MAP01", &map, &WriteOptions::strict(), &opts)
            .expect("one-shot builds");
    assert!(
        warnings.is_empty(),
        "a clean map builds warning-free: {warnings:?}"
    );

    let bytes = builder.build().expect("WAD serializes");
    let wad = Wad::from_bytes(bytes).expect("built WAD parses");
    let ssectors_idx = wad
        .lumps()
        .iter()
        .position(|l| l.name() == "SSECTORS")
        .expect("SSECTORS present");
    let ssectors_bytes = wad.lump_bytes(ssectors_idx).expect("lump bytes present");
    assert_eq!(
        &ssectors_bytes[..4],
        b"ZGL3",
        "SSECTORS carries the ZGL3 stream signature"
    );

    let group = wad.map_group("MAP01").expect("map group present");
    let assembled = Map::assemble(&wad, &group).expect("strict assembly");
    assert!(
        !assembled.segs().is_empty(),
        "the doorway map yields at least one seg"
    );
}

// --- Proptest ----------------------------------------------------------------

/// Whether a `build_gl_nodes` error is one the plan permits (never a panic): the
/// shared narrowing/format errors plus the GL-specific structural guards. Mirrors
/// `build_lumps.rs`'s `is_plan_known_error`.
fn is_plan_known_error(err: &NodeBuildError) -> bool {
    matches!(
        err,
        NodeBuildError::EmptyGeometry
            | NodeBuildError::Write(_)
            | NodeBuildError::TooManyElements { .. }
            | NodeBuildError::MixedSectorSubsector { .. }
            | NodeBuildError::DegeneratePartition { .. }
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
/// to the three GL-tolerated classes (`Write | VanillaCeilingExceeded |
/// MixedSectorSubsector`), enforced inside
/// [`validate_gl_structure`]; every other warning, and every error, fails the
/// sweep. Geometric convexity is **not** asserted here: retail maps routinely
/// carry self-referencing and crossing lines, which yield valid-but-non-convex
/// GL subsector loops (see [`assert_subsectors_convex`]).
#[cfg(feature = "sweep-tests")]
#[test]
#[allow(clippy::too_many_lines)]
fn sweep_builds_gl_nodes_for_every_classic_map() {
    use crustywad::ParseOptions;
    use crustywad::map::{MapFormat, detect_map_format};

    let paths = common::wad_files("CRUSTYWAD_SWEEP_DIR");
    if paths.is_empty() {
        return; // skip note already printed by the helper
    }

    let mut maps_built = 0usize;
    let mut doom64_skipped = 0usize;
    let mut total_segs = 0usize;
    let mut total_minisegs = 0usize;
    let mut degenerate_subsectors = 0usize;

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

            // Informational: degenerate (≤2-distinct-vertex) subsectors are normal
            // GL output (closed by the connecting-miniseg path); count them only to
            // report their prevalence, not to gate anything.
            for ss in &built.subsectors {
                let mut vs: Vec<(i64, i64)> = built.segs[ss.segs.clone()]
                    .iter()
                    .flat_map(|s| [s.start, s.end])
                    .map(|r| gl_ref_fixed(&map, &built, r))
                    .collect();
                vs.sort_unstable();
                vs.dedup();
                if vs.len() < 3 {
                    degenerate_subsectors += 1;
                }
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
        "built GL nodes for {} WAD(s): {maps_built} classic map(s), {doom64_skipped} Doom 64 skipped, {total_segs} total GL segs ({total_minisegs} minisegs); {degenerate_subsectors} degenerate (<=2-vertex) subsector(s), all closed and validate-clean",
        paths.len()
    );
}

// --- add_udmf_map_with_nodes one-shot (#354) ---------------------------------

/// The UDMF one-shot emits the four-lump `MAP01`/`TEXTMAP`/`ZNODES`/`ENDMAP`
/// group, the `ZNODES` payload carries the auto-selected minimal GL dialect
/// (`XGLN` for this whole-unit map), and the group re-parses and re-assembles
/// strictly with the decoded node arenas matching what `build_gl_nodes` built.
#[test]
fn add_udmf_map_with_nodes_emits_a_reparsable_znodes_group() {
    let map = two_room_doorway_map();
    let mut builder = WadBuilder::new(WadKind::Pwad);
    // `NodeBuildOptions` is `#[non_exhaustive]`, so a struct-update expression is
    // unavailable from outside the crate — mutate a strict base instead.
    let mut opts = NodeBuildOptions::strict();
    opts.format = NodeFormat::Gl;
    let warnings =
        add_udmf_map_with_nodes(&mut builder, "MAP01", &map, &WriteOptions::strict(), &opts)
            .expect("one-shot succeeds");
    assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
    let bytes = builder.build().expect("build");
    let wad = Wad::from_bytes_with_options(bytes, ParseOptions::strict()).expect("parse");
    let names: Vec<_> = wad.lumps().iter().map(crustywad::Lump::name).collect();
    assert_eq!(names, ["MAP01", "TEXTMAP", "ZNODES", "ENDMAP"]);
    // The ZNODES payload starts with a GL signature (gl auto starts at XGLN
    // and escalates only when needed; this small map needs no escalation).
    let znodes = wad.lump_bytes(2).expect("ZNODES lump present");
    assert_eq!(&znodes[..4], b"XGLN");
    // Round-trip: the group assembles strictly, and the decoded node arrays
    // match what build_gl_nodes produced for the same map.
    let group = wad.map_groups().into_iter().next().expect("one group");
    let assembled = Map::assemble_with_options(&wad, &group, ParseOptions::strict())
        .expect("assembles with ZNODES");
    let (direct, _) = build_gl_nodes(&map, &opts).expect("direct build");
    assert_eq!(assembled.segs().len(), direct.segs.len());
    assert_eq!(assembled.subsectors().len(), direct.subsectors.len());
}

/// The `Xnod` non-GL extended format emits a single `XNOD` stream in `ZNODES`:
/// the four-lump `MAP01`/`TEXTMAP`/`ZNODES`/`ENDMAP` group re-parses, the
/// `ZNODES` payload carries the `XNOD` signature, and strict re-assembly yields
/// seg/subsector arenas whose counts match a direct `build_nodes` run on the
/// same map.
#[test]
fn add_udmf_map_with_nodes_emits_xnod_znodes() {
    let map = two_room_doorway_map();
    let mut builder = WadBuilder::new(WadKind::Pwad);
    let mut opts = NodeBuildOptions::strict();
    opts.format = NodeFormat::Xnod;
    let warnings =
        add_udmf_map_with_nodes(&mut builder, "MAP01", &map, &WriteOptions::strict(), &opts)
            .expect("one-shot succeeds");
    assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
    let bytes = builder.build().expect("build");
    let wad = Wad::from_bytes_with_options(bytes, ParseOptions::strict()).expect("parse");
    let names: Vec<_> = wad.lumps().iter().map(crustywad::Lump::name).collect();
    assert_eq!(names, ["MAP01", "TEXTMAP", "ZNODES", "ENDMAP"]);
    // The ZNODES payload is a non-GL extended stream (XNOD), not a GL one.
    let znodes = wad.lump_bytes(2).expect("ZNODES lump present");
    assert_eq!(
        &znodes[..4],
        b"XNOD",
        "the non-GL extended stream carries the XNOD signature"
    );
    // Round-trip: the group assembles strictly, and the decoded seg/subsector
    // counts match what build_nodes produced for the same map.
    let group = wad.map_groups().into_iter().next().expect("one group");
    let assembled = Map::assemble_with_options(&wad, &group, ParseOptions::strict())
        .expect("assembles with ZNODES");
    let (direct, _) = build_nodes(&map, &opts).expect("direct build");
    assert_eq!(assembled.segs().len(), direct.segs.len());
    assert_eq!(assembled.subsectors().len(), direct.subsectors.len());
}

/// The `Znod` zlib-compressed non-GL twin lands the same stream in `ZNODES`
/// under the `ZNOD` signature (feature-gated), and the compressed group still
/// re-assembles strictly with a populated seg arena.
///
/// There is no cfg-off twin: without `extended-nodes-zlib` the `NodeFormat::Znod`
/// variant does not exist and `NodeFormat::compressed()` is always `false`, so
/// `NodeBuildError::CompressionUnavailable` is unreachable through this one-shot.
/// (That error's direct coverage lives in `nodes.rs`'s
/// `compressed_without_feature_errors` unit test, which calls
/// `to_extended_lump_bytes(.., true)` directly.) This mirrors the file's
/// single feature-gated `oneshot_emits_zgl3_in_ssectors`.
#[cfg(feature = "extended-nodes-zlib")]
#[test]
fn add_udmf_map_with_nodes_emits_znod_znodes() {
    let map = two_room_doorway_map();
    let mut builder = WadBuilder::new(WadKind::Pwad);
    let mut opts = NodeBuildOptions::strict();
    opts.format = NodeFormat::Znod;
    let warnings =
        add_udmf_map_with_nodes(&mut builder, "MAP01", &map, &WriteOptions::strict(), &opts)
            .expect("one-shot succeeds");
    assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
    let bytes = builder.build().expect("build");
    let wad = Wad::from_bytes_with_options(bytes, ParseOptions::strict()).expect("parse");
    let znodes_idx = wad
        .lumps()
        .iter()
        .position(|l| l.name() == "ZNODES")
        .expect("ZNODES present");
    let znodes = wad.lump_bytes(znodes_idx).expect("ZNODES bytes present");
    assert_eq!(
        &znodes[..4],
        b"ZNOD",
        "the compressed non-GL extended stream carries the ZNOD signature"
    );
    let group = wad.map_groups().into_iter().next().expect("one group");
    let assembled = Map::assemble_with_options(&wad, &group, ParseOptions::strict())
        .expect("assembles with compressed ZNODES");
    assert!(
        !assembled.segs().is_empty(),
        "the doorway map yields at least one seg"
    );
}

/// The `Classic` format has no UDMF representation — UDMF carries no classic
/// binary node lumps — so it is rejected with `UnsupportedNodeFormat` before any
/// lump is added, leaving the builder untouched (mirrors the Doom one-shot's
/// fail-fast ordering).
#[test]
fn add_udmf_map_with_nodes_rejects_classic_format() {
    let map = two_room_doorway_map();
    let mut builder = WadBuilder::new(WadKind::Pwad);
    let opts = NodeBuildOptions::strict(); // format defaults to Classic
    assert_eq!(opts.format, NodeFormat::Classic);
    let err = add_udmf_map_with_nodes(&mut builder, "MAP01", &map, &WriteOptions::strict(), &opts)
        .unwrap_err();
    assert!(matches!(err, NodeBuildError::UnsupportedNodeFormat { .. }));
    // The builder must be left unmodified on error (mirrors the Doom one-shot):
    // parsing its output yields an empty, lump-less WAD.
    let bytes = builder.build().expect("empty build");
    let wad = Wad::from_bytes_with_options(bytes, ParseOptions::strict()).expect("parse");
    assert!(wad.lumps().is_empty());
}

#[test]
fn udmf_write_error_recoverability_delegates_through_node_build_error() {
    use crustywad::map::MapFormat;
    use crustywad::map::udmf::UdmfWriteError;

    let recoverable = NodeBuildError::UdmfWrite {
        source: UdmfWriteError::EmptyNamespace,
    };
    assert!(recoverable.is_lenient_recoverable());
    let unrecoverable = NodeBuildError::UdmfWrite {
        source: UdmfWriteError::UnsupportedSourceFormat {
            format: MapFormat::Doom,
        },
    };
    assert!(!unrecoverable.is_lenient_recoverable());
}

#[test]
fn classic_rejection_precedes_udmf_write_errors() {
    // A map whose empty namespace would also fail `write_udmf` in strict
    // mode: the statically-checkable format rejection must win, so a
    // `Classic` call reports `UnsupportedNodeFormat` deterministically.
    let textmap = concat!(
        "namespace = \"\";\n",
        "vertex { x = 0; y = 0; }\n",
        "vertex { x = 128; y = 0; }\n",
        "vertex { x = 128; y = 128; }\n",
        "vertex { x = 0; y = 128; }\n",
        "sector { texturefloor = \"FLOOR4_8\"; textureceiling = \"CEIL3_5\"; }\n",
        "sidedef { sector = 0; texturemiddle = \"STARTAN3\"; }\n",
        "sidedef { sector = 0; texturemiddle = \"STARTAN3\"; }\n",
        "sidedef { sector = 0; texturemiddle = \"STARTAN3\"; }\n",
        "sidedef { sector = 0; texturemiddle = \"STARTAN3\"; }\n",
        "linedef { v1 = 0; v2 = 1; sidefront = 0; blocking = true; }\n",
        "linedef { v1 = 1; v2 = 2; sidefront = 1; blocking = true; }\n",
        "linedef { v1 = 2; v2 = 3; sidefront = 2; blocking = true; }\n",
        "linedef { v1 = 3; v2 = 0; sidefront = 3; blocking = true; }\n",
        "thing { x = 64; y = 64; type = 1; }\n",
    );
    let mut src = WadBuilder::new(WadKind::Pwad);
    src.add_lump("MAP01", b"");
    src.add_lump("TEXTMAP", textmap.as_bytes());
    src.add_lump("ENDMAP", b"");
    let bytes = src.build().expect("source build");
    let wad = Wad::from_bytes_with_options(bytes, ParseOptions::strict()).expect("parse");
    let group = wad.map_groups().into_iter().next().expect("one group");
    let map = Map::assemble(&wad, &group).expect("strict assembly");

    let mut builder = WadBuilder::new(WadKind::Pwad);
    let err = add_udmf_map_with_nodes(
        &mut builder,
        "MAP01",
        &map,
        &WriteOptions::strict(),
        &NodeBuildOptions::strict(), // format defaults to Classic
    )
    .unwrap_err();
    assert!(matches!(err, NodeBuildError::UnsupportedNodeFormat { .. }));

    // Non-vacuity: with a valid (GL) format the same map genuinely fails
    // UDMF serialization on the empty namespace.
    let mut gl_opts = NodeBuildOptions::strict();
    gl_opts.format = NodeFormat::Gl;
    let err = add_udmf_map_with_nodes(
        &mut builder,
        "MAP01",
        &map,
        &WriteOptions::strict(),
        &gl_opts,
    )
    .unwrap_err();
    assert!(matches!(err, NodeBuildError::UdmfWrite { .. }));
    // Builder untouched by both failures.
    let bytes = builder.build().expect("empty build");
    let wad = Wad::from_bytes_with_options(bytes, ParseOptions::strict()).expect("parse");
    assert!(wad.lumps().is_empty());
}
