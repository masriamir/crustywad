//! Optional integration test over a local pk3 collection (ADR-0031).
//!
//! pk3s are third-party mods, never fetched or committed — point
//! `CRUSTYWAD_PK3_DIR` at a directory of `*.pk3` files (**absolute path**:
//! cargo runs the test binary with its CWD at the package root, so a relative
//! path silently misses; `just test-pk3` defaults to the repo's gitignored
//! `PK3-EXT/`). File names may contain spaces.
//!
//! Asserts, for every archive: it opens strictly with zero warnings, every
//! member reads (CRC-verified by the reader), every `maps/*.wad` parses and
//! yields at least one map group, and every embedded WAD parses.

#![cfg(feature = "pk3-tests")]

mod common;

use crustywad::ParseOptions;
use crustywad::archive::{Archive, MapKind};

#[test]
fn every_local_pk3_opens_reads_and_yields_its_maps() {
    let paths = common::files_with_extension("CRUSTYWAD_PK3_DIR", "pk3");
    if paths.is_empty() {
        return; // skip note already printed
    }
    let mut members_read = 0usize;
    let mut maps_parsed = 0usize;
    for path in &paths {
        let archive = Archive::from_path_with_options(path, ParseOptions::strict())
            .unwrap_or_else(|e| panic!("{}: failed to open: {e}", path.display()));
        assert!(
            archive.warnings().is_empty(),
            "{}: strict mode collects no warnings",
            path.display()
        );
        assert!(
            !archive.members().is_empty(),
            "{}: no members",
            path.display()
        );
        for member in archive.members() {
            archive
                .read(member)
                .unwrap_or_else(|e| panic!("{}: {}: {e}", path.display(), member.path()));
            members_read += 1;
        }
        for map in archive.maps() {
            if map.kind() != MapKind::Wad {
                continue;
            }
            let member = &archive.members()[map.member_index()];
            let wad = archive
                .wad(member)
                .unwrap_or_else(|e| panic!("{}: {}: {e}", path.display(), member.path()));
            assert!(
                !wad.map_groups().is_empty(),
                "{}: {} holds no map group",
                path.display(),
                member.path()
            );
            maps_parsed += 1;
        }
        for member in archive.embedded_wads() {
            archive
                .wad(member)
                .unwrap_or_else(|e| panic!("{}: embedded {}: {e}", path.display(), member.path()));
        }
        eprintln!(
            "{}: {} members, {} maps, {} embedded WADs",
            path.display(),
            archive.members().len(),
            archive.maps().len(),
            archive.embedded_wads().len()
        );
    }
    eprintln!(
        "pk3 sweep: {} archives, {members_read} members read, {maps_parsed} maps parsed",
        paths.len()
    );
    assert!(members_read > 0);
}
