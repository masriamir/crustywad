# ADR-0017: UDMF representation and parsing strategy

- **Status:** Accepted
- **Date:** 2026-07-10
- **Deciders:** @masriamir
- **Tracking issue:** https://github.com/masriamir/crustywad/issues/57

## Context and problem statement

Epic #17 requires `crustywad` to read UDMF (Universal Doom Map Format) maps.
ADR-0014 named UDMF as the fourth `MapFormat` and explicitly deferred its
representation to this spike: *"UDMF (#57–#60). A fundamentally different
paradigm: a map's geometry lives in a single **text** `TEXTMAP` lump … It
needs its own lexer/parser and its own line/column-aware error type."*
ADR-0015 built the assembled `Map` graph with `f64` coordinates *specifically*
so UDMF's floating-point geometry would fit, declared `MapFormat`
`#[non_exhaustive]`, and sketched (but did not finalize) a `UdmfParseError`
type and a `map::udmf` module as forward references. ADR-0016 mandated that
UDMF parsing be depth-bounded via a `Limits { max_depth }` type threaded
through `ParseOptions`, introduced "only when UDMF lands."

This ADR is the deliverable of spike #57. It decides:

1. how a parsed UDMF document normalizes into the existing `Map` model
   (ADR-0015) — closing that ADR's two UDMF forward references
   (`UdmfParseError`, `map::udmf`);
2. the parser's grammar, tokenization, and depth-bounding strategy
   (ADR-0016 §2), and the concrete `Limits` / `ParseOptions.limits` plumbing;
3. how UDMF slots into map-group detection and per-format dispatch
   (ADR-0014 §2, `detect_map_format`), replacing today's blanket
   `MapAssembleError::UnsupportedFormat` refusal for `TEXTMAP`.

It does **not** decide game/engine semantic tables (thing-type IDs, linedef
specials — ADR-0014 Non-goals, still out of scope) or the write path (#60);
those are named only as consequences/forward references (see Scope
boundaries below).

### The UDMF spec, verified against source

The authoritative spec is James Haley's UDMF v1.1 (2009), distributed by
ZDoom/GZDoom and other ports as `specs/udmf.txt`. Key facts, verified against
the raw spec text (not paraphrased from memory, per the ADR-writing
checklist):

- **Container layout:** a UDMF map is `(HEADER) TEXTMAP ... ENDMAP` — a
  (conventionally empty) header/marker lump, a single `TEXTMAP` text lump
  holding all geometry, an arbitrary run of port-specific auxiliary lumps
  (`BLOCKMAP`, `ZNODES`, `DIALOGUE`, …), and a **required** empty `ENDMAP`
  closing marker. The spec explicitly recommends scanning for `ENDMAP` to
  find the full lump run, "even non-standard ones."
- **Grammar (verbatim from the spec):**

  ```text
  translation_unit := global_expr_list
  global_expr_list := global_expr global_expr_list
  global_expr := block | assignment_expr
  block := identifier '{' expr_list '}'
  expr_list := assignment_expr expr_list
  assignment_expr := identifier '=' value ';' | nil
  identifier := [A-Za-z_]+[A-Za-z0-9_]*
  value := integer | float | quoted_string | keyword
  integer := [+-]?[1-9]+[0-9]* | 0[0-9]+ | 0x[0-9A-Fa-f]+
  float := [+-]?[0-9]+'.'[0-9]*([eE][+-]?[0-9]+)?
  quoted_string := "([^"\\]*(\\.[^"\\]*)*)"
  keyword := [^{}();"'\n\t ]+
  ```

  **This grammar is exactly two levels deep: global scope, then one block
  body.** A block's body (`expr_list`) can only contain `assignment_expr`s —
  a `value` can never itself be a block. No legal UDMF document nests a block
  inside a block. This matters directly for the depth-bounding discussion
  below (§2).
- Identifiers/keywords are case-insensitive; `true`/`false` are the only
  keywords; comments are C-style (`//`, `/* */`, non-nestable); unknown
  block-header identifiers, unknown block-level fields, and unknown global
  assignments must be silently ignored by compliant parsers (with a
  should-preserve-for-round-trip note that does not bind readers); custom
  fields are conventionally prefixed `user_`.
- **`namespace` declaration:** every map should open with
  `namespace = "value";` (a plain string assignment, not a block). Reserved
  values `Doom`/`Heretic`/`Hexen`/`Strife` mean "100% vanilla-compatible
  specials/types for that game"; ports define their own namespaces (`zdoom`,
  `eternity`, …) for extended features.
- **Standardized fields** (v1.0/v1.1), confirmed field-by-field against the
  spec text — summarized in the mapping table under Decision outcome §1.
  Integers are "signed with a range of at least 32 bits"; floats are "double
  precision"; strings have no defined length limit.

### Current code this ADR must not contradict

- `crates/crustywad/src/map/graph.rs`: `MapFormat` has one variant today
  (`Doom`, `#[non_exhaustive]`); `Map { name, format, vertices, linedefs,
  sidedefs, sectors, things, warnings }` (all arena fields `pub(crate)`);
  `LineSpecial { special: u16, tag: u16 }` (the type is *currently* named
  `LineSpecial`; this ADR renames it to `Special` — see §1); `MapThing {
  x: f64, y: f64, angle: u16, type_id: u16, flags: u32 }` — **no `z`/`height`
  or `id`/`tid` field exists yet**, despite ADR-0015's sketch comment
  anticipating "Hexen tid/args."
- `crates/crustywad/src/map/assemble.rs`: `Map::assemble_with_options`
  currently refuses any group containing a `TEXTMAP` or `BEHAVIOR` lump via
  `MapAssembleError::UnsupportedFormat` — there is no `MapAssembleError::Udmf`
  variant yet, and no `detect_map_format` function exists anywhere in the
  codebase (it is only sketched in ADR-0014).
- `crates/crustywad/src/map/group.rs`: `MAP_DATA_LUMPS` (the set consulted by
  `marker_run_end`) does **not** include `"ENDMAP"`. `TEXTMAP` is listed, so
  a UDMF marker is recognized, but the data-lump run stops at the first lump
  name *not* in `MAP_DATA_LUMPS` — it does not scan forward for `ENDMAP` as
  the spec's own lump-grouping algorithm requires. A UDMF map carrying any
  port-specific auxiliary lump between `TEXTMAP` and `ENDMAP` (e.g. ZDoom's
  `ZNODES`) would have its group truncated right after `TEXTMAP`, and
  `ENDMAP` itself would fall outside `data_indices`. This is a genuine gap in
  the already-Accepted ADR-0015 `MapGroup` algorithm, not something this ADR
  is free to leave unaddressed if UDMF is to assemble correctly (§3).
- `crates/crustywad/src/lib.rs`: `ParseOptions { strictness: Strictness }` —
  no `limits` field yet. `coerce_i32` is the existing precedent for
  "strict rejects an out-of-range value, lenient clamps and warns" coercion
  that this ADR follows for UDMF integer fields.
- `fuzz/fuzz_targets/`: `fuzz_wad_strict.rs`, `fuzz_wad_lenient.rs`,
  `fuzz_parse_records_thing.rs`, `fuzz_assemble_map.rs` exist today. No UDMF
  text target exists yet (expected — ADR-0016 §3 assigns it to #57–#58).

## Decision drivers

- **Converge on the ADR-0015 `Map` model.** Consumers must never branch on
  `MapFormat`; UDMF's assembled output must be indistinguishable in shape
  from Doom's.
- **Honor ADR-0016's hardening policy.** Depth-bounded (iterative or
  explicit-depth-counter) parsing, `Limits` plumbing, a `cargo-fuzz` target,
  and both `Strictness` modes handled without panicking.
- **Fit the existing typed-record architecture**, not invent a new paradigm:
  `map::doom` / `map::common` are typed `binrw` structs decoded by
  `parse_records`, then `assemble.rs` has one `normalize_*` function per
  record kind. UDMF's text path should have a directly analogous shape.
- **UDMF's defining feature is unbounded, per-port extensibility.** The
  design must not force `Map`'s core arenas to grow for every source port's
  namespace-specific fields; it must give a genuine, motivated answer to
  "what does *not* enter `Map`."
- **Pre-1.0 is still the cheapest window for breaking changes** (ADR-0014's
  driver, still true) — `LineSpecial` (renamed to `Special` by this ADR) and
  `ParseOptions` can still change shape without a deprecation cycle.
- **This is a read-design spike.** Game/engine semantics (#59) and the write
  path (#60) are out of scope; decisions here should not foreclose either but
  must not attempt to resolve them.

## Considered options

1. **Direct-to-`Map` single-pass parser** — tokenize `TEXTMAP` text and build
   `Map`'s arenas and cross-references inline, with no intermediate typed
   representation.
2. **Generic key/value AST + stringly-typed extraction** — parse into a
   format-agnostic `Vec<(block_kind: String, fields: Vec<(String,
   UdmfValue)>)>`, then a second pass string-matches known field names while
   building `Map`.
3. **Two-stage: typed intermediate `map::udmf` model, normalized by
   dedicated functions** — an iterative tokenizer/parser builds typed
   `Udmf{Vertex,Linedef,Sidedef,Sector,Thing}` structs (one per standardized
   block kind, with spec defaults applied), mirroring `map::doom`/
   `map::common`; a second pass (`normalize_udmf_*` functions in
   `assemble.rs`, parallel to the existing `normalize_vertices` /
   `normalize_sectors` / … functions) builds the `Map` arenas and validates
   cross-references with the same strict/lenient rules as the binary path.

## Decision outcome

Chosen option: **Option 3 — typed intermediate model, normalized by dedicated
functions.** It is the only option that mirrors the codebase's existing
typed-record-then-normalize architecture closely enough that a contributor
who has read `map/doom.rs` and `map/assemble.rs` can predict the shape of the
UDMF path, gives compile-time field-name safety during normalization (no
stringly-typed lookups), and keeps a reusable, independently testable
intermediate (`UdmfMap`) available to a future full-fidelity consumer (a map
editor, #18) without forcing every namespace extension into `Map` itself.

### 1. Representation: `map::udmf`, field mapping, and what enters `Map`

Add `MapFormat::Udmf` (a bare variant, matching ADR-0014's already-accepted
sketch — `MapFormat` stays `#[non_exhaustive]`, so this is additive, not
breaking):

```rust
pub enum MapFormat {
    Doom,
    // Hexen, Doom64 land with #55 / #54.
    /// UDMF text layout (`TEXTMAP` lump). The map's `namespace` declaration
    /// is exposed via `Map::namespace()`, not folded into this variant,
    /// because `detect_map_format` decides the format from lump *names*
    /// alone and does not parse `TEXTMAP` text to reach it.
    Udmf,
}
```

`Map` gains a `namespace` field and accessor (purely additive: `Map`'s arena
fields are already `pub(crate)`, so external code cannot pattern-match them
and this cannot break existing callers):

```rust
impl Map {
    /// Returns the map's UDMF `namespace` declaration (e.g. `"doom"`,
    /// `"zdoom"`), or `None` for binary-format maps.
    #[must_use]
    pub fn namespace(&self) -> Option<&str> { self.namespace.as_deref() }
}
```

New module `map::udmf` holds the typed, un-normalized intermediate model
(field types below use the UDMF-appropriate width, `i32` for standardized
integers per the spec's "at least 32 bits signed," `f64` for floats matching
`Map`'s existing convention):

```rust
pub struct UdmfMap {
    pub namespace: String,
    pub vertices: Vec<UdmfVertex>,
    pub linedefs: Vec<UdmfLinedef>,
    pub sidedefs: Vec<UdmfSidedef>,
    pub sectors: Vec<UdmfSector>,
    pub things: Vec<UdmfThing>,
}

pub struct UdmfVertex { pub x: f64, pub y: f64 }

pub struct UdmfLinedef {
    pub v1: i32, pub v2: i32,             // no valid default; missing => Semantic error
    pub sidefront: i32,                    // no valid default
    pub sideback: Option<i32>,             // default -1, normalized to None
    pub id: i32,                           // default -1
    pub special: i32, pub args: [i32; 5],  // default 0 each
    pub flags: DoomLineFlags,              // the 9 fields with a Doom-bit equivalent (below)
}
// UdmfSidedef / UdmfSector / UdmfThing follow the same shape; field list
// mirrors the spec's "III. Standardized Fields" section per block kind.

/// Parses `TEXTMAP` text into a typed, un-normalized UDMF document. This is
/// a lexical/grammatical + per-field-default pass only — it does not
/// resolve vertex/sidedef/sector cross-references; that happens during
/// `Map` assembly (§3), which calls this internally and remains the primary
/// public entry point per ADR-0015 §3.
///
/// # Errors
/// Returns `UdmfParseError` if `text` is not valid UDMF syntax, if block
/// nesting exceeds `limits.max_depth`, or if a block omits a field with no
/// valid spec default (e.g. `vertex.x`).
pub fn parse_udmf(text: &str, limits: Limits) -> Result<UdmfMap, UdmfParseError>;
```

**Field mapping into `Map` (what normalizes, and how):**

| UDMF field(s) | Spec type / default | Normalizes into |
|---|---|---|
| `vertex.x`, `.y` | float, no default | `MapVertex.x`/`.y` — direct, no cast (both already `f64`) |
| `linedef.v1`, `.v2` | int, no default | `MapLinedef.start`/`.end` via a generalized `resolve_required` (see §3) |
| `linedef.sidefront` | int, no default | `MapLinedef.right` |
| `linedef.sideback` | int, default `-1` | `MapLinedef.left` — `-1` (explicit or defaulted) means one-sided, the direct UDMF analog of Doom's `0xffff` sentinel (ADR-0015 §4); any other out-of-range value is a dangling reference under the same strict/lenient rules |
| `linedef.blocking, blockmonsters, twosided, dontpegtop, dontpegbottom, secret, blocksound, dontdraw, mapped` | bool ×9, default `false` | `MapLinedef.flags` bits 0–8 — these 9 names correspond 1:1 to the 9 documented bit positions of the *existing* Doom on-disk `Linedef.flags` (`doom.rs`), so `flags` keeps a single, format-agnostic meaning across Doom and UDMF |
| `linedef.passuse`, SPAC flags (`playercross`, …), Strife flags (`translucent`, `jumpover`, `blockfloaters`), and all port-extension flags (ZDoom `blockplayers`, `blocksight`, …) | bool | **not modeled in `Map` yet** — see "What is deferred" below |
| `linedef.special`, `arg0`–`arg4` | int, default 0 | folds into the extended `Special` type (below) |
| `linedef.id` | int, default per spec | **new** `MapLinedef.id: i32` field — line identification (the UDMF/ZDoom line id), parallel to `MapThing.id` and `MapSector.tag`; the UDMF value is preserved, and Doom/Hexen-format linedefs populate `0`. (It has no slot in `Special`, which holds only `special` + `args[5]`, so it is modeled as a distinct linedef field.) |
| `sidedef.offsetx`, `.offsety` | int, default 0 | `MapSidedef.x_offset`/`.y_offset` |
| `sidedef.texturetop`, `.texturebottom`, `.texturemiddle` | string, default `"-"` | `MapSidedef.upper`/`.lower`/`.middle` |
| `sidedef.sector` | int, no default | `MapSidedef.sector` via `resolve_required` |
| `sector.heightfloor`, `.heightceiling` | int, default 0 | `MapSector.floor_height`/`.ceiling_height` |
| `sector.texturefloor`, `.textureceiling` | string, no default | `MapSector.floor_flat`/`.ceiling_flat` |
| `sector.lightlevel` | int, default 160 | `MapSector.light` |
| `sector.special` | int, default 0 | `MapSector.special` |
| `sector.id` | int, default 0 | `MapSector.tag` |
| `thing.x`, `.y` | float, no default | `MapThing.x`/`.y` |
| `thing.type` | int (DoomedNum), no default | `MapThing.type_id` — narrowed to `u16` with strict-reject / lenient-clamp+warning if out of range (new `MapWarning::FieldOutOfRange`, below) |
| `thing.angle` | int degrees, default 0 | `MapThing.angle` — reduced mod 360 (`rem_euclid(360)`) then cast to `u16`, since UDMF permits negative / out-of-range degrees. Doom-format angles are preserved as-is: the existing normalizer copies the on-disk `u16` verbatim and does **not** guarantee `0..360` (a deliberate per-format difference, not an invariant) |
| `thing.height` | float, default 0 | **new** `MapThing.height: f64` field (also needed by Hexen's `z: i16`, per ADR-0014 §"Hexen" — this ADR adds the field; #55 populates it for Hexen) |
| `thing.id` | int, default 0 | **new** `MapThing.id: i32` field (the Hexen/UDMF `tid`; Doom-format things always populate `0`, matching the existing "0 = untagged" convention used by `MapSector.tag` and `Special`) |
| `thing.special`, `arg0`–`arg4` | int, default 0 | **new** `MapThing.special: Special` (reuses the same extended type as linedefs — see below) |
| `thing.skill1`–`5`, `single`, `dm`, `coop`, `ambush`, `friend`, `dormant`, `class1`–`3`, Strife flags | bool | **not modeled in `Map` yet** — see below (Doom's on-disk 5-bit `flags` and UDMF's ~15 discrete booleans are not a 1:1 bit-for-bit shape, unlike linedef flags) |
| `namespace` | string | `Map.namespace: Option<String>` |
| `*.comment`, `user_*` fields | string / any | not modeled — parsed for syntax validity only, then dropped (no write path yet to round-trip them) |

**Extended, renamed `Special` type (closes ADR-0015's forward reference for
both Hexen and UDMF, which share the same special+args shape).** This ADR
**renames `LineSpecial` → `Special`**, because the type is now carried by
both `MapLinedef` and `MapThing`, so a linedef-flavored name is a misnomer.
The rename plus the field change is a single breaking change taken now,
pre-1.0:

```rust
pub struct Special {
    pub special: i32,     // widened from u16 — Doom/Hexen/UDMF specials all fit
    pub args: [i32; 5],   // Hexen/UDMF-style args; args[0] carries the
                           // tag/id where Doom used a standalone `tag` field
}
```

This **removes `LineSpecial.tag`** and renames the type (breaking; the type
is currently named `LineSpecial` and re-exported at the `map` module root
(`crate::map`, in `map/mod.rs`), so both the name and the field change ripple
to every re-export and call site). The Doom normalizer becomes:

```rust
Special {
    special: i32::from(ld.special_type),
    args: [i32::from(ld.sector_tag), 0, 0, 0, 0],
}
```

`MapThing` gains `pub special: Special`, reusing the type for thing specials
(Hexen/UDMF give things the identical `special`+`arg0..4` shape as linedefs).
The rename to `Special` (rather than keeping the now-misleading `LineSpecial`
name) is a decided part of this ADR, since pre-1.0 is the cheapest window for
it (see Decisions resolved during spike review, below).

**What is deferred (explicit, motivated non-goals of this pass):**

- The ~20 standardized boolean fields with no Doom-bit equivalent (SPAC
  activation flags, `passuse`, Strife/ZDoom-only flags) and all
  namespace-specific extensions (ZDoom sector plane equations, vertex
  `zfloor`/`zceiling`, sidedef texture scaling, sector portals, 3D floors,
  …). Modeling every port's extension now would repeat the "largest unknown"
  scope mistake ADR-0014 flagged for Doom 64. `UdmfMap`/`Udmf{Sector,
  Sidedef,Linedef,Thing}` remain available pre-normalization for a caller
  needing full text-level fidelity (e.g. a future editor, #18); the
  assembled `Map` only gets the fields listed in the table above.
- `MapThing.flags`'s UDMF synthesis is **deferred** (decided). Doom's on-disk
  `Thing.flags` packs skill into a coarse 3-bit triplet and a single
  "multiplayer-only" bit; UDMF exposes 5 independent `skill1..skill5`
  booleans plus separate `single`/`dm`/`coop` booleans. These are not the
  same bit shape, so — unlike linedef flags — there is no clean 1:1
  synthesis. `MapThing.flags` therefore keeps its current Doom-raw meaning,
  and the discrete UDMF thing booleans stay in `UdmfThing` (available to a
  full-fidelity consumer) rather than being force-fit into a normalized bit
  layout. A crustywad-normalized thing-flag representation is left for a
  concrete future consumer to motivate (see Decisions resolved during spike
  review, below).
- `*.comment` and `user_*` fields: parsed (so they don't cause a syntax
  error) but not retained. Lossless round-trip is a write-path (#60)
  concern, not decided here.
- Sector plane-equation / slope fields: `MapSector` keeps its existing flat
  floor/ceiling-height shape; no slope support is added.

### 2. Parser approach: iterative, depth-bounded, and why the grammar mostly answers the depth question itself

ADR-0016 §2 states UDMF "is an arbitrarily nested text grammar" and mandates
depth-bounded parsing. The verified grammar (Context, above) is actually
**exactly two levels deep for any spec-legal document** — a block's body can
only contain assignments, never another block. The residual risk ADR-0016
correctly anticipated is not the *legal* grammar but a **naive parser
implementation** that, on encountering an unexpected `{` (e.g. while
attempting lenient error recovery), recursively re-enters "parse a block" to
try to salvage it — that recursion depth *is* attacker-controlled by a small,
maliciously-crafted brace run, independent of the document being spec-valid.

Decision: implement the parser as a **single iterative token-stream loop**
with an explicit two-state machine (`ExpectGlobalItem` / `ExpectBlockItem`,
selected by a `depth: usize` that only ever needs to reach 1 for legal
input) — not recursive-descent at all. Any `{` encountered while
`depth != 0` (i.e., an attempt to open a block inside a block) is rejected
immediately as `UdmfParseError::Syntax`, without invoking any further
parse call. This has **zero native call-stack growth as a function of
input**, which is the strongest of the two options ADR-0016 permits
("iterative, or recursive with an explicit depth counter"). The tokenizer is
likewise a flat scan (no recursion): it classifies `{`, `}`, `=`, `;`,
identifiers, `true`/`false` keywords, quoted strings (with backslash-escape
handling), and integers/floats per the grammar's lexical rules, tracking
line/column for error reporting.

`Limits::max_depth` (ADR-0016 §2, default `64`) is still honored — checked
once per `{` token against a `depth` counter — even though a strictly
spec-compliant document never exceeds depth 1. It remains valuable
defense-in-depth exactly for the "naive recovery recursion" scenario above,
it is effectively free to check, and it future-proofs the `Limits` plumbing
for any later text format that *does* have legitimate nested structure. This
ADR is the one that "pulls the trigger" on adding `Limits` (ADR-0016 §2
introduced it as a type "only when UDMF lands"):

```rust
pub struct ParseOptions {
    pub strictness: Strictness,
    /// Resource limits applied to parsing (currently: UDMF nesting depth).
    /// Ignored by all binary-format paths.
    pub limits: Limits,
}
```

`ParseOptions::strict()`/`::lenient()` are updated to also set
`limits: Limits::default()`; this is an additive-but-breaking change to
`ParseOptions` struct-literal construction, already anticipated and
sequenced by ADR-0016.

`UdmfParseError` (closing ADR-0015's forward reference; distinct from
`ParseError` and `MapParseError` because it carries a source position that
neither of those model):

```rust
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum UdmfParseError {
    /// The `TEXTMAP` lump's bytes are not valid UTF-8.
    #[error("TEXTMAP lump is not valid UTF-8 at byte offset {offset}")]
    InvalidEncoding { offset: usize },
    /// A lexical or grammatical error at a specific source position.
    #[error("syntax error at line {line}, column {column}: {message}")]
    Syntax { line: usize, column: usize, message: String },
    /// Block nesting exceeded `ParseOptions.limits.max_depth`.
    #[error("nesting depth exceeded the configured limit ({max_depth}) at line {line}, column {column}")]
    DepthExceeded { max_depth: usize, line: usize, column: usize },
    /// A block omitted a field with no valid spec default (e.g. `vertex.x`),
    /// or the document lacked a `namespace` declaration.
    #[error("semantic error: {message}")]
    Semantic { message: String },
}
```

`TEXTMAP` bytes are decoded as UTF-8 only (`std::str::from_utf8`); the
spec's alternative encodings (ISO-8859-1, Windows-1252) are not decoded in
this pass — this is a **decided deferral** (see Decisions resolved during
spike review, below), not a silent gap: anything non-UTF-8 is a clean
`UdmfParseError::InvalidEncoding` in both modes, and real UDMF content is
overwhelmingly ASCII (a subset of all three encodings).

**Strict vs. lenient (ADR-0014 §6, applied to text):** structural failures
(bad token, unbalanced braces, exceeded depth, a required field with no
valid default missing) are hard errors in **both** modes, mirroring the
"empty required arena is always fatal" precedent already established for
binary cross-reference assembly (ADR-0015 §4) — there is no well-defined
recovery for "this vertex has no `x`." Recoverable anomalies are narrower
than in the binary path: an out-of-range *value* for a field whose target
type is narrower than the spec's guarantee (e.g. `thing.type` not fitting
`u16`) is a new `MapWarning::FieldOutOfRange { field, from, value }` in
lenient mode (clamped) and a `MapAssembleError` in strict mode, following the
existing `coerce_i32` precedent (`lib.rs`) rather than inventing a new
pattern.

### 3. Format dispatch: `detect_map_format`, the `ENDMAP` gap, and assembly

Implement `detect_map_format` for real (ADR-0014 §2 sketched only the
signature):

```rust
pub fn detect_map_format(group: &MapGroup) -> MapFormat {
    // TEXTMAP present ⇒ Udmf; BEHAVIOR present ⇒ Hexen (once #55 lands);
    // else ⇒ Doom. Doom 64 signature TBD in #54.
}
```

**`MapGroup` fix required for UDMF to assemble correctly.** As noted in
Context, `group.rs`'s `marker_run_end` stops the data-lump run at the first
name outside `MAP_DATA_LUMPS`, which cannot reach `ENDMAP` past any
port-specific auxiliary lump. This ADR proposes a UDMF-specific branch,
landing with #58: when the lump immediately after a marker is `TEXTMAP`, the
run is bounded by the first subsequent lump literally named `ENDMAP` rather
than by `MAP_DATA_LUMPS` membership. `ENDMAP` is deliberately **not** added
to `MAP_DATA_LUMPS` itself — that would not fix the gap, since the
contiguous-recognized-name rule still cannot skip over intervening
unrecognized (port-specific) lump names to reach it. `MapGroup`'s *shape*
(`marker_index`/`name`/`data_indices`) is unchanged; only the algorithm that
populates `data_indices` for the `TEXTMAP` case changes.

**Missing `ENDMAP` is strictness-aware (decided).** The spec makes `ENDMAP`
a required closing lump, so its absence means a malformed/truncated UDMF map.
This ADR resolves the two modes differently, mirroring the strict/lenient
contract (ADR-0003) rather than failing closed in both:

- **Strict:** a `TEXTMAP` run with no subsequent `ENDMAP` is a hard failure —
  the group is not recognized as a valid UDMF map (and assembly, if invoked
  on such a group, errors rather than silently guessing where the map ends).
- **Lenient:** the run is recovered best-effort — bounded by the next real
  map marker (the next lump whose successor is a recognized map data lump) or
  end-of-directory, whichever comes first — and a warning is recorded so the
  truncated map is still inspectable, consistent with how lenient mode
  recovers elsewhere.

This makes the `ENDMAP`-scan path (whether in `marker_run_end`/group
detection or in the UDMF assembly branch) **strictness-aware**; today's
`map_groups`/`marker_run_end` take no `ParseOptions`, so threading strictness
into group detection (or performing the missing-`ENDMAP` decision at the
assembly boundary where options are already available) is a plumbing choice
left to #58. Either way the scan remains a single bounded pass over the
directory, preserving ADR-0016 §1's `O(input)` invariant.

**Assembly dispatch.** `Map::assemble_with_options` drops `"TEXTMAP"` from
the current blanket `UnsupportedFormat` refusal loop (`"BEHAVIOR"` stays
refused until #55) and adds a UDMF branch:

```rust
let bytes = lump_bytes(wad, group, "TEXTMAP")
    .ok_or(MapAssembleError::MissingLump { lump: "TEXTMAP" })?;
let text = std::str::from_utf8(bytes).map_err(|e| MapAssembleError::Udmf {
    source: UdmfParseError::InvalidEncoding { offset: e.valid_up_to() },
})?;
let udmf = parse_udmf(text, options.limits)
    .map_err(|source| MapAssembleError::Udmf { source })?;
// normalize_udmf_vertices / _linedefs / _sidedefs / _sectors / _things,
// mirroring the existing normalize_* functions and reusing (a generalized,
// i32-aware) resolve_required / resolve_left for cross-references.
```

`MapAssembleError` gains the `Udmf` variant ADR-0015 §4 already sketched:

```rust
#[error("failed to parse UDMF text map: {source}")]
Udmf { #[source] source: UdmfParseError },
```

`resolve_required`/`resolve_left` (private helpers in `assemble.rs`) widen
their index parameter from `u16` to `i32` so UDMF's signed, wider indices
share the same validation code as Doom's; a negative index is simply treated
as out of range (`usize::try_from` failure), reusing the existing
strict-error/lenient-clamp-and-warn branches unchanged. This is an internal
signature change only (`resolve_required`/`resolve_left` are not `pub`).

### 4. Scope boundaries

This ADR (and #57/#58) covers **representation and the read path only**:
`map::udmf`, `parse_udmf`, `detect_map_format`, `MapGroup`'s `ENDMAP` fix,
and normalization into `Map`. Two adjacent concerns are explicitly named as
forward references, not decided here:

- **#59 (game/engine semantic conversion).** Namespace-gated field legality
  (e.g. "`passuse` is meaningless outside Boom-derived namespaces"),
  DoomedNum/special interpretation tables — all Non-goals of ADR-0014,
  unchanged by this ADR. This ADR's parser accepts standardized fields
  uniformly regardless of the declared `namespace`; namespace-based
  filtering, if ever added, is #59's decision.
- **#60 (UDMF write).** Serializing a `Map` back to `TEXTMAP` text (`f64` →
  UDMF float/integer narrowing, re-deriving per-field defaults to omit
  redundant assignments, comment/custom-field round-trip if desired) is a
  write-path concern (ADR-0006 lineage) and is untouched here.

## Consequences

- **New public API:** `MapFormat::Udmf` (additive — `#[non_exhaustive]`),
  `Map::namespace()`, `map::udmf` (`UdmfMap`, `UdmfVertex`, `UdmfLinedef`,
  `UdmfSidedef`, `UdmfSector`, `UdmfThing`, `parse_udmf`), `UdmfParseError`,
  `Limits` + `ParseOptions.limits`, `MapAssembleError::Udmf`,
  `MapWarning::FieldOutOfRange`, `detect_map_format`.
- **Breaking (pre-1.0, intentional):** `LineSpecial` is renamed to `Special`;
  its `tag` field is removed in favor of `args[0]` and its `special` field
  widens `u16` → `i32`. `MapThing` gains `height: f64`, `id: i32`, and
  `special: Special`; `MapLinedef` gains `id: i32` (line identification).
  `ParseOptions` gains a `limits` field (mitigated by the `strict()`/`lenient()`
  constructors, per ADR-0016). UDMF thing angles are reduced mod 360 on the way
  into `MapThing.angle` (UDMF permits negative / out-of-range degrees);
  Doom-format angles are unchanged — preserved verbatim as today, with no new
  invariant imposed.
- **Good** — UDMF converges cleanly onto the existing `Map` model; no
  consumer branches on `MapFormat`.
- **Good** — the parser/normalize split mirrors `map::doom` +
  `assemble.rs`'s existing architecture; `resolve_required`/`resolve_left`
  are reused (generalized), not duplicated.
- **Good** — `UdmfMap` stays available for a future full-fidelity consumer
  (#18) without forcing every namespace extension into `Map`.
- **Neutral** — this ADR fixes a real gap in the Accepted ADR-0015
  `MapGroup` algorithm (the missing `ENDMAP` scan); it does not reopen
  `MapGroup`'s shape, only the `TEXTMAP`-case algorithm inside
  `marker_run_end`.
- **Hardening** — a `fuzz_parse_udmf` target (str-oriented, no-panic oracle,
  seeded with valid and malformed `TEXTMAP` samples plus pathological brace
  runs) is required per ADR-0016 §3, landing with #58; `fuzz_assemble_map`'s
  corpus gains UDMF-bearing synthetic WADs.
- **Deferred (decided — see Decisions resolved during spike review)** — the
  ~20 non-bit-mappable boolean fields, `MapThing.flags`'s UDMF synthesis,
  comment/custom-field retention, and non-UTF-8 `TEXTMAP` decoding.
  Namespace-gated semantics (#59) and the write path (#60) remain out of
  scope by forward reference (§4).

## Pros and cons of the options

### Option 1 — direct-to-`Map` single-pass parser

- Good, because it is the least code — no intermediate types.
- Bad, because it couples the lexer directly to `Map`'s exact arena shape,
  making the parser hard to reuse for a future full-fidelity consumer (#18)
  that wants comments/custom fields/unmapped booleans — directly working
  against the "should not foreclose a future mutable layer" driver ADR-0015
  already established.
- Bad, because the grammar layer and the cross-reference-resolution layer
  become inseparable, so neither can be unit-tested independently the way
  `normalize_vertices` etc. are today.

### Option 2 — generic key/value AST + stringly-typed extraction

- Good, because the parser core is block-agnostic — no new type per block
  kind, and a future 6th block kind needs no parser change.
- Bad, because field-name typos in the extraction pass fail silently (fall
  back to the field's default) instead of being caught by the compiler,
  unlike Option 3's typed struct fields.
- Bad, because value-type coercion (is this `value` actually a float where a
  float is expected?) must be re-validated at every extraction call site
  instead of once during parsing.

### Option 3 — typed intermediate model, normalized by dedicated functions (chosen)

- Good, because it mirrors `map::doom`/`map::common` + `assemble.rs`'s
  existing normalize-function pattern, so the UDMF path is immediately
  legible to anyone familiar with the Doom path.
- Good, because typed fields give compile-time safety during normalization
  and each `normalize_udmf_*` function is independently testable, matching
  the "Adding a new lump type" testing conventions already in `CLAUDE.md`.
- Bad, because it is more code than Option 1 or 2: a tokenizer, a
  block-shape parser, five typed intermediate structs, and five normalize
  functions.
- Bad, because the typed `Udmf*` structs duplicate field lists already
  present in `Map`'s normalized types — a new standardized spec field means
  touching two struct definitions instead of one.

## More information

- Tracking spike: #57. Feeds #58 (UDMF read implementation), which this ADR
  is the design for. Forward references only (not decided here): #59
  (game/engine semantic conversion), #60 (UDMF write).
- Depends on / completes forward references from: ADR-0014 (multi-format
  strategy — `MapFormat::Udmf`, `detect_map_format`, the illustrative
  `parse_udmf`/`UdmfParseError` sketch), ADR-0015 (assembled `Map` model —
  `f64` coordinates, `UdmfParseError` reference, `map::udmf` reference, the
  "exact auxiliary fields… finalized alongside the Hexen/UDMF work" note for
  `LineSpecial`/`MapThing`), ADR-0016 (parser hardening — `Limits`,
  depth-bounding, per-PR fuzz checklist).
- Related ADRs: ADR-0002 (binrw / typed errors — the pattern `UdmfParseError`
  deliberately does *not* follow, since UDMF is text, not binrw), ADR-0003
  (strict/lenient default), ADR-0009 (`cargo-fuzz` harness — extended by the
  new UDMF target), ADR-0013 (restraint — motivates deferring the ~20
  non-bit-mappable boolean fields rather than modeling them speculatively).
- Source anchors: `crates/crustywad/src/map/graph.rs` (`MapFormat`, `Map`,
  `LineSpecial`, `MapThing`), `crates/crustywad/src/map/assemble.rs`
  (`MapAssembleError`, the current `UnsupportedFormat` refusal loop,
  `resolve_required`/`resolve_left`, `normalize_*`), `crates/crustywad/src/
  map/group.rs` (`MAP_DATA_LUMPS`, `marker_run_end`), `crates/crustywad/src/
  lib.rs` (`ParseOptions`, `coerce_i32`), `fuzz/fuzz_targets/
  fuzz_assemble_map.rs` (existing O(input) / warning-bound assertions to
  extend). Spec source: `specs/udmf.txt` (v1.1, James Haley, 2009),
  distributed at
  <https://github.com/rheit/zdoom/blob/master/specs/udmf.txt> and
  <https://github.com/coelckers/gzdoom/blob/master/specs/udmf_zdoom.txt>
  (ZDoom namespace extensions); <https://doomwiki.org/wiki/UDMF> and
  <https://zdoom.org/wiki/UDMF> for cross-port summary and namespace list.

### Decisions resolved during spike review (#57)

The five questions raised by the draft were resolved by @masriamir (sole
decider) during spike review and are folded into the sections above; recorded
here with one-line rationale each:

1. **Bind the `special`/`args[5]` shape now (don't wait for #55).** The
   extended `Special { special: i32, args: [i32; 5] }` shape in §1 is the
   decision, not a provisional sketch — Hexen (#55) must conform to it.
   Rationale: ADR-0015 tied the Hexen and UDMF special encodings to one
   normalized type, so fixing it once here avoids two half-designs racing.
2. **Rename `LineSpecial` → `Special`.** Since `MapThing` now also carries
   one, the linedef-flavored name is a misnomer; the rename is applied in §1
   and Consequences. Rationale: pre-1.0 is the cheapest window for the rename;
   deferring only banks a later breaking change.
3. **Defer `MapThing.flags` UDMF synthesis.** `MapThing.flags` keeps its
   Doom-raw meaning and the discrete UDMF thing booleans stay in `UdmfThing`
   (§1). Rationale: Doom's coarse bit-packing and UDMF's ~15 independent
   booleans have no clean 1:1 mapping (unlike linedef flags), so a normalized
   thing-flag layout waits for a concrete consumer to motivate it.
4. **Missing `ENDMAP`: strict fails closed, lenient recovers best-effort.**
   §3 now specifies that a `TEXTMAP` run without a following `ENDMAP` is a
   hard failure in strict mode, but in lenient mode the run is recovered
   bounded by the next real marker or end-of-directory with a warning.
   Rationale: consistent with the strict/lenient contract (ADR-0003) — strict
   surfaces the malformed map, lenient keeps a truncated map inspectable.
5. **Defer non-UTF-8 `TEXTMAP` decoding.** UTF-8 only for now, with
   `UdmfParseError::InvalidEncoding` for anything else in both modes (§2). A
   lenient lossy fallback (the `Name8` `from_utf8_lossy` precedent) waits for
   a real fixture. Rationale: real UDMF content is overwhelmingly ASCII, a
   subset of all permitted encodings, so the fallback is speculative until
   demonstrated needed (ADR-0013 restraint).

- **Revisit condition:** reopen if a concrete UDMF fixture needs any of the
  deferred boolean fields, the `MapThing.flags` normalization, slope fields,
  or non-UTF-8 decoding; or if #59's namespace-semantic work finds that
  namespace-gated field *parsing* (not just interpretation) is actually
  required, contradicting this ADR's "accept standardized fields uniformly
  regardless of namespace" choice in §4. Note that #55 (Hexen) does **not**
  trigger a revisit of the `Special`/`args[5]` shape: per decision 1 above,
  #55 must conform to the shape this ADR fixes, not the other way around.
