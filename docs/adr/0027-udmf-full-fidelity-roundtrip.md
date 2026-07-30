# ADR-0027: UDMF full-fidelity retention and semantic round-trip

- **Status:** Accepted
- **Date:** 2026-07-29
- **Deciders:** @masriamir
- **Tracking issue:** https://github.com/masriamir/crustywad/issues/257

## Context and problem statement

ADR-0017 scoped the UDMF read path to the standardized fields that normalize
into the assembled `Map`, and recorded a set of explicit deferrals; issue #257
tracks them. Two of those deferrals — `comment`/`user_*` retention and the
"full text fidelity" role of `UdmfMap` — were motivated by "no write path yet
to round-trip them." That premise has since expired: the UDMF writer landed
with #60 (`write_udmf`/`add_udmf_map` in `map/udmf/write.rs`, `write`
feature), and format conversion (ADR-0019) made `cwad convert` a real
round-trip consumer.

What the code actually retains today, verified against
`crates/crustywad/src/map/udmf/`:

- `parse_udmf` drops every assignment it does not have a typed field for.
  `BlockState::Unknown` discards unrecognized blocks wholesale (header name
  included); each `*Builder::set_field` has a silent `_ => {}` arm for
  unrecognized fields inside recognized blocks (SPAC/Strife/port booleans,
  `comment`, `user_*`, `zfloor`/`zceiling`, …); and the global-scope loop
  keeps only `namespace`, dropping every other global assignment.
- `UdmfThing.flags` is a **lossy** fold (ADR-0019): `ThingBuilder::finish`
  packs `skill1|skill2` into bit 0 and `skill4|skill5` into bit 2, so which
  member of each pair was set is unrecoverable, and `class1`–`class3`,
  `dormant`, and `standing` are dropped entirely.
- `UdmfLinedef.flags` is, by contrast, a **lossless** fold: the 9 recognized
  linedef booleans correspond 1:1 to bits 0–8.
- The only write path is `Map`-sourced (`write_udmf(map: &Map, opts:
  &WriteOptions)`), so everything the assembled `Map` does not model is
  unwritable even when `UdmfMap` holds it.

Consequently #257's own claim that "`UdmfMap` already carries full text
fidelity" is not true for dropped assignments; this ADR makes it true and
adds the write half. The scoping decisions for this pass (made during #257
activation review):

1. **Consumer: lossless round-trip.** Read `TEXTMAP` → write it back with no
   assignment lost. Fidelity lives in `UdmfMap`; the assembled `Map` graph is
   untouched (no new `Map` fields, no changes to normalization output).
2. **Contract: semantic-lossless, not byte-identical.** Canonical output
   formatting is acceptable; a trivia-preserving CST (byte-identical output,
   lexical-comment retention) is explicitly deferred to a follow-up issue on
   the editor (#18) chain.
3. **Recorded here as a new ADR** rather than a third ADR-0017 amendment,
   because it supersedes one of ADR-0017's decided deferrals and adds a new
   public API surface.

### The lexer already bounds the value domain

Everything `parse_udmf` can retain is one of exactly four token shapes
(`map/udmf/lex.rs`): `Token::Bool(bool)`, `Token::Int(i64)`,
`Token::Float(f64)`, `Token::Str(String)`. `read_value` rejects any other
token in value position (a bare non-`true`/`false` keyword value is a
`Syntax` error today — unchanged by this ADR), string escapes are resolved at
lex time, and `scan_number` rejects any float literal that parses to a
non-finite `f64`. Two consequences this design leans on:

- a retained-value enum needs exactly four variants, and
- every value in a parsed `UdmfMap` is representable in output text, so a
  writer over `UdmfMap` can be **infallible**.

### A latent float-formatting hazard in the existing writer

`Writer::fmt_float` emits finite floats with `format!("{value}")`. Rust's
`Display` for `f64` never uses exponent notation, so a whole value like
`1e300` emits as ~301 bare digits — which re-lex as an *integer* literal,
overflow `i64`, and fail to parse. The `Map`-sourced writer has no round-trip
guarantee so this was survivable there; a guaranteed-round-trip writer cannot
inherit it. The lexer accepts exponent floats without a decimal point
(`is_float` flips on `e`/`E`), which gives the fix a clean target form.

## Decision drivers

- **The round-trip contract must be total over parseable input:** any
  document `parse_udmf` accepts must survive `parse → write → parse`
  unchanged, or the guarantee is a trap.
- **ADR-0016 hardening:** retention must stay `O(input)`, add no recursion,
  and extend the existing fuzz surface.
- **ADR-0013 restraint:** no speculative typed modeling of port extensions;
  the editor epic (#18) remains the future consumer for anything richer.
- **Non-breaking:** all five `Udmf*` structs and `UdmfMap` are
  `#[non_exhaustive]`, so field additions are additive; this pass should stay
  a `feat:` (patch bump under the pre-1.0 policy).
- **Legibility:** the writer must mirror the existing `write_udmf` emission
  conventions (defaults elided, same block order) so the two paths read as
  siblings.

## Considered options

### Retention representation

1. **Extras side-channel** — typed fields stay as-is; each block struct gains
   an ordered `extras: Vec<UdmfAssignment>` for everything not losslessly
   held by a typed field; `UdmfMap` gains `unknown_blocks` and
   `global_extras`.
2. **Fully typed standardized fields** — add the ~20 remaining standardized
   booleans as typed `bool` fields on `UdmfLinedef`/`UdmfThing`; extras only
   for unknown/`user_*`/port-specific fields.
3. **Generic ordered field lists** — represent every block as an ordered
   `Vec<(String, UdmfValue)>` with typed accessor views.

### Round-trip contract

- **Semantic-lossless (chosen):** `parse_udmf(write(m)) == m` — every
  assignment's value survives; formatting, block grouping, lexical comments,
  and value spelling (hex, e-notation, explicit defaults) canonicalize.
- **Byte-identical (deferred):** unmodified input writes back byte-for-byte;
  requires a trivia-preserving CST layer. Deferred to the #18 chain.

## Decision outcome

Chosen: **Option 1 (extras side-channel) with the semantic-lossless
contract.** Option 2 still needs the extras mechanism anyway (for
`user_*`/port extensions) while adding a large typed surface no consumer
reads today; Option 3 is ADR-0017's already-rejected Option 2 (stringly-typed
extraction) wearing a new hat. The extras representation is additive,
`O(input)`, and leaves the door open to typing individual hot fields later
(`#[non_exhaustive]` structs make that non-breaking too).

### 1. Retained-value model (`map::udmf::model`)

Three new public types (verified absent from the codebase today):

```rust
/// A UDMF assignment value. Mirrors the lexer's four value token shapes.
#[derive(Debug, Clone, PartialEq)]
pub enum UdmfValue {
    /// A `true`/`false` literal.
    Bool(bool),
    /// An integer literal (decimal or hex; the lexer holds `i64`).
    Int(i64),
    /// A floating-point literal (always finite; the lexer rejects
    /// overflow-to-infinity).
    Float(f64),
    /// A quoted string literal, escapes resolved.
    Str(String),
}

/// A retained `name = value;` assignment.
#[derive(Debug, Clone, PartialEq)]
pub struct UdmfAssignment {
    /// The field name (lowercased, as the parser normalizes identifiers).
    pub name: String,
    /// The assigned value.
    pub value: UdmfValue,
}

/// A retained block whose header identifier is not one of the five
/// standardized kinds (e.g. a port-specific block).
#[derive(Debug, Clone, PartialEq)]
pub struct UdmfUnknownBlock {
    /// The block header identifier (lowercased).
    pub name: String,
    /// The block's assignments, in declaration order (last-wins on
    /// duplicate names).
    pub fields: Vec<UdmfAssignment>,
}
```

Each of `UdmfVertex`, `UdmfLinedef`, `UdmfSidedef`, `UdmfSector`, `UdmfThing`
gains `pub extras: Vec<UdmfAssignment>`; `UdmfMap` gains
`pub unknown_blocks: Vec<UdmfUnknownBlock>` and
`pub global_extras: Vec<UdmfAssignment>` (global assignments other than
`namespace`, in declaration order). All additions are non-breaking: every
affected struct is `#[non_exhaustive]`, so no external struct literal can
exist. The module docs (which currently say every unrecognized field "is
parsed for syntactic validity and dropped") are updated to describe
retention.

**Routing rule.** An assignment enters `extras` iff it is not losslessly
reconstructible from a typed field:

| Block | Typed (not in extras) | Extras |
|---|---|---|
| `vertex` | `x`, `y` | everything else (`zfloor`, `zceiling`, `comment`, `user_*`, …) |
| `linedef` | `v1`, `v2`, `sidefront`, `sideback`, `id`, `special`, `arg0`–`arg4`, and the 9 flag booleans (`blocking`, `blockmonsters`, `twosided`, `dontpegtop`, `dontpegbottom`, `secret`, `blocksound`, `dontdraw`, `mapped` — bit↔bool 1:1, lossless) | everything else (SPAC activation booleans, `passuse`, Strife/port booleans, `comment`, `user_*`, …) |
| `sidedef` | `offsetx`, `offsety`, `texturetop`, `texturebottom`, `texturemiddle`, `sector` | everything else |
| `sector` | `heightfloor`, `heightceiling`, `texturefloor`, `textureceiling`, `lightlevel`, `special`, `id` | everything else |
| `thing` | `x`, `y`, `height`, `angle`, `type`, `id`, `special`, `arg0`–`arg4` | everything else, **plus** the 10 dual-stored booleans below |

**Thing dual-store.** The 10 recognized thing booleans (`skill1`–`skill5`,
`ambush`, `single`, `dm`, `coop`, `friend`) are both folded into
`UdmfThing.flags` (unchanged ADR-0019 packing — assembly's input) **and**
retained verbatim in `extras`, because the fold is not invertible
(`skill1|skill2` share bit 0, `skill4|skill5` share bit 2, and the game-mode
bits are negations). `UdmfThing.flags` is documented as *derived*; `extras`
is the round-trip source of truth. The writer never emits from `flags`
(§3). Duplicate assignments to one name within a block are last-wins in
`extras`, matching the typed fields' existing overwrite behavior.

### 2. The semantic round-trip contract

For every `text` accepted by `parse_udmf` (with `m = parse_udmf(text, limits)`):

```text
parse_udmf(m.to_textmap(), limits) == m
```

(`UdmfMap` and all its element types derive `PartialEq`.) Canonicalizations —
all semantic-equality-preserving, none data-losing:

- blocks regroup by kind (vertices, linedefs, sidedefs, sectors, things,
  then unknown blocks), preserving order **within** each kind (indices
  reference position within a kind, so cross-kind interleaving carries no
  meaning);
- assignments at their spec default elide (typed fields only; extras always
  emit — an extras name has no reliable default to elide against);
- hex integers emit as decimal; e-notation collapses to the canonical float
  form; string escapes re-emit via the existing `escape_udmf_string`;
- lexical `//` and `/* */` comments are trivia and do not survive (the
  standardized `comment` *string field* is an assignment and does — via
  extras);
- duplicate assignments collapse to their last value (which is what a
  re-parse of the original would observe anyway).

**Float formatting rule** (shared with the `Map`-sourced writer, fixing the
latent hazard described in Context). For **typed float fields** (`x`, `y`,
`height`), whose re-parse coerces through `as_f64` (which accepts integer
tokens): a finite `f64` that is whole and within `i64`'s exact conversion
range emits as bare integer digits (`i64 → f64` round-trips exactly for any
integer obtained from a whole `f64` in range); any other finite value emits
Rust's shortest-round-trip form (`format!("{value:?}")`), with a decimal
point or exponent guaranteed present — the lexer accepts exponent floats
without a dot, and shortest-round-trip formatting re-parses to the identical
`f64` by construction. A retained **extras `Float` value** is different: its
re-parse preserves the token's shape as the `UdmfValue` variant, so bare
digits would come back as `UdmfValue::Int` and break equality — extras
floats therefore always emit in the float-shaped `{:?}` form, whole or not
(amended during implementation, which caught the unscoped original rule
contradicting the contract). Non-finite values cannot occur in a parsed
`UdmfMap` (lexer guarantee), which is what makes the new writer infallible;
the `Map`-sourced path keeps its existing strict-error/lenient-warn handling
for non-finite coordinates, since `Map` values do not come with the lexer's
guarantee.

### 3. Writer API (`write` feature, `map/udmf/write.rs`)

```rust
impl UdmfMap {
    /// Serializes this document to canonical UDMF `TEXTMAP` text with the
    /// semantic round-trip guarantee: re-parsing the output yields a value
    /// equal to `self`.
    #[must_use]
    pub fn to_textmap(&self) -> String;
}

/// Adds a complete UDMF map group — the `name` marker lump, a `TEXTMAP`
/// lump holding `map.to_textmap()`, and an `ENDMAP` lump — to `builder`.
pub fn add_udmf_textmap(builder: &mut WadBuilder, name: &str, map: &UdmfMap);
```

Both are infallible (§2). Emission order: `namespace` declaration, global
extras, then blocks by kind; within a block, typed fields at non-default
values (required fields always emit), then that block's extras in retained
order. `UdmfThing.flags` is never emitted — it is not a UDMF field; the
discrete booleans emit from extras. Linedef flag bits decode back to their 9
booleans (lossless). The `Map`-sourced `write_udmf`/`add_udmf_map` keep
their exact signatures and strictness behavior, gaining only the shared
float-formatting fix.

### 4. Hardening (ADR-0016 checklist)

1. **Bounded allocation:** extras/unknown-block storage is proportional to
   the assignments present in the input — `O(input length)`, unchanged.
2. **No unbounded recursion:** the flat two-state parser loop and the flat
   writer loops are untouched; retention adds pushes, not calls.
3. **Fuzz target:** `fuzz_parse_udmf` gains the round-trip oracle — when a
   document parses, assert `parse(write(parse(text))) == parse(text)` and
   that the retained-assignment count is bounded by input length. The fuzz
   crate already builds `crustywad` with `nodebuild` (which implies `write`),
   so no feature plumbing is needed. `fuzz_assemble_udmf` is unaffected.
4. **Both strictness modes:** retention is strictness-independent —
   `parse_udmf` takes only `Limits`, the spec requires unknown fields to be
   ignored silently (no new warnings), and no new failure mode is introduced
   in either mode.

### 5. What this supersedes, and what stays deferred

- **Superseded from ADR-0017:** the "`*.comment` and `user_*` fields …
  parsed for syntax validity only, then dropped (no write path yet to
  round-trip them)" deferral, and spike-review decision 3's premise that the
  discrete thing booleans "stay in `UdmfThing`" only as folded bits — they
  are now retained discretely (in extras). ADR-0017's representation,
  grammar, depth-bounding, and `Map`-normalization decisions are all
  unchanged.
- **Still deferred, re-homed to follow-up issues filed with this ADR:**
  byte-identical/CST round-trip (trivia preservation, lexical comments,
  original interleaving — #18 chain); `Map`-graph enrichment (slope/plane
  fields, SPAC booleans in `Map`, a normalized thing-flag representation);
  and non-UTF-8 `TEXTMAP` decoding (ADR-0017 spike-review decision 5,
  premise unchanged: still no real fixture).

## Consequences

- **New public API (all additive):** `UdmfValue`, `UdmfAssignment`,
  `UdmfUnknownBlock`; `extras` on the five `Udmf*` structs;
  `UdmfMap::{unknown_blocks, global_extras}`; `UdmfMap::to_textmap` and
  `add_udmf_textmap` (`write` feature).
- **Non-breaking** (`#[non_exhaustive]` structs; no signature changes):
  ships as `feat(map):` → patch bump under the pre-1.0 versioning policy.
  `PartialEq` on the `Udmf*` types now compares retained extras too — a
  semantic change to equality, not an API break.
- **Good** — `cwad convert` and any future editor (#18) can round-trip UDMF
  maps without losing port extensions, `user_*` data, or `comment` fields.
- **Good** — the latent whole-huge-float emission bug in the existing writer
  is fixed as a side effect of the shared formatting rule.
- **Neutral** — `UdmfThing` carries its 10 dual-stored booleans twice (folded
  + extras); the invariant ("`flags` derived, extras authoritative") is
  documented on the field and enforced only by the parser being `UdmfMap`'s
  sole constructor (`#[non_exhaustive]` blocks external literals).
- **Bad** — canonical output means `cwad`-rewritten `TEXTMAP`s diff noisily
  against their sources until the CST follow-up lands; that trade was made
  explicitly (contract decision above).

## More information

- Tracking issue: #257 (this pass closes it). Predecessors: ADR-0017 (UDMF
  representation; its deferrals are this ADR's subject), ADR-0019 (format
  conversion; the thing-flag fold and the `Map`-sourced writer conventions),
  ADR-0016 (hardening checklist), ADR-0013 (restraint precedent for what
  stays deferred).
- Source anchors: `crates/crustywad/src/map/udmf/model.rs` (the five
  `#[non_exhaustive]` structs), `parse.rs` (`BlockState`, `read_value`, the
  `*Builder::set_field` drop arms, `ThingBuilder::finish`'s ADR-0019 fold,
  the namespace-only global loop), `lex.rs` (`Token`, `scan_number`'s
  finite-float guarantee, exponent-without-dot acceptance), `write.rs`
  (`write_udmf`, `add_udmf_map`, `Writer::fmt_float`, `escape_udmf_string`),
  `fuzz/fuzz_targets/fuzz_parse_udmf.rs`.
- Spec: `specs/udmf.txt` v1.1 — unknown fields "should be preserved" for
  round-trip (a recommendation this ADR now honors), silent-ignore rule for
  readers, the four value productions.
