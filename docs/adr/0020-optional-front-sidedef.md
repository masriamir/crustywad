# ADR-0020: Optional front sidedef (the `0xffff` sentinel applies to both sides)

- **Status:** Accepted
- **Date:** 2026-07-13
- **Deciders:** @masriamir
- **Tracking issue:** https://github.com/masriamir/crustywad/issues/252

## Context and problem statement

ADR-0015's assembled graph models a linedef's two sidedef references
asymmetrically: `MapLinedef.left` is `Option<SidedefIdx>` (`None` == the on-disk
`0xffff` "no back side" sentinel), but `MapLinedef.right` is a required
`SidedefIdx`. A binary linedef whose **front** (`right_sidedef`) field is
`0xffff` is therefore treated as an out-of-range cross-reference: strict
assembly fails with `MapAssembleError::DanglingReference`, and lenient assembly
recovers only by warning.

Real retail data exercises this path. Dogfooding the map assembler across a
17-WAD retail collection (2026-07-13) found exactly one strict failure:
`hexen_ex.wad` MAP08 linedef 668, whose raw record is `right_sidedef = 0xffff`,
`left_sidedef = 0xffff`, `flags = 0x0001` (blocking), `special = 0` — an
invisible, impassable line with no render surfaces on either side. Every other
map in the collection (551 classic/Hexen groups) assembles strict-clean.

**Verified engine behavior** (source inspected 2026-07-13, per the project rule
that format facts are never stated from memory):

- **Chocolate Doom** and **Chocolate Hexen** (`P_LoadLineDefs`,
  `src/doom/p_setup.c` / `src/hexen/p_setup.c`): both sidedef fields get the
  identical treatment — `if (ld->sidenum[0] != -1) ld->frontsector =
  sides[ld->sidenum[0]].sector; else ld->frontsector = 0;` and the same for
  `sidenum[1]`/`backsector`. A front of `0xffff` (read into a signed `short` as
  `-1`) is "no front side", not an error. The map loads and plays in
  vanilla-lineage engines.
- **GZDoom** (`LoadLineDefs`, `src/maploader/maploader.cpp`): front `NO_INDEX`
  prints `"Line %d has no first side."` and patches the reference to dummy
  sidedef `0` — accepted with a diagnostic, never rejected.

The formats themselves are asymmetric in the other direction:

- **Binary Doom/Hexen**: `right_sidedef`/`left_sidedef` are each `u16`, and
  `0xffff` is engine-sanctioned "none" for **either** field (above).
- **UDMF** (`udmf.txt`): `sidefront = <integer>; // Sidedef 1 index. No valid
  default.` — required — while `sideback` defaults to `-1`. A frontless linedef
  has **no valid UDMF representation**.

So strict mode currently rejects data that the engines defining the binary
format accept, and the graph model cannot represent a real retail map.

## Decision

### 1. `MapLinedef.right` becomes `Option<SidedefIdx>`

`None` means "no front side" — the binary `0xffff` sentinel — exactly mirroring
`left`. `Map::linedef_right` changes signature accordingly:

```rust
pub fn linedef_right(&self, l: &MapLinedef) -> Option<&MapSidedef>
```

This is a breaking pre-1.0 change to the ADR-0015 model (the cheapest window
for it), and this ADR amends ADR-0015 §1's linedef-resolution contract.

### 2. Binary read path: `0xffff` is the "no side" sentinel for both fields, in both modes

Assembly maps a binary `right_sidedef == 0xffff` to `right: None` in **both**
strictness modes, with **no warning** — identical to the existing `left`
treatment. It is valid, vanilla-sanctioned data, not a defect to recover from.
Any other out-of-range value (e.g. an index past the sidedef arena) keeps the
existing behavior: strict `DanglingReference` error, lenient `None` + warning.

### 3. UDMF read path: `sidefront` stays required; dangling resolves to `None`

The UDMF parser continues to reject a linedef block with no `sidefront`
assignment (spec: no valid default). At normalization, an out-of-range or
negative `sidefront` (including a port-written `-1`) resolves like any dangling
optional reference: strict error, lenient `right: None` + `DanglingReference`
warning — replacing the previous lenient clamp-to-index-0, which fabricated a
reference not present in the source.

### 4. Write paths

- **`map::doom::write`**: `right: None` serializes as `0xffff` — a frontless
  binary linedef round-trips losslessly. A hand-constructed
  `Some(SidedefIdx(0xffff))` remains a strict-mode error (`0xffff` is the
  reserved sentinel, `NO_SIDEDEF`), as it is for `left` today.
- **`map::udmf::write`**: `right: None` is unrepresentable in UDMF. Strict mode
  fails with a new `UdmfWriteError::NoFrontSide { index }`; lenient mode writes
  `sidefront = -1;` (tolerated by ports, per GZDoom's own load-time handling)
  and records a new `UdmfWriteWarning::NoFrontSideDefaulted { index }`. This
  extends ADR-0019's asymmetric-reversibility inventory: Doom → UDMF conversion
  of a frontless line is lossy by necessity of the UDMF spec.

## Considered options

1. **`right: Option<SidedefIdx>` + sentinel symmetry (chosen)** — faithful to
   the verified on-disk contract; retail data assembles strict-clean; binary
   round-trip is lossless. Cost: a breaking model change and `Option` handling
   for consumers.
2. **GZDoom-style clamp** — keep `right` required; map front `0xffff` to
   `SidedefIdx(0)` + warning in both modes. Non-breaking, but fabricates a
   reference that is not in the file: a read → write round-trip silently
   rewrites map bytes, which is data corruption for an I/O-fidelity library.
   GZDoom can afford this because it is a renderer, not a round-tripping
   serializer.
3. **Document the status quo** — leave strict rejection in place and document
   that some retail maps require lenient mode. Zero code cost, but strict mode
   stays permanently stricter than the engines that define the format, and the
   retail-IWAD sweep test (#254) would carry a MAP08 allowlist entry forever.

## Consequences

- `hexen_ex.wad` MAP08 — and any map using the vanilla-sanctioned frontless
  idiom — assembles strict-clean; the #254 sweep needs no allowlist.
- Breaking API change: `MapLinedef.right`, `Map::linedef_right`. Downstream
  consumers must handle `None` (rare in practice: one linedef in one map across
  the 17-WAD retail collection).
- UDMF conversion of frontless lines is explicitly lossy (strict error /
  lenient default + warning), documented alongside ADR-0019's other
  reversibility limits.
- ADR-0015 §1's "right sidedef is required" resolution rule is superseded by
  this ADR; ADR-0015 is otherwise unchanged.

## More information

- Verified sources: Chocolate Doom/Hexen `P_LoadLineDefs` (master, 2026-07-13),
  GZDoom `maploader.cpp` (master, 2026-07-13), `udmf.txt` §linedef.
- Relates to: ADR-0015 (graph model; amended here), ADR-0019 (conversion
  reversibility inventory; extended here), ADR-0003 (strict/lenient contract),
  #252 (tracking issue), #254 (retail-IWAD sweep).
