## Summary

- 

## Validation

- [ ] `cargo build --workspace --all-features`
- [ ] `cargo test --workspace --all-features`
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] `cargo fmt --all --check`
- [ ] `cargo doc --workspace --all-features --no-deps`

## Convention changes (docs-sync checklist)

If this PR renames or introduces a shared convention phrase:

- [ ] Updated the phrase in `.github/copilot-instructions.md`
- [ ] Updated the phrase in `.claude/CLAUDE.md`
- [ ] Updated the phrase in `docs/design.md`
- [ ] Updated `anchors.txt` (added new anchor or updated existing wording)
- [ ] `just docs-sync` passes locally

If this PR bumps the crate's **minor** version (a `release-plz` release PR):

- [ ] Updated the `crustywad = "X.Y"` pins in `README.md` and `docs/guide/src/` (a 0.x caret is minor-pinned, so a stale pin stops resolving for readers; `just docs-sync` enforces this)

## Roadmap milestones

- [ ] Header and directory parsing
- [ ] Map lump records
- [ ] Real-world Freedoom fixture coverage
- [ ] Follow-up TODOs filed for deferred work
