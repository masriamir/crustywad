//! Display-contract tests for [`ParseError`] (#416).
//!
//! `ParseError`'s docs direct callers to "fall back to the `Display` message
//! for logging or user-facing output", so every variant must render as a
//! single human-readable line.  The `Header` and `Directory` variants wrap a
//! [`binrw::Error`], whose own rendering is a multi-line backtrace report with
//! box-drawing characters, optional ANSI escapes, and machine-local source
//! paths — none of which may leak into `Display`.

use std::error::Error as _;
use std::io;

use crustywad::{ParseError, Wad};

/// A `binrw` end-of-input error, as produced when a read hits EOF.
fn eof_error() -> binrw::Error {
    binrw::Error::Io(io::Error::new(
        io::ErrorKind::UnexpectedEof,
        "failed to fill whole buffer",
    ))
}

// ---------------------------------------------------------------------------
// The #416 repro: a truncated header through the public parse path
// ---------------------------------------------------------------------------

#[test]
fn truncated_header_display_is_single_line_without_ansi() {
    let err = Wad::from_bytes(vec![0u8; 9]).expect_err("truncated header must fail");
    let msg = err.to_string();
    assert!(
        !msg.contains('\u{1b}'),
        "Display contains ANSI escapes: {msg:?}"
    );
    assert!(!msg.contains('\n'), "Display is multi-line: {msg:?}");
    assert_eq!(msg, "failed to parse WAD header: unexpected end of input");
}

#[test]
fn truncated_header_source_preserves_binrw_error() {
    let err = Wad::from_bytes(vec![0u8; 9]).expect_err("truncated header must fail");
    let source = err
        .source()
        .expect("binrw detail must stay reachable via source()");
    let binrw_err = source
        .downcast_ref::<binrw::Error>()
        .expect("source must be the underlying binrw::Error");
    assert!(binrw_err.is_eof());
}

// ---------------------------------------------------------------------------
// Directory variant (constructed directly: the strict parse path bounds-checks
// the directory span before binrw ever reads an entry)
// ---------------------------------------------------------------------------

#[test]
fn directory_display_is_single_line() {
    let err = ParseError::Directory {
        index: 3,
        source: eof_error(),
    };
    assert_eq!(
        err.to_string(),
        "failed to parse WAD directory entry 3: unexpected end of input"
    );
}

// ---------------------------------------------------------------------------
// Root-cause mapping, one arm at a time
// ---------------------------------------------------------------------------

#[test]
fn backtrace_wrapped_error_displays_its_root_cause() {
    let wrapped = binrw::Error::Backtrace(binrw::error::Backtrace::new(eof_error(), Vec::new()));
    let err = ParseError::Header(wrapped);
    assert_eq!(
        err.to_string(),
        "failed to parse WAD header: unexpected end of input"
    );
}

#[test]
fn non_eof_io_error_displays_its_message() {
    let err = ParseError::Header(binrw::Error::Io(io::Error::new(
        io::ErrorKind::PermissionDenied,
        "denied",
    )));
    assert_eq!(
        err.to_string(),
        "failed to parse WAD header: I/O error: denied"
    );
}

#[test]
fn bad_magic_displays_offset_only() {
    let err = ParseError::Header(binrw::Error::BadMagic {
        pos: 0x10,
        found: Box::new(1234u32),
    });
    assert_eq!(
        err.to_string(),
        "failed to parse WAD header: bad magic at offset 0x10"
    );
}

#[test]
fn assert_fail_message_is_flattened_to_one_line() {
    let err = ParseError::Header(binrw::Error::AssertFail {
        pos: 2,
        message: "first line\nsecond line".to_owned(),
    });
    assert_eq!(
        err.to_string(),
        "failed to parse WAD header: first line second line at offset 0x2"
    );
}

#[test]
fn custom_error_displays_offset_only() {
    let err = ParseError::Header(binrw::Error::Custom {
        pos: 4,
        err: Box::new("opaque payload"),
    });
    assert_eq!(
        err.to_string(),
        "failed to parse WAD header: custom parser error at offset 0x4"
    );
}

#[test]
fn no_variant_match_displays_offset() {
    let err = ParseError::Header(binrw::Error::NoVariantMatch { pos: 8 });
    assert_eq!(
        err.to_string(),
        "failed to parse WAD header: no matching variant at offset 0x8"
    );
}

#[test]
fn enum_errors_collapse_without_joining_sub_errors() {
    let err = ParseError::Header(binrw::Error::EnumErrors {
        pos: 8,
        variant_errors: vec![("VariantA", eof_error()), ("VariantB", eof_error())],
    });
    let msg = err.to_string();
    assert!(!msg.contains('\n'), "Display is multi-line: {msg:?}");
    assert_eq!(
        msg,
        "failed to parse WAD header: no matching variant at offset 0x8"
    );
}
