# ADR-0025: Extended and GL node-format reading

- **Status:** Accepted
- **Date:** 2026-07-20
- **Deciders:** @masriamir
- **Tracking issue:** https://github.com/masriamir/crustywad/issues/199

## Context and problem statement

crustywad reads the **classic** BSP node encoding — the 28-byte `NODES`
records with `u16` children, plus classic `SEGS`/`SSECTORS`
(`map/common.rs`, assembled by `normalize_bsp` in `map/assemble.rs`). Modern
WADs routinely ship node data the classic model cannot represent: ZDoom
**extended/compressed** nodes (32-bit indices packed into the `NODES` or
`SSECTORS` lump), classic **GL** nodes (`GL_*` lumps), and **DeePBSP v4**
(`xNd4`, a 32-bit-widened classic layout). ADR-0015's revisit condition
anticipated this: *"reopen … when a new node format (GL nodes / extended ZDBSP
nodes, tracked in #199) needs representation beyond the classic `NODES`
encoding."*

Today those formats are **detected and gated, not parsed**. The
#204 amendment to ADR-0015 added the `EXTENDED_NODE_SIGNATURES` table
(`map/assemble.rs`, exactly `XNOD ZNOD XGLN ZGLN XGL2 XGL3 ZGL2 ZGL3`) and a
gate (in `Map::assemble_with_options`, same file): a `NODES`/`SSECTORS` lump
whose first four bytes match a signature returns
`MapAssembleError::UnsupportedNodeEncoding`
in strict mode, or (lenient) pushes `MapWarning::UnsupportedNodeEncoding` and
empties the three BSP arenas. Both variants cite #199 as the future reading
path. This ADR decides that path.

The design is grounded in three source-verified research passes recorded on
#199 (2026-07-20): a format taxonomy from the zdbsp writers and GZDoom
loaders, a survey of engine consumption and de facto usage across the modern
ports, and a crustywad-state audit of the read model and its seams.

### What the research established

**Fourteen format names collapse to a few readers.**

- **Compression is orthogonal.** Every `Z*` tag is its `X*` twin wrapped in a
  raw zlib stream after the 4-byte tag; GZDoom decodes both to the same
  internal `type` and calls one routine (`maploader.cpp:692-761`). One inflate
  front-end + four layout readers cover all eight ZDoom formats.
- **The ZDoom family shares a framing.** XNOD/XGLN/XGL2/XGL3 all begin with
  the same vertex header (`u32 origVerts`, `u32 newVerts`, then `newVerts`
  16.16 fixed-point split vertices) and the same subsector block
  (`u32 count`, then `u32 numsegs` per subsector). Only the **seg record** and
  **node record** differ: XGLN uses a `u16` linedef, XGL2/XGL3 a `u32`
  linedef, and XGL3 alone widens the node partition coordinates from `int16`
  to `int32` (fractional splitters). XNOD is the non-GL variant (explicit
  `v1`+`v2` segs, no partner segs). A single `type`-parameterized parser with
  pluggable seg/node decoders covers the family.
- **DeePBSP v4 is a *classic-widened* format, not a ZDoom variant.** It keeps
  the classic seg semantics (`angle`/`offset`, no partner), uses an 8-byte
  `xNd4\0\0\0\0` signature (NODES lump only), and spreads 32-bit records across
  three separate lumps. It needs its own small reader, structurally close to
  the classic path (a classic reader parameterized on child width and leaf-flag
  bit covers vanilla, GL v2/v3/v5, and DeeP v4 nodes).
- **Classic GL (`gNd2`…`gNd5`) is deprecated.** Only the ZDoom line still reads
  it; DSDA-Doom, Woof, Eternity, and Crispy dropped or never added it, and it
  is fully superseded by the XGL family (which lives in standard lumps).

**XGL3 is the format that matters.** Ultimate Doom Builder's default UDMF
nodebuilder profile ("ZDBSP – UDMF Normal") emits the uncompressed XGL family —
XGLN, or XGL2 past 65534 lines, or **XGL3** when a map needs sub-integer
splitter precision (common for UDMF). UDMF maps ship XGL3/ZGL3 or nothing,
because ports *reject classic nodes for text maps*. So the modern common case
is XGL3.

**No port *requires* shipped extended/GL nodes** — every modern port (GZDoom
included) rebuilds internally when they are absent or fail to load. The value
of *reading* them in a library is therefore **analysis/tooling, round-trip
fidelity, and the future editor (#18)** — not feeding a port. Covering the one
format modern tooling emits beats chasing completeness.

**The read model is already prepared** (verified, not recalled):

- The index newtypes are `usize`-backed (`graph.rs:27-57`:
  `VertexIdx`…`NodeIdx`), which ADR-0015 §data-model states was deliberate so
  "extended formats … are not capped at `u16`" (`0015…md:144-145`). The wider
  32-bit indices fit without a graph change.
- `MapSeg`/`MapSubsector`/`MapNode`/`NodeChild` (`graph.rs:285-382`) already
  model the domain; `MapNode` stores `i32` partition coords and bboxes, so
  XGL3's fractional splitters (rounded, or later widened) and the classic
  16-bit ones both fit.
- The nodebuilder's `BuiltNodes` (`map/build/nodes.rs:70-88`) uses the exact
  same arena types, so this read work and a future extended-node **writer**
  (ADR-0024's own revisit condition) reuse one type set; only the codec
  differs.
- The `RETAIL-EXT` gate-contract sweep (`tests/sweep.rs:113-179`,
  `CRUSTYWAD_SWEEP_EXTENDED_DIR`, #269) holds zdbsp-derived XNOD/ZNOD Freedoom
  fixtures and currently asserts they error strictly / empty leniently. Its own
  module doc says it "becomes [the] positive-read fixture set" when #199 lands.

## Decision drivers

- **Cover the real common case first.** XGL3/ZGL3 is what the standard
  UDMF toolchain emits; leading with it delivers the modern case with the least
  code, via the shared `type`-parameterized reader.
- **One data path (ADR-0015).** Extended nodes decode into the *same*
  `MapSeg`/`MapSubsector`/`MapNode` arenas as classic nodes. A consumer of a
  `Map` never learns which codec produced the BSP — there is one graph.
- **Incremental narrowing of the gate.** The existing gate is the seam. Each
  stage moves signatures from "gated (error/empty)" to "decoded" without
  changing the error/warning surface for the ones still unsupported.
- **Keep the default build dependency-free.** Uncompressed formats need no new
  dependency; the zlib decompressor for compressed formats sits behind a
  feature, keeping the default build zlib-free.
- **Hardening (ADR-0016).** Extended-node parsing is a new parse surface:
  bounded allocation, no unbounded recursion, fuzz targets, both `Strictness`
  modes non-panicking. Compression adds an unbounded-*output* risk that a
  decoded-size cap must bound.
- **Read now, write later.** #199 is the read side. A writer that emits
  extended nodes from a `BuiltNodes` (ADR-0024's revisit) is out of scope here
  and staged separately, to avoid over-scoping the read work.

## Considered options

1. **Full breadth immediately** — read every format (ZDoom X/Z, classic GL
   v1–v5, DeePBSP v4) in one push.
2. **Staged ZDoom-first, then DeeP; skip classic GL** — one
   `type`-parameterized ZDoom reader leading with XGL3, uncompressed before
   compressed, a separate DeePBSP v4 reader, and no classic-GL support.
3. **Wrap/shell-out or document external tools** — punt reading to an external
   converter.

## Decision outcome

Chosen option: **Option 2 — staged, ZDoom-first, skip classic GL.**

### 1. Scope: the ZDoom extended family, plus DeePBSP v4; classic GL is out

crustywad will read, decoding into the classic BSP arenas:

- **ZDoom extended-regular:** `XNOD`, `ZNOD`.
- **ZDoom extended-GL:** `XGLN`, `XGL2`, `XGL3`, and their compressed twins
  `ZGLN`, `ZGL2`, `ZGL3`.
- **DeePBSP v4:** `xNd4` (Stage 3).

**Classic GL nodes (`gNd2`…`gNd5`, the `GL_*` lumps) are out of scope for the
staged #199 work** — deprecated, read only by the ZDoom line, and fully
superseded by the XGL family. `gNd*`/`GL_*` are not even detected today; they
remain undetected here (a `GL_*` lump is an ordinary non-map lump). They are
**not, however, abandoned:** high compatibility with older WADs is a project
value, so classic-GL reading is tracked as a backlog item, **#324**, to be
implemented after the #199 read stages (it reuses this ADR's reader
scaffolding — a classic-node reader parameterized on child width and leaf-flag
bit already covers vanilla, GL v2/v3/v5, and DeeP v4 nodes). The `gNd*`
detection and the v1/v4 read-or-reject policy are settled there, not here.

### 2. The reader: one `type`-parameterized ZDoom parser + a DeeP reader

A new module `map/nodes/` (or `map/extended.rs`; the implementation picks the
layout) holds the codecs. It is **read-side core** — map assembly is always-on,
so decoding uncompressed extended nodes is unconditional, like classic
decoding. The structure mirrors the research's shared-layout finding:

- **A `type`-parameterized ZDoom-extended parser.** After the 4-byte tag, the
  parser reads the shared vertex header and subsector block once, then
  dispatches the seg record and node record on a `type` derived from the
  signature: `XNOD`→regular (explicit v1/v2 segs, `u16` linedef, `int16`
  nodes), `XGLN`→GL with `u16` linedef and `int16` nodes, `XGL2`→GL with `u32`
  linedef and `int16` nodes, `XGL3`→GL with `u32` linedef and `int32`
  (fractional) nodes. The GL variants carry partner segs and implicit `v2`
  (the next seg's `v1` within the subsector, wrapping at the subsector end),
  which the parser materializes into explicit `MapSeg` endpoints.
- **An inflate front-end.** A `Z*` tag inflates the zlib stream after the tag,
  then hands the plaintext to the exact same `X*` parser (`ZNOD`≡`XNOD`, etc.).
- **A separate DeePBSP v4 reader.** `xNd4` is decoded by a classic-shaped
  reader parameterized on 32-bit child width and the bit-31 leaf flag, reading
  its widened `SEGS`/`SSECTORS`/`NODES` records from the three lumps.

All three produce the crate's existing `MapSeg`/`MapSubsector`/`MapNode`
arenas. The `NF_SUBSECTOR` leaf flag (bit 15 classic, bit 31 extended) is
decoded once into `NodeChild`, exactly as the classic path already does
(`resolve_node_child`). Split/GL vertices from the extended stream are appended
to the map's vertex arena (the extended header stores only the *new* vertices;
indices below `origVerts` refer to the existing `VERTEXES` lump) — the mirror
of how the nodebuilder's `split_vertices` extend `VERTEXES` on write.

### 3. Gate integration: narrow the seam per stage

The gate in `Map::assemble_with_options` (`map/assemble.rs`) becomes a
dispatch:

- A signature this build **can decode** is parsed into the BSP arenas.
- A signature it **cannot yet decode** keeps the current contract — strict
  `MapAssembleError::UnsupportedNodeEncoding { lump, signature }`, lenient
  `MapWarning::UnsupportedNodeEncoding` + empty arenas.

So after Stage 1 the four `X*` signatures decode while the four `Z*` still
gate (until Stage 2). `xNd4` was expected here to join `EXTENDED_NODE_SIGNATURES`
as an 8-byte signature when Stage 3 lands — but the **Stage 3 amendment below
revised this**: `xNd4` is detected by a *separate* 8-byte check (not added to the
4-byte `EXTENDED_NODE_SIGNATURES` table), and before Stage 3 an `xNd4` lump is
undetected and falls through to the classic decoder rather than gating. `gNd*` is
never added. The strictness contract is unchanged for everything still gated:
this is incremental narrowing, not a new policy.

### 4. UDMF `ZNODES` lump routing

For UDMF maps the ZDoom-GL family lives in a lump literally named `ZNODES`
(placed after `TEXTMAP`), not in `NODES`/`SSECTORS`. The current gate fires only
on the binary path (`NODES`/`SSECTORS`), because UDMF and Doom 64 are routed
away before the gate (`map/assemble.rs`). The UDMF read path therefore gains
`ZNODES` handling: detect and decode it with the same `type`-parameterized
parser, feeding the same arenas the binary path produces. A UDMF map with a
`ZNODES` lump this build cannot decode gates with the same strict/lenient
contract, surfaced on the UDMF path.

### 5. Compression is a feature; inflation is bounded (ADR-0016)

Uncompressed `X*`/`xNd4` reading is **always-on core** (no dependency).
Decoding the compressed `Z*` family requires a zlib inflater; a
`flate2`/`miniz_oxide` stack is already vendored transitively via
`png`/`doom64-gfx`, so Stage 2 lives behind a feature (working name
`extended-nodes-zlib`, or it may reuse an existing feature) that pulls the
decompressor, keeping the default build zlib-free.

zlib inflation is the one **unbounded-output** risk on this surface (a small
compressed lump can inflate to an arbitrarily large stream). Following the
`doom64-gfx` precedent (`Limits::max_decoded_pixels`), `Limits` gains a
**decoded-node-stream cap** (e.g. `max_decoded_node_bytes`) that bounds the
inflate output; exceeding it is a strict error / lenient recovery
(gate-to-empty), never an OOM. The uncompressed path is already `O(input)`:
record counts come from the stream's own count fields and are bounded by the
lump length divided by the minimum record size.

### 6. Read now; a writer is a separate follow-up

This ADR delivers **reading** only. ADR-0024 designed `BuiltNodes` to feed a
future extended-node *writer* (its revisit condition); that writer — emitting
`XGL3`/`ZGL3` from a `BuiltNodes` so `add_doom_map_with_nodes` output can be
UDMF-scale or fractional — reuses these codecs in reverse. It is tracked as
**#323** (depends on #199) and gets its own short ADR amendment before
implementation. Keeping write out of #199 avoids coupling the read surface to a
second, larger design.

## Staging — the implementation issues (filed from this ADR after merge)

1. **Stage 1 — uncompressed ZDoom extended, no new dependency.** The
   `type`-parameterized parser for `XNOD`/`XGLN`/`XGL2`/`XGL3`; gate dispatch so
   these decode while `Z*`/`xNd4` still gate; UDMF `ZNODES` routing for the
   uncompressed GL variants; flip the `RETAIL-EXT` sweep's XNOD fixtures from
   gate-contract to positive-read; a `fuzz_extended_nodes` target; both modes
   non-panicking. This is the milestone's center of gravity — it unlocks the
   modern XGL3 common case.
2. **Stage 2 — compressed.** `ZNOD`/`ZGLN`/`ZGL2`/`ZGL3` behind the zlib
   feature; `Limits` decoded-size cap (ADR-0016 §1); inflate front-end reusing
   the Stage-1 parser; flip the ZNOD sweep fixtures; extend the fuzz target
   with compressed seeds.
3. **Stage 3 — DeePBSP v4.** The separate `xNd4` reader; 8-byte-signature
   detection; its own small fixture. Low traffic, old-WAD compatibility.

Classic GL (`gNd*`) remains out of scope.

## Consequences

- **New public surface** is small: extended nodes decode into the *existing*
  `Map` arenas, so `map.segs()/subsectors()/nodes()/bsp_root()` gain no new
  types — a `Map` assembled from an XGL3 WAD looks like any other. New items
  are limited to the `Limits` cap field (Stage 2) and possibly a feature flag;
  `MapWarning`/`MapAssembleError` may gain variants (additive, `#[non_exhaustive]`).
- **The gate's meaning shifts** from "extended nodes are unsupported" to
  "extended nodes this build cannot decode are unsupported" — a strictly
  smaller set each stage. The strict/lenient contract for still-gated
  signatures is unchanged.
- **The `RETAIL-EXT` sweep inverts** from a gate-contract test to a
  positive-read fixture set (per its own design note), stage by stage.
- **ADR-0015 is amended again** (its revisit condition discharged), and
  ADR-0016's checklist applies to the new parse surface. ADR-0024's
  `BuiltNodes` becomes the shared type basis for the deferred writer (#323).
- **Default builds stay zlib-free**; only the compressed stage pulls a
  decompressor, behind a feature.
- **Good** — the modern common case (UDMF + XGL3) becomes analyzable,
  round-trippable, and available to the future editor, with no external tool.
- **Bad** — classic GL WADs remain unreadable-as-nodes for now (they assemble,
  but their `GL_*` lumps are ignored). Deferred, not dropped: the format is
  deprecated and unused by new content, so it sits behind the ZDoom family, but
  it is tracked for eventual support (#324).
- **Bad** — the GL variants' implicit-`v2` and partner-seg materialization is
  the subtle part; it gets the same oracle discipline the nodebuilder used
  (round-trip and, where a builder exists, cross-checks), plus fuzzing.

## Amendment — Stage 2 shipped (2026-07-21, #327)

Stage 2 (the compressed `Z*` family) is implemented; this records the concrete
decisions §5 left open:

- **Feature name is `extended-nodes-zlib`** (the working name from §5), not a
  reuse of an existing feature. `png`/`doom64-gfx` vendor a decompressor
  *transitively*, but reusing them would couple compressed-node reading to the
  Doom 64 graphics stack; a dedicated, purpose-named flag keeps the surface
  legible and lets the default build stay decompressor-free.
- **`miniz_oxide` is the direct inflater**, taken as a first-class optional
  dependency (`dep:miniz_oxide`) rather than reaching it through `flate2` or the
  `png` transitive path. It is pure Rust (no C, preserving the crate's
  `#![deny(unsafe_code)]` posture in the core) and — decisively — exposes
  `decompress_to_vec_zlib_with_limit`, whose built-in **output limit** *is* the
  ADR-0016 §1 bounded-output guard: inflation stops at the cap instead of
  materializing an oversized buffer and checking after the fact.
- **`Limits::max_decoded_node_bytes` default is `1 << 26`** (64 MiB). This
  comfortably clears the real fixtures — the `RETAIL-EXT` ZNOD sweep (36 maps of
  compressed Freedoom nodes) inflates every map well under the cap — while still
  bounding a hostile "zip bomb". Exceeding it surfaces
  `ExtendedNodeError::DecodedSizeExceeded` (strict) or a whole-BSP
  degrade-to-empty warning (lenient); an un-inflatable stream is
  `ExtendedNodeError::CorruptStream` under the same split.
- **Decode path:** skip the 4-byte plaintext tag, inflate the remainder, and
  feed the inflated body to the *same* Stage-1 parser the uncompressed twin
  uses — so a `Z*` lump yields arenas byte-identical to its `X*` twin, on both
  the binary `NODES`/`SSECTORS` seam and the UDMF `ZNODES` lump. Diagnostics
  report the compressed `Z*` tag (not the `X*` twin the classifier maps to).
- **Still out of scope after Stage 2:** DeePBSP `xNd4` (Stage 3, #328) and
  classic GL `gNd*` (#324) — unchanged from §1/§5.

## Amendment — Stage 3 shipped (2026-07-21, #328)

Stage 3 (DeePBSP v4, `xNd4`) is implemented; the extended-node **read** layer is
now complete (Stages 1–3). This records the concrete decisions §1/§3 left open:

- **A separate `map::deepbsp` reader, not the ZDoom parser.** DeePBSP v4 is a
  *classic-widened* format: it keeps the three separate `SEGS`/`SSECTORS`/`NODES`
  lumps (widening only vertex/seg/child indices to 32-bit), stores classic seg
  semantics (`angle`/`offset`/`side` on disk, no minisegs, no new vertices),
  and shares nothing with the ZDoom family's single self-describing blob. The
  reader is a clean-room classic-shaped decoder that reuses the classic
  `resolve_required` cross-reference discipline and the `normalize_bsp_or_degrade`
  whole-BSP lenient-degrade posture (an out-of-range reference clamps and warns;
  a reference into an empty arena degrades the whole BSP to empty with one
  warning). It is **uncompressed always-on core** — no feature flag.
- **8-byte-signature detection, separate from `EXTENDED_NODE_SIGNATURES`.** The
  `xNd4\0\0\0\0` signature is 8 bytes and heads the `NODES` lump only. The gate
  detects it **first** — before the 4-byte `EXTENDED_NODE_SIGNATURES` `find_map`
  — via `deepbsp::is_deepbsp`/`DEEPBSP_SIGNATURE`, and routes to
  `decode_deepbsp`. It is **not** added to `EXTENDED_NODE_SIGNATURES` (which
  stays a 4-byte table). A `NODES` lump without the `xNd4` signature falls
  through to the 4-byte extended check, then classic — unchanged. DeePBSP is
  binary-only: it never touches the UDMF `ZNODES` path (where `xNd4` remains an
  unrecognized, gated tag).
- **Framing-defect policy: hard error in *both* modes, no new `MapWarning`
  variant.** A structurally malformed DeePBSP lump — a record stream whose
  length is not a whole multiple of its record size (16 `SEGS` / 6 `SSECTORS` /
  32 post-signature `NODES`), or a `NODES` lump shorter than its 8-byte
  signature — is a fatal `MapAssembleError::Records` in **both** strictness
  modes. This deliberately **mirrors the classic `SEGS`/`SSECTORS`/`NODES` path**
  DeePBSP structurally resembles (the classic `decode_optional`/`parse_records`
  decoders reject a misaligned record stream in both modes too), and differs
  from the ZDoom `X*`/`Z*` readers' whole-BSP lenient degrade: for the ZDoom
  family a single self-describing blob degrades to empty on any structural fault,
  whereas DeePBSP's three separate classic-shaped lumps get classic framing
  discipline. Lenient recovery still applies to cross-references, not to
  unparseable bytes.
- **No sweep fixture.** `zdbsp` does not emit DeePBSP, so there is no DeePBSP map
  in `RETAIL-EXT`; Stage 3 coverage is the `deepbsp.rs` unit tests, the
  `tests/deepbsp.rs` integration tests, and the `fuzz_deepbsp` target.
- **The ADR-0015 extended-node revisit condition is fully discharged for
  reading.** Reading the ZDoom uncompressed (#326), compressed (#327), and
  DeePBSP v4 (#328) node formats is done; writing (`#323`) remains a separate
  follow-up (§6).

## Pros and cons of the options

### Option 2 — staged, ZDoom-first, skip classic GL (chosen)

- Good: leads with the one format modern tooling emits; the shared parser makes
  the rest of the ZDoom family nearly free once XGL3 works.
- Good: uncompressed ships with zero new dependencies; compression is a clean,
  deferrable, feature-gated increment.
- Good: reuses the existing arenas and gate seam — one data path, incremental
  narrowing.
- Bad: DeePBSP v4 and classic-GL WADs are second-class (DeeP later, GL never).

### Option 1 — full breadth immediately

- Good: complete on day one.
- Bad: classic GL is the most irregular family (per-lump, per-version magic and
  offsets) for the *least* value (deprecated, superseded), inflating the hardest
  reader for content nobody ships. Bad cost/benefit; delays the common case.

### Option 3 — shell out / document external tools

- Good: no reader to write.
- Bad: a library whose mission is safe typed WAD I/O should read the structures
  it already gates; shelling out fails the analysis/round-trip/editor use cases
  and (per ADR-0024 §6) is a process-execution and determinism smell.

## More information

- **Research record:** the three consolidated passes on #199
  (2026-07-20) — taxonomy, engine consumption, crustywad state — with
  `path:line` anchors into zdbsp/GZDoom and the crustywad codebase.
- **Amends** ADR-0015 (the assembled-map model; its revisit condition and the
  #204 gate amendment) and relates to ADR-0016 (hardening — the new parse
  surface and the inflate cap), ADR-0018 (Doom 64, routed away from the gate),
  ADR-0024 (the nodebuilder; `BuiltNodes` is the shared type basis and the
  deferred writer's input).
- **Source anchors:** `map/assemble.rs` (`EXTENDED_NODE_SIGNATURES` 212-213,
  the gate 1604-1639, `UnsupportedNodeEncoding` 90-95, the UDMF/Doom64 routing
  1552-1554), `map/graph.rs` (the `usize` index newtypes 27-57, `MapSeg`/
  `MapSubsector`/`MapNode`/`NodeChild` 285-382), `map/build/nodes.rs`
  (`BuiltNodes` 70-88), `tests/sweep.rs` (the `RETAIL-EXT` gate sweep 113-179).
- **Related backlog issues:** classic-GL reading (#324) and the extended-node
  writer (#323), both tracked and both depending on #199's read stages.
- **Revisit conditions:** reopen when (a) classic-GL reading (#324) is
  scheduled — it reuses this ADR's scaffolding and settles the `gNd*` detection
  and v1/v4 policy; (b) the extended-node *writer* (#323) is scheduled (it
  reuses these codecs and `BuiltNodes`); or (c) a node format beyond these (a
  future ZDoom `XGL4`, GL PVS data) needs representation.
