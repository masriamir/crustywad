//! Optional integration tests that inspect local `FreeDoom` fixtures.

#![cfg(feature = "freedoom-tests")]

use std::path::PathBuf;

use crustywad::Wad;

fn freedoom_dir() -> Option<PathBuf> {
    std::env::var_os("CRUSTYWAD_FREEDOOM_DIR").map(PathBuf::from)
}

#[test]
fn parses_freedoom_when_fixtures_are_available() {
    let Some(dir) = freedoom_dir() else {
        eprintln!("skipping FreeDoom fixture test: CRUSTYWAD_FREEDOOM_DIR not set");
        return;
    };

    for name in ["freedoom1.wad", "freedoom2.wad"] {
        let path = dir.join(name);
        if !path.exists() {
            eprintln!("skipping missing FreeDoom fixture: {}", path.display());
            continue;
        }

        let wad = Wad::from_path(&path).expect("fixture should parse");
        assert!(
            wad.lump_count() > 0,
            "{} should contain lumps",
            path.display()
        );
    }
}
