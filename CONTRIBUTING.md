# Contributing

Thanks for your interest in `crustywad`.

## Local setup

1. Install Rust **1.94.0** or newer.
2. Install `just`.
3. Optionally install `cargo-llvm-cov`, `cargo-deny`, and `lefthook`.
4. Run `lefthook install` to enable the local hooks.

## Common commands

```text
just build
just test
just lint
just doc
just cov
just deny
just fetch-fixtures
just ci-fast
just ci
just ci-full
```

**Always run `just ci` before pushing.** It is the fail-fast pre-push gate (`docs-sync`, `fmt`, `clippy`, `test`, `doc`) and catches failures locally before they reach CI. `just ci-fast` is the mid-iteration loop (skips doctests and the rustdoc pass; not a pre-push substitute), and `just ci-full` adds the workspace `build` and the `cargo deny` dependency audit — use it before releases and on branches that change `Cargo.toml`/`Cargo.lock`.

## Conventional Commits

Commits are expected to follow Conventional Commits. The bundled `lefthook.yml` validates commit messages with a lightweight Python-based check.

## Freedoom fixtures

Some optional tests use Freedoom WADs. Download them with `just fetch-fixtures`, then run `just test-freedoom`. To invoke cargo directly, pass `--features freedoom-tests` (or `--all-features`) **and** set `CRUSTYWAD_FREEDOOM_DIR` to an **absolute** path — e.g. `CRUSTYWAD_FREEDOOM_DIR="$PWD/tests/fixtures/freedoom"`. A relative path does not resolve (cargo runs the test binary from the package root), and the fixture tests then skip silently instead of failing.

The Freedoom version is configurable — see `tests/fixtures/README.md` for details.

Freedoom is an open-source project; see the [Freedoom repository](https://github.com/freedoom/freedoom) for its license. Do not commit downloaded WAD blobs.

## Releases

`release-plz` is wired up for release PRs and Conventional Commits, but publishing to crates.io is intentionally disabled for now. To enable publishing later, update `release-plz.toml`, provide `CARGO_REGISTRY_TOKEN`, and adjust the release workflow.

**Version bump checklist:** `crates/crustywad-cli/Cargo.toml` declares the `crustywad` path dependency with an explicit `version` field (`crustywad = { path = "../crustywad", version = "X.Y.Z" }`), which Cargo treats as a caret requirement — patch/compatible bumps to `crustywad`'s version need no change here. `cargo-deny` requires this field (`wildcards = "deny"`), but it does not auto-track `crustywad`'s version. Crates are versioned independently (ADR-0011 §3); update this field only when `crustywad`'s version moves outside the current caret range (e.g. `0.1.z` → `0.2.0`).
