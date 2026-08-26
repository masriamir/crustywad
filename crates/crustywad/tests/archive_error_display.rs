//! Display-contract tests for [`ArchiveError`] and [`ArchiveWarning`].
//!
//! Mirrors `tests/error_display.rs`, which pins the same contract for
//! [`ParseError`]: `archive/error.rs`'s module docs promise that "every
//! `Display` message is a single line with no terminal escape sequences and
//! names the member it concerns", so every `#[error(...)]` arm is constructed
//! here and checked against that promise. The enums are `#[non_exhaustive]`
//! but their variants are not, so an integration test can build each one
//! directly — the only way to reach the arms that no fixture produces.

#![cfg(feature = "archive")]

use std::io;

use crustywad::archive::{ArchiveError, ArchiveWarning, ContainerKind, Method};
use crustywad::{ParseError, Wad};

/// The path used by every variant that names a member, chosen so a leaked
/// newline or ESC byte would be visible in the assertions below.
const MEMBER: &str = "maps/MAP01.wad";

/// A real [`ParseError`] for the `Wad` variant: nine bytes are too few for a
/// 12-byte WAD header, so the header read fails in either strictness mode.
fn wad_parse_error() -> ParseError {
    Wad::from_bytes(vec![0_u8; 9]).expect_err("a truncated header must fail")
}

/// Every `ArchiveError` variant, paired with the member path its message must
/// contain (`None` for the archive-level variants that name no member).
///
/// One flat table, deliberately: its whole point is that a reader can check it
/// against `archive/error.rs` variant by variant, and splitting it would hide
/// a missing arm.
#[allow(clippy::too_many_lines)]
fn every_error() -> Vec<(ArchiveError, Option<&'static str>)> {
    vec![
        (
            ArchiveError::Io {
                path: "some/where.pk3".to_string(),
                source: io::Error::new(io::ErrorKind::NotFound, "not found"),
            },
            Some("some/where.pk3"),
        ),
        (ArchiveError::NotAnArchive, None),
        (ArchiveError::UnsupportedContainer(ContainerKind::Pk7), None),
        (ArchiveError::EmptyArchive, None),
        (ArchiveError::SpannedArchive, None),
        (
            ArchiveError::CorruptDirectory {
                index: 7,
                reason: "bad central directory signature",
            },
            None,
        ),
        (
            ArchiveError::TooManyMembers {
                declared: 70_000,
                limit: 65_536,
            },
            None,
        ),
        (
            ArchiveError::UnsupportedMethod {
                path: MEMBER.to_string(),
                method: Method::Lzma,
            },
            Some(MEMBER),
        ),
        (
            ArchiveError::Encrypted {
                path: MEMBER.to_string(),
            },
            Some(MEMBER),
        ),
        (
            ArchiveError::MemberTooLarge {
                path: MEMBER.to_string(),
                declared: 1 << 30,
                limit: 1 << 20,
            },
            Some(MEMBER),
        ),
        (
            ArchiveError::SizeMismatch {
                path: MEMBER.to_string(),
                declared: 100,
                actual: Some(64),
            },
            Some(MEMBER),
        ),
        (
            ArchiveError::SizeMismatch {
                path: MEMBER.to_string(),
                declared: 100,
                actual: None,
            },
            Some(MEMBER),
        ),
        (
            ArchiveError::CorruptStream {
                path: MEMBER.to_string(),
            },
            Some(MEMBER),
        ),
        (
            ArchiveError::ChecksumMismatch {
                path: MEMBER.to_string(),
            },
            Some(MEMBER),
        ),
        (
            ArchiveError::NonAsciiName {
                path: MEMBER.to_string(),
            },
            Some(MEMBER),
        ),
        (
            ArchiveError::ForeignMember {
                path: MEMBER.to_string(),
            },
            Some(MEMBER),
        ),
        (
            ArchiveError::NotAWad {
                path: MEMBER.to_string(),
            },
            Some(MEMBER),
        ),
        (
            ArchiveError::Wad {
                path: MEMBER.to_string(),
                source: wad_parse_error(),
            },
            Some(MEMBER),
        ),
    ]
}

/// Every `ArchiveWarning` variant; all four name a member path.
fn every_warning() -> Vec<ArchiveWarning> {
    vec![
        ArchiveWarning::UnreadableMember {
            path: MEMBER.to_string(),
            reason: "unsupported compression method lzma".to_string(),
        },
        ArchiveWarning::MemberTooLarge {
            path: MEMBER.to_string(),
            declared: 1 << 30,
            limit: 1 << 20,
        },
        ArchiveWarning::NonAsciiName {
            path: MEMBER.to_string(),
        },
        ArchiveWarning::DuplicatePath {
            path: MEMBER.to_string(),
        },
    ]
}

/// Asserts the shared contract: one non-empty line, no ANSI escape byte.
fn assert_single_line(rendered: &str) {
    assert!(!rendered.is_empty(), "Display is empty");
    assert!(
        !rendered.contains('\n') && !rendered.contains('\r'),
        "Display is multi-line: {rendered:?}"
    );
    assert!(
        !rendered.contains('\u{1b}'),
        "Display contains a raw ESC byte: {rendered:?}"
    );
}

#[test]
fn every_error_variant_renders_one_line_and_names_its_member() {
    for (err, member) in every_error() {
        let rendered = err.to_string();
        assert_single_line(&rendered);
        if let Some(path) = member {
            assert!(
                rendered.contains(path),
                "message drops the member path: {rendered:?}"
            );
        }
    }
}

#[test]
fn every_warning_variant_renders_one_line_and_names_its_member() {
    for warning in every_warning() {
        let rendered = warning.to_string();
        assert_single_line(&rendered);
        assert!(
            rendered.contains(MEMBER),
            "message drops the member path: {rendered:?}"
        );
    }
}

#[test]
fn a_hostile_member_path_is_flattened_into_the_message() {
    // Zip names may hold any byte but `/`; a member called "evil\nname\x1b"
    // must not be able to forge a second log line or emit an escape sequence.
    let hostile = "evil\nname\u{1b}[31m.wad";
    for err in [
        ArchiveError::NotAWad {
            path: hostile.to_string(),
        },
        ArchiveError::CorruptStream {
            path: hostile.to_string(),
        },
    ] {
        let rendered = err.to_string();
        assert_single_line(&rendered);
        assert!(
            rendered.contains("evil name[31m.wad"),
            "path is flattened, not dropped: {rendered:?}"
        );
    }
    let warning = ArchiveWarning::DuplicatePath {
        path: hostile.to_string(),
    };
    assert_single_line(&warning.to_string());
}

#[test]
fn size_mismatch_distinguishes_a_known_length_from_an_overlong_stream() {
    let known = ArchiveError::SizeMismatch {
        path: MEMBER.to_string(),
        declared: 100,
        actual: Some(64),
    };
    assert_eq!(
        known.to_string(),
        "member `maps/MAP01.wad` decoded to 64 bytes, expected 100"
    );
    let overlong = ArchiveError::SizeMismatch {
        path: MEMBER.to_string(),
        declared: 100,
        actual: None,
    };
    assert_eq!(
        overlong.to_string(),
        "member `maps/MAP01.wad` decoded to more than the declared bytes, expected 100"
    );
}

#[test]
fn the_wad_variant_keeps_the_parse_error_reachable_as_a_source() {
    use std::error::Error as _;

    let err = ArchiveError::Wad {
        path: MEMBER.to_string(),
        source: wad_parse_error(),
    };
    assert_eq!(
        err.to_string(),
        "member `maps/MAP01.wad`: failed to parse WAD header: unexpected end of input"
    );
    let source = err.source().expect("the ParseError stays reachable");
    assert!(source.downcast_ref::<ParseError>().is_some());
}
