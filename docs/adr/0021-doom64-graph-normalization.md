# ADR-0021: Doom 64 graph normalization

- **Status:** Accepted
- **Date:** 2026-07-14
- **Deciders:** @masriamir
- **Tracking issue:** https://github.com/masriamir/crustywad/issues/243

## Context and problem statement

ADR-0018 (#54) delivered Doom 64 as a standalone raw-record module —
`map::doom64` with `read_doom64_map` and the free detection helper
`is_doom64_map_lump` — and explicitly deferred `MapFormat::Doom64`,
`map_groups` visibility, and a `Map::assemble` path "to whenever Doom 64 is
normalized into the `Map` graph." That deferral's cost is now measured:
against a retail Steam (2020 KEX) `DOOM64.WAD`, `read_doom64_map` reads all 40
nested-WAD maps strict-clean, while `Wad::map_groups()` finds **0** of them (a
nested-WAD map lump has no classic data-lump successor, so `marker_run_end`
never fires). The generic map API and the Doom 64 entrypoint disagree about
whether the WAD contains maps at all, every consumer must special-case Doom 64
(the #254 sweep test carries a dedicated sniff loop today), and `cwad info`'s
name-heuristic map listing diverges from the library (#253 tracks unifying
them, and is blocked on this ADR).

Normalization was deferred for a concrete reason (ADR-0018 §4): Doom 64
records carry **`u16` texture/flat indices** and **colored-lighting IDs**
(`Sector.colors: [u16; 5]` indexing the per-map `LIGHTS` palette,
`Light { r, g, b, tag, unknown }`), which have no resolvable meaning until a
texture/graphics layer exists (v0.5.0, #156/#157). The graph's texture fields
are `String`s (`MapSidedef.upper/lower/middle`,
`MapSector.floor_flat/ceiling_flat`) and its lighting field is a scalar
(`MapSector.light: i32`); Doom 64 has neither names nor a scalar light. This
ADR decides what "normalized" means before that layer exists.

The remaining record fields map cleanly onto the existing graph:
`doom64::Vertex` is 16.16 fixed point (`i32`), which widens losslessly into
`MapVertex`'s `f64`; `doom64::Linedef` has `u32` flags (fits
`MapLinedef.flags: u32`), a `u16` `special` + `u16` `tag` (the Doom
`Special`/`args[0]` shape), and `u16` sidedef references; `doom64::Thing`
carries `z` and `id` (`i16`), for which `MapThing.height`/`MapThing.id`
already exist (Hexen/UDMF use them).

## Decision

### 1. Detection and grouping — amends ADR-0018 §3

A directory lump is a Doom 64 map group **iff** its name matches `MAPxx`
(`MAP` + two ASCII digits) **and** its data passes `is_doom64_map_lump`
(nested-WAD `IWAD`/`PWAD` magic, non-empty). Requiring both keeps an arbitrary
resource lump whose bytes happen to start with WAD magic from becoming a
phantom map, and an empty classic `MAPxx` *marker* lump from matching (empty
data fails the magic check). This is the same dual condition the #254 sweep
test already applies.

Such a lump yields a `MapGroup` with `marker_index` pointing at the lump and
`data_indices` empty — the existing struct shape is unchanged.
`detect_map_format` inspects the marker lump's data magic **first** (the
existing `TEXTMAP`/`BEHAVIOR` checks examine data lumps, of which a Doom 64
group has none) and returns the new `MapFormat::Doom64` variant. `MapFormat`
is `#[non_exhaustive]`, so the variant is additive; ADR-0018's rejected
"Option 2" objection — a `MapFormat` variant that cannot assemble — no longer
applies because this ADR ships the assemble arm with it.

### 2. The assemble arm

`Map::assemble_with_options` on a `Doom64` group calls `read_doom64_map`
(container parsed strictly in both modes, per ADR-0018) and normalizes:

| Doom 64 record field | Graph target | Rule |
|---|---|---|
| `Vertex.x/y: i32` (16.16 fixed) | `MapVertex.x/y: f64` | `value / 65536.0` — lossless (32 significant bits into f64's 52-bit mantissa) |
| `Linedef.flags: u32` | `MapLinedef.flags: u32` | direct |
| `Linedef.special/tag: u16` | `MapLinedef.special: Special` | `special` widens; `tag` → `args[0]` (Doom rule) |
| `Linedef.sidefront/sideback: u16` | `MapLinedef.right/left: Option<SidedefIdx>` | via `resolve_binary_side` (ADR-0020); Doom 64's use of the `0xffff` "no side" sentinel is **verified against Doom64 EX source during implementation**, not assumed |
| `Sidedef.x_offset/y_offset: i16` | `MapSidedef.x_offset/y_offset: i32` | widen |
| `Sidedef.upper/lower/middle: u16` | `TextureRef::Index` (§3) | raw index |
| `Sidedef.sector: u16` | `MapSidedef.sector: SectorIdx` | `resolve_required` |
| `Sector.floor_height/ceiling_height: i16` | `i32` | widen |
| `Sector.floor_tex/ceiling_tex: u16` | `TextureRef::Index` (§3) | raw index |
| `Sector.colors: [u16; 5]` | `MapSector.colors: Option<[LightIdx; 5]>` (§4) | each resolved against the lights arena |
| `Sector.special/tag: u16` | `i32` | widen |
| `Sector.flags: u16` | **new** `MapSector.flags: u32` | raw carry; `0` for every other format (mirrors `MapLinedef.flags`) |
| `Thing.x/y: i16` | `f64` | widen |
| `Thing.z: i16` | `MapThing.height: f64` | widen |
| `Thing.angle: i16` | `MapThing.angle: u16` | wrapped modulo 360 (the UDMF-path rule) |
| `Thing.type_id: i16` | `MapThing.type_id: u16` | negative → strict error / lenient warn (no valid doomednum is negative; rule confirmed against Doom64 EX during implementation) |
| `Thing.flags: i16` | `MapThing.flags: u32` | **translated** into the graph's normalized Doom/Boom layout (ADR-0019 §2) via a bit table **verified against Doom64 EX / KEX source**; Doom 64-only bits with no slot drop, exactly as Hexen's dormant/class bits do |
| `Thing.id: i16` | `MapThing.id: i32` | widen |
| `lights: Vec<Light>` | **new** `Map::lights()` arena of `MapLight` (§4) | `r`/`g`/`b`/`tag` carried; `Light.unknown` (tentative semantics) stays raw-only in `Doom64Map` |
| `segs`/`subsectors`/`nodes`/`reject`/`blockmap` | not normalized | the same deferral the classic path makes — BSP traversal is #204 |
| `leafs`/`macros` | not normalized | #244 / #245 |

`Doom64Map` remains public as the full-fidelity pre-normalization form — the
`UdmfMap` precedent (ADR-0017 §1).

`MapSector.light` keeps its classic scalar meaning and is `0` for Doom 64
maps (documented on the field): Doom 64 has no scalar light level, and
synthesizing one from the color palette would fabricate data no engine
computes.

### 3. `TextureRef` — the graph's texture fields become format-honest

```rust
/// A texture or flat reference in the assembled graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextureRef {
    /// A texture name (Doom/Hexen 8-byte lump name, or a UDMF string).
    Name(String),
    /// A Doom 64 texture/flat table index — resolvable to a texture identity
    /// once the texture layer (v0.5.0) exists.
    Index(u16),
}
```

The five texture fields — `MapSidedef.upper/lower/middle`,
`MapSector.floor_flat/ceiling_flat` — change from `String` to `TextureRef`.
Classic/Hexen/UDMF assembly always produces `Name`; Doom 64 always produces
`Index`. This is a **breaking pre-1.0 change**, taken in the same cheap window
as ADR-0020's `MapLinedef.right` change and for the same reason: the
alternative representations (parallel half-empty fields, or indices encoded
into strings) misrepresent the data or push a parsing obligation onto every
consumer.

### 4. Colored lighting — full carry

```rust
/// A zero-based index into [`Map::lights`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LightIdx(pub usize);

/// A normalized Doom 64 colored-lighting palette entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapLight {
    /// Red channel (`0`–`255`).
    pub r: u8,
    /// Green channel (`0`–`255`).
    pub g: u8,
    /// Blue channel (`0`–`255`).
    pub b: u8,
    /// Small identifier (observed `0`–`2` in retail data; tentative semantics).
    pub tag: u8,
}

impl Map {
    /// The map's colored-lighting palette; empty for non-Doom 64 maps.
    pub fn lights(&self) -> &[MapLight];
}
```

`MapSector` gains `colors: Option<[LightIdx; 5]>` — `Some` for Doom 64 maps,
`None` for every other format. Each of the five IDs is cross-validated against
the lights arena with the standard resolver pattern: out of range is a strict
`MapAssembleError::DanglingReference` / lenient clamp-to-0 plus warning
(`resolve_required` semantics — a Doom 64 sector's color slots are not
optional references). The **per-slot meaning** of the five entries is
documented from Doom64 EX source during implementation, not asserted here;
`doom64::Sector::colors` itself records the semantics as "per Doom64 EX".
`Light.unknown` (a `u16` whose high byte is always zero in retail data,
meaning unconfirmed) is deliberately **not** normalized — a consumer needing
it reads `Doom64Map`.

### 5. Write and conversion boundary — structured reject, both modes

Neither writer can faithfully express a Doom 64-sourced `Map`: there is no
Doom 64 binary writer, `TextureRef::Index` has no name until the texture layer
exists, and colored lighting has no classic/UDMF slot. Silently dropping the
format's defining visual data would corrupt maps, so this is not a
lenient-recoverable defect:

- `write_udmf`/`add_udmf_map` and `write_doom_map`/`add_doom_map` return a new
  structured error — `UdmfWriteError::UnsupportedSourceFormat { format: MapFormat }`
  and `DoomWriteError::UnsupportedSourceFormat { format: MapFormat }` — for a
  `MapFormat::Doom64` map in **both** strictness modes.
- Defensively, a `TextureRef::Index` reaching a writer in a *non*-Doom 64 map
  (`Map` fields are public, so hand-construction can produce this) is also a
  both-modes error: `UnresolvedTextureIndex { block: &'static str, field:
  &'static str, index: usize }` on each writer's error enum.
- `cwad convert` reports the error per map, consistent with ADR-0019's
  conversion-failure surfacing.

Lifting the rejection — resolving `Index` to names during conversion — is
explicitly the v0.5.0 texture layer's job, extending ADR-0019's reversibility
inventory when it lands.

## Considered options

### Texture representation
1. **`TextureRef` enum (chosen)** — honest model of the format axis; one
   breaking change in the pre-1.0 window; writers get a typed value to reject.
2. **Parallel `Option<u16>` index fields** — two half-empty representations
   per field; every consumer must know which side to read per format.
3. **Stringified indices (`"#0042"`)** — zero API change but fabricates fake
   names, forces parsing at every consumer, and a careless writer would emit
   the marker string into a real WAD.

### Lighting
1. **Full carry (chosen)** — `colors` + `lights()` arena; assembly stays
   lossless for the format's defining feature; the viewer epic (#64) gets one
   data path.
2. **Raw-only (`Doom64Map`)** — smallest change, but `Map::assemble` becomes
   lossy for exactly the data a renderer needs, forcing a second data path.
3. **Synthesize a scalar `light`** — fabricates a value no engine computes and
   still loses the colors.

### Writer policy
1. **Structured reject, both modes (chosen)** — unrepresentable content is an
   error, not a warning; matches ADR-0003's contract that lenient mode
   recovers *defects*, and ADR-0019 §4's precedent that output must not be
   silently degraded.
2. **Lenient best-effort** — placeholder textures produce loadable-but-mangled
   output; data corruption for an I/O-fidelity library.
3. **Defer** — `MapFormat::Doom64` forces the writers' match arms to do
   *something*; unspecified means accidental behavior ships.

## Consequences

- Doom 64 maps become visible to `map_groups`/`map_group`,
  `detect_map_format`, and `Map::assemble` — one API for all four formats.
  #253 (`cwad info` unification) is unblocked; the #254 sweep test drops its
  Doom 64 sniff loop in favor of the generic path (asserting 40 groups in the
  retail WAD).
- **Breaking changes** (pre-1.0): the five texture fields become `TextureRef`;
  `MapSector` gains `colors` and `flags`; `Map` gains `lights()`;
  `MapThing.flags` gains a Doom 64 translation; writers gain new error
  variants. `MapFormat::Doom64` itself is additive.
- Hardening (ADR-0016): the assemble arm is iterative and `O(input)` (record
  counts already bounded by `read_doom64_map`; the lights arena and color
  validation are linear). The assembly fuzz target extends to route
  nested-WAD map bytes through `Map::assemble` with the no-panic and
  output-size oracles; both strictness modes covered.
- ADR-0018 §3 (detection kept out of `detect_map_format`) and §4 (no
  `MapFormat::Doom64`, no assemble path) are superseded by this ADR;
  ADR-0018's record layouts, nested-WAD reading, and `map::doom64` module
  boundary are unchanged. ADR-0015's field inventory is extended
  (`TextureRef`, `colors`, `flags`, `lights`); its arena/index-newtype model
  is unchanged.

## More information

- Every Doom 64 field type above is anchored to `map/doom64.rs` struct
  definitions (`Vertex.x/y: i32`, `Sector.colors: [u16; 5]`,
  `Light { r, g, b, tag: u8, unknown: u16 }`, …), not recalled from memory.
  Two facts are deliberately left to implementation-time verification against
  Doom64 EX / KEX source, per the project's format-constants rule: the
  `0xffff` side-sentinel convention and the thing-flags bit table (plus the
  per-slot meaning of `Sector.colors`).
- Relates to: ADR-0018 (superseded in part, as noted), ADR-0015 (graph model,
  extended), ADR-0017 (`UdmfMap` full-fidelity precedent), ADR-0019
  (normalized thing-flag contract; conversion inventory), ADR-0020
  (`resolve_binary_side`), ADR-0016 (hardening), #204/#244/#245 (deferred
  lumps), #253 (unblocked), v0.5.0 texture layer (#156/#157 — resolves
  `TextureRef::Index`).
