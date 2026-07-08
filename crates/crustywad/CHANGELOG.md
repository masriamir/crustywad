# Changelog

All notable changes to the `crustywad` library crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
