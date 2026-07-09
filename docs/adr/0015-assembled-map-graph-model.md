# ADR-0015: Assembled map graph model

- **Status:** Proposed
- **Date:** 2026-07-08
- **Deciders:** @masriamir
- **Tracking issue:** https://github.com/masriamir/crustywad/issues/155

## Context and problem statement

Milestone 1 gives callers *flat* typed records: `parse_records::<T>(bytes)`
(`crates/crustywad/src/map.rs`) turns one lump's bytes into a `Vec<Thing>`,
`Vec<Linedef>`, `Vec<Vertex>`, and so on. Nothing connects them. A `Linedef`
holds `start_vertex: u16` / `right_sidedef: u16` as bare indices; a `Sidedef`
holds `sector: u16`; a `Node` encodes child links in the high bit of a `u16`.
Downstream consumers — the map viewer (#64), a future editor (#18), any geometry
query — need those indices *resolved* into a navigable structure, with the
cross-references validated once rather than re-checked at every access.

This ADR is the deliverable of #155 (roadmap milestone 2) and defines:

1. how a single map's lumps are identified within the flat directory
   (the `MapGroup` type that ADR-0014's `detect_map_format` consumes);
2. the assembled-map data model and how it is constructed from parsed records;
3. cross-reference validation under `Strictness::Strict` vs `Strictness::Lenient`;
4. the public API for requesting an assembled graph versus raw typed records.

It is also a stated prerequisite of ADR-0014: that ADR defers the `MapGroup` type
and the requirement that the assembled model accommodate UDMF floating-point
geometry to this decision.

### What "assembly" must resolve

Anchored to the current record definitions in `map.rs`:

- `Linedef.start_vertex` / `end_vertex` (`u16`) → `Vertex` entries.
- `Linedef.right_sidedef` (`u16`) → a `Sidedef`; `Linedef.left_sidedef` (`u16`)
  → a `Sidedef` **or** the sentinel `0xffff`, which means *one-sided* (no left
  side) and is a **valid value, not a dangling reference**.
- `Sidedef.sector` (`u16`) → a `Sector`.
- `Subsector.first_seg` / `seg_count` (`u16`) → a run in the `Seg` list.
- `Seg.linedef` / `start_vertex` / `end_vertex` (`u16`) → `Linedef` / `Vertex`.
- `Node.right_child` / `left_child` (`u16`) → a child `Node` or, when bit 15 is
  set, a `Subsector`.

### Consumers pull in two directions

A renderer (#64) wants an immutable, cache-friendly graph it can traverse
quickly. An editor (#18) will eventually want to *mutate* geometry. This ADR
scopes only the **read/assembly** model; mutation is explicitly deferred, but the
representation chosen should not foreclose a future mutable layer.

### Multiple formats must converge on one model

Per ADR-0014 the source records differ by `MapFormat`: Doom and Hexen share
geometry topology (Hexen only extends `Thing`/`Linedef`), UDMF stores geometry as
**floating-point** text, and Doom 64 is deferred to #54. The assembled model must
be a single format-agnostic shape that all of these normalize into, so consumers
never branch on format.

## Decision drivers

- **No unsafe / no self-referential graphs.** The core crate is
  `#![deny(unsafe_code)]`; a pointer-linked graph would fight the borrow checker
  or require `Rc<RefCell<…>>` ceremony.
- **Validate references once.** Bounds-check every index during assembly so
  traversal is infallible and allocation is bounded.
- **Consistency** with the strict/lenient contract (ADR-0003), the `binrw`
  record layer (ADR-0002/0014), and the existing `warnings()`-on-success pattern
  on `Wad`.
- **One model across formats** — Doom/Hexen/UDMF (and later Doom 64) produce the
  same `Map`.
- **Every new parse surface is an attack surface** — assembly must have bounded
  allocation and its own fuzz target under the parser-hardening pass.

## Considered options

1. **Reference/pointer graph** — resolve indices into `&`/`Rc<RefCell<…>>` links
   between record structs.
2. **Index-arena model** — the `Map` owns flat `Vec`s of *normalized* geometry
   plus validated adjacency, addressed by typed newtype indices, with accessor
   methods that resolve an index to a reference on demand.
3. **External graph library** (e.g. `petgraph`) — store the map as a generic
   graph and attach record data to nodes/edges.

## Decision outcome

Chosen option: **Option 2 — an index-arena model.** It needs no unsafe and no
self-referential lifetimes, mirrors the on-disk index model directly, is
cache-friendly for traversal, validates references exactly once at construction,
and leaves room for a future mutable editor layer over the same arenas.

### 1. `MapGroup` — identifying one map's lumps

A map is a (conventionally zero-size) *marker* lump — `E1M1`, `MAP01`, or any
name — immediately followed by a contiguous run of recognized map data lumps. A
lump is a map header when the lump *directly after it* is one of the recognized
map data lump names (`THINGS`, `LINEDEFS`, `SIDEDEFS`, `VERTEXES`, `SEGS`,
`SSECTORS`, `NODES`, `SECTORS`, `REJECT`, `BLOCKMAP`, `BEHAVIOR`) or `TEXTMAP`
(UDMF). This marker-plus-data-lump rule handles both standard `ExMy`/`MAPxx`
names and non-standard names without a brittle name regex.

```rust
/// One map's lumps within a WAD: the marker lump plus its associated data
/// lumps, addressed by directory index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapGroup {
    /// Directory index of the map's marker lump (e.g. `E1M1`, `MAP01`).
    pub marker_index: usize,
    /// The map's name, taken from the marker lump.
    pub name: String,
    /// Directory indices of the data lumps belonging to this map, in order.
    pub data_indices: Vec<usize>,
}
```

New `Wad` accessors (in `lib.rs`, no feature gate — the `map` module is already
part of the default API):

```rust
impl Wad {
    /// Identifies every map lump group in directory order.
    #[must_use]
    pub fn map_groups(&self) -> Vec<MapGroup>;

    /// Returns the first map group whose marker lump is named `name`.
    #[must_use]
    pub fn map_group(&self, name: &str) -> Option<MapGroup>;
}
```

`MapGroup` is the input to `detect_map_format` (ADR-0014) and to `Map::assemble`.

### 2. The `Map` model — normalized, index-addressed

`Map` owns flat `Vec`s of **normalized** geometry — not the per-format `binrw`
records. Normalization is what lets Doom, Hexen, and UDMF converge: coordinates
are stored as **`f64`** (binary `i16` widens losslessly; UDMF floats fit
natively — satisfying ADR-0014's requirement), and the two per-format special
encodings (Doom `special_type`/`sector_tag` vs Hexen `special` + `args[5]`) are
folded into one normalized `LineSpecial`.

Typed newtype indices prevent cross-arena index confusion and are `usize`-backed
so extended formats (UDMF, extended nodes) are not capped at `u16`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)] pub struct VertexIdx(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)] pub struct SidedefIdx(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)] pub struct SectorIdx(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)] pub struct LinedefIdx(pub usize);
```

Representative normalized element types (exact auxiliary fields — e.g. the full
`LineSpecial` shape — are finalized alongside the Hexen/UDMF work, but the
identity and reference fields below are fixed by this ADR):

```rust
pub struct MapVertex { pub x: f64, pub y: f64 }

pub struct MapLinedef {
    pub start: VertexIdx,
    pub end: VertexIdx,
    pub right: SidedefIdx,
    pub left: Option<SidedefIdx>, // None == one-sided (the 0xffff sentinel)
    pub flags: u32,
    pub special: LineSpecial,     // normalized Doom/Hexen special encoding
}

pub struct MapSidedef {
    pub sector: SectorIdx,
    pub x_offset: i32,
    pub y_offset: i32,
    pub upper: String,
    pub lower: String,
    pub middle: String,
}

pub struct MapSector { /* floor/ceiling heights, flat names, light, special, tag */ }
pub struct MapThing  { /* normalized position, angle, type, flags (+ Hexen tid/args) */ }
```

The assembled map itself:

```rust
pub struct Map {
    name: String,
    format: MapFormat,
    vertices: Vec<MapVertex>,
    linedefs: Vec<MapLinedef>,
    sidedefs: Vec<MapSidedef>,
    sectors:  Vec<MapSector>,
    things:   Vec<MapThing>,
    // Engine-built BSP data (segs/subsectors/nodes) is optional; see item 5.
    warnings: Vec<MapWarning>,
}

impl Map {
    #[must_use] pub fn name(&self) -> &str;
    #[must_use] pub fn format(&self) -> MapFormat;
    #[must_use] pub fn vertices(&self) -> &[MapVertex];
    #[must_use] pub fn linedefs(&self) -> &[MapLinedef];
    #[must_use] pub fn sidedefs(&self) -> &[MapSidedef];
    #[must_use] pub fn sectors(&self)  -> &[MapSector];
    #[must_use] pub fn things(&self)   -> &[MapThing];

    // Reference resolvers — infallible because assembly validated every index.
    #[must_use] pub fn linedef_vertices(&self, l: &MapLinedef) -> (&MapVertex, &MapVertex);
    #[must_use] pub fn linedef_right(&self, l: &MapLinedef) -> &MapSidedef;
    #[must_use] pub fn linedef_left(&self, l: &MapLinedef) -> Option<&MapSidedef>;
    #[must_use] pub fn sidedef_sector(&self, s: &MapSidedef) -> &MapSector;

    /// Non-fatal issues collected during lenient assembly (empty in strict mode).
    #[must_use] pub fn warnings(&self) -> &[MapWarning];
}
```

The resolver methods are total: because assembly bounds-checks and clamps every
index up front, traversal never returns an error or panics.

### 3. Construction entry point

Assembly reuses the existing `ParseOptions` / `Strictness` (ADR-0003) rather than
introducing a parallel options type:

```rust
impl Map {
    /// Assembles a map from a WAD and one of its groups, using strict rules.
    ///
    /// # Errors
    /// Returns [`MapAssembleError`] if a required lump is missing, a record lump
    /// fails to decode, or (in strict mode) any cross-reference is out of range.
    pub fn assemble(wad: &Wad, group: &MapGroup) -> Result<Map, MapAssembleError>;

    /// Assembles a map under explicit options. In lenient mode, dangling
    /// references are recorded as [`MapWarning`]s (observe via [`Map::warnings`])
    /// instead of failing.
    ///
    /// # Errors
    /// As [`assemble`][Map::assemble]; in lenient mode only structural failures
    /// (missing required lump, undecodable records) return an error.
    pub fn assemble_with_options(
        wad: &Wad,
        group: &MapGroup,
        options: ParseOptions,
    ) -> Result<Map, MapAssembleError>;
}
```

`assemble` internally calls `detect_map_format(group)` and dispatches: Doom/Hexen
decode the per-format binary records (`map::doom` / `map::hexen`) and normalize;
UDMF parses the `TEXTMAP` text (`map::udmf`) and normalizes; Doom 64 is deferred
to #54. All paths yield the same `Map`.

### 4. Cross-reference validation

New error and warning types, following the `thiserror` pattern of `ParseError` /
`ParseWarning`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum MapAssembleError {
    #[error("map group is missing required lump {lump}")]
    MissingLump { lump: &'static str },
    #[error("failed to decode {lump} records: {source}")]
    Records { lump: &'static str, #[source] source: MapParseError },
    #[error("failed to parse UDMF text map: {source}")]
    Udmf { #[source] source: UdmfParseError },
    #[error("{referent} index {index} referenced from {from} is out of range ({count} available)")]
    DanglingReference { referent: &'static str, index: usize, from: &'static str, count: usize },
}

#[derive(Debug, thiserror::Error)]
pub enum MapWarning {
    #[error("{referent} index {index} referenced from {from} is out of range ({count} available); reference clamped")]
    DanglingReference { referent: &'static str, index: usize, from: &'static str, count: usize },
}
```

The two decode paths surface distinct error sources: binary formats
(Doom/Hexen/Doom 64) report record-decode failures via `Records` (a
`MapParseError`), while UDMF reports text-parse failures via `Udmf` (a
`UdmfParseError`, defined in ADR-0014) — the byte-level and line/column failure
modes are kept as separate variants rather than force-fit into one source type.

Validation rules:

- The `0xffff` `left_sidedef` sentinel maps to `left: None` and is **never** an
  error or warning — it is the canonical one-sided encoding.
- Any other index outside its target arena (linedef → vertex/sidedef, sidedef →
  sector, seg → linedef/vertex, subsector → seg run, node → child):
  - **Strict:** return `MapAssembleError::DanglingReference`.
  - **Lenient:** push `MapWarning::DanglingReference` and keep the graph
    structurally valid — a required reference (e.g. a linedef's right sidedef) is
    clamped to a safe in-range fallback *when its target arena is non-empty*, and
    a non-sentinel out-of-range *left* reference becomes `None`. Lenient assembly
    thus yields a traversable `Map` a renderer can walk without panicking, except
    in the empty-arena case below.
  - **Empty required arena:** clamping presumes a valid fallback exists. If a
    *required* referent's target arena is empty (`count == 0` — e.g. a linedef
    references a sidedef but `SIDEDEFS` decoded to zero records), there is no
    in-range index to clamp to, so this is a structural failure returned as
    `MapAssembleError::DanglingReference` (with `count: 0`) **even in lenient
    mode**. An empty arena behind an *optional* reference simply yields `None`.

### 5. Required vs optional lumps

The core geometry lumps (`VERTEXES`, `LINEDEFS`, `SIDEDEFS`, `SECTORS`,
`THINGS`; or `TEXTMAP` for UDMF) are required — a missing one is
`MapAssembleError::MissingLump`. The engine-built BSP lumps (`SEGS`, `SSECTORS`,
`NODES`) and `REJECT` / `BLOCKMAP` are optional: many editable PWADs ship without
built nodes, so their absence is not an error. Their accessors return empty
slices (or a dedicated `Option`) rather than failing assembly.

## Consequences

- **New public surface:** `MapGroup`, `Map`, the four index newtypes,
  `MapVertex`/`MapLinedef`/`MapSidedef`/`MapSector`/`MapThing`,
  `MapAssembleError`, `MapWarning`, and `Wad::map_groups` / `Wad::map_group`.
  All default (no new feature flag), consistent with the already-default `map`
  module.
- **Good** — one format-agnostic model; consumers (#64 renderer, #18 editor)
  never branch on `MapFormat`.
- **Good** — traversal is infallible; references are validated once.
- **Satisfies ADR-0014** — `MapGroup` is defined here and `f64` geometry
  accommodates UDMF, closing that ADR's two forward references.
- **Neutral / deferred** — the model is immutable. A future editor (#18) needs a
  mutable layer; it can wrap these same arenas without changing the read API.
- **Write path is out of scope (flagged).** Emitting an assembled `Map` back to
  lumps must narrow `f64` → `i16` with range validation and re-derive per-format
  specials; that is a write-path concern (ADR-0006 lineage) and is not decided
  here.
- **Hardening** — assembly is a new parse/allocation surface: crafted
  `seg_count` / record counts must be validated against actual lump sizes, and
  arena allocation must be bounded. This ADR's implementation will add a
  `cargo-fuzz` target for assembly and resource limits, per the parser-hardening
  pass (ADR-0009 lineage).
- **`lump_by_name` interaction** — map-group identification scans the directory
  positionally (not by `lump_by_name`), so it is unaffected by ADR-0013's
  first-match lookup semantics.

## Pros and cons of the options

### Option 1 — reference/pointer graph

- Good, because traversal would be direct pointer-following with no index
  indirection.
- Bad, because a map is inherently cyclic (linedef→sidedef→sector, sector
  referenced by many sidedefs), so safe Rust forces `Rc<RefCell<…>>`, adding
  runtime borrow-checking and allocation-per-node overhead.
- Bad, because it does not serialize back to the index-based on-disk format
  without rebuilding indices anyway.

### Option 2 — index-arena model (chosen)

- Good, because it needs no unsafe and no self-referential lifetimes.
- Good, because it mirrors the on-disk index model, so read *and* a future write
  path share the same representation.
- Good, because references are bounds-checked once, making traversal total.
- Bad, because callers resolve through accessor methods rather than following
  fields directly — a minor ergonomic cost mitigated by the resolver helpers.

### Option 3 — external graph library (`petgraph`)

- Good, because graph algorithms (traversal, connectivity) come for free.
- Bad, because it erases the domain typing (everything becomes generic
  nodes/edges), pushing type-safety back onto the caller.
- Bad, because it adds a dependency and an impedance mismatch with the
  index-based on-disk format for the eventual write path.

## More information

- Tracking issue: #155 (roadmap milestone 2, `docs/design.md`).
- Depends on / feeds: ADR-0014 (multi-format strategy) — `MapGroup` and `f64`
  geometry defined here close ADR-0014's two forward references; `Map::assemble`
  dispatches on `detect_map_format`.
- Consumers: #64 (visual map viewer core), #18 (editor — future mutable layer).
- Related ADRs: ADR-0002 (binrw / typed errors), ADR-0003 (default to strict
  parsing), ADR-0004 (parse API and safety contracts), ADR-0009 (`cargo-fuzz` —
  add an assembly target), ADR-0013 (`lump_by_name` — unaffected, positional
  scan).
- Source anchors: `crates/crustywad/src/map.rs` (record index fields and the
  `0xffff` one-sided sentinel), `crates/crustywad/src/lib.rs` (`Wad`, `Lump`,
  `ParseOptions`, `Strictness`).
- Revisit condition: reopen when #18 introduces map mutation (a mutable arena
  layer), or when a new node format (GL nodes / extended ZDBSP nodes) needs
  representation beyond the classic `NODES` encoding.
