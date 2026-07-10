# Changelog

All notable changes to the `crustywad-cli` crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.2](https://github.com/masriamir/crustywad/compare/crustywad-cli-v0.1.1...crustywad-cli-v0.1.2) - 2026-07-10

### Added

- *(map)* assemble Doom map records into a validated Map graph ([#155](https://github.com/masriamir/crustywad/pull/155)) ([#205](https://github.com/masriamir/crustywad/pull/205))

## [0.1.1](https://github.com/masriamir/crustywad/compare/crustywad-cli-v0.1.0...crustywad-cli-v0.1.1) - 2026-07-08

### Other

- *(release)* ship cwad binaries via dist + GitHub App ([#185](https://github.com/masriamir/crustywad/pull/185))
- *(release)* enable crates.io publishing via Trusted Publishing (OIDC) ([#182](https://github.com/masriamir/crustywad/pull/182))

## [0.1.0](https://github.com/masriamir/crustywad/releases/tag/crustywad-cli-v0.1.0) - 2026-07-07

### Added

- *(cli)* cwad merge — combine multiple WAD files ([#38](https://github.com/masriamir/crustywad/pull/38)) ([#124](https://github.com/masriamir/crustywad/pull/124))
- *(cli)* write support wired through cwad CLI ([#25](https://github.com/masriamir/crustywad/pull/25)) ([#123](https://github.com/masriamir/crustywad/pull/123))
- *(cli)* cwad extract — extract lumps to disk ([#39](https://github.com/masriamir/crustywad/pull/39)) ([#116](https://github.com/masriamir/crustywad/pull/116))
- *(cli)* cwad diff — lump-by-lump WAD comparison ([#37](https://github.com/masriamir/crustywad/pull/37)) ([#118](https://github.com/masriamir/crustywad/pull/118))
- *(cli)* expand cwad info with richer WAD metadata ([#36](https://github.com/masriamir/crustywad/pull/36)) ([#119](https://github.com/masriamir/crustywad/pull/119))
- *(cli)* cwad validate + CLI foundation (Format enum, exit codes, build.rs) ([#35](https://github.com/masriamir/crustywad/pull/35)) ([#104](https://github.com/masriamir/crustywad/pull/104))
- *(mmap)* expose mmap loading as explicit from_path_mapped API
- enhance error handling and parsing contracts in Wad and map records
- configurable FreeDoom version, split map tests, add CodeQL, add copilot instructions, update README
- scaffold crustywad workspace

### Fixed

- *(docs)* mark write support complete in roadmap listings ([#148](https://github.com/masriamir/crustywad/pull/148))
- pin crustywad path dependency version in Cargo.toml
- specify version for crustywad dependency in Cargo.toml
- update dependencies and improve comments for clarity in mmap handling

### Other

- overhaul README.md as a concise crates.io landing page ([#179](https://github.com/masriamir/crustywad/pull/179))
- *(release)* implement independent versioning migration (ADR-0011 §3) ([#171](https://github.com/masriamir/crustywad/pull/171))
- *(release)* add crates.io metadata and verify publish dry-run ([#173](https://github.com/masriamir/crustywad/pull/173))
- *(readme)* add benchmarks and deps.rs badges; document write feature flag ([#161](https://github.com/masriamir/crustywad/pull/161))
- *(cli)* replace hardcoded /nonexistent paths with TempDir paths ([#142](https://github.com/masriamir/crustywad/pull/142))
- *(cli)* comprehensive CLI hardening test suite ([#40](https://github.com/masriamir/crustywad/pull/40)) ([#136](https://github.com/masriamir/crustywad/pull/136))
- *(cli)* CLI usage documentation and man page ([#29](https://github.com/masriamir/crustywad/pull/29)) ([#117](https://github.com/masriamir/crustywad/pull/117))
- *(release)* per-crate changelogs and release-plz changelog_path ([#50](https://github.com/masriamir/crustywad/pull/50)) ([#107](https://github.com/masriamir/crustywad/pull/107))
- *(lib)* improve rustdoc coverage and examples for public API ([#75](https://github.com/masriamir/crustywad/pull/75))
- *(cli)* add integration tests for all cwad subcommands
- clarify WAD structure and memory-mapped file loading details in documentation
- update contributing guidelines and clarify unsafe code restrictions

### Added

- Minimal CLI for printing WAD metadata and lump listings.
