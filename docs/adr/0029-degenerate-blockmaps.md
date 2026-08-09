# ADR-0029: Degenerate blockmaps — discard, don't patch

- **Status:** Accepted
- **Date:** 2026-08-08
- **Deciders:** @masriamir
- **Tracking issue:** https://github.com/masriamir/crustywad/issues/422

## Context and problem statement

Modern megawad maps routinely exceed what the vanilla `BLOCKMAP` lump can
represent: block-list offsets are 16-bit words, so a lump larger than
65,536 words cannot address its own content, and node builders knowingly
emit truncated/degenerate blockmaps for such maps because source ports
rebuild the blockmap at load time anyway (ADR-0024 already builds on this:
ports "rebuild missing/oversized BLOCKMAP and REJECT"). Eviternity.wad
MAP15/MAP29/MAP32 (37,776 / 29,699 / 60,375 linedefs) are live examples:
found via crustyview, all three failed `Map::assemble` (strict default) on
crustywad 0.9.4 with `DanglingReference { from: "blockmap block" }`, while
lenient assembly "recovered" by emptying each defective block — 4,357 /
7,889 / 211 warnings respectively — and keeping the rest.

Strict-by-default was working as designed (the lump *is* malformed), but
lenient's per-block patching had two defects:

1. **Wrapped offsets corrupt the lump globally.** An entry that passes
   range validation can still be garbage (the offset that led to it
   wrapped), so a "recovered" blockmap is plausible-looking wrong data —
   worse than no blockmap for a collision-style consumer.
2. **An O(blocks) warning flood** for what is a single condition: the
   lump is unusable.

Prior art in the crate: lenient assembly already degrades the *whole BSP*
(empty arenas, one warning) when a BSP reference cannot be clamped —
optional derived data never fails a lenient assembly, and never survives
partially. The blockmap's per-block patching was the outlier.

## Decision

1. **Lenient mode discards a defective blockmap wholesale.** On the first
   block-level defect — out-of-lump block offset, unterminated list, or
   dangling linedef reference; blocks scanned in index order, checks in
   that order within a block — `MapBlockmap::parse` records a single
   `MapWarning` (the existing variant for that defect kind, carrying the
   first defective block's diagnostics) and yields `Ok(None)`, exactly as
   the `MalformedBlockmap` structural path already did. No partial
   blockmap is ever surfaced: `Map::blockmap()` is `Some` only when every
   block decoded cleanly.
2. **Strict mode is unchanged** — the first defect is an error. A
   validator must keep reporting that the lump is garbage; engine-style
   silent dropping or rebuilding would hide real information (`cwad
   validate` relies on it).
3. **Consumers that never read a lump can skip it.**
   `MapGroup::without_lumps(&wad, &["BLOCKMAP", ...])` returns a filtered
   copy of the group; absent `REJECT`/`BLOCKMAP` lumps decode to `None`
   with no error and no warning in both modes, so e.g. a 2D viewer can
   strictly validate everything it actually consumes. This formalizes the
   pattern the CLI already used internally (ZNODES and node-lump exclusion
   ahead of a rebuild). It is deliberately a **group transformation, not
   an assemble option**: assembly keeps exactly one robustness knob
   (`Strictness`), and the method composes with all three assemble entry
   points without new variants.

## Consequences

- Eviternity MAP15/29/32 assemble leniently with one warning each and
  `blockmap()` = `None`; strict assembly still rejects them. Downstream
  viewers choose lenient mode or `without_lumps`.
- No API shape changed: the three warning variants keep their names and
  fields (only Display text and docs changed); `without_lumps` is purely
  additive.
- The retail sweep's zero-warning invariant is unaffected: Eviternity is
  not part of the sweep collection (it strict-fails by design, which the
  sweep treats as a policy decision recorded here, not an exception to
  carve out).
- The `fuzz_parse_reject_blockmap` oracle tightens: a kept blockmap
  implies zero warnings; every discard/error path warns at most once.
