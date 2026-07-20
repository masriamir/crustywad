//! The engine-playable one-shot: [`add_doom_map_with_nodes`] bundles a map's
//! five data lumps with real, built node lumps (`SEGS`/`SSECTORS`/`NODES`,
//! `REJECT`, `BLOCKMAP`) into a [`WadBuilder`], so the resulting WAD runs in a
//! vanilla engine without an external nodebuilder pass (ADR-0024 §4).

use crate::map::Map;
use crate::map::doom::DoomWriteWarning;
use crate::map::write_doom_map;
use crate::write::{WadBuilder, WriteOptions};

use super::{
    NodeBuildError, NodeBuildOptions, NodeBuildWarning, build_blockmap, build_nodes, build_reject,
};

/// Serializes `map`, builds its node lumps, and adds a complete, **engine-playable**
/// Doom map group to `builder`: the `name` marker, the five data lumps, and the
/// built `SEGS`, `SSECTORS`, `NODES`, `REJECT`, and `BLOCKMAP` lumps in canonical
/// order (ADR-0024 §4).
///
/// Unlike [`add_doom_map`](crate::map::add_doom_map) — which emits zero-length
/// node lumps and always warns [`DoomWriteWarning::NodesNotBuilt`] — this one-shot
/// runs the clean-room builders ([`build_reject`], [`build_blockmap`],
/// [`build_nodes`]) and writes their output, so the map is playable as-is. The
/// returned warnings therefore **never** include
/// [`DoomWriteWarning::NodesNotBuilt`]: that warning describes the empty-lump
/// write path and is filtered out here (Global Constraint 4). Every other
/// write-path recovery still surfaces, wrapped as [`NodeBuildWarning::Write`].
///
/// The `REJECT` table [`build_reject`] produces is all zeros: it rejects no
/// sector pair, so every pair is treated as potentially visible. That is the
/// engine-correct default (a set bit only ever *suppresses* a line-of-sight
/// check) and exactly what a freshly built map wants.
///
/// The lumps are added in the canonical order (Global Constraint 5): the `name`
/// marker, `THINGS`, `LINEDEFS`, `SIDEDEFS`, `VERTEXES` (the map's vertices with
/// the BSP pass's split vertices appended), `SEGS`, `SSECTORS`, `NODES`,
/// `SECTORS`, `REJECT`, `BLOCKMAP`. The split vertices **must** be appended to
/// `VERTEXES` (done here) or the seg vertex indices would dangle.
///
/// Warnings are returned in a deterministic order (Global Constraint 6): the
/// write-path warnings first (excluding `NodesNotBuilt`), then the
/// `BLOCKMAP`-build warnings, then the BSP-build warnings.
///
/// The caller invokes [`WadBuilder::build`] afterward (which returns
/// [`WriteError`](crate::WriteError)).
///
/// # Errors
///
/// Returns a [`NodeBuildError`] if any of the three builders or the shared write
/// pass fails; the builder is left unmodified in that case (every build runs
/// before the first lump is added). Reachable variants:
///
/// - [`NodeBuildError::EmptyGeometry`] (**both** modes) — the map has no geometry
///   to build nodes or a blockmap from (zero vertices, linedefs, sidedefs, or
///   sectors, or geometry that yields no segs).
/// - [`NodeBuildError::Write`] wrapping any [`DoomWriteError`](crate::map::DoomWriteError)
///   — a coordinate or field could not be narrowed to its Doom on-disk type in
///   strict mode (via [`write_doom_map`] or the builders' shared narrowing pass).
/// - [`NodeBuildError::BlockmapOverflow`] — a `BLOCKMAP` word offset exceeds the
///   addressable range (or, in strict mode, the vanilla signed-offset ceiling).
/// - [`NodeBuildError::TooManyElements`] (**both** modes) — a BSP arena exceeds
///   what the on-disk format can index.
/// - [`NodeBuildError::MixedSectorSubsector`] (**strict** mode) — a convex
///   subsector spans multiple sectors with no separating partition.
/// - [`NodeBuildError::DegeneratePartition`] (**both** modes) — a hardening guard
///   for adversarial geometry a selected partition cannot separate.
pub fn add_doom_map_with_nodes(
    builder: &mut WadBuilder,
    name: &str,
    map: &Map,
    write_opts: &WriteOptions,
    build_opts: &NodeBuildOptions,
) -> Result<Vec<NodeBuildWarning>, NodeBuildError> {
    // 1. The five data lumps and the write-path warnings, first: strict-mode
    //    narrowing of any field (THINGS, LINEDEFS, …) errors here, before the
    //    expensive BSP/blockmap builds, so a doomed conversion fails fast.
    let (mut data, write_ws) = write_doom_map(map, write_opts)?;

    // 2. REJECT — infallible, all-zeros (engine-correct: every sector pair visible).
    let reject = build_reject(map).to_lump_bytes();

    // 3. BLOCKMAP — collect its build warnings.
    let (blockmap, blockmap_ws) = build_blockmap(map, build_opts)?;
    let blockmap = blockmap.to_lump_bytes()?;

    // 4. SEGS/SSECTORS/NODES — collect its build warnings.
    let (nodes, node_ws) = build_nodes(map, build_opts)?;
    let node_lumps = nodes.to_lump_bytes()?;

    // Deterministic warning order: write (minus NodesNotBuilt), blockmap, nodes.
    // `NodesNotBuilt` describes the empty-lump write path and is a lie here — we
    // built the nodes — so it is filtered out (Global Constraint 4).
    let mut warnings: Vec<NodeBuildWarning> = write_ws
        .into_iter()
        .filter(|w| !matches!(w, DoomWriteWarning::NodesNotBuilt))
        .map(NodeBuildWarning::Write)
        .collect();
    warnings.extend(blockmap_ws);
    warnings.extend(node_ws);

    // 5. Append the split vertices so the seg vertex indices resolve
    //    (Global Constraint 5).
    data.vertexes.extend_from_slice(&node_lumps.split_vertexes);

    // 6. Add the eleven lumps in canonical order (Global Constraint 5).
    builder.add_lump(name, b"");
    builder.add_lump("THINGS", data.things);
    builder.add_lump("LINEDEFS", data.linedefs);
    builder.add_lump("SIDEDEFS", data.sidedefs);
    builder.add_lump("VERTEXES", data.vertexes);
    builder.add_lump("SEGS", node_lumps.segs);
    builder.add_lump("SSECTORS", node_lumps.ssectors);
    builder.add_lump("NODES", node_lumps.nodes);
    builder.add_lump("SECTORS", data.sectors);
    builder.add_lump("REJECT", reject);
    builder.add_lump("BLOCKMAP", blockmap);

    Ok(warnings)
}
