## Summary

- 

## Validation

- [ ] `just ci` passed locally on a toolchain matching CI's `stable` — or the documented tier for this branch: `just ci-xtask` (xtask-only), `just ci-docs` (Markdown-only outside `docs/guide/src/`), `just ci-full` (touches `Cargo.toml`/`Cargo.lock`, or precedes a release)

## Convention changes (docs-sync checklist)

If this PR renames or introduces a shared convention phrase:

- [ ] Updated the phrase in `AGENTS.md` — **but** if it lives inside a synced `meta:` block (`language-en-us`, `commit-conventions`, `branch-naming`, `copilot-review-loop`, `board-transitions`), edit the canonical source in `masriamir/.github` and bump the `.meta-manifest.toml` pin instead; editing the marked region directly makes `meta-check` fail
- [ ] Updated the phrase in `docs/design.md`
- [ ] Updated `anchors.txt` (added new anchor or updated existing wording)
- [ ] `just docs-sync` (and `just meta-check` if a synced block changed) passes locally

If this PR bumps the crate's **minor** version (a `release-plz` release PR):

- [ ] Updated the `crustywad = "X.Y.Z"` pins in `README.md` and `docs/guide/src/` (a 0.x caret is minor-pinned, so after a minor bump a stale pin stops resolving for readers; `just docs-sync` enforces this. Patch bumps still resolve and need no change.)
