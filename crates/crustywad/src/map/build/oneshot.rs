//! The engine-playable one-shot: [`add_doom_map_with_nodes`] bundles a map's
//! five data lumps with real, built node lumps (`SEGS`/`SSECTORS`/`NODES`,
//! `REJECT`, `BLOCKMAP`) into a [`WadBuilder`], so the resulting WAD runs in a
//! vanilla engine without an external nodebuilder pass (ADR-0024 §4).
//! [`add_udmf_map_with_nodes`] is its UDMF counterpart: it bundles a UDMF map
//! group (`TEXTMAP`) with a built GL `ZNODES` stream.

use crate::map::Map;
use crate::map::doom::DoomWriteWarning;
use crate::map::udmf::write_udmf;
use crate::map::write_doom_map;
use crate::write::{WadBuilder, WriteOptions};

use super::{
    NodeBuildError, NodeBuildOptions, NodeBuildWarning, build_blockmap, build_gl_nodes,
    build_nodes, build_reject,
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
/// marker, `THINGS`, `LINEDEFS`, `SIDEDEFS`, `VERTEXES`, `SEGS`, `SSECTORS`,
/// `NODES`, `SECTORS`, `REJECT`, `BLOCKMAP`. Under the default
/// [`Classic`](super::NodeFormat::Classic) format, `VERTEXES` carries the map's
/// vertices with the BSP pass's split vertices appended — they **must** be
/// appended (done here) or the classic seg vertex indices would dangle. The extended and
/// GL formats below instead carry builder-added vertices in their stream
/// headers and leave `VERTEXES` untouched.
///
/// When [`build_opts.format`](NodeBuildOptions::format) selects a non-GL
/// extended format (ADR-0025, #323) — [`Xnod`](super::NodeFormat::Xnod) or
/// `Znod` — the node data is instead emitted as a
/// single `XNOD`/`ZNOD` stream in `NODES`, with `SEGS`/`SSECTORS` left empty
/// and the split vertices left out of `VERTEXES` entirely (they live in the
/// stream header, not the classic lump). The default
/// [`Classic`](super::NodeFormat::Classic) writes the vanilla three-lump
/// layout described above.
///
/// When `build_opts.format` selects a GL format (ADR-0026 §3, #364) —
/// [`Xgl3`](super::NodeFormat::Xgl3) or `Zgl3` — the classic BSP pass
/// ([`build_nodes`]) does **not** run at all; [`build_gl_nodes`] runs in its
/// place and its `XGL3`/`ZGL3` stream is carried in **`SSECTORS`** instead of
/// `NODES` (both `SEGS` and `NODES` are left empty). This is the carrier
/// inverse of the reader's dispatch, which probes `NODES` first and falls
/// back to a GL-stream signature in `SSECTORS` only when `NODES` is empty. As
/// with the non-GL extended formats, the GL split vertices live in the
/// stream's own vertex header and are never appended to `VERTEXES`.
///
/// Warnings are returned in a deterministic order (Global Constraint 6): the
/// write-path warnings first (excluding `NodesNotBuilt`), then the
/// `BLOCKMAP`-build warnings, then the BSP-build warnings (from whichever of
/// [`build_nodes`] or [`build_gl_nodes`] ran for the selected format).
///
/// The caller invokes [`WadBuilder::build`] afterward (which returns
/// [`WriteError`](crate::WriteError)).
///
/// # Errors
///
/// Returns a [`NodeBuildError`] if any of the builders, the shared write
/// pass, or the node-lump serialization fails; the builder is left unmodified
/// in that case (every build runs before the first lump is added). Reachable
/// variants:
///
/// - [`NodeBuildError::EmptyGeometry`] (**both** modes) — the map has no geometry
///   to build nodes or a blockmap from (zero vertices, linedefs, sidedefs, or
///   sectors, or geometry that yields no segs).
/// - [`NodeBuildError::Write`] wrapping any [`DoomWriteError`](crate::map::DoomWriteError)
///   — a coordinate or field could not be narrowed to its Doom on-disk type in
///   strict mode (via [`write_doom_map`] or the builders' shared narrowing pass).
/// - [`NodeBuildError::BlockmapOverflow`] — a `BLOCKMAP` word offset exceeds the
///   addressable range (or, in strict mode, the vanilla signed-offset ceiling).
/// - [`NodeBuildError::TooManyElements`] (**both** modes) — a BSP (or GL BSP)
///   arena exceeds what the on-disk format can index.
/// - [`NodeBuildError::MixedSectorSubsector`] (**strict** mode) — a convex
///   subsector spans multiple sectors with no separating partition. Reachable
///   from either [`build_nodes`] or [`build_gl_nodes`].
/// - [`NodeBuildError::DegeneratePartition`] (**both** modes) — a hardening guard
///   for adversarial geometry a selected partition cannot separate.
/// - For a non-GL extended format, the errors documented on
///   [`BuiltNodes::to_extended_lump_bytes`](super::BuiltNodes::to_extended_lump_bytes)
///   propagate as well (e.g. [`NodeBuildError::MinisegUnsupported`],
///   [`NodeBuildError::CompressionUnavailable`]).
/// - For a GL format, the errors documented on
///   [`BuiltGlNodes::to_extended_lump_bytes`](super::BuiltGlNodes::to_extended_lump_bytes)
///   propagate instead (e.g. [`NodeBuildError::TooManyElements`] when the GL arena
///   overflows the extended index space, or [`NodeBuildError::Write`] wrapping a
///   coordinate that will not narrow).
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

    // Shared warning prefix: write (minus NodesNotBuilt) + blockmap. Each format
    // arm below appends its own build warnings last, keeping the deterministic
    // order (Global Constraint 6). `NodesNotBuilt` describes the empty-lump
    // write path and is a lie here — we built the nodes — so it is filtered out
    // (Global Constraint 4).
    let mut warnings: Vec<NodeBuildWarning> = write_ws
        .into_iter()
        .filter(|w| !matches!(w, DoomWriteWarning::NodesNotBuilt))
        .map(NodeBuildWarning::Write)
        .collect();
    warnings.extend(blockmap_ws);

    // 4. Emit the node lumps per the target format (ADR-0025 §5, ADR-0026 §3).
    if build_opts.format.is_gl() {
        // GL: a single XGL3/ZGL3 stream carried in SSECTORS — the inverse of
        // the reader's NODES-then-SSECTORS probe (an empty NODES lump makes
        // the assembler fall through to SSECTORS, where a GL stream signature
        // is recognized; ADR-0026 §3). SEGS and NODES are both left empty, and
        // — like the non-GL extended arm — the split (GL) vertices live in the
        // stream's own vertex header, NOT appended to VERTEXES. The classic
        // `build_nodes` BSP pass does not run at all in this arm.
        let (gl_nodes, gl_ws) = build_gl_nodes(map, build_opts)?;
        let stream = gl_nodes.to_extended_lump_bytes(map.vertices().len(), build_opts.format)?;
        warnings.extend(gl_ws);

        builder.add_lump(name, b"");
        builder.add_lump("THINGS", data.things);
        builder.add_lump("LINEDEFS", data.linedefs);
        builder.add_lump("SIDEDEFS", data.sidedefs);
        builder.add_lump("VERTEXES", data.vertexes);
        builder.add_lump("SEGS", b"");
        builder.add_lump("SSECTORS", stream);
        builder.add_lump("NODES", b"");
        builder.add_lump("SECTORS", data.sectors);
        builder.add_lump("REJECT", reject);
        builder.add_lump("BLOCKMAP", blockmap);
        return Ok(warnings);
    }

    // 5. SEGS/SSECTORS/NODES (classic or non-GL extended) — collect its build
    //    warnings.
    let (nodes, node_ws) = build_nodes(map, build_opts)?;
    warnings.extend(node_ws);

    if build_opts.format.is_extended() {
        // Extended: a single XNOD/ZNOD stream in NODES; SEGS/SSECTORS empty; the
        // split vertices live in the stream header, NOT appended to VERTEXES.
        let stream =
            nodes.to_extended_lump_bytes(map.vertices().len(), build_opts.format.compressed())?;
        builder.add_lump(name, b"");
        builder.add_lump("THINGS", data.things);
        builder.add_lump("LINEDEFS", data.linedefs);
        builder.add_lump("SIDEDEFS", data.sidedefs);
        builder.add_lump("VERTEXES", data.vertexes);
        builder.add_lump("SEGS", b"");
        builder.add_lump("SSECTORS", b"");
        builder.add_lump("NODES", stream);
        builder.add_lump("SECTORS", data.sectors);
        builder.add_lump("REJECT", reject);
        builder.add_lump("BLOCKMAP", blockmap);
        return Ok(warnings);
    }

    // Classic: the vanilla three-lump layout with split verts appended to
    // VERTEXES so the seg vertex indices resolve (Global Constraint 5).
    let node_lumps = nodes.to_lump_bytes()?;
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

/// Serializes `map` as a UDMF map group with freshly built nodes — the
/// `name` marker, `TEXTMAP`, a `ZNODES` lump carrying the node stream,
/// and `ENDMAP` — and adds all four lumps to `builder` (ADR-0026 §3, #354;
/// ADR-0025, #384).
///
/// The `ZNODES` payload depends on [`build_opts.format`](NodeBuildOptions::format):
///
/// - A **GL** format ([`Xgln`](super::NodeFormat::Xgln)/`Xgl2`/`Xgl3`, their
///   `Z`-prefixed zlib twins, or the auto-resolving
///   [`Gl`](super::NodeFormat::Gl)/`Zgl`) runs the GL kernel [`build_gl_nodes`]
///   and serializes an `XGL*`/`ZGL*` stream via
///   [`BuiltGlNodes::to_extended_lump_bytes`](super::BuiltGlNodes::to_extended_lump_bytes)
///   (`Gl`/`Zgl` auto-select the minimal dialect).
/// - A **non-GL extended** format ([`Xnod`](super::NodeFormat::Xnod) or `Znod`)
///   runs the classic BSP pass [`build_nodes`] and serializes an `XNOD`/`ZNOD`
///   stream via
///   [`BuiltNodes::to_extended_lump_bytes`](super::BuiltNodes::to_extended_lump_bytes)
///   (ADR-0025, #384).
/// - The default [`Classic`](super::NodeFormat::Classic) format is rejected with
///   [`NodeBuildError::UnsupportedNodeFormat`] — UDMF has no `SEGS`/`SSECTORS`/
///   `NODES` lumps to carry a classic binary tree; all UDMF node data lives in
///   the single self-describing `ZNODES` stream.
///
/// The lumps are added in the UDMF canonical order: the `name` marker,
/// `TEXTMAP`, `ZNODES`, `ENDMAP` — the same layout
/// [`add_udmf_map`](crate::map::add_udmf_map) emits, with the built `ZNODES`
/// inserted before `ENDMAP`. Unlike the Doom one-shot, no `VERTEXES` fix-up is
/// needed for either family: the split (or GL) vertices live in the stream's own
/// vertex header, and UDMF geometry lives entirely in `TEXTMAP`.
///
/// Warnings are returned in a deterministic order: the UDMF write-path warnings
/// first (wrapped as [`NodeBuildWarning::UdmfWrite`]), then the node-build
/// warnings (wrapped as their existing [`NodeBuildWarning`] variants) — the
/// write-then-build ordering the Doom one-shot uses.
///
/// The caller invokes [`WadBuilder::build`] afterward (which returns
/// [`WriteError`](crate::WriteError)).
///
/// # Errors
///
/// Returns a [`NodeBuildError`] if the UDMF write, the node build, or the stream
/// serialization fails; the builder is left unmodified in that case (every
/// fallible step runs before the first lump is added). Reachable variants:
///
/// - [`NodeBuildError::UdmfWrite`] wrapping any
///   [`UdmfWriteError`](crate::map::UdmfWriteError) — e.g. a NaN/∞ coordinate,
///   an empty namespace, a linedef with no front sidedef, an unresolved
///   texture index, or an unrepresentable/unsupported source format (see
///   [`write_udmf`]).
/// - [`NodeBuildError::UnsupportedNodeFormat`] (**both** modes) —
///   `build_opts.format` is [`Classic`](super::NodeFormat::Classic).
/// - [`NodeBuildError::EmptyGeometry`] (**both** modes) — the map yields no
///   segs to build a tree from.
/// - [`NodeBuildError::MixedSectorSubsector`] (**strict** mode) — a convex
///   subsector spans multiple sectors with no separating partition. Reachable
///   from either [`build_gl_nodes`] or [`build_nodes`].
/// - [`NodeBuildError::DegeneratePartition`] (**both** modes) — a hardening guard
///   for adversarial geometry a selected partition cannot separate.
/// - For a **GL** format, the errors documented on
///   [`BuiltGlNodes::to_extended_lump_bytes`](super::BuiltGlNodes::to_extended_lump_bytes)
///   propagate as well — e.g. [`NodeBuildError::TooManyElements`] when the GL
///   arena overflows the extended index space,
///   [`NodeBuildError::PartitionPrecision`] when an explicit `Xgln`/`Xgl2` meets
///   a fractional partition, [`NodeBuildError::CompressionUnavailable`] for a
///   `Z…` GL format without the `extended-nodes-zlib` feature, or
///   [`NodeBuildError::Write`] wrapping a coordinate that will not narrow.
/// - For a **non-GL extended** format, the errors documented on
///   [`BuiltNodes::to_extended_lump_bytes`](super::BuiltNodes::to_extended_lump_bytes)
///   propagate — e.g. [`NodeBuildError::TooManyElements`] when the arena or a
///   seg's linedef index overflows the extended ceilings, or
///   [`NodeBuildError::Write`] wrapping a coordinate that will not narrow.
///   [`NodeBuildError::MinisegUnsupported`] is documented there but is **not**
///   reachable through this path: the classic BSP pass [`build_nodes`] emits only
///   linedef-backed segs. [`NodeBuildError::CompressionUnavailable`] is likewise
///   unreachable here — the compressed [`Znod`](super::NodeFormat::Znod) variant
///   exists only when the `extended-nodes-zlib` feature (which supplies the
///   compressor) is enabled.
pub fn add_udmf_map_with_nodes(
    builder: &mut WadBuilder,
    name: &str,
    map: &Map,
    write_opts: &WriteOptions,
    build_opts: &NodeBuildOptions,
) -> Result<Vec<NodeBuildWarning>, NodeBuildError> {
    // Every fallible step runs before the first `add_lump`, so the builder is
    // untouched on error (asserted by the reject-Classic and cfg tests): the
    // UDMF text first, then the node build, then the stream serialization.
    let (text, udmf_ws) =
        write_udmf(map, write_opts).map_err(|source| NodeBuildError::UdmfWrite { source })?;

    // Build the ZNODES stream per the target format family. UDMF stores no
    // classic binary node lumps, so `Classic` (neither GL nor extended) is
    // rejected explicitly with `UnsupportedNodeFormat`; a GL format runs the GL
    // kernel and carries an XGL*/ZGL* stream; a non-GL extended format (`Xnod`/
    // `Znod`) runs the classic BSP pass and carries an XNOD/ZNOD stream. Every
    // fallible call in each arm runs before any `add_lump`, preserving fail-fast.
    let (znodes, build_ws): (Vec<u8>, Vec<NodeBuildWarning>) = if build_opts.format.is_gl() {
        let (gl, gl_ws) = build_gl_nodes(map, build_opts)?;
        let stream = gl.to_extended_lump_bytes(map.vertices().len(), build_opts.format)?;
        (stream, gl_ws)
    } else if build_opts.format.is_extended() {
        let (nodes, node_ws) = build_nodes(map, build_opts)?;
        let stream =
            nodes.to_extended_lump_bytes(map.vertices().len(), build_opts.format.compressed())?;
        (stream, node_ws)
    } else {
        return Err(NodeBuildError::UnsupportedNodeFormat {
            format: build_opts.format,
        });
    };

    // Deterministic warning order: UDMF write-path warnings first, then the
    // node-build warnings (write-then-build, matching the Doom one-shot).
    let mut warnings: Vec<NodeBuildWarning> = udmf_ws
        .into_iter()
        .map(NodeBuildWarning::UdmfWrite)
        .collect();
    warnings.extend(build_ws);

    builder.add_lump(name, b"");
    builder.add_lump("TEXTMAP", text.into_bytes());
    builder.add_lump("ZNODES", znodes);
    builder.add_lump("ENDMAP", b"");
    Ok(warnings)
}
