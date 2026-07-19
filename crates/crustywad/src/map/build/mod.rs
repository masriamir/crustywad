//! Clean-room node-lump builders: BLOCKMAP and REJECT today, the classic
//! BSP pass later (#315), generated from an assembled [`Map`]. A map is
//! engine-playable on vanilla only once the BSP stage also lands.
//!
//! This module is gated behind the `nodebuild` feature (which enables `write`).
//! It fulfills ADR-0019 §4's revisit condition — `add_doom_map` emits
//! zero-length node lumps with an always-on
//! [`DoomWriteWarning::NodesNotBuilt`](crate::map::DoomWriteWarning::NodesNotBuilt)
//! — by building those lumps for real (ADR-0024).
//!
//! # Staging
//!
//! Per ADR-0024 §9 this arrives in stages. Stage 1 (this module, ADR-0024
//! §9.1) ships both the BLOCKMAP builder
//! ([`build_blockmap`](crate::map::build::build_blockmap)) and the REJECT
//! builder ([`build_reject`](crate::map::build::build_reject)); the classic
//! BSP pass follows in stage 2 (issue #315). Every builder narrows
//! coordinates through the same pass as the write path (ADR-0024 §3) so the
//! geometry it operates on is exactly the `i16` values the engine reads.
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
mod reject;

pub use blockmap::build_blockmap;
pub use reject::build_reject;

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
        }
    }

    /// Lenient building — recoverable overflows produce [`NodeBuildWarning`]s
    /// rather than errors, and the output is still emitted.
    #[must_use]
    pub const fn lenient() -> Self {
        Self {
            strictness: Strictness::Lenient,
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
}
