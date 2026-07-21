//! Detects whether the mdBook guide sources are present in this checkout, so the
//! `guide-doctests` harness (`src/guide_doctests.rs`) can gate on their
//! existence.
//!
//! The harness `include_str!`s repo-level `docs/guide/src/*.md` files that are
//! **not** shipped inside the published crate. Gating the harness on
//! `cfg(has_guide_sources)` — set only when those files exist — makes enabling
//! the `guide-doctests` feature outside the source workspace (e.g. a plain
//! `cargo test --all-features` on the packaged crate) a graceful no-op instead
//! of an `include_str!` compile error.

use std::path::Path;

fn main() {
    // Declare the cfg unconditionally so `unexpected_cfgs` stays quiet whether or
    // not it ends up set.
    println!("cargo:rustc-check-cfg=cfg(has_guide_sources)");

    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is always set");
    // The guide lives at the repo root (`docs/guide/src/`), two levels up from
    // this crate. Probe one representative page.
    let probe = Path::new(&manifest).join("../../docs/guide/src/getting-started.md");
    println!("cargo:rerun-if-changed={}", probe.display());
    if probe.exists() {
        println!("cargo:rustc-cfg=has_guide_sources");
    }
}
