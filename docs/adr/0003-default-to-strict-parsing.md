# ADR-0003: Default to strict parsing

- **Status:** Accepted
- **Date:** 2026-06-05
- **Deciders:** @masriamir
- **Tracking issue:** N/A (established during initial workspace scaffold)

## Context

The library must support both strict validation and lenient recovery, but the default constructor still needs a predictable behavior.

## Decision

`Wad::from_bytes` and `Wad::from_path` default to `Strictness::Strict`. Callers that want best-effort parsing must opt in through `ParseOptions`.

## Consequences

The default API surface is explicit and fail-fast, while lenient mode remains available for real-world PWAD inspection tools and future migration utilities.
