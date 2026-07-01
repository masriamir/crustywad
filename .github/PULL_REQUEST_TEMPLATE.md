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

## Roadmap milestones

- [ ] Header and directory parsing
- [ ] Map lump records
- [ ] Real-world Freedoom fixture coverage
- [ ] Follow-up TODOs filed for deferred work
