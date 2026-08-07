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

use crustywad::{ParseError, ParseOptions, Wad};

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
fn unrecognized_root_cause_falls_back_to_generic_phrase() {
    // `Backtrace::new` guarantees `error` is never itself a `Backtrace`, so
    // `root_cause()` normally cannot return one. Forcing that shape through
    // the public `error` field proves the mapping degrades to its generic
    // fallback phrase instead of leaking binrw's multi-line rendering.
    let mut outer = binrw::error::Backtrace::new(eof_error(), Vec::new());
    *outer.error = binrw::Error::Backtrace(binrw::error::Backtrace::new(eof_error(), Vec::new()));
    let err = ParseError::Header(binrw::Error::Backtrace(outer));
    assert_eq!(err.to_string(), "failed to parse WAD header: binary read error");
}

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

// ---------------------------------------------------------------------------
// Other variants that interpolate input-derived strings
// ---------------------------------------------------------------------------

/// Bytes 4–11 of a structurally valid, empty WAD (numlumps 0, infotableofs 12).
fn empty_wad_tail() -> [u8; 8] {
    let mut tail = [0u8; 8];
    tail[4..8].copy_from_slice(&12i32.to_le_bytes());
    tail
}

#[test]
fn invalid_magic_display_escapes_control_bytes() {
    let mut bytes = vec![0x1b, 0x5b, 0x31, 0x6d]; // "\x1b[1m" — an ANSI bold sequence
    bytes.extend_from_slice(&empty_wad_tail());
    let err = Wad::from_bytes(bytes).expect_err("unknown magic must fail in strict mode");
    assert!(matches!(err, ParseError::InvalidMagic { .. }));
    let msg = err.to_string();
    assert!(
        !msg.contains('\u{1b}'),
        "Display contains a raw ESC byte: {msg:?}"
    );
    assert!(!msg.contains('\n'), "Display is multi-line: {msg:?}");
    assert_eq!(msg, "invalid WAD magic `\\u{1b}[1m`");
}

#[test]
fn invalid_magic_warning_escapes_control_bytes() {
    let mut bytes = vec![b'W', b'A', b'D', 0x0a]; // newline inside the magic
    bytes.extend_from_slice(&empty_wad_tail());
    let wad = Wad::from_bytes_with_options(bytes, ParseOptions::lenient())
        .expect("lenient mode must recover from unknown magic");
    let warning = wad
        .warnings()
        .first()
        .expect("lenient parse must record an InvalidMagic warning");
    let msg = warning.to_string();
    assert!(!msg.contains('\n'), "Display is multi-line: {msg:?}");
    assert_eq!(msg, "unrecognized WAD magic `WAD\\n`");
}

#[test]
fn io_path_display_is_flattened_to_one_line() {
    let err = ParseError::Io {
        path: "evil\nname\u{1b}.wad".to_owned(),
        source: io::Error::new(io::ErrorKind::NotFound, "not found"),
    };
    let msg = err.to_string();
    assert!(!msg.contains('\n'), "Display is multi-line: {msg:?}");
    assert!(
        !msg.contains('\u{1b}'),
        "Display contains a raw ESC byte: {msg:?}"
    );
    assert_eq!(msg, "failed to read `evil name.wad`: not found");
}

#[test]
fn io_message_with_escape_sequence_is_stripped() {
    let err = ParseError::Header(binrw::Error::Io(io::Error::other("\u{1b}[1mloud\u{1b}[22m")));
    let msg = err.to_string();
    assert!(
        !msg.contains('\u{1b}'),
        "Display contains a raw ESC byte: {msg:?}"
    );
    assert_eq!(msg, "failed to parse WAD header: I/O error: [1mloud[22m");
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
