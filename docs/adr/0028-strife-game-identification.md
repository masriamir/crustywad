# ADR-0028: Strife game identification and semantic attribution

- **Status:** Accepted
- **Date:** 2026-08-02
- **Deciders:** @masriamir
- **Tracking issue:** https://github.com/masriamir/crustywad/issues/246

## Context and problem statement

Strife WADs assemble today as `MapFormat::Doom` with zero errors and zero
warnings — dogfooding (2026-07-13) assembled all 34 maps of retail
`strife1.wad` and all 38 of `SVE.wad` cleanly. Structurally that is correct:
the spike (#246) verified from Chocolate Strife source that **all eight binary
map records are byte-for-byte identical to Doom's** (same widths, offsets, and
signedness; full-file diff of `doomdata.h` and every `p_setup.c` loader).
Semantically it is a silent misread: Strife reassigns thing-flag bit `8` from
AMBUSH to STAND (relocating AMBUSH to bit `32`), adds thing flags `64`–`1024`
and linedef flags `512`–`4096`, extends the linedef special table to 232 (+666)
with several specials repurposing the `tag` field as a packed parameter, and
uses a distinct thing-type namespace. A consumer reading a Strife map through
crustywad gets Doom semantics with **no signal that they are wrong** — in
retail `strife1.wad` alone, 2,290 things carry bit `8` (STAND, not AMBUSH) and
1,342 carry Strife's relocated AMBUSH bit `32`, which Doom-semantics readers
simply drop.

The full research record — engine-source citations (Chocolate Doom/Strife @
`353cf5001dfd5777c13327010fa58acb57b913b2`, Strife: Veteran Edition GPL source
@ `ac2381d3d6cf32f52acb1506d5ba47a856813098`) and an empirical scan of the
17-WAD retail collection — lives in four comments on #246. This ADR decides
how crustywad identifies Strife WADs and attributes Strife semantics, and
stages the implementation (epic #241).

## Verified findings (spike #246 summary)

| # | Spike question | Answer (citations in the #246 research record) |
|---|---|---|
| 1 | Record layouts | Byte-for-byte identical to Doom for all eight records; differences are flag/special/type *semantics* and post-decode engine fixups only |
| 2 | Detection | The reference engine identifies Strife **by IWAD filename only** (content check is a `STRIFE-TODO` stub); there is no per-map marker (map groups are exactly Doom's 10-lump shape). Reliable signals are WAD-level lump content: `SCRIPTnn`/`LOGnn` appear in both Strife WADs and none of the other 15 retail WADs; every retail `SCRIPTnn` lump is an exact multiple of the 1516-byte dialogue record |
| 3 | Semantic surface | Thing-flag bit `8` diverges (AMBUSH↔STAND); Strife-only thing flags `32`/`64`/`128`/`256` (+ unused-in-retail `512`/`1024`); additive linedef flags `512`–`4096`; specials to 232 (+666) with `tag`-packing; sector special 15 (instant death) and 18 (tag-packed water current) |
| 4 | Dialogue lumps | Per-map `SCRIPTnn` + global `SCRIPT00`; flat fixed-size records — 1516 bytes retail, 1488-byte demo variant (SVE-source-verified), chosen by a lump-length modulus heuristic; all-i32-LE + fixed NUL-padded strings; no nesting or variable-length data |
| 5 | SVE | Adds **no** required map-format surface: classic path byte-identical to Chocolate Strife; per-map `GL_MAPnn` groups are the classic GLBSP `gNd2` dialect crustywad already reads (ADR-0025, #324); no `LM_MAPnn` groups ship in `SVE.wad`; `SVE.wad` is a **PWAD** without `XLATAB`, carrying 4 `SCRIPTnn` + 10 `LOGnn` lumps of its own |
| 6 | Interim warning | Decided in §3 below: attribution in both modes via `Map::game()`, advisory `MapWarning` in lenient mode (matching the documented lenient-only `MapWarning` contract) |

Two heuristics were empirically **disqualified** for detection: linedef
high-flag bits (DOOM.WAD itself contains 249 linedefs with stale `0xFE__`
garbage bits) and special-value ranges (Strife stays within Doom's numeric
range). Only lump-content fingerprints survive the false-positive matrix.

## Decision drivers

- **End the silent misread (#241)** with a positive, content-validated signal
  that identifies both `strife1.wad` and the standalone `SVE.wad` overlay with
  zero false positives across the 15 non-Strife retail WADs.
- **Honor ADR-0014's axes model.** `MapFormat` is the byte/text *layout*
  axis; game/engine identity is orthogonal, and ADR-0014's Boom worked example
  sets the rule for byte-identical-layout-plus-new-values cases: "`MapFormat::
  Doom` plus extended engine semantics — never a new `MapFormat` variant."
  The spike proved Strife is exactly such a case, so Strife support is the
  game axis's first representation, not a new `MapFormat`.
- **Honor the strictness contract** (ADR-0003): a Strife WAD is not malformed
  input; nothing here may become a strict-mode error, and `MapWarning` stays
  lenient-only as documented.
- **ADR-0016 hardening** applies to any new parse surface (the dialogue
  reader); detection itself must add no parse surface.
- **Keep #247 tractable**: dialogue parsing is separable from identification
  and attribution.

## Considered options

1. **`MapFormat::Strife` variant** — extend the layout enum and teach
   `detect_map_format` a Strife branch.
2. **WAD-level game identification** — a new `WadGame` axis (`Wad::detect_game`)
   feeding per-map attribution (`Map::game()`) and a lenient advisory warning;
   `MapFormat` unchanged.
3. **Documentation only** — document the Strife bit tables and leave detection
   to consumers.

## Decision outcome

Chosen option: **option 2, WAD-level game identification**, because it is the
only option that both ends the silent misread and keeps ADR-0014's model
intact: Strife has no per-map signal (its map groups are byte-structurally
Doom), so a `MapFormat` variant would misfile a WAD-level *game* property on
the per-map *layout* axis and force `detect_map_format` to consult state it
was designed not to have. Option 3 leaves #241 standing.

### §1 The `WadGame` axis and the detection rule

A new public enum in `lib.rs` (beside `WadKind` — both are WAD-level
properties), representing **positively identifiable game families**:

```rust
/// A game family positively identified from WAD-level lump content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum WadGame {
    /// Strife (Rogue Entertainment) — identified by dialogue `SCRIPTnn` lumps.
    Strife,
}

impl Wad {
    /// Detects the game family this WAD targets, if it can be positively
    /// identified from lump content. Returns `None` when no fingerprint
    /// matches (a Doom/Heretic/... WAD, or a content-free PWAD).
    #[must_use]
    pub fn detect_game(&self) -> Option<WadGame>;
}
```

**Detection rule (Strife):** the WAD contains at least one lump whose 8-byte
name is exactly `SCRIPT` followed by two ASCII digits (`SCRIPT00`–`SCRIPT99`,
filling the name field with no NUL padding) **and** whose size is a nonzero
exact multiple of 1516 bytes (the retail dialogue record) or, failing that, of
1488 bytes (the demo-format record; release-size precedence mirrors SVE's own
`P_getDialogFormat` heuristic). One qualifying lump suffices.

Grounding: `SCRIPTnn` is loaded by name pattern `"script%02d"` in the engine
(`p_dialog.c`); the name pattern appears in **zero** of the 15 non-Strife
retail WADs; all 23 `strife1.wad` and all 4 `SVE.wad` script lumps pass the
size check (`SCRIPT00` = 63,672 B = exactly 42 records). The size condition
makes the rule content-validated rather than name-only, so a stray empty or
junk lump named `SCRIPT01` does not fire. Corroborating lumps (`LOGnn`,
`XLATAB`, `ENDSTRF`, `SERIAL`) are deliberately **not** required: `SVE.wad`
ships none of the vanilla-IWAD globals, and requiring them would miss it.

Detection is a name/size scan of the already-parsed directory — no new parse
surface, no allocation beyond the return value, `O(lump count)` per call.
`WadGame` is `#[non_exhaustive]`: a future Heretic/Hexen-family fingerprint
can be added without a breaking change. `Doom64`/`Hexen` are *not* initial
variants — those are already positively identified on the layout axis, and
inventing a second signal for them now would be speculative.

### §2 Per-map attribution: `Map::game()`

`Map` gains a game-attribution field set during assembly, in **both**
strictness modes:

```rust
impl Map {
    /// The game family the source WAD was positively identified as, if any.
    /// `Some(WadGame::Strife)` means this map's raw `flags`/`special`/`type`
    /// values carry Strife semantics even though [`Map::format`] reports the
    /// (byte-identical) Doom layout.
    #[must_use]
    pub fn game(&self) -> Option<WadGame>;
}
```

`Map::assemble*` computes `wad.detect_game()` once per call and records it on
the resulting map regardless of the map's `MapFormat` (game identity is a
property of the WAD; a UDMF map inside a Strife WAD still belongs to Strife —
its own `namespace` governs its field semantics, but the attribution is not
suppressed). `Map::format` is untouched: a Strife map correctly reports
`MapFormat::Doom`, because that *is* its layout.

### §3 Interim advisory warning (spike Q6)

A new `MapWarning` variant, emitted in **lenient mode only** (the documented
`MapWarning` contract — strict mode must not fail on a well-formed WAD, and
the existing contract says warnings are produced by lenient assembly):

```rust
/// The source WAD was positively identified as a game whose record semantics
/// (thing/linedef flags, specials, thing types) the assembled graph does not
/// model; raw values are preserved but follow that game's meaning, not Doom's.
#[error(
    "map belongs to a {game:?} WAD; record semantics beyond the Doom baseline are not modeled"
)]
UnmodeledGameSemantics {
    /// The positively identified game family.
    game: WadGame,
},
```

Pushed once per map when `detect_game()` returns `Some(Strife)` and the map's
format is `MapFormat::Doom` (binary Doom-layout records are where the
misinterpretation risk lives; UDMF maps carry their own namespace). The
warning is *interim by design*: it exists until the graph models the
semantics it names, and the variant (not the mechanism) should be revisited
if a future issue enriches `Map` with typed Strife flags — see revisit
condition (a).

### §4 Semantic surface: documented constants, no graph enrichment

Per ADR-0014's standing non-goal, this ADR does **not** add game-semantic
decoding to the `Map` graph. What #247 ships instead is a documented constants
module so consumers can interpret the raw values correctly:

- `map::strife` with the source-verified bit constants: thing flags
  (`MTF_STAND` = 8, `MTF_AMBUSH` = 32, `MTF_FRIEND` = 64, `MTF_TRANSLUCENT` =
  256, `MTF_MVIS` = 512, plus the two engine-acknowledged unknowns 128/1024)
  and linedef flags (`ML_JUMPOVER` = 512, `ML_BLOCKFLOATERS` = 1024,
  `ML_TRANSPARENT1` = 2048, `ML_TRANSPARENT2` = 4096), each doc-commented with
  its Chocolate Strife citation and, where the engine itself is unsure
  (JUMPOVER/TRANSPARENT semantics are `villsa` TODO guesses in source), that
  caveat verbatim.
- A guide section pairing the constants with the divergence table (bit `8`
  AMBUSH↔STAND being the headline hazard).

Typed flag views on `MapThing`/`MapLinedef` (the enrichment path #380 takes
for UDMF booleans) are deferred — revisit condition (a).

### §5 Dialogue lumps: in-epic, separately staged

Typed `SCRIPTnn`/`SCRIPT00` parsing is **in scope for epic #241 but not for
#247** — it is a new parse surface with its own ADR-0016 obligations and no
coupling to attribution. A follow-up sub-issue (filed at ADR merge) will ship:

- A `map::strife` dialogue reader accepting **both** record sizes via the
  length heuristic SVE's engine uses (divisible by 1516 → retail layout;
  else divisible by 1488 → demo layout, which drops `checkitem`/`jumptoconv`/
  `backpic`): flat records, all integers i32 LE, all strings fixed NUL-padded
  byte arrays (not guaranteed NUL-terminated — `trim_nul` semantics, never C
  strings).
- Strictness: strict rejects a lump whose length matches neither modulus
  (an empty lump is zero retail records in BOTH modes — engine-faithful:
  `P_getDialogFormat` checks the retail modulus first and `0 % 1516 == 0`;
  adjudicated during #393); lenient floor-divides like the engine and warns
  about trailing bytes. Contract shape: `Result<(Vec<DialogueRecord>, Vec<DialogueWarning>),
  DialogueError>` — warnings observable on success; exact signatures are
  finalized in the sub-issue.
- ADR-0016 checklist in full: bounded allocation (`lump_len / 1488` records
  worst case), no recursion, a fuzz target with seed corpus, both modes
  panic-free.

`LOGnn` lumps are plain text and need no typed parser (the existing lump API
suffices); the dialogue records reference them by number (`objective` field),
surfaced as-is.

### §6 SVE: no additional format work

SVE requires nothing beyond classic Strife support for playable geometry
(engine-source-verified). Its `GL_MAPnn` groups are the classic `gNd2` GL-node
dialect already read since #324 (ADR-0025); no `LM_MAPnn` groups ship in
`SVE.wad`; its `LOCATION`/shader/frontend lumps are game data outside map
parsing. The detection rule of §1 identifies `SVE.wad` standalone via its own
4 script lumps. The demo-dialogue record size (§5) is likewise SVE-derived and
covers the teaser format by design, though no teaser fixture exists to test
against — revisit condition (c).

### §7 CLI surface

`cwad info` gains a `game:` line (human and JSON/CSV forms) when
`detect_game()` fires, so the attribution is visible without writing code.
No new subcommand or flag; `convert`/`build` behavior is unchanged (Strife
maps are Doom-layout and already convert/build correctly at the byte level).

### §8 Staging

- **#247 — "Strife records, detection, and assembly"**: §1 `WadGame` +
  `Wad::detect_game`, §2 `Map::game()`, §3 the lenient warning, §4 the
  constants module + guide section, §7 `cwad info`, and a retail-sweep
  assertion that `strife1.wad` and `SVE.wad` both attribute as Strife
  (the epic's acceptance). No new record structs — the spike proved none are
  needed.
- **New sub-issue of #241** (filed at ADR merge): §5 dialogue reader.

### Consequences

- Good, because the silent misread ends with a signal that is content-
  validated, zero-false-positive on the retail collection, and works for both
  the retail IWAD and the standalone SVE PWAD.
- Good, because ADR-0014's axes survive intact: `MapFormat` still means
  layout, and the game-identity axis ADR-0014 deferred gets its first,
  deliberately minimal representation.
- Good, because everything in #247 is additive API (new enum, new accessors,
  new lenient warning variant on a `#[non_exhaustive]` enum) — no breaking
  change.
- Bad, because the game axis initially identifies only Strife; consumers
  cannot distinguish Doom from Heretic through it (unchanged from today, and
  extensible later).
- Bad, because name+size fingerprinting is probabilistic in principle: a
  non-Strife PWAD could theoretically ship a conforming `SCRIPTnn` lump. The
  size condition makes accidental collision vanishingly unlikely, and the
  consequence (an advisory attribution) is mild.
- Neutral, because `detect_game()` is recomputed per `assemble*` call
  (`O(lump count)` name compares; trivially cheap even across a 4,000-lump
  IWAD × 34 maps).

## Pros and cons of the options

### Option 1 — `MapFormat::Strife`

- Good, because attribution would ride an existing, well-known surface.
- Bad, because it contradicts ADR-0014: "`MapFormat` being `#[non_exhaustive]`
  is for genuinely new *layouts* (a future port's on-disk format)", and its
  allowance that "future ports (Strife, …) may add layouts" was conditioned on
  Strife actually being a new layout — which the spike disproved. ADR-0014's
  Boom rule (byte-identical layout + new values in existing fields → never a
  new `MapFormat` variant) applies to Strife verbatim.
- Bad, because `detect_map_format` is per-map by design and Strife has no
  per-map signal; the function would need WAD-global state, changing its
  contract for every format.
- Bad, because every existing exhaustive-by-wildcard `match` on `MapFormat`
  (assembly dispatch, conversion, node building) would silently route a
  layout-identical "new format" through fallback arms — churn with no
  behavioral gain over §2's orthogonal accessor.

### Option 2 — WAD-level game identification (chosen)

- Good, because it models the truth the spike established: same layout,
  different game.
- Good, because detection lives at the level where the signal actually exists
  (the lump directory), and per-map surfaces (`Map::game()`, the warning)
  are derived from it without contract changes.
- Bad, because it introduces a new one-variant public enum — small surface
  that must be documented as deliberately minimal.

### Option 3 — documentation only

- Good, because zero code.
- Bad, because #241's core defect — *no signal* — remains; every consumer
  must reimplement fingerprinting to notice they are misreading flags.

## More information

- Research record: four comments on #246 (2026-08-02) — record layouts &
  flag/special semantics; detection signals & map-group structure; dialogue
  format & SVE surfaces; empirical 17-WAD retail scan. Engine sources:
  Chocolate Doom/Strife @ `353cf5001dfd5777c13327010fa58acb57b913b2`,
  Strife: Veteran Edition @ `ac2381d3d6cf32f52acb1506d5ba47a856813098`.
- Related ADRs: ADR-0014 (the format/game/engine axes; amended 2026-08-02 to
  point here),
  ADR-0003 (strictness), ADR-0015 (graph model), ADR-0016 (hardening — binds
  the §5 dialogue reader), ADR-0017 §"Strife booleans" and ADR-0027 (UDMF
  already retains Strife-specific fields losslessly), ADR-0025/#324 (the
  `gNd2` GL-node reading SVE relies on).
- Revisit conditions: **(a)** typed Strife flag views on `MapThing`/
  `MapLinedef` (graph enrichment; would retire or narrow §3's warning);
  **(b)** additional `WadGame` variants when another family gains a verified
  fingerprint; **(c)** a teaser (`strife0.wad`) fixture becoming available —
  the 1488-byte demo path is engine-verified but fixture-untested.
