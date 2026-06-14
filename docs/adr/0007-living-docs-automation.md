# 0007. Living-docs automation strategy

- Status: Proposed
- Date: 2026-06-14
- Deciders: @masriamir
- Tracking issue: https://github.com/masriamir/crustywad/issues/32

## Context and problem statement

The repository currently maintains three overlapping documentation artefacts
that describe coding conventions, workflow, and project structure:

1. `.github/copilot-instructions.md` — context injected into GitHub Copilot
   chat sessions; scoped to what a code-completion assistant needs to know.
2. `.claude/CLAUDE.md` — context injected into Claude Code sessions; contains
   the same core conventions but also Claude-specific guidance (ADR workflow,
   `just ci` reminder, `thiserror` rules, etc.).
3. `docs/design.md` and `docs/adr/` — human-readable design rationale and
   decision records; the authoritative long-form source of truth.

All three files currently describe the same conventions (error-handling rules,
lint settings, naming, strictness model, commit prefixes, CI jobs, feature
flags). When a convention changes — for example, a new lint group is added, the
MSRV bumps, or a new `just` recipe lands — every file must be updated manually.
In practice each file has already drifted: `copilot-instructions.md` omits the
`ParseWarning` rule present in `CLAUDE.md`; `CLAUDE.md` has a richer "Adding a
new lump type" checklist absent from `copilot-instructions.md`; neither
mentions the `docs/adr/` ADR-first workflow in the same detail as
`0001-record-architecture-decisions.md`.

A single-source-of-truth approach is appealing but faces a structural
constraint: each file is tailored to its consumer. `CLAUDE.md` includes
Claude-specific caveats (inner-attribute syntax, `#![doc = r#"..."]` reminder)
that would be noise in a Copilot context. `copilot-instructions.md` includes
inline code examples optimised for completion assistance that would clutter a
design document. Templating tools that generate both from one source would need
to preserve these per-consumer sections, adding complexity.

## Decision drivers

- Convention drift causes silent inconsistency: AI assistants give advice
  based on stale context and humans make wrong choices based on outdated docs.
- The project is a one-maintainer hobby repository at milestone 1; solutions
  that require significant ongoing infrastructure are a poor fit.
- Any automated enforcement must integrate naturally with the existing `just ci`
  local-check loop and the GitHub Actions CI pipeline.
- AI-context files (`CLAUDE.md`, `copilot-instructions.md`) have intentionally
  different scopes; a strategy must accommodate per-consumer customisation
  without discarding shared content.

## Considered options

### Option 1 — Manual with PR template checklist

Add a `.github/pull_request_template.md` containing a docs-sync checklist:

```markdown
- [ ] If conventions changed, updated `.github/copilot-instructions.md`
- [ ] If conventions changed, updated `.claude/CLAUDE.md`
- [ ] If design changed, updated `docs/design.md` or opened an ADR
```

No tooling required. Authors are trusted to tick the boxes.

**Pros:** Zero infrastructure; works today; respects per-consumer file
differences; no false positives.

**Cons:** Relies entirely on discipline. The checklist is easy to skip or
ignore. Past experience in the repository (the current drift) shows this
already does not happen reliably without a prompt.

### Option 2 — Single source of truth with generation

Designate one file (e.g. `docs/conventions.md`) as the canonical source.
Write a script or use a Markdown pre-processor (e.g. `mdbook`, a Jinja
template, or a small Python script) to generate `CLAUDE.md` and
`copilot-instructions.md` from it. CI fails if the generated files differ
from the committed ones.

**Pros:** True single source; drift is structurally impossible for shared
sections; enforced by CI.

**Cons:** Per-consumer sections (Claude-specific caveats, Copilot-specific
examples) require a templating mechanism. The generated files must not be
edited directly, which confuses contributors and AI assistants that read
`CLAUDE.md` and try to update it in-place. Adds a build-time dependency on
the generator and increases the risk that CI becomes flaky if the generator is
not pinned.

### Option 3 — GitHub Actions drift detector (chosen)

Extract a small set of "conventions anchor" strings — short, stable sentences
that appear in every documentation file when they describe the same rule. A CI
script (a `bash` or `python` one-liner) checks that each anchor is present in
all three files. The check runs as a new `docs-sync` step in the existing
`ci.yml` workflow and is also exercisable locally via `just docs-sync`.

Anchors are chosen from lines that are unlikely to change wording frequently,
such as:

```
"cargo clippy --workspace --all-targets --all-features -- -D warnings"
"missing_docs = \"deny\""
"ParseOptions { strictness"
"just ci"
```

When a convention changes, the author updates all files and the anchor
naturally appears in the diff. If the author updates only one file, CI fails
with a clear message naming the anchor and the files where it is missing.

Per-consumer sections (Claude-specific text, Copilot-specific text) are
intentionally excluded from anchors, so the detector never flags legitimate
divergence.

**Pros:** Simple to implement (< 50 lines of shell or Python); runs in CI with
no new external dependencies; preserves per-consumer customisation; gives a
clear, actionable failure message; does not require generated files or a
template language.

**Cons:** Anchor curation is manual — if someone renames a convention phrase,
they must also update the anchor list. A large refactor of wording could
produce many anchor failures at once. Does not catch omissions of *new*
conventions in files that predate them (only detects drift of *existing*
anchors).

### Option 4 — AI-assisted sync hook

A post-merge GitHub Actions workflow calls the Claude API (or uses GitHub
Copilot Workspace) to read all three files, identify sections that have
diverged, and either open a follow-up PR with suggested edits or post a
comment on the merge commit.

**Pros:** Handles wording differences intelligently; can spot semantic drift
that string matching misses; would keep up with organic convention evolution.

**Cons:** Requires an API key stored as a GitHub secret; response quality is
non-deterministic; could produce noisy or incorrect suggestions; the workflow
would need throttling and error handling; significantly higher maintenance
cost than the alternatives; overkill for a one-maintainer repository at this
stage.

## Decision outcome

**Chosen option: Option 3 — GitHub Actions drift detector**, because it
enforces consistency automatically in CI, is proportionate to the repository's
size and team, requires no external API dependencies, and leaves per-consumer
sections free to diverge intentionally.

**Option 1** is adopted as a complementary lightweight measure: a PR template
checklist is added for convention changes that require adding a *new* anchor,
so authors are reminded to update both the anchor list and all three files.

**Option 2** is deferred: if the documentation surface grows substantially
(e.g. a contributors' guide, a public-facing book), a template-based approach
becomes worthwhile. At that point the anchor detector can be replaced by a
generated-file check.

**Option 4** is deferred indefinitely: the value-to-complexity ratio is
unfavourable for the current team size, and the anchor detector solves the
core problem adequately.

### Implementation sketch

1. Create `scripts/check_doc_anchors.py` — reads an `anchors.txt` file (one
   anchor string per line) and checks each against
   `.github/copilot-instructions.md`, `.claude/CLAUDE.md`, and
   `docs/design.md`. Exits non-zero if any anchor is missing from any file,
   printing the anchor and the files where it is absent.
2. Add `anchors.txt` to the repository root with an initial set of ~8–12
   anchor strings drawn from the overlapping conventions sections.
3. Add a `docs-sync` recipe to `justfile`:
   ```
   docs-sync:
       python scripts/check_doc_anchors.py
   ```
4. Add a `docs-sync` job to `.github/workflows/ci.yml` that runs
   `python scripts/check_doc_anchors.py` on `ubuntu-latest` with no
   Rust toolchain required.
5. Add `.github/pull_request_template.md` with a checklist reminding authors
   to update the anchor list when renaming a convention phrase.

### Consequences

- Good: convention drift is caught in CI before it reaches `main`.
- Good: per-consumer sections (`CLAUDE.md` Claude-specific caveats, Copilot
  completion examples) remain intentionally different without triggering false
  failures.
- Good: no new runtime dependencies; the check runs in any environment with
  Python 3.
- Neutral: an initial curation pass is needed to select and commit the anchor
  strings. This is a one-time cost.
- Neutral: anchor strings must be updated whenever a matched convention phrase
  changes wording. The PR template checklist mitigates the risk of forgetting.
- Bad: the detector cannot catch a *new* convention that is added to only one
  file and has no anchor yet. This is mitigated by the PR template reminder to
  add an anchor for each new shared convention.
- Future: if `release-plz` bumps MSRV or a new `just` recipe is added as part
  of a release, the anchor update should be included in the same PR that
  introduces the convention change.

## More information

- Tracking issue: https://github.com/masriamir/crustywad/issues/32
- Implementation will be tracked in a follow-up issue once this ADR is accepted.
- ADR template: [`template.md`](template.md)
- ADR index: [`README.md`](README.md)
