#![no_main]
//! Fuzzes pk3 (zip) archive reading (`archive/`, ADR-0031) through the public
//! API: `Archive::from_bytes_with_options` in both `Strictness` modes, then
//! `read()` and — for `.wad` members — `wad()` on every member, plus `maps()`
//! and `embedded_wads()`.
//!
//! Oracle (ADR-0016): no panic in either mode; the member table is
//! `O(input)` — a central-directory entry is at least 46 bytes, so
//! `members.len() <= input.len() / 46`; and the total decoded output is
//! bounded by `members × MAX_DECODED`, the inflate cap pinned into
//! `Limits::max_decoded_member_bytes` (modest here so a highly compressible
//! seed trips `MemberTooLarge`/`SizeMismatch` cheaply instead of inflating to
//! the 256 MiB production default). The member cap is pinned too, so a lying
//! entry count fails fast on `TooManyMembers` rather than walking a huge
//! declared directory.
//!
//! Corpus seeds (`fuzz/corpus/fuzz_archive/`) are small real zips built by the
//! `ZipBuilder` fixture builder from the integration tests: a stored-only
//! archive, a deflated archive with `maps/MAP01.wad` and a root embedded WAD,
//! a ZIP64 archive, and a data-descriptor archive.

use libfuzzer_sys::fuzz_target;

use crustywad::ParseOptions;
use crustywad::archive::Archive;

/// Inflate cap pinned into `Limits::max_decoded_member_bytes`.
const MAX_DECODED: usize = 1 << 20;
/// Member cap pinned into `Limits::max_archive_members`.
const MAX_MEMBERS: usize = 4096;

fuzz_target!(|data: &[u8]| {
    if data.len() > (1usize << 24) {
        return;
    }
    for base in [ParseOptions::strict(), ParseOptions::lenient()] {
        let mut options = base;
        options.limits = options
            .limits
            .with_max_decoded_member_bytes(MAX_DECODED)
            .with_max_archive_members(MAX_MEMBERS);
        let Ok(archive) = Archive::from_bytes_with_options(data.to_vec(), options) else {
            continue;
        };
        let members = archive.members().len();
        assert!(
            members <= data.len() / 46,
            "member count {members} exceeds O(input) bound {}",
            data.len() / 46
        );
        assert!(members <= MAX_MEMBERS, "member cap not enforced");
        let mut decoded_total = 0usize;
        for member in archive.members() {
            if let Ok(bytes) = archive.read(member) {
                assert!(
                    bytes.len() <= MAX_DECODED,
                    "decoded {} bytes over the cap",
                    bytes.len()
                );
                decoded_total += bytes.len();
            }
            if member.is_embedded_wad() {
                let _ = std::hint::black_box(archive.wad(member));
            }
        }
        assert!(
            decoded_total <= members.saturating_mul(MAX_DECODED),
            "total decoded output {decoded_total} exceeds members × cap"
        );
        for map in archive.maps() {
            let member = &archive.members()[map.member_index()];
            let _ = std::hint::black_box(archive.wad(member));
        }
        let _ = std::hint::black_box(archive.embedded_wads().len());
        let _ = std::hint::black_box(archive.warnings().len());
    }
});
