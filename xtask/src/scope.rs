//! Traversal scope: include/skip/triage per DESIGN.md §4.2.
//!
//! Scope decision: everything map-bearing. The table below is the
//! spike-verified §4.2 call; `Triage` marks roots the spike surfaced but
//! nobody has inspected yet — they are skipped *loudly* until DESIGN §4.2
//! records an include/skip decision for them.

/// What the harvest does with a directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeDecision {
    /// Enumerate and enrich.
    Include,
    /// Deliberately out of scope (not map-bearing).
    Skip,
    /// Untriaged — skipped, but surfaced in the run summary so the §4.2
    /// include/skip call gets made.
    Triage,
}

/// Map-bearing roots (§4.2), also the BFS-fallback seeds.
pub const BFS_ROOTS: [&str; 5] = ["levels/", "deathmatch/", "combos/", "prefabs/", "themes/"];

const SKIP_ROOTS: [&str; 14] = [
    "music/",
    "sounds/",
    "utils/",
    "lmps/",
    "docs/",
    "graphics/",
    "source/",
    "idstuff/",
    "skins/",
    "misc/",
    "historic/",
    "roguestuff/",
    "incoming/",
    "newstuff/",
];

/// Decide scope for a normalized (trailing-slash) directory path.
///
/// The archive root (`""`) is `Skip` — it holds no map archives, only
/// housekeeping files like `ls-laR.gz`.
///
/// **Include** roots: `levels/`, `deathmatch/`, `combos/`, `prefabs/`, `themes/`.
/// Exception: `levels/reviews/` is `Skip` (text-only), but subtrees like `levels/doom/reviews/` remain `Include`.
///
/// **Skip** roots: `music/`, `sounds/`, `utils/`, `lmps/`, `docs/`, `graphics/`, `source/`, `idstuff/`, `skins/`, `misc/`, `historic/`, `roguestuff/`, `incoming/`, `newstuff/`.
///
/// **Triage** (skip + surface loudly): any top-level directory not in any list (future-proofing — a new root
/// must be noticed, not silently skipped). Resolved roots (`misc/`, `historic/`, `roguestuff/` via 2026-08-16
/// #407 decision; `incoming/`, `newstuff/` — staging directories — via 2026-08-18 #408 decision) are now in Skip.
pub fn decide(dir: &str) -> ScopeDecision {
    if dir.is_empty() {
        return ScopeDecision::Skip;
    }
    // levels/reviews/ is text-only (§4.2) — the one exception inside an
    // include root.
    if dir == "levels/reviews/" || dir.starts_with("levels/reviews/") {
        return ScopeDecision::Skip;
    }
    if BFS_ROOTS.iter().any(|r| dir == *r || dir.starts_with(r)) {
        return ScopeDecision::Include;
    }
    if SKIP_ROOTS.iter().any(|r| dir == *r || dir.starts_with(r)) {
        return ScopeDecision::Skip;
    }
    // Known-but-untriaged roots, and anything the spike never saw.
    ScopeDecision::Triage
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn include_roots_and_subtrees() {
        for dir in [
            "levels/",
            "levels/doom/",
            "levels/doom/0-9/",
            "levels/strife/",
            "levels/doom64/",
            "levels/doom2/Ports/megawads/",
            "deathmatch/",
            "combos/",
            "prefabs/",
            "themes/x-z/",
        ] {
            assert_eq!(decide(dir), ScopeDecision::Include, "{dir}");
        }
    }

    #[test]
    fn levels_reviews_is_text_only_skip() {
        assert_eq!(decide("levels/reviews/"), ScopeDecision::Skip);
        assert_eq!(decide("levels/reviews/2003/"), ScopeDecision::Skip);
        // Only that subtree — a hypothetical file dir named reviews elsewhere is fine.
        assert_eq!(decide("levels/doom/reviews/"), ScopeDecision::Include);
    }

    #[test]
    fn skip_roots() {
        for dir in [
            "music/",
            "sounds/",
            "utils/",
            "lmps/",
            "docs/",
            "graphics/",
            "source/",
            "idstuff/",
            "skins/",
            "utils/exes/",
        ] {
            assert_eq!(decide(dir), ScopeDecision::Skip, "{dir}");
        }
    }

    #[test]
    fn resolved_triage_roots_are_skipped() {
        for dir in [
            "misc/",
            "historic/",
            "roguestuff/",
            "misc/sub/",
            "historic/x/",
            "incoming/",
            "newstuff/",
            "incoming/sub/",
            "newstuff/sub/",
        ] {
            assert_eq!(decide(dir), ScopeDecision::Skip, "{dir}");
        }
    }

    #[test]
    fn triage_roots_and_unknowns() {
        // Any top-level directory not in any list remains Triage until a
        // decision is recorded.
        assert_eq!(decide("brandnew/"), ScopeDecision::Triage);
    }

    #[test]
    fn archive_root_is_skip() {
        assert_eq!(decide(""), ScopeDecision::Skip);
    }

    #[test]
    fn bfs_roots_match_include_table() {
        assert_eq!(
            BFS_ROOTS,
            ["levels/", "deathmatch/", "combos/", "prefabs/", "themes/"]
        );
        for root in BFS_ROOTS {
            assert_eq!(decide(root), ScopeDecision::Include);
        }
    }
}
