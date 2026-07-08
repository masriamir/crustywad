# ADR-0014: Multi-format map support strategy

- **Status:** Proposed
- **Date:** 2026-07-08
- **Deciders:** @masriamir
- **Tracking issue:** https://github.com/masriamir/crustywad/issues/53

## Context and problem statement

`crustywad` currently understands exactly one map data format: the classic Doom
binary layout. `crates/crustywad/src/map.rs` defines eight `binrw`-derived record
structs (`Thing`, `Linedef`, `Sidedef`, `Vertex`, `Seg`, `Subsector`, `Node`,
`Sector`) whose fields match the Doom on-disk layout — e.g. `Thing` is 10 bytes
(`x: i16`, `y: i16`, `angle: u16`, `type_id: u16`, `flags: u16`) and `Linedef` is
14 bytes. These structs hold the *unqualified* names even though the bytes they
describe are Doom-specific.

Epic #17 requires the crate to read Doom 64 (#54), Hexen (#55), Heretic (#56),
and UDMF (#57–#60) maps. Before any of those land we must decide how the format
axis is modeled, how a map's format is detected, how record types are organized,
and how much of the existing Doom code is reused versus duplicated. This ADR is
the deliverable of the format-landscape spike (#53) and **blocks #54, #55, #56,
and #57**.

### The two independent axes

The single biggest source of confusion in multi-format support is conflating two
orthogonal properties:

- **Map data format** — the *byte/text layout* of a map's lumps. There are three
  binary layouts (Doom, Hexen, Doom 64) and one text layout (UDMF).
- **Game/engine identity** — *which game* the WAD targets (Doom, Doom II,
  Heretic, Hexen, Strife, Doom 64). This governs semantic tables (thing-type IDs,
  linedef specials) but **not** byte layout.

These do not line up one-to-one. The clearest example: **Heretic maps use the
exact same binary layout as Doom maps** — 10-byte things, 14-byte linedefs, and
identical `Vertex`/`Sidedef`/`Sector` records. Heretic is a different *game* but
not a different *map format*. Conversely, a single Hexen IWAD can contain both
Hexen-binary maps and (in modern ports) UDMF maps. `WadKind` (`lib.rs:97`) sits
on a third axis entirely — it is the container kind (`Iwad`/`Pwad`/`Unknown`) and
identifies neither game nor map format.

### What each format actually requires

Field sizes below are stated as on-disk byte layouts, anchored to the record
definitions they extend.

- **Doom** (baseline, already implemented). `Thing` 10 B, `Linedef` 14 B; the
  other six records as in `map.rs` today.
- **Heretic** (#56). Byte-identical to Doom at the map-record level. Requires
  **no new record structs** — it reuses the Doom records. Only game-semantic
  tables differ, and those are out of scope for this ADR (see Non-goals).
- **Hexen** (#55). Diverges in exactly two records:
  - `Thing` → 20 B: adds `tid: u16`, `z: i16` (spawn height), and replaces the
    Doom flag tail with `special: u8` + `args: [u8; 5]`.
  - `Linedef` → 16 B: replaces `special_type`/`sector_tag` (two `u16`) with
    `special: u8` + `args: [u8; 5]`.
  - Adds a compiled-ACS `BEHAVIOR` lump to each map group. **The presence of a
    `BEHAVIOR` lump in a map's lump group is the canonical signal that the map is
    Hexen-format.** `Vertex`, `Sidedef`, `Sector`, `Seg`, `Subsector`, `Node`
    are unchanged from Doom.
- **Doom 64** (#54). The most divergent target and the largest unknown. Beyond
  map records it uses a distinct graphics/palette format and extra lumps
  (`LIGHTS`, `MACROS`). Exact record byte layouts and the container specifics of
  the Doom 64 EX / 2020 re-release IWAD are **not settled by this ADR** and must
  be confirmed against those specs in #54. This ADR only commits to *where* Doom
  64 records live and *how* the format is selected, not their field layouts.
- **UDMF** (#57–#60). A fundamentally different paradigm: a map's geometry lives
  in a single **text** `TEXTMAP` lump (a block grammar — `thing { x = 1.5; … }`)
  introduced by a `namespace "doom";` declaration, with coordinates as
  **floating-point**, not `i16`. It is not a fixed-size binary record stream and
  cannot flow through the existing `binrw` path. It needs its own lexer/parser
  and its own line/column-aware error type.

### Current parsing entry point

`parse_records::<T>(bytes) -> Result<Vec<T>, MapParseError>` is constrained to
`T: for<'a> BinRead<Args<'a> = ()>`. This constraint is **sufficient for the
binary map formats whose layouts are settled**: Doom and Hexen records are
fixed-size, context-free `binrw` structs, so the same helper decodes both without
change. Doom 64 is *expected* to fit the same mold, but its record layouts are
not settled by this ADR (see the Doom 64 note above) — the `Args<'a> = ()`
assumption for it is contingent on confirmation in #54. The `Args<'a> = ()` bound
only becomes a limitation for a format that needs runtime-parameterized record
parsing, which none of the confirmed binary map formats require; if #54 finds
Doom 64 needs parse context, that is the trigger to revisit the bound (see the
revisit condition). UDMF sidesteps the helper entirely because it is text.

### Name handling duplication

Two independent implementations of "trim an 8-byte NUL-padded Doom name" exist:
`Lump::name` / `decode_name` in `lib.rs` (strict-validates ASCII, used for
directory names) and `Name8::as_str_lossy` in `map.rs` (always decodes via
`String::from_utf8_lossy` — lossless for pure ASCII, replacing only non-ASCII
bytes — used for in-record texture names). Every new format that references
texture/flat names
will reach for `Name8`, so this fork is worth resolving before it spreads.

## Non-goals

This ADR decides the **map data format** axis (byte/text layout). It deliberately
does **not** decide two adjacent concerns:

- **Game/engine semantic tables** — the *meaning* of a record's `special` / `type`
  values (thing-type IDs, linedef and sector special tables). These differ by
  game (Doom vs Heretic) and by engine, and are handled by later per-format /
  per-game work, not by this format-layout decision.
- **Engine / compatibility level** — a *third* axis, distinct from both
  `WadKind` (container) and `MapFormat` (layout): the source-port compatibility a
  map targets, e.g. **vanilla → Boom → MBF/MBF21 → ZDoom-in-Doom-format**. It
  governs which `special` numbers are valid and how they are interpreted, but it
  does **not** change byte layout.

  **Boom is the worked example.** A Boom-compatible map uses the *byte-identical
  vanilla Doom binary layout* (10-byte `THINGS`, 14-byte `LINEDEFS`, unchanged
  `VERTEXES`/`SIDEDEFS`/`SECTORS`). Boom adds only *new values* in the existing
  `special`/`type` fields (generalized linedef types, extra sector effects, deep
  water, friction, scrollers). Therefore Boom is **`MapFormat::Doom` plus
  extended engine semantics — never a new `MapFormat` variant.** Like Heretic,
  it is **not auto-detectable from the map lumps** (a Boom map carries no
  `BEHAVIOR` or `TEXTMAP` marker; compatibility is declared by the author /
  complevel), which is the same lesson `detect_map_format` already draws for
  Heretic. Implementers should model Boom/MBF on the engine-level axis, not by
  extending `MapFormat`.

  Boom-era **auxiliary lumps** (`ANIMATED`, `SWITCHES`, `TRANMAP`, `COLORMAP`)
  are new binary formats, but they belong to the graphics/texture/animation
  domain (later milestones, ~#156/#157), not to map geometry, and are out of
  scope here too.

The engine-level axis does not need a representation *now* (semantics are
deferred), but it is named here so it is not later mistaken for a `MapFormat`
variant. `MapFormat` being `#[non_exhaustive]` is for genuinely new *layouts*
(a future port's on-disk format), not for compatibility levels over the Doom
layout.

## Decision drivers

- **`#155` map-graph assembly is the agreed prerequisite for #17** (see the
  recommended foundation sequence). Formats must converge on a single assembled
  map model, not each invent their own.
- **Pre-1.0 is the cheapest window for a breaking reorganization** — only eight
  record types and a handful of consumers exist today.
- **Reuse where layouts are identical** (Heretic == Doom) and **isolate where
  they diverge** (Hexen `Thing`/`Linedef`, all of UDMF).
- **Text (UDMF) and binary paths must not be forced into one mechanism.**
- **Every format multiplies the hostile-input surface**, so the abstraction must
  be compatible with the parser-hardening pass adopted alongside this work.

## Considered options

1. **Overload the existing generic records with a runtime format discriminant** —
   keep single `Thing`/`Linedef` types, widen them to hold every format's fields,
   and thread a `format` flag through parsing to decide which fields are valid.
2. **Per-format record modules unified by a `MapFormat` enum and a detection
   layer, all converging on the `#155` assembled-map graph** — reorganize `map`
   into `common` (shared records) plus one submodule per divergent format.
3. **Fully separate, self-contained modules (or crates) per game with no shared
   record types** — duplicate `Vertex`/`Sidedef`/`Sector`/etc. into each.

## Decision outcome

Chosen option: **Option 2 — per-format record modules unified by a `MapFormat`
enum and a detection layer.** It is the only option that both reuses the
byte-identical records (Heretic, and Hexen's six unchanged records) and cleanly
isolates the genuinely divergent pieces, while keeping a single convergence point
(the `#155` graph) for all downstream consumers.

Concretely:

### 1. Introduce a `MapFormat` enum (this ADR owns it)

```rust
/// The on-disk layout family of a single map's lumps.
///
/// Distinct from `WadKind` (container kind) and from game identity: Heretic maps
/// are `Doom`, and one IWAD may contain maps of more than one `MapFormat`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MapFormat {
    /// Classic Doom binary layout. Also used by Heretic.
    Doom,
    /// Hexen binary layout (extended `Thing`/`Linedef`, `BEHAVIOR` lump).
    Hexen,
    /// Doom 64 binary layout. Record specifics confirmed in #54.
    Doom64,
    /// UDMF text layout (`TEXTMAP` lump). Sub-flavor carried by its `namespace`.
    Udmf,
}
```

`#[non_exhaustive]` because future ports (Strife, Eternity, ZDoom-in-binary) may
add layouts. `MapFormat` is **not** merged into `WadKind`; the two axes stay
separate, and `WadKind` is unchanged by this ADR.

### 2. Detect format from a map's lump group, not from the container

Format is a per-map property. Detection operates on the lumps belonging to one
map (the map header lump such as `E1M1`/`MAP01` and its following data lumps up to
the next non-map lump) using this precedence:

1. A `TEXTMAP` lump present in the group ⇒ `MapFormat::Udmf` (the `namespace`
   line inside refines the sub-flavor for UDMF parsing).
2. Otherwise a `BEHAVIOR` lump present ⇒ `MapFormat::Hexen`.
3. Otherwise the Doom-64-specific lump signature (confirmed in #54) ⇒
   `MapFormat::Doom64`.
4. Otherwise ⇒ `MapFormat::Doom`.

The public detection entry point has the shape:

```rust
pub fn detect_map_format(group: &MapGroup) -> MapFormat;
```

where `MapGroup` is the "one map's lumps" type **defined by ADR #155**, not by
this ADR. Until #155 lands, detection is exercised against a provisional
slice-of-lumps input; the final signature binds to `MapGroup` when #155 is
accepted. Heretic is deliberately *not* a `MapFormat` value: it is
indistinguishable from Doom by map bytes alone, so game identity (if ever needed)
is tracked separately (see Non-goals).

### 3. Reorganize the `map` module by format (breaking, pre-1.0)

```text
map/
  mod.rs      // MapFormat, detect_map_format, parse_records, shared re-exports
  common.rs   // records byte-identical across formats:
              //   Vertex, Sidedef, Sector, Seg, Subsector, Node, Name8
  doom.rs     // Thing, Linedef                 (Doom + Heretic)
  hexen.rs    // Thing, Linedef                 (20 B / 16 B, + args)
  doom64.rs   // Doom 64 records                (layouts land in #54)
  udmf.rs     // text parser + typed UDMF model (lands in #57/#58)
```

This moves `crustywad::map::Thing` to `crustywad::map::doom::Thing` and
`crustywad::map::Linedef` to `crustywad::map::doom::Linedef`. Heretic (#56) reuses
`map::doom::*` with no new structs. The six records that are byte-identical across
Doom and Hexen move to `map::common` and are shared, not duplicated — so Option 3
(duplicate everything) is explicitly rejected. This is a breaking public-API
change, taken now while the surface is small.

### 4. Keep `parse_records` for all binary formats; give UDMF its own path

`parse_records::<T>` is retained unchanged and is the decoder for every binary
record type across Doom, Hexen, and Doom 64 (all satisfy
`BinRead<Args<'a> = ()>`). UDMF does **not** use it. UDMF gets a dedicated text
entry point and a dedicated error type, introduced in #57/#58:

```rust
// map::udmf (illustrative; finalized in #57/#58)
pub fn parse_udmf(text: &str) -> Result<UdmfMap, UdmfParseError>;

pub enum UdmfParseError {
    /// Lexical/grammar error with source position.
    Syntax { line: usize, column: usize, message: String },
    /// A required field or block was missing or malformed.
    Semantic { message: String },
}
```

`UdmfParseError` is separate from both `ParseError` (container-level) and
`MapParseError` (binary-record-level) because neither models a text position, and
folding text errors into a binary error enum would misrepresent both. UDMF's
float coordinates mean the `#155` assembled-map model must accommodate
floating-point geometry, not only `i16`; this ADR flags that requirement for
#155 rather than resolving it here.

### 5. Consolidate Doom-name decoding

Extract the 8-byte NUL-trim decode into one shared helper in `map::common`,
consumed by both the directory-name path (`Lump::name`) and the in-record texture
path (`Name8`). The two call sites keep their distinct policies — strict-ASCII for
directory names, lossy for texture references — but share the underlying routine
instead of duplicating it.

### 6. Strictness applies to every format

Both binary and text parsing honor the existing `Strictness::Strict` /
`Strictness::Lenient` contract (ADR-0003). For UDMF, strict rejects the first
syntactic/semantic violation; lenient recovers where well-defined and records a
warning. Each format's own implementation issue must state its strict-vs-lenient
recovery rules, consistent with ADR-0003.

## Consequences

- **Good** — Heretic (#56) becomes near-free: it reuses `map::doom::*` and adds
  only game-semantic tables (out of scope here).
- **Good** — Shared records live once in `map::common`; only genuinely divergent
  records (Hexen `Thing`/`Linedef`, Doom 64, UDMF) carry per-format code.
- **Good** — Text and binary paths are cleanly separated, each with an error type
  that fits its domain.
- **Bad / breaking** — `crustywad::map::Thing` and `::Linedef` move under
  `::map::doom`. This breaks existing imports and is a deliberate pre-1.0 cost. It
  must ship as a single reorganization PR *before* the first non-Doom format, and
  the guide/README/`CLAUDE.md` map-type references must be updated in lockstep.
- **Neutral** — `MapFormat` is `#[non_exhaustive]`; downstream `match`es must
  carry a wildcard arm, which is the intended forward-compatibility trade-off.
- **Dependency** — `detect_map_format` and any per-map API bind to the `MapGroup`
  type from ADR #155. **#155 must be accepted before this ADR's detection surface
  is finalized;** the record reorganization (items 3, 5) has no such dependency
  and can land first.
- **Hardening** — UDMF adds a text parser (a new, unbounded-input attack surface:
  nesting depth, allocation, string sizes) and Doom 64 adds graphics decode
  surface. Both must be covered by the parser-hardening pass (per-format fuzz
  targets and resource limits) adopted alongside this epic; each format PR adds
  its own fuzz target rather than deferring.
- **Write path (out of scope, flagged)** — This ADR governs *reading* formats.
  Emitting Hexen/Doom 64/UDMF is a separate write-path concern (ADR-0006 lineage)
  and is not decided here; #60 (UDMF write) will need its own design.
- **Validation** — Multi-format correctness is validated by two complementary
  mechanisms (both adopted): free-equivalent plus hand-crafted synthetic fixtures
  that run in CI, and an env-var + feature-gated local harness for real
  Hexen/Doom 64 IWADs (generalizing today's `CRUSTYWAD_FREEDOOM_DIR` setup), since
  Hexen and Doom 64 lack freely redistributable IWADs. See ADR-0010 (proptest) and
  the fixtures README; the harness generalization is tracked with the format work.

## Pros and cons of the options

### Option 1 — overload generic records with a runtime discriminant

- Good, because it avoids any breaking rename of `map::Thing` / `map::Linedef`.
- Good, because there is a single record type per concept to import.
- Bad, because a `Thing` that must hold both the Doom (10 B) and Hexen (20 B)
  field sets carries fields that are meaningless in one format, inviting
  use-of-invalid-field bugs the type system no longer prevents.
- Bad, because it cannot express UDMF at all — UDMF is text with float
  coordinates, not a widened binary record.
- Bad, because `binrw`'s derived `BinRead` wants a fixed layout; a
  discriminant-widened record needs hand-written conditional parsing, discarding
  the main benefit of the `binrw` approach (ADR-0002).

### Option 2 — per-format modules + `MapFormat` + detection (chosen)

- Good, because byte-identical records are shared (`map::common`) and divergent
  ones are isolated, matching the real shape of the format landscape.
- Good, because each format has a type set that models exactly its own fields, so
  the compiler rejects cross-format field access.
- Good, because it accommodates UDMF's separate text path without contorting the
  binary types.
- Bad, because it is a breaking reorganization of the public `map` module.
- Bad, because callers must map-detect per map and `match` a `#[non_exhaustive]`
  enum.

### Option 3 — fully separate modules/crates per game, no shared types

- Good, because each game is fully self-contained with zero cross-coupling.
- Bad, because `Vertex`/`Sidedef`/`Sector`/`Seg`/`Subsector`/`Node` are identical
  across Doom, Heretic, and Hexen, so this duplicates six record types (and their
  tests and docs) several times over.
- Bad, because downstream consumers (the `#155` graph, a future renderer) would
  need per-game conversions for records that are literally the same bytes.

## More information

- Tracking spike: #53. Blocks #54 (Doom 64), #55 (Hexen), #56 (Heretic),
  #57–#60 (UDMF spike/read/convert/write).
- Prerequisite: ADR for #155 (map-graph assembly) — defines the `MapGroup` /
  assembled-map model that `detect_map_format` and per-map APIs bind to, and must
  accommodate UDMF floating-point geometry.
- Related ADRs: ADR-0002 (binrw and typed errors), ADR-0003 (default to strict
  parsing), ADR-0004 (parse API and safety contracts), ADR-0006 (write design),
  ADR-0009 (`cargo-fuzz` harness — extend per format), ADR-0010 (proptest),
  ADR-0013 (`lump_by_name` — namespace-scoped lookup becomes relevant as
  graphics/textures formats land).
- Source anchors: `crates/crustywad/src/lib.rs` (`WadKind`, `Lump::name`,
  `decode_name`), `crates/crustywad/src/map.rs` (existing Doom records, `Name8`,
  `parse_records`).
- Revisit condition: reopen if a target format needs runtime-parameterized record
  parsing (breaking the `BinRead<Args = ()>` assumption in item 4), or if a port
  requires distinguishing Heretic from Doom by map bytes (breaking the "Heretic ==
  Doom format" assumption in item 2).
