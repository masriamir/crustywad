# ADR-0001: Record architecture decisions

- **Status:** Accepted
- **Date:** 2026-06-09
- **Deciders:** @masriamir
- **Tracking issue:** N/A (establishes the ADR process itself)

## Context and problem statement

`crustywad` has a growing roadmap of significant features — WAD writing, CLI
expansion, multi-format support, release engineering, and a potential
GUI/editor. Many of these carry non-trivial design trade-offs. We need a
lightweight, durable way to capture *why* decisions were made so future
contributors (human or AI) can understand the reasoning without archaeology
through pull requests.

## Decision drivers

- Decisions should be discoverable and version-controlled alongside the code.
- The process must be lightweight enough that it is actually used.
- Roadmap epics already mandate "a spike + ADR before implementation."

## Considered options

1. Architecture Decision Records (ADRs) as Markdown files in `docs/adr/`.
2. A wiki page per decision.
3. Decisions captured only in pull request descriptions.

## Decision outcome

Chosen option: "ADRs as Markdown files in `docs/adr/`", because they live with
the code, are reviewable through the normal PR flow, and are trivially
greppable. This follows Michael Nygard's well-established ADR pattern.

### Consequences

- Good, because every significant decision has a permanent, linkable home.
- Good, because spikes can land an ADR as their concrete deliverable.
- Neutral, because contributors must remember to author one — reinforced by the
  epic/spike issues that each call out "requires an ADR."

## More information

- Template: [`0000-adr-template.md`](0000-adr-template.md)
- Index: [`README.md`](README.md)
- Background: Michael Nygard, "Documenting Architecture Decisions" (2011).
