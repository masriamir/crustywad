//! Integration tests for the `archive` feature: pk3 (zip) reading through
//! the public `crustywad::archive::Archive` API, in both strictness modes.

#![cfg(feature = "archive")]

mod common;

use crustywad::ParseOptions;
use crustywad::archive::{Archive, ArchiveError, ContainerKind, Member};

/// Both strictness modes, so every negative test runs under each.
fn both_modes() -> [ParseOptions; 2] {
    [ParseOptions::strict(), ParseOptions::lenient()]
}

#[test]
fn random_bytes_are_not_an_archive() {
    for options in both_modes() {
        let err = Archive::from_bytes_with_options(b"IWAD\0\0\0\0\0\0\0\0".to_vec(), options)
            .expect_err("a WAD is not an archive");
        assert!(matches!(err, ArchiveError::NotAnArchive), "{err}");
    }
}

#[test]
fn empty_archive_signature_is_rejected_by_name() {
    for options in both_modes() {
        let err = Archive::from_bytes_with_options(b"PK\x05\x06".to_vec(), options)
            .expect_err("empty archive");
        assert!(matches!(err, ArchiveError::EmptyArchive), "{err}");
    }
}

#[test]
fn spanned_archive_signature_is_rejected_by_name() {
    for options in both_modes() {
        let err = Archive::from_bytes_with_options(b"PK\x07\x08".to_vec(), options)
            .expect_err("spanned archive");
        assert!(matches!(err, ArchiveError::SpannedArchive), "{err}");
    }
}

#[test]
fn pk7_is_detected_and_named_in_the_error() {
    for options in both_modes() {
        let err = Archive::from_bytes_with_options(b"7z\xbc\xaf\x27\x1c\0\0".to_vec(), options)
            .expect_err("7z container");
        assert!(
            matches!(err, ArchiveError::UnsupportedContainer(ContainerKind::Pk7)),
            "{err}"
        );
        assert!(
            err.to_string().contains("pk7"),
            "message names the format: {err}"
        );
    }
}

#[test]
fn errors_display_on_a_single_line() {
    let err = Archive::from_bytes(b"PK\x07\x08".to_vec()).unwrap_err();
    assert!(!err.to_string().contains('\n'));
}

#[test]
fn zip_builder_round_trips_through_python_compatible_layout() {
    // A stored + a deflated member; the archive must open and list both.
    let zip = common::ZipBuilder::new()
        .stored("MAPINFO.txt", b"map MAP01 \"Entry\"")
        .deflate("sprites/TROOA1.png", &[0_u8; 512])
        .build();
    assert!(zip.starts_with(b"PK\x03\x04"));
    let archive = Archive::from_bytes(zip).expect("opens");
    let paths: Vec<&str> = archive.members().iter().map(Member::path).collect();
    assert_eq!(paths, ["MAPINFO.txt", "sprites/TROOA1.png"]);
}
