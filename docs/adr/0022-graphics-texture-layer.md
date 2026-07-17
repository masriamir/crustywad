# ADR-0022: The graphics & texture layer — formats, staging, and the Doom 64 name-hash

- **Status:** Accepted
- **Date:** 2026-07-16
- **Deciders:** @masriamir
- **Tracking issue:** https://github.com/masriamir/crustywad/issues/271

## Context and problem statement

The roadmap's v0.6.0 milestone opens the graphics/texture layer, and both of its
draft implementation issues (#156, #157) say only "needs an ADR" — there is no
recorded decision for the on-disk formats, the API shape, or how the layer
interacts with the map graph. It also interacts with existing decisions:
ADR-0021 §5 rejects any `Map` sourced from a `MapFormat::Doom64` group at the
writer boundary, in both strictness modes, specifically because
`TextureRef::Index` "has no name until the texture layer exists" — that gate
is this layer's job to resolve, and ADR-0021 states so explicitly. The graph's
current `TextureRef::Index` rustdoc (`crates/crustywad/src/map/graph.rs`) also
describes the Doom 64 `u16` as "a Doom 64 texture/flat table index", implying a
plain array index; the research below shows that reading is not the on-disk
truth.

This spike (#271) ran two engine-source research passes plus one empirical
validation, and this ADR records their decisions before implementation begins,
per the project's ADR-before-code planning process.

- **Classic** (Chocolate Doom — `v_patch.h`, `r_data.c`, `w_wad.c`, `v_video.c`,
  `i_video.c`, `st_stuff.c`, `r_main.c`/`.h`, `r_local.h`): a full inventory of
  picture, PLAYPAL, COLORMAP, PNAMES, TEXTURE1/2, flat, and namespace-marker
  formats, plus the composition algorithm — and, most load-bearing for this
  ADR, an inventory of every unchecked count/offset/index the vanilla renderer
  trusts (below, one row per finding).
- **Doom 64** (Doom64 EX — the classic 2.5 C engine plus the master C++
  rewrite plus the `wadgen` ROM→PC converter): the PC WAD's texture/sprite/gfx
  lumps are standard PNGs, and the `u16` in sidedef/sector records is a
  truncated rolling hash of the texture's lump name, not an index.
- **Empirical validation** against the user's retail KEX `RETAIL/DOOM64.WAD`
  (1668 lumps): **503/503** texture-section lumps carry PNG magic (and 50/50
  sampled sprites); the reimplemented name-hash, resolved first-match-in-disk-
  order, resolves **82/82** distinct texture references in `MAP01` with zero
  misses — confirming the hash algorithm and resolution order against real
  data, not just engine source.

## Decision

### 1. `TextureRef::Index` is formally a name hash

Doom 64's on-disk `mapsidedef_t`/`mapsector_t` (`doomdata.h`) carry `u16`
`toptexture`/`bottomtexture`/`midtexture`/`floorpic`/`ceilingpic` fields. These
are **not** array indices into a texture table; they are `W_HashLumpName`
truncated to 16 bits — a rolling hash of the texture's lump name, identical in
both the classic 2.5 engine (`doom64/wadfile/w_wad.c`) and the master rewrite
(`include/imp/Wad`):

```c
hash = 1315423911;
for (each of up to 8 characters, uppercased) {
    hash ^= (hash << 5) + c + (hash >> 2);
}
result = hash & 0xFFFF;
```

Resolution at map load hashes every lump name in the texture section
(`T_START+1..T_END-1`, on-disk order) into a table, then looks up each
reference by **first match in that same disk order** — the engine's own
`P_InitTextureHashTable`/`P_GetTextureHashKey` do exactly this, and the master
rewrite's `std::map`-based reimplementation preserves the same first-match
semantics. **On a hash miss the engine silently falls back to texture ordinal
0** — a wrong-but-plausible texture, indistinguishable from a real reference to
the first texture in the section. This is the exact anti-pattern this ADR
rejects for crustywad: a miss must be an explicit `None` plus a warning, never
a silent substitute value.

**Collisions are structurally possible, not observed**: a 16-bit hash over up
to 8 uppercase characters has a large but finite domain, so the engine's own
tie-break rule (first match in disk order) is load-bearing for any WAD that
does produce one — a resolver that picked, say, last-match or an arbitrary
match would diverge from the engine on such a WAD. The empirical pass across
the retail `DOOM64.WAD`'s `MAP01` observed zero collisions: all 82 distinct
references resolved uniquely (82/82) with zero misses using exactly this
first-match rule (samples: `32→SDFLTAB`, `111→"?"` — a texture literally named
`?`, `4098→SFLATAE`), which validates the algorithm and the tie-break rule
against real data rather than engine source alone.

**Documentation correction.** `TextureRef::Index`'s current rustdoc reads:

> A Doom 64 texture/flat table index — resolvable to a texture identity once
> the texture layer (#156/#157) exists.

This describes a plain index; it is a name hash. The doc comment (and its
three call-sites on `MapSidedef.upper/lower/middle` and
`MapSector.floor_flat/ceiling_flat`) must be corrected to state the true
on-disk meaning — a truncated rolling hash of the referenced lump's name,
resolved first-match-in-disk-order over the texture section, "index" being a
historical/engine-internal name for the field rather than a description of
its encoding.

**The variant name `Index` stays.** Renaming it (e.g. to `Hash`) is pure
churn against every existing call-site and the ADR-0021 API surface that
shipped it, for a cosmetic gain; the semantics note travels with the doc
comment instead. This is a deliberate churn-vs-clarity tradeoff, not an
oversight.

### 2. Single-WAD namespace/section API on `Wad`

A new API — first-class enumeration of marker-delimited sections of a
**single** WAD's directory — is the prerequisite for everything else in this
ADR (issue 1, staging below) because both the classic composition tier and
Doom 64's hash-resolution table need "every lump name between these two
markers," and today that logic does not exist anywhere in the crate.

**Marker inventory**, from the two research passes:

- **Classic:** `F_START`/`F_END` (flats), `S_START`/`S_END` (sprites),
  `P_START`/`P_END` (patches) — plus nested sub-namespace pairs
  `F1_START`/`F1_END`, `F2_START`/`F2_END`, `P1_START`/`P1_END`, and
  `P2_START`/`P2_END` (with third pairs `F3_START`/`F3_END` and
  `P3_START`/`P3_END` in DOOM2.WAD and its BFG-edition variant),
  verified against the retail collection: DOOM/DOOM2/Heretic/Hexen IWADs.
  Sprites do **not** nest in any retail IWAD — no `S1_`/`S2_` pair appears
  anywhere in the collection. Boom additionally recognizes a doubled-first-letter
  alias rule for single-character prefixes (`FF_`/`PP_`/`SS_` aliasing
  `F_`/`P_`/`S_`), per PrBoom+'s `w_wad.c` `IsMarker`:
  > doubled first character test for single-character prefixes only
  > FF_\* is valid alias for F_\*, but HI_\* should not allow HHI_\*
- **Doom 64:** `S_START..END` (sprites), `T_START..END` (world textures —
  walls *and* flats share this one pool, see §4), `DS_START..END` (digital
  sound); `G_START..END` appears in Doom64 EX's own `wadgen` emission order
  but is **absent from the retail KEX WAD** (confirmed empirically) and must
  be tolerated as optional; trailing `CHECKSUM` (an MD5 of the directory) and
  `ENDOFWAD`, both zero-size like the section markers.

**Strict/lenient contract.** Both engines resolve a section's extent by naive
subtraction of two independently-looked-up marker positions
(`numflats = lastflat - firstflat + 1` in Chocolate Doom;
`numtextures = (t_end - t_start) + 1` in Doom64 EX's `InitWorldTextures`) with
**no guard against a missing, inverted, or duplicated marker** — a missing
name is fatal deep inside the name-lookup helper (`I_Error`/`W_GetNumForName`
abort) in both engines, and an inverted pair silently yields a negative count
that then drives further unchecked arithmetic. This ADR's API does the
opposite: a missing, inverted, or unpaired marker is a
`Strictness::Strict` error and a `Strictness::Lenient` warning (with a
best-effort empty/skipped section), never a naive subtraction whose sign is
never checked.

**Non-goal: multi-WAD overlay.** Both engines' load-order precedence rules for
markers spread across *multiple* loaded files (`w_wad.c`'s backward linear
scan comment "so patch lump files take precedence" — i.e. last-loaded wins)
are explicitly out of scope. `crustywad` has no multi-file load model today;
that semantic belongs to the editor epic's future lump/resource manager
(#65), and a cross-link comment was posted on #65 on 2026-07-16 recording the
deferral.

### 3. Classic decode: three dependency-free tiers in core

Classic graphics decode ships in the core crate, not behind a feature flag —
the same precedent as map parsing (no format-specific gate for a format this
central to the WAD ecosystem). It is organized into three tiers, each
building on the last.

**Tier 1 — raw typed lumps**, byte layouts from `classic-research.md`:

- **`Picture`** (patches and sprites): an 8-byte header — four little-endian
  `i16` fields `width`, `height`, `leftoffset`, `topoffset` — followed by
  exactly `width` little-endian `i32` column offsets, each counted **from the
  start of the lump** (not from the header's end; vanilla's
  `R_GenerateComposite` dereferences `(byte*)patch + columnofs[x]` directly).
  Each column is a chain of posts: `topdelta` (`u8`; `0xFF` terminates the
  chain), `length` (`u8`), one padding byte, `length` data bytes, one trailing
  padding byte. Vanilla's `topdelta` is plain (non-cumulative): a post's
  vertical position is `originy + topdelta`, with no DeePsea-style
  "tall patch" cumulative-offset handling in vanilla.
- **`Playpal`**: `N × 768` bytes (256 entries × RGB), with **no count field on
  disk** — the count is derived from the lump's length (`len / 768`); vanilla
  never validates the lump against the number of palette indices it
  hardcodes (up to 14). Strict mode rejects a length that is not an exact
  multiple of 768; lenient mode truncates the remainder and warns.
- **`Colormap`**: `32 × 256` bytes (`NUMCOLORMAPS` is a vanilla compile-time
  constant, not a value read from the lump) = 8192 bytes exactly; vanilla
  never checks the lump's actual size against this. Strict mode requires
  exactly 8192 bytes; lenient mode pads-or-truncates (semantics finalized at
  implementation time, informed by #256's virtual zero-pad precedent for a
  short lump).
- **`Pnames`**: a little-endian `i32` count, then `count × 8`-byte NUL-padded
  names. Vanilla reads the count from a lump that might be under 4 bytes,
  never checks `4 + count * 8 <= lump length`, and sizes an allocation from
  the unchecked count directly — an unbounded-allocation vector this layer
  must close.
- **`TextureX`** (TEXTURE1/TEXTURE2): a little-endian `i32` count, then
  `count` little-endian `i32` offsets (absolute from the lump start), each
  pointing at a 22-byte `maptexture_t`: `name` (8 bytes), a **dead** `i32`
  field historically named `masked`, `i16` `width`, `i16` `height`, a
  **second dead** `i32` field (historically `columndirectory`, an obsolete
  disk offset never read), and `i16` `patchcount`; followed by
  `patchcount × 10`-byte `mappatch_t` records: `i16` `originx`, `i16`
  `originy`, `i16` `patch` (an index into `PNAMES`), and two more dead `i16`
  fields (`stepdir`, `colormap`, both historical and unused). Vanilla
  validates each texture's offset **only against the whole-lump length**, not
  against `offset + 22 + patchcount * 10` — the struct's full extent — so a
  crafted offset near the end of the lump still reads out of bounds; and
  `mappatch_t.patch` indexes the resolved patch-lookup table with **no bounds
  check against the patch count**, only checking the *result* of the name
  lookup for a `-1` miss.
- **`Flat`**: a raw 4096-byte (64×64) blob — an assumption made only at
  render time in vanilla, never validated against the lump's actual length
  at load. Strict mode requires exactly 4096 bytes; lenient mode warns and
  proceeds with what is present.

**Tier 2 — composition**, reimplementing the contract of vanilla's
`R_GenerateComposite`/`R_GenerateLookup` as an indexed-pixel-plus-coverage-mask
output rather than vanilla's lazy per-column cache:

- **Single-patch shortcut**: a column touched by exactly one patch is served
  directly from that patch's post data — vanilla's `collump[x]` pointing at
  the patch lump with no composite buffer allocated for it; this layer
  reproduces that equivalence rather than always materializing a full
  composite.
- **Multi-patch assembly**: a column touched by more than one patch is copied
  post-by-post into a composite buffer, exactly mirroring vanilla's
  accumulate-then-copy behavior, including its **horizontal clamping**
  (a patch's placement is clamped so `x1 < 0` clamps to `0` and `x2 > width`
  clamps to `width` before any column is touched).
- **Medusa policy.** Vanilla's `R_GenerateLookup` handles a texture column
  with **no contributing patch** by printing "column without a patch" and
  **returning early from the entire function**, leaving every later column's
  composite state uninitialized — a silent, partial, engine-visible bug
  (the well-known "Medusa effect"). The stricter alternative, an `I_Error`
  abort, is present in the source but **commented out**. This layer treats
  the Medusa case as a `Strictness::Strict` error and a
  `Strictness::Lenient` warning-with-hole (the column decodes with an
  explicit gap rather than either aborting the whole texture or silently
  leaving other columns corrupt) — deliberately better than either of
  vanilla's two behaviors.

**Tier 3 — RGBA8 convenience**: palette application for pictures, flats, and
composed textures, converting indexed-pixel-plus-palette data into `Vec<u8>`
RGBA8 for consumers that do not want to manage palette lookups themselves.

### 4. Doom 64 resolution in core, and the ADR-0021 §5 gate lift

The name-hash function (§1) and a resolution table built by hashing every
lump name in the texture section (§2) into a first-match-in-disk-order lookup
ship in the core crate. A resolution miss is an explicit `None` plus a
warning — never the engine's silent fallback to texture ordinal 0.

This resolution table feeds `TextureRef::Index` → name lookup, which **lifts
the convert gate ADR-0021 §5 imposed**. ADR-0021 §5's exact wording:
`write_udmf`/`add_udmf_map` and `write_doom_map`/`add_doom_map` "return a new
structured error — `UdmfWriteError::UnsupportedSourceFormat { format:
MapFormat }` and `DoomWriteError::UnsupportedSourceFormat { format: MapFormat
}` — for a `MapFormat::Doom64` map in **both** strictness modes," and:
"Lifting the rejection — resolving `Index` to names during conversion — is
explicitly the v0.5.0 texture layer's job, extending ADR-0019's reversibility
inventory when it lands." (The quote is verbatim; its "v0.5.0" predates the
2026-07-15 milestone renumbering that made v0.5.0 the map-domain closeout —
the texture layer it refers to is this ADR's v0.6.0 scope.) This ADR performs
that amendment:
once every `TextureRef::Index` on a Doom 64-sourced map resolves to a name via
the hash table, writers accept the map instead of unconditionally rejecting
it.

The gate's other half — colored lighting (`MapSector.colors`,
`Map::lights()`, ADR-0021 §4) — has no classic or UDMF slot and is **not**
resolved by this ADR; it follows ADR-0019's established three-tier data-loss
policy instead: `Strictness::Strict` refuses a map with unrepresentable
colored lighting, `Strictness::Lenient` converts with a best-effort mapping
and warns. The exact target-format mapping (which classic/UDMF field, if any,
approximates a colored-lighting entry) is deliberately left to the
implementation issue (staging #4, below) rather than decided here.

### 5. Doom 64 pixel decode behind an optional `doom64-gfx` feature

Doom 64's texture, sprite, and gfx lumps are complete standard PNG files
(palette-type `IHDR`, embedded `PLTE` of up to 16 rows of 16 colors for
runtime palette variants, an optional `tRNS`, and sprite pixel offsets
carried in a private `grAb` chunk — the same convention ZDoom uses). Decoding
them pulls in a PNG library, so this is the one part of the graphics layer
gated behind an optional feature, following the `mmap` feature's precedent
for optional dependencies (ADR-0005 lineage): the `png` crate.

The feature decodes to **indexed pixels + the PNG's `PLTE` rows + the `grAb`
offset pair** — the data crustywad's own composition/palette-application
tiers need — not a fully-decoded RGBA image (that composition happens in
core, per §3 tier 3, over whichever palette a caller selects).

This layer imposes its **own** `Limits` dimension caps, independent of
whatever internal limits the `png` crate itself enforces. This is a direct
response to a confirmed engine defect: Doom64 EX's `I_PNGReadData` trusts
`libpng`-reported dimensions to size a `Z_Calloc(rowSize * height)`
allocation with **no engine-side cap**, and separately **hard-crashes the
process** (`I_Error`, an abort) on an RGB8 PNG carrying a `tRNS` chunk it does
not expect. Neither behavior is acceptable for a library: an oversized
dimension is a bounded, rejectable error against `Limits`, not an unbounded
allocation; and a `tRNS`-bearing PNG the decoder dislikes is a recoverable
error (or lenient warning), never a panic or abort.

The feature-flag four-place sync rule (`docs/guide/src/features.md`, this
file's summary table, `.github/copilot-instructions.md`, `README.md`) applies
when `doom64-gfx` actually lands with the implementation issue (staging #5)
— not to this ADR, which only names the feature.

### 6. Hardening (ADR-0016) applies wholesale

Every unchecked count, offset, or index surfaced by the two research passes
becomes a bounded, validated, both-strictness-modes-tested requirement, per
ADR-0016's per-PR checklist:

| Data | Engine trust point | This layer's requirement |
|---|---|---|
| `Picture` column offsets | Dereferenced against the lump with no bound (`R_GenerateComposite`) | Bounds-checked against lump length before use; strict error / lenient skip-with-warning |
| Post-chain termination | Walked until `0xFF` with no upper bound | Bounded by remaining lump bytes; malformed (missing terminator) is an error/warning, not an unbounded walk |
| PNAMES count | Sizes an allocation and a loop directly, unchecked | Validated against `4 + count * 8 <= lump length` before any allocation |
| `mappatch_t.patch` index | Indexes the patch-lookup table with no bounds check | Bounds-checked against the patch/PNAMES count; out-of-range is a strict error / lenient warning |
| TEXTUREx offsets/`patchcount` | Offset checked only against whole-lump length, not struct extent; `patchcount` sizes allocation/loop uncapped | Each texture's full extent (`offset + 22 + patchcount * 10`) validated against the lump; `patchcount` bounded by remaining lump bytes |
| PLAYPAL / COLORMAP / flat size | Assumed exact length, never checked | Exact-length check in strict mode; defined truncate/pad/warn behavior in lenient mode |
| Marker-pair sanity | Naive subtraction of two independent lookups, no guard on missing/inverted pairs | Explicit existence and ordering check before computing a section extent (§2) |
| PNG dimensions (`doom64-gfx`) | Trusted for allocation sizing with no engine-side cap | Bounded by this layer's own `Limits`, independent of the `png` crate's internal caps |

**Fuzz-target staging**, one per surface (ADR-0016 §3's per-format pattern,
each landing in the same PR as the surface it covers):

1. The picture parser (header + column offsets + post chains).
2. PNAMES + TEXTUREx + composition (the cross-referencing surface, since
   `mappatch_t.patch` → PNAMES is exactly the kind of index ADR-0016 targets).
3. The PNG decode path, gated behind `doom64-gfx`.

**Bounded-work statement.** Composition (tier 2, §3) is `O(texture area +
input)` — proportional to the pixels actually produced plus the patch data
consumed — replacing vanilla's ad hoc ">64k composite" `I_Error` (the one
size bailout vanilla has, r_data.c) with this layer's explicit `Limits`
mechanism (ADR-0016 §2's precedent), rather than a single hardcoded
threshold.

## Staging — the five v0.6.0 implementation issues

| # | Issue | Depends on |
|---|---|---|
| 1 | #280: `Wad` namespace/section API | — |
| 2 | #156 (re-scoped): classic pictures, flats, PLAYPAL/COLORMAP + picture/flat RGBA | #280 |
| 3 | #157 (re-scoped): PNAMES/TEXTUREx, composition, texture RGBA | #156 |
| 4 | #281: Doom 64 texture-name resolution + ADR-0021 §5 convert-gate lift | #280 |
| 5 | #282: Doom 64 PNG pixel decode (`doom64-gfx` feature, `png` crate) | #281 |

Issue 4 (#281) is deliberately cheap and can land immediately after #280, ahead of
#156/#157 — it closes the oldest user-visible wart (Doom 64 maps rejected at
`convert`) without waiting on the full classic decode stack. All five issues
carry the ADR-0016 hardening checklist individually.

**API sketches** (staging-level signatures; exact type names and error enum
shapes are each implementation issue's latitude — this ADR fixes only the
staging vocabulary and the strict-`Result`/lenient-`&mut Vec<Warning>` idiom
ADR-0003 and the map layer (`Map::parse`, `assemble_with_options`) already
establish):

```rust
// §2 (issue 1)
impl Wad {
    /// Enumerates marker-delimited sections of this WAD's own directory.
    /// Refined in issue 1.
    pub fn sections(&self) -> Vec<Section>; // Section { kind: SectionKind, lumps: Range<usize> }
}

// §3 tier 1 (issue 2) — parse-from-bytes constructors per lump type
impl Picture {
    /// Refined in issue 2.
    pub fn parse(
        bytes: &[u8],
        strictness: Strictness,
        warnings: &mut Vec<GfxWarning>,
    ) -> Result<Picture, GfxError>;
}

// §3 tier 2/3 (issue 3)
/// Refined in issue 3.
pub fn compose_texture(
    def: &TextureDef,
    pnames: &Pnames,
    patches: impl Fn(&str) -> Option<Picture>,
    strictness: Strictness,
    warnings: &mut Vec<GfxWarning>,
) -> Result<ComposedTexture, GfxError>;

impl ComposedTexture {
    /// Refined in issue 3.
    pub fn to_rgba(&self, palette: &Palette) -> Vec<u8>;
}

// §1/§4 (issue 4)
/// Refined in issue 4.
pub fn doom64_texture_name_hash(name: &str) -> u16;

impl Wad {
    /// First-match-in-disk-order resolution table. Refined in issue 4.
    pub fn doom64_texture_names(&self) -> Doom64TextureNames;
}
```

## Considered options

### Doom 64 scope

1. **Hash-only** — ship §1's name-hash function and resolution table without
   the PNG pixel decode (§5). Rejected: it lifts the ADR-0021 §5 gate for
   text-only conversion but leaves Doom 64 assets fully opaque to any
   consumer that wants pixels, splitting the format's story across an
   unpredictable future ADR for no compounding benefit — the PNG work is
   small once the resolution table exists (issue 5 depends only on issue 4).
2. **Defer Doom 64 entirely** — land only the classic tiers (§3) this cycle
   and leave ADR-0021 §5's gate in place. Rejected: it leaves the
   longest-standing user-visible wart (retail Doom 64 WADs cannot convert)
   unresolved despite this spike's research already answering the two
   questions (the hash algorithm, the PNG format) that blocked it — deferring
   would discard completed, empirically-validated research for no reason.
3. **Full resolution + PNG decode, both in this cycle (chosen)** — the
   research is complete and empirically validated (82/82, 503/503); staging
   the two Doom 64 issues (4, 5) as a short dependent chain after issue 1
   captures the win without waiting on the larger classic-tier work (issues
   2/3).

### Classic decode depth

1. **Lumps-only (tier 1 alone)** — ship typed `Picture`/`Playpal`/`Colormap`/
   `Pnames`/`TextureX`/`Flat` readers but no composition or RGBA conversion.
   Rejected: a `TEXTURE1` definition without composition is inert data —
   nothing in the crate could actually produce a renderable texture, which
   defeats the purpose of adding the format at all.
2. **No RGBA tier** — ship tiers 1 and 2 (raw lumps + composition) but stop at
   indexed-pixel output, leaving palette application to every consumer.
   Rejected: palette application is a few lines of code once composition
   exists, and every foreseeable consumer (a viewer, an image exporter) needs
   it; omitting it would just relocate identical code into every downstream
   caller.
3. **Three dependency-free tiers, all in core (chosen)** — lumps, composition,
   and RGBA convenience together give a complete, self-contained read path
   with no external dependency, matching how map parsing already ships
   fully in core.

## Consequences

- The five staged issues (§ Staging table) supersede #156/#157's original
  draft scopes — those two issues are re-scoped rather than closed, and no
  longer describe the full graphics surface on their own.
- ADR-0021 §5's write/convert-gate contract is amended, not replaced: the
  `MapFormat::Doom64` structured-reject stays in force for every writer path
  except the specific lift issue 4 implements (`TextureRef::Index` resolved
  to a name via the hash table); colored lighting's gate is not lifted by this
  ADR.
- A new optional-dependency class enters the crate via `doom64-gfx` (the
  `png` crate) — the first optional runtime dependency since `mmap`'s
  `memmap2` (ADR-0005 lineage); the four-place feature-flag sync rule applies
  once issue 5 lands the feature, not to this ADR.
- The §6 hardening table is this surface's fuzz-target charter, not a
  one-time checklist: each of its rows maps to one of the three staged fuzz
  targets, and every implementation issue is expected to extend it rather
  than treat ADR-0016 compliance as satisfied by this ADR alone.
- `TextureRef::Index`'s rustdoc correction (§1) lands with issue 4, alongside
  the name-hash function and resolution table it documents — not as a
  standalone doc-only change.
- `crates/crustywad/src/map/graph.rs`'s `TextureRef` and the ADR-0021 API
  surface are otherwise unchanged: no renaming, no new enum variants beyond
  what ADR-0021 already shipped.

## More information

- Tracking issue: #271. Re-scoped by this ADR: #156, #157.
- Related ADRs: ADR-0016 (parser/assembly hardening policy — §6 above applies
  it wholesale to this surface); ADR-0021 (Doom 64 graph normalization — §5's
  convert gate is amended by §4 above; `TextureRef::Index`'s doc is corrected
  by §1 above).
- The #65 cross-link comment (multi-WAD overlay non-goal, §2) was posted
  2026-07-16.
- Source anchors: the three spike research files — classic (Chocolate Doom,
  branch master: `src/v_patch.h`, `src/doom/r_data.c`, `src/doom/r_data.h`,
  `src/w_wad.c`, `src/v_video.c`, `src/i_video.c`, `src/doom/st_stuff.c`,
  `src/doom/r_main.c`/`.h`, `src/doom/r_local.h`); Doom 64 (svkaiser/Doom64EX,
  tag `2.5-sourceforge` and `master` HEAD: `doom64/wadfile/w_wad.c`,
  `doom64/opengl/gl_texture.c`, `doom64/playloop/p_setup.c`,
  `doom64/system/i_png.c`, `src/engine/wad/*.cc`,
  `src/engine/playloop/p_setup.cc`, `include/imp/Wad`,
  `src/engine/wadgen/*`); PrBoom+ (`prboom2/src/w_wad.c`); the empirical validation against the user's retail
  `RETAIL/DOOM64.WAD` (2020 Steam KEX re-release, 1668 lumps).
- Out of scope, documented here rather than silently dropped: multi-WAD
  load-order/overlay semantics (→ #65); graphics **writing** (patch/TEXTUREx/
  PNG emission) — a future cycle, after the read path proves the model, the
  same read-then-write sequencing ADR-0019 used; Doom 64 sprite
  `PAL<name><n>` external-palette modeling beyond what the PNG decode needs
  (left to issue 5's design); sound/music lumps (v0.7.0); the `?`-named
  texture and other naming oddities, which are data, not defects —
  `Name8`/lump names already carry arbitrary bytes.

## Amendment (2026-07-17, #156): retail sizes correct §3's COLORMAP and flat rules

The first retail graphics sweep falsified two §3 sizing claims:

- **COLORMAP:** "32 × 256 = 8192 bytes exactly" described vanilla's
  `NUMCOLORMAPS` compile-time constant, not on-disk reality. Every retail
  WAD in the collection carrying the lump — 11 of 11, across id, Freedoom,
  Raven, and Rogue — ships 34 × 256 = 8704 bytes. The engine loads the lump
  with no size check (§3's own observation), so the corrected strict rule is
  a whole number of 256-byte tables totaling at least 8192 (the 32-table
  floor every consumer indexes); all N tables are exposed.
- **Flats:** "4096 exactly" fails retail data: `HERETIC.WAD` ships seven
  4160-byte flats and `HEXEN.WAD` eleven 8192-byte flats. Corrected strict
  rule: a whole number of 64-byte rows totaling at least 4096; the rendered
  view remains the first 64×64 (vanilla's renderer reads exactly that
  regardless of lump size), with all bytes exposed raw.
- **Sky placeholders:** `F_SKY*` lumps are name-special-cased by engines and
  their pixels never read (Heretic's `F_SKY1` is 4 bytes); the graphics
  sweep skips them by name — engine-faithful, not a data exemption.
- **SVE.wad** opens a bare top-level `P3_START` with no `P_` parent and
  therefore strict-errors the §2 section scan; whether that shape is a
  legitimate wild variant or an anomaly is deferred to #292. The graphics
  sweep enumerates sections leniently so its decode assertions stay strict.
- Strife (`strife1.wad`), the anticipated risk, decodes fully strict-clean.

## Amendment (2026-07-17, #157): Strife's negative-patchcount records

The retail compose sweep found exactly one IWAD in the collection — Strife
(`strife1.wad`) — shipping four `TEXTUREx` records with genuinely negative
on-disk `patchcount` fields (`SIGN12`/`SIGN13` at -96, `WALTEK12` at -18,
`STAIR07` at -15; verified by a raw directory walk, no other retail WAD
affected). Engines survive these only because the patch loop in
`R_InitTextures` (r_data.c, cited in §3's basis) uses a signed bound a
negative count never satisfies — the records silently become zero-patch
textures. Adjudication: `NegativePatchCount` remains a strict error (the
field is malformed, and strict mode says so honestly); the sweep pins the
anomaly as a gate contract in the #269 style — the one affected IWAD must
fail strict with exactly the pinned first offender, then build leniently
with exactly four warnings and compose every texture without panicking.
