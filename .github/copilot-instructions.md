# GitHub Copilot Instructions for `crustywad`

`crustywad` is a Rust workspace for safe, documented Doom WAD file I/O (Rust 2024, MSRV 1.94.0, dual-licensed MIT OR Apache-2.0).

**The conventions to review against live in [`AGENTS.md`](../AGENTS.md), which you also read** — code style, error handling (`thiserror`), safety (`#![deny(unsafe_code)]` outside `mmap.rs`), lints (`clippy::pedantic`, warnings-as-errors), the strictness model, testing practices, and the American-English spelling rule. Don't restate them here; review changes for adherence to that file.

## Review focus

- Public fallible functions need a `# Errors` doc section; public items need doc comments (`missing_docs = "deny"`).
- New parse/assembly surfaces must meet the ADR-0016 hardening checklist (bounded allocation, no unbounded recursion, a `cargo-fuzz` target, both `Strictness` modes non-panicking).
- Map-record field types are layout-critical (signed vs unsigned, LE) — verify against the WAD spec.

## Known false positives (do not flag)

- **"Freedoom"** is the correct project name; suggestions to write it "FreeDoom" are always wrong.
- A **wildcard arm on a `#[non_exhaustive]` match** is required by the compiler, not dead code.
- The **American-English rule lists counter-examples**, so its own text necessarily contains the spellings it names as the pattern; backticked code spans and third-party vocabulary (e.g. the Actions `cancelled` literal) are exempt (see `AGENTS.md`).
- Speculative clippy claims: verify against a real `cargo clippy --workspace --all-targets --all-features` run before flagging.
