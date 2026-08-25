@AGENTS.md

# CLAUDE.md — crustywad

Claude-only operating notes for this repo. The shared, tool-neutral guidance (project layout,
conventions, workflow, CI) lives in [`AGENTS.md`](AGENTS.md), imported above; this file adds only
what is specific to Claude driving the work here.

## Project tracking

Work is tracked on the **[Crustywad](https://github.com/users/masriamir/projects/5)** GitHub Project board — the single source of "what to pick up next". Every roadmap issue lives there with three planning dimensions:

- **Status** (workflow stage): `Backlog` → `Ready` → `In progress` → `In review` → `Done`. Pull work from the `Ready` column. Most transitions are agent-driven (below); only `Done` is set automatically by the board on merge/close.
- **Horizon** (priority bucket): `Now` / `Next` / `Later`. This carries planning intent for issues that have no milestone yet (e.g. the editor epic #18 and its long-horizon spikes); it replaces the former `Short Term` / `Future` milestones.
- **Milestone** (release scope): milestones are **release-scoped and scope-named** (`Audio layer`, `Nodebuilder`, …) — each the set of epics/issues intended to ship together. Names deliberately do **not** encode version numbers: crates version independently (ADR-0011 §3) and `release-plz` derives the actual versions from Conventional Commits **at ship time**, so a version-shaped milestone name is a prediction that cannot reliably be kept (the `v0.7.0` milestone shipped as crustywad 0.6.1 / cli 0.3.1 on 2026-07-19, prompting this policy). At closeout, record the shipped crate versions in the milestone description. Historical milestones `v0.1.0`–`v0.6.0` keep their version-shaped names.

**Epics** (the `epic` label, e.g. #17, #18) use GitHub **native sub-issues**, so they show automatic progress rollup — attach each new format/feature issue as a sub-issue of its epic.

Typical flow: pick a `Ready` + `Now` item → begin planning (`In progress`) → branch by its issue number → open a PR (`In review`) → merge closes the issue and sets `Done`.

### Issue status transitions (agent-driven)

<!-- >>> meta:board-transitions -->
Move the GitHub Project board yourself as work progresses and **announce each change** in your reply ("moved #201 → In progress") rather than asking first — board edits are internal and easily reversed.

| Transition | Trigger |
|---|---|
| `Backlog → Ready` | the user says they want to start work on an issue |
| `Ready → In progress` | you begin brainstorming or drafting a plan — **before** any branch or code |
| `In progress → In review` | the PR opens |
| `In review → Done` | the PR merges/closes — **board-automated**, not manual |

`In review` holds through the entire review loop, until human review and merge. Transitions apply only to an issue that is on the board; if one exists but isn't on the board, add it first. Epics carry an **aggregate** Status: `In progress` when their first sub-issue starts work, and `Done` (board-automated) only when every sub-issue closes — set the epic's Status yourself and announce it, since GitHub rolls up completion progress but not the Status field.
<!-- <<< meta:board-transitions -->

The `gh project item-edit` recipe (Status field + option IDs) lives in the `reference-project-board` memory; it needs the `read:project,project` scope — if that scope is missing, surface it and ask the user to grant it rather than silently skipping the transition. An epic's Horizon (`Now`/`Next`/`Later`) carries its planning intent, separate from the aggregate Status above.

### Milestone closeout (propose-and-confirm)

A milestone is **complete** when BOTH conditions hold:

1. **All milestone items closed** — the milestone reports `open_issues == 0`. That is GitHub's milestone counter, which covers every assigned item (issues *and* any pull requests), not just issues, and is distinct from the board's `Done` Status field — closing an item typically sets both, but the milestone counter is the signal here.
2. **Shipped** — a `release-plz` PR (`chore: release`) has merged at or after the last milestone item closed. This is tag-agnostic: milestones are scope-named, so no version correspondence is expected — any release PR merged on/after that final close is the ship signal. When closing, add the shipped crate versions to the milestone description.

GitHub never auto-closes milestones. Unlike the agent-driven board Status transitions above, milestone closeout is **propose-and-confirm**: when both conditions hold for an open milestone, surface it and **ask before closing**. On approval:

```bash
gh api -X PATCH repos/masriamir/crustywad/milestones/<milestone_number> -f state=closed
```

Closure is reversible (`-f state=open`). To find `<milestone_number>` (and its issue counts), the companion listing recipe — `gh api "repos/masriamir/crustywad/milestones?state=all"` — lives in the `reference-project-board` memory.
