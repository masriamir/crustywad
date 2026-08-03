//! The BLOCKMAP builder (ADR-0024 §5, staging §9.1).
//!
//! A `BLOCKMAP` lump is the engine's coarse spatial index: the map is covered by
//! a grid of 128×128-map-unit blocks and, for each block, the lump lists every
//! linedef whose segment crosses it. The engine uses it to narrow collision and
//! sight checks to a handful of nearby linedefs instead of the whole map
//! (`P_BlockLinesIterator`).
//!
//! [`build_blockmap`] rasterizes an assembled [`Map`]'s linedefs onto that grid
//! and serializes the canonical word image the parser
//! ([`MapBlockmap::parse`](crate::map::MapBlockmap)) reads back. Coordinates are
//! narrowed through the *same* pass as the write path (ADR-0024 §3), so the grid
//! is built on exactly the `i16` geometry the engine sees.
//!
//! # Rasterization
//!
//! For each linedef the builder walks the blocks its (origin-relative) segment
//! bounding box touches and keeps a block when the segment's infinite line
//! passes through the block's closed AABB — an exact `i64` corner cross-product
//! test. The result is a *conservative superset* (Global Constraint 6): a
//! linedef may be listed in a block it only grazes, but it is never missing from
//! a block it crosses. Blocks whose edges a segment merely touches are listed on
//! **both** sides (closed intervals), matching what real nodebuilders emit so
//! the engine never misses a boundary wall.
//!
//! # Serialization and dedup
//!
//! Identical blocklists share one offset (ZDBSP-style whole-list dedup): the
//! lookup is content-keyed but output order is driven purely by block order, so
//! the bytes are deterministic (Global Constraint 7). All empty blocks thereby
//! collapse onto a single `[0, 0xFFFF]` list.
//!
//! # Offset ceilings
//!
//! Blocklist-start offsets are 16-bit word indices (ADR-0024 §5). A new list
//! starting past the unsigned ceiling (> 65,535) is a
//! [`NodeBuildError::BlockmapOverflow`] in both modes; one past the vanilla
//! signed ceiling (> 32,767) is a strict error and, in lenient mode, a single
//! [`NodeBuildWarning::BlockmapVanillaOverflow`] (first offender only) with the
//! lump still emitted.
//!
//! [`Map`]: crate::map::Map

use std::collections::HashMap;

use crate::Strictness;
use crate::map::DoomWriteError;
use crate::map::build::{NodeBuildError, NodeBuildOptions, NodeBuildWarning};
use crate::map::doom::{Narrower, narrow_vertices};
use crate::map::graph::{LinedefIdx, Map, MapBlockmap};

/// Grid cell size, `log2(128)`: `col = x >> BLOCK_SHIFT`.
const BLOCK_SHIFT: u32 = 7;
/// Grid cell size in map units (Chocolate Doom `MAPBLOCKUNITS`).
const BLOCK_UNITS: i32 = 1 << BLOCK_SHIFT;
/// A blocklist entry references a linedef by `u16` index, but `0xFFFF` is the
/// blocklist terminator word, so the maximum *encodable* linedef index is
/// 65,534 — at most 65,535 linedefs. This deliberately differs from the write
/// path's generic 65,536-element ceiling (`MAX_INDEXED` in
/// `map/doom/write.rs`): index 65,535 would serialize as the terminator and
/// corrupt every list containing it, so the blockmap encoding domain is one
/// element stricter.
const MAX_LINEDEFS: usize = 65_535;
/// The blocklist terminator word (`-1` in `P_BlockLinesIterator`).
const TERMINATOR: u16 = 0xFFFF;
/// Vanilla's signed blocklist-offset ceiling (`i16::MAX`).
const VANILLA_OFFSET_CEILING: usize = 32_767;
/// The unsigned 16-bit blocklist-offset ceiling (`u16::MAX`).
const WORD_OFFSET_CEILING: usize = 65_535;

/// Reinterprets a signed `i16` header field as its unsigned word image, so
/// `word.to_le_bytes()` reproduces the `i16`'s on-disk bytes exactly.
fn as_word(value: i16) -> u16 {
    u16::from_le_bytes(value.to_le_bytes())
}

/// The lowest block index a closed span `[lo, ..]` (origin-relative, `lo >= 0`)
/// touches: a coordinate exactly on a block boundary belongs to the block on
/// **both** sides, so its low block is one less than `lo >> BLOCK_SHIFT`.
fn block_lo(lo: i32) -> i32 {
    if lo <= 0 { 0 } else { (lo - 1) >> BLOCK_SHIFT }
}

/// The highest block index a closed span `[.., hi]` (origin-relative,
/// `hi >= 0`) touches.
fn block_hi(hi: i32) -> i32 {
    hi >> BLOCK_SHIFT
}

/// Whether the segment `(x1,y1)-(x2,y2)` crosses block `(bx, by)`'s closed AABB.
///
/// Exact `i64` corner cross-product test: if all four corners of the block sit
/// strictly on one side of the segment's infinite line the line misses the box;
/// otherwise (a corner is collinear, or the corners straddle) the line passes
/// through it. The caller already bounds `(bx, by)` to the segment's block
/// bounding box, so a hit here means the *segment* — not merely its line —
/// reaches the block. A zero-length segment yields all-zero cross products and
/// so is reported as crossing its containing block(s).
fn crosses_block(x1: i32, y1: i32, x2: i32, y2: i32, bx: i32, by: i32) -> bool {
    let ex = i64::from(x2) - i64::from(x1);
    let ey = i64::from(y2) - i64::from(y1);
    let left = i64::from(bx) * i64::from(BLOCK_UNITS);
    let bottom = i64::from(by) * i64::from(BLOCK_UNITS);
    let corners_x = [left, left + i64::from(BLOCK_UNITS)];
    let corners_y = [bottom, bottom + i64::from(BLOCK_UNITS)];

    let mut all_positive = true;
    let mut all_negative = true;
    for cy in corners_y {
        for cx in corners_x {
            let cross = ex * (cy - i64::from(y1)) - ey * (cx - i64::from(x1));
            if cross <= 0 {
                all_positive = false;
            }
            if cross >= 0 {
                all_negative = false;
            }
        }
    }
    !(all_positive || all_negative)
}

/// Builds the `BLOCKMAP` spatial index for `map` (ADR-0024 §5).
///
/// Narrows the vertex arena through the shared write-path pass (`opts.strictness`
/// drives it), computes the 128-unit grid over the linedef endpoints, rasterizes
/// every linedef into the blocks its segment crosses (a conservative superset),
/// deduplicates identical blocklists, and returns the typed [`MapBlockmap`]
/// together with any recovered [`NodeBuildWarning`]s. The result round-trips
/// through [`MapBlockmap::parse`] byte-for-byte (Global Constraint 4) via
/// [`MapBlockmap::to_lump_bytes`].
///
/// # Errors
///
/// - [`NodeBuildError::EmptyGeometry`] (both modes) when the map has zero
///   vertices, linedefs, sidedefs, or sectors — there is no geometry to index.
/// - [`NodeBuildError::Write`] wrapping a [`DoomWriteError`] from the shared
///   narrowing pass: in strict mode a non-finite/fractional/out-of-range
///   coordinate; in both modes more than 65,535 linedefs (a blocklist index of
///   65,535 would collide with the `0xFFFF` terminator word —
///   [`DoomWriteError::TooManyElements`], `kind: "linedefs"`, `max: 65_535`).
/// - [`NodeBuildError::BlockmapOverflow`] when a blocklist starts past the
///   16-bit word ceiling (> 65,535, both modes) or — in strict mode only — past
///   the vanilla signed ceiling (> 32,767). Lenient mode instead recovers the
///   latter with a [`NodeBuildWarning::BlockmapVanillaOverflow`].
///
/// # Panics
///
/// Does not panic. The internal `expect` calls are guarded by construction:
/// origin components come from narrowed `i16` coordinates, grid spans are
/// non-negative and at most `u16::MAX` (so `columns`/`rows` fit `u16`), linedef
/// indices are bounded by the 65,535-linedef check above, block coordinates
/// are non-negative, and every stored offset passes the ceiling checks before
/// the offset table is filled.
#[allow(clippy::too_many_lines)]
pub fn build_blockmap(
    map: &Map,
    opts: &NodeBuildOptions,
) -> Result<(MapBlockmap, Vec<NodeBuildWarning>), NodeBuildError> {
    // Global Constraint 8: nothing to index without a full set of geometry.
    if map.vertices().is_empty()
        || map.linedefs().is_empty()
        || map.sidedefs().is_empty()
        || map.sectors().is_empty()
    {
        return Err(NodeBuildError::EmptyGeometry);
    }

    // ADR-0024 §3 / Global Constraint 9: narrow through the identical write-path
    // pass. A strict narrowing failure surfaces as `NodeBuildError::Write`
    // (the `?`/`From` conversion); recoveries become `NodeBuildWarning::Write`,
    // never `NodesNotBuilt` (this narrower is not seeded with it).
    let mut narrower = Narrower::new(opts.strictness);
    let arena = narrow_vertices(&mut narrower, map.vertices())?;
    let mut warnings: Vec<NodeBuildWarning> = narrower
        .warnings
        .into_iter()
        .map(NodeBuildWarning::Write)
        .collect();

    let linedefs = map.linedefs();
    // Both modes: `0xFFFF` is the blocklist terminator word, so the maximum
    // encodable linedef index is 65,534 — a 65,536th linedef (index 65,535)
    // would serialize as the terminator itself and corrupt every list carrying
    // it. This ceiling is therefore deliberately one element stricter than the
    // write path's generic 65,536-element `MAX_INDEXED`; not a strictness
    // question, the index is simply unrepresentable.
    if linedefs.len() > MAX_LINEDEFS {
        return Err(NodeBuildError::Write(DoomWriteError::TooManyElements {
            kind: "linedefs",
            count: linedefs.len(),
            max: MAX_LINEDEFS,
        }));
    }

    // Extents over linedef ENDPOINTS only (unused vertices never widen the grid).
    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;
    for line in linedefs {
        for vi in [line.start.0, line.end.0] {
            let v = &arena[vi];
            let (x, y) = (i32::from(v.x), i32::from(v.y));
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    }

    // Origin components come from narrowed i16 coordinates, so they fit i16.
    let origin_x = i16::try_from(min_x).expect("origin x is a narrowed i16 coordinate");
    let origin_y = i16::try_from(min_y).expect("origin y is a narrowed i16 coordinate");

    // Non-negative spans; `>> BLOCK_SHIFT` is floor-division by 128. Each span is
    // at most `u16::MAX`, so `columns`/`rows` are at most 512.
    let columns = usize::try_from((max_x - min_x) >> BLOCK_SHIFT).expect("non-negative span") + 1;
    let rows = usize::try_from((max_y - min_y) >> BLOCK_SHIFT).expect("non-negative span") + 1;
    let block_count = columns * rows;

    // Per-block linedef lists, row-major. Ascending linedef iteration means each
    // list is already in ascending order with no duplicates.
    let mut lists: Vec<Vec<u16>> = vec![Vec::new(); block_count];
    for (li, line) in linedefs.iter().enumerate() {
        // `li < linedefs.len() <= MAX_LINEDEFS`, so it fits `u16`.
        let idx = u16::try_from(li).expect("linedef count bounded above");
        let v1 = &arena[line.start.0];
        let v2 = &arena[line.end.0];
        let x1 = i32::from(v1.x) - min_x;
        let y1 = i32::from(v1.y) - min_y;
        let x2 = i32::from(v2.x) - min_x;
        let y2 = i32::from(v2.y) - min_y;

        let cx0 = block_lo(x1.min(x2));
        let cx1 = block_hi(x1.max(x2));
        let cy0 = block_lo(y1.min(y2));
        let cy1 = block_hi(y1.max(y2));
        for by in cy0..=cy1 {
            for bx in cx0..=cx1 {
                if crosses_block(x1, y1, x2, y2, bx, by) {
                    let row = usize::try_from(by).expect("non-negative block row");
                    let col = usize::try_from(bx).expect("non-negative block column");
                    lists[row * columns + col].push(idx);
                }
            }
        }
    }

    // Canonical word image: header, offset-table placeholder, then blocklists.
    let mut words: Vec<u16> = vec![
        as_word(origin_x),
        as_word(origin_y),
        u16::try_from(columns).expect("columns <= 512"),
        u16::try_from(rows).expect("rows <= 512"),
    ];
    let table_start = words.len();
    words.resize(table_start + block_count, 0);

    // Keyed by the bare list: the emitted framing (leading `0`, trailing
    // terminator) is a constant bijection of it, so equal lists imply equal
    // emitted words. Hit-path lookups borrow and allocate nothing; only the
    // first occurrence of a list clones it as the stored key.
    let mut dedup: HashMap<Vec<u16>, usize> = HashMap::new();
    let mut offsets: Vec<usize> = Vec::with_capacity(block_count);
    let mut vanilla_warned = false;
    for list in &lists {
        if let Some(&existing) = dedup.get(list) {
            offsets.push(existing);
            continue;
        }

        // Ceiling checks fire on each NEW list's start offset (ADR-0024 §5).
        let offset = words.len();
        if offset > WORD_OFFSET_CEILING {
            return Err(NodeBuildError::BlockmapOverflow { offset });
        }
        if offset > VANILLA_OFFSET_CEILING {
            match opts.strictness {
                Strictness::Strict => return Err(NodeBuildError::BlockmapOverflow { offset }),
                Strictness::Lenient => {
                    if !vanilla_warned {
                        warnings.push(NodeBuildWarning::BlockmapVanillaOverflow { offset });
                        vanilla_warned = true;
                    }
                }
            }
        }
        words.push(0);
        words.extend_from_slice(list);
        words.push(TERMINATOR);
        dedup.insert(list.clone(), offset);
        offsets.push(offset);
    }

    for (block, &offset) in offsets.iter().enumerate() {
        words[table_start + block] = u16::try_from(offset).expect("offset within word ceiling");
    }

    // Mirror the parser's arena layout (`MapBlockmap::parse` stores the full
    // word image): `entries` is the
    // full word image, one `LinedefIdx` per word; each block's range skips the
    // leading `0` delimiter and ends at its `0xFFFF` terminator.
    let entries: Vec<LinedefIdx> = words.iter().map(|&w| LinedefIdx(usize::from(w))).collect();
    let mut blocks: Vec<std::ops::Range<usize>> = Vec::with_capacity(block_count);
    for (list, &offset) in lists.iter().zip(&offsets) {
        let start = offset + 1;
        let end = start + list.len();
        blocks.push(start..end);
    }

    let blockmap = MapBlockmap {
        origin_x: f64::from(origin_x),
        origin_y: f64::from(origin_y),
        columns,
        rows,
        entries,
        blocks,
    };
    Ok((blockmap, warnings))
}

impl MapBlockmap {
    /// Serializes this `BLOCKMAP` to its lump bytes: the stored word image
    /// (`entries`) dumped verbatim as little-endian `u16`.
    ///
    /// The parser stores the complete word image — header, offset table,
    /// delimiters, and terminators included ([`MapBlockmap::parse`] builds
    /// its arena the same way) — so
    /// serialization is a straight word dump. Round-trips exactly through
    /// [`MapBlockmap::parse`] against the owning linedef count (ADR-0024 §7).
    ///
    /// # Errors
    ///
    /// [`NodeBuildError::BlockmapOverflow`] carrying the offending word index if
    /// any entry exceeds `u16::MAX`. This is a defensive invariant only:
    /// [`build_blockmap`] and [`MapBlockmap::parse`] both source `entries` from
    /// `u16` words, so it is unreachable for values either produces — it guards
    /// the `pub(crate)` field against a future in-crate constructor that stores
    /// a wider value.
    pub fn to_lump_bytes(&self) -> Result<Vec<u8>, NodeBuildError> {
        let mut out = Vec::with_capacity(self.entries.len() * 2);
        for (i, entry) in self.entries.iter().enumerate() {
            let word = u16::try_from(entry.0)
                .map_err(|_| NodeBuildError::BlockmapOverflow { offset: i })?;
            out.extend_from_slice(&word.to_le_bytes());
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::MapWarning;
    use crate::map::doom::DoomWriteWarning;
    use crate::map::graph::{
        MapFormat, MapLinedef, MapSector, MapSidedef, MapThing, MapVertex, SectorIdx, SidedefIdx,
        Special, TextureRef, VertexIdx,
    };
    use proptest::prelude::*;

    /// Builds a `Map` from vertex coordinates and `(start, end)` linedef index
    /// pairs, with the minimum sidedef/sector needed to clear the empty-geometry
    /// gate. Coordinates are `f64` so narrowing edge cases are reachable.
    fn map_from(vertices: &[(f64, f64)], lines: &[(usize, usize)]) -> Map {
        Map {
            name: "MAP01".into(),
            format: MapFormat::Doom,
            namespace: None,
            vertices: vertices.iter().map(|&(x, y)| MapVertex { x, y }).collect(),
            linedefs: lines
                .iter()
                .map(|&(s, e)| MapLinedef {
                    start: VertexIdx(s),
                    end: VertexIdx(e),
                    right: Some(SidedefIdx(0)),
                    left: None,
                    flags: 0,
                    special: Special {
                        special: 0,
                        args: [0; 5],
                    },
                    id: 0,
                })
                .collect(),
            sidedefs: vec![MapSidedef {
                sector: SectorIdx(0),
                x_offset: 0,
                y_offset: 0,
                upper: TextureRef::Name("-".into()),
                lower: TextureRef::Name("-".into()),
                middle: TextureRef::Name("STARTAN3".into()),
            }],
            sectors: vec![MapSector {
                floor_height: 0,
                ceiling_height: 128,
                floor_flat: TextureRef::Name("FLOOR4_8".into()),
                ceiling_flat: TextureRef::Name("CEIL3_5".into()),
                light: 160,
                special: 0,
                tag: 0,
                colors: None,
                flags: 0,
            }],
            things: vec![MapThing {
                x: 0.0,
                y: 0.0,
                angle: 0,
                type_id: 1,
                flags: 0,
                id: 0,
                height: 0.0,
                special: Special {
                    special: 0,
                    args: [0; 5],
                },
            }],
            lights: Vec::new(),
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
            game: None,
            warnings: Vec::new(),
        }
    }

    #[test]
    fn hand_fixture_matches_controller_derivation() {
        // (0,0)-(64,0): 1x1 grid, one block listing linedef 0.
        let map = map_from(&[(0.0, 0.0), (64.0, 0.0)], &[(0, 1)]);
        let (bm, warnings) = build_blockmap(&map, &NodeBuildOptions::strict()).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(bm.origin(), (0.0, 0.0));
        assert_eq!((bm.columns(), bm.rows()), (1, 1));

        let expected_bytes: Vec<u8> = [
            0x0000u16, 0x0000, 0x0001, 0x0001, 0x0005, 0x0000, 0x0000, 0xFFFF,
        ]
        .iter()
        .flat_map(|w| w.to_le_bytes())
        .collect();
        assert_eq!(bm.to_lump_bytes().unwrap(), expected_bytes);
        assert_eq!(
            bm.block(0, 0),
            Some(&[LinedefIdx(0)][..]),
            "the single block lists linedef 0"
        );
    }

    #[test]
    fn zero_length_linedef_is_listed_in_its_block() {
        // A degenerate point at (50,50): origin (50,50), 1x1 grid, block 0.
        let map = map_from(&[(50.0, 50.0)], &[(0, 0)]);
        let (bm, _) = build_blockmap(&map, &NodeBuildOptions::strict()).unwrap();
        assert_eq!((bm.columns(), bm.rows()), (1, 1));
        assert_eq!(bm.block(0, 0), Some(&[LinedefIdx(0)][..]));
    }

    #[test]
    fn too_many_linedefs_errors_in_both_modes() {
        // 65,536 linedefs over two shared vertices: index 65,535 would collide
        // with the 0xFFFF blocklist terminator word, so the ceiling is 65,535 —
        // one element stricter than the write path's generic 65,536 — and the
        // rejection is unconditional (not a strictness question).
        let mut map = map_from(&[(0.0, 0.0), (64.0, 0.0)], &[(0, 1)]);
        map.linedefs = vec![map.linedefs[0].clone(); 65_536];
        for opts in [NodeBuildOptions::strict(), NodeBuildOptions::lenient()] {
            assert_eq!(
                build_blockmap(&map, &opts).unwrap_err(),
                NodeBuildError::Write(DoomWriteError::TooManyElements {
                    kind: "linedefs",
                    count: 65_536,
                    max: 65_535,
                })
            );
        }
    }

    #[test]
    fn empty_geometry_is_rejected_in_both_modes() {
        // A directly-built Map with empty arenas. Assembly can produce one
        // too — `decode_required` accepts present-but-empty required lumps —
        // which is exactly why the EmptyGeometry gate exists.
        let mut map = map_from(&[(0.0, 0.0), (64.0, 0.0)], &[(0, 1)]);
        map.linedefs.clear();
        for opts in [NodeBuildOptions::strict(), NodeBuildOptions::lenient()] {
            assert_eq!(
                build_blockmap(&map, &opts).unwrap_err(),
                NodeBuildError::EmptyGeometry
            );
        }
    }

    #[test]
    fn fractional_coordinate_shares_the_write_path_narrowing() {
        // Global Constraint 9: strict errors via the write path; lenient rounds
        // and warns with a `Write` warning, never `NodesNotBuilt`.
        let map = map_from(&[(0.5, 0.0), (64.0, 0.0)], &[(0, 1)]);

        let err = build_blockmap(&map, &NodeBuildOptions::strict()).unwrap_err();
        assert!(matches!(
            err,
            NodeBuildError::Write(DoomWriteError::FractionalCoordinate { .. })
        ));

        let (_, warnings) = build_blockmap(&map, &NodeBuildOptions::lenient()).unwrap();
        assert!(warnings.iter().any(|w| matches!(
            w,
            NodeBuildWarning::Write(DoomWriteWarning::CoordinateRounded { .. })
        )));
        assert!(
            !warnings
                .iter()
                .any(|w| matches!(w, NodeBuildWarning::Write(DoomWriteWarning::NodesNotBuilt))),
            "nodebuild warnings must never carry NodesNotBuilt"
        );
    }

    #[test]
    fn determinism_identical_bytes_across_builds() {
        let map = map_from(
            &[(0.0, 0.0), (300.0, 0.0), (0.0, 300.0), (128.0, 128.0)],
            &[(0, 1), (0, 2), (1, 3)],
        );
        let a = build_blockmap(&map, &NodeBuildOptions::strict())
            .unwrap()
            .0
            .to_lump_bytes()
            .unwrap();
        let b = build_blockmap(&map, &NodeBuildOptions::strict())
            .unwrap()
            .0
            .to_lump_bytes()
            .unwrap();
        assert_eq!(a, b);
    }

    /// Reads a single little-endian `u16` word at `index` from lump bytes.
    fn word_at(bytes: &[u8], index: usize) -> u16 {
        u16::from_le_bytes([bytes[index * 2], bytes[index * 2 + 1]])
    }

    #[test]
    fn empty_blocks_share_one_offset() {
        // 3x3 L: horizontal (0,0)-(300,0) + vertical (0,0)-(0,300) leaves the
        // four top-right blocks empty; all four must share one blocklist offset.
        let map = map_from(&[(0.0, 0.0), (300.0, 0.0), (0.0, 300.0)], &[(0, 1), (0, 2)]);
        let (bm, _) = build_blockmap(&map, &NodeBuildOptions::strict()).unwrap();
        assert_eq!((bm.columns(), bm.rows()), (3, 3));
        let bytes = bm.to_lump_bytes().unwrap();

        let empty = [(1usize, 1usize), (2, 1), (1, 2), (2, 2)];
        for &(col, row) in &empty {
            assert_eq!(bm.block(col, row), Some(&[][..]), "block is empty");
        }
        // Offset table word for block `row*columns+col` lives at word 4 + that.
        let (c0, r0) = empty[0];
        let shared = word_at(&bytes, 4 + (r0 * 3 + c0));
        for &(col, row) in &empty {
            assert_eq!(
                word_at(&bytes, 4 + (row * 3 + col)),
                shared,
                "every empty block dedups to one offset"
            );
        }
    }

    #[test]
    fn boundary_line_is_listed_in_both_adjacent_columns() {
        // Horizontal (0,0)-(300,0) fixes origin 0 and a 3-column grid; the
        // vertical line exactly on the x=128 block boundary is the closed-
        // interval superset case (Global Constraint 6).
        let map = map_from(
            &[(0.0, 0.0), (300.0, 0.0), (128.0, 0.0), (128.0, 100.0)],
            &[(0, 1), (2, 3)],
        );
        let (bm, _) = build_blockmap(&map, &NodeBuildOptions::strict()).unwrap();
        assert_eq!((bm.columns(), bm.rows()), (3, 1));

        // Horizontal linedef 0 crosses all three columns.
        for col in 0..3 {
            assert!(
                bm.block(col, 0).unwrap().contains(&LinedefIdx(0)),
                "linedef 0 missing from column {col}"
            );
        }
        // Vertical linedef 1 at x=128 is listed in BOTH adjacent columns 0 and 1.
        assert!(bm.block(0, 0).unwrap().contains(&LinedefIdx(1)));
        assert!(bm.block(1, 0).unwrap().contains(&LinedefIdx(1)));
    }

    /// The grid arithmetic the ceiling tests assert, computed from the formula
    /// rather than hardcoded, so a formula change is caught here too.
    fn grid_and_first_offset(span: i32) -> (usize, usize) {
        let columns = usize::try_from(span >> BLOCK_SHIFT).unwrap() + 1;
        let block_count = columns * columns;
        (columns, 4 + block_count)
    }

    #[test]
    fn vanilla_offset_ceiling_strict_errors_lenient_warns_once() {
        // Diagonal (0,0)-(25000,25000): 196x196 grid, first list offset 38420
        // (> 32767, <= 65535).
        let span = 25_000;
        let (columns, first_offset) = grid_and_first_offset(span);
        assert_eq!(columns, 196);
        assert_eq!(first_offset, 38_420);
        assert!(first_offset > VANILLA_OFFSET_CEILING && first_offset <= WORD_OFFSET_CEILING);

        let map = map_from(&[(0.0, 0.0), (f64::from(span), f64::from(span))], &[(0, 1)]);

        assert_eq!(
            build_blockmap(&map, &NodeBuildOptions::strict()).unwrap_err(),
            NodeBuildError::BlockmapOverflow {
                offset: first_offset
            }
        );

        let (bm, warnings) = build_blockmap(&map, &NodeBuildOptions::lenient()).unwrap();
        assert_eq!(
            warnings
                .iter()
                .filter(|w| matches!(w, NodeBuildWarning::BlockmapVanillaOverflow { .. }))
                .count(),
            1,
            "exactly one vanilla-overflow warning (first offender only)"
        );
        // Round-trip still holds in lenient mode.
        let bytes = bm.to_lump_bytes().unwrap();
        let mut parse_warnings: Vec<MapWarning> = Vec::new();
        let parsed = MapBlockmap::parse(&bytes, 1, Strictness::Strict, &mut parse_warnings)
            .unwrap()
            .unwrap();
        assert_eq!(parsed, bm);
        assert!(parse_warnings.is_empty());
    }

    #[test]
    fn word_offset_ceiling_errors_in_both_modes() {
        // Diagonal (0,0)-(32700,32700): 256x256 grid, first list offset 65540
        // (> 65535).
        let span = 32_700;
        let (columns, first_offset) = grid_and_first_offset(span);
        assert_eq!(columns, 256);
        assert_eq!(first_offset, 65_540);
        assert!(first_offset > WORD_OFFSET_CEILING);

        let map = map_from(&[(0.0, 0.0), (f64::from(span), f64::from(span))], &[(0, 1)]);
        for opts in [NodeBuildOptions::strict(), NodeBuildOptions::lenient()] {
            assert_eq!(
                build_blockmap(&map, &opts).unwrap_err(),
                NodeBuildError::BlockmapOverflow {
                    offset: first_offset
                }
            );
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        /// Round-trip typed equality and the no-missing sampling oracle: every
        /// point sampled along a linedef (steps <= 16 units, endpoints included)
        /// must fall in a block that lists that linedef.
        #[test]
        fn random_maps_round_trip_and_have_no_missing_listings(
            coords in prop::collection::vec((-2000i16..=2000, -2000i16..=2000), 2..=12),
            raw_lines in prop::collection::vec((0usize..12, 0usize..12), 1..=16),
        ) {
            let verts: Vec<(f64, f64)> = coords
                .iter()
                .map(|&(x, y)| (f64::from(x), f64::from(y)))
                .collect();
            let n = verts.len();
            let lines: Vec<(usize, usize)> =
                raw_lines.iter().map(|&(s, e)| (s % n, e % n)).collect();
            let map = map_from(&verts, &lines);

            let (bm, warnings) = build_blockmap(&map, &NodeBuildOptions::strict()).unwrap();
            prop_assert!(warnings.is_empty());

            // (a) Round-trip typed equality through parse.
            let bytes = bm.to_lump_bytes().unwrap();
            let mut parse_warnings: Vec<MapWarning> = Vec::new();
            let parsed =
                MapBlockmap::parse(&bytes, lines.len(), Strictness::Strict, &mut parse_warnings)
                    .unwrap()
                    .unwrap();
            prop_assert_eq!(&parsed, &bm);
            prop_assert!(parse_warnings.is_empty());

            // (b) No-missing sampling oracle. Sample exact points ON each
            // segment (float, so a point is genuinely on the line, unlike an
            // integer-rounded lattice point) and require its block to list the
            // linedef. Integer coords narrow to themselves, so `bm.origin()` is
            // the min endpoint coordinate.
            let (ox, oy) = bm.origin();
            let last_col = f64::from(u32::try_from(bm.columns() - 1).unwrap());
            let last_row = f64::from(u32::try_from(bm.rows() - 1).unwrap());
            for (li, &(s, e)) in lines.iter().enumerate() {
                let (x1, y1) = (f64::from(coords[s].0), f64::from(coords[s].1));
                let (x2, y2) = (f64::from(coords[e].0), f64::from(coords[e].1));
                let span = (x2 - x1).abs().max((y2 - y1).abs());
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let steps = (span / 16.0).ceil().max(1.0) as u32;
                for step in 0..=steps {
                    let t = f64::from(step) / f64::from(steps);
                    let px = x1 + (x2 - x1) * t;
                    let py = y1 + (y2 - y1) * t;
                    let col_f = ((px - ox) / 128.0).floor().clamp(0.0, last_col);
                    let row_f = ((py - oy) / 128.0).floor().clamp(0.0, last_row);
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let (col, row) = (col_f as usize, row_f as usize);
                    let listed = bm.block(col, row).unwrap().contains(&LinedefIdx(li));
                    prop_assert!(
                        listed,
                        "linedef {li} sample ({px},{py}) missing from block ({col},{row})"
                    );
                }
            }
        }
    }
}
