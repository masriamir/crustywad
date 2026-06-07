# crustywad

[![CI](https://github.com/masriamir/crustywad/actions/workflows/ci.yml/badge.svg)](https://github.com/masriamir/crustywad/actions/workflows/ci.yml)
[![CodeQL](https://github.com/masriamir/crustywad/actions/workflows/codeql.yml/badge.svg)](https://github.com/masriamir/crustywad/actions/workflows/codeql.yml)
[![Coverage](https://codecov.io/gh/masriamir/crustywad/graph/badge.svg)](https://codecov.io/gh/masriamir/crustywad)
[![docs.rs](https://img.shields.io/badge/docs.rs-pending-blue)](https://docs.rs/crustywad)
[![crates.io](https://img.shields.io/badge/crates.io-pending-inactive)](https://crates.io/crates/crustywad)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)
[![MSRV](https://img.shields.io/badge/MSRV-1.85.0-blue)](https://www.rust-lang.org)

WAD file I/O implemented in Rust.

A Doom WAD is a container format that stores a header plus a directory of named "lumps" containing maps, graphics, audio, and other game data. The [Doom Wiki](https://doomwiki.org/wiki/WAD) is a good starting point for the unofficial format specification.

## Status

`crustywad` currently provides a safe, documented foundation for reading WAD headers and lump directories, plus typed map-record scaffolding for the classic Doom map lumps.

Integration tests for each layer live in `crates/crustywad/tests/`:
- `wad_reader.rs` — WAD header and directory parsing
- `map_records.rs` — typed map-record decoding (`Thing`, `Linedef`, `Sector`, etc.)
- `freedoom.rs` — optional tests against real FreeDoom WAD fixtures

## Workspace layout

- `crates/crustywad` — core library for safe WAD parsing.
- `crates/crustywad-cli` — small CLI binary (`cwad`) for dogfooding the parser.
- `docs/` — design notes and ADRs.
- `.github/` — CI, release automation, issue templates, and repository policy files.

## Quickstart

### Library

```rust
use crustywad::Wad;

let bytes = [
    b'I', b'W', b'A', b'D',
    0, 0, 0, 0,
    12, 0, 0, 0,
];

let wad = Wad::from_bytes(bytes)?;
assert_eq!(wad.lump_count(), 0);
# Ok::<(), crustywad::ParseError>(())
```

### CLI

```text
cargo run -p crustywad-cli -- info path/to/file.wad
cargo run -p crustywad-cli -- list path/to/file.wad
```

## Feature flags

| Feature | Default | Description |
| --- | --- | --- |
| `mmap` | no | Enables memory-mapped file loading via `memmap2`. The file is mapped read-only and held for the `Wad`'s lifetime — no heap copy is made. |
| `freedoom-tests` | no | Enables optional integration tests that inspect locally downloaded FreeDoom fixtures. |

## Development

Install [`just`](https://github.com/casey/just) and run:

```text
just build
just test
just lint
just doc
just fetch-fixtures
just ci
```

`just cov` uses `cargo-llvm-cov`, and the Codecov upload in CI may require a `CODECOV_TOKEN` repository secret depending on repository visibility and Codecov settings.

### FreeDoom fixtures

Optional integration tests parse real FreeDoom WAD files. The FreeDoom version to download is configurable:

```bash
# Default version (v0.13.0)
just fetch-fixtures

# Override via argument
just fetch-fixtures version=v0.14.0

# Override via environment variable
FREEDOOM_VERSION=v0.14.0 just fetch-fixtures
```

Enable the optional fixture coverage by passing `--features freedoom-tests` (or `--all-features`) **and** setting `CRUSTYWAD_FREEDOOM_DIR=tests/fixtures/freedoom` when running tests locally.

## MSRV

The minimum supported Rust version is **1.85.0**, matching the first stable release with Rust edition 2024 support.

## Roadmap

1. Directory reading
2. Map lump parsing
3. Graphics
4. Textures
5. Audio
6. Write support

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for setup, hooks, fixture handling, and release notes.

## License

Licensed under either of:

- [MIT License](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

at your option.
