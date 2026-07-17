# ADR-0016: Parser and assembly hardening policy

- **Status:** Accepted
- **Date:** 2026-07-08
- **Deciders:** @masriamir
- **Tracking issue:** https://github.com/masriamir/crustywad/issues/195

## Context and problem statement

`crustywad` parses untrusted input: `Wad::from_bytes` /
`Wad::from_bytes_with_options` (header + directory), `parse_records::<T>` (map
record lumps), and — once ADR-0015 lands — `Map::assemble` (graph assembly).
Epic #17 adds three more parse surfaces (Hexen, Doom 64, and the UDMF *text*
format), and ADR-0015 adds cross-reference resolution. Before that surface
multiplies, this ADR sets one explicit policy for keeping every parse path
resistant to malicious input, so each new format inherits a checklist rather than
re-deriving hardening ad hoc.

This is a stated "now" item of the foundation sequence and is distinct from the
repository-settings hardening tracked separately (#188, which covers CI/action
pinning and branch rulesets, **not** the parser).

### Threat model: availability, not memory safety

The core crate is `#![deny(unsafe_code)]` (unsafe is quarantined to `mmap.rs`,
ADR-0005 lineage). Rust's safety guarantees therefore rule out the
memory-corruption / RCE bug classes for the parsing code: an out-of-bounds access
panics rather than reading arbitrary memory. The residual threat from adversarial
input is **denial of service** — an unexpected `panic`/abort, unbounded memory
allocation (OOM), unbounded work, or stack overflow from deep recursion. This
ADR's scope is those availability failures. (Fuzzing under sanitizers, ADR-0009,
still guards the `mmap.rs` unsafe boundary against UB.)

### What is already hardened (verified)

The header/directory path is already overflow- and allocation-safe:

- Header field coercion rejects or clamps negative `numlumps` / `infotableofs`
  (`coerce_i32`), and directory-span multiplication is `checked_mul` with a
  strict-error / lenient-`usize::MAX`-warning fallback (`lib.rs`).
- The directory allocation `Vec::with_capacity(lump_count)` (in `parse_bytes`)
  is bounded: in strict mode it is only reached after the out-of-bounds check
  proves the directory fits — `info_table_offset + numlumps * 16 <= len` — and in
  lenient mode `lump_count` is capped at `available_entries`, i.e.
  `(len - info_table_offset) / 16`. Either way capacity is `O(input length)`.
- `parse_records`'s `Vec::with_capacity(bytes.len() / record_size)`
  (in `map.rs`) is likewise bounded by the input slice length.
- The parser is fully **iterative** — there is no recursion in any current parse
  path, so no stack-overflow surface exists today.
- ADR-0009's `fuzz_wad_lenient` target already asserts a bound on warning-vector
  growth (`warnings <= lump_count * 5 + 5`), a hardening invariant this ADR
  generalizes.

So the existing binary parser is in good shape; the risk is in the *new* surfaces.

### Where the new risk is

- **New binary formats (Hexen, Doom 64).** Same fixed-record shape as Doom; the
  `O(input)` allocation property holds *if* each new record type preserves it.
  The risk is regression, not a new class.
- **Map assembly (#155 / ADR-0015).** Cross-reference resolution over
  record-count-sized arenas is `O(records)` = `O(input)`; no unbounded expansion,
  but this must be verified and fuzzed.
- **UDMF (text, #57–#58).** The genuinely new risk. UDMF is an arbitrarily
  nested text grammar, so a naive recursive-descent parser has a **stack-overflow
  surface** independent of input *size* (a small file can request deep nesting).
  Allocation remains `O(input)`, but recursion depth is not naturally bounded.

## Decision drivers

- Untrusted input across an expanding set of entry points.
- Safety guarantees narrow the threat to DoS — the policy should target that, not
  over-engineer against RCE classes that cannot occur in safe code.
- **Restraint (ADR-0013 lineage):** do not add configurable caps the data does
  not justify; the binary path is already input-bounded.
- New PRs should inherit a fixed, checkable standard.

## Considered options

1. **Ad hoc** — rely on the existing overflow-hardening and add fuzz targets
   opportunistically per format, with no written policy.
2. **Global resource budget** — thread a total-allocation and/or wall-time budget
   through every parse and assembly call.
3. **Targeted defense-in-depth** — codify the `O(input)` allocation invariant and
   fuzz-guard it for all binary paths, add a *depth* limit only where input shape
   is genuinely unbounded (UDMF), expand fuzz coverage per format, and attach a
   per-PR hardening checklist.

## Decision outcome

Chosen option: **Option 3 — targeted defense-in-depth.** It hardens the real
(availability) threat where it actually appears, without adding speculative
configuration to paths the input size already bounds.

### 1. Codify and fuzz-guard the `O(input)` allocation invariant

Every binary parse path (header/directory, `parse_records::<T>` for all formats,
and `Map::assemble` arenas) must allocate memory in `O(input length)`. This is an
invariant, not a config knob. Each format's fuzz target asserts an output-size
bound analogous to ADR-0009's warning bound — e.g. record/element counts are
bounded by `input_len / min_record_size`, and warning/error vectors by a linear
function of that. No configurable cap is added to binary paths, per the restraint
driver.

### 2. UDMF parsing must be depth-bounded

The UDMF parser (landing in #57–#58) must be **iterative, or recursive with an
explicit depth counter** that fails cleanly (a `ParseError` / UDMF error, never a
stack overflow) when a configurable maximum nesting depth is exceeded. To carry
that bound, introduce a minimal `Limits` type surfaced through `ParseOptions`:

```rust
/// Resource limits applied to parsing and assembly of untrusted input.
///
/// Binary formats are naturally bounded by input length and are unaffected by
/// these limits; they exist to bound input shapes that input *size* does not —
/// currently only UDMF nesting depth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Maximum nesting depth for structured text formats (UDMF). Parsing fails
    /// with an error rather than recursing (and risking stack overflow) beyond
    /// this. Default: `64`.
    pub max_depth: usize,
}

impl Default for Limits {
    fn default() -> Self { Self { max_depth: 64 } }
}
```

`ParseOptions` gains a `limits: Limits` field (default `Limits::default()`), so
the same options value already threaded through `Wad::*_with_options`,
`Map::assemble_with_options` (ADR-0015), and the format dispatch (ADR-0014)
carries the bound. `Limits` is introduced **only when UDMF lands**; until then it
has no effect on any binary path. Adding the field is a minor, additive breaking
change to `ParseOptions` literal construction, absorbed with the other pre-1.0
reorganizations; the `strict()` / `lenient()` constructors keep working
unchanged.

### 3. Expand `cargo-fuzz` coverage per surface

Extend the ADR-0009 harness (which explicitly anticipated this) with one target
per new surface, each structurally identical to the existing three (arbitrary
bytes / text in, no-panic oracle, `Err(...)` allowed, plus the §1 size-bound
assertion):

- `parse_records::<T>` for each new binary record type (Hexen, Doom 64), added
  with that format's PR.
- A **map-assembly** target (`Map::assemble` over a fuzzed WAD), added with #155.
- A **UDMF text** target (`str`-oriented), added with #57–#58, seeded with valid
  and malformed `TEXTMAP` samples and pathologically deep nesting.

Each target lands *in the same PR* as the surface it covers, with committed seed
corpus (ADR-0009 §4).

### 4. Tighten the fuzz cadence once the surface grows

ADR-0009 §3 flagged that the weekly `fuzz.yml` schedule should be revisited when
"write support or complex graph assembly" arrives. This ADR pulls that trigger:
once assembly (#155) and UDMF land, tighten the schedule (e.g. nightly) and/or
adopt OSS-Fuzz for continuous coverage (the existing `fuzz/` package is directly
reusable). Fuzzing remains non-blocking for PRs.

### 5. Per-PR hardening checklist

Every PR that adds a parse or assembly surface must satisfy, and state in its
description, all of:

1. Allocation is `O(input length)` (or, for text, explicitly depth-/count-limited
   via `Limits`).
2. No unbounded recursion — iterative, or depth-guarded against `Limits::max_depth`.
3. A `cargo-fuzz` target exists with the no-panic oracle and a §1 output-size
   assertion, plus committed seed corpus.
4. Both `Strictness` modes reject or recover from malformed input without panicking.

This checklist is added to the "Adding a new lump type" / "Adding a new format"
guidance in `.claude/CLAUDE.md`.

## Consequences

- **New public API:** the `Limits` type and a `ParseOptions.limits` field, both
  introduced with UDMF (#57–#58), not now. Default behavior is unchanged for all
  binary formats.
- **Good** — new formats inherit a fixed hardening standard; reviewers have a
  concrete checklist instead of case-by-case judgment.
- **Good** — the policy matches the real threat (DoS) and adds no speculative
  configuration to already-bounded binary paths, honoring ADR-0013's restraint.
- **Neutral** — the `ParseOptions` field addition is a minor breaking change to
  struct-literal construction, sequenced with the other pre-1.0 reorganizations
  (ADR-0014); the convenience constructors are unaffected.
- **Fuzz maintenance grows** — one target per format/surface and a tighter
  schedule (or OSS-Fuzz) increase CI/dev tooling upkeep; accepted as the cost of
  broad untrusted-input coverage.
- **Write path is separate (flagged).** Write-side validation (size/offset/name
  bounds) is already covered by ADR-0006 and is out of scope here; this ADR is
  about *reading* untrusted input.
- **Depends on / feeds:** ADR-0009 (fuzz harness — extended), ADR-0014 (per-format
  dispatch that must honor the checklist), ADR-0015 (assembly target + arena
  bound). The `Limits` plumbing is consumed by the UDMF work (#57–#58).

## Pros and cons of the options

### Option 1 — ad hoc

- Good, because it adds no new types or policy surface now.
- Bad, because each new format re-derives hardening (or forgets it); the warning-
  bound style of invariant would be applied inconsistently.
- Bad, because UDMF's stack-overflow risk is easy to miss without an explicit
  depth rule.

### Option 2 — global resource budget

- Good, because a single budget would cap total work regardless of path.
- Bad, because threading an allocation/time budget through every call is
  invasive and speculative for binary paths that input size already bounds
  (contra ADR-0013).
- Bad, because a wall-time budget introduces nondeterminism into a pure parsing
  API and complicates testing.

### Option 3 — targeted defense-in-depth (chosen)

- Good, because it hardens exactly the surfaces that need it and documents an
  invariant the code already satisfies.
- Good, because the one new knob (`Limits::max_depth`) exists only where input
  shape is genuinely unbounded.
- Good, because the per-PR checklist makes the standard enforceable in review.
- Bad, because it relies on reviewer diligence to apply the checklist (mitigated
  by putting it in `CLAUDE.md` and the fuzz requirement).

## More information

- Related ADRs: ADR-0005 (isolate unsafe), ADR-0009 (`cargo-fuzz` harness — the
  base this extends), ADR-0013 (restraint on speculative additions), ADR-0014
  (multi-format dispatch), ADR-0015 (assembly arenas + fuzz target).
- Source anchors: `crates/crustywad/src/lib.rs` (`coerce_i32`, `checked_mul`,
  `Vec::with_capacity(lump_count)` at the directory loop), `crates/crustywad/src/map.rs`
  (`parse_records` capacity bound), `fuzz/fuzz_targets/` (existing three targets),
  `.github/workflows/fuzz.yml`.
- Revisit condition: if a future format needs a bound that input size does not
  provide beyond nesting depth (e.g. a compression format with a decompression
  ratio limit), extend `Limits` with that specific field rather than adding a
  global budget.

## Amendment (2026-07-17, #157): `Limits` grew and is now `#[non_exhaustive]`

The `Limits` sketch above predates #157: the struct now also carries
`max_composite_pixels` (the texture-composition allocation cap, enforced in
both strictness modes) and is `#[non_exhaustive]`, so struct-literal
construction no longer compiles — construct via `Limits::new()` and the
`with_max_depth`/`with_max_composite_pixels` setters. Future limits land
without further breaking changes.
