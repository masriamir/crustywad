# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- **Breaking:** `Seg::angle` field type corrected from `i16` to `u16`. Binary
  angles (BAMS) are unsigned; `0x8000` encodes 180° and must not be
  sign-extended to `-32768`.

### Added

- Initial workspace scaffold with a safe WAD header and directory reader.
- Minimal CLI for printing WAD metadata and lump listings.
- CI, release-plz, repository policy files, ADRs, and development tooling.
