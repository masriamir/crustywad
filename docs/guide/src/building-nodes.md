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

`cwad convert --to doom --nodes` is the turnkey path — it runs the one-shot for
every converted map, so the output WAD is engine-playable with no external step:

```bash
cwad convert udmf.wad -o doom.wad --to doom --nodes --lenient
cwad validate --deep doom.wad
```

`--nodes` only affects classic Doom output; it is ignored (with a note on
stderr) for `--to udmf`, which has no binary node lumps. The global `--lenient`
flag selects lenient mode for both the conversion and the node build. See
[CLI Usage](cli.md#convert) for the full flag reference.

### Choosing the on-disk node format

`--node-format` selects how the built nodes are stored (Doom output only; it
has no effect without `--nodes`, and a non-`classic` value passed without
`--nodes` prints a note on stderr and is ignored):

| Value | On-disk form | Notes |
|---|---|---|
| `classic` (default) | `SEGS` / `SSECTORS` / `NODES` (16-bit) | Vanilla-compatible; unchanged from plain `--nodes`. |
| `xnod` | A single uncompressed `XNOD` stream in `NODES` (`SEGS`/`SSECTORS` empty) | ZDoom non-GL extended nodes; lifts the vanilla 16-bit ceilings. |
| `znod` | A zlib-compressed `ZNOD` stream | Same as `xnod`, compressed. Requires `cwad` built with the `extended-nodes-zlib` feature (on by default); a `--no-default-features` build rejects `znod` with a clear error. |

```bash
cwad convert udmf.wad -o doom.wad --to doom --nodes --node-format xnod
```

### `cwad build --nodes`

`cwad build --nodes` runs the same one-shot for a WAD assembled from scratch
out of `NAME=FILE` lump specifications: after packing, it rebuilds every
Doom-format map group in the result with real, engine-playable node lumps,
overwriting any empty placeholders that were packed alongside the map's
other lumps:

```bash
cwad build --nodes MAP01=map01.lmp THINGS=things.lmp ... -o playable.wad
```

Hexen, Doom 64, and UDMF map groups are not yet supported by `build --nodes`
and are passed through unchanged with a note on stderr (follow-up issues
#352/#353/#354); non-map lumps always pass through unchanged. Like plain
`--nodes`, `build --nodes` emits **classic** node
lumps only — there is no `--node-format` flag on `build`. Reach for
`--node-format xnod`/`znod` extended-node output through `convert --nodes`
instead. See [CLI Usage](cli.md#build) for the full flag reference.

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

## When you still need an external nodebuilder

The clean-room builder targets the **classic 16-bit tier only** — vanilla-layout
`SEGS`/`SSECTORS`/`NODES`, which covers every real classic map with wide margin
(ADR-0024 §1). It does **not** generate GL nodes or the extended/compressed
encodings (`XNOD`/`ZNOD`, GL variants). For those, the long-standing recipe
still applies: run an external nodebuilder (`zdbsp`, `bsp`, …) over the output.

`crustywad` *reads* both the ZDoom extended family and classic GL nodes (ADR-0025 and its
amendments, `Extended nodes` milestone, #199/#324) — see
[Extended node encodings](map-records.md#extended-node-encodings) and
[Classic GL nodes](map-records.md#classic-gl-nodes) in the map-records guide. *Generating*
GL/extended nodes is a separate, still-open follow-up (issue #323) that this clean-room
builder does not cover. A `cwad build --nodes` for authoring a WAD from loose lumps is
deferred to issue #320 — today the node-building CLI surface is `cwad convert --nodes`.
