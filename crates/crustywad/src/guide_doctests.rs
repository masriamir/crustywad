//! Compile-checks the mdBook user guide's Rust code samples.
//!
//! Each `#[doc = include_str!(...)]` pulls a guide page into the crate **only**
//! under the doctest build, and only when the `guide-doctests` feature and the
//! build-script `has_guide_sources` cfg are both set (both described below) —
//! so `cargo test --doc --all-features` **compiles** every ` ```rust ` block the
//! guide presents as real code — and **runs** those not marked `no_run` — with
//! the crate fully linked and every feature enabled, while `cargo doc` and
//! normal builds never see these items. (` ```rust,ignore ` blocks are neither
//! compiled nor run; ` ```rust,no_run ` are compiled but not executed.) This is
//! the single source of truth: the guide's snippets live in the Markdown and
//! are checked here, so API drift breaks CI
//! instead of slipping through hand-verification.
//!
//! `mdbook test` cannot do this: it can only pass `-L` to rustdoc, never
//! `--extern`, so it cannot link `crustywad`, and it has no way to enable Cargo
//! features (`write`, `nodebuild`, `doom64-gfx`). Compiling the pages as
//! crate doctests under `--all-features` sidesteps both limitations.
//!
//! The module is gated `cfg(all(doctest, feature = "guide-doctests",
//! has_guide_sources))`. The feature is **off by default**, and `build.rs` sets
//! `has_guide_sources` only when the repo-level `docs/guide/src/` files (which
//! the `include_str!`s reach) actually exist — so enabling `guide-doctests`
//! outside the source workspace, e.g. a `cargo test --all-features` on the
//! *published* crate, is a graceful no-op instead of a missing-file compile
//! error. In this workspace both hold, so CI's existing
//! `cargo test --workspace --all-features` (the `test` job) runs these — no
//! dedicated job is needed; `just guide-test` does the same locally. Keep the
//! crate's doctests enabled (do not set `[lib] doctest = false`), or this check
//! silently stops running.
//!
//! Blocks that cannot compile as-is are marked in the Markdown: struct-layout
//! illustrations use ` ```rust,ignore `, and snippets that do real file I/O use
//! ` ```rust,no_run ` (compiled, not executed).
//!
//! Only pages that contain ` ```rust ` blocks are listed here. Pages with no
//! Rust samples (for example `performance.md`, `building-nodes.md`, `cli.md`)
//! are intentionally absent.

#[doc = include_str!("../../../docs/guide/src/getting-started.md")]
struct GettingStarted;

#[doc = include_str!("../../../docs/guide/src/reading-wads.md")]
struct ReadingWads;

#[doc = include_str!("../../../docs/guide/src/writing-wads.md")]
struct WritingWads;

#[doc = include_str!("../../../docs/guide/src/converting-maps.md")]
struct ConvertingMaps;

#[doc = include_str!("../../../docs/guide/src/features.md")]
struct Features;

#[doc = include_str!("../../../docs/guide/src/map-records.md")]
struct MapRecords;

#[doc = include_str!("../../../docs/guide/src/graphics.md")]
struct Graphics;
