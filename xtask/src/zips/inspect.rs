//! Central-directory extraction over a `RangeSource` (DESIGN.md §5.2/§5.3/§5.5).
//!
//! The driver is two bounded miss-and-retry stages over the same
//! [`SparseBuffer`] (not to be confused with harvester "phase 2" —
//! `harvest-zips` as a whole — which this module is *part of*), because
//! the two kinds of miss it copes with have very different correct widths.
//!
//! **Stage 1 — locate and parse the central directory.** Round 1 fetches
//! the last [`TAIL_LEN`] bytes (§5.2 item 1 — covers the worst-case EOCD
//! scan and any ZIP64 EOCD structures). If `zip::ZipArchive::new`'s reads
//! miss below the tail, the missing extent — by construction the central
//! directory (§5.2 item 2) — is fetched *in full*, in one request, and the
//! parse re-run: that gap genuinely is what we need next. Well-formed
//! archives converge in ≤2 fetches; [`MAX_FETCH_ROUNDS`] fails the
//! pathological rest closed via [`CD_CAP`].
//!
//! **Stage 2 — walk the members.** [`inspection_from_archive`] classifies
//! each entry via `name_for_index` (pure central-directory metadata, no
//! I/O), so only an actual `.wad` match ever calls `by_index_raw`. That
//! call never decompresses a member (the §5.2 no-payload invariant holds),
//! but it *does* eagerly parse that member's local file header — a fixed
//! ~30-byte block (zip 8.6.0 `ZipLocalEntryBlock`, `src/types.rs:235-265`;
//! it reads only that fixed block, never the name/extra bytes that follow
//! it) — to compute a data offset it never uses. For a `.wad` positioned
//! early in a file larger than [`TAIL_LEN`] whose central directory
//! nonetheless resolves from the tail, that header sits outside every
//! fetched extent, and the eager read is a cache miss.
//!
//! Unlike a stage-1 miss, a stage-2 miss must **not** widen to the whole
//! gap: the gap between a member's header and the cached tail is, at
//! corpus scale, essentially the member's own payload (a ~1.8 MiB corpus
//! average) — fetching it would mean ranged reads quietly mirror the
//! archive, which ADR-0030 §4 forbids and which §9.3's "small fraction of
//! bytes transferred" acceptance would fail. So a stage-2 miss fetches only
//! [`LOCAL_HEADER_PAD`] bytes (capped by whatever is actually missing), one
//! header at a time, under its own [`MAX_MEMBER_ROUNDS`] budget —
//! independent of stage 1's [`MAX_FETCH_ROUNDS`], since a handful of
//! `.wad`s scattered outside the tail is normal and each only costs a
//! nibble. [`inspection_from_archive`] stops its member walk at the first
//! such miss (checking the shared cell right after a `by_index_raw` error)
//! rather than continuing through every remaining entry: with a
//! single-slot miss cell, "keep walking" would let a *later*, coincidental
//! miss overwrite an *earlier* one, so each retry would only widen around
//! whichever miss happened to be recorded last — for many scattered
//! members that can take far more than a handful of rounds to converge.
//! Stopping at the first miss means each stage-2 round makes forward
//! progress on exactly one header and the next round re-walks from
//! entry 0 over now-cached bytes for free.
//! `three_wad_headers_scattered_outside_tail_cost_one_fetch_each` below
//! exercises this directly (three `.wad`s, three separate nibble fetches);
//! `cd_outside_tail_takes_exactly_two_fetches` (0 `.wad` members) proves
//! stage 2 never even starts when there is nothing for it to do.

use std::cell::Cell;
use std::io::{Read, Seek};

use crate::schema::WadMember;
use crate::zips::range_reader::{RangeReader, SparseBuffer, TAIL_LEN};

/// §5.4/ADR-0016: a lying EOCD can declare any CD size; never read more
/// than this into memory.
pub const CD_CAP: u64 = 64 * 1024 * 1024;

/// Stage-1 (CD-locate) fetch rounds before declaring the access pattern
/// too chatty (§5.2 expects 2–3 requests; 4 leaves headroom for ZIP64
/// oddities). Governs only central-directory location/parse; see
/// [`MAX_MEMBER_ROUNDS`] for the separate stage-2 (member-walk) budget.
pub(crate) const MAX_FETCH_ROUNDS: u32 = 4;

/// Upper bound on a single stage-2 (member-walk) fetch: a local file
/// header is a fixed ~30-byte block (zip 8.6.0 `ZipLocalEntryBlock`,
/// vendored `src/types.rs:235-265` — `find_data_start` reads only that
/// fixed block, never the name/extra bytes that follow it), so this is a
/// generous pad, not a measured size. Bounding a stage-2 fetch by this
/// (rather than widening to the next cached byte, as stage 1 does) is
/// what keeps a `.wad` member's local header from pulling in everything
/// up to the tail — at corpus scale, its own payload — in one "miss".
const LOCAL_HEADER_PAD: u64 = 256;

/// Stage-2 (member-walk) fetch rounds before declaring the access pattern
/// too chatty (record-don't-skip: any `.wad` still unresolved past this
/// point never blocks the archive). Independent of stage 1's
/// [`MAX_FETCH_ROUNDS`] and set much higher: each round costs one small,
/// bounded fetch (see [`LOCAL_HEADER_PAD`]), and a handful of `.wad`
/// members with local headers scattered outside the tail is a normal
/// shape for a real archive, not a pathological one.
const MAX_MEMBER_ROUNDS: u32 = 24;

/// One ranged fetch against some byte source. Async-in-trait is fine here
/// for the same reason as `ListingSource`: internal-only, driven as
/// `&mut impl RangeSource` on a single call chain.
#[allow(async_fn_in_trait)]
pub trait RangeSource {
    /// Fetch exactly `len` bytes at `offset`.
    ///
    /// # Errors
    /// [`FetchFailure`] — transport failure, missing file, or a mirror that
    /// answers `200` to a range request.
    async fn fetch(&mut self, offset: u64, len: u64) -> Result<Vec<u8>, FetchFailure>;
}

/// Why a fetch (or a whole entry) failed at the transport level.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FetchFailure {
    /// Every usable mirror answered `200` to a ranged request (§5.2: THE
    /// no-range-support signal) — full-download fallback territory.
    RangeUnsupported,
    /// Every mirror answered 404 (§5.5: entry in the DB, not on mirrors).
    NotFound,
    /// Transport/HTTP failure after retries (detail for the ledger).
    Http(String),
}

impl std::fmt::Display for FetchFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RangeUnsupported => write!(f, "mirrors refuse ranges"),
            Self::NotFound => write!(f, "404 on all mirrors"),
            Self::Http(detail) => write!(f, "{detail}"),
        }
    }
}

/// What phase 2 learns about one archive entry.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inspection {
    /// ZIP64 EOCD locator present in the tail (§5.3).
    pub zip64: bool,
    /// Total central-directory entries.
    pub member_count: u64,
    /// `.wad` members (ASCII case-insensitive, §5.5).
    pub wads: Vec<WadMember>,
    /// Every other member name, CD order.
    pub other_members: Vec<String>,
}

/// Why an entry could not be inspected.
#[allow(dead_code)]
#[derive(Debug)]
pub enum InspectError {
    /// Transport failure — carries the classified [`FetchFailure`].
    Fetch(FetchFailure),
    /// The implied CD extent exceeds [`CD_CAP`] (ADR-0016 fail-closed).
    CdTooLarge { needed: u64 },
    /// Either stage still missed after its round budget: the CD-locate
    /// stage's [`MAX_FETCH_ROUNDS`], or the member-walk stage's
    /// [`MAX_MEMBER_ROUNDS`].
    TooChatty { rounds: u32 },
    /// Bytes arrived whole but the zip did not parse.
    Parse(String),
}

impl std::fmt::Display for InspectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fetch(failure) => write!(f, "fetch failed: {failure}"),
            Self::CdTooLarge { needed } => {
                write!(f, "central directory needs {needed} bytes, cap is {CD_CAP}")
            }
            Self::TooChatty { rounds } => {
                write!(f, "gave up after {rounds} fetch rounds without converging")
            }
            Self::Parse(detail) => write!(f, "zip did not parse: {detail}"),
        }
    }
}

/// Inspect one remote zip of `file_size` bytes via ranged reads.
///
/// # Errors
/// [`InspectError`] — every variant maps onto a §5.6 `fetch_status`.
#[allow(dead_code)]
pub async fn inspect_zip(
    source: &mut impl RangeSource,
    file_size: u64,
) -> Result<Inspection, InspectError> {
    inspect_zip_with_caps(source, file_size, CD_CAP, MAX_FETCH_ROUNDS).await
}

/// Cap-parameterized body of [`inspect_zip`] so tests exercise the guards
/// without multi-mebibyte fixtures (mirror.rs cap-override precedent).
/// `max_rounds` bounds only stage 1 (CD-locate); stage 2 (member-walk) has
/// its own fixed [`MAX_MEMBER_ROUNDS`] budget (see module docs for why the
/// two are independent).
#[allow(dead_code)]
pub(crate) async fn inspect_zip_with_caps(
    source: &mut impl RangeSource,
    file_size: u64,
    cd_cap: u64,
    max_rounds: u32,
) -> Result<Inspection, InspectError> {
    if file_size == 0 {
        return Err(InspectError::Parse("zero-length file".into()));
    }
    // Round 1: the tail — or the whole file when it fits (§5.2 small-file
    // rule: cheaper than three round trips).
    let tail_start = file_size.saturating_sub(TAIL_LEN);
    let first = source
        .fetch(tail_start, file_size - tail_start)
        .await
        .map_err(InspectError::Fetch)?;
    let zip64 = zip64_present(&first);
    let mut buf = SparseBuffer::new(file_size);
    buf.insert(tail_start, first);

    // Stage 1: locate and parse the central directory. A miss here means
    // the CD (or the EOCD scan) isn't fully cached yet — by construction
    // the whole gap up to the tail IS what we need, so fetch it in full.
    for round in 1..=max_rounds {
        let missing = Cell::new(None);
        match zip::ZipArchive::new(RangeReader::new(&buf, &missing)) {
            // The CD parses; nothing further is needed from this archive
            // (stage 2 below re-parses fresh once cache misses are
            // resolved), so drop it immediately — its `RangeReader` holds
            // an immutable borrow of `buf` that must end before stage 2
            // can insert newly-fetched bytes into it.
            Ok(_) => {
                return inspect_members(source, &mut buf, zip64).await;
            }
            Err(e) => {
                let Some((miss_at, _len)) = missing.get() else {
                    // A genuine parse failure, not our cache miss.
                    return Err(InspectError::Parse(e.to_string()));
                };
                let widen_to = buf.next_covered_start(miss_at);
                let needed = widen_to - miss_at;
                if needed > cd_cap {
                    return Err(InspectError::CdTooLarge { needed });
                }
                if round == max_rounds {
                    return Err(InspectError::TooChatty { rounds: round });
                }
                let bytes = source
                    .fetch(miss_at, needed)
                    .await
                    .map_err(InspectError::Fetch)?;
                buf.insert(miss_at, bytes);
            }
        }
    }
    Err(InspectError::TooChatty { rounds: max_rounds })
}

/// Stage 2: walk the members over an already-fully-cached central
/// directory. A miss here is a `.wad` member's local header (never the CD
/// itself — stage 1 already fully resolved it), so unlike stage 1 it is
/// fetched bounded by [`LOCAL_HEADER_PAD`], never widened to the whole gap
/// up to the tail (module docs: at corpus scale that gap is essentially
/// the member's own payload). Each round re-parses the CD (already cached,
/// so this costs no further fetches) and re-walks from entry 0; a fresh
/// [`SparseBuffer`] borrow is required each round because inserting the
/// fetched header mutably borrows `buf`, which the previous round's
/// `RangeReader` was still (immutably) borrowing.
async fn inspect_members(
    source: &mut impl RangeSource,
    buf: &mut SparseBuffer,
    zip64: bool,
) -> Result<Inspection, InspectError> {
    for round in 1..=MAX_MEMBER_ROUNDS {
        let missing = Cell::new(None);
        let parsed = zip::ZipArchive::new(RangeReader::new(buf, &missing))
            .map(|mut archive| inspection_from_archive(&mut archive, zip64, &missing));

        match (parsed, missing.get()) {
            (Ok(inspection), None) => return Ok(inspection),
            (Err(e), None) => {
                // Central directory is already fully cached (stage 1
                // succeeded); a fresh parse failure here is genuine, not a
                // cache miss.
                return Err(InspectError::Parse(e.to_string()));
            }
            (_, Some((miss_at, miss_len))) => {
                if round == MAX_MEMBER_ROUNDS {
                    return Err(InspectError::TooChatty { rounds: round });
                }
                // Fetch only the missing local header, never the gap up to
                // the already-cached tail (that gap is the member's own
                // payload at corpus scale — see module docs).
                let capped_end = buf
                    .next_covered_start(miss_at)
                    .min(miss_at + miss_len.max(LOCAL_HEADER_PAD));
                let needed = capped_end - miss_at;
                let bytes = source
                    .fetch(miss_at, needed)
                    .await
                    .map_err(InspectError::Fetch)?;
                buf.insert(miss_at, bytes);
            }
        }
    }
    Err(InspectError::TooChatty {
        rounds: MAX_MEMBER_ROUNDS,
    })
}

/// Walk the parsed central directory and split members into `.wad` records
/// and other names.
///
/// Classification uses [`zip::ZipArchive::name_for_index`] — pure
/// central-directory metadata, no further I/O — so a non-`.wad` member
/// (the common case: `.txt`/`.deh`/nested archives) never touches
/// `by_index_raw` at all. Only a `.wad` match pays for `by_index_raw`,
/// which never decompresses a member (the §5.2 no-payload invariant) but
/// does eagerly parse that member's local file header to compute a data
/// offset it never uses; see the module docs for how the driver above
/// copes with a member whose local header lands outside the fetched
/// extent.
///
/// `missing` is the same cell the caller's [`RangeReader`] writes to on a
/// cache miss (`archive`'s reader must be backed by that same cell for
/// this to mean anything). When a `by_index_raw` call fails **and**
/// `missing` is set, that failure is a cache miss, not a corrupt entry:
/// the walk stops immediately rather than recording it and continuing (see
/// module docs for why "keep walking" is the wrong call with a
/// single-slot miss cell). The caller is expected to check `missing` after
/// this returns and discard/retry on a partial result — that's also what
/// makes this safe to call on a fully-covered, miss-proof reader (e.g. a
/// full-download `Cursor`): pass any cell (it will simply never be set) and
/// the walk always runs to completion.
#[allow(dead_code)]
pub fn inspection_from_archive<R: Read + Seek>(
    archive: &mut zip::ZipArchive<R>,
    zip64: bool,
    missing: &Cell<Option<(u64, u64)>>,
) -> Inspection {
    let mut wads = Vec::new();
    let mut other_members = Vec::new();
    for i in 0..archive.len() {
        let Some(name) = archive.name_for_index(i) else {
            other_members.push(format!("<unreadable entry {i}>"));
            continue;
        };
        if !name.to_ascii_lowercase().ends_with(".wad") {
            other_members.push(name.to_owned());
            continue;
        }
        match archive.by_index_raw(i) {
            Ok(file) => {
                wads.push(WadMember {
                    name: file.name().to_owned(),
                    compressed: file.compressed_size(),
                    uncompressed: file.size(),
                    method: method_label(file.compression()),
                    encrypted: file.encrypted(),
                });
            }
            Err(_) if missing.get().is_some() => {
                // A cache miss on this member's local header — abandon the
                // walk now. The caller sees `missing` set and retries with
                // exactly that extent fetched; this partial `Inspection` is
                // discarded, never surfaced as a result.
                break;
            }
            Err(_) => {
                // A genuine unreadable entry (not a cache miss):
                // record-and-continue (ADR-0030 §5).
                other_members.push(format!("<unreadable entry {i}>"));
            }
        }
    }
    Inspection {
        zip64,
        member_count: u64::try_from(archive.len()).unwrap_or(u64::MAX),
        wads,
        other_members,
    }
}

/// §5.6 method label: the two expected methods by name, anything §5.5-odd
/// via the crate's own Display (lowercased) so new methods stay visible.
///
/// Compares against [`zip::CompressionMethod::STORE`]/[`zip::CompressionMethod::DEFLATE`]
/// rather than matching the `Stored`/`Deflated` variants directly: the
/// production `[dependencies]` entry builds with `default-features =
/// false` (§3 — phase 2 never decompresses a member), which compiles out
/// the `Deflated` variant entirely (it requires the `_deflate-any`
/// feature) — matching the bare variant doesn't compile under that
/// configuration. The associated constants sidestep this correctly, not
/// just by compiling: `CompressionMethod::parse_from_u16(8)` (what a
/// method-id-8 CD entry actually parses to) and `CompressionMethod::DEFLATE`
/// are defined by the *same* `#[cfg(feature = "_deflate-any")]` gate
/// (vendored `src/compression.rs:105-106,194-195`) — both resolve to
/// `Unsupported(8)` when the feature is off, both resolve to the real
/// `Deflated` variant when it's on. They're always equal for a real
/// deflate member, so this comparison labels it `"deflate"` correctly
/// under both configurations (test unification turns the feature on;
/// production leaves it off) — not a production/test discrepancy.
#[allow(dead_code)]
fn method_label(method: zip::CompressionMethod) -> String {
    if method == zip::CompressionMethod::STORE {
        "stored".into()
    } else if method == zip::CompressionMethod::DEFLATE {
        "deflate".into()
    } else {
        format!("{method:?}").to_ascii_lowercase()
    }
}

/// Scan `tail` for the classic EOCD signature and check whether the 20
/// bytes immediately before it are a ZIP64 EOCD locator (§5.3: the locator
/// always immediately precedes the EOCD). Precise — not a size heuristic.
#[allow(dead_code)]
pub fn zip64_present(tail: &[u8]) -> bool {
    const EOCD_SIG: [u8; 4] = 0x0605_4b50_u32.to_le_bytes();
    const LOCATOR_SIG: [u8; 4] = 0x0706_4b50_u32.to_le_bytes();
    // Backward scan: the LAST EOCD signature is the real one (§5.3).
    let Some(eocd_at) = tail.windows(4).rposition(|w| w == EOCD_SIG) else {
        return false;
    };
    eocd_at >= 20 && tail[eocd_at - 20..eocd_at - 16] == LOCATOR_SIG
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    /// File-backed fake: serves ranges from an in-memory byte vec and counts
    /// fetches — the §9.1 request-count regression instrument — plus bytes
    /// served, which catches a widen-too-far bug that a fetch count alone
    /// cannot (a single miss-handling fetch can still transfer the whole
    /// gap up to the tail; see `bytes_served` assertions below).
    struct FakeSource {
        bytes: Vec<u8>,
        fetches: u32,
        bytes_served: u64,
    }

    impl FakeSource {
        fn new(bytes: Vec<u8>) -> Self {
            Self {
                bytes,
                fetches: 0,
                bytes_served: 0,
            }
        }
        fn len(&self) -> u64 {
            u64::try_from(self.bytes.len()).unwrap()
        }
    }

    impl RangeSource for FakeSource {
        async fn fetch(&mut self, offset: u64, len: u64) -> Result<Vec<u8>, FetchFailure> {
            self.fetches += 1;
            let start = usize::try_from(offset).unwrap();
            let end = start + usize::try_from(len).unwrap();
            let served = self
                .bytes
                .get(start..end)
                .map(<[u8]>::to_vec)
                .ok_or_else(|| FetchFailure::Http("range beyond EOF".into()))?;
            self.bytes_served += u64::try_from(served.len()).unwrap();
            Ok(served)
        }
    }

    /// Build a stored-method zip with the given `(name, contents)` members via
    /// the crate's writer, plus an optional archive comment.
    fn stored_zip(members: &[(&str, &[u8])], comment: &[u8]) -> Vec<u8> {
        let mut w = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        for (name, contents) in members {
            w.start_file(*name, opts).unwrap();
            w.write_all(contents).unwrap();
        }
        w.set_raw_comment(comment.to_vec().into()).unwrap();
        w.finish().unwrap().into_inner()
    }

    /// Flip general-purpose bit 0 (encryption) in both the local and the
    /// central-directory header of a single-member zip. We never decrypt, so a
    /// bit-flip is a fully valid §5.5 fixture.
    fn set_encryption_bit(zip: &mut [u8]) {
        patch_u16_after_sig(zip, &0x0403_4b50_u32.to_le_bytes(), 6, |flags| flags | 1);
        patch_u16_after_sig(zip, &0x0201_4b50_u32.to_le_bytes(), 8, |flags| flags | 1);
    }

    /// Overwrite the compression-method field (local offset 8, CD offset 10)
    /// of a single-member zip with an arbitrary method id (e.g. 12 = bzip2).
    fn set_method(zip: &mut [u8], method: u16) {
        patch_u16_after_sig(zip, &0x0403_4b50_u32.to_le_bytes(), 8, |_| method);
        patch_u16_after_sig(zip, &0x0201_4b50_u32.to_le_bytes(), 10, |_| method);
    }

    /// Find `sig` (first occurrence) and rewrite the little-endian u16 at
    /// `sig_pos + offset` through `f`.
    fn patch_u16_after_sig(zip: &mut [u8], sig: &[u8], offset: usize, f: impl Fn(u16) -> u16) {
        let pos = zip
            .windows(sig.len())
            .position(|w| w == sig)
            .expect("signature present");
        let at = pos + offset;
        let old = u16::from_le_bytes([zip[at], zip[at + 1]]);
        zip[at..at + 2].copy_from_slice(&f(old).to_le_bytes());
    }

    /// Hand-assembled minimal ZIP64 archive: one stored member `BIG.WAD`
    /// (8 payload bytes) whose CD sizes are `0xFFFFFFFF` sentinels resolved by
    /// a ZIP64 extended-information extra field, plus ZIP64 EOCD record +
    /// locator + classic EOCD (§5.3 layouts). Small on disk, structurally
    /// ZIP64 — exercises exactly the "read the extra field, not the sentinel"
    /// path that silent 4 GiB truncation gets wrong.
    fn zip64_fixture() -> Vec<u8> {
        let payload = b"WADDATA!";
        let crc = crc32(payload); // helper below
        let name = b"BIG.WAD";
        let payload_len = u64::try_from(payload.len()).unwrap();
        let mut z = Vec::new();
        // -- local file header --
        z.extend_from_slice(&0x0403_4b50_u32.to_le_bytes());
        z.extend_from_slice(&45_u16.to_le_bytes()); // version needed: ZIP64
        z.extend_from_slice(&0_u16.to_le_bytes()); // flags
        z.extend_from_slice(&0_u16.to_le_bytes()); // method: stored
        z.extend_from_slice(&0_u32.to_le_bytes()); // dos time+date
        z.extend_from_slice(&crc.to_le_bytes());
        z.extend_from_slice(&0xFFFF_FFFF_u32.to_le_bytes()); // compressed: sentinel
        z.extend_from_slice(&0xFFFF_FFFF_u32.to_le_bytes()); // uncompressed: sentinel
        z.extend_from_slice(&u16::try_from(name.len()).unwrap().to_le_bytes());
        z.extend_from_slice(&20_u16.to_le_bytes()); // extra len: 4 + 16
        z.extend_from_slice(name);
        z.extend_from_slice(&0x0001_u16.to_le_bytes()); // ZIP64 extra header id
        z.extend_from_slice(&16_u16.to_le_bytes()); // data size
        z.extend_from_slice(&payload_len.to_le_bytes()); // uncompressed
        z.extend_from_slice(&payload_len.to_le_bytes()); // compressed
        z.extend_from_slice(payload);
        // -- central directory --
        let cd_offset = u64::try_from(z.len()).unwrap();
        z.extend_from_slice(&0x0201_4b50_u32.to_le_bytes());
        z.extend_from_slice(&45_u16.to_le_bytes()); // version made by
        z.extend_from_slice(&45_u16.to_le_bytes()); // version needed
        z.extend_from_slice(&0_u16.to_le_bytes()); // flags
        z.extend_from_slice(&0_u16.to_le_bytes()); // method
        z.extend_from_slice(&0_u32.to_le_bytes()); // dos time+date
        z.extend_from_slice(&crc.to_le_bytes());
        z.extend_from_slice(&0xFFFF_FFFF_u32.to_le_bytes()); // compressed: sentinel
        z.extend_from_slice(&0xFFFF_FFFF_u32.to_le_bytes()); // uncompressed: sentinel
        z.extend_from_slice(&u16::try_from(name.len()).unwrap().to_le_bytes());
        z.extend_from_slice(&20_u16.to_le_bytes()); // extra len
        z.extend_from_slice(&0_u16.to_le_bytes()); // comment len
        z.extend_from_slice(&0_u16.to_le_bytes()); // disk start
        z.extend_from_slice(&0_u16.to_le_bytes()); // internal attrs
        z.extend_from_slice(&0_u32.to_le_bytes()); // external attrs
        z.extend_from_slice(&0_u32.to_le_bytes()); // local header offset (fits)
        z.extend_from_slice(name);
        z.extend_from_slice(&0x0001_u16.to_le_bytes());
        z.extend_from_slice(&16_u16.to_le_bytes());
        z.extend_from_slice(&payload_len.to_le_bytes());
        z.extend_from_slice(&payload_len.to_le_bytes());
        let cd_size = u64::try_from(z.len()).unwrap() - cd_offset;
        // -- ZIP64 EOCD record --
        let zip64_eocd_offset = u64::try_from(z.len()).unwrap();
        z.extend_from_slice(&0x0606_4b50_u32.to_le_bytes());
        z.extend_from_slice(&44_u64.to_le_bytes()); // size of remaining record
        z.extend_from_slice(&45_u16.to_le_bytes()); // version made by
        z.extend_from_slice(&45_u16.to_le_bytes()); // version needed
        z.extend_from_slice(&0_u32.to_le_bytes()); // this disk
        z.extend_from_slice(&0_u32.to_le_bytes()); // cd start disk
        z.extend_from_slice(&1_u64.to_le_bytes()); // entries this disk
        z.extend_from_slice(&1_u64.to_le_bytes()); // entries total
        z.extend_from_slice(&cd_size.to_le_bytes()); // CD size (offset 40, §5.3)
        z.extend_from_slice(&cd_offset.to_le_bytes()); // CD offset (offset 48, §5.3)
        // -- ZIP64 EOCD locator (immediately precedes EOCD, §5.3) --
        z.extend_from_slice(&0x0706_4b50_u32.to_le_bytes());
        z.extend_from_slice(&0_u32.to_le_bytes()); // disk with ZIP64 EOCD
        z.extend_from_slice(&zip64_eocd_offset.to_le_bytes()); // record offset (byte 8)
        z.extend_from_slice(&1_u32.to_le_bytes()); // total disks
        // -- classic EOCD --
        z.extend_from_slice(&0x0605_4b50_u32.to_le_bytes());
        z.extend_from_slice(&0_u16.to_le_bytes()); // this disk
        z.extend_from_slice(&0_u16.to_le_bytes()); // cd disk
        z.extend_from_slice(&1_u16.to_le_bytes()); // entries this disk
        z.extend_from_slice(&1_u16.to_le_bytes()); // entries total
        z.extend_from_slice(&u32::try_from(cd_size).unwrap().to_le_bytes());
        z.extend_from_slice(&u32::try_from(cd_offset).unwrap().to_le_bytes());
        z.extend_from_slice(&0_u16.to_le_bytes()); // comment len
        z
    }

    /// Tiny table-free CRC-32 (IEEE) for fixture construction only.
    fn crc32(bytes: &[u8]) -> u32 {
        let mut crc = 0xFFFF_FFFF_u32;
        for &b in bytes {
            crc ^= u32::from(b);
            for _ in 0..8 {
                crc = (crc >> 1) ^ (0xEDB8_8320 & (0_u32.wrapping_sub(crc & 1)));
            }
        }
        !crc
    }

    async fn inspect_fake(bytes: Vec<u8>) -> (Result<Inspection, InspectError>, u32) {
        let mut src = FakeSource::new(bytes);
        let size = src.len();
        let result = inspect_zip(&mut src, size).await;
        (result, src.fetches)
    }

    #[tokio::test]
    async fn standard_small_zip_resolves_in_one_fetch() {
        // Under TAIL_LEN → the whole file arrives in round 1 (§5.2 small-file rule).
        let zip = stored_zip(&[("MAP01.WAD", b"wad bytes"), ("README.TXT", b"txt")], b"");
        let (result, fetches) = inspect_fake(zip).await;
        let i = result.unwrap();
        assert_eq!(fetches, 1);
        assert_eq!(i.member_count, 2);
        assert_eq!(i.wads.len(), 1);
        assert_eq!(i.wads[0].name, "MAP01.WAD");
        assert_eq!(i.wads[0].uncompressed, 9);
        assert_eq!(i.wads[0].method, "stored");
        assert!(!i.wads[0].encrypted);
        assert_eq!(i.other_members, vec!["README.TXT"]);
        assert!(!i.zip64);
    }

    #[tokio::test]
    async fn large_zip_with_tail_resident_cd_needs_second_fetch_for_wad_header() {
        // >TAIL_LEN file whose CD sits inside the tail: the CD parse itself
        // takes 0 extra fetches. But the file's one member is a `.wad`, and
        // (Step-1 finding, module docs) `by_index_raw` still eagerly parses
        // that member's *local* header to compute a data offset it never
        // uses — the local header for this single, first-and-only member
        // sits at offset 0, empirically outside the tail for a payload this
        // size (verified: file_size=204916, tail_start=137332,
        // local_header_start=0). That single extra fetch is real, necessary
        // work: it's the only way to get a correct `uncompressed` size, not
        // a caching miss to optimize away. 2 fetches total, not 1 — but the
        // second fetch must be a header-sized nibble, NOT the ~137 KiB gap
        // back to offset 0 (that gap is essentially this member's own
        // payload at corpus scale — the bug the coordinator flagged in fix
        // round 1). `bytes_served` proves the nibble, not the gap.
        let big = vec![0_u8; 200 * 1024];
        let zip = stored_zip(&[("LEVEL.WAD", big.as_slice())], b"");
        let mut src = FakeSource::new(zip);
        let size = src.len();
        let result = inspect_zip(&mut src, size).await;
        assert_eq!(
            src.fetches, 2,
            "CD in tail avoids a CD refetch; the lone .wad's local header does not"
        );
        assert!(
            src.bytes_served <= TAIL_LEN + LOCAL_HEADER_PAD,
            "second fetch must be a header-sized nibble, not the payload gap: bytes_served={}",
            src.bytes_served
        );
        assert_eq!(result.unwrap().wads[0].uncompressed, 200 * 1024);
    }

    #[tokio::test]
    async fn three_wad_headers_scattered_outside_tail_cost_one_fetch_each() {
        // Three `.wad` members, each with a ~100 KiB stored payload, so the
        // file is far bigger than TAIL_LEN and each member's local header
        // sits at a different offset outside the cached tail, while the CD
        // (three small entries) still resolves from the tail in round 1.
        // Each local-header miss must cost its own bounded nibble — this is
        // the multi-`.wad` corner the original report flagged as untested,
        // and the exact shape fix round 1's Finding 1 was about: a driver
        // that widened to "the whole gap" would transfer ~300 KiB here;
        // one that used "last miss wins" per full walk (the first attempted
        // fix) could take many more than 4 rounds to converge with several
        // scattered misses. Expect 4 fetches total (1 tail + 3 headers) and
        // bytes_served bounded by three header-sized nibbles, not the gaps.
        let payload = |n: usize| vec![0_u8; n];
        let zip = stored_zip(
            &[
                ("AAAA.WAD", payload(100 * 1024).as_slice()),
                ("BBBB.WAD", payload(100 * 1024).as_slice()),
                ("CCCC.WAD", payload(100 * 1024).as_slice()),
            ],
            b"",
        );
        assert!(
            u64::try_from(zip.len()).unwrap() > TAIL_LEN + 3 * LOCAL_HEADER_PAD,
            "fixture too small to prove the nibble bound"
        );
        let mut src = FakeSource::new(zip);
        let size = src.len();
        let result = inspect_zip(&mut src, size).await;
        let i = result.unwrap();
        assert_eq!(i.wads.len(), 3);
        for wad in &i.wads {
            assert_eq!(wad.uncompressed, 100 * 1024, "{}", wad.name);
        }
        assert_eq!(
            src.fetches, 4,
            "1 tail fetch + one nibble per scattered .wad header"
        );
        assert!(
            src.bytes_served <= TAIL_LEN + 3 * LOCAL_HEADER_PAD,
            "each miss must cost a nibble, not a gap: bytes_served={}",
            src.bytes_served
        );
    }

    #[tokio::test]
    async fn cd_outside_tail_takes_exactly_two_fetches() {
        // Many members → CD bigger than TAIL_LEN, so round 2 fetches the CD
        // extent. This is THE request-count regression (§9.1): the caching is
        // the point, and "3 requests instead of 2" is a real politeness bug.
        let names: Vec<String> = (0..1500)
            .map(|i| format!("{i:04}-a-fairly-long-member-name-padding.txt"))
            .collect();
        let members: Vec<(&str, &[u8])> = names.iter().map(|n| (n.as_str(), &b""[..])).collect();
        let zip = stored_zip(&members, b"");
        assert!(
            u64::try_from(zip.len()).unwrap() > 2 * TAIL_LEN,
            "fixture too small"
        );
        let (result, fetches) = inspect_fake(zip).await;
        let i = result.unwrap();
        assert_eq!(fetches, 2);
        assert_eq!(i.member_count, 1500);
        assert!(i.wads.is_empty()); // zero-`.wad` §5.5 case doubles up here
    }

    #[tokio::test]
    async fn worst_case_comment_zip_still_resolves() {
        // 65,535-byte comment (§9.1): EOCD sits 65,557 bytes from EOF — inside
        // TAIL_LEN by design.
        let zip = stored_zip(&[("E1M1.WAD", b"x")], &vec![b'c'; 65_535]);
        let (result, _fetches) = inspect_fake(zip).await;
        assert_eq!(result.unwrap().wads[0].name, "E1M1.WAD");
    }

    #[tokio::test]
    async fn zip64_fixture_resolves_sizes_and_flags() {
        let (result, _fetches) = inspect_fake(zip64_fixture()).await;
        let i = result.unwrap();
        assert!(i.zip64, "ZIP64 locator must be detected");
        assert_eq!(i.wads.len(), 1);
        // The §5.3 classic bug: reading the 0xFFFFFFFF sentinel instead of the
        // extra field. The crate must resolve 8, not u32::MAX.
        assert_eq!(i.wads[0].uncompressed, 8);
        assert_eq!(i.wads[0].compressed, 8);
    }

    #[tokio::test]
    async fn encrypted_member_is_recorded_not_skipped() {
        let mut zip = stored_zip(&[("SECRET.WAD", b"payload")], b"");
        set_encryption_bit(&mut zip);
        let (result, _fetches) = inspect_fake(zip).await;
        let i = result.unwrap();
        assert!(
            i.wads[0].encrypted,
            "§5.5: encryption recorded, not skipped"
        );
    }

    #[tokio::test]
    async fn non_deflate_method_is_labeled() {
        let mut zip = stored_zip(&[("ODD.WAD", b"payload")], b"");
        set_method(&mut zip, 12); // bzip2
        let (result, _fetches) = inspect_fake(zip).await;
        let i = result.unwrap();
        assert_ne!(i.wads[0].method, "stored");
        assert_ne!(i.wads[0].method, "deflate");
    }

    #[tokio::test]
    async fn multi_wad_and_case_insensitive_matching() {
        let zip = stored_zip(
            &[
                ("a.WAD", b"1"),
                ("b.Wad", b"22"),
                ("c.wad", b"333"),
                ("d.txt", b""),
            ],
            b"",
        );
        let (result, _fetches) = inspect_fake(zip).await;
        let i = result.unwrap();
        assert_eq!(i.wads.len(), 3, "§5.5: .WAD/.Wad/.wad all match");
        assert_eq!(i.other_members, vec!["d.txt"]);
    }

    #[tokio::test]
    async fn hostile_member_name_is_recorded_verbatim() {
        // §9.1: `../../etc/passwd`. We never extract, so the raw name is data,
        // not a path — it must land in other_members untouched, without panic.
        let mut zip = stored_zip(&[("AAAAAAAAAAAAAAAA", b"x")], b""); // 16 chars
        let hostile = b"../../etc/passwd"; // 16 chars — same length, byte-patched
        let pos = zip
            .windows(16)
            .position(|w| w == b"AAAAAAAAAAAAAAAA")
            .unwrap();
        zip[pos..pos + 16].copy_from_slice(hostile); // local header name
        let pos2 = pos
            + 16
            + zip[pos + 16..]
                .windows(16)
                .position(|w| w == b"AAAAAAAAAAAAAAAA")
                .unwrap();
        zip[pos2..pos2 + 16].copy_from_slice(hostile); // CD name
        let (result, _fetches) = inspect_fake(zip).await;
        let i = result.unwrap();
        assert_eq!(i.other_members, vec!["../../etc/passwd"]);
    }

    #[tokio::test]
    async fn garbage_bytes_are_a_parse_error_not_a_panic() {
        let (result, _fetches) = inspect_fake(b"MZ this is not a zip at all".to_vec()).await;
        assert!(matches!(result, Err(InspectError::Parse(_))));
    }

    #[tokio::test]
    async fn zero_length_file_is_a_parse_error_without_fetching() {
        let mut src = FakeSource::new(Vec::new());
        let result = inspect_zip(&mut src, 0).await;
        assert!(matches!(result, Err(InspectError::Parse(_))));
        assert_eq!(src.fetches, 0);
    }

    #[tokio::test]
    async fn cd_cap_rejects_oversized_directories() {
        // Cap parameterized for tests (mirror.rs read_cached_tree_with_cap
        // precedent): a CD extent above the cap must fail closed, not allocate.
        let names: Vec<String> = (0..1500)
            .map(|i| format!("{i:04}-a-fairly-long-member-name-padding.txt"))
            .collect();
        let members: Vec<(&str, &[u8])> = names.iter().map(|n| (n.as_str(), &b""[..])).collect();
        let zip = stored_zip(&members, b"");
        let mut src = FakeSource::new(zip);
        let size = src.len();
        let result = inspect_zip_with_caps(&mut src, size, 4 * 1024, MAX_FETCH_ROUNDS).await;
        assert!(matches!(result, Err(InspectError::CdTooLarge { .. })));
    }

    #[tokio::test]
    async fn round_cap_stops_a_chatty_pattern() {
        let names: Vec<String> = (0..1500)
            .map(|i| format!("{i:04}-a-fairly-long-member-name-padding.txt"))
            .collect();
        let members: Vec<(&str, &[u8])> = names.iter().map(|n| (n.as_str(), &b""[..])).collect();
        let zip = stored_zip(&members, b"");
        let mut src = FakeSource::new(zip);
        let size = src.len();
        // One round is not enough for an out-of-tail CD → TooChatty, cleanly.
        let result = inspect_zip_with_caps(&mut src, size, CD_CAP, 1).await;
        assert!(matches!(result, Err(InspectError::TooChatty { rounds: 1 })));
    }

    #[test]
    fn zip64_detection_reads_the_locator_not_luck() {
        assert!(zip64_present(&zip64_fixture()));
        let plain = stored_zip(&[("a.wad", b"x")], b"");
        assert!(!zip64_present(&plain));
        assert!(!zip64_present(b"too short"));
    }
}
