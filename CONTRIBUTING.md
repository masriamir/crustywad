# Contributing

Thanks for your interest in `crustywad`.

## Local setup

1. Install Rust **1.85.0** or newer.
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
just ci
```

## Conventional Commits

Commits are expected to follow Conventional Commits. The bundled `lefthook.yml` validates commit messages with a lightweight Python-based check.

## FreeDoom fixtures

Some optional tests use FreeDoom WADs. Download them with `just fetch-fixtures`, then pass `--features freedoom-tests` (or `--all-features`) **and** set `CRUSTYWAD_FREEDOOM_DIR=tests/fixtures/freedoom` when running tests locally.

The FreeDoom version is configurable — see `tests/fixtures/README.md` for details.

FreeDoom is an open-source project; see the [FreeDoom repository](https://github.com/freedoom/freedoom) for its license. Do not commit downloaded WAD blobs.

## Releases

`release-plz` is wired up for release PRs and Conventional Commits, but publishing to crates.io is intentionally disabled for now. To enable publishing later, update `release-plz.toml`, provide `CARGO_REGISTRY_TOKEN`, and adjust the release workflow.
