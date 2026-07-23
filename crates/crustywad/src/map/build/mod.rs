//! Clean-room node-lump builders: BLOCKMAP, REJECT, and the classic BSP pass
//! (`SEGS`/`SSECTORS`/`NODES`), generated from an assembled [`Map`] — together
//! the full set of node lumps a vanilla engine needs to run a converted map.
//!
//! This module is gated behind the `nodebuild` feature (which enables `write`).
//! It fulfills ADR-0019 §4's revisit condition — `add_doom_map` emits
//! zero-length node lumps with an always-on
//! [`DoomWriteWarning::NodesNotBuilt`](crate::map::DoomWriteWarning::NodesNotBuilt)
//! — by building those lumps for real (ADR-0024).
//!
//! # Builders
//!
//! The BLOCKMAP builder
//! ([`build_blockmap`](crate::map::build::build_blockmap)), the REJECT builder
//! ([`build_reject`](crate::map::build::build_reject)), and the classic BSP pass
//! ([`build_nodes`](crate::map::build::build_nodes), ADR-0024 §9.2) all narrow
//! coordinates through the same pass as the write path (ADR-0024 §3) so the
//! geometry each operates on is exactly the `i16` values the engine reads.
//! `build_nodes` is validated against the full retail collection (551 classic
//! maps); its one accepted soft defect — the mixed-sector fan — is a lenient
//! [`NodeBuildWarning::MixedSectorSubsector`](crate::map::build::NodeBuildWarning::MixedSectorSubsector)
//! the retail masters ship too (ADR-0024 §7 amendment).
//!
//! [`add_doom_map_with_nodes`](crate::map::build::add_doom_map_with_nodes)
//! bundles all three into one call (ADR-0024 §9.3): it writes the map's data
//! lumps plus the built `SEGS`/`SSECTORS`/`NODES`/`REJECT`/`BLOCKMAP` into a
//! [`WadBuilder`](crate::WadBuilder), producing **engine-playable** output —
//! unlike [`add_doom_map`](crate::map::add_doom_map), it never emits
//! [`DoomWriteWarning::NodesNotBuilt`](crate::map::DoomWriteWarning::NodesNotBuilt).
//!
//! # Errors and warnings
//!
//! Narrowing failures surface as
//! [`NodeBuildError::Write`](crate::map::build::NodeBuildError) and narrowing
//! recoveries as [`NodeBuildWarning::Write`](crate::map::build::NodeBuildWarning)
//! — the write path's decision table (ADR-0019 §3), reused rather than
//! restated.
//! [`DoomWriteWarning::NodesNotBuilt`](crate::map::DoomWriteWarning::NodesNotBuilt)
//! never appears among build warnings: it describes the write path's
//! empty-lump output, not a property of what these builders produce.
//!
//! [`Map`]: crate::map::Map

use crate::Strictness;
use crate::map::DoomWriteError;
use crate::map::doom::DoomWriteWarning;

mod blockmap;
mod nodes;
mod oneshot;
mod reject;

pub use blockmap::build_blockmap;
pub use nodes::{BuiltNodeLumps, BuiltNodes, build_nodes};
pub use oneshot::add_doom_map_with_nodes;
pub use reject::build_reject;

/// Default [`NodeBuildOptions::split_cost`] (ADR-0024 §B.3): the observed
/// nodebuilder-standard weight of one straddling split.
const DEFAULT_SPLIT_COST: u32 = 8;
/// Default [`NodeBuildOptions::aa_preference`] (ADR-0024 §B.3): the observed
/// nodebuilder-standard axis-aligned preference divisor.
const DEFAULT_AA_PREFERENCE: u32 = 16;

/// The on-disk node format `build_nodes` targets (ADR-0025, #323).
///
/// `Classic` (the default) emits the vanilla `SEGS`/`SSECTORS`/`NODES` lumps
/// with the 16-bit index ceilings. The extended formats emit a single `ZDoom`
/// non-GL node stream — `Xnod`, or its zlib twin `Znod` — that widens the
/// subsector/node/seg/vertex counts to 32 bits, letting a past-vanilla map
/// serialize. The seg `linedef` reference stays 16-bit in every format, so a map
/// with more than 65,534 linedefs is unrepresentable here (that needs the GL
/// `XGL2` format, tracked in #345).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum NodeFormat {
    /// Vanilla `SEGS`/`SSECTORS`/`NODES` with 16-bit indices (the default).
    #[default]
    Classic,
    /// The uncompressed `ZDoom` non-GL extended stream (`XNOD`).
    Xnod,
    /// The zlib-compressed non-GL extended stream (`ZNOD`); requires the
    /// `extended-nodes-zlib` feature.
    #[cfg(feature = "extended-nodes-zlib")]
    Znod,
}

impl NodeFormat {
    /// Whether this is an extended (non-`Classic`) format, i.e. one that uses the
    /// 32-bit `MAX_EXTENDED_INDEX` ceilings rather than the vanilla 16-bit ones.
    #[must_use]
    pub(crate) fn is_extended(self) -> bool {
        !matches!(self, NodeFormat::Classic)
    }

    /// Whether this format's stream is zlib-compressed (`Znod`).
    #[must_use]
    pub(crate) fn compressed(self) -> bool {
        #[cfg(feature = "extended-nodes-zlib")]
        {
            matches!(self, NodeFormat::Znod)
        }
        #[cfg(not(feature = "extended-nodes-zlib"))]
        {
            false
        }
    }
}

/// Options controlling node-lump building (ADR-0024 §5).
///
/// Mirrors [`WriteOptions`](crate::WriteOptions): a strict build rejects any
/// content that would not fit the vanilla engine; a lenient build emits what it
/// can and reports the overflow as a [`NodeBuildWarning`].
///
/// # Examples
///
/// ```
/// use crustywad::map::build::NodeBuildOptions;
///
/// let strict = NodeBuildOptions::strict();
/// let lenient = NodeBuildOptions::lenient();
/// ```
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct NodeBuildOptions {
    /// Whether to build strictly (errors on any format/vanilla overflow) or
    /// leniently (recovers where the format allows, warning instead).
    pub strictness: Strictness,
    /// Partition-heuristic weight per straddling seg a candidate would split
    /// (ADR-0024 §B.3). A candidate's score is
    /// `split_cost * n_split + |n_front - n_back|` (plus a diagonal penalty),
    /// and lower scores win, so a higher `split_cost` favors partitions that
    /// cut fewer segs — fewer split vertices and smaller `SEGS`/`VERTEXES`
    /// lumps, at the cost of a less balanced tree. Defaults to `8`. `0` is
    /// legal and turns scoring into balance-only (`|n_front - n_back|`).
    pub split_cost: u32,
    /// Partition-heuristic divisor rewarding axis-aligned partitions
    /// (ADR-0024 §B.3). A diagonal candidate (`dx != 0 && dy != 0`) has
    /// `(n_front + n_back + n_split) / aa_preference` added to its score, so a
    /// *larger* value applies a *weaker* penalty (a larger divisor). Defaults
    /// to `16`. `0` is treated as "no diagonal penalty" — the build path guards
    /// the division and skips the term entirely rather than dividing by zero.
    pub aa_preference: u32,
    /// The on-disk node format to target (ADR-0025, #323). Defaults to
    /// [`NodeFormat::Classic`]. Set to [`NodeFormat::Xnod`] (or, with the
    /// `extended-nodes-zlib` feature, [`NodeFormat::Znod`]) to emit a `ZDoom`
    /// non-GL extended stream that lifts the vanilla 16-bit node ceilings.
    pub format: NodeFormat,
}

impl Default for NodeBuildOptions {
    fn default() -> Self {
        Self::strict()
    }
}

impl NodeBuildOptions {
    /// Strict building — any arena or offset overflow is an error.
    ///
    /// This is the default.
    #[must_use]
    pub const fn strict() -> Self {
        Self {
            strictness: Strictness::Strict,
            split_cost: DEFAULT_SPLIT_COST,
            aa_preference: DEFAULT_AA_PREFERENCE,
            format: NodeFormat::Classic,
        }
    }

    /// Lenient building — recoverable overflows produce [`NodeBuildWarning`]s
    /// rather than errors, and the output is still emitted.
    #[must_use]
    pub const fn lenient() -> Self {
        Self {
            strictness: Strictness::Lenient,
            split_cost: DEFAULT_SPLIT_COST,
            aa_preference: DEFAULT_AA_PREFERENCE,
            format: NodeFormat::Classic,
        }
    }
}

/// An error that prevents building a map's node lumps (ADR-0024 §5).
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum NodeBuildError {
    /// A narrowing failure from the shared write-path pass (ADR-0024 §3): a
    /// coordinate or index could not be narrowed to the Doom `i16`/`u16`
    /// on-disk field in strict mode. Carries the underlying
    /// [`DoomWriteError`].
    #[error(transparent)]
    Write(#[from] DoomWriteError),
    /// The map has no geometry to build from — zero vertices, linedefs,
    /// sidedefs, or sectors. Vanilla requires at least one subsector, so there
    /// is nothing to emit. Returned in **both** strictness modes
    /// (`build_blockmap` and the BSP pass; [`build_reject`] is infallible and
    /// simply yields an empty table).
    #[error(
        "map has no geometry to build nodes from (needs vertices, linedefs, sidedefs, sectors)"
    )]
    EmptyGeometry,
    /// A `BLOCKMAP` word offset exceeds what the format can encode: a
    /// blocklist starting past 65,535 words (or — in strict mode — past the
    /// vanilla signed-offset ceiling of 32,767; ADR-0024 §5), or a stored
    /// word whose value cannot be serialized as `u16`. The offending word
    /// offset from the lump start is reported.
    #[error("blockmap word offset {offset} exceeds the addressable range")]
    BlockmapOverflow {
        /// The offending word offset, counted from the lump start.
        offset: usize,
    },
    /// A BSP arena grew past what the on-disk format can index (ADR-0024 §5,
    /// Global Constraint 6). Returned in **both** strictness modes when the
    /// count is *structurally* unrepresentable: more than 65,536 vertices (map
    /// plus split) or segs — the `u16` index ceiling — or more than 32,768
    /// subsectors or nodes, because a BSP child reference reserves bit 15 as
    /// the `NF_SUBSECTOR` leaf flag, so those indices must fit 15 bits. The
    /// distinct vanilla-only 32,768 vertex/seg *soft* ceiling is instead a
    /// strict-mode error paired with a lenient
    /// [`NodeBuildWarning::VanillaCeilingExceeded`].
    #[error("{kind} count {count} exceeds the node-build maximum of {max}")]
    TooManyElements {
        /// The arena name (e.g. `"subsectors"`).
        kind: &'static str,
        /// The actual element count.
        count: usize,
        /// The maximum the format can index.
        max: usize,
    },
    /// A seg had no backing linedef — a GL miniseg (`MapSeg::linedef == None`,
    /// introduced with extended-node reading, #326). The classic on-disk `SEGS`
    /// format cannot represent a linedef-less seg, so serializing a `BuiltNodes`
    /// containing one fails cleanly (both modes) rather than panicking. The
    /// in-tree node builder never produces minisegs; this guards a `BuiltNodes`
    /// hand-constructed via its public fields.
    #[error("seg {seg} is a GL miniseg (no linedef); the classic SEGS format cannot represent it")]
    MinisegUnsupported {
        /// The index of the offending seg in the `BuiltNodes` seg arena.
        seg: usize,
    },
    /// A convex subsector spans multiple sectors and no seg line separates them
    /// (ADR-0024 §C, §7 amendment 2026-07-19). The engine would render such a
    /// subsector with the first seg's sector — the wrong flats — so strict mode
    /// rejects it, naming the map that cannot yield a single-sector tree.
    /// Lenient mode instead accepts the subsector and emits
    /// [`NodeBuildWarning::MixedSectorSubsector`] once per such leaf — the same
    /// engine-tolerated output the retail masters ship (30 shipped maps carry 47
    /// such subsectors; the fan cannot be split without synthesizing a non-seg
    /// partition line, which the ADR rejects as gilding past parity).
    #[error(
        "a convex subsector of {subsector_segs} segs spans multiple sectors with no separating partition"
    )]
    MixedSectorSubsector {
        /// The number of segs in the offending convex subsector.
        subsector_segs: usize,
    },
    /// A partition line chosen by the selector (ADR-0024 §B) failed to separate
    /// its seg set into two non-empty sides. `select` only returns a candidate
    /// whose classification places content on both sides, so this is an
    /// internal invariant — but the endpoint-coincidence split fallback
    /// (ADR-0024 §C.3, which routes a straddling seg whole to one side when its
    /// rounded intersection lands on an endpoint) can, for adversarial
    /// geometry, collapse every straddling seg onto a single side. Rather than
    /// emit a degenerate node or risk non-termination, the build fails cleanly
    /// in **both** strictness modes (a denial-of-service hardening guard on a
    /// fuzzed-input path, ADR-0016). Well-formed geometry never trips it.
    #[error("selected partition of {set_segs} segs did not separate them into two non-empty sides")]
    DegeneratePartition {
        /// The number of segs in the set that the selected line failed to
        /// partition.
        set_segs: usize,
    },
    /// [`NodeFormat::Znod`] output was requested (`compressed`) but the crate was
    /// built without the `extended-nodes-zlib` feature, so no zlib compressor is
    /// available. Reachable only through
    /// [`BuiltNodes::to_extended_lump_bytes`](crate::map::build::BuiltNodes::to_extended_lump_bytes)
    /// with `compressed = true` and the feature off — the `NodeFormat::Znod`
    /// variant that drives it does not exist without the feature.
    #[error("zlib compression requires the `extended-nodes-zlib` feature")]
    CompressionUnavailable,
}

impl NodeBuildError {
    /// Whether re-running the build with [`NodeBuildOptions::lenient`] recovers
    /// from this error, turning it into a [`NodeBuildWarning`]-carrying success.
    ///
    /// Mirrors [`DoomWriteError::is_lenient_recoverable`]. The CLI uses this to
    /// decide whether suggesting `--lenient` after a strict-mode refusal would
    /// be honest (#264): the hint appears only when lenient mode would actually
    /// accept the input.
    ///
    /// Returns:
    ///
    /// - `true` for [`MixedSectorSubsector`][Self::MixedSectorSubsector], which
    ///   strict mode rejects but lenient mode accepts with a
    ///   [`NodeBuildWarning::MixedSectorSubsector`].
    /// - The wrapped error's own classification for
    ///   [`Write`][Self::Write] — the shared write-path pass decides.
    /// - `false` for the errors produced identically in **both** strictness
    ///   modes — [`EmptyGeometry`][Self::EmptyGeometry],
    ///   [`DegeneratePartition`][Self::DegeneratePartition],
    ///   [`MinisegUnsupported`][Self::MinisegUnsupported] — and for the
    ///   arena/offset ceilings ([`TooManyElements`][Self::TooManyElements],
    ///   [`BlockmapOverflow`][Self::BlockmapOverflow]): each conflates a
    ///   dominant structurally unrepresentable case (both modes error) with a
    ///   strict-only vanilla-ceiling subset that lenient recovers. Classifying
    ///   the whole variant `false` never yields a dishonest `--lenient` hint
    ///   (a false positive, the #264 anti-pattern); at worst it omits the hint
    ///   for the rare vanilla-ceiling subset, matching how
    ///   [`DoomWriteError::TooManyElements`] classifies `false`.
    #[must_use]
    pub fn is_lenient_recoverable(&self) -> bool {
        match self {
            Self::Write(inner) => inner.is_lenient_recoverable(),
            Self::MixedSectorSubsector { .. } => true,
            Self::EmptyGeometry
            | Self::BlockmapOverflow { .. }
            | Self::TooManyElements { .. }
            | Self::MinisegUnsupported { .. }
            | Self::DegeneratePartition { .. }
            | Self::CompressionUnavailable => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lenient_recoverable_classification() {
        // Strict-only: lenient accepts the mixed-sector fan with a warning.
        assert!(
            NodeBuildError::MixedSectorSubsector { subsector_segs: 3 }.is_lenient_recoverable()
        );

        // Both-modes structural / hardening errors: lenient does not recover.
        assert!(!NodeBuildError::EmptyGeometry.is_lenient_recoverable());
        assert!(!NodeBuildError::BlockmapOverflow { offset: 70_000 }.is_lenient_recoverable());
        assert!(
            !NodeBuildError::TooManyElements {
                kind: "segs",
                count: 70_000,
                max: 65_536,
            }
            .is_lenient_recoverable()
        );
        assert!(!NodeBuildError::DegeneratePartition { set_segs: 4 }.is_lenient_recoverable());
        assert!(!NodeBuildError::MinisegUnsupported { seg: 0 }.is_lenient_recoverable());

        // Write delegates to the wrapped write error's own classification.
        assert!(
            NodeBuildError::Write(DoomWriteError::ValueOutOfRange {
                block: "vertex",
                field: "x",
                index: 0,
                value: 40_000,
            })
            .is_lenient_recoverable()
        );
        assert!(
            !NodeBuildError::Write(DoomWriteError::TooManyElements {
                kind: "vertices",
                count: 70_000,
                max: 65_536,
            })
            .is_lenient_recoverable()
        );
    }

    #[test]
    fn node_build_options_default_to_classic_format() {
        assert_eq!(NodeBuildOptions::default().format, NodeFormat::Classic);
        assert_eq!(NodeBuildOptions::strict().format, NodeFormat::Classic);
        assert_eq!(NodeBuildOptions::lenient().format, NodeFormat::Classic);
        assert_eq!(NodeFormat::default(), NodeFormat::Classic);
        assert!(!NodeFormat::Classic.is_extended());
        assert!(NodeFormat::Xnod.is_extended());
        assert!(!NodeFormat::Xnod.compressed());
    }
}

/// A non-fatal condition recovered while building a map's node lumps in lenient
/// mode (ADR-0024 §5).
///
/// The crate's warning enums derive [`thiserror::Error`] for their `Display`;
/// this one follows suit (compare
/// [`ParseWarning`](crate::ParseWarning) and
/// [`DoomWriteWarning`]).
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum NodeBuildWarning {
    /// A narrowing recovery from the shared write-path pass (ADR-0024 §3): a
    /// coordinate or value was rounded, clamped, truncated, or dropped in
    /// lenient mode. Carries the underlying [`DoomWriteWarning`] — never
    /// [`DoomWriteWarning::NodesNotBuilt`], which is a property of the
    /// *empty*-node write path, not of a real build.
    #[error(transparent)]
    Write(DoomWriteWarning),
    /// Lenient mode: a packed `BLOCKMAP` blocklist starts past the vanilla
    /// signed-offset ceiling (> 32,767) but still fits an unsigned 16-bit word.
    /// The lump was emitted; a limit-removing port is needed to read it
    /// (ADR-0024 §5).
    #[error("blockmap blocklist offset {offset} exceeds the vanilla signed-offset ceiling")]
    BlockmapVanillaOverflow {
        /// The offending blocklist-start word offset, counted from the lump
        /// start.
        offset: usize,
    },
    /// Lenient mode: a BSP arena exceeded the vanilla-only 32,768 vertex/seg
    /// *soft* ceiling but still fits the format's 16-bit indices (ADR-0024
    /// §5). The lumps were emitted; a limit-removing port is needed to read
    /// them. This is distinct from the structural
    /// [`NodeBuildError::TooManyElements`] ceilings, which no engine can read.
    #[error("{kind} count {count} exceeds the vanilla ceiling of {max}")]
    VanillaCeilingExceeded {
        /// The arena name (`"vertices"` or `"segs"`).
        kind: &'static str,
        /// The actual element count.
        count: usize,
        /// The vanilla ceiling (32,768).
        max: usize,
    },
    /// Lenient mode: a convex subsector spans multiple sectors with no seg line
    /// that separates them (ADR-0024 §C, §7 amendment 2026-07-19). It was
    /// accepted as a single subsector; the engine renders it with the first
    /// seg's sector (the wrong flats on a micro-sliver — a soft-contract defect
    /// the retail masters themselves ship). Emitted **once per such leaf**. The
    /// strict-mode counterpart is [`NodeBuildError::MixedSectorSubsector`].
    #[error(
        "a convex subsector of {subsector_segs} segs spans multiple sectors with no separating partition; rendered with the first seg's sector"
    )]
    MixedSectorSubsector {
        /// The number of segs in the accepted mixed-sector subsector.
        subsector_segs: usize,
    },
}
