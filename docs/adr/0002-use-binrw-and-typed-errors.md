# 0002. Use `binrw` and typed library errors

- **Status:** Accepted
- Date: 2026-06-05

## Context

WAD parsing is binary-format work, and the library should remain small and memory safe.

## Decision

Use `binrw` for declarative binary record parsing and `thiserror` for typed library errors. Reserve `anyhow` for the CLI only.

## Consequences

The parser code stays explicit about endianness and record layouts, while the public library API exposes stable error types.
