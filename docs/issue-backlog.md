# crustywad issue backlog

This document is the planning source-of-truth and **backup** for the GitHub
issue tree generated from [`todo.md`](../todo.md). It captures every epic,
sub-issue, spike, label, milestone, dependency, and code reference so the plan
survives even if automated issue creation is incomplete.

**Legend:** 🗂️ epic · 🔬 spike (ADR-gated) · ⛓️ blocked-by dependency ·
👤 assignee `@masriamir`

> **Process rule:** No implementation sub-issue may start until its sibling 🔬
> spike has landed an **accepted ADR** under [`docs/adr/`](adr/). Spikes
> therefore *block* their implementation siblings.

## Labels

| Label | Purpose | Status |
| --- | --- | --- |
| `enhancement` | general feature work | existing |
| `documentation` | docs-only work | existing |
| `epic` | tracking epic | create |
| `spike` | time-boxed research → ADR | create |
| `cli` | `cwad` CLI work | create |
| `testing` | tests, coverage, fuzzing | create |
| `release` | packaging/CI/CD/release | create |
| `format-support` | new WAD/map formats | create |
| `performance` | benchmarking/optimization | create |
| `future` | long-horizon/aspirational | create |

## Milestones

| Milestone | Scope |
| --- | --- |
| `v1.0` | Feature-complete first release |
| `Short Term` | Post-1.0 near-term goals |
| `Future` | Aspirational/long-horizon |

---

## 🗂️ Epic #12 — Add support for writing `.wad` files (`v1.0`)

Labels: `epic`, `enhancement` · Milestone: `v1.0`

- [ ] 🔬 **Spike: WAD write design & ADR** — in-place modification vs. full
  rebuild, offset/directory recomputation, and a buffer/writer abstraction.
  Deliverable: accepted ADR. `spike` — **blocks** every sibling below.
  Refs: `crates/crustywad/src/lib.rs` (`WadData` L141-156, `into_bytes`
  L405-411, `parse_bytes` L414-510), `docs/design.md` (milestone 6, L47).
- [ ] Implement header + lump-directory serialization. `enhancement` ·
  ⛓️ spike. Refs: `lib.rs` `RawHeader` L186-192, `RawDirectoryEntry` L194-200.
- [ ] Lump payload serialization + recompute offsets/sizes. `enhancement` ·
  ⛓️ spike. Refs: `lib.rs` `Lump` L113-139, `lump_bytes`/`lump_data` L366-391.
- [ ] Strict/lenient validation on write. `enhancement` · ⛓️ spike.
  Refs: `lib.rs` `Strictness` L57-64, `validate_entry` L554-622.
- [ ] Round-trip read→write→read invariants (property tests). `testing` ·
  ⛓️ spike. Refs: `crates/crustywad/tests/wad_reader.rs`.
- [ ] CLI write wiring (expose writing to `cwad`). `cli` · ⛓️ spike.
  Refs: `crates/crustywad-cli/src/main.rs` L22-70.
- [ ] Exhaustive test coverage for all new write paths. `testing` · ⛓️ spike.
- [ ] Unblock & link write-benchmark stubs (#5, #6) once write support lands.
  `performance` · ⛓️ spike.

## 🗂️ Epic #13 — Documentation, API reference & contributor experience (`v1.0`)

Labels: `epic`, `documentation`, `enhancement` · Milestone: `v1.0`

- [ ] API reference coverage for `lib.rs` and the full public surface
  (`#![deny(missing_docs)]` already enforced). `documentation`.
  Refs: `crates/crustywad/src/lib.rs`, `Cargo.toml` L23-24.
- [ ] CLI usage documentation: subcommands, options, examples, man page.
  `documentation`, `cli`. Refs: `crates/crustywad-cli/src/main.rs`.
- [ ] Architecture & data-flow diagrams (mermaid in `docs/`). `documentation`.
  Refs: `docs/design.md`.
- [ ] User guide & examples published via GitHub Pages. `documentation`.
- [ ] 🔬 **Spike: living-docs automation** — keep `.github/copilot-instructions.md`,
  `CLAUDE.md`, and `docs/` in sync automatically. Deliverable: ADR. `spike`,
  `documentation` — **blocks** the living-docs task below.
  Refs: `.github/copilot-instructions.md`.
- [ ] Keep all `docs/` current as living documents (implements the spike).
  `documentation` · ⛓️ living-docs spike.

## 🗂️ Epic #14 — CLI feature expansion & tool UX (`v1.0`)

Labels: `epic`, `cli`, `enhancement` · Milestone: `v1.0`

- [ ] 🔬 **Spike: CLI UX & architecture** — subcommand structure, arg parsing,
  output formats, OS packaging implications. Deliverable: ADR. `spike`, `cli`
  — **blocks** every command below. Refs: `crates/crustywad-cli/src/main.rs`.
- [ ] `cwad validate` — check WAD correctness. `cli` · ⛓️ spike.
- [ ] `cwad info` — display WAD metadata (expand existing command). `cli` ·
  ⛓️ spike. Refs: `main.rs` `Command::Info` L24-28, L44-52.
- [ ] `cwad diff` — compare two WAD files. `cli` · ⛓️ spike.
- [ ] `cwad merge` — combine multiple WADs. `cli` · ⛓️ spike.
- [ ] `cwad extract` — extract specific lumps. `cli` · ⛓️ spike.
  Refs: `lib.rs` `lump_bytes` L366-371.
- [ ] CLI hardening: comprehensive test suite + invalid/edge-case regressions.
  `cli`, `testing` · ⛓️ spike.

## 🗂️ Epic #15 — Test coverage, fuzzing & integration testing (`v1.0`)

Labels: `epic`, `testing`, `enhancement` · Milestone: `v1.0`

- [ ] Raise measured coverage to ≥90% (gate with `cargo-llvm-cov`). `testing`.
  Refs: `justfile` `cov` L17-18.
- [ ] Add/expand malformed & large WAD corpus. `testing`.
  Refs: `crates/crustywad/tests/common/mod.rs`.
- [ ] End-to-end read→modify→write integration tests. `testing` ·
  ⛓️ **blocked by Epic #12 (write support)**.
- [ ] 🔬 **Spike: `cargo-fuzz` harness** design. Deliverable: ADR. `spike`,
  `testing` — **blocks** the fuzz implementation below.
  Refs: `justfile` `fuzz` L27-28.
- [ ] Implement `cargo-fuzz` targets. `testing` · ⛓️ fuzz spike.
- [ ] 🔬 **Spike: `proptest` strategy** for parser invariants. Deliverable: ADR.
  `spike`, `testing` — **blocks** the proptest implementation below.
- [ ] Implement `proptest` invariant/property tests. `testing` · ⛓️ proptest
  spike. Refs: `crates/crustywad/src/map.rs` `parse_records` L196-225.

## 🗂️ Epic #16 — Release engineering: crates.io, CI/CD, docs.rs (`v1.0`)

Labels: `epic`, `release`, `enhancement` · Milestone: `v1.0`

- [ ] 🔬 **Spike: publish workflow** — workspace publish ordering, changelog
  generation, version strategy. Deliverable: ADR. `spike`, `release` —
  **blocks** every sibling below. Refs: `.github/workflows/release-plz.yml`,
  `Cargo.toml` L1-29.
- [ ] docs.rs config (all features, cross-crate linking, badges). `release`,
  `documentation` · ⛓️ spike.
- [ ] Changelog automation. `release` · ⛓️ spike.
- [ ] SemVer, MSRV & release-policy document. `release`, `documentation` ·
  ⛓️ spike. Refs: `Cargo.toml` `rust-version` L8.
- [ ] Cross-platform binary/CLI release artifacts. `release` · ⛓️ spike.
  Refs: `.github/workflows/ci.yml`.

## 🗂️ Epic #17 — Multi-format & map-format support: Doom 64, Hexen, Heretic, UDMF (`Short Term`)

Labels: `epic`, `format-support`, `enhancement` · Milestone: `Short Term`

- [ ] 🔬 **Spike: format-landscape review** — collect specs, review compat,
  propose an extension strategy. Deliverable: ADR. `spike`, `format-support` —
  **blocks** Doom 64 / Hexen / Heretic below.
  Refs: `crates/crustywad/src/lib.rs` `WadKind` L46-55.
- [ ] Doom 64 format support. `format-support` · ⛓️ format spike.
- [ ] Hexen format support. `format-support` · ⛓️ format spike.
  Refs: `crates/crustywad/src/map.rs` `Thing` L33-46 (Hexen extends THINGS).
- [ ] Heretic format support. `format-support` · ⛓️ format spike.
- [ ] 🔬 **Spike: UDMF design/representation**. Deliverable: ADR. `spike`,
  `format-support` — **blocks** UDMF read/convert/write below.
- [ ] UDMF read support. `format-support` · ⛓️ UDMF spike.
- [ ] UDMF ↔ classic WAD conversion. `format-support` · ⛓️ UDMF spike.
- [ ] UDMF write support. `format-support` · ⛓️ UDMF spike,
  ⛓️ **blocked by Epic #12 (write support)**.

## 🗂️ Epic #18 — Future: GUI/web WAD editor, visualizer & version control (`Future`)

Labels: `epic`, `future`, `enhancement` · Milestone: `Future`

- [ ] 🔬 **Spike: GUI framework evaluation** (egui, Tauri, Qt6, cross-platform).
  Deliverable: ADR. `spike`, `future` — **blocks** viewer/manager/preview.
- [ ] 🔬 **Spike: map-renderer design** (2D/3D/interactive). Deliverable: ADR.
  `spike`, `future` — **blocks** the visual map viewer.
- [ ] 🔬 **Spike: editor architecture + versioned WADs** (UDB-in-Rust, git-like
  version control). Deliverable: ADR. `spike`, `future` — **blocks** the
  resource manager and live preview.
- [ ] Visual map viewer core. `future` · ⛓️ GUI spike, ⛓️ renderer spike.
  Refs: `crates/crustywad/src/map.rs`.
- [ ] Lump/resource manager. `future` · ⛓️ GUI spike, ⛓️ editor spike.
- [ ] Live preview / test integration. `future` · ⛓️ GUI spike, ⛓️ editor spike.

## 🗂️ NEW Epic — Benchmarking & performance (`Short Term`)

Labels: `epic`, `performance`, `enhancement` · Milestone: `Short Term`

Groups the existing standalone benchmark issues and adds the perf-optimization
goal from `todo.md` (Short Term #3).

- [ ] (existing **#2**) Set up Criterion benchmarking infrastructure & reporting.
- [ ] (existing **#3**) Benchmark WAD parsing, lump access, map-record decoding.
- [ ] (existing **#4**) Benchmark & potentially optimize `lump_by_name` linear
  lookup. Refs: `crates/crustywad/src/lib.rs` `lump_by_name` L360-364.
- [ ] (existing **#5**) Stub write benchmarks & suppress from reporting.
  ⛓️ **blocked by Epic #12 (write support)**.
- [ ] (existing **#6**) Implement write benchmarks once write support lands.
  ⛓️ **blocked by Epic #12 (write support)**.
- [ ] NEW: Implement performance enhancements identified by benchmarking
  (parsing algorithm, memory usage). `performance`, `enhancement`.
  Refs: `crates/crustywad/src/lib.rs` `parse_bytes` L414-510, `justfile`
  `bench` L30-31.

---

## Cross-epic dependencies

- Epic #15 “read→modify→write” tests ⛓️ Epic #12.
- Epic #17 “UDMF write” ⛓️ Epic #12.
- Benchmarking #5 and #6 ⛓️ Epic #12.

## Notes

- All issues are assigned to `@masriamir`.
- Existing benchmark issues (#2–#6) keep their current bodies; they are adopted
  as sub-issues of the Benchmarking & performance epic. Consider setting their
  milestone to `Short Term`.
- Each 🔬 spike's concrete deliverable is an accepted ADR in `docs/adr/`
  (template: `docs/adr/0000-adr-template.md`).
