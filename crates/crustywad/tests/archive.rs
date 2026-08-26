//! Integration tests for the `archive` feature: pk3 (zip) reading through
//! the public `crustywad::archive::Archive` API, in both strictness modes.

#![cfg(feature = "archive")]

mod common;

use crustywad::archive::{
    Archive, ArchiveError, ArchiveWarning, ContainerKind, Member, Method, Namespace,
};
use crustywad::{Limits, ParseOptions};

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

fn with_limits(options: ParseOptions, limits: Limits) -> ParseOptions {
    let mut o = options;
    o.limits = limits;
    o
}

#[test]
fn lists_members_with_namespace_short_name_method_and_sizes() {
    let zip = common::ZipBuilder::new()
        .stored("MAPINFO.txt", b"x")
        .deflate("SPRITES/trooa1.png", &[7_u8; 300])
        .stored("maps/MAP01.wad", b"PWAD")
        .stored("zscript/actors.zs", b"")
        .build();
    for options in both_modes() {
        let archive = Archive::from_bytes_with_options(zip.clone(), options).expect("opens");
        let m = archive.members();
        assert_eq!(m.len(), 4);
        assert_eq!(
            (m[0].path(), m[0].namespace(), m[0].short_name()),
            ("MAPINFO.txt", Namespace::Global, Some("MAPINFO"))
        );
        assert_eq!(
            (m[1].path(), m[1].namespace(), m[1].short_name()),
            ("SPRITES/trooa1.png", Namespace::Sprites, Some("TROOA1"))
        );
        assert_eq!(m[1].method(), Method::Deflate);
        assert_eq!(m[1].size(), 300);
        assert!(
            m[1].compressed_size() < 300,
            "deflate shrinks 300 identical bytes"
        );
        assert_eq!(
            (m[2].namespace(), m[2].short_name()),
            (Namespace::Hidden, None)
        );
        assert_eq!(m[3].method(), Method::Stored);
        assert_eq!(m[3].index(), 3);
        assert!(archive.warnings().is_empty());
    }
}

#[test]
fn directory_entries_are_dropped() {
    let zip = common::ZipBuilder::new()
        .stored("sprites/", b"")
        .stored("sprites/TROOA1.png", b"x")
        .build();
    let archive = Archive::from_bytes(zip).expect("opens");
    assert_eq!(archive.members().len(), 1);
    assert_eq!(archive.members()[0].path(), "sprites/TROOA1.png");
}

#[test]
fn member_lookup_is_case_insensitive_and_accepts_backslashes() {
    let zip = common::ZipBuilder::new()
        .stored("maps/MAP01.wad", b"PWAD")
        .build();
    let archive = Archive::from_bytes(zip).expect("opens");
    assert!(archive.member("MAPS/map01.WAD").is_some());
    assert!(archive.member("maps\\MAP01.wad").is_some());
    assert!(archive.member("/maps/MAP01.wad").is_some());
    assert!(archive.member("maps/MAP02.wad").is_none());
}

#[test]
fn eocd_is_found_behind_a_maximal_comment_but_not_beyond_it() {
    let found = common::ZipBuilder::new()
        .stored("a.txt", b"a")
        .comment(&vec![b'c'; 65_535])
        .build();
    assert_eq!(
        Archive::from_bytes(found).expect("opens").members().len(),
        1
    );
    let mut lost = common::ZipBuilder::new().stored("a.txt", b"a").build();
    lost.extend(std::iter::repeat_n(b'c', 65_536));
    for options in both_modes() {
        let err = Archive::from_bytes_with_options(lost.clone(), options).unwrap_err();
        assert!(matches!(err, ArchiveError::NotAnArchive), "{err}");
    }
}

#[test]
fn zip64_records_are_honored() {
    let zip = common::ZipBuilder::new()
        .zip64(true)
        .stored("MAPINFO.txt", b"hello")
        .deflate("graphics/A.png", &[1_u8; 100])
        .build();
    let archive = Archive::from_bytes(zip).expect("opens");
    assert_eq!(archive.members().len(), 2);
    assert_eq!(archive.members()[0].size(), 5);
    assert_eq!(archive.members()[1].size(), 100);
}

#[test]
fn declared_member_count_over_the_limit_is_refused_in_both_modes() {
    let zip = common::ZipBuilder::new()
        .stored("a.txt", b"a")
        .stored("b.txt", b"b")
        .build();
    for options in both_modes() {
        let options = with_limits(options, Limits::new().with_max_archive_members(1));
        let err = Archive::from_bytes_with_options(zip.clone(), options).unwrap_err();
        assert!(
            matches!(
                err,
                ArchiveError::TooManyMembers {
                    declared: 2,
                    limit: 1
                }
            ),
            "{err}"
        );
    }
}

#[test]
fn a_count_larger_than_the_directory_is_corrupt_in_both_modes() {
    let zip = common::ZipBuilder::new()
        .stored("a.txt", b"a")
        .entry_count_override(50)
        .build();
    for options in both_modes() {
        let err = Archive::from_bytes_with_options(zip.clone(), options).unwrap_err();
        assert!(
            matches!(err, ArchiveError::CorruptDirectory { .. }),
            "{err}"
        );
    }
}

#[test]
fn truncated_central_directory_is_corrupt() {
    let zip = common::ZipBuilder::new().stored("abcdef.txt", b"a").build();
    // Damage the central directory entry's name length so it overruns.
    let cd = zip
        .windows(4)
        .position(|w| w == [0x50, 0x4b, 0x01, 0x02])
        .unwrap();
    let mut bad = zip.clone();
    bad[cd + 28] = 0xFF; // name length low byte
    bad[cd + 29] = 0x7F;
    for options in both_modes() {
        let err = Archive::from_bytes_with_options(bad.clone(), options).unwrap_err();
        assert!(
            matches!(err, ArchiveError::CorruptDirectory { index: 0, .. }),
            "{err}"
        );
    }
}

#[test]
fn unsupported_methods_are_named_strict_errors_and_lenient_warnings() {
    for (code, method) in [
        (1_u16, Method::Shrink),
        (6, Method::Implode),
        (12, Method::Bzip2),
        (14, Method::Lzma),
        (95, Method::Xz),
        (98, Method::Ppmd),
        (42, Method::Other(42)),
    ] {
        let mut entry = common::ZipEntry::stored("sounds/DSPISTOL.wav", b"data");
        entry.method = code;
        let zip = common::ZipBuilder::new()
            .stored("MAPINFO.txt", b"x")
            .entry(entry)
            .build();

        let err =
            Archive::from_bytes_with_options(zip.clone(), ParseOptions::strict()).unwrap_err();
        assert!(
            matches!(&err, ArchiveError::UnsupportedMethod { path, method: m } if path == "sounds/DSPISTOL.wav" && *m == method),
            "{err}"
        );
        assert!(
            err.to_string().contains(&method.to_string()),
            "names the method: {err}"
        );

        let archive = Archive::from_bytes_with_options(zip, ParseOptions::lenient())
            .expect("lenient lists it");
        assert_eq!(archive.members().len(), 2);
        assert_eq!(archive.members()[1].method(), method);
        assert!(
            matches!(&archive.warnings()[0], ArchiveWarning::UnreadableMember { path, .. } if path == "sounds/DSPISTOL.wav")
        );
    }
}

#[test]
fn encrypted_members_are_strict_errors_and_lenient_warnings() {
    let mut entry = common::ZipEntry::stored("secret.txt", b"data");
    entry.flags = common::ZIP_FLAG_ENCRYPTED;
    let zip = common::ZipBuilder::new().entry(entry).build();
    let err = Archive::from_bytes_with_options(zip.clone(), ParseOptions::strict()).unwrap_err();
    assert!(matches!(err, ArchiveError::Encrypted { .. }), "{err}");
    let archive = Archive::from_bytes_with_options(zip, ParseOptions::lenient()).expect("listed");
    assert!(archive.members()[0].is_encrypted());
    assert!(matches!(
        archive.warnings()[0],
        ArchiveWarning::UnreadableMember { .. }
    ));
}

#[test]
fn non_ascii_paths_are_strict_errors_and_lenient_nameless_members() {
    let zip = common::ZipBuilder::new()
        .stored("graphics/T\u{cf}TLE.png", b"x")
        .build();
    let err = Archive::from_bytes_with_options(zip.clone(), ParseOptions::strict()).unwrap_err();
    assert!(matches!(err, ArchiveError::NonAsciiName { .. }), "{err}");
    let archive = Archive::from_bytes_with_options(zip, ParseOptions::lenient()).expect("listed");
    assert_eq!(archive.members()[0].short_name(), None);
    assert_eq!(archive.members()[0].namespace(), Namespace::Graphics);
    assert!(matches!(
        archive.warnings()[0],
        ArchiveWarning::NonAsciiName { .. }
    ));
}

#[test]
fn duplicate_paths_error_strictly_and_later_wins_leniently() {
    let zip = common::ZipBuilder::new()
        .stored("MAPINFO.txt", b"first")
        .stored("mapinfo.TXT", b"second")
        .build();
    let err = Archive::from_bytes_with_options(zip.clone(), ParseOptions::strict()).unwrap_err();
    assert!(matches!(err, ArchiveError::DuplicatePath { .. }), "{err}");
    let archive =
        Archive::from_bytes_with_options(zip, ParseOptions::lenient()).expect("both kept");
    assert_eq!(archive.members().len(), 2);
    assert_eq!(archive.member("MAPINFO.txt").unwrap().index(), 1);
    assert!(matches!(
        archive.warnings()[0],
        ArchiveWarning::DuplicatePath { .. }
    ));
}

#[test]
fn oversized_declared_members_are_refused_strictly_and_flagged_leniently() {
    let zip = common::ZipBuilder::new()
        .stored("big.bin", &[0_u8; 64])
        .build();
    let limits = Limits::new().with_max_decoded_member_bytes(32);
    let err =
        Archive::from_bytes_with_options(zip.clone(), with_limits(ParseOptions::strict(), limits))
            .unwrap_err();
    assert!(
        matches!(
            err,
            ArchiveError::MemberTooLarge {
                declared: 64,
                limit: 32,
                ..
            }
        ),
        "{err}"
    );
    let archive =
        Archive::from_bytes_with_options(zip, with_limits(ParseOptions::lenient(), limits))
            .expect("listed");
    assert!(matches!(
        archive.warnings()[0],
        ArchiveWarning::MemberTooLarge {
            declared: 64,
            limit: 32,
            ..
        }
    ));
}

#[test]
fn a_hostile_zip64_locator_offset_is_corrupt_not_a_panic() {
    let zip = common::ZipBuilder::new()
        .zip64(true)
        .stored("a.txt", b"a")
        .build();
    let locator = zip
        .windows(4)
        .rposition(|w| w == [0x50, 0x4b, 0x06, 0x07])
        .expect("zip64 locator");
    for target in [0_u64, u64::MAX, u64::from(u32::MAX), 1 << 62] {
        let mut bad = zip.clone();
        bad[locator + 8..locator + 16].copy_from_slice(&target.to_le_bytes());
        for options in both_modes() {
            let err = Archive::from_bytes_with_options(bad.clone(), options).unwrap_err();
            assert!(
                matches!(err, ArchiveError::CorruptDirectory { .. }),
                "{err}"
            );
        }
    }
}

/// The offset of the first central-directory entry in a built fixture.
fn central_directory_start(zip: &[u8]) -> usize {
    zip.windows(4)
        .position(|w| w == [0x50, 0x4b, 0x01, 0x02])
        .expect("central directory")
}

#[test]
fn a_buffer_shorter_than_an_eocd_is_not_an_archive() {
    for options in both_modes() {
        let err = Archive::from_bytes_with_options(b"PK\x03\x04\0\0\0\0".to_vec(), options)
            .expect_err("too short to hold an EOCD");
        assert!(matches!(err, ArchiveError::NotAnArchive), "{err}");
    }
}

#[test]
fn a_bad_central_directory_signature_is_corrupt() {
    let zip = common::ZipBuilder::new()
        .stored("a.txt", b"a")
        .stored("b.txt", b"b")
        .build();
    let second = zip
        .windows(4)
        .rposition(|w| w == [0x50, 0x4b, 0x01, 0x02])
        .expect("second central directory entry");
    let mut bad = zip;
    bad[second + 2] = 0xFF;
    for options in both_modes() {
        let err = Archive::from_bytes_with_options(bad.clone(), options).unwrap_err();
        assert!(
            matches!(err, ArchiveError::CorruptDirectory { index: 1, .. }),
            "{err}"
        );
    }
}

#[test]
fn a_count_the_remaining_directory_cannot_hold_is_corrupt() {
    // Two entries are declared but only one 96-byte entry is written, so the
    // second read starts inside the last 46 bytes of the directory.
    let zip = common::ZipBuilder::new()
        .stored(
            "a-fifty-character-name-for-a-long-directory-entry.txt",
            b"a",
        )
        .entry_count_override(2)
        .build();
    for options in both_modes() {
        let err = Archive::from_bytes_with_options(zip.clone(), options).unwrap_err();
        assert!(
            matches!(err, ArchiveError::CorruptDirectory { index: 1, .. }),
            "{err}"
        );
    }
}

#[test]
fn a_non_utf8_member_name_is_decoded_lossily_and_flagged() {
    let zip = common::ZipBuilder::new().stored("a.txt", b"a").build();
    let cd = central_directory_start(&zip);
    let mut bad = zip;
    bad[cd + 46] = 0xFF; // first byte of the name is not valid UTF-8
    let err = Archive::from_bytes_with_options(bad.clone(), ParseOptions::strict()).unwrap_err();
    assert!(matches!(err, ArchiveError::NonAsciiName { .. }), "{err}");
    let archive = Archive::from_bytes_with_options(bad, ParseOptions::lenient()).expect("listed");
    assert_eq!(archive.members()[0].path(), "\u{fffd}.txt");
    assert_eq!(archive.members()[0].short_name(), None);
    assert!(matches!(
        archive.warnings()[0],
        ArchiveWarning::NonAsciiName { .. }
    ));
}

#[test]
fn zip64_extra_field_length_lies_are_corrupt() {
    let zip = common::ZipBuilder::new()
        .zip64(true)
        .stored("a.txt", b"a")
        .build();
    let cd = central_directory_start(&zip);
    let name_len = usize::from(u16::from_le_bytes([zip[cd + 28], zip[cd + 29]]));
    let field_len_at = cd + 46 + name_len + 2;
    // 0xFFFF overruns the entry's own extra region; 8 leaves room for only
    // one of the three ZIP64 replacements the sentinels ask for.
    for declared in [0xFFFF_u16, 8] {
        let mut bad = zip.clone();
        bad[field_len_at..field_len_at + 2].copy_from_slice(&declared.to_le_bytes());
        for options in both_modes() {
            let err = Archive::from_bytes_with_options(bad.clone(), options).unwrap_err();
            assert!(
                matches!(err, ArchiveError::CorruptDirectory { index: 0, .. }),
                "{err}"
            );
        }
    }
}

#[test]
fn reads_stored_and_deflated_members_back_exactly() {
    let big: Vec<u8> = (0..5000_u32).map(|i| (i % 251) as u8).collect();
    let zip = common::ZipBuilder::new()
        .stored("a.txt", b"hello")
        .deflate("b/big.bin", &big)
        .build();
    for options in both_modes() {
        let archive = Archive::from_bytes_with_options(zip.clone(), options).unwrap();
        assert_eq!(archive.read(&archive.members()[0]).unwrap(), b"hello");
        assert_eq!(archive.read(&archive.members()[1]).unwrap(), big);
    }
}

#[test]
fn data_descriptor_members_read_via_central_directory_sizes() {
    let mut entry = common::ZipEntry::deflate("graphics/A.png", &[9_u8; 1000]);
    entry.flags = common::ZIP_FLAG_DATA_DESCRIPTOR;
    let zip = common::ZipBuilder::new().entry(entry).build();
    let archive = Archive::from_bytes(zip).unwrap();
    assert_eq!(
        archive.read(&archive.members()[0]).unwrap(),
        vec![9_u8; 1000]
    );
}

#[test]
fn crc_lie_is_a_checksum_mismatch_in_both_modes() {
    let mut entry = common::ZipEntry::stored("a.txt", b"hello");
    entry.crc_override = Some(0xDEAD_BEEF);
    let zip = common::ZipBuilder::new().entry(entry).build();
    for options in both_modes() {
        let archive = Archive::from_bytes_with_options(zip.clone(), options).unwrap();
        let err = archive.read(&archive.members()[0]).unwrap_err();
        assert!(
            matches!(err, ArchiveError::ChecksumMismatch { .. }),
            "{err}"
        );
    }
}

#[test]
fn declared_size_smaller_than_the_stream_is_a_size_mismatch() {
    let mut entry = common::ZipEntry::deflate("a.bin", &[1_u8; 400]);
    entry.size_override = Some(100);
    let zip = common::ZipBuilder::new().entry(entry).build();
    for options in both_modes() {
        let archive = Archive::from_bytes_with_options(zip.clone(), options).unwrap();
        let err = archive.read(&archive.members()[0]).unwrap_err();
        assert!(
            matches!(
                err,
                ArchiveError::SizeMismatch {
                    declared: 100,
                    actual: None,
                    ..
                }
            ),
            "{err}"
        );
    }
}

#[test]
fn declared_size_larger_than_the_stream_is_a_size_mismatch() {
    let mut entry = common::ZipEntry::deflate("a.bin", &[1_u8; 100]);
    entry.size_override = Some(400);
    let zip = common::ZipBuilder::new().entry(entry).build();
    for options in both_modes() {
        let archive = Archive::from_bytes_with_options(zip.clone(), options).unwrap();
        let err = archive.read(&archive.members()[0]).unwrap_err();
        assert!(
            matches!(
                err,
                ArchiveError::SizeMismatch {
                    declared: 400,
                    actual: Some(100),
                    ..
                }
            ),
            "{err}"
        );
    }
}

#[test]
fn stored_member_whose_sizes_disagree_is_a_size_mismatch() {
    let mut entry = common::ZipEntry::stored("a.bin", &[1_u8; 100]);
    entry.size_override = Some(50);
    let zip = common::ZipBuilder::new().entry(entry).build();
    let archive = Archive::from_bytes(zip).unwrap();
    let err = archive.read(&archive.members()[0]).unwrap_err();
    assert!(matches!(err, ArchiveError::SizeMismatch { .. }), "{err}");
}

#[test]
fn corrupt_deflate_stream_is_reported() {
    let mut entry = common::ZipEntry::deflate("a.bin", &[1_u8; 100]);
    entry.compressed_override = Some(vec![0xFF; 20]);
    let zip = common::ZipBuilder::new().entry(entry).build();
    let archive = Archive::from_bytes(zip).unwrap();
    let err = archive.read(&archive.members()[0]).unwrap_err();
    assert!(
        matches!(
            err,
            ArchiveError::CorruptStream { .. } | ArchiveError::SizeMismatch { .. }
        ),
        "{err}"
    );
}

#[test]
fn local_header_disagreeing_with_the_directory_is_corrupt_at_read() {
    let zip = common::ZipBuilder::new().stored("abcdef.txt", b"a").build();
    let mut bad = zip.clone();
    // Local header name length (offset 26) -> lie.
    bad[26] = 0xFF;
    bad[27] = 0x7F;
    for options in both_modes() {
        let archive = Archive::from_bytes_with_options(bad.clone(), options).unwrap();
        let err = archive.read(&archive.members()[0]).unwrap_err();
        assert!(
            matches!(err, ArchiveError::CorruptDirectory { index: 0, .. }),
            "{err}"
        );
    }
}

#[test]
fn reading_an_unreadable_lenient_member_fails_by_name() {
    let mut encrypted = common::ZipEntry::stored("secret.txt", b"x");
    encrypted.flags = common::ZIP_FLAG_ENCRYPTED;
    let mut lzma = common::ZipEntry::stored("packed.bin", b"x");
    lzma.method = 14;
    let zip = common::ZipBuilder::new()
        .entry(encrypted)
        .entry(lzma)
        .build();
    let archive = Archive::from_bytes_with_options(zip, ParseOptions::lenient()).unwrap();
    assert!(matches!(
        archive.read(&archive.members()[0]).unwrap_err(),
        ArchiveError::Encrypted { .. }
    ));
    assert!(matches!(
        archive.read(&archive.members()[1]).unwrap_err(),
        ArchiveError::UnsupportedMethod {
            method: Method::Lzma,
            ..
        }
    ));
}

#[test]
fn reading_a_lenient_oversized_member_fails_with_the_limit() {
    let zip = common::ZipBuilder::new()
        .stored("big.bin", &[0_u8; 64])
        .build();
    let options = with_limits(
        ParseOptions::lenient(),
        Limits::new().with_max_decoded_member_bytes(32),
    );
    let archive = Archive::from_bytes_with_options(zip, options).unwrap();
    let err = archive.read(&archive.members()[0]).unwrap_err();
    assert!(
        matches!(
            err,
            ArchiveError::MemberTooLarge {
                declared: 64,
                limit: 32,
                ..
            }
        ),
        "{err}"
    );
}

#[test]
fn wad_parses_a_wad_member_and_refuses_non_wads() {
    let wad = common::build_wad(*b"PWAD", &[("MAP01", &[]), ("THINGS", &[])]);
    let zip = common::ZipBuilder::new()
        .deflate("maps/MAP01.wad", &wad)
        .stored("readme.txt", b"hi")
        .build();
    let archive = Archive::from_bytes(zip).unwrap();
    let parsed = archive
        .wad(&archive.members()[0])
        .expect("member parses as a WAD");
    assert_eq!(parsed.lump_count(), 2);
    let err = archive.wad(&archive.members()[1]).unwrap_err();
    assert!(matches!(err, ArchiveError::NotAWad { .. }), "{err}");
}

#[test]
fn wad_parse_failure_keeps_the_member_path() {
    let zip = common::ZipBuilder::new()
        .stored("maps/MAP01.wad", b"PWAD\xff\xff\xff\x7f\0\0\0\0")
        .build();
    let archive = Archive::from_bytes(zip).unwrap();
    let err = archive.wad(&archive.members()[0]).unwrap_err();
    assert!(
        matches!(&err, ArchiveError::Wad { path, .. } if path == "maps/MAP01.wad"),
        "{err}"
    );
    assert!(err.to_string().starts_with("member `maps/MAP01.wad`: "));
}

#[test]
fn a_local_header_that_is_missing_or_misplaced_is_corrupt_at_read() {
    let zip = common::ZipBuilder::new().stored("a.txt", b"hello").build();
    let cd = central_directory_start(&zip);
    // The directory points the local header at (a) the end of the file, so
    // the fixed 30 bytes do not fit, and (b) a spot whose four leading bytes
    // are not the local-header signature.
    let past_eof = u32::try_from(zip.len()).expect("fixture fits in u32");
    for offset in [past_eof, 4] {
        let mut bad = zip.clone();
        bad[cd + 42..cd + 46].copy_from_slice(&offset.to_le_bytes());
        for options in both_modes() {
            let archive = Archive::from_bytes_with_options(bad.clone(), options).unwrap();
            let err = archive.read(&archive.members()[0]).unwrap_err();
            assert!(
                matches!(
                    &err,
                    ArchiveError::CorruptDirectory { index: 0, reason }
                        if *reason == "local header missing or misplaced"
                ),
                "{err}"
            );
        }
    }
}

#[test]
fn a_zip64_local_header_offset_of_u64_max_is_corrupt_not_an_overflow() {
    let zip = common::ZipBuilder::new()
        .zip64(true)
        .stored("a.txt", b"hello")
        .build();
    let cd = central_directory_start(&zip);
    let name_len = usize::from(u16::from_le_bytes([zip[cd + 28], zip[cd + 29]]));
    // ZIP64 extra field: header (4) + uncompressed (8) + compressed (8), then
    // the local-header offset.
    let offset_field = cd + 46 + name_len + 4 + 16;
    let mut bad = zip.clone();
    bad[offset_field..offset_field + 8].copy_from_slice(&u64::MAX.to_le_bytes());
    for options in both_modes() {
        let archive = Archive::from_bytes_with_options(bad.clone(), options).unwrap();
        let err = archive.read(&archive.members()[0]).unwrap_err();
        assert!(
            matches!(
                &err,
                ArchiveError::CorruptDirectory { index: 0, reason }
                    if *reason == "local header missing or misplaced"
            ),
            "{err}"
        );
    }
}

#[test]
fn a_member_from_another_archive_is_refused_without_panicking() {
    let one = Archive::from_bytes(common::ZipBuilder::new().stored("a.txt", b"a").build()).unwrap();
    let two = Archive::from_bytes(
        common::ZipBuilder::new()
            .stored("a.txt", b"a")
            .stored("b.txt", b"b")
            .build(),
    )
    .unwrap();
    let err = one.read(&two.members()[1]).unwrap_err();
    assert!(
        matches!(
            &err,
            ArchiveError::CorruptDirectory { index: 1, reason }
                if *reason == "member does not belong to this archive"
        ),
        "{err}"
    );
}
