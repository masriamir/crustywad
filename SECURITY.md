# Security Policy

## Supported versions

This project is pre-1.0.0. Only the latest published release of each crate is supported: security fixes land on `main` and ship in the next release.

## Reporting a vulnerability

Please use [private vulnerability reporting](https://github.com/masriamir/crustywad/security/advisories/new) instead of filing a public issue for a suspected vulnerability.

## Security posture

The core library confines `unsafe` code to the optional `mmap` module (behind the `mmap` feature flag) by convention — `#![deny(unsafe_code)]` is set crate-wide with a scoped `#[allow(unsafe_code)]` in `mmap.rs` only. This is a policy boundary, not a compile-time hard guarantee; a future `#![forbid(unsafe_code)]` migration is tracked in ADR-0005. All parsing and validation logic is free of `unsafe`.
