# ADR-0013: `lump_by_name` lookup strategy

- **Status:** Accepted
- **Date:** 2026-07-06
- **Deciders:** @masriamir
- **Tracking issue:** https://github.com/masriamir/crustywad/issues/4

## Context

`Wad::lump_by_name` performs a linear scan over `self.lumps`, comparing each `Lump`'s name
until the first match:

```rust
pub fn lump_by_name(&self, name: &str) -> Option<&Lump> {
    self.lumps.iter().find(|lump| lump.name() == name)
}
```

Issue #4 asked whether this O(n) scan is a bottleneck worth optimizing, with an explicit
instruction to benchmark first and "only if a measurable bottleneck is confirmed, implement
an optimization." ADR-0012 landed the Criterion infrastructure needed to answer this with
data rather than intuition, including dedicated `lump_by_name_hit`, `lump_by_name_hit_last`,
and `lump_by_name_miss` benchmarks in `crates/crustywad/benches/read_ops.rs`.

## Decision drivers

- Doom WAD semantics: lump names are not unique, and `lump_by_name` must keep returning the
  *first* match. Any indexed alternative has to preserve this ordering guarantee.
- YAGNI: added complexity (extra memory, an index structure, first-match-preservation logic)
  is only worth it if the current approach is a measured bottleneck.
- `Wad` is immutable after construction (no `&mut self` methods exist), so an index, if ever
  added, would not need invalidation handling.
- Real usage: nothing in the crate's library or CLI production code calls `lump_by_name`
  today (it is exercised only by the benchmarks and integration tests that measure it) — it
  is a pure downstream-consumer API. The roadmap work that would plausibly call it in a loop
  (graphics, texture composition, audio lump lookup — issues #156, #157, #158) has not been
  implemented yet.

## Considered options

1. Keep the existing linear scan unchanged.
2. Add a lazily-built `HashMap<String, usize>` first-index cache (e.g. via `OnceLock`),
   populated on first call to `lump_by_name`.
3. Build an eager index at parse time, stored alongside `lumps` on every `Wad`.

## Decision outcome

Chosen option: **keep the existing linear scan (option 1)**, because the benchmark data shows
no measurable bottleneck and the crate has no current caller that would benefit from a faster
path.

Measured on the existing `lump_access` Criterion group (100-lump synthetic WAD, all lumps
sharing one name to force worst-case behavior):

| Case | Time |
|---|---|
| First-match hit | ~7.3ns |
| Worst-case hit (last of 100 lumps) | ~44.6ns |
| Worst-case miss (full scan, no match) | ~53.0ns |

Extrapolated linearly to a realistic full IWAD (~2,000–3,000 lumps, the rough scale of
`doom2.wad`/`freedoom2.wad` — 20–30x this benchmark's lump count), worst case lands around
0.9–1.6 microseconds. That is the cost of a single `lump_by_name` call; no production code in
the crate calls it in a loop today.

### Consequences

- Good, because issue #4 closes with the decision backed by real numbers instead of
  intuition, and with zero implementation risk (no code change to a public, widely-visible
  API).
- Good, because `Wad`'s existing simplicity (no cached, invalidatable state) is preserved.
- Neutral, because the O(n) cost remains and will scale with lump count — this is
  unsurprising and was never in question; only whether it mattered in practice.
- Bad, because if a future consumer (e.g. texture composition resolving many patch names,
  issue #156/#157) calls `lump_by_name` in a hot loop, this decision will need revisiting.
  That trigger condition is captured below rather than guessed at now.

## Pros and cons of the options

### Option 1: keep the linear scan

- Good, because it requires no code change, no new invariants to maintain, and no added
  memory.
- Good, because the benchmark data shows it is already fast in absolute terms at realistic
  scale.
- Bad, because it is still O(n), which will show up if a future caller uses it in a loop over
  many lookups.

### Option 2: lazy `HashMap` first-index cache

- Good, because it would turn repeated lookups into O(1) after the first call.
- Bad, because it requires preserving first-match semantics carefully (the map must store the
  *first* occurrence's index per name, not the last), adding a subtle invariant a future
  contributor could get wrong.
- Bad, because it adds memory and a `OnceLock`/interior-mutability field to `Wad` for a
  problem the data does not show exists today.

### Option 3: eager index at parse time

- Good, because it removes any first-call latency spike that option 2's laziness would carry.
- Bad, because it pays the index-construction cost on every `Wad`, including the common case
  where `lump_by_name` is never called at all.
- Bad, because same first-match-preservation risk as option 2.

## More information

- Benchmarks: `crates/crustywad/benches/read_ops.rs` (`lump_access` group), infrastructure
  from ADR-0012.
- Related ADR: ADR-0012 (Criterion benchmarking infrastructure) — provided the data this
  decision is based on.
- Revisit condition: if issues #155 (map lump graph assembly), #156 (graphics/patches), #157
  (texture composition), or #158 (audio lumps) introduce a consumer that calls
  `lump_by_name` in a loop, re-benchmark with that real workload before reconsidering options
  2 or 3 — don't speculate ahead of that data either.
