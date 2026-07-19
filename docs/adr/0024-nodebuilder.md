# ADR-0024: The nodebuilder — clean-room BLOCKMAP, REJECT, and classic BSP generation

- **Status:** Accepted
- **Date:** 2026-07-19
- **Deciders:** @masriamir
- **Tracking issue:** https://github.com/masriamir/crustywad/issues/255

## Context and problem statement

`add_doom_map` (ADR-0019 §4) deliberately emits zero-length `SEGS`, `SSECTORS`,
`NODES`, `REJECT`, and `BLOCKMAP` with an always-on
`DoomWriteWarning::NodesNotBuilt`: the output is editor- and nodebuilder-ready,
**not engine-playable** — an external nodebuilder must process it first.
ADR-0019's own revisit condition anticipated this ADR: *"reopen … if a
nodebuilder lands, at which point the empty-node-lump decision (§4) and its
unconditional `NodesNotBuilt` warning become obsolete and the output can become
engine-playable."* Issue #199 tracks *reading* GL/extended node formats
(milestone `Extended nodes`); nothing tracked *building* nodes until #255.

This ADR answers #255's four questions: whether a nodebuilder is in scope at
all, how to stage it if so, what it honestly costs, and what prior art is
license-compatible. The evidence base is four research reports recorded as
comments on #255 (2026-07-19): a licensing/landscape survey, a zdbsp source
deep-dive, an engine-source consumption contract (Chocolate Doom / GZDoom /
original id source, all `path:line`-cited), and an empirical scan of the
551 classic-format maps in the local retail collection.

### What the research established

**Licensing: nothing is portable; clean-room is the only permissive path.**
Every existing nodebuilder is GPLv2/GPLv2+ — zdbsp, the classic `bsp`, glBSP,
AJBSP, ZenNode, ZokumBSP, ZDRay (each verified from its actual license file or
doomwiki, per the survey's verdict table) — or worse (DeePBSP is closed
freeware; the original id DoomBSP shipped under ambiguous 1994 terms that make
it unsafe even to study). GPL code cannot be copied or line-by-line translated
into this crate (MIT OR Apache-2.0). But the *algorithm* is documented
independently of any GPL code — doomwiki's Node/Node builder/Blockmap/Reject
prose, the Unofficial Doom Specs, the textbook BSP literature (Fuchs–Kedem–
Naylor 1980), and the on-disk formats themselves (facts and interfaces, not
copyrightable expression). Tunable constants observed in zdbsp (split cost 8,
axis-aligned preference 16, the 128-unit blockmap grid) are likewise facts.
**No Rust nodebuilder exists anywhere** — crates.io and GitHub searches found
only readers and renderers — so a native builder would be first-of-its-kind in
the ecosystem.

**The engine contract splits by tier, and the builder's value is entirely the
vanilla tier.** GZDoom-family ports detect all-empty SEGS/SSECTORS/NODES —
explicitly, as "the map has no nodes and the engine is supposed to build them"
(`maploader.cpp:3088`) — and run their internal ZDBSP-derived builder, plus
rebuild missing/oversized BLOCKMAP and REJECT. **crustywad's current empty-lump
output is therefore already playable on the ZDoom family.** Vanilla and
Chocolate Doom rebuild nothing: loaders copy indices with almost no validation
(negative or out-of-range indices walk off arrays), an empty `NODES` collapses
the whole map to subsector 0, empty node lumps together crash, a missing
`BLOCKMAP` means no collision, and offsets are read as *signed* 16-bit — the
hard 64 KiB blockmap ceiling. Two findings shape the staging directly: **an
all-zeros, correctly-sized REJECT is confirmed functionally correct** (the
early-out at Chocolate Doom `p_sight.c:359` simply never fires; every sight
check falls through to the full BSP trace), and **zdbsp itself never computes a
real REJECT** — its `--full-reject` option prints "unsupported" and its actual
reject builder sits unused outside the build. Zero-fill is not a shortcut; it
is what the reference tool ships.

**Empirically, the classic 16-bit tier covers everything real.** Across all
551 classic-format retail maps: the worst count on any axis is 11,330 segs
(Freedoom 2 MAP28) — roughly a third of the tightest format ceiling; every
non-empty `REJECT` is exactly `ceil(sectors²/8)` bytes (521/521; the only
variance in the wild is deliberate omission, in the Hexen remasters); the
largest `BLOCKMAP` is 43,592 bytes (~66% of the vanilla ceiling); segs run
~1.6× linedefs at the median (2.6× worst); split vertices add ~16% to the
vertex array; and `nodes == subsectors − 1` holds on 551/551 maps — the BSP is
universally a full binary tree, a reliable validation invariant. Extended-node
output exists to escape ceilings that **no retail classic map approaches**.

**Effort, calibrated against zdbsp's source.** The algorithmic heart of zdbsp
is ~2,900 LOC of C++ (the BSP recursion, splitter heuristic, seg splitting,
vertex dedup, extraction) plus a 442-line blockmap builder; support code (I/O,
CLI, UDMF, viewer) triples that. A correct-output classic-nodes Rust
reimplementation is estimated at **~2,000–3,000 LOC plus its testing story** —
the one genuinely hard component — with the difficulty concentrated in named
subtleties: the side-classification epsilon regime, near-endpoint split
penalties and vertex dedup, partner-seg lockstep splitting (both halves of a
two-sided line must split at the same vertex or walls develop holes), and
forcing subsectors to be single-sector. The blockmap builder is small and
self-contained (~250–400 LOC; a DDA line rasterizer plus offset dedup), and the
zero-fill reject is trivial (~50 LOC).

### Current code this ADR must not contradict (verified, not recalled)

- `map/doom/write.rs`: `write_doom_map` / `add_doom_map` / `DoomMapLumps`
  (five data lumps, `#[non_exhaustive]`) exist as ADR-0019 specified;
  `add_doom_map` emits the marker plus the canonical lump run with `SEGS`,
  `SSECTORS`, `NODES`, `REJECT`, `BLOCKMAP` all zero-length; the `Narrower`
  seeds its warning list with `DoomWriteWarning::NodesNotBuilt`
  unconditionally, so every `write_doom_map` call reports it. Both
  `DoomWriteError` and `DoomWriteWarning` are `#[non_exhaustive]`.
- `map/graph.rs`: the read side already models the full BSP domain —
  `MapSeg` (`start`/`end`/`angle: u16`/`linedef`/`direction: u16`/
  `offset: i32`), `MapSubsector` (validated seg `Range<usize>`), `MapNode`
  (partition line, two `[i32; 4]` bboxes, `NodeChild` children), `MapReject`
  (row-major LSB-first bit matrix, layout verified against Chocolate Doom),
  and `MapBlockmap` (128-unit grid; parse strips the conventional leading `0`
  word from each blocklist and shares storage for aliased lists).
  `Map::bsp_root()` is the *last* node, matching the engine's
  `numnodes - 1` root. `Map`'s arena fields are `pub(crate)` — "only assembly
  builds one directly" — so a builder cannot and should not mutate an
  assembled `Map`.
- `map/mod.rs` has no `build` module; no symbol named `build_nodes`,
  `build_blockmap`, `NodeBuildError`, or `BuiltNodes` exists anywhere in the
  crate. Everything in §2 below is genuinely new.
- `fuzz/fuzz_targets/` already covers the adjacent surfaces:
  `fuzz_write_doom_map` (the ADR-0019 write path) and
  `fuzz_parse_reject_blockmap` (the read side of the two simple lumps).

## Decision drivers

- **Engine-tier honesty.** The ZDoom family already plays our output; the gap
  is vanilla-faithful ports. A builder that doesn't satisfy the vanilla
  contract (the hard/soft/quality requirement list in research report 3) adds
  nothing over what exists today.
- **Licensing.** GPL prior art is study-only; every line must be clean-room,
  derived from GPL-free documentation and the engine contract, never from GPL
  source. Constants and on-disk formats are facts and are usable.
- **One data path (ADR-0015).** The builder consumes the assembled `Map` graph
  and produces the graph's own BSP types; its serialized output must
  round-trip through the existing typed readers. No second notion of a map.
- **The strictness contract (ADR-0003).** Strict/lenient must remain a single
  decision table read twice, extended — not forked — for build-specific
  failure modes.
- **Hardening (ADR-0016)** applies to the new surface: bounded allocation, no
  unbounded recursion, fuzz targets with the no-panic oracle, both modes
  non-panicking.
- **Ship separable value first.** BLOCKMAP and REJECT are small, independent,
  and load-bearing for vanilla correctness; the BSP pass is an order of
  magnitude harder. Staging must not gate the easy wins on the hard one.
- **Don't duplicate #199.** GL/extended node *reading* is the `Extended nodes`
  milestone; extended *generation* belongs there too when something needs it —
  no retail classic map does.

## Considered options

1. **Build natively: a clean-room, staged builder in the core crate** —
   BLOCKMAP + zero-fill REJECT first, then a classic (16-bit) BSP pass,
   integrated behind the existing `write` feature.
2. **Don't build; document external tools** — a guide recipe for running
   zdbsp/AJBSP over `add_doom_map` output, making "invoke an external
   nodebuilder" the permanent answer.
3. **Wrap external builders** — `cwad` spawns a user-installed zdbsp/AJBSP
   binary as a child process and re-reads its output.
4. **Port zdbsp to Rust** — translate the reference implementation directly.

## Decision outcome

Chosen option: **Option 1 — build natively, clean-room, in three stages.**
The licensing verdict eliminates option 4 outright. Options 2 and 3 leave the
library's write story ending mid-pipeline and are retained only as the interim
state (the guide already tells users to run an external nodebuilder) and as a
non-goal, respectively (§6).

### 1. Scope: the classic tier, clean-room, format-agnostic input

The milestone targets **classic 16-bit output only**: vanilla-layout `SEGS`,
`SSECTORS`, `NODES`, plus `BLOCKMAP` and `REJECT`. This covers 100% of the
retail corpus with wide margin and is precisely the tier where generated lumps
are load-bearing (vanilla ports). GL nodes and extended/compressed encodings
(XNOD/ZNOD and friends) are **out of scope** here and staged into the
`Extended nodes` milestone alongside their read side (#199) if and when a
consumer needs them — the known triggers are maps exceeding the classic
ceilings and GL-consuming renderers, neither of which retail content exercises.

**Clean-room policy.** Implementation derives from: the engine-source
consumption contract (research report 3 — reading *engine* source to learn
what a *consumer* requires is not derivation from a nodebuilder), doomwiki and
Unofficial Doom Specs prose, the textbook BSP algorithm, the on-disk formats,
and observed constants (split cost 8, axis-aligned preference 16, 128-unit
blockmap grid — facts). GPL nodebuilder *source* is study-only background for
the spike record and must not be open during implementation; no code, comment,
or identifier structure is to be transcribed from it. PR descriptions for the
implementation issues state this explicitly.

**Input is any assembled `Map`.** Seg/subsector/node geometry is identical
across Doom- and Hexen-format maps, and the builders read only the graph's
format-agnostic arenas (vertices, linedefs, sidedefs, sectors), so
`build_nodes`/`build_blockmap`/`build_reject` accept any `Map` — including a
Doom 64- or UDMF-sourced one — subject to the same narrowing rules as the
write path (§3). The one-shot `add_doom_map_with_nodes` (§4) is Doom-target
only, exactly like `add_doom_map`. Hexen polyobject-aware splitting (keeping
polyobject container subsectors intact) is deferred with the nonexistent Hexen
binary write target (ADR-0019 scope): Doom-format output has no polyobjects,
so the special-casing has no consumer yet.

### 2. The public surface: `map::build`, behind the existing `write` feature

A new directory module `map/build/` (`mod.rs`, `blockmap.rs`, `reject.rs`,
`nodes.rs`), gated behind the existing `write` feature — the builder exists to
make written maps playable, and it pulls **no new dependencies** (pure
computation; `ZNOD`-style compression would need a zlib dependency, one more
reason it stays out of scope). No new feature flag.

```rust
/// Options for the node/blockmap builders.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct NodeBuildOptions {
    /// Strict mode fails on any vanilla-unsafe output; lenient mode emits it
    /// with a warning where a modern port can still consume it (§5 table).
    pub strictness: Strictness,
    /// Splitter-selection weight: reward for leaving a seg unsplit
    /// (default `8`).
    pub split_cost: u32,
    /// Splitter-selection weight: preference divisor for axis-aligned
    /// partition lines (default `16`).
    pub aa_preference: u32,
}

impl Default for NodeBuildOptions { /* strict, 8, 16 */ }

/// The BSP arenas produced by [`build_nodes`], mirroring the read-side graph
/// types. Index domains: seg `start`/`end` index the map's vertices followed
/// by `split_vertices`; `subsectors[i].segs` ranges index `segs`; node
/// children index `nodes`/`subsectors`; the root is the **last** node,
/// matching `Map::bsp_root()`.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct BuiltNodes {
    /// Vertices created by seg splits, appended after the map's own arena.
    pub split_vertices: Vec<MapVertex>,
    /// The built segs.
    pub segs: Vec<MapSeg>,
    /// The built subsectors (every one non-empty and single-sector).
    pub subsectors: Vec<MapSubsector>,
    /// The built nodes; `nodes.len() == subsectors.len() - 1` unless the map
    /// is a single convex subsector (then `nodes` is empty).
    pub nodes: Vec<MapNode>,
}

pub fn build_nodes(map: &Map, opts: &NodeBuildOptions)
    -> Result<(BuiltNodes, Vec<NodeBuildWarning>), NodeBuildError>;

pub fn build_blockmap(map: &Map, opts: &NodeBuildOptions)
    -> Result<(MapBlockmap, Vec<NodeBuildWarning>), NodeBuildError>;

/// Builds the correctly-sized all-zeros REJECT (`ceil(sectors²/8)` bytes).
/// Infallible: an all-clear table is always functionally correct (Chocolate
/// Doom `p_sight.c` — the reject early-out simply never fires).
pub fn build_reject(map: &Map) -> MapReject;
```

Serialization closes the loop through the existing typed readers rather than
around them — each read-side type gains a `write`-gated serializer, so the
on-disk layout stays declared in one place per lump family:

```rust
impl MapBlockmap {
    /// Serializes to `BLOCKMAP` lump bytes (writes the conventional leading
    /// `0` delimiter word per blocklist and deduplicates identical lists).
    ///
    /// # Errors
    /// [`NodeBuildError::BlockmapOverflow`] when an offset would exceed the
    /// unsigned 16-bit word range (§5).
    pub fn to_lump_bytes(&self) -> Result<Vec<u8>, NodeBuildError>;
}
impl MapReject {
    /// Serializes to `REJECT` lump bytes (the stored table, verbatim).
    pub fn to_lump_bytes(&self) -> Vec<u8>;
}
impl BuiltNodes {
    /// Serializes to (`SEGS`, `SSECTORS`, `NODES`) lump byte triples, plus
    /// the split vertices as trailing `VERTEXES` records.
    /// # Errors — §5 ceilings.
    pub fn to_lump_bytes(&self) -> Result<BuiltNodeLumps, NodeBuildError>;
}

/// The serialized node-data lumps from [`BuiltNodes::to_lump_bytes`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct BuiltNodeLumps {
    /// Trailing `VERTEXES` records for the split vertices (appended to the
    /// map's own serialized vertex records by the caller).
    pub split_vertexes: Vec<u8>,
    /// The serialized `SEGS` lump.
    pub segs: Vec<u8>,
    /// The serialized `SSECTORS` lump.
    pub ssectors: Vec<u8>,
    /// The serialized `NODES` lump.
    pub nodes: Vec<u8>,
}
```

`BuiltNodes` intentionally does **not** mutate or extend the input `Map` —
`Map`'s arenas stay `pub(crate)` and assembly-only. Round-tripping is instead
proven at the WAD level: write a map with built lumps, re-read it, assemble,
and the assembled graph's BSP arenas must equal the built ones (§7).

### 3. Builders run on the narrowed integer geometry

The graph stores `f64` coordinates; the engine consumes `i16` vertices. A
builder that partitioned the `f64` geometry and then rounded would let a seg
land on the wrong side of its partition line after rounding. The builders
therefore **narrow first, build second**: the same narrowing pass as
`write_doom_map` (ADR-0019 §3's three-tier table — non-finite, fractional,
out-of-range coordinates) produces the `i16` vertex table, and all BSP,
blockmap, and bbox computation runs in integer/fixed-point space on exactly
the values the engine will read. Narrowing failures surface as
`NodeBuildError::Write(DoomWriteError)` and narrowing recoveries as
`NodeBuildWarning::Write(DoomWriteWarning)` — the write path's decision table,
reused, not restated. Split vertices are computed in 16.16 fixed-point and
round to integer classic vertices exactly once, at creation.

### 4. `add_doom_map_with_nodes`: the playable one-shot

```rust
/// Serializes `map` into `builder` as a complete, engine-playable Doom map:
/// the marker, the five data lumps, and **built** `SEGS`/`SSECTORS`/`NODES`/
/// `REJECT`/`BLOCKMAP` in canonical order.
///
/// Unlike [`add_doom_map`], the returned warnings never include
/// [`DoomWriteWarning::NodesNotBuilt`].
///
/// # Errors
/// Everything [`write_doom_map`] rejects (via
/// [`NodeBuildError::Write`]), plus the build-specific failures of §5.
pub fn add_doom_map_with_nodes(
    builder: &mut WadBuilder,
    name: &str,
    map: &Map,
    write_opts: &WriteOptions,
    build_opts: &NodeBuildOptions,
) -> Result<Vec<NodeBuildWarning>, NodeBuildError>;
```

`add_doom_map` itself is **unchanged**: its editor-target contract (empty node
lumps, unconditional `NodesNotBuilt`) remains exactly as ADR-0019 §4 decided,
for callers who *want* a nodebuilder-ready skeleton. This fulfills ADR-0019's
revisit condition by adding the playable path beside the existing one rather
than amending it; `DoomWriteError` and `DoomWriteWarning` gain no variants.
The `NodesNotBuilt` warning that `write_doom_map` seeds internally is filtered
by the with-nodes path (it is not a property of *this* output).

### 5. The strictness decision table, extended

Build failures follow the house pattern — one table, two readings. The
ceilings come straight from the engine contract: vanilla reads index fields
through a *signed* 16-bit cast (safe max index 32,767), while the format
itself is unsigned 16-bit (max 65,535) and Boom-heritage ports read it that
way; subsector and node child references reserve bit 15 as the leaf flag in
**both** readings, so their ceiling is structural.

```rust
#[non_exhaustive]
pub enum NodeBuildError {
    /// A narrowing failure from the shared write-path pass (§3).
    Write(DoomWriteError),
    /// The map has no linedefs (or no vertices/sidedefs/sectors): there is no
    /// geometry to partition, and vanilla requires at least one subsector.
    /// Both modes.
    EmptyGeometry,
    /// A built arena exceeds its output ceiling (table below).
    TooManyElements { kind: &'static str, count: usize, max: usize },
    /// A blocklist offset in the packed `BLOCKMAP` exceeds the applicable
    /// offset ceiling (table below).
    BlockmapOverflow { offset: usize },
}

#[non_exhaustive]
pub enum NodeBuildWarning {
    /// A narrowing recovery from the shared write-path pass (§3).
    Write(DoomWriteWarning),
    /// Lenient mode: a built arena exceeds the vanilla-safe ceiling but fits
    /// the format ceiling; the output was emitted and needs a limit-removing
    /// port.
    VanillaCeilingExceeded { kind: &'static str, count: usize, max: usize },
    /// Lenient mode: the packed `BLOCKMAP` exceeds the vanilla signed-offset
    /// ceiling but fits unsigned 16-bit offsets; emitted, needs a
    /// limit-removing port.
    BlockmapVanillaOverflow { offset: usize },
}
```

| Condition | Strict | Lenient |
|---|---|---|
| built vertices (map + split) > 32,768 · segs > 32,768 | `TooManyElements` | emit, `VanillaCeilingExceeded` |
| built vertices > 65,536 · segs > 65,536 | `TooManyElements` | `TooManyElements` (unencodable) |
| subsectors > 32,768 · nodes > 32,768 | `TooManyElements` | `TooManyElements` (bit 15 is the leaf flag — structural) |
| any `BLOCKMAP` blocklist offset > 32,767 | `BlockmapOverflow` | emit, `BlockmapVanillaOverflow` |
| any `BLOCKMAP` blocklist offset > 65,535 | `BlockmapOverflow` | `BlockmapOverflow` (the offset word is unsigned 16-bit at most) |
| zero linedefs/vertices/sidedefs/sectors | `EmptyGeometry` | `EmptyGeometry` |

No retail classic map reaches a third of the tightest of these ceilings; the
strict/lenient split exists for synthetic and converted content, and the
unconditional rows are the write path's tier-1 "structurally impossible"
precedent applied to the build outputs.

**Output conventions** (the soft/quality half of the engine contract): seg
`angle` is the BAM of `v1→v2`; seg `offset` is the distance along the linedef
from its start vertex to the seg's start; each blocklist is emitted with the
leading `0` delimiter word (vanilla tolerates it, GZDoom's blockmap verifier
*requires* it) and a `0xFFFF` terminator; identical blocklists are
deduplicated (the read side already models aliased lists); every linedef
appears in every block its geometry crosses (the hard collision requirement);
node bboxes bound their subtrees; subsectors are convex, non-empty, and
single-sector, with their first seg carrying the sidedef that determines the
subsector's sector; `nodes.len() == subsectors.len() − 1` (validated — it held
on 551/551 retail maps); and building is **deterministic** — identical input
and options produce identical bytes, with no iteration-order-sensitive
containers on any output path.

### 6. What this ADR does *not* build

- **A real (line-of-sight) REJECT.** Zero-fill is engine-correct and is what
  zdbsp ships; a true visibility computation is a separate, sizable project
  (RMB territory) with no current consumer. Revisit on demand.
- **GL nodes and extended/compressed encodings (XNOD/ZNOD, GL variants).**
  `Extended nodes` milestone, with #199's read side. The classic ceilings in
  §5 are the trigger: content that legitimately exceeds them needs XNOD
  output, and none exists in the retail corpus.
- **External-tool wrapping.** `cwad` will not spawn zdbsp/AJBSP. The guide's
  existing "run an external nodebuilder" note remains accurate for GL/extended
  needs until that milestone.
- **Hexen polyobject-aware splitting** (§1) and a Hexen binary write target
  (ADR-0019, unchanged).
- **A `Map`-mutating rebuild API.** Assembly remains the only constructor of
  `Map`; rebuilt-in-place graphs would create a second provenance for BSP
  arenas.

### 7. Testing story

- **Round-trip through the readers (the ADR-0015 contract).** Unit level:
  `MapBlockmap::parse(built.to_lump_bytes()) == built` and likewise for
  `MapReject`. WAD level: `add_doom_map_with_nodes` → `WadBuilder::build` →
  `Wad::parse` → `Map::assemble` in **strict** mode with zero warnings, and
  the assembled BSP arenas equal the built ones.
- **Self-validating retail sweep.** For every classic map in the local retail
  collection (gated like the existing sweep, `CRUSTYWAD_SWEEP_DIR`): rebuild
  nodes/blockmap/reject from the assembled graph, then assert the engine
  contract mechanically — every index in range, tree acyclic and rooted last,
  every subsector non-empty/single-sector, every seg geometrically on the
  correct side of every ancestor partition line, every linedef present in
  every blockmap cell its geometry crosses (verified by independent
  brute-force rasterization), `nodes == subsectors − 1`. This oracle needs no
  external builder and no golden files.
- **Calibration corridor.** The sweep additionally asserts the build stays
  inside the empirically established envelope (segs ≤ ~3× linedefs, split
  vertices a bounded fraction of the arena) — a tripwire for pathological
  splitting regressions, not a byte-comparison against any reference builder
  (matching another builder's exact output is a non-goal).
- **Property tests** (ADR-0010): generated small convex/concave polygon maps
  build, round-trip, and satisfy the contract invariants.
- **Fuzz (ADR-0016 item 3):** a `fuzz_build_nodes` target — arbitrary WAD
  bytes → lenient assemble → `build_nodes` + `build_blockmap` +
  `build_reject` → no panic — with a committed seed corpus (seeds must not
  begin with 8 hex characters, per the corpus-glob rule) and `fuzz.yml`
  wiring.

### 8. Hardening (ADR-0016), stated per the checklist

1. **Bounded allocation.** The §5 ceilings double as allocation bounds: seg,
   subsector, node, and split-vertex arenas are capped at 65,536/32,768
   elements *by the failure table itself*, so build memory is O(1)-bounded
   regardless of input pathology. The blockmap grid is bounded by the `i16`
   coordinate domain (at most 512×512 cells). The one honest exception:
   `REJECT` is Θ(sectors²/8) **by format definition** — `ceil(65,536²/8)` =
   512 MiB at the write path's tier-1 sector ceiling — and the fuzz harness
   therefore asserts the documented `sectors²/8` bound rather than a linear
   one. This deviation is inherent to the lump, not to the implementation.
2. **No unbounded recursion.** The BSP pass uses an explicit work stack, not
   call recursion; the seg ceiling is simultaneously the termination backstop
   (any runaway splitting fails cleanly as `TooManyElements` rather than
   looping or overflowing the stack).
3. **Fuzz target** — §7.
4. **Both modes non-panicking** — the §5 table is total over the failure
   space; `#![deny(unsafe_code)]` holds.

### 9. Staging — the implementation issues

Filed after this ADR merges, all in milestone `Nodebuilder`, dependency-ordered:

1. **Stage 1 — BLOCKMAP and REJECT builders.** `map/build/` module skeleton,
   `NodeBuildOptions`/`NodeBuildError`/`NodeBuildWarning` (the §5 subset that
   applies), `build_blockmap` + `MapBlockmap::to_lump_bytes`, `build_reject` +
   `MapReject::to_lump_bytes`, round-trip and retail-sweep coverage for both,
   fuzz coverage. Small, self-contained, ships the two lumps whose absence
   breaks vanilla *collision and sight* even on maps that already have nodes.
2. **Stage 2 — the classic BSP pass.** `build_nodes`, `BuiltNodes`,
   `BuiltNodeLumps`, the narrowed-geometry kernel (§3), the full §5 table, the
   self-validating retail sweep and calibration corridor, property tests,
   `fuzz_build_nodes`. The milestone's center of gravity — a plan-reviewed,
   SDD-executed algorithmic component.
3. **Stage 3 — the playable one-shot and the CLI.** `add_doom_map_with_nodes`
   (§4), a `--nodes` flag on `cwad convert` and `cwad build` (making their
   Doom-format output engine-playable; zdbsp's option surface is the
   documented prior art for any future tunable flags), the guide page
   ("Building nodes"), and benchmark coverage in `write_ops.rs`.

Stage 1 has no dependency on stage 2; stage 3 depends on both.

## Consequences

- **New public API** (all behind the existing `write` feature): `map::build`
  — `NodeBuildOptions`, `BuiltNodes`, `BuiltNodeLumps`, `NodeBuildError`,
  `NodeBuildWarning`, `build_nodes`, `build_blockmap`, `build_reject`,
  `add_doom_map_with_nodes` — plus `write`-gated `to_lump_bytes` serializers
  on `MapBlockmap` and `MapReject`. No new feature flag, no new dependencies,
  no changes to any existing signature, error, or warning enum.
- **crustywad's write story becomes end-to-end** for the classic tier: read →
  `Map` → write → *playable on vanilla*, with the strictness contract
  answering "does this fit vanilla?" the same way it already answers "does
  this fit Doom format?".
- **First nodebuilder in the Rust ecosystem**, and the first permissively
  licensed one anywhere in the Doom toolchain landscape the survey could find.
- **Honest cost:** stage 2 is the largest single algorithmic component in the
  crate to date (~2,000–3,000 LOC plus its oracle tests), with subtle
  epsilon/geometry behavior that unit tests alone cannot certify — hence the
  self-validating sweep as the primary correctness instrument. Stages 1 and 3
  are small.
- **ADR-0019 §4 is fulfilled, not amended:** `add_doom_map` keeps its
  editor-target contract and its warning; the playable path is a parallel
  entry point. ADR-0019's revisit condition is discharged.
- **The `Extended nodes` milestone inherits a clean seam:** §5's ceilings are
  the exact trigger conditions for XNOD output, and `BuiltNodes` (typed,
  index-based, format-agnostic) is the natural input to a future extended
  serializer.
- **Risk, named:** clean-room BSP construction is the kind of work where
  plausible-looking output can be subtly wrong (a seg on the wrong side of a
  partition renders correctly from most viewpoints). The mitigation is
  structural: the mechanical engine-contract oracle over 551 real maps, plus
  determinism, plus property tests — not visual inspection.

## Pros and cons of the options

### Option 1 — build natively, clean-room, staged (chosen)

- Good, because it closes the write pipeline's last gap with a genuinely
  novel, license-clean component, on the strength of a documented algorithm
  and a mechanical correctness oracle.
- Good, because staging ships the small load-bearing lumps (BLOCKMAP/REJECT)
  without waiting on the hard one.
- Good, because the builders reuse the graph, the narrowing pass, the
  strictness table, and the typed readers — no parallel data path.
- Bad, because stage 2 is a multi-thousand-line algorithmic effort with real
  schedule risk; the estimate is calibrated against the reference
  implementation's measured size, not optimism.

### Option 2 — don't build; document external tools

- Good, because it costs nothing and the external tools are battle-tested.
- Bad, because it is the status quo: crustywad output stays unplayable on
  vanilla without a GPL binary the user must find, build, and trust, outside
  our test matrix — and the ecosystem gap stays open.
- Bad, because "the library that reads and validates BSP but cannot produce
  it" is an asymmetry with no principled boundary — the read side already
  models every structure the builder needs to emit.

### Option 3 — wrap external builders from `cwad`

- Good, because it would make `cwad convert` output playable quickly.
- Bad, because it is a CLI-only answer (the library API gains nothing), adds
  a runtime process dependency with platform/packaging burden, and couples our
  UX to tools we cannot version or test. Distribution of GPL binaries
  alongside an MIT/Apache tool invites exactly the licensing ambiguity the
  survey was commissioned to avoid.

### Option 4 — port zdbsp

- Good, because zdbsp's behavior is the de-facto standard.
- Bad, because it is legally foreclosed: GPLv2+ code cannot be relicensed
  MIT OR Apache-2.0 by translation. Eliminated.

## More information

- **Research record:** four reports posted as comments on #255 (2026-07-19) —
  prior-art/licensing survey, zdbsp deep-dive, engine consumption contract,
  retail empirical scan. Every factual claim above about engine behavior,
  licenses, zdbsp internals, or retail statistics traces to one of them.
- **Related ADRs:** ADR-0003 (strictness contract), ADR-0006 (write design),
  ADR-0015 (the graph the builder consumes and round-trips through), ADR-0016
  (hardening checklist, applied in §8), ADR-0019 (the write path this
  completes; its §4 revisit condition is discharged by §4 here).
- **Related issues:** #255 (this spike), #199 / milestone `Extended nodes`
  (GL/extended reading; extended generation staged there), the three staging
  issues of §9 (filed post-merge).
- **Revisit conditions:** reopen when (a) extended/GL *generation* gains a
  concrete consumer (UDMF-scale maps exceeding §5 ceilings, GL-node-consuming
  renderers) — that work lands in `Extended nodes` with this ADR's `BuiltNodes`
  as input; (b) a real line-of-sight REJECT gains a consumer; or (c) a Hexen
  binary write target lands (ADR-0019's own revisit condition), which would
  motivate polyobject-aware splitting.
