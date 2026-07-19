# Changelog

All notable changes to the `crustywad` library crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.6.1](https://github.com/masriamir/crustywad/compare/crustywad-v0.6.0...crustywad-v0.6.1) - 2026-07-19

### Added

- *(cli)* audio-aware extract with WAV wrapping and MUS-to-MIDI conversion ([#310](https://github.com/masriamir/crustywad/pull/310))
- *(audio)* SNDINFO, SNDSEQ, and SNDCURVE script lumps (vanilla dialect) ([#309](https://github.com/masriamir/crustywad/pull/309))
- *(sections)* recognize the KEX remaster's DM_START..DM_END music section ([#308](https://github.com/masriamir/crustywad/pull/308))
- *(audio)* MUS score, MIDI/WAV chunk parsers, GENMIDI and DMXGUS instrument banks ([#307](https://github.com/masriamir/crustywad/pull/307))
- *(audio)* AudioKind content detection plus DMX and PC-speaker sound decode ([#305](https://github.com/masriamir/crustywad/pull/305))

## [0.6.0](https://github.com/masriamir/crustywad/compare/crustywad-v0.5.0...crustywad-v0.6.0) - 2026-07-18

### Added

- *(gfx)* Doom 64 PNG decode behind the doom64-gfx feature ([#298](https://github.com/masriamir/crustywad/pull/298))
- *(gfx)* [**breaking**] texture composition — PNAMES, TEXTUREx, and the R_GenerateComposite contract ([#295](https://github.com/masriamir/crustywad/pull/295))
- *(gfx)* classic graphics decode — pictures, flats, PLAYPAL/COLORMAP ([#293](https://github.com/masriamir/crustywad/pull/293))
- *(map)* Doom 64 texture-name resolution and convert-gate lift ([#287](https://github.com/masriamir/crustywad/pull/287))
- *(wad)* marker-delimited directory section API ([#284](https://github.com/masriamir/crustywad/pull/284))

## [0.5.0](https://github.com/masriamir/crustywad/compare/crustywad-v0.4.0...crustywad-v0.5.0) - 2026-07-16

### Added

- *(map)* decode Doom 64 MACROS scripts onto the Map graph ([#276](https://github.com/masriamir/crustywad/pull/276))
- *(map)* decode Doom 64 LEAFS render leaves onto the Map graph ([#275](https://github.com/masriamir/crustywad/pull/275))
- *(map)* parse REJECT and BLOCKMAP into typed structures ([#274](https://github.com/masriamir/crustywad/pull/274))

### Fixed

- *(cli)* only print the --lenient hint for writer errors lenient mode can recover ([#272](https://github.com/masriamir/crustywad/pull/272))

## [0.4.0](https://github.com/masriamir/crustywad/compare/crustywad-v0.3.1...crustywad-v0.4.0) - 2026-07-15

### Added

- *(map)* BSP traversal — SEGS/SSECTORS/NODES onto the Map graph ([#268](https://github.com/masriamir/crustywad/pull/268))
- *(map)* [**breaking**] Doom 64 graph normalization — MapFormat::Doom64, TextureRef, engine light table (ADR-0021) ([#265](https://github.com/masriamir/crustywad/pull/265))

### Fixed

- *(map)* [**breaking**] honor the 0xffff front-sidedef sentinel on both sides (ADR-0020) ([#260](https://github.com/masriamir/crustywad/pull/260))

### Other

- *(sweep)* gate-expecting extended-node collection via CRUSTYWAD_SWEEP_EXTENDED_DIR ([#270](https://github.com/masriamir/crustywad/pull/270))
- *(sweep)* use ParseOptions::strict() explicitly; surface the read_dir error in skip notes ([#263](https://github.com/masriamir/crustywad/pull/263))
- *(sweep)* add the env-gated retail-WAD sweep behind sweep-tests ([#262](https://github.com/masriamir/crustywad/pull/262))

## [0.3.1](https://github.com/masriamir/crustywad/compare/crustywad-v0.3.0...crustywad-v0.3.1) - 2026-07-14

### Other

- pin documented crustywad version to 0.3 and guard against future drift ([#237](https://github.com/masriamir/crustywad/pull/237))

## [0.3.0](https://github.com/masriamir/crustywad/compare/crustywad-v0.2.0...crustywad-v0.3.0) - 2026-07-13

### Added

- *(cli)* cwad convert — UDMF <-> Doom map conversion ([#234](https://github.com/masriamir/crustywad/pull/234))
- *(map)* UDMF <-> Doom map conversion (library) ([#233](https://github.com/masriamir/crustywad/pull/233))
- *(map)* serialize maps to UDMF TEXTMAP (write_udmf / add_udmf_map) ([#231](https://github.com/masriamir/crustywad/pull/231))
- *(map)* read Doom 64 nested-WAD maps into raw records (map::doom64) ([#230](https://github.com/masriamir/crustywad/pull/230))
- *(map)* assemble UDMF maps into the Map graph (MapFormat::Udmf) — PR B of #58 ([#228](https://github.com/masriamir/crustywad/pull/228))
- *(map)* UDMF text-map parser (parse_udmf + map::udmf) — PR A of #58 ([#227](https://github.com/masriamir/crustywad/pull/227))
- *(map)* [**breaking**] UDMF foundation — Limits/ParseOptions.limits + resolve_* i32 widening ([#58](https://github.com/masriamir/crustywad/pull/58)) ([#224](https://github.com/masriamir/crustywad/pull/224))
- *(map)* Hexen map format support + MapFormat substrate ([#55](https://github.com/masriamir/crustywad/pull/55)) ([#221](https://github.com/masriamir/crustywad/pull/221))

### Other

- *(map)* [**breaking**] reconcile graph types with ADR-0017 §1 (Special rename; MapThing id/height) ([#222](https://github.com/masriamir/crustywad/pull/222)) ([#223](https://github.com/masriamir/crustywad/pull/223))
- *(fixtures)* generalize the local-fixture harness for Hexen & Doom 64 ([#216](https://github.com/masriamir/crustywad/pull/216)) ([#218](https://github.com/masriamir/crustywad/pull/218))
- *(map)* Heretic and Doom II map support via the Doom path ([#56](https://github.com/masriamir/crustywad/pull/56)) ([#208](https://github.com/masriamir/crustywad/pull/208))

## [0.2.0](https://github.com/masriamir/crustywad/compare/crustywad-v0.1.1...crustywad-v0.2.0) - 2026-07-10

### Added

- *(map)* assemble Doom map records into a validated Map graph ([#155](https://github.com/masriamir/crustywad/pull/155)) ([#205](https://github.com/masriamir/crustywad/pull/205))

### Fixed

- *(bench)* give large-input parse benchmarks more measurement time ([#191](https://github.com/masriamir/crustywad/pull/191))

### Other

- *(map)* [**breaking**] split records into map::doom / map::common; consolidate name decode ([#201](https://github.com/masriamir/crustywad/pull/201)) ([#202](https://github.com/masriamir/crustywad/pull/202))

## [0.1.1](https://github.com/masriamir/crustywad/compare/crustywad-v0.1.0...crustywad-v0.1.1) - 2026-07-08

### Other

- *(release)* ship cwad binaries via dist + GitHub App ([#185](https://github.com/masriamir/crustywad/pull/185))
- *(release)* enable crates.io publishing via Trusted Publishing (OIDC) ([#182](https://github.com/masriamir/crustywad/pull/182))

## [0.1.0](https://github.com/masriamir/crustywad/releases/tag/crustywad-v0.1.0) - 2026-07-07

### Added

- *(bench)* Criterion benchmarking infrastructure ([#150](https://github.com/masriamir/crustywad/pull/150))
- *(write)* strict/lenient write validation per ADR-0006 ([#23](https://github.com/masriamir/crustywad/pull/23)) ([#120](https://github.com/masriamir/crustywad/pull/120))
- *(write)* WAD builder with header and directory serialization ([#21](https://github.com/masriamir/crustywad/pull/21)) ([#103](https://github.com/masriamir/crustywad/pull/103))
- *(mmap)* expose mmap loading as explicit from_path_mapped API
- enhance error handling and parsing contracts in Wad and map records
- configurable FreeDoom version, split map tests, add CodeQL, add copilot instructions, update README
- scaffold crustywad workspace

### Fixed

- *(ci)* exclude lib from bench mode and update crossbeam-epoch ([#167](https://github.com/masriamir/crustywad/pull/167))
- *(docs)* mark write support complete in roadmap listings ([#148](https://github.com/masriamir/crustywad/pull/148))
- update dependencies and improve comments for clarity in mmap handling

### Other

- overhaul README.md as a concise crates.io landing page ([#179](https://github.com/masriamir/crustywad/pull/179))
- *(release)* implement independent versioning migration (ADR-0011 §3) ([#171](https://github.com/masriamir/crustywad/pull/171))
- add write-path guide page, runnable examples/, and expand write-path doctests ([#177](https://github.com/masriamir/crustywad/pull/177))
- *(release)* add crates.io metadata and verify publish dry-run ([#173](https://github.com/masriamir/crustywad/pull/173))
- *(adr)* lump_by_name lookup strategy ([#160](https://github.com/masriamir/crustywad/pull/160))
- *(readme)* add benchmarks and deps.rs badges; document write feature flag ([#161](https://github.com/masriamir/crustywad/pull/161))
- *(e2e)* read→modify→write integration tests ([#43](https://github.com/masriamir/crustywad/pull/43)) ([#122](https://github.com/masriamir/crustywad/pull/122))
- *(write)* round-trip write→read proptest invariant ([#24](https://github.com/masriamir/crustywad/pull/24)) ([#121](https://github.com/masriamir/crustywad/pull/121))
- *(write)* exhaustive write coverage for WadBuilder edge cases ([#26](https://github.com/masriamir/crustywad/pull/26)) ([#125](https://github.com/masriamir/crustywad/pull/125))
- *(release)* per-crate changelogs and release-plz changelog_path ([#50](https://github.com/masriamir/crustywad/pull/50)) ([#107](https://github.com/masriamir/crustywad/pull/107))
- *(docs)* docs.rs configuration (all-features, rustdoc-args) ([#49](https://github.com/masriamir/crustywad/pull/49)) ([#105](https://github.com/masriamir/crustywad/pull/105))
- *(proptest)* parser invariant property tests per ADR-0010 ([#47](https://github.com/masriamir/crustywad/pull/47)) ([#108](https://github.com/masriamir/crustywad/pull/108))
- *(coverage)* add tests to raise line coverage to >=90% ([#78](https://github.com/masriamir/crustywad/pull/78))
- *(corpus)* add malformed and large WAD test corpus ([#76](https://github.com/masriamir/crustywad/pull/76))
- *(lib)* improve rustdoc coverage and examples for public API ([#75](https://github.com/masriamir/crustywad/pull/75))
- *(mmap)* simplify tests for nonexistent file handling
- *(mmap)* add tests for handling nonexistent and empty files
- *(cli)* add integration tests for all cwad subcommands
- clarify memory-mapped file handling and safety in WAD documentation
- clarify safety guarantees for memory-mapped file handling in mmap.rs
- add comprehensive tests for record parsing and lump file position handling
- clarify WAD structure and memory-mapped file loading details in documentation
- enhance directory handling and type specifications in map structure
- update contributing guidelines and clarify unsafe code restrictions
- enhance memory management and parsing in Wad structure with mmap support
- improve formatting of error handling in lump range checks
- streamline CI configuration and enhance error handling in lib.rs
- format error messages and improve code readability
- move inline tests from lib.rs into integration test file

### Changed

- **Breaking:** `Seg::angle` field type corrected from `i16` to `u16`. Binary
  angles (BAMS) are unsigned; `0x8000` encodes 180° and must not be
  sign-extended to `-32768`.
- `parse_records` now derives the per-record byte size from BinRead cursor
  advancement instead of `size_of::<T>()`. This correctly handles types whose
  in-memory size (including alignment padding) exceeds their on-disk byte count,
  and maps a truncated first-record read to `MapParseError::TrailingBytes`
  instead of `MapParseError::Binrw`.

### Added

- Initial workspace scaffold with a safe WAD header and directory reader.
- CI, release-plz, repository policy files, ADRs, and development tooling.
