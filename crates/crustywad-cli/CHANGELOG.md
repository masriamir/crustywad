# Changelog

All notable changes to the `crustywad-cli` crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.4](https://github.com/masriamir/crustywad/compare/crustywad-cli-v0.3.3...crustywad-cli-v0.3.4) - 2026-07-22

### Other

- updated the following local packages: crustywad

## [0.3.3](https://github.com/masriamir/crustywad/compare/crustywad-cli-v0.3.2...crustywad-cli-v0.3.3) - 2026-07-21

### Added

- *(map)* read compressed ZDoom extended nodes (ZNOD/ZGL*) behind extended-nodes-zlib ([#327](https://github.com/masriamir/crustywad/pull/327)) ([#332](https://github.com/masriamir/crustywad/pull/332))

### Other

- compile-check the guide's Rust code samples as doctests ([#297](https://github.com/masriamir/crustywad/pull/297)) ([#325](https://github.com/masriamir/crustywad/pull/325))

## [0.3.2](https://github.com/masriamir/crustywad/compare/crustywad-cli-v0.3.1...crustywad-cli-v0.3.2) - 2026-07-20

### Added

- nodebuilder stage 3 — add_doom_map_with_nodes + cwad convert --nodes (ADR-0024 §9.3) ([#321](https://github.com/masriamir/crustywad/pull/321))
- *(build)* the classic BSP pass (build_nodes) with the mixed-sector amendment (ADR-0024 stage 2) ([#319](https://github.com/masriamir/crustywad/pull/319))
- *(build)* nodebuild feature with BLOCKMAP and REJECT builders (ADR-0024 stage 1) ([#317](https://github.com/masriamir/crustywad/pull/317))

## [0.3.1](https://github.com/masriamir/crustywad/compare/crustywad-cli-v0.3.0...crustywad-cli-v0.3.1) - 2026-07-19

### Added

- *(cli)* audio-aware extract with WAV wrapping and MUS-to-MIDI conversion ([#310](https://github.com/masriamir/crustywad/pull/310))
- *(sections)* recognize the KEX remaster's DM_START..DM_END music section ([#308](https://github.com/masriamir/crustywad/pull/308))
- *(audio)* AudioKind content detection plus DMX and PC-speaker sound decode ([#305](https://github.com/masriamir/crustywad/pull/305))

## [0.3.0](https://github.com/masriamir/crustywad/compare/crustywad-cli-v0.2.1...crustywad-cli-v0.3.0) - 2026-07-18

### Added

- *(gfx)* Doom 64 PNG decode behind the doom64-gfx feature ([#298](https://github.com/masriamir/crustywad/pull/298))
- *(gfx)* [**breaking**] texture composition — PNAMES, TEXTUREx, and the R_GenerateComposite contract ([#295](https://github.com/masriamir/crustywad/pull/295))
- *(gfx)* classic graphics decode — pictures, flats, PLAYPAL/COLORMAP ([#293](https://github.com/masriamir/crustywad/pull/293))
- *(map)* Doom 64 texture-name resolution and convert-gate lift ([#287](https://github.com/masriamir/crustywad/pull/287))
- *(wad)* marker-delimited directory section API ([#284](https://github.com/masriamir/crustywad/pull/284))

## [0.2.1](https://github.com/masriamir/crustywad/compare/crustywad-cli-v0.2.0...crustywad-cli-v0.2.1) - 2026-07-16

### Added

- *(map)* decode Doom 64 MACROS scripts onto the Map graph ([#276](https://github.com/masriamir/crustywad/pull/276))
- *(map)* decode Doom 64 LEAFS render leaves onto the Map graph ([#275](https://github.com/masriamir/crustywad/pull/275))
- *(map)* parse REJECT and BLOCKMAP into typed structures ([#274](https://github.com/masriamir/crustywad/pull/274))

### Fixed

- *(cli)* only print the --lenient hint for writer errors lenient mode can recover ([#272](https://github.com/masriamir/crustywad/pull/272))

## [0.2.0](https://github.com/masriamir/crustywad/compare/crustywad-cli-v0.1.4...crustywad-cli-v0.2.0) - 2026-07-15

### Added

- *(cli)* validate --deep assembles every map with per-map reporting ([#267](https://github.com/masriamir/crustywad/pull/267))
- *(cli)* cwad info delegates map detection to Wad::map_groups ([#266](https://github.com/masriamir/crustywad/pull/266))
- *(map)* [**breaking**] Doom 64 graph normalization — MapFormat::Doom64, TextureRef, engine light table (ADR-0021) ([#265](https://github.com/masriamir/crustywad/pull/265))

### Fixed

- *(map)* [**breaking**] honor the 0xffff front-sidedef sentinel on both sides (ADR-0020) ([#260](https://github.com/masriamir/crustywad/pull/260))

### Other

- *(sweep)* add the env-gated retail-WAD sweep behind sweep-tests ([#262](https://github.com/masriamir/crustywad/pull/262))

## [0.1.4](https://github.com/masriamir/crustywad/compare/crustywad-cli-v0.1.3...crustywad-cli-v0.1.4) - 2026-07-14

### Other

- pin documented crustywad version to 0.3 and guard against future drift ([#237](https://github.com/masriamir/crustywad/pull/237))

## [0.1.3](https://github.com/masriamir/crustywad/compare/crustywad-cli-v0.1.2...crustywad-cli-v0.1.3) - 2026-07-13

### Added

- *(cli)* cwad convert — UDMF <-> Doom map conversion ([#234](https://github.com/masriamir/crustywad/pull/234))
- *(map)* UDMF <-> Doom map conversion (library) ([#233](https://github.com/masriamir/crustywad/pull/233))
- *(map)* serialize maps to UDMF TEXTMAP (write_udmf / add_udmf_map) ([#231](https://github.com/masriamir/crustywad/pull/231))
- *(map)* read Doom 64 nested-WAD maps into raw records (map::doom64) ([#230](https://github.com/masriamir/crustywad/pull/230))
- *(map)* Hexen map format support + MapFormat substrate ([#55](https://github.com/masriamir/crustywad/pull/55)) ([#221](https://github.com/masriamir/crustywad/pull/221))

### Other

- *(fixtures)* generalize the local-fixture harness for Hexen & Doom 64 ([#216](https://github.com/masriamir/crustywad/pull/216)) ([#218](https://github.com/masriamir/crustywad/pull/218))
- *(map)* Heretic and Doom II map support via the Doom path ([#56](https://github.com/masriamir/crustywad/pull/56)) ([#208](https://github.com/masriamir/crustywad/pull/208))

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
