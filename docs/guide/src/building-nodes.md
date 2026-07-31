# Building nodes

A classic Doom map is not engine-playable from its editable lumps alone
(`THINGS`, `LINEDEFS`, `SIDEDEFS`, `VERTEXES`, `SECTORS`). The engine also
reads a run of *derived* lumps — `SEGS`, `SSECTORS`, `NODES` (the BSP tree),
`REJECT` (sector-to-sector line-of-sight), and `BLOCKMAP` (the collision
grid) — that a **nodebuilder** computes from the geometry. `crustywad`'s
[`nodebuild`](features.md#nodebuild) feature is a clean-room nodebuilder for the
classic 16-bit tier (ADR-0024).

This page explains when you need it, the pieces it provides, and the one-line
turnkey path. For the full Rust worked example, see the
[`nodebuild` feature reference](features.md#nodebuild); this page links to it
rather than duplicating the code.

## Why the empty lumps are not always enough

`add_doom_map()` and `cwad convert --to doom` (see
[Converting maps](converting-maps.md)) emit **zero-length** `SEGS`, `SSECTORS`,
`NODES`, `REJECT`, and `BLOCKMAP` — the canonical lump run, present but empty —
and always warn `NodesNotBuilt`. Whether that output plays depends on the
engine tier (ADR-0024):

- **The ZDoom family (GZDoom, Zandronum, …) rebuilds missing nodes at load.**
  It detects all-empty `SEGS`/`SSECTORS`/`NODES` and runs its own internal
  nodebuilder, and likewise regenerates a missing or oversized `BLOCKMAP` and
  `REJECT`. On these ports the empty-lump output is already playable.
- **Vanilla and Chocolate Doom rebuild nothing.** Their loaders copy the node
  indices with almost no validation and walk the tree directly. Empty node
  lumps send every point to subsector 0 or crash outright. These ports need
  **real** node lumps.

The nodebuilder's entire value is the vanilla tier: it produces the lumps a
faithful port requires, so a freshly converted or generated map runs without an
external tool.

## The builders and the one-shot

With `nodebuild` enabled, `crustywad::map::build` exposes three builders, each
turning an assembled [`Map`](map-records.md) into one part of the node run:

| Builder | Produces | Notes |
|---|---|---|
| `build_reject` | `REJECT` | Infallible; the correctly-sized all-zeros table (`ceil(sectors² / 8)` bytes) — an all-clear table pre-rejects no line of sight, exactly what `zdbsp` emits. |
| `build_blockmap` | `BLOCKMAP` | The packed 128-unit collision grid, deduplicated blocklists. |
| `build_nodes` | `SEGS` / `SSECTORS` / `NODES` (+ split vertices) | The classic BSP pass: partitions the map on seg lines into a deterministic tree. |

When `build_nodes` splits a seg it creates a new vertex; its
`to_lump_bytes()` output carries those `split_vertexes`, which **must** be
appended to the map's `VERTEXES` lump or the segs' vertex indices dangle. The
[`nodebuild` worked example](features.md#nodebuild) shows the full manual
assembly, including this append and the canonical lump order.

### The one-shot: `add_doom_map_with_nodes`

`add_doom_map_with_nodes(builder, name, map, write_opts, build_opts)` bundles
all of the above — it serializes the five data lumps, runs the three builders,
appends the split vertices, and adds the complete engine-playable map group to a
`WadBuilder` in canonical lump order. Unlike `add_doom_map`, it **never** emits
`NodesNotBuilt` (it built the nodes); every other write-path recovery still
surfaces, wrapped as `NodeBuildWarning::Write`. Reach for it when you want a
playable Doom map group in one call rather than orchestrating the builders by
hand.

## From the CLI: `cwad convert --nodes`

`cwad convert --to doom --nodes` is the turnkey path — it runs the
`add_doom_map_with_nodes` one-shot for every converted map, so the output WAD
is engine-playable with no external step:

```bash
cwad convert udmf.wad -o doom.wad --to doom --nodes --lenient
cwad validate --deep doom.wad
```

`--to udmf --nodes` instead runs `add_udmf_map_with_nodes`: UDMF has no binary
node lumps, so the only thing to build is a GL `ZNODES` stream carrying the
minimal-sufficient dialect (`gl` auto-format, noted on stderr):

```bash
cwad convert doom.wad -o udmf.wad --to udmf --nodes
```

`--node-format` still selects the dialect, but a UDMF target only accepts a GL
value — the non-GL extended pair (`xnod`/`znod`) has no GL stream to become,
so passing either exits `3` (see #384). The global `--lenient` flag selects
lenient mode for both the conversion and the node build. See
[CLI Usage](cli.md#convert) for the full flag reference.

### Choosing the on-disk node format

`--node-format` selects how the built nodes are stored for a Doom target, or
which GL dialect fills a UDMF target's `ZNODES` stream. It has no effect
without `--nodes`, and a non-`classic` value passed without `--nodes` prints
a note on stderr and is ignored. A UDMF target only accepts a GL value (or
the default `classic`, which auto-selects `gl`) — the non-GL extended pair
(`xnod`/`znod`) has no GL stream to become and is rejected (see #384):

| Value | On-disk form | Notes |
|---|---|---|
| `classic` (default) | `SEGS` / `SSECTORS` / `NODES` (16-bit) | Vanilla-compatible; unchanged from plain `--nodes`. |
| `xnod` | A single uncompressed `XNOD` stream in `NODES` (`SEGS`/`SSECTORS` empty) | ZDoom non-GL extended nodes; lifts the vanilla 16-bit ceilings. |
| `znod` | A zlib-compressed `ZNOD` stream | Same as `xnod`, compressed. Requires `cwad` built with the `extended-nodes-zlib` feature (on by default); a `--no-default-features` build rejects `znod` with a clear error. |
| `xgln` | An uncompressed `XGLN` stream, carried in `SSECTORS` (`SEGS`/`NODES` empty) | The minimal GL dialect: 16-bit seg linedef reference (`0xFFFF` reserved as the miniseg sentinel), whole-unit `i16` node partitions. |
| `xgl2` | An uncompressed `XGL2` stream, carried in `SSECTORS` | Like `xgln` but with a 32-bit seg linedef reference; still whole-unit `i16` node partitions. |
| `xgl3` | An uncompressed `XGL3` stream, carried in `SSECTORS` | Like `xgl2`, plus fractional (sub-unit) node partitions. |
| `gl` | Whichever of `xgln`/`xgl2`/`xgl3` is the minimal dialect the map needs, emitted uncompressed | Escalates only if the geometry requires it (a real linedef index colliding with `XGLN`'s sentinel, or a fractional partition). |
| `zgln` / `zgl2` / `zgl3` / `zgl` | The zlib-compressed twins of the four GL rows above | Each carried in `SSECTORS`, same as its uncompressed twin. Requires `cwad` built with the `extended-nodes-zlib` feature (on by default); without it, these exit `3` with a clear error. |

```bash
cwad convert udmf.wad -o doom.wad --to doom --nodes --node-format xnod
cwad convert udmf.wad -o doom.wad --to doom --nodes --node-format gl
```

### `cwad build --nodes`

`cwad build --nodes` runs the same builders for a WAD assembled from scratch
out of `NAME=FILE` lump specifications: after packing, it rebuilds every
Doom-format map group in the result with real, engine-playable node lumps —
`SEGS`/`SSECTORS`/`NODES`, `REJECT`, and `BLOCKMAP` — overwriting whatever
was packed for those lumps, whether empty placeholders or existing data. The
packed `VERTEXES` lump can also grow, since the BSP pass appends any split
vertices it creates to it. It also rebuilds every UDMF-format map group's
`ZNODES` stream in place (replacing an existing one, or inserting it right
after `TEXTMAP`), the rest of the group's lumps carried through unchanged:

```bash
cwad build --nodes MAP01=map01.lmp THINGS=things.lmp ... -o playable.wad
cwad build --nodes --node-format gl MAP01=map01.lmp THINGS=things.lmp ... -o playable.wad
```

`build --nodes` accepts the same `--node-format` values as `convert --nodes`
(the table above), including the GL dialects and their `z*` zlib twins. As
with `convert --to udmf --nodes`, a UDMF map group's `ZNODES` only accepts a
GL dialect: `classic` auto-selects `gl` (noted on stderr); an explicit
`xnod`/`znod` skips that one group with a note instead of failing the whole
build (see #384). Hexen (#352) and Doom 64 (#353) map groups are not yet
supported by `build --nodes` and are passed through unchanged with a note on
stderr; non-map lumps always pass through unchanged. See
[CLI Usage](cli.md#build) for the full flag reference.

## The tolerated mixed-sector fan

Real geometry occasionally produces a convex leaf that spans more than one
sector with no seg line able to separate them — the **mixed-sector fan** (two
sectors meeting at a bare corner vertex). Across the full retail collection,
551 classic maps build clean save for exactly this case:

- **Strict** `build_nodes` / `add_doom_map_with_nodes` **rejects** such a map.
- **Lenient** accepts the leaf and emits `NodeBuildWarning::MixedSectorSubsector`
  — the exact engine-tolerated output the retail masters themselves ship
  (ADR-0024 §7 amendment).

This is why converting real maps with `--nodes` often needs `--lenient`: the
warning names an inherent property of the source geometry, not a defect the
builder could fix.

## What the library generates, and when you still need an external nodebuilder

The clean-room builder now spans three tiers of output:

- **Classic 16-bit** — vanilla-layout `SEGS`/`SSECTORS`/`NODES`, which covers every
  real classic map with wide margin (ADR-0024 §1). This is `NodeFormat::Classic`.
- **Non-GL extended** — the `XNOD` stream (and its zlib twin `ZNOD`, behind
  `extended-nodes-zlib`) via `build_nodes` + `BuiltNodes::to_extended_lump_bytes`,
  lifting the vanilla node ceilings (ADR-0025 §Amendment #323).
- **GL extended** — the `XGLN`/`XGL2`/`XGL3` streams (and their zlib twins
  `ZGLN`/`ZGL2`/`ZGL3` with `extended-nodes-zlib`) via `build_gl_nodes`
  + `BuiltGlNodes::to_extended_lump_bytes`, or the `add_doom_map_with_nodes` one-shot,
  which carries the GL stream in `SSECTORS` (ADR-0026, #364, #365). The
  `add_udmf_map_with_nodes` one-shot builds the same GL stream for a UDMF map
  group, carried in `ZNODES` instead (#354).
  `NodeFormat::Gl`/`NodeFormat::Zgl` auto-select the minimal dialect a map needs — `XGLN` unless a
  real linedef index collides with `XGLN`'s `0xFFFF` miniseg sentinel (forcing `XGL2`'s
  32-bit linedefs) or a fractional partition forces `XGL3`.

`crustywad` *reads* the full ZDoom extended family and classic GL nodes (ADR-0025 and its
amendments, `Extended nodes` milestone, #199/#324) — see
[Extended node encodings](map-records.md#extended-node-encodings) and
[Classic GL nodes](map-records.md#classic-gl-nodes) in the map-records guide.

Both `cwad convert --nodes` and `cwad build --nodes` expose the full tier set through
`--node-format` — classic, the non-GL extended pair, and all four GL dialects (see the table
above) — so no external nodebuilder pass is needed to reach any of them from the CLI.
