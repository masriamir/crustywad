# ADR-0011: crates.io publish workflow

- **Status:** Accepted
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
explicit version requirement:

```toml
crustywad = { path = "../crustywad", version = "0.1.0" }
```

crates.io resolves path dependencies as registry dependencies at publish time,
so the version named in `version = "..."` must already exist in the registry
before `cargo publish` is run for `crustywad-cli`. Publishing in reverse order
produces an unresolvable registry dependency and will be rejected.

`release-plz` handles this automatically: when `release-plz release` runs (via
the workflow job described in §2), it publishes workspace crates in dependency
order, so `crustywad` will be indexed on crates.io before `crustywad-cli` is
published. No explicit `cargo publish` calls or custom sequencing configuration
are required.

**Manual step — version bump:** `crustywad-cli/Cargo.toml` specifies the
library dependency as `version = "0.1.0"`. In Cargo, an unadorned version
string like `"0.1.0"` is a caret requirement (`^0.1.0`), not an exact pin —
patch bumps within `[0.1.0, 0.2.0)` satisfy the constraint without any change.
However, when `crustywad` is bumped to `0.2.0` or beyond, the caret range is
no longer satisfied: `cargo build`, `cargo test`, and `cargo clippy` all fail
before `cargo deny check` even runs, and publishing `crustywad-cli` would
register a dependency that crates.io cannot resolve. Update the version
specifier in `crustywad-cli/Cargo.toml` to match the new library version
before merging. This is a release-time checklist item, not an automated step.

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
changelog_path = "crates/crustywad/CHANGELOG.md"

[[package]]
name = "crustywad-cli"
release = true
publish = true          # was false; see note below on whether to publish the CLI
changelog_path = "crates/crustywad-cli/CHANGELOG.md"
```

A corresponding `release` job must be added to `.github/workflows/release-plz.yml`:

```yaml
  release:
    runs-on: ubuntu-latest
    needs: release-pr
    if: github.event_name == 'push'
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
`crustywad` and `crustywad-cli` crate names.

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

**Chosen: independent versioning (each crate carries its own version).**

Under independent versioning, each crate has its own `version` field in its
`Cargo.toml`; neither uses `version.workspace = true`. `release-plz` manages
each package independently, proposing version bumps only for crates whose
content has changed since the last release.

**Migration:** completed in PR #171. Both crates previously inherited the
workspace version via `version.workspace = true`; this was replaced with an
explicit `version` field in each crate's `[package]` block, starting from the
then-current workspace version:

```toml
# in crates/crustywad/Cargo.toml and crates/crustywad-cli/Cargo.toml:
version = "0.1.0"   # replaces version.workspace = true
```

Pros:
- A CLI fix does not force a library version bump, and vice-versa.
- Library consumers see version increments that reflect only API-relevant
  changes, reducing semver noise.
- Each crate's release cadence can diverge naturally as the project matures.

Cons:
- Whenever the library version changes beyond the caret range, the version
  requirement in `crustywad-cli/Cargo.toml` must be updated manually to match
  (see §1 manual step). This is the same checklist item as before, but it fires only
  on library releases rather than on every release.
- Two version numbers to track instead of one.

The library and CLI serve different audiences and have different change
frequencies. Decoupling their versions now avoids false semver signals to
library consumers before the project reaches 1.0.

### 4. Changelog management

Each crate maintains its own `CHANGELOG.md` under its crate directory:

- `crates/crustywad/CHANGELOG.md` — library release history
- `crates/crustywad-cli/CHANGELOG.md` — CLI release history

**Required migration:** Neither file exists yet — the repo currently has only a
root `CHANGELOG.md`. Before activating `changelog_path` in `release-plz.toml`,
create both files with an `[Unreleased]` section (seeded from the relevant
entries in the root changelog), then remove the root `CHANGELOG.md`.

`release-plz` writes entries to the per-crate paths via the `changelog_path`
setting in each `[[package]]` block (see §2).

`release-plz` will automatically:
- Move the `[Unreleased]` section content into a versioned `## [X.Y.Z]` section
  in the relevant crate's changelog when it creates a release PR.
- Add a git tag when the release PR is merged and the `release` job runs. With
  independent versioning, tags are per-crate (e.g., `crustywad-v0.2.0` and
  `crustywad-cli-v0.1.3`).

### 5. Pre-publish checklist

The following steps must be completed **before** setting `publish = true` in
`release-plz.toml` and merging to `main`:

1. **crates.io account** — confirm that the `masriamir` crates.io account exists
   and is in good standing (email verified, 2FA enabled).

2. **Metadata dry-run** — run `cargo publish --dry-run -p crustywad` and
   `cargo publish --dry-run -p crustywad-cli` locally to verify the crate names
   are available and metadata is valid. The dry-run will catch missing
   `description`, `license`, and `repository` fields before they reach CI.
   Note: dry-run does not reserve crate names on crates.io; the name is claimed
   only when the first `cargo publish` succeeds.

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

- Enabling publishing requires several one-time changes beyond toggling
  `release-plz.toml` and adding the `CARGO_REGISTRY_TOKEN` secret: both crates
  must switch from `version.workspace = true` to explicit `version` fields (§3
  migration), per-crate `CHANGELOG.md` files must be created and seeded (§4
  migration), and the `description` field must be added to `[workspace.package]`
  with `description.workspace = true` in each crate (checklist item 3). None of
  these touch library source code, but they do require `Cargo.toml` and
  documentation file changes before the first publish succeeds.
- The version requirement in `crustywad-cli/Cargo.toml` must be updated
  manually whenever `crustywad` bumps beyond the caret range (e.g., `0.1.x` →
  `0.2.0`). A bump outside that range breaks Cargo dependency resolution —
  `cargo build`, `cargo test`, and `cargo clippy` all fail before `cargo deny
  check` runs. With independent versioning this only fires on library releases,
  not on every release. A future improvement would be a `release-plz` post-hook
  that patches this automatically, but that is out of scope for this ADR.
- Independent versioning gives library consumers accurate semver signals: a
  version bump means the library changed, not the CLI.
- Each crate maintains its own `CHANGELOG.md`; the root `CHANGELOG.md` is
  retired when publishing is enabled.
- Once `crustywad 0.1.0` is on crates.io, yanking it is possible but
  disruptive. Ensuring the dry-run CI step and pre-publish checklist are
  completed before the first release avoids the need to yank.
- Issues #49–#52 (implementation work for milestones 2–4) are blocked on this
  ADR being accepted. The publish workflow must be agreed before implementation
  of those milestones begins, so that crate name reservation and the release
  pipeline are in place when the first stable milestone ships.
