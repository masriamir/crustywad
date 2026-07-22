//! The assembled, index-addressed map graph (ADR-0015 §2).

/// The source format a [`Map`] was assembled from. [`Doom`][MapFormat::Doom],
/// [`Hexen`][MapFormat::Hexen], [`Udmf`][MapFormat::Udmf], and
/// [`Doom64`][MapFormat::Doom64] (ADR-0021 §2) all assemble into a [`Map`]
/// today.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MapFormat {
    /// The classic Doom binary layout — also used, unchanged, by **Doom II** and
    /// **Heretic** (which differ only in map-marker naming, not record format).
    Doom,
    /// The Hexen binary layout — extends `THINGS`/`LINEDEFS` (see
    /// [`map::hexen`][crate::map::hexen]); detected by the presence of a
    /// `BEHAVIOR` lump.
    Hexen,
    /// The Universal Doom Map Format (UDMF) text layout (see
    /// [`map::udmf`][crate::map::udmf]); detected by the presence of a
    /// `TEXTMAP` lump.
    Udmf,
    /// The Doom 64 nested-WAD layout — the map's record lumps live inside the
    /// `MAPxx` marker lump itself (see [`map::doom64`][crate::map::doom64]);
    /// detected by the marker's nested `IWAD`/`PWAD` magic (ADR-0021 §1).
    Doom64,
}

/// A zero-based index into [`Map::vertices`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VertexIdx(pub usize);

/// A zero-based index into [`Map::sidedefs`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SidedefIdx(pub usize);

/// A zero-based index into [`Map::sectors`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SectorIdx(pub usize);

/// A zero-based index into [`Map::linedefs`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LinedefIdx(pub usize);

/// A zero-based index into [`Map::lights`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LightIdx(pub usize);

/// A zero-based index into [`Map::segs`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SegIdx(pub usize);

/// A zero-based index into [`Map::subsectors`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubsectorIdx(pub usize);

/// A zero-based index into [`Map::nodes`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeIdx(pub usize);

/// A zero-based index into [`Map::gl_vertices`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlVertexIdx(pub usize);

/// A zero-based index into [`Map::gl_segs`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlSegIdx(pub usize);

/// A zero-based index into [`Map::gl_subsectors`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlSubsectorIdx(pub usize);

/// A zero-based index into [`Map::gl_nodes`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlNodeIdx(pub usize);

/// A normalized map vertex; coordinates are `f64` so binary `i16` widens
/// losslessly and future UDMF floats fit natively.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapVertex {
    /// The vertex's X coordinate, in map units.
    pub x: f64,
    /// The vertex's Y coordinate, in map units.
    pub y: f64,
}

/// A normalized action special: the `special` number plus its five `args`.
///
/// Shared by [`MapLinedef`] and [`MapThing`] — Doom, Hexen, and UDMF all use this
/// `special` + `args` shape. For a Doom *linedef*, the sector tag is carried in `args[0]`
/// (Doom things have no special, so a Doom thing's `Special` is all zero).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Special {
    /// The action special number.
    pub special: i32,
    /// The special's five arguments; `args[0]` carries Doom's sector tag.
    pub args: [i32; 5],
}

/// A normalized linedef, referencing its endpoints, sidedefs, and special by
/// index into the owning [`Map`]'s arenas.
#[derive(Debug, Clone, PartialEq)]
pub struct MapLinedef {
    /// The index of the linedef's start vertex.
    pub start: VertexIdx,
    /// The index of the linedef's end vertex.
    pub end: VertexIdx,
    /// The index of the linedef's right (front) sidedef; `None` == no front
    /// side. Sources: the binary `0xffff` sentinel (vanilla-sanctioned and
    /// rare, e.g. an invisible blocking line; ADR-0020) or lenient-mode
    /// recovery of a dangling reference.
    pub right: Option<SidedefIdx>,
    /// The index of the linedef's left (back) sidedef; `None` == no back
    /// side (a one-sided wall in the common case, though a line can also be
    /// fully sideless). Sources: the binary `0xffff` sentinel, an omitted
    /// UDMF `sideback`, or lenient-mode recovery of a dangling reference.
    pub left: Option<SidedefIdx>,
    /// The linedef's bit flags (blocking, two-sided, secret, etc.).
    pub flags: u32,
    /// The linedef's action special and arguments.
    pub special: Special,
    /// The linedef's identifier (the UDMF/ZDoom line id); `0` for Doom/Hexen maps.
    pub id: i32,
}

/// The [`MapLinedef::id`] value that means "this linedef has no id", which is
/// **source-dependent**: UDMF's spec default is `-1`, while a Doom/Hexen map's
/// linedefs are assembled with `0` (the graph convention). Assembly copies each
/// source's sentinel into `MapLinedef.id` verbatim, so any consumer asking
/// "does this linedef carry a real id?" must know the format.
///
/// Both writers depend on this rule and must agree on it — `map::udmf::write`
/// omits the sentinel so a Doom line is not written as a genuine UDMF `id = 0`,
/// and `map::doom::write` treats it as the *absence* of an id rather than
/// tier-3 data loss. Defining it once is what keeps a Doom → UDMF → Doom
/// round-trip from resurrecting a sentinel as data.
#[cfg(feature = "write")]
#[must_use]
pub(crate) fn linedef_id_unset(format: MapFormat) -> i32 {
    if format == MapFormat::Udmf { -1 } else { 0 }
}

/// A texture or flat reference in the assembled graph (ADR-0021 §3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextureRef {
    /// A texture name (a Doom/Hexen 8-byte lump name, or a UDMF string).
    Name(String),
    /// A truncated rolling hash of the referenced lump's name (ADR-0022
    /// §1), resolved first-match-in-disk-order over the outer WAD's texture
    /// section during assembly when one is present. "Index" is the engine's
    /// historical field name for this hash, not a positional index.
    Index(u16),
}

impl TextureRef {
    /// The texture name, or `None` for a Doom 64 [`TextureRef::Index`].
    #[must_use]
    pub fn as_name(&self) -> Option<&str> {
        match self {
            TextureRef::Name(name) => Some(name),
            TextureRef::Index(_) => None,
        }
    }
}

impl PartialEq<&str> for TextureRef {
    fn eq(&self, other: &&str) -> bool {
        self.as_name() == Some(*other)
    }
}

/// A normalized sidedef, referencing its sector by index into the owning
/// [`Map`]'s sector arena.
#[derive(Debug, Clone, PartialEq)]
pub struct MapSidedef {
    /// The index of the sector this sidedef faces.
    pub sector: SectorIdx,
    /// The horizontal texture offset, in map units.
    pub x_offset: i32,
    /// The vertical texture offset, in map units.
    pub y_offset: i32,
    /// The upper texture, or an empty name if none. A Doom 64 map's
    /// [`TextureRef::Index`] resolves to a [`TextureRef::Name`] during
    /// assembly when the outer WAD's texture section is present (ADR-0022
    /// §4); otherwise it stays an unresolved hash.
    pub upper: TextureRef,
    /// The lower texture, or an empty name if none. A Doom 64 map's
    /// [`TextureRef::Index`] resolves to a [`TextureRef::Name`] during
    /// assembly when the outer WAD's texture section is present (ADR-0022
    /// §4); otherwise it stays an unresolved hash.
    pub lower: TextureRef,
    /// The middle texture, or an empty name if none. A Doom 64 map's
    /// [`TextureRef::Index`] resolves to a [`TextureRef::Name`] during
    /// assembly when the outer WAD's texture section is present (ADR-0022
    /// §4); otherwise it stays an unresolved hash.
    pub middle: TextureRef,
}

/// A normalized Doom 64 light-table entry (ADR-0021 §4).
///
/// [`Map::lights`] is built the way the engine builds its table (Doom64 EX
/// `P_LoadLights`): entries `0`–`255` are synthesized identity-grayscale
/// values (`r = g = b = index`, `tag = 0`), and the map's `LIGHTS` lump
/// records follow starting at index `256`.
///
/// The raw record's trailing `unknown` field (tentative semantics) is not
/// normalized; a consumer needing it reads [`Doom64Map`][crate::map::Doom64Map].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapLight {
    /// Red channel (`0`–`255`).
    pub r: u8,
    /// Green channel (`0`–`255`).
    pub g: u8,
    /// Blue channel (`0`–`255`).
    pub b: u8,
    /// Small identifier (observed `0`–`2` in retail data; tentative semantics).
    pub tag: u8,
}

/// A normalized sector.
#[derive(Debug, Clone, PartialEq)]
pub struct MapSector {
    /// The floor height, in map units.
    pub floor_height: i32,
    /// The ceiling height, in map units.
    pub ceiling_height: i32,
    /// The floor flat (texture). A Doom 64 map's [`TextureRef::Index`]
    /// resolves to a [`TextureRef::Name`] during assembly when the outer
    /// WAD's texture section is present (ADR-0022 §4); otherwise it stays
    /// an unresolved hash.
    pub floor_flat: TextureRef,
    /// The ceiling flat (texture). A Doom 64 map's [`TextureRef::Index`]
    /// resolves to a [`TextureRef::Name`] during assembly when the outer
    /// WAD's texture section is present (ADR-0022 §4); otherwise it stays
    /// an unresolved hash.
    pub ceiling_flat: TextureRef,
    // Doom stores sector special/tag as i16; widen losslessly to i32
    // (avoids an i16->u16 sign-loss cast that clippy::pedantic rejects).
    /// The light level, in the range `0..=255` on disk, widened to `i32`.
    /// Always `0` for a Doom 64 map — the format has no scalar light level;
    /// its lighting lives in [`MapSector::colors`] (ADR-0021 §4).
    pub light: i32,
    /// The sector special number.
    pub special: i32,
    /// The sector tag.
    pub tag: i32,
    /// Doom 64 colored lighting: five references into [`Map::lights`], carried
    /// positionally — Doom64 EX's map-format headers do not name the slots
    /// (ADR-0021 §4). The values index the combined light table: `0`–`255`
    /// select the implicit grayscale entries, `256` and above select the
    /// map's `LIGHTS` lump records. `None` for every other format.
    pub colors: Option<[LightIdx; 5]>,
    /// The sector's raw Doom 64 flag bits (`Sector.flags`, stored opaquely);
    /// `0` for every other format (mirrors `MapLinedef.flags`).
    pub flags: u32,
}

/// A normalized map thing (monster, item, player start, etc.).
#[derive(Debug, Clone, PartialEq)]
pub struct MapThing {
    /// The thing's X coordinate, in map units.
    pub x: f64,
    /// The thing's Y coordinate, in map units.
    pub y: f64,
    /// The thing's facing angle, in degrees.
    pub angle: u16,
    /// The thing's doomednum, identifying its type.
    pub type_id: u16,
    /// The thing's bit flags in the Doom/Boom-MBF layout — skill 1–2 (bit 0),
    /// skill 3 (bit 1), skill 4–5 (bit 2), ambush (bit 3), not-in-single-player
    /// (bit 4), not-in-deathmatch (bit 5), not-in-co-op (bit 6), friendly
    /// (bit 7). Note that bits 4–6 are *negative*: a clear bit means the thing
    /// **does** appear in that game mode.
    ///
    /// This layout is the graph's single contract — every source format is
    /// normalized into it on read (ADR-0019 §2), so the field means the same
    /// thing regardless of where the map came from:
    ///
    /// - **Doom/Heretic**: the on-disk word, used as-is.
    /// - **Hexen**: translated on assembly. Hexen's game-mode bits are positive
    ///   and live elsewhere (`0x0100` single-player, `0x0200` co-op, `0x0400`
    ///   deathmatch), so they are inverted into bits 4/5/6; Hexen's `dormant`
    ///   (`0x0010`) and fighter/cleric/mage class filters
    ///   (`0x0020`/`0x0040`/`0x0080`) have no Doom equivalent and are dropped.
    ///   Bit 7 (friend) is always `0` — Hexen has no such flag.
    /// - **UDMF**: the discrete booleans (`skill1`…`skill5`, `ambush`, `single`,
    ///   `dm`, `coop`, `friend`) are packed into this layout; the skill pairs
    ///   OR-fold into one bit each and `single`/`dm`/`coop` are inverted.
    pub flags: u32,
    /// The thing's identification tag (Hexen/UDMF tid); `0` for Doom maps and untagged things.
    pub id: i32,
    /// The thing's spawn height above the floor, in map units; `0.0` for Doom maps.
    pub height: f64,
    /// The thing's activation special; `Special { special: 0, args: [0; 5] }` for Doom maps.
    pub special: Special,
}

/// A child reference in the BSP tree — either an internal [`MapNode`] or a
/// leaf [`MapSubsector`]. Decoded once at assembly from the on-disk `u16`'s
/// bit 15 (set for a subsector leaf, clear for a node), which the raw
/// [`Node`][crate::map::common::Node] record does not distinguish (ADR-0015
/// amendment).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeChild {
    /// An internal BSP node, indexed into [`Map::nodes`].
    Node(NodeIdx),
    /// A leaf subsector, indexed into [`Map::subsectors`].
    Subsector(SubsectorIdx),
}

/// A normalized seg (a linedef fragment used to build a subsector's walls).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapSeg {
    /// The index of the seg's start vertex.
    pub start: VertexIdx,
    /// The index of the seg's end vertex.
    pub end: VertexIdx,
    /// The seg's raw binary angle (BAM); render-domain interpretation is
    /// deferred to the viewer work (#64).
    pub angle: u16,
    /// The linedef this seg was cut from: `Some(idx)` for a normal seg;
    /// `None` for a GL miniseg — a seg along a BSP partition line with no
    /// backing linedef, introduced with `ZDoom` extended GL nodes (#326,
    /// ADR-0025). Classic/vanilla BSP segs are always `Some`.
    pub linedef: Option<LinedefIdx>,
    /// The seg's direction relative to its linedef: `0` if the seg runs the
    /// same way as the linedef, `1` if reversed.
    pub direction: u16,
    /// The seg's distance along its linedef from the linedef's start vertex,
    /// in map units — the on-disk `i16` widened.
    pub offset: i32,
}

/// One Doom 64 render leaf: a corner of its subsector's convex polygon
/// (Doom64 EX `P_LoadLeafs`, `p_setup.cc`). Leaves exist only on
/// [`MapFormat::Doom64`] maps; see [`Map::leafs`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapLeaf {
    /// The leaf corner's vertex.
    pub vertex: VertexIdx,
    /// The seg this leaf edge follows, or `None` for the on-disk `-1`
    /// sentinel ("no seg": the edge is implicit geometry).
    pub seg: Option<SegIdx>,
}

/// One action of a Doom 64 macro script (Doom64 EX `P_LoadMacros`,
/// `p_setup.cc`). Macros exist only on [`MapFormat::Doom64`] maps; see
/// [`Map::macros`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapMacroAction {
    /// The action's line-special/macro-op identifier.
    pub id: i16,
    /// The tag the action targets — a symbolic sector/line tag, not an
    /// arena index (macros carry no cross-references).
    pub tag: i16,
    /// The action's special value.
    pub special: i16,
}

/// One Doom 64 macro: the engine-visible action sequence, decoded
/// read-only (execution semantics are out of scope; see the ACS epic).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapMacro {
    /// The macro's actions — the on-disk `count + 1` entries, verbatim:
    /// the engine reads one more action than the record's count field
    /// states (`P_LoadMacros`).
    pub actions: Vec<MapMacroAction>,
}

/// A normalized subsector (a leaf of the BSP tree): a contiguous run of segs,
/// plus — on Doom 64 maps — a run of render leaves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapSubsector {
    /// The validated `first_seg..first_seg + seg_count` run into [`Map::segs`].
    pub segs: std::ops::Range<usize>,
    /// The validated run into [`Map::leafs`]; `0..0` for every source format
    /// except Doom 64, and after a lenient whole-`LEAFS` degrade.
    pub leafs: std::ops::Range<usize>,
}

/// A normalized BSP node: a partition line plus its two children and their
/// bounding boxes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapNode {
    /// The X coordinate of the partition line's start point, in map units.
    pub x: i32,
    /// The Y coordinate of the partition line's start point, in map units.
    pub y: i32,
    /// The partition line's X direction from `(x, y)`, in map units.
    pub dx: i32,
    /// The partition line's Y direction from `(x, y)`, in map units.
    pub dy: i32,
    /// Axis-aligned bounding box for the right child, as `[top, bottom, left,
    /// right]` in map units.
    pub right_bbox: [i32; 4],
    /// Axis-aligned bounding box for the left child, as `[top, bottom, left,
    /// right]` in map units.
    pub left_bbox: [i32; 4],
    /// The right (front) child: another node, or a subsector leaf.
    pub right: NodeChild,
    /// The left (back) child: another node, or a subsector leaf.
    pub left: NodeChild,
}

/// A GL vertex from a `GL_VERT` lump. Coordinates are `f64` world units,
/// converted losslessly from the on-disk 16.16 fixed-point (`raw / 65536.0`,
/// mirroring [`MapVertex`]'s widening; #324).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlVertex {
    /// The vertex's X coordinate, in map units.
    pub x: f64,
    /// The vertex's Y coordinate, in map units.
    pub y: f64,
}

/// A GL seg endpoint: either a normal `VERTEXES` vertex or a `GL_VERT`
/// vertex. This encodes the GL-vertex high-bit convention in the on-disk
/// `GL_SEGS` record (#324).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlVertexRef {
    /// A vertex from the map's own `VERTEXES` lump, indexed into [`Map::vertices`].
    Normal(VertexIdx),
    /// A vertex from the `GL_VERT` lump, indexed into [`Map::gl_vertices`].
    Gl(GlVertexIdx),
}

/// A single GL seg from a `GL_SEGS` lump.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlSeg {
    /// The seg's start vertex.
    pub start: GlVertexRef,
    /// The seg's end vertex.
    pub end: GlVertexRef,
    /// Source linedef, or `None` for a GL miniseg (on-disk `0xFFFF`).
    pub linedef: Option<LinedefIdx>,
    /// `0` = right/front side, `1` = left/back side.
    pub side: u8,
    /// The adjacent subsector's partner seg, or `None` for a one-sided edge.
    pub partner: Option<GlSegIdx>,
}

/// A GL subsector: a contiguous run of [`Map::gl_segs`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlSubsector {
    /// The validated run into [`Map::gl_segs`].
    pub segs: core::ops::Range<usize>,
}

/// A GL BSP node child: an interior node or a subsector leaf.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GlNodeChild {
    /// An internal GL BSP node, indexed into [`Map::gl_nodes`].
    Node(GlNodeIdx),
    /// A leaf GL subsector, indexed into [`Map::gl_subsectors`].
    Subsector(GlSubsectorIdx),
}

/// A GL BSP node from a `GL_NODES` lump. Partition-line and bbox fields
/// mirror [`MapNode`]; children reference the GL arenas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlNode {
    /// The X coordinate of the partition line's start point, in map units.
    pub x: i32,
    /// The Y coordinate of the partition line's start point, in map units.
    pub y: i32,
    /// The partition line's X direction from `(x, y)`, in map units.
    pub dx: i32,
    /// The partition line's Y direction from `(x, y)`, in map units.
    pub dy: i32,
    /// Axis-aligned bounding box for the right child, as `[top, bottom, left,
    /// right]` in map units.
    pub right_bbox: [i32; 4],
    /// Axis-aligned bounding box for the left child, as `[top, bottom, left,
    /// right]` in map units.
    pub left_bbox: [i32; 4],
    /// The right (front) child: another GL node, or a GL subsector leaf.
    pub right: GlNodeChild,
    /// The left (back) child: another GL node, or a GL subsector leaf.
    pub left: GlNodeChild,
}

/// The `REJECT` lump decoded into a typed sector-visibility table.
///
/// The table is a row-major bit matrix, `sector_count` × `sector_count`
/// bits, LSB-first within each byte; a set bit means the engine may skip
/// the line-of-sight check from the row sector to the column sector
/// ("rejected" = potentially hidden). Layout verified against Chocolate
/// Doom `P_LoadReject` (`p_setup.c`) and `P_CheckSight` (`p_sight.c`).
///
/// Built by [`MapReject::parse`] (directly, or during map assembly — see
/// [`Map::reject`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapReject {
    /// Table dimension: the owning map's sector count at parse time.
    pub(crate) sector_count: usize,
    /// The stored table bytes: `min(lump length, (n² + 7) / 8)` — an
    /// undersized lenient-mode table is padded *virtually* by
    /// [`is_rejected`](Self::is_rejected), never materialized (ADR-0016 §1).
    pub(crate) bits: Box<[u8]>,
}

impl MapReject {
    /// Table dimension (the owning map's sector count at parse time).
    #[must_use]
    pub fn sector_count(&self) -> usize {
        self.sector_count
    }

    /// Whether the table pre-rejects line-of-sight from `a` to `b` (bit set
    /// = "hidden"). Bits beyond the stored bytes — possible after a lenient
    /// undersized recovery, or when the bit index itself would not fit in
    /// `usize` — read as `false` ("not rejected"), a deterministic choice
    /// made by this reader: vanilla instead pads undersized tables with
    /// level-dependent garbage that emulates its own overflow bug
    /// (`PadRejectArray`, called from `P_LoadReject`), which is
    /// renderer-quirk fidelity a parsing library should not reproduce.
    ///
    /// Returns `None` if either index is `>= sector_count`.
    #[must_use]
    pub fn is_rejected(&self, a: SectorIdx, b: SectorIdx) -> Option<bool> {
        if a.0 >= self.sector_count || b.0 >= self.sector_count {
            return None;
        }
        // Checked arithmetic: a pathological standalone-caller table
        // dimension can push the bit index past `usize`; such a bit lies
        // beyond any storable byte, so it reads as virtual padding rather
        // than wrapping into a wrong byte.
        let Some(bit) =
            a.0.checked_mul(self.sector_count)
                .and_then(|row| row.checked_add(b.0))
        else {
            return Some(false);
        };
        let mask = 1u8 << (bit % 8);
        Some(self.bits.get(bit / 8).is_some_and(|byte| byte & mask != 0))
    }
}

/// The `BLOCKMAP` lump decoded into a typed spatial index: a grid of
/// 128×128-map-unit blocks, each listing the linedefs that cross it.
///
/// Layout verified against Chocolate Doom `P_LoadBlockMap` (`p_setup.c`) and
/// `P_BlockLinesIterator` (`p_maputl.c`); grid cell size is `MAPBLOCKUNITS`
/// (`p_local.h`). Built by [`MapBlockmap::parse`] (directly, or during map
/// assembly — see [`Map::blockmap`]).
///
/// Internally the lump's words are stored once and each block holds a
/// validated range into them, so offset aliasing (ZDBSP-style whole-list
/// sharing) and tail sharing (ZokumBSP-style) cost no extra memory
/// (ADR-0016 §1).
#[derive(Debug, Clone, PartialEq)]
pub struct MapBlockmap {
    /// Grid origin (map units), from the header's two `i16` fields.
    pub(crate) origin_x: f64,
    /// See `origin_x`.
    pub(crate) origin_y: f64,
    /// Grid width in blocks.
    pub(crate) columns: usize,
    /// Grid height in blocks.
    pub(crate) rows: usize,
    /// Every lump word, converted once; block ranges index into this.
    pub(crate) entries: Vec<LinedefIdx>,
    /// One validated `entries` span per block, row-major, `columns * rows`
    /// long.
    pub(crate) blocks: Vec<std::ops::Range<usize>>,
}

impl MapBlockmap {
    /// Grid cell size in map units (Chocolate Doom `MAPBLOCKUNITS`,
    /// `p_local.h` — verified Task 2 Step 0).
    const BLOCK_UNITS: f64 = 128.0;

    /// The grid origin in map units.
    #[must_use]
    pub fn origin(&self) -> (f64, f64) {
        (self.origin_x, self.origin_y)
    }

    /// Grid width in blocks.
    #[must_use]
    pub fn columns(&self) -> usize {
        self.columns
    }

    /// Grid height in blocks.
    #[must_use]
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// The linedefs crossing block (`col`, `row`), or `None` outside the
    /// grid.
    ///
    /// A literal leading `0` word in the on-disk list is treated as the
    /// conventional delimiter and excluded (the convention every
    /// nodebuilder writes and later ports skip — `PrBoom+`'s
    /// `P_BlockLinesIterator`; vanilla instead reads it as "linedef 0 in
    /// every block", a known engine quirk). A genuine first entry of
    /// linedef 0 written *without* a delimiter is indistinguishable and is
    /// also stripped.
    #[must_use]
    pub fn block(&self, col: usize, row: usize) -> Option<&[LinedefIdx]> {
        if col >= self.columns || row >= self.rows {
            return None;
        }
        Some(&self.entries[self.blocks[row * self.columns + col].clone()])
    }

    /// Grid lookup by map-space coordinates, or `None` outside the grid
    /// (including non-finite coordinates).
    #[must_use]
    pub fn block_at(&self, x: f64, y: f64) -> Option<&[LinedefIdx]> {
        let col = ((x - self.origin_x) / Self::BLOCK_UNITS).floor();
        let row = ((y - self.origin_y) / Self::BLOCK_UNITS).floor();
        // NaN and negative values are rejected explicitly (rather than via a
        // negated `>= 0.0`, which clippy flags as unclear for partially
        // ordered types); a huge finite value instead saturates the cast
        // below and fails the grid bound inside `block`.
        if col.is_nan() || row.is_nan() || col < 0.0 || row < 0.0 {
            return None;
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        self.block(col as usize, row as usize)
    }
}

/// A non-fatal issue recorded during lenient map assembly.
///
/// Produced by [`Map::assemble_with_options`] in [`Strictness::Lenient`] mode
/// and collected on the resulting map; retrieve them via [`Map::warnings`].
///
/// [`Strictness::Lenient`]: crate::Strictness::Lenient
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum MapWarning {
    /// A reference from one arena to another (e.g. a linedef's vertex index)
    /// pointed past the end of the referenced arena and was recovered during
    /// lenient assembly — clamped to an in-range index for a required
    /// reference, or dropped to `None` for an optional left sidedef.
    #[error(
        "{referent} index {index} referenced from {from} is out of range ({count} available); recovered during lenient assembly"
    )]
    DanglingReference {
        /// The name of the arena the out-of-range index referred to (e.g. `"vertex"`).
        referent: &'static str,
        /// The out-of-range index value that was encountered (signed, since UDMF
        /// indices may be negative).
        index: i32,
        /// The name of the element kind the dangling reference was found on (e.g. `"linedef"`).
        from: &'static str,
        /// The number of elements actually available in the referenced arena.
        count: usize,
    },
    /// A field value was outside its target field's representable range and was
    /// clamped during lenient assembly.
    #[error("{field} value {value} on {from} is out of range; clamped during lenient assembly")]
    FieldOutOfRange {
        /// The UDMF field name (e.g. `"thing.type"`).
        field: &'static str,
        /// The element kind the field was on (e.g. `"thing"`).
        from: &'static str,
        /// The offending value.
        value: i32,
    },
    /// A UDMF map lacked its `ENDMAP` terminator and was recovered best-effort.
    #[error("UDMF map '{name}' has no ENDMAP terminator; recovered during lenient assembly")]
    UnterminatedUdmf {
        /// The map's marker name.
        name: String,
    },
    /// A non-fatal issue recovered while reading a Doom 64 map's nested WAD
    /// during lenient assembly (ADR-0021 §2); see
    /// [`Doom64Warning`](crate::map::doom64::Doom64Warning).
    #[error("{0}")]
    Doom64(crate::map::doom64::Doom64Warning),
    /// A `NODES`/`SSECTORS` lump (or the UDMF `ZNODES` lump) carried an
    /// extended node-encoding signature this build cannot decode — a compressed
    /// `Z*` twin without the `extended-nodes-zlib` feature (#327), or another
    /// gated encoding; the uncompressed `X*` family always decodes (#326). The
    /// BSP arenas were left empty during lenient assembly.
    #[error("{lump} uses an unsupported extended node encoding; skipped; BSP arenas left empty")]
    UnsupportedNodeEncoding {
        /// The name of the lump carrying the extended encoding (`"NODES"`,
        /// `"SSECTORS"`, or the UDMF `"ZNODES"`).
        lump: &'static str,
    },
    /// An uncompressed `ZDoom` extended-node stream
    /// (`XNOD`/`XGLN`/`XGL2`/`XGL3`) was recovered during lenient assembly:
    /// either an individual mismatch was tolerated and decoding continued, or a
    /// structural fault degraded the whole BSP to empty arenas. See
    /// [`ExtendedNodeError`](crate::map::ExtendedNodeError) (ADR-0025).
    #[error("recovered malformed {dialect} extended node stream during lenient assembly: {reason}")]
    ExtendedNode {
        /// The dialect tag naming the stream (`"XNOD"`, `"XGLN"`, `"XGL2"`, or `"XGL3"`).
        dialect: &'static str,
        /// The specific structural fault that was recovered.
        reason: crate::map::ExtendedNodeError,
    },
    /// The `REJECT` lump was smaller than its map's sector count requires;
    /// the missing bits read as "not rejected" during lenient assembly.
    #[error(
        "REJECT lump is {actual} bytes ({expected} required for {sectors} sectors); missing bits read as not-rejected during lenient assembly"
    )]
    UndersizedReject {
        /// The lump's actual byte length.
        actual: usize,
        /// The required table size, `(sectors² + 7) / 8` bytes.
        expected: usize,
        /// The owning map's sector count.
        sectors: usize,
    },
    /// A structurally unusable `BLOCKMAP` lump (short header, non-positive
    /// dimensions, or truncated offset table) was discarded during lenient
    /// assembly.
    #[error("BLOCKMAP lump is malformed ({detail}); discarded during lenient assembly")]
    MalformedBlockmap {
        /// What made the lump unusable.
        detail: &'static str,
    },
    /// A `BLOCKMAP` block's offset pointed outside the lump; that block's
    /// list was emptied during lenient assembly.
    #[error(
        "BLOCKMAP block {block} offset {offset} is outside the lump ({words} words); block list emptied during lenient assembly"
    )]
    BlockmapBlockOffset {
        /// The 0-based block (offset-table) index.
        block: usize,
        /// The out-of-range word offset.
        offset: usize,
        /// The lump's total word count.
        words: usize,
    },
    /// A `BLOCKMAP` block's linedef list had no `0xFFFF` terminator and was
    /// truncated at the lump end during lenient assembly.
    #[error(
        "BLOCKMAP block {block} linedef list is unterminated; truncated during lenient assembly"
    )]
    UnterminatedBlockmapList {
        /// The 0-based block index.
        block: usize,
    },
    /// A `BLOCKMAP` block's list referenced a linedef past the end of the
    /// linedef arena; the whole block list was emptied during lenient
    /// assembly (entry-level dropping would require materializing patched
    /// list copies — see the spec's hardening notes).
    #[error(
        "BLOCKMAP block {block} references linedef {index} ({count} available); block list emptied during lenient assembly"
    )]
    BlockmapListDangling {
        /// The 0-based block index.
        block: usize,
        /// The first out-of-range linedef index in the list.
        index: u16,
        /// The linedef arena length.
        count: usize,
    },
    /// The `LEAFS` lump was structurally unusable (truncated record or
    /// trailing partial bytes); all leaves were discarded during lenient
    /// assembly.
    #[error("LEAFS lump is malformed ({detail}); all leaves discarded during lenient assembly")]
    MalformedLeafs {
        /// What made the lump unusable.
        detail: &'static str,
    },
    /// The `LEAFS` lump's record count did not match the subsector count —
    /// a hard engine invariant (Doom64 EX `P_LoadLeafs` fatal-errors on
    /// it); all leaves were discarded during lenient assembly.
    #[error(
        "LEAFS record count {leaves} does not match subsector count {subsectors}; all leaves discarded during lenient assembly"
    )]
    LeafCountMismatch {
        /// The number of leaf records the lump encodes.
        leaves: usize,
        /// The owning map's subsector count.
        subsectors: usize,
    },
    /// A leaf referenced a vertex or seg past the end of its arena; all
    /// leaves were discarded during lenient assembly (leaves are
    /// interlocked render data — partial salvage would mislead).
    #[error(
        "leaf references {referent} {index} ({count} available); all leaves discarded during lenient assembly"
    )]
    LeafsDangling {
        /// The arena the out-of-range index referred to (`"vertex"` or `"seg"`).
        referent: &'static str,
        /// The out-of-range index.
        index: u16,
        /// The referenced arena's length.
        count: usize,
    },
    /// The `MACROS` lump was structurally unusable (short header, negative
    /// count, truncated record, or trailing bytes); all macros were
    /// discarded during lenient assembly.
    #[error("MACROS lump is malformed ({detail}); all macros discarded during lenient assembly")]
    MalformedMacros {
        /// What made the lump unusable.
        detail: &'static str,
    },
    /// A marker anomaly recovered from the outer WAD's section scan while
    /// building the Doom 64 texture-name table during lenient assembly.
    /// The scan classifies markers of every kind, so the wrapped
    /// [`SectionWarning`](crate::sections::SectionWarning) may concern a
    /// non-texture section; it names the section kind itself.
    #[error("scanning sections for Doom 64 texture resolution: {0}")]
    TextureSection(crate::sections::SectionWarning),
    /// A Doom 64 texture hash matched no texture-section lump; the
    /// unresolved [`TextureRef::Index`] was kept during lenient assembly
    /// (never the engine's silent fallback to texture 0).
    #[error(
        "texture name hash {hash:#06x} on {from} matches no texture-section lump; kept as an unresolved index during lenient assembly"
    )]
    UnresolvedTextureHash {
        /// The on-disk 16-bit name hash.
        hash: u16,
        /// The element kind carrying the reference (`"sidedef"` or `"sector"`).
        from: &'static str,
    },
    /// GL nodes were present but refused (V1/V4); GL arenas left empty.
    #[error("GL node version {version} refused (deprecated/insufficient); GL data skipped")]
    GlNodesRefused {
        /// The refused GL node version number.
        version: u8,
    },
    /// GL nodes were malformed; GL arenas degraded to empty (Lenient).
    #[error("GL nodes malformed; GL data skipped")]
    GlNodesDegraded,
}

/// An assembled Doom map graph: normalized elements addressed by index,
/// with infallible resolvers for following references between arenas.
///
/// `Map` is constructed by map assembly ([`Map::assemble`] /
/// [`Map::assemble_with_options`]); its arena fields are crate-private so only
/// assembly builds one directly.
#[derive(Debug, Clone)]
pub struct Map {
    pub(crate) name: String,
    pub(crate) format: MapFormat,
    pub(crate) namespace: Option<String>,
    pub(crate) vertices: Vec<MapVertex>,
    pub(crate) linedefs: Vec<MapLinedef>,
    pub(crate) sidedefs: Vec<MapSidedef>,
    pub(crate) sectors: Vec<MapSector>,
    pub(crate) things: Vec<MapThing>,
    pub(crate) lights: Vec<MapLight>,
    pub(crate) segs: Vec<MapSeg>,
    pub(crate) subsectors: Vec<MapSubsector>,
    pub(crate) nodes: Vec<MapNode>,
    pub(crate) gl_vertices: Vec<GlVertex>,
    pub(crate) gl_segs: Vec<GlSeg>,
    pub(crate) gl_subsectors: Vec<GlSubsector>,
    pub(crate) gl_nodes: Vec<GlNode>,
    pub(crate) leafs: Vec<MapLeaf>,
    pub(crate) macros: Vec<MapMacro>,
    pub(crate) reject: Option<MapReject>,
    pub(crate) blockmap: Option<MapBlockmap>,
    pub(crate) warnings: Vec<MapWarning>,
}

impl Map {
    /// Returns the map's lump name (e.g. `"E1M1"`).
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the source format this map was assembled from.
    #[must_use]
    pub fn format(&self) -> MapFormat {
        self.format
    }

    /// Returns the map's UDMF `namespace` declaration (e.g. `"doom"`), or `None`
    /// for binary-format maps.
    #[must_use]
    pub fn namespace(&self) -> Option<&str> {
        self.namespace.as_deref()
    }

    /// Returns the map's vertex arena.
    #[must_use]
    pub fn vertices(&self) -> &[MapVertex] {
        &self.vertices
    }

    /// Returns the map's linedef arena.
    #[must_use]
    pub fn linedefs(&self) -> &[MapLinedef] {
        &self.linedefs
    }

    /// Returns the map's sidedef arena.
    #[must_use]
    pub fn sidedefs(&self) -> &[MapSidedef] {
        &self.sidedefs
    }

    /// Returns the map's sector arena.
    #[must_use]
    pub fn sectors(&self) -> &[MapSector] {
        &self.sectors
    }

    /// Returns the map's thing arena.
    #[must_use]
    pub fn things(&self) -> &[MapThing] {
        &self.things
    }

    /// Returns the non-fatal warnings collected during assembly.
    #[must_use]
    pub fn warnings(&self) -> &[MapWarning] {
        &self.warnings
    }

    /// The map's light table, mirroring the engine's (Doom64 EX
    /// `P_LoadLights`): indices `0`–`255` are implicit grayscale entries
    /// (`r = g = b = index`, `tag = 0`), followed by the map's `LIGHTS` lump
    /// records starting at index `256`. Empty for non-Doom 64 maps.
    #[must_use]
    pub fn lights(&self) -> &[MapLight] {
        &self.lights
    }

    /// Returns the map's seg arena. Empty for a map assembled without BSP
    /// data (an absent `SEGS` lump, a gated extended node encoding, or a lenient-mode whole-BSP degrade).
    #[must_use]
    pub fn segs(&self) -> &[MapSeg] {
        &self.segs
    }

    /// Returns the map's subsector arena. Empty for a map assembled without
    /// BSP data (an absent `SSECTORS` lump, a gated extended node encoding, or a lenient-mode whole-BSP degrade).
    #[must_use]
    pub fn subsectors(&self) -> &[MapSubsector] {
        &self.subsectors
    }

    /// Returns the map's BSP node arena. Empty for a map assembled without
    /// BSP data (an absent `NODES` lump, a gated extended node encoding, or a lenient-mode whole-BSP degrade).
    #[must_use]
    pub fn nodes(&self) -> &[MapNode] {
        &self.nodes
    }

    /// Returns the map's GL vertex arena, decoded from `GL_VERT`. Additive to
    /// (not a replacement for) [`Map::vertices`]. Empty for a map assembled
    /// without a classic GL-node group (#324, ADR-0025) — an absent group, a
    /// refused version (V1/V4), or a lenient-mode whole-group degrade.
    #[must_use]
    pub fn gl_vertices(&self) -> &[GlVertex] {
        &self.gl_vertices
    }

    /// Returns the map's GL seg arena, decoded from `GL_SEGS`. Additive to
    /// (not a replacement for) [`Map::segs`]. Empty for a map assembled
    /// without a classic GL-node group (#324, ADR-0025) — an absent group, a
    /// refused version (V1/V4), or a lenient-mode whole-group degrade.
    #[must_use]
    pub fn gl_segs(&self) -> &[GlSeg] {
        &self.gl_segs
    }

    /// Returns the map's GL subsector arena, decoded from `GL_SSECT`.
    /// Additive to (not a replacement for) [`Map::subsectors`]. Empty for a
    /// map assembled without a classic GL-node group (#324, ADR-0025) — an
    /// absent group, a refused version (V1/V4), or a lenient-mode whole-group
    /// degrade.
    #[must_use]
    pub fn gl_subsectors(&self) -> &[GlSubsector] {
        &self.gl_subsectors
    }

    /// Returns the map's GL BSP node arena, decoded from `GL_NODES`. Additive
    /// to (not a replacement for) [`Map::nodes`]. Empty for a map assembled
    /// without a classic GL-node group (#324, ADR-0025) — an absent group, a
    /// refused version (V1/V4), or a lenient-mode whole-group degrade.
    #[must_use]
    pub fn gl_nodes(&self) -> &[GlNode] {
        &self.gl_nodes
    }

    /// Returns the index of the BSP tree's root node, or `None` if the map
    /// has no nodes. By convention the root is the *last* node in the arena
    /// — Chocolate Doom's `R_RenderPlayerView` starts at
    /// `R_RenderBSPNode(numnodes - 1)`.
    #[must_use]
    pub fn bsp_root(&self) -> Option<NodeIdx> {
        (!self.nodes.is_empty()).then(|| NodeIdx(self.nodes.len() - 1))
    }

    /// All decoded Doom 64 render leaves, in subsector order (each
    /// [`MapSubsector::leafs`] range indexes into this arena). Empty for
    /// every source format except [`MapFormat::Doom64`], and after a
    /// lenient whole-`LEAFS` degrade.
    #[must_use]
    pub fn leafs(&self) -> &[MapLeaf] {
        &self.leafs
    }

    /// All decoded Doom 64 macros, in lump order. Empty for every source
    /// format except [`MapFormat::Doom64`], for a Doom 64 map that has
    /// none, and after a lenient whole-`MACROS` degrade.
    ///
    /// The `MACROS` header's second field (`specialcount`) has
    /// unestablished semantics and is deliberately not carried into the
    /// graph — the raw [`Doom64Map`](crate::map::Doom64Map) bytes retain
    /// it (interpreting it is the ACS spike's decision).
    #[must_use]
    pub fn macros(&self) -> &[MapMacro] {
        &self.macros
    }

    /// The decoded `REJECT` sector-visibility table, or `None` when the
    /// group carried no `REJECT` lump or an empty one ("not built",
    /// ADR-0019 §4).
    #[must_use]
    pub fn reject(&self) -> Option<&MapReject> {
        self.reject.as_ref()
    }

    /// The decoded `BLOCKMAP` spatial index, or `None` when the group
    /// carried no `BLOCKMAP` lump or an empty one ("not built",
    /// ADR-0019 §4).
    #[must_use]
    pub fn blockmap(&self) -> Option<&MapBlockmap> {
        self.blockmap.as_ref()
    }

    /// Resolves a linedef's start/end vertices. Total for elements produced by
    /// this map's own assembly; a linedef carrying an out-of-range index (e.g.
    /// hand-constructed, since `MapLinedef`'s fields are public) may panic.
    #[must_use]
    pub fn linedef_vertices(&self, l: &MapLinedef) -> (&MapVertex, &MapVertex) {
        (&self.vertices[l.start.0], &self.vertices[l.end.0])
    }

    /// Resolves a linedef's right (front) sidedef, if present (`None` == no
    /// front side; see [`MapLinedef::right`] for its sources). Total for
    /// elements produced by this map's own assembly; a linedef carrying an
    /// out-of-range index (e.g. hand-constructed, since `MapLinedef`'s fields
    /// are public) may panic.
    #[must_use]
    pub fn linedef_right(&self, l: &MapLinedef) -> Option<&MapSidedef> {
        l.right.map(|i| &self.sidedefs[i.0])
    }

    /// Resolves a linedef's left (back) sidedef, if present (`None` == no
    /// back side; see [`MapLinedef::left`] for its sources). Total for
    /// elements produced by this map's own assembly; a linedef carrying an
    /// out-of-range index (e.g. hand-constructed, since `MapLinedef`'s fields
    /// are public) may panic.
    #[must_use]
    pub fn linedef_left(&self, l: &MapLinedef) -> Option<&MapSidedef> {
        l.left.map(|i| &self.sidedefs[i.0])
    }

    /// Resolves a sidedef's sector. Total for elements produced by this map's
    /// own assembly; a sidedef carrying an out-of-range index (e.g.
    /// hand-constructed, since `MapSidedef`'s fields are public) may panic.
    #[must_use]
    pub fn sidedef_sector(&self, s: &MapSidedef) -> &MapSector {
        &self.sectors[s.sector.0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_map() -> Map {
        Map {
            name: "E1M1".into(),
            format: MapFormat::Doom,
            namespace: None,
            vertices: vec![MapVertex { x: 0.0, y: 0.0 }, MapVertex { x: 64.0, y: 0.0 }],
            sidedefs: vec![MapSidedef {
                sector: SectorIdx(0),
                x_offset: 0,
                y_offset: 0,
                upper: TextureRef::Name(String::new()),
                lower: TextureRef::Name(String::new()),
                middle: TextureRef::Name("WALL".into()),
            }],
            sectors: vec![MapSector {
                floor_height: 0,
                ceiling_height: 128,
                floor_flat: TextureRef::Name("FLOOR".into()),
                ceiling_flat: TextureRef::Name("CEIL".into()),
                light: 160,
                special: 0,
                tag: 0,
                colors: None,
                flags: 0,
            }],
            linedefs: vec![MapLinedef {
                start: VertexIdx(0),
                end: VertexIdx(1),
                right: Some(SidedefIdx(0)),
                left: None,
                flags: 1,
                special: Special {
                    special: 0,
                    args: [0; 5],
                },
                id: 0,
            }],
            things: vec![],
            lights: vec![],
            segs: Vec::new(),
            subsectors: Vec::new(),
            nodes: Vec::new(),
            gl_vertices: Vec::new(),
            gl_segs: Vec::new(),
            gl_subsectors: Vec::new(),
            gl_nodes: Vec::new(),
            leafs: Vec::new(),
            macros: Vec::new(),
            reject: None,
            blockmap: None,
            warnings: vec![],
        }
    }

    #[test]
    fn resolvers_follow_indices() {
        let m = tiny_map();
        let l = &m.linedefs()[0];
        let (a, b) = m.linedef_vertices(l);
        assert_eq!((a.x, b.x), (0.0, 64.0));
        let right = m.linedef_right(l).expect("fronted line");
        assert_eq!(right.middle, "WALL");
        assert!(m.linedef_left(l).is_none());
        assert_eq!(m.sidedef_sector(right).ceiling_height, 128);
    }

    #[test]
    fn classic_map_has_no_lights_and_no_colors() {
        let m = tiny_map();
        assert!(m.lights().is_empty());
        assert_eq!(m.sectors()[0].colors, None);
        assert_eq!(m.sectors()[0].flags, 0);
    }

    #[test]
    fn maps_without_bsp_have_empty_arenas_and_no_root() {
        let m = tiny_map();
        assert!(m.segs().is_empty());
        assert!(m.subsectors().is_empty());
        assert!(m.nodes().is_empty());
        assert_eq!(m.bsp_root(), None);
    }

    #[test]
    fn non_gl_map_has_empty_gl_arenas() {
        // A map assembled without GL lumps has empty GL arenas (#324 Task 1:
        // the arenas exist but nothing decodes into them yet).
        let m = tiny_map();
        assert!(m.gl_vertices().is_empty());
        assert!(m.gl_segs().is_empty());
        assert!(m.gl_subsectors().is_empty());
        assert!(m.gl_nodes().is_empty());
    }
}
