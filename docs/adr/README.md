# Architecture Decision Records

This directory contains the Architecture Decision Records (ADRs) for `crustywad`.

An ADR captures a single architectural decision, its context, and its
consequences. Per the project's planning process, **every roadmap feature that
requires design work must be preceded by a spike and an accepted ADR before
implementation begins.**

## Process

1. Copy [`0000-adr-template.md`](0000-adr-template.md) to a new file named
   `NNNN-short-title.md`, where `NNNN` is the next zero-padded sequence number.
2. Fill in the sections. Start the ADR in the `Proposed` status.
3. Open a pull request for discussion and link the ADR to its tracking issue /
   spike.
4. Once accepted, set the status to `Accepted` and merge. Implementation may
   then begin.
5. If a later decision overrides this one, set the status to
   `Superseded by ADR-NNNN` and link forward.

## Status values

- `Proposed` — under discussion
- `Accepted` — agreed and in force
- `Rejected` — considered and declined
- `Deprecated` — no longer relevant
- `Superseded by ADR-NNNN` — replaced by a newer decision

## Index

| ADR | Title | Status |
| ---: | --- | --- |
| [0001](0001-record-architecture-decisions.md) | Record architecture decisions | Accepted |
| [0002](0002-use-binrw-and-typed-errors.md) | Use `binrw` and typed library errors | Accepted |
| [0003](0003-default-to-strict-parsing.md) | Default to strict parsing | Accepted |
| [0004](0004-parse-api-and-safety-contracts.md) | Parse API and safety contracts for `Wad` and map records | Accepted |
| [0005](0005-isolate-unsafe-code-in-platform-crate.md) | Isolate unsafe code in a dedicated `crustywad-platform` workspace crate | Proposed |
| [0006](0006-wad-write-design.md) | WAD write design | Accepted |
| [0007](0007-living-docs-automation.md) | Living-docs automation strategy | Accepted |
| [0008](0008-cli-ux-architecture.md) | `cwad` CLI UX and architecture | Accepted |
| [0009](0009-cargo-fuzz-harness.md) | `cargo-fuzz` harness for WAD parser | Accepted |
| [0010](0010-proptest-strategy.md) | Proptest invariant testing strategy | Accepted |
| [0011](0011-publish-workflow.md) | crates.io publish workflow | Accepted |
| [0012](0012-criterion-benchmarking-infrastructure.md) | Criterion benchmarking infrastructure, GitHub Pages trend reporting, and CI integration | Accepted |
| [0013](0013-lump-by-name-lookup-strategy.md) | `lump_by_name` lookup strategy | Accepted |
| [0014](0014-multi-format-map-support-strategy.md) | Multi-format map support strategy | Accepted |
| [0015](0015-assembled-map-graph-model.md) | Assembled map graph model | Accepted |
| [0016](0016-parser-hardening-policy.md) | Parser and assembly hardening policy | Accepted |

<!-- Add new ADRs to the table above in ascending order. -->
