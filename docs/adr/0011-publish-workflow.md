# ADR-0011: crates.io publish workflow

- **Status:** Proposed
- **Date:** 2026-06-14
- **Deciders:** @masriamir
- **Tracking issue:** https://github.com/masriamir/crustywad/issues/48

## Context

Publishing to crates.io is intentionally disabled (`publish = false` in
`release-plz.toml` and in each crate's `[[package]]` block). The project uses
`release-plz` to automate release PRs, CHANGELOG entries, and git tags, but
the final `cargo publish` step has never been wired up.

Before enabling publishing, several decisions must be made: which crates
publish, in what order, how versions are managed across the workspace, whether
the current CHANGELOG format is acceptable, and what manual prerequisites must
be met the first time.

This ADR documents the chosen direction for each of those five areas and the
concrete configuration changes required to implement it.

## Decision

### 1. Publish order

`crustywad` (the library) **must** be published before `crustywad-cli` (the
binary). `crustywad-cli` carries a hard dependency on the library with an
explicit version pin:

```toml
crustywad = { path = "../crustywad", version = "0.1.0" }
```

crates.io resolves path dependencies as registry dependencies at publish time,
so the version named in `version = "..."` must already exist in the registry
before `cargo publish` is run for `crustywad-cli`. Publishing in reverse order
produces an unresolvable registry dependency and will be rejected.

`release-plz` supports this via its `publish_jobs` configuration. Setting a
`publish_sequence` (or simply publishing crates individually in dependency
order) ensures the library lands in the registry before the CLI attempts to
reference it. The release workflow job should call `cargo publish -p crustywad`
first, wait for crates.io to index it (typically under 60 seconds; the
`--no-verify` flag is not needed), and then call `cargo publish -p crustywad-cli`.

**Manual step — version bump:** `cargo-deny` enforces `wildcards = "deny"`,
which means every dependency in `Cargo.toml` must carry an explicit version
specifier. `crustywad-cli/Cargo.toml` does not inherit the workspace version
for its `crustywad` dependency — it pins it explicitly. Whenever the workspace
version is bumped (by `release-plz` or manually), the pinned version in
`crustywad-cli/Cargo.toml` **must** be updated to match or `cargo deny check`
will fail in CI. This is a release-time checklist item, not an automated step.

### 2. release-plz configuration

The following changes to `release-plz.toml` are required to enable publishing:

```toml
[workspace]
release = true
publish = true          # was false
changelog_update = true
allow_dirty = false
semver_check = false    # no baseline until 0.1.0 is published; enable after first release

[[package]]
name = "crustywad"
release = true
publish = true          # was false

[[package]]
name = "crustywad-cli"
release = true
publish = true          # was false; see note below on whether to publish the CLI
```

A corresponding `publish` job must be added to `.github/workflows/release-plz.yml`:

```yaml
  release:
    runs-on: ubuntu-latest
    needs: release-pr
    steps:
      - uses: actions/checkout@v7
        with:
          fetch-depth: 0
      - uses: dtolnay/rust-toolchain@stable
      - uses: release-plz/action@v0.5
        with:
          command: release
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}
```

The **required GitHub secret** is `CARGO_REGISTRY_TOKEN`. It must be set in the
repository's Settings → Secrets and variables → Actions. The token must be
scoped to the `publish-new` and `publish-update` operations for the
`masriamir:crustywad` and `masriamir:crustywad-cli` crate names.

**Should `crustywad-cli` publish?** Yes, but with lower priority than the
library. The CLI (`cwad`) provides a useful dogfooding surface for end users who
want a command-line WAD inspector without writing Rust. Publishing it costs
nothing beyond maintaining the crate name reservation on crates.io. Setting
`publish = true` for `crustywad-cli` is the chosen direction. If the CLI is
later deprecated (e.g., the project decides the binary is out of scope),
`publish = false` can be re-applied to the `[[package]]` block without affecting
the library.

`semver_check` is left `false` for the first publish. `release-plz` runs
`cargo-semver-checks` to detect accidental breaking changes before a release PR
is merged, but the check requires a published baseline to diff against. Because
no baseline exists until `crustywad 0.1.0` lands on crates.io, enabling it
immediately would produce spurious failures. Once `0.1.0` is published, flip
`semver_check` to `true` in `release-plz.toml` (see pre-publish checklist §7).

### 3. Version strategy

**Chosen: unified versioning (both crates share the workspace version).**

Both crates carry `version.workspace = true` in their `Cargo.toml` files and
will track the same `[workspace.package] version` field. When `release-plz`
proposes a version bump it bumps that single field and both crates move
together.

Pros:
- Simple mental model: one version number describes the whole project at any
  point in time.
- The explicit pin in `crustywad-cli/Cargo.toml` stays correct as long as it
  is updated in the same commit that bumps the workspace version (see §1).
- Release notes and the CHANGELOG are naturally unified.

Cons:
- A patch fix to the CLI forces a library version bump (and vice-versa), even
  if only one crate changed.
- Once the API surface is large and stable, consumers of the library may
  accumulate unnecessary semver churn from CLI-only changes.

The cons are acceptable at the project's current scale (pre-1.0, small API
surface). If the library stabilizes and the CLI diverges significantly in
release cadence, switching to independent versioning is straightforward: remove
`version.workspace = true` from one crate, give it its own `version` field, and
update `release-plz.toml` to manage the two packages independently.

### 4. Changelog management

The current format — a single `CHANGELOG.md` at the workspace root using
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) with `## [Unreleased]`
headers — is acceptable as-is and no customization is required at this stage.

`release-plz` will automatically:
- Move the `[Unreleased]` section content into a versioned `## [X.Y.Z]` section
  when it creates a release PR.
- Add a git tag `vX.Y.Z` when the release PR is merged and the `release` job
  runs.

No `changelog_config` block is needed in `release-plz.toml`. The only
configuration that should be considered in a follow-up is whether to generate
per-crate changelogs (e.g., `CHANGELOG.md` inside each crate directory) once the
two crates evolve at different rates. For now, the single root changelog is
sufficient.

### 5. Pre-publish checklist

The following steps must be completed **before** setting `publish = true` in
`release-plz.toml` and merging to `main`:

1. **crates.io account** — confirm that the `masriamir` crates.io account exists
   and is in good standing (email verified, 2FA enabled).

2. **Crate name reservation** — run `cargo publish --dry-run -p crustywad` and
   `cargo publish --dry-run -p crustywad-cli` locally to verify the crate names
   are available and metadata is valid. The dry-run will catch missing
   `description`, `license`, and `repository` fields before they reach CI.

3. **`description` field** — add a `description` field to `[workspace.package]`
   in `Cargo.toml` and `description.workspace = true` to each crate's `[package]`
   block. crates.io requires this field and `cargo publish` will fail without it
   (workspace fields are not automatically inherited — each crate must opt in):
   ```toml
   # in [workspace.package]:
   description = "Safe, documented Doom WAD file I/O"
   # in each crate's [package]:
   description.workspace = true
   ```

4. **README** — `readme = "README.md"` is already set in the workspace package
   block. Ensure `README.md` exists at the workspace root and contains enough
   content to serve as the crates.io landing page for `crustywad`. A minimal
   README with a short description, installation snippet, and link to docs.rs
   is sufficient for 0.1.0.

5. **keywords and categories** — `keywords = ["doom", "wad", "parser"]` and
   `categories = ["parser-implementations", "game-development"]` are already
   set in `[workspace.package]`. Verify both values are on the
   [allowed categories list](https://crates.io/category_slugs) for crates.io.
   `"game-development"` maps to the `game-development` slug; confirm
   `"parser-implementations"` is accepted (it is, as of the current slug list).

6. **`CARGO_REGISTRY_TOKEN` secret** — generate a crates.io API token with
   `publish-new` and `publish-update` scopes and store it as a GitHub Actions
   secret named `CARGO_REGISTRY_TOKEN`. Do not use a token with `yank` or
   `delete` scope in CI.

7. **`semver_check` baseline** — after `0.1.0` publishes, `cargo-semver-checks`
   will have a baseline to diff against. Ensure `semver_check = true` is set in
   `release-plz.toml` before the second release.

8. **Dry-run publish in CI** — add a CI step that runs
   `cargo publish --dry-run -p crustywad` on every PR targeting `main`. This
   catches metadata problems before the release PR is merged.

## Consequences

- Enabling publishing requires touching `release-plz.toml`, the release workflow
  YAML, and adding the `CARGO_REGISTRY_TOKEN` secret — no source code changes.
- The version pin in `crustywad-cli/Cargo.toml` introduces a permanent manual
  maintenance burden: every version bump must update that field or CI fails. A
  future improvement would be a custom `release-plz` post-hook that patches this
  automatically, but that is out of scope for this ADR.
- Unified versioning means downstream library consumers may see version numbers
  advance faster than the API changes warrant. Semver pre-1.0 (`0.x.y`) gives
  broad latitude here; this is re-evaluated at 1.0 planning.
- Once `crustywad 0.1.0` is on crates.io, yanking it is possible but
  disruptive. Ensuring the dry-run CI step and pre-publish checklist are
  completed before the first release avoids the need to yank.
- Issues #49–#52 (implementation work for milestones 2–4) are blocked on this
  ADR being accepted. The publish workflow must be agreed before implementation
  of those milestones begins, so that crate name reservation and the release
  pipeline are in place when the first stable milestone ships.
