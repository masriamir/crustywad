# ADR-0019: Map format conversion (UDMF ↔ Doom)

- **Status:** Accepted
- **Date:** 2026-07-13
- **Deciders:** @masriamir
- **Tracking issue:** https://github.com/masriamir/crustywad/issues/59

## Context and problem statement

Epic #17's last unshipped piece is conversion between UDMF text maps and the
classic Doom binary map layout. This ADR decides the conversion contract, the
data-loss policy, and the public write surface it needs. It also **amends
ADR-0017's "Decisions resolved during spike review" item 3**, which deferred
UDMF thing-flag normalization, describing a normalized representation as
"left for a concrete future consumer to motivate" (ADR-0017 §1, "What is
deferred"): #59 is that consumer, and the deferral ends here (§2).

The crate is asymmetric today, and the asymmetry is the problem:

- **Classic → UDMF mostly falls out of what already shipped.**
  `write_udmf(&Map, &WriteOptions) -> Result<(String, Vec<UdmfWriteWarning>), UdmfWriteError>`
  (`map/udmf/write.rs`, #231) accepts *any* assembled `Map` — `MapFormat::Doom`,
  `Hexen`, or `Udmf` — and emits `TEXTMAP` text. Because `Map` is
  format-agnostic (ADR-0015), the conversion is a consequence of the graph
  model rather than a feature anyone wrote. Two defects keep it from being
  correct (§5).
- **UDMF → classic does not exist in any form.** There is **no binary map write
  path in the crate at all**: no `BinWrite` on any map record struct, no
  `write_doom_map`, nothing. There is no typed *binary* map-level builder
  helper either — `WadBuilder`'s own inherent methods offer only raw
  `add_lump(name, bytes)`; the sole map-level helper today is
  `add_udmf_map(&mut WadBuilder, name, &Map, &WriteOptions)` (`map/udmf/write.rs`,
  #231), and it writes UDMF text, not the binary format.
- **A fidelity hole sits directly under conversion.** Because ADR-0017 deferred
  thing-flag normalization, UDMF's `skill1..5`, `ambush`, `single`, `dm`,
  `coop`, and `friend` booleans are parsed for syntax and then dropped, and
  assembly hardcodes `flags: 0` for UDMF things. Converting a UDMF map to Doom
  today would emit a map in which every thing has zero flags — i.e. appears on
  no skill level, in no game mode. Conversion cannot ship on top of that.

### Current code this ADR must not contradict (verified, not recalled)

Each item below was confirmed by opening the file, per the ADR-writing
checklist:

- `map/udmf/model.rs`: `UdmfThing { x: f64, y: f64, height: f64, angle: i32,
  type_id: i32, id: i32, special: i32, args: [i32; 5] }` — it has **no `flags`
  field**. `UdmfLinedef` **does**: `flags: u32`, "the Doom-mapped linedef flags
  packed into bits 0–8." Every `map::udmf` model struct is `#[non_exhaustive]`,
  so adding a field is additive.
- `map/assemble.rs`, `normalize_udmf_things`: constructs `MapThing` with
  `flags: 0`, commented "UDMF thing flags are not modeled in Map yet
  (ADR-0017 §1)."
- `map/udmf/write.rs`, the thing-flag emitter: tests bits `0x0001`, `0x0002`,
  `0x0004`, `0x0008`, `0x0010` only — **bits 0–4**. Bits 5–7 (`dm`, `coop`,
  `friend`) are never emitted. Its own comment concedes the mapping "is
  currently one-way within the crate."
- `map/udmf/write.rs`, `write_udmf`: when `map.namespace()` is `None` it writes
  `"doom"` regardless of `map.format()`, so a Hexen-sourced map is labeled
  `doom`. It pushes `UdmfWriteWarning::NamespaceDefaulted { used: "doom" }` only
  in lenient mode; strict mode writes the same default silently.
- On-disk record types (`map/doom.rs`, `map/common.rs`), all `#[br(little)]`
  `BinRead` derives — these are the exact target types the graph's wider fields
  narrow *to*:

  | Record | Fields |
  |---|---|
  | `doom::Thing` | `x: i16`, `y: i16`, `angle: u16`, `type_id: u16`, `flags: u16` |
  | `doom::Linedef` | `start_vertex: u16`, `end_vertex: u16`, `flags: u16`, `special_type: u16`, `sector_tag: u16`, `right_sidedef: u16`, `left_sidedef: u16` (`0xffff` = none) |
  | `common::Vertex` | `x: i16`, `y: i16` |
  | `common::Sidedef` | `x_offset: i16`, `y_offset: i16`, `upper_texture`/`lower_texture`/`middle_texture: Name8`, `sector: u16` |
  | `common::Sector` | `floor_height: i16`, `ceiling_height: i16`, `floor_texture`/`ceiling_texture: Name8`, `light_level: i16`, `special_type: i16`, `tag: i16` |

  `Name8` is a newtype over `[u8; 8]`, NUL-padded on the right — exactly 8
  bytes on disk, never "up to 8 characters."
- `lib.rs`: `RawHeader` and `RawDirectoryEntry` already carry
  `#[cfg_attr(feature = "write", derive(binrw::BinWrite))]` +
  `bw(little)` alongside their `BinRead` derives. That is the established
  pattern for adding a write side to an existing `binrw` record — the map
  records simply have not been given it.
- `write.rs`: `WriteOptions { strictness: Strictness }`; `WriteWarning::NameTruncated`
  is the existing precedent for lenient 8-byte name truncation.
- `map/doom.rs` is a single file; `map::udmf` is already a directory module
  (`mod.rs`, `model.rs`, `parse.rs`, `write.rs`).

## Decision drivers

- **One data path.** ADR-0015 made `Map` the single normalized graph so that
  consumers never branch on `MapFormat`. A converter that reads or writes
  around the graph would create a second, divergent notion of what a map *is*.
- **Honest loss reporting.** Doom's binary records are strictly narrower than
  UDMF's text fields. Conversion must say what it lost, and strict mode must
  refuse rather than silently degrade (ADR-0003).
- **A single decision table for the two strictness modes.** Every strict-mode
  error should have exactly one lenient-mode counterpart, so the modes are two
  readings of one table rather than two implementations.
- **Declare the on-disk layout once.** The byte layout already lives in the
  `binrw` record structs; the writer must reuse them, not restate them.
- **ADR-0016 hardening applies to the new write surface** (bounded allocation,
  no recursion, a fuzz target, both modes non-panicking).
- **Do not re-open the "largest unknown" scope trap** (ADR-0014): conversion is
  a structural transform, not a game-semantics layer.

### Scope

**In scope:** UDMF ↔ Doom, both directions.

**Out of scope, explicitly:**

- **A Hexen binary write target.** Hexen remains a read-only *source* format: a
  Hexen `Map` can be written to UDMF, but there is no Hexen binary writer.
- **Node building.** crustywad has no nodebuilder; GL nodes are #199 (horizon
  `Later`).
- **Game/engine semantic tables** — thing-type (DoomedNum) and linedef-special
  interpretation, namespace-gated field legality. These remain the standing
  ADR-0014 non-goal, unchanged.

## Considered options

1. **The `Map` graph is the conversion contract** — conversion is
   `read → Map → write`, with a new binary writer hanging off `Map` exactly as
   `write_udmf` already does.
2. **A direct format-to-format converter** — a dedicated UDMF-text → Doom-record
   path (and back) that bypasses `Map`, so it can carry fields the graph does
   not model (`user_*`, SPAC booleans, slopes, comments) straight across.
3. **Widen `Map` into a full-fidelity superset** of every format, then convert
   through it — model every UDMF boolean, slope, and custom field in the graph
   so that conversion is lossless by construction.

## Decision outcome

Chosen option: **Option 1 — the `Map` graph is the conversion contract.**

### 1. `read → Map → write`, and no second data path

Conversion introduces **no new data path**. A UDMF field that `Map` does not
model is lost at *read* time — that is the already-accepted ADR-0017 contract —
not at conversion time. Strict conversion therefore polices only the loss that
is **visible in `Map`**; it cannot, and does not claim to, police loss that
already happened upstream.

Concretely, these UDMF fields are dropped on read and are **not** conversion
concerns: `*.comment`, `user_*` custom fields, `class1`–`class3`, `dormant`,
`standing`, the SPAC/Strife/port-extension linedef booleans, sector slope and
plane-equation fields, and the `skill1 != skill2` / `skill4 != skill5`
distinction (§2). Callers needing full text-level fidelity use the
un-normalized `UdmfMap` intermediate, exactly as ADR-0017 §1 intended.

### 2. `MapThing.flags` is populated for UDMF (amends ADR-0017 decision 3)

ADR-0017 decision 3 read: *"Defer `MapThing.flags` UDMF synthesis."* Its §1
"What is deferred" list adds that a normalized representation is *"left for a
concrete future consumer to motivate (see Decisions resolved during spike
review, below)."* **That deferral is hereby closed.** #59 is the
consumer, and the reason is concrete: without a synthesis, every UDMF map
converted to Doom produces things with `flags == 0`, which appear on no skill
level. What changes: `UdmfThing` gains `flags: u32`, packed by
`map/udmf/parse.rs` exactly the way `UdmfLinedef.flags` already is (`UdmfThing`
is `#[non_exhaustive]`, so this is additive), and `normalize_udmf_things` copies
it through instead of hardcoding `0`. Nothing else in ADR-0017 is disturbed.

The packing uses the Doom/Boom-MBF bit layout, so `MapThing.flags` means one
thing across every format:

| UDMF boolean(s) | UDMF default | `flags` bit | Doom meaning |
|---|---|---|---|
| `skill1` \| `skill2` | `false` | 0 (`0x0001`) | appears on skill 1–2 |
| `skill3` | `false` | 1 (`0x0002`) | appears on skill 3 |
| `skill4` \| `skill5` | `false` | 2 (`0x0004`) | appears on skill 4–5 |
| `ambush` | `false` | 3 (`0x0008`) | deaf / ambush |
| `!single` | `true` | 4 (`0x0010`) | multiplayer only |
| `!dm` | `true` | 5 (`0x0020`) | not in deathmatch (Boom) |
| `!coop` | `true` | 6 (`0x0040`) | not in co-op (Boom) |
| `friend` | `false` | 7 (`0x0080`) | friendly (MBF) |

`single`/`dm`/`coop` default to `true` in UDMF (the thing appears in that mode
unless excluded), which is why the Doom "not in X" bits are their inverse.

**Accepted lossiness.** `skill1`/`skill2` and `skill4`/`skill5` are independent
booleans in UDMF and a single bit each in Doom; they are **OR-folded**. A UDMF
map with `skill1 = true; skill2 = false;` normalizes identically to one with
both set. `class1`–`class3`, `dormant`, and `standing` have no Doom bit and are
dropped. This is inherent to the Decision 1 contract — it is loss at read time,
not at conversion time — so it is **not** reported as a warning, consistent with
every other unmodeled UDMF field, which ADR-0017 drops silently. It is
documented on `MapThing.flags` and here.

### 3. The binary write path: `map::doom::write`, and a three-tier loss policy

`map::doom` becomes a directory module (`map/doom/mod.rs` for the records,
`map/doom/write.rs` for the serializer), mirroring `map::udmf`'s existing
layout. The surface is parallel to `write_udmf` / `add_udmf_map` and is gated
behind the existing `write` feature:

```rust
/// The five serialized Doom map data lumps.
pub struct DoomMapLumps {
    pub things: Vec<u8>,
    pub linedefs: Vec<u8>,
    pub sidedefs: Vec<u8>,
    pub vertexes: Vec<u8>,
    pub sectors: Vec<u8>,
}

pub fn write_doom_map(map: &Map, opts: &WriteOptions)
    -> Result<(DoomMapLumps, Vec<DoomWriteWarning>), DoomWriteError>;

pub fn add_doom_map(builder: &mut WadBuilder, name: &str, map: &Map, opts: &WriteOptions)
    -> Result<Vec<DoomWriteWarning>, DoomWriteError>;
```

`BinWrite` is **derived on the existing** `map::doom::{Thing, Linedef}` and
`map::common::{Vertex, Sidedef, Sector, Name8}` structs, alongside their current
`BinRead` — the same `#[cfg_attr(feature = "write", derive(binrw::BinWrite))]` +
`bw(little)` pattern `RawHeader`/`RawDirectoryEntry` already use. These are not
new types. Serialization is therefore `Map` → typed records → bytes: the on-disk
layout stays declared in exactly one place, and round-trip tests can assert
`parse_records(write(x)) == x`.

Narrowing the graph's wider fields (`f64` coordinates, `i32` specials and tags)
into the on-disk types tabulated in Context is where conversion loses data. The
policy has three tiers.

#### Tier 1 — structurally impossible: errors in *both* modes

Doom addresses vertices, sidedefs, and sectors with `u16` indices, so a graph
too large to index cannot be encoded and has no honest recovery — truncating an
arena corrupts the geometry. This mirrors assembly's established "empty required
arena is always fatal" precedent (ADR-0015 §4).

| Arena | Maximum | Why |
|---|---|---|
| vertices | `65_536` | indices `0..=65_535` |
| sectors | `65_536` | indices `0..=65_535` |
| sidedefs | `65_535` | `0xffff` is the "no sidedef" sentinel |

Reported as `DoomWriteError::TooManyElements { kind, count, max }` in both
strictness modes.

#### Tier 2 — value loss: strict errors, lenient recovers and warns

| Loss | Lenient recovery |
|---|---|
| Fractional `f64` coordinate (vertex `x`/`y`, thing `x`/`y`) | round to nearest `i16` (half away from zero) |
| Coordinate outside `i16` range | clamp to `i16::MIN`/`i16::MAX` |
| Linedef `special` outside `u16`; `args[0]` (the sector tag) outside `u16` | clamp |
| Sidedef `x_offset` / `y_offset` outside `i16` | clamp |
| Sector `floor_height` / `ceiling_height` / `light` / `special` / `tag` outside `i16` | clamp |
| Thing `flags` with any bit above 15 set | truncate to `u16` |
| Texture/flat name longer than 8 bytes | truncate to 8 bytes (mirrors the existing `WriteWarning::NameTruncated`) |

Doom's on-disk name field is a raw `[u8; 8]` and `Name8` preserves those bytes
verbatim, so a **non-ASCII name is not loss**: any name that fits in 8 bytes
writes and reads back byte-for-byte. Only exceeding 8 bytes is loss.

Non-finite (`NaN`/`∞`) coordinates follow the precedent `write_udmf` already
set: strict errors, lenient substitutes `0` and warns.

#### Tier 3 — no slot in the Doom format: strict errors, lenient drops and warns

A Doom linedef carries only `special_type` plus `sector_tag`; a Doom thing
carries no special, no tid, and no height. A nonzero value in any of the
following is data the target format simply cannot hold:

- linedef `args[1..=4]` (nonzero)
- linedef `id`
- thing `special` and `args[0..=4]` (nonzero)
- thing `height` (nonzero)
- thing `id` (the tid)

A ZDoom-namespace UDMF map will therefore typically **fail strict conversion**,
with an error naming the first offending field. That is the intended answer —
"this map is not expressible in Doom format" — and `--lenient` (library:
`WriteOptions::lenient()`) is the single-flag acknowledgment of the loss.

#### Error and warning types

```rust
#[non_exhaustive]
pub enum DoomWriteError {
    TooManyElements { kind: &'static str, count: usize, max: usize },   // both modes
    NonFiniteCoordinate { block: &'static str, field: &'static str, index: usize },
    FractionalCoordinate { block: &'static str, field: &'static str, index: usize, value: f64 },
    ValueOutOfRange { block: &'static str, field: &'static str, index: usize, value: i64 },
    UnrepresentableField { block: &'static str, field: &'static str, index: usize },
    NameTooLong { name: String, len: usize },
}

#[non_exhaustive]
pub enum DoomWriteWarning {
    NodesNotBuilt,                                                       // always, both modes
    NonFiniteReplaced { block: &'static str, field: &'static str, index: usize },
    CoordinateRounded { block: &'static str, field: &'static str, index: usize, from: f64, to: i16 },
    ValueClamped { block: &'static str, field: &'static str, index: usize, from: i64, to: i64 },
    FieldDropped { block: &'static str, field: &'static str, index: usize },
    NameTruncated { name: String },
}
```

Each strict-mode `DoomWriteError` has exactly one lenient-mode
`DoomWriteWarning` counterpart, so the two modes are a single decision table
read twice.

### 4. Empty node lumps, and an always-on warning

`add_doom_map` emits the `name` marker, the five data lumps above, and
**zero-length `SEGS`, `SSECTORS`, `NODES`, `REJECT`, and `BLOCKMAP`** — the
canonical Doom lump run that editors and nodebuilders expect to find. Every call
emits a `DoomWriteWarning::NodesNotBuilt`, in **both** strictness modes (it is
not a defect to be fixed by strictness; it is a property of the output).

The output is therefore **editor- and nodebuilder-ready, not engine-playable**:
an external nodebuilder (`zdbsp`, `bsp`, …) must process it before it will run.
This is stated in the `add_doom_map` docs, the guide, and here. Building nodes
is out of scope (#199); this ADR does not smuggle in a nodebuilder by any other
name.

### 5. Asymmetric reversibility — and two fixes to the classic → UDMF direction

Two defects in `map/udmf/write.rs` (both verified above) are corrected:

1. **Namespace mislabeling.** `write_udmf` writes `"doom"` whenever
   `map.namespace()` is `None`, mislabeling a Hexen-sourced map. Derive the
   namespace from `map.format()` instead — `MapFormat::Doom → "doom"`,
   `MapFormat::Hexen → "hexen"` — keeping
   `UdmfWriteWarning::NamespaceDefaulted { used }` to report what was chosen.
2. **Incomplete thing-flag emission.** The writer emits bits 0–4 only. Extend it
   to bits 5–7 — `dm = false` (bit 5), `coop = false` (bit 6), `friend = true`
   (bit 7) — making it the exact inverse of the read-side packing in §2.

With both fixed, the two directions are **not** symmetric, and the ADR states
this plainly rather than implying otherwise:

- **Doom → UDMF → Doom is a byte-identical round-trip** for the five data lumps.
  Every Doom on-disk field has an exact UDMF representation, and the read-side
  packing and write-side emission of §2/§5 are inverses. This becomes the
  headline property test.
- **UDMF → Doom → UDMF is *not* reversible.** `f64` → `i16` rounding and the
  Tier-3 drops are one-way, and the OR-folding of §2 is one-way on top of that.
  No option, flag, or mode makes it reversible. This is documented in the module
  docs and the guide as well as here.

## Consequences

- **New public API** (all behind the existing `write` feature, except the read-
  side field): `map::doom::write` — `DoomMapLumps`, `DoomWriteError`,
  `DoomWriteWarning`, `write_doom_map`, `add_doom_map`; and `UdmfThing.flags`
  (additive, `#[non_exhaustive]`).
- **`map::doom` becomes a directory module** (`map/doom/mod.rs` +
  `map/doom/write.rs`). Public paths are unchanged — `map::doom::Thing` still
  resolves — so this is a source reorganization, not a breaking change.
- **`BinWrite` derives are added to existing record structs**
  (`doom::{Thing, Linedef}`, `common::{Vertex, Sidedef, Sector, Name8}`). These
  types already exist and already derive `BinRead`; nothing new is introduced,
  and the on-disk layout is still declared exactly once.
- **ADR-0017 decision 3 is amended, not superseded.** Only the thing-flag
  deferral is closed. ADR-0017's other deferrals — the ~20 non-bit-mappable
  booleans, comment/`user_*` retention, slopes, non-UTF-8 `TEXTMAP` decoding —
  all stand.
- **Behavior change (pre-1.0, intentional):** UDMF-sourced `MapThing.flags` was
  always `0`; it now carries the synthesized bits above. `write_udmf` now labels
  a Hexen-sourced map `hexen` rather than `doom`, and emits three thing-flag
  booleans it previously omitted.
- **Good** — conversion needs no new notion of "a map." A format added later
  (Hexen write, Doom 64 normalization) plugs into the same `Map` graph and
  inherits the read side for free.
- **Good** — strict mode is a usable gate: "does this UDMF map fit in Doom?" is
  answered by `write_doom_map(&map, &WriteOptions::strict())` returning `Ok`.
- **Bad** — a full-fidelity, lossless UDMF → UDMF passthrough is impossible
  through this API by construction. That is the accepted cost of Decision 1, and
  the `UdmfMap` intermediate remains the answer for a consumer that needs it.
- **Bad** — converted Doom output cannot be played without an external
  nodebuilder. Users must be told this every time, which is why the warning is
  unconditional.
- **Hardening (ADR-0016)** — the write path is a new surface and takes the full
  checklist: a `fuzz_write_doom_map` target (arbitrary WAD bytes → lenient
  assemble → lenient `write_doom_map`, no-panic oracle plus the `O(input)`
  output-size assertion, committed seed corpus, wired into `fuzz.yml`); output
  size is `O(elements)` with every count bounded by the input `Map`'s arenas; no
  recursion anywhere on the write path; both strictness modes non-panicking.
  `#![deny(unsafe_code)]` holds.

## Pros and cons of the options

### Option 1 — the `Map` graph is the conversion contract (chosen)

- Good, because it adds no second notion of a map: `read → Map → write` reuses
  the ADR-0015 graph, the existing readers, and the existing `WriteOptions` /
  strictness contract.
- Good, because classic → UDMF already works through it, so only the missing
  half (a binary writer) is new code.
- Good, because every future format gets conversion to and from every existing
  one by implementing one reader and one writer, not N² converters.
- Bad, because the conversion's fidelity ceiling is `Map`'s fidelity ceiling: a
  field the graph does not model cannot be carried across, even between two
  formats that both have it.

### Option 2 — a direct format-to-format converter

- Good, because it could carry `user_*` fields, comments, and unmodeled booleans
  through a UDMF → UDMF or UDMF → ZDoom-Doom path that the graph loses.
- Bad, because it creates a second, parallel definition of a map's contents that
  must be kept in step with `Map` forever — precisely what ADR-0015 exists to
  prevent.
- Bad, because it scales as N² converters across the four formats, and each one
  needs its own loss policy, its own errors, and its own fuzz target.
- Bad, because the fidelity it buys is only realizable in UDMF → UDMF; the Doom
  target has no slot for those fields either way.

### Option 3 — widen `Map` into a full-fidelity superset

- Good, because conversion would be lossless by construction for any field the
  graph models.
- Bad, because it forces every port's namespace extension (slopes, 3D floors,
  portals, per-sidedef scaling) into the core graph — the exact "largest unknown"
  scope trap ADR-0014 flagged and ADR-0017 §1 explicitly refused.
- Bad, because it does not actually remove the loss: Doom's `i16`/`u16` records
  cannot hold the extra fields regardless of how much the graph models, so Tiers
  2 and 3 survive unchanged. It buys fidelity only for conversions that do not
  target Doom.
- Bad, because it would make every consumer pay (in API surface and match arms)
  for fields only a full-fidelity editor wants — which is what the `UdmfMap`
  intermediate already serves.

## More information

- Tracking issue: #59, the last piece of Epic #17 (multi-format maps),
  milestone v0.3.0. Depends on #58 (UDMF read, merged) and #60 (UDMF write,
  merged). Delivered as three PRs: this ADR; the library (read-side flags,
  `map::doom::write`, the `write_udmf` fixes, tests, fuzz, guide page); and the
  `cwad convert` subcommand.
- **Amends** ADR-0017 ("Decisions resolved during spike review" item 3 — the
  `MapThing.flags` deferral), whose own Revisit condition anticipated exactly
  this trigger: *"reopen if a concrete UDMF fixture needs … the `MapThing.flags`
  normalization."* ADR-0017 is otherwise unchanged and remains Accepted.
- Related ADRs: ADR-0003 (strict/lenient contract — the three-tier policy is its
  application to the write path), ADR-0006 (WAD write design — `WadBuilder`,
  `WriteOptions`, `WriteWarning::NameTruncated`), ADR-0014 (multi-format
  strategy; game/engine semantic tables remain a non-goal), ADR-0015 (the `Map`
  graph this ADR makes the conversion contract), ADR-0016 (hardening — the
  `fuzz_write_doom_map` obligation), ADR-0018 (Doom 64, still outside the `Map`
  graph and therefore outside conversion).
- Source anchors: `crates/crustywad/src/map/udmf/model.rs` (`UdmfThing`,
  `UdmfLinedef.flags`), `crates/crustywad/src/map/udmf/write.rs`
  (`write_udmf`, the namespace default, the bits 0–4 thing-flag emitter),
  `crates/crustywad/src/map/assemble.rs` (`normalize_udmf_things`),
  `crates/crustywad/src/map/doom.rs` and `crates/crustywad/src/map/common.rs`
  (the on-disk record types), `crates/crustywad/src/write.rs` (`WriteOptions`,
  `WriteWarning`), `crates/crustywad/src/lib.rs` (the
  `cfg_attr(feature = "write", derive(binrw::BinWrite))` precedent).
- **Revisit condition:** reopen if a **Hexen binary write target** is needed —
  `write_doom_map` would then gain a second target format, and the single
  decision table of §3 would have to become per-target; or if a **nodebuilder
  lands (#199)**, at which point the empty-node-lump decision (§4) and its
  unconditional `NodesNotBuilt` warning become obsolete and the output can
  become engine-playable.
