# 0005. Isolate unsafe code in a dedicated `crustywad-platform` workspace crate

- Status: proposed
- Date: 2026-06-06

## Context

The core `crustywad` library originally used `#![forbid(unsafe_code)]`, giving a
hard guarantee that no unsafe code could appear anywhere in the crate — not even
with a scoped `#[allow]`. When real memory-mapped file loading was introduced via
`memmap2`, that guarantee had to be weakened to `#![deny(unsafe_code)]` with a
module-level `#![allow(unsafe_code)]` in `mmap.rs`. The unsafe block is small
and justified (one call to `MmapOptions::map`), but the policy change means a
future contributor could introduce additional unsafe blocks elsewhere in the
library and have them compile as long as they add a matching `#[allow]`.

The standard Rust idiom for this situation — used by e.g. the standard library's
own collections and by crates like `bytes` — is to push `unsafe` into a thin
wrapper crate with a safe public API surface. The consumer crate then calls only
safe functions and can restore `#![forbid(unsafe_code)]`.

Beyond mmap, several other planned or plausible milestones involve operations
that would require `unsafe` in Rust today:

- **SIMD-accelerated decoding** — fast byte search and bulk decompression for
  graphics and audio lumps (stable `std::simd` is not yet complete; portable
  SIMD crates use unsafe internally, and hand-written SIMD needs unsafe
  directly).
- **C FFI / embedding API** — exposing `crustywad` to downstream C or C++ Doom
  engines via `cbindgen` requires `extern "C"` entry points and raw pointer
  handling.
- **Direct / unbuffered I/O** — using `O_DIRECT` or `FILE_FLAG_NO_BUFFERING` for
  large-WAD streaming scenarios would bypass the OS page cache at the cost of
  alignment requirements that need unsafe pointer arithmetic.
- **Custom arena allocation** — an optional arena-backed lump store would avoid
  heap fragmentation on WADs with thousands of small lumps and would require
  unsafe to implement the arena bump-pointer logic.

Each of these is a contained, well-defined unsafe surface. Grouping them all in
one workspace crate makes it easy to audit, scope, and review unsafe code without
touching the public library API.

## Decision

Add a new internal workspace crate, `crates/crustywad-platform/`, that hosts all
unsafe Rust in the project. The crate:

- Is not published to crates.io (set `publish = false` in its `Cargo.toml`).
- Exposes only safe public functions — all `unsafe` stays behind the module
  boundary.
- Is depended on by `crustywad` as an optional dependency, gated by the same
  feature flags as the capability it provides (e.g. `mmap = ["dep:crustywad-platform"]`).

The initial migration moves `mmap::open` from `crates/crustywad/src/mmap.rs`
into `crustywad-platform`, which then re-exports it as:

```rust
/// Opens `path` as a read-only memory-mapped file.
pub fn mmap_open(path: &Path) -> io::Result<memmap2::Mmap> { … }
```

With that in place, `crustywad/src/mmap.rs` becomes a trivial forwarding shim
(or is removed entirely), and `lib.rs` reverts to `#![forbid(unsafe_code)]`.

Future unsafe capabilities (SIMD helpers, FFI entry points, arena allocator,
direct I/O primitives) are added as additional modules inside
`crustywad-platform`, each with its own safe public API surface.

## Consequences

- `crustywad` regains `#![forbid(unsafe_code)]`, restoring the original
  hard guarantee and making the SECURITY.md and policy documentation accurate
  again without caveats.
- All unsafe code in the project lives in one crate. A single `cargo audit` or
  security review of `crustywad-platform` covers the entire unsafe surface.
- `crustywad-platform` carries no `#![forbid(unsafe_code)]` restriction. Each
  new unsafe addition should include a `// SAFETY:` comment and be reviewed as
  if it were public API — the safety of the whole stack depends on it.
- Adds one workspace crate for the initial work. The first iteration is a thin
  wrapper that mainly moves an existing `unsafe` block; the overhead is low.
- The `crustywad-platform` crate name signals "internal implementation detail"
  but is not self-evidently scoped to unsafe. A `// This crate is the unsafe
  boundary for the workspace` comment at the top of `lib.rs` should make the
  intent explicit for new contributors.
- If `std::simd` stabilises or a future Rust edition provides safe SIMD
  intrinsics, the SIMD modules in `crustywad-platform` can be migrated back to
  `crustywad` without any public API change.
