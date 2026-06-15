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
| [0008](0008-cli-ux-architecture.md) | `cwad` CLI UX and architecture | Proposed |

<!-- Add new ADRs to the table above in ascending order. -->
