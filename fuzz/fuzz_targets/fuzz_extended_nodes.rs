#![no_main]
//! Fuzzes the ZDoom extended-node decoders (`map/extended.rs`, ADR-0025,
//! #326/#327) via the public assembly API.
//!
//! `decode_extended_nodes` / `decode_compressed_extended_nodes` are both
//! crate-private, so this target reaches them the same way a real caller
//! would: build a small, fixed, always-valid Doom map (two vertices, one
//! one-sided linedef, one sidedef, one sector) and substitute the fuzzer's
//! bytes as the `NODES` lump, prefixed with one of the eight dialect tags
//! chosen by the input's first byte (`% 8`): the four uncompressed
//! (`XNOD`/`XGLN`/`XGL2`/`XGL3`) and, since #327's `extended-nodes-zlib`
//! feature is enabled on this crate's `crustywad` dependency, the four
//! zlib-compressed twins (`ZNOD`/`ZGLN`/`ZGL2`/`ZGL3`). That reliably routes
//! the assembler's extended-node signature check (`assemble.rs`) into one of
//! the two decoders, instead of falling through to the classic
//! `SEGS`/`SSECTORS`/`NODES` triple — a raw whole-WAD fuzz target (like
//! `fuzz_assemble_map`) would only reach `map/extended.rs` by chance, since it
//! would also need to randomly assemble a valid map around it.
//! `Map::assemble_with_options` is then called in both [`Strictness`] modes.
//!
//! For the compressed `Z*` tags the fuzzer's bytes after the selector are the
//! raw (candidate) zlib stream: most inputs fail to inflate and surface a
//! clean `Err` (`CorruptStream`), while a valid zlib seed inflates to a tagless
//! body decoded by the very same parser the uncompressed path uses. The harness
//! caps the inflate output via `Limits::max_decoded_node_bytes` (a modest
//! `MAX_DECODED` here) so a "zip bomb" seed hits `DecodedSizeExceeded`, not
//! unbounded work.
//!
//! Oracle: no panic in either mode (ADR-0016 §2 — each decoder is a single
//! forward pass with an explicit budget check per section, never recursive;
//! the compressed path adds only a bounded-output inflate), and the decoded
//! BSP arenas are `O(input)` (ADR-0016 §1) — on a successful decode,
//! `segs().len() + subsectors().len() + nodes().len()` is bounded by the number
//! of bytes the decoder actually reads: the stream length itself for the
//! uncompressed path, or the inflate cap `MAX_DECODED` for the compressed path
//! (each record consumes several bytes of that body, so this generous per-item
//! bound holds without needing the exact per-dialect record size).
//!
//! Corpus seeds (`fuzz/corpus/fuzz_extended_nodes/`) match this harness's
//! input model: a leading selector byte whose `% 8` picks the dialect,
//! followed by the dialect-appropriate body. For the uncompressed tags the
//! body is the *tagless* stream (the real 4-byte tag is synthesized here, not
//! read from the input, so a seed must not carry its own tag or it collapses
//! into the wrong branch); for the compressed tags it is the *zlib stream* of
//! that tagless body. Seeds are lifted from `map/extended.rs`'s unit-test
//! fixtures: `seed_xnod.bin` selector `0x00` (`0 % 8 == 0` -> XNOD),
//! `seed_xgln.bin` selector `0x01` (-> XGLN), `seed_xgl2.bin` selector `0x02`
//! (-> XGL2), `seed_xgl3_node.bin` selector `0x03` (-> XGL3); and the
//! compressed twins `seed_znod.bin` selector `0x04` (-> ZNOD), `seed_zgln.bin`
//! selector `0x05` (-> ZGLN), `seed_zgl2.bin` selector `0x06` (-> ZGL2),
//! `seed_zgl3.bin` selector `0x07` (-> ZGL3).

use libfuzzer_sys::fuzz_target;

use crustywad::map::Map;
use crustywad::{ParseOptions, Wad};

/// Appends one lump's directory entry and payload to `payload`/`directory`.
fn push_lump(payload: &mut Vec<u8>, directory: &mut Vec<u8>, name: &str, bytes: &[u8]) {
    let filepos = 12 + payload.len();
    payload.extend_from_slice(bytes);
    directory.extend_from_slice(
        &i32::try_from(filepos)
            .expect("fixture payload stays within i32 range (the fuzz body caps input length)")
            .to_le_bytes(),
    );
    directory.extend_from_slice(
        &i32::try_from(bytes.len())
            .expect("fixture lump stays within i32 range (the fuzz body caps input length)")
            .to_le_bytes(),
    );
    let mut encoded = [0u8; 8];
    for (slot, byte) in name.as_bytes().iter().take(8).enumerate() {
        encoded[slot] = *byte;
    }
    directory.extend_from_slice(&encoded);
}

/// Builds a minimal, always-valid Doom-format map PWAD (`MAP01`) whose
/// `NODES` lump is `nodes_bytes` — the fuzzer-controlled extended-node
/// stream, tag included.
fn build_map_wad(nodes_bytes: &[u8]) -> Vec<u8> {
    // Two vertices: (0,0) and (64,0).
    let vertexes: Vec<u8> = [0i16, 0, 64, 0]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();

    // One 30-byte sidedef: no offsets, "-" textures (none), sector 0.
    let mut sidedef = vec![0u8; 4];
    sidedef.extend_from_slice(b"-\0\0\0\0\0\0\0");
    sidedef.extend_from_slice(b"-\0\0\0\0\0\0\0");
    sidedef.extend_from_slice(b"-\0\0\0\0\0\0\0");
    sidedef.extend_from_slice(&0u16.to_le_bytes());

    // One 26-byte sector: floor 0, ceiling 128, "-" flats, light 160, no
    // special/tag.
    let mut sector = Vec::new();
    sector.extend_from_slice(&0i16.to_le_bytes());
    sector.extend_from_slice(&128i16.to_le_bytes());
    sector.extend_from_slice(b"-\0\0\0\0\0\0\0");
    sector.extend_from_slice(b"-\0\0\0\0\0\0\0");
    sector.extend_from_slice(&160i16.to_le_bytes());
    sector.extend_from_slice(&0i16.to_le_bytes());
    sector.extend_from_slice(&0i16.to_le_bytes());

    // One 14-byte Doom linedef: v1=0, v2=1, impassable + one-sided
    // (right=0, left=0xffff).
    let mut linedef = Vec::new();
    linedef.extend_from_slice(&0u16.to_le_bytes()); // start_vertex
    linedef.extend_from_slice(&1u16.to_le_bytes()); // end_vertex
    linedef.extend_from_slice(&1u16.to_le_bytes()); // flags
    linedef.extend_from_slice(&0u16.to_le_bytes()); // special_type
    linedef.extend_from_slice(&0u16.to_le_bytes()); // sector_tag
    linedef.extend_from_slice(&0u16.to_le_bytes()); // right_sidedef
    linedef.extend_from_slice(&0xffffu16.to_le_bytes()); // left_sidedef

    let mut payload = Vec::new();
    let mut directory = Vec::new();
    for (name, bytes) in [
        ("MAP01", &[][..]),
        ("THINGS", &[][..]),
        ("LINEDEFS", &linedef[..]),
        ("SIDEDEFS", &sidedef[..]),
        ("VERTEXES", &vertexes[..]),
        ("SECTORS", &sector[..]),
        ("NODES", nodes_bytes),
    ] {
        push_lump(&mut payload, &mut directory, name, bytes);
    }

    let lump_count = 7i32;
    let directory_offset =
        i32::try_from(12 + payload.len()).expect("fixture payload stays within i32 range (the fuzz body caps input length)");

    let mut wad = Vec::new();
    wad.extend_from_slice(b"PWAD");
    wad.extend_from_slice(&lump_count.to_le_bytes());
    wad.extend_from_slice(&directory_offset.to_le_bytes());
    wad.extend_from_slice(&payload);
    wad.extend_from_slice(&directory);
    wad
}

/// The inflate output cap the harness pins into `Limits::max_decoded_node_bytes`
/// for the compressed path — modest, so a highly-compressible ("zip bomb") seed
/// trips `DecodedSizeExceeded` cheaply instead of inflating to the 64 MiB
/// production default. It is also the `O(input)` output bound for that path.
const MAX_DECODED: usize = 1 << 20;

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }
    // Bound the input: the synthetic WAD's directory offsets/sizes are `i32`
    // (`build_map_wad`), so an over-large stream would overflow the `try_from`
    // conversions there — a HARNESS panic that would masquerade as a
    // no-panic-oracle failure of the decoder under test. Real node streams are
    // far below this; the cap keeps every size within `i32` by construction.
    if data.len() > (1usize << 24) {
        return;
    }

    // Cycle through all eight dialects by the input's first byte: `0..=3` the
    // uncompressed `X*` family, `4..=7` the zlib-compressed `Z*` twins. The rest
    // of the input becomes the tag-less stream (X*) or the raw zlib stream (Z*)
    // that follows the synthesized 4-byte tag.
    let (tag, compressed): (&[u8; 4], bool) = match data[0] % 8 {
        0 => (b"XNOD", false),
        1 => (b"XGLN", false),
        2 => (b"XGL2", false),
        3 => (b"XGL3", false),
        4 => (b"ZNOD", true),
        5 => (b"ZGLN", true),
        6 => (b"ZGL2", true),
        _ => (b"ZGL3", true),
    };
    let mut nodes_bytes = Vec::with_capacity(4 + data.len() - 1);
    nodes_bytes.extend_from_slice(tag);
    nodes_bytes.extend_from_slice(&data[1..]);

    let wad_bytes = build_map_wad(&nodes_bytes);

    // The `O(input)` output bound (ADR-0016 §1): for the uncompressed path each
    // arena element consumes several bytes of the stream itself; for the
    // compressed path the decoder reads the *inflated* body, which the cap
    // bounds by `MAX_DECODED` regardless of the (smaller) compressed input.
    let output_bound = if compressed {
        MAX_DECODED
    } else {
        nodes_bytes.len()
    };

    for base in [ParseOptions::strict(), ParseOptions::lenient()] {
        // Pin the inflate cap so a compressible seed can't balloon the decode.
        let mut options = base;
        options.limits = options.limits.with_max_decoded_node_bytes(MAX_DECODED);

        // The fixed map bytes around the fuzzed NODES lump are always valid,
        // but the container parse is kept in the loop (rather than hoisted
        // out and shared) so a future change to either parser can't
        // silently stop exercising this path.
        let Ok(wad) = Wad::from_bytes_with_options(wad_bytes.clone(), options) else {
            continue;
        };
        for group in wad.map_groups() {
            let Ok(map) = Map::assemble_with_options(&wad, &group, options) else {
                // A structural fault in the fuzzed stream (truncation, a bad
                // count, a dangling reference in strict mode, an un-inflatable
                // or over-cap compressed stream, ...) is an expected, clean
                // `Err` — not a panic.
                continue;
            };

            // O(input) bound (ADR-0016 §1): every arena element consumes at
            // least several bytes of the bytes the decoder reads, so a
            // generous per-element bound is that read length.
            let element_count = map.segs().len() + map.subsectors().len() + map.nodes().len();
            assert!(
                element_count <= output_bound,
                "decoded element count {element_count} exceeds O(input) bound (bound {output_bound})",
            );

            std::hint::black_box(&map);
        }
    }
});
