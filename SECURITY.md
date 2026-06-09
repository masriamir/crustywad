# Security Policy

## Supported versions

This project is pre-1.0.0. Security fixes will land on the latest `main` branch until a stable release process is established.

## Reporting a vulnerability

Please use GitHub Security Advisories or a private maintainer contact path instead of filing a public issue for a suspected vulnerability.

## Security posture

The core library confines `unsafe` code to the optional `mmap` module (behind the `mmap` feature flag) by convention — `#![deny(unsafe_code)]` is set crate-wide with a scoped `#[allow(unsafe_code)]` in `mmap.rs` only. This is a policy boundary, not a compile-time hard guarantee; a future `#![forbid(unsafe_code)]` migration is tracked in ADR-0005. All parsing and validation logic is free of `unsafe`.
