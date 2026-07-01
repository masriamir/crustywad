# Versioning and Release Policy

This page documents the SemVer guarantees, MSRV policy, versioning model, and release
cadence for `crustywad` and `crustywad-cli`.

---

## Semantic Versioning

Both crates follow [Semantic Versioning 2.0.0](https://semver.org/). While the crates are
currently at `0.y.z` — which SemVer treats as explicitly unstable — this project uses
patch, minor, and major increments as compatibility signals as documented on this page.
A `0.y.z` version is not a license to make arbitrary breaking changes in patches.

### Patch releases (`0.x.PATCH`)

A patch release fixes a bug without changing any public API. It is safe for all existing
callers to upgrade without modification.

Examples of patch changes:

- Correcting incorrect byte offsets in a parser
- Fixing a panic or incorrect error variant in an existing code path
- Updating documentation without changing behavior
- Updating a dependency to a compatible patch version

### Minor releases (`0.MINOR.0`)

A minor release adds new functionality in a backward-compatible way. Existing callers
compile and run without modification.

Examples of minor changes:

- Adding a new public type, function, or method
- Adding a new feature flag that is off by default
- Adding a new variant to a non-exhaustive enum
- Raising the MSRV (see [MSRV policy](#msrv-policy) below)

### Major releases (`MAJOR.0.0`)

A major release contains at least one breaking change. Callers may need to update their
code after upgrading.

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

The current minimum supported Rust version (MSRV) is **1.85.0**, set via `rust-version` in
`Cargo.toml`. The project targets the Rust 2024 edition.

Rules:

- **An MSRV bump is a minor version change**, never a patch. A caller pinned to the old
  compiler will fail to build after an MSRV bump, so it is treated as a backward-incompatible
  change to the build environment even though the public API is unchanged.
- **MSRV bumps are need-driven.** The MSRV will only be raised when there is a concrete need
  (for example, a required dependency or language feature), and only to a toolchain version
  that has been stable for a reasonable period.
- **CI enforces the declared MSRV.** The `msrv` job in CI builds and tests the workspace on
  the declared MSRV on every PR. The toolchain version is pinned explicitly in
  `.github/workflows/ci.yml` and does not auto-track `[workspace.package].rust-version`. A
  PR that raises the MSRV must update both the `rust-version` field in `Cargo.toml` and the
  `toolchain:` pin in the workflow file, then bump the crate version accordingly.

---

## Versioning Model

### Current state: shared workspace version

Both crates currently use `version.workspace = true`, inheriting their version from
`[workspace.package]` in the root `Cargo.toml`. A single version bump increments the
version for both crates simultaneously.

**Dependency constraint:** `crustywad-cli/Cargo.toml` pins the library with an explicit
caret requirement (e.g., `crustywad = { version = "0.1.0", ... }`), required by
`cargo-deny`'s `wildcards = "deny"` setting (which disallows `*` version requirements).
`version = "0.1.0"` resolves as `^0.1.0` (`>=0.1.0, <0.2.0`), so patch bumps within
the same minor series are satisfied automatically. When the workspace version moves outside
that range (e.g., to `0.2.0`), this field must be updated manually before merging —
otherwise `cargo build` and crates.io publishing will fail.

### Planned: independent per-crate versioning

Per [ADR-0011](https://github.com/masriamir/crustywad/blob/main/docs/adr/0011-publish-workflow.md),
the chosen long-term strategy is independent per-crate versioning: each crate will carry
its own explicit `version` field rather than inheriting from the workspace. This migration
is a required step before enabling crates.io publishing. Until then, both crates share the
workspace version as described above.

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
   and an updated `CHANGELOG.md`.
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
