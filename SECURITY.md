# Security Policy

## Supported versions

This project is pre-1.0.0. Security fixes will land on the latest `main` branch until a stable release process is established.

## Reporting a vulnerability

Please use GitHub Security Advisories or a private maintainer contact path instead of filing a public issue for a suspected vulnerability.

## Security posture

The core library restricts `unsafe` code to the optional `mmap` module (behind the `mmap` feature flag) and aims to treat malformed WAD data as untrusted input. All parsing and validation logic is free of `unsafe`.
