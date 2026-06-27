# ADR-0010: Proptest invariant testing strategy

- **Status:** Proposed
- **Date:** 2026-06-14
- **Deciders:** @masriamir
- **Tracking issue:** https://github.com/masriamir/crustywad/issues/46

## Context

`proptest` is already a dev-dependency and one property-based test exists in
`crates/crustywad/tests/wad_reader.rs` (the `strict_parser_handles_generated_empty_wads`
test). That test generates only the two valid magic values and checks that a zero-lump
WAD parses without error. It does not cover the much larger class of invariants that
the parser must uphold regardless of what bytes it receives.

Four design questions need to be decided before writing more proptest tests:

1. Which invariants should be tested with property-based tests rather than
   hand-crafted unit tests?
2. What generation strategy should produce inputs (structured WAD bytes vs. raw
   arbitrary bytes vs. both)?
3. Where in the crate directory tree do the tests live?
4. How should proptest be configured (number of cases, CI settings, regression
   file policy)?

These decisions interact: a raw-bytes strategy trivially covers the "no panic"
class of invariants but cannot express structured invariants such as "`wad.lump_count()`
equals `wad.lumps().len()`"; those require a structured generator.

## Decision

### 1. Invariants to test

The following invariants are chosen because they are not easily exhausted by
hand-crafted tests yet are provable by random generation:

| # | Invariant | Test classification |
|---|---|---|
| I-1 | **No panic on arbitrary bytes** — `Wad::from_bytes` never panics regardless of input content or length, in both strict and lenient modes. An `Err` result is acceptable; a panic is not. | Safety / robustness |
| I-2 | **Lump count consistency** — for a successfully parsed WAD, `wad.lump_count()` equals `wad.lumps().len()` and equals `wad.header().num_lumps`. | API contract |
| I-3 | **`lump_by_name` / `lumps()` agreement** — for every `Lump` returned by `lumps()`, `lump_by_name(lump.name())` returns `Some`. No name returned by any lump is absent from the `lump_by_name` lookup. | API contract |
| I-4 | **Lump name ASCII validity and length** — every name returned by a successfully parsed lump satisfies `name.is_ascii()` and `name.len() <= 8` in strict mode. Lenient mode may return non-ASCII names only when a `ParseWarning::NonAsciiName` warning accompanies them. | Correctness |
| I-5 | **`parse_records` no-panic** — `map::parse_records::<T>` never panics on arbitrary byte slices for any map-record type (`Thing`, `Linedef`, `Sidedef`, `Vertex`, `Seg`, `Subsector`, `Node`, `Sector`). A `MapParseError` result is acceptable. | Safety / robustness |
| I-6 | **Strict errors / lenient warnings correspondence** — for any input `bytes: Vec<u8>` that causes `from_bytes_with_options(bytes.clone(), ParseOptions::strict())` to return `Err(e)`, calling `from_bytes_with_options(bytes, ParseOptions::lenient())` either also returns `Err` (for unrecoverable errors such as a truncated header) or returns `Ok` with a non-empty `warnings()` slice. A strict error must never silently disappear in lenient mode. (Proptest implementations must clone before the strict call since `from_bytes_with_options` consumes its input via `Into<Vec<u8>>`.) | Correctness |
| I-7 | **`lump_bytes` bounds safety** — for every lump index `i < wad.lump_count()`, `wad.lump_bytes(i)` returns `Some` and the returned slice is fully within the original input bytes (no out-of-bounds access). | Safety / correctness |
| I-8 | **`parse_records` trailing-bytes semantics** — scoped to types where `BinRead` consumes at least one byte per record (i.e. `record_size > 0`; all fixed-size map-record types in this module satisfy this). For any non-empty byte slice whose length is not an exact multiple of `record_size`, `parse_records::<T>` returns `Err(MapParseError::TrailingBytes { .. })`. For any byte slice whose length is an exact multiple, the function never returns `Err(MapParseError::TrailingBytes { .. })`; if it returns `Ok`, the `Vec` has `bytes.len() / record_size` elements. (`record_size` is inferred by parsing the first record — it is not `size_of::<T>()`, which may include in-memory alignment padding. A length-multiple slice may still yield `Err(MapParseError::Binrw(_))` if a record fails to decode.) | Correctness |

### 2. Generation strategy

Three options were considered:

**Option A — structured WADs only (`build_wad`-based):** Generate valid WAD
bytes using the existing `common::build_wad` helper with arbitrary names, lump
counts, and payloads. This is straightforward to write and always produces
structurally well-formed WAD bytes (correct header offsets, consistent
directory), but strict parseability also requires constraining the generator
(magic ∈ {`IWAD`, `PWAD`}, ASCII-only names). Without those constraints, valid
WAD structure is no guarantee of successful strict parsing. It also cannot reach
the adversarial / malformed-input invariants (I-1, I-6) without additional
mutation passes.

**Option B — raw arbitrary bytes only:** Feed proptest's `vec(any::<u8>(), 0..4096)`
directly to `from_bytes`. This trivially covers I-1 and I-6 but cannot express
the structured invariants (I-2, I-3, I-4, I-7) because those require a
successfully parsed WAD, which raw bytes rarely produce.

**Option C — both strategies, matched to invariant class (recommended):**
- Use a structured `arb_valid_wad` strategy (built on `build_wad`) for
  invariants that require a successfully parsed WAD: I-2, I-3, I-4, I-7.
- Use raw `vec(any::<u8>(), 0..MAX_RAW_BYTES)` for no-panic and
  strictness-correspondence invariants: I-1, I-6.
- Use `vec(any::<u8>(), 0..4096)` directly for map-record invariants: I-5, I-8.

Option C is chosen. It prevents the test suite from being both too weak (A alone)
and too vacuous (B alone, where most runs hit `Err` and verify nothing structural).

The `arb_valid_wad` strategy will be defined in `crates/crustywad/tests/common/mod.rs`
alongside `build_wad`, so all proptest files share it without duplication:

```rust
/// A proptest `Strategy` that produces structurally valid WAD bytes.
pub fn arb_valid_wad() -> impl Strategy<Value = Vec<u8>> {
    let kind = prop_oneof![Just(*b"IWAD"), Just(*b"PWAD")];
    let lumps = proptest::collection::vec(arb_lump_pair(), 0..=16);
    (kind, lumps).prop_map(|(k, pairs)| {
        let refs: Vec<(&str, &[u8])> = pairs
            .iter()
            .map(|(n, d)| (n.as_str(), d.as_slice()))
            .collect();
        build_wad(k, &refs)
    })
}
```

A companion `arb_lump_pair()` strategy generates ASCII names of 1–8 characters
and payloads of 0–256 bytes.

### 3. Test placement

Per `.claude/CLAUDE.md`, property-based tests live in the same integration test file as
the regular tests they complement:

- Invariants I-1, I-2, I-3, I-4, I-6, I-7 → `crates/crustywad/tests/wad_reader.rs`
  (alongside the existing WAD reader proptest).
- Invariants I-5, I-8 → `crates/crustywad/tests/map_records.rs` (alongside the
  typed map-record integration tests).

No new test files are created. The `arb_valid_wad` and `arb_lump_pair` helpers
are added to `crates/crustywad/tests/common/mod.rs`.

### 4. Proptest configuration

**Case count:** Use proptest's default of 256 cases per property locally. In CI
the `PROPTEST_CASES` environment variable is not set — CI runs with the same 256
cases as local development. This keeps CI times predictable and avoids the common
trap of a low local count that misses regressions found only on CI. If a specific
invariant is identified as needing more exploration, a per-test
`ProptestConfig { cases: N, .. }` can override the default without affecting
other tests.

**Regression files (`.proptest-regressions/`):** Regression files should be committed
to the repository when produced. Proptest appends a minimal failing seed whenever it shrinks a
counter-example; committing those files ensures that the exact failure is always
replayed on every subsequent `cargo test` run, even after the fix, until the file
is explicitly removed. A `.gitignore` entry is intentionally not added for these
files.

**`MAX_RAW_BYTES`:** The raw-bytes strategy caps input size at 8 192 bytes. This
is large enough to exercise multi-lump directories without making test runs
prohibitively slow on the MSRV toolchain (Rust 1.85.0).

## Consequences

- Four proptest blocks will be added to `wad_reader.rs` and two to `map_records.rs`.
  Existing hand-crafted tests will not be removed; proptest will complement them.
- `common/mod.rs` will gain `arb_valid_wad` and `arb_lump_pair`. These will be public
  within the test crate only (`pub` inside `tests/`).
- CI wall-clock time increases by the time required to run 256 × 7 = 1 792
  proptest cases across all property tests (the 6 new blocks plus the existing
  `strict_parser_handles_generated_empty_wads` property in `wad_reader.rs`). On
  a contemporary laptop each case is sub-millisecond, so the total overhead is
  under two seconds.
- If proptest shrinks a counter-example in CI, it will appear in the job log and
  a regression seed will be emitted. Because the seed is not automatically
  committed by CI, a developer must copy it into the local regression file and
  commit. This is an acceptable manual step given that CI jobs are read-only.
- Issue #47 (implementing the proptest blocks) is unblocked once this ADR is
  accepted.
