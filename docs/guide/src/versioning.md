# Versioning and Release Policy

This page documents the SemVer guarantees, MSRV policy, versioning model, and release
cadence for `crustywad` and `crustywad-cli`.

---

## Semantic Versioning

Both crates follow [Semantic Versioning 2.0.0](https://semver.org/), adapted to the pre-1.0
phase. While the crates are at `0.y.z` — which SemVer treats as explicitly unstable — this
project uses version increments as deliberate compatibility signals; a `0.y.z` version is not a
license to make arbitrary breaking changes in patches.

### Pre-1.0 version mapping (current)

Cargo reads a `0.MINOR` caret requirement (a `"0.8"` dependency means `^0.8`, i.e.
`>=0.8.0, <0.9.0`) as allowing patch updates but **not** a minor bump. So while at `0.y.z`, **the
minor bump is the breaking-change boundary** — the opposite of the post-1.0 intuition where a
minor release is a safe feature drop. To keep that boundary meaningful, the scheme collapses to
two levels until 1.0:

| Change | Bump while at 0.x | Does a `0.MINOR` caret pin auto-upgrade? |
|---|---|---|
| Breaking change (or MSRV raise) | **minor** — `0.8.0` → `0.9.0` | No — the consumer must opt in. |
| New backward-compatible API or feature | **patch** — `0.8.0` → `0.8.1` | Yes. |
| Bug fix | **patch** — `0.8.0` → `0.8.1` | Yes. |

Every backward-compatible change — new public types/functions/methods, new off-by-default feature
flags, *and* bug fixes — ships as a **patch**; the **minor** bump is reserved for breaking changes
(and MSRV raises, which break the build environment). This maximizes what a `0.MINOR` consumer
receives automatically while still giving them a hard signal — the minor bump — before anything
can break them. `release-plz` derives the bump from
[Conventional Commits](https://www.conventionalcommits.org/) accordingly: a `!` / `BREAKING CHANGE`
commit bumps the minor; every other releasable commit (`feat`, `fix`, …) bumps the patch.

**At `1.0.0` this expands to standard `MAJOR.MINOR.PATCH`**, and the per-channel meanings below
apply literally (backward-compatible new API → minor, breaking → major).

### Patch releases (`0.MINOR.PATCH`)

Canonically (at 1.0+), a patch release fixes a bug without changing any public API, and is safe
for all existing callers to upgrade without modification. **While at 0.x, a patch release carries
every backward-compatible change** — bug fixes *and* new additive API/features (per the pre-1.0
mapping above), since all of them are safe for a `0.MINOR` caret consumer to receive.

Examples of patch changes:

- Correcting incorrect byte offsets in a parser
- Fixing a panic or incorrect error variant in an existing code path
- Adding a new public type, function, or method _(0.x — a minor change at 1.0+)_
- Adding a new off-by-default feature flag, or a variant to a `#[non_exhaustive]` enum _(0.x)_
- Updating documentation without changing behavior
- Updating a dependency to a compatible patch version

### Minor releases (`0.MINOR.0`)

**While at 0.x, a minor bump signals a breaking change** — it is the boundary a `0.MINOR` caret
consumer must opt into (see the [pre-1.0 mapping](#pre-10-version-mapping-current) above and the
breaking-change list below). An **MSRV raise is also a minor bump**: a caller on an older compiler
can no longer build, so it is treated as a build-environment break (see
[MSRV policy](#msrv-policy)).

Canonically (at 1.0+), the minor channel instead carries backward-compatible new functionality —
adding a public type, function, or method; a new off-by-default feature flag; a new
`#[non_exhaustive]` enum variant. Until 1.0 those ship as **patches** (above); only breaking
changes and MSRV raises bump the minor.

### Major releases (`MAJOR.0.0`)

A major release contains at least one breaking change. Callers may need to update their
code after upgrading.

> **Pre-1.0 note:** While this crate is at `0.y.z`, there is no `1.0.0` to bump to.
> Breaking changes are instead signaled by a **minor** bump (e.g. `0.1.0` → `0.2.0`).
> The breaking-change examples below apply regardless of whether the release is `0.MINOR.0`
> or a future `MAJOR.0.0`.

Examples of breaking changes:

- Removing or renaming a public type, function, method, or field
- Changing a function signature (parameter types, return type, added required parameter)
- Adding a variant to an exhaustive enum
- Changing the behavior of an existing function in a way that violates the previous contract
- Changing a feature flag that is on by default
- Implementing a foreign trait (from `std` or a dependency) on an existing public type
  (may cause coherence conflicts in downstream code)

### What is not a breaking change

- Adding new public items (types, functions, methods)
- Adding new trait impls for traits defined in this crate
- Adding variants to enums marked `#[non_exhaustive]`
- Adding optional feature flags
- Internal implementation changes with identical observable behavior
- Updating dependencies to compatible versions (patch or minor per their own SemVer)

---

## MSRV Policy

The current minimum supported Rust version (MSRV) is **1.94.0**, set via `rust-version` in
`Cargo.toml`. The project targets the Rust 2024 edition.

Rules:

- **An MSRV bump is a minor version change**, never a patch. A caller pinned to the old
  compiler will fail to build after an MSRV bump, so it is treated as a backward-incompatible
  change to the build environment even though the public API is unchanged.
- **Rolling N-3 target.** The MSRV tracks a bounded window: at each release it is
  **(latest stable Rust minor at release time) − 3**, so the crates are guaranteed to build on
  the **last four stable Rust releases** (roughly the most recent six months). This replaces the
  former need-driven policy — the window makes the compatibility promise explicit rather than
  leaving it implicit, and keeps the toolchain modern enough for the current dependency
  ecosystem.
- **Revisited each release.** The MSRV is reviewed at every release and raised when the rolling
  window advances, or earlier when a required dependency or language feature demands a newer
  toolchain. Raising it stays a **minor** version bump (see the first rule); dropping support for
  releases below the new floor is the deliberate, semver-signaled cost of a bounded window.
- **CI enforces the declared MSRV.** The `msrv` job in CI builds and tests the workspace on
  the declared MSRV on every PR. The toolchain version is pinned explicitly in
  `.github/workflows/ci.yml` and does not auto-track `[workspace.package].rust-version`. A
  PR that raises the MSRV must update both the `rust-version` field in `Cargo.toml` and the
  `toolchain:` pin in the workflow file, then bump the version of each affected crate (a
  minor bump) — both crates currently share `rust-version.workspace = true`, so an MSRV
  bump affects both. If `crustywad`'s version moves outside `crustywad-cli`'s pinned caret
  range as a result, update that pin too.

---

## Versioning Model

### Independent per-crate versioning

Per [ADR-0011](https://github.com/masriamir/crustywad/blob/main/docs/adr/0011-publish-workflow.md),
each crate carries its own explicit `version` field in its `[package]` block rather than
inheriting from `[workspace.package]`. `release-plz` manages each package independently,
proposing version bumps only for crates whose content has changed since the last release.

**Dependency constraint:** `crates/crustywad-cli/Cargo.toml` pins the library with an explicit
caret requirement (currently `crustywad = { version = "0.9.0", ... }`), required by
`cargo-deny`'s `wildcards = "deny"` setting (which disallows `*` version requirements).
`version = "0.9.0"` resolves as `^0.9.0` (`>=0.9.0, <0.10.0`), so patch bumps to `crustywad`
within the same minor series are satisfied automatically. When `crustywad`'s version moves
outside that range (e.g., to `0.10.0`), this field must be updated manually before merging —
otherwise `cargo build` and crates.io publishing will fail.

---

## Release Cadence

Releases are automated by [release-plz](https://release-plz.dev/), which monitors
`main` for Conventional Commits and opens a release PR whenever releasable changes
accumulate.

The workflow:

1. Commits land on `main` via merged PRs, following the
   [Conventional Commits](https://www.conventionalcommits.org/) format (`feat:`, `fix:`,
   `docs:`, etc.).
2. `release-plz` inspects the commit history and proposes a release PR with a version bump
   and an updated `CHANGELOG.md`. Breaking changes must be marked (`feat!:` or a
   `BREAKING CHANGE:` footer) for the commit-derived bump to be correct; as a safety net,
   `release-plz` also runs [`cargo-semver-checks`](https://github.com/obi1kenobi/cargo-semver-checks)
   against the previously published version (`semver_check` in `release-plz.toml`) so
   unmarked API breakage still produces the required minor bump rather than a patch.
3. The maintainer reviews and merges the release PR.
4. Once publishing is enabled, `release-plz` runs `cargo publish` automatically after the
   release PR merges, in dependency order (`crustywad` before `crustywad-cli`).

There is no fixed release schedule. Releases happen when meaningful changes have
accumulated. The `release-plz` release PR is the signal that a release is ready.

**Publishing status:** Publishing to crates.io is currently disabled while credentials and
release infrastructure are being finalized (see
[ADR-0011](https://github.com/masriamir/crustywad/blob/main/docs/adr/0011-publish-workflow.md)
for the full publish workflow design).

---

## Version Compatibility Table

| Scenario | Patch | Minor | Major |
|---|---|---|---|
| Bug fix, no API change | yes | | |
| New public type or function | | yes | |
| New optional feature flag | | yes | |
| MSRV raised | | yes | |
| Public type removed or renamed | | | yes |
| Function signature changed | | | yes |
| Exhaustive enum variant added | | | yes |
