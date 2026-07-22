#![no_main]
//! Fuzzes the `DeePBSP` v4 (`xNd4`) node decoder (`map/deepbsp.rs`, ADR-0025
//! Stage 3, #328) via the public assembly API.
//!
//! `decode_deepbsp` is crate-private, so this target reaches it the same way a
//! real caller would: build a small, fixed, always-valid Doom map (two
//! vertices, one one-sided linedef, one sidedef, one sector) and substitute the
//! fuzzer's bytes as the three classic-family BSP lumps. The fuzzer input is
//! split into three slices used as `SEGS`, `SSECTORS`, and the post-signature
//! `NODES` body; the `NODES` lump is always prefixed with the 8-byte
//! `xNd4\0\0\0\0` signature so the assembler's gate (`assemble.rs`) routes into
//! `decode_deepbsp` (ahead of the 4-byte ZDoom extended-signature check) rather
//! than falling through to the classic decoders.
//! `Map::assemble_with_options` is then called in both [`Strictness`] modes.
//!
//! Oracle: no panic in either mode (ADR-0016 §2 — the decoder is a single
//! forward pass: `parse_records` over each lump, then an iterative normalize;
//! the BSP tree is stored, not walked, so no crafted input can recurse), and
//! the decoded BSP arenas are `O(input)` (ADR-0016 §1) — on a successful decode,
//! `segs().len() + subsectors().len() + nodes().len()` is bounded by the total
//! fuzzer input length, since every record consumes at least six bytes of a
//! lump (the smallest, `SSECTORS`, is 6 bytes; `SEGS` 16; `NODES` 32).
//!
//! Corpus seeds (`fuzz/corpus/fuzz_deepbsp/`) match this harness's input model:
//! two leading length bytes (`SEGS` length, `SSECTORS` length) then the record
//! body (`SEGS` records, `SSECTORS` records, post-signature `NODES` records).
//! Non-hex seed names describe their shape (`seed_square.bin` a valid one-node
//! map, `seed_degenerate.bin` a signature-only NODES lump).

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
/// `SEGS`/`SSECTORS`/`NODES` lumps are the fuzzer-controlled DeePBSP records.
fn build_map_wad(segs: &[u8], ssectors: &[u8], nodes: &[u8]) -> Vec<u8> {
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

    // One 26-byte sector: floor 0, ceiling 128, "-" flats, light 160.
    let mut sector = Vec::new();
    sector.extend_from_slice(&0i16.to_le_bytes());
    sector.extend_from_slice(&128i16.to_le_bytes());
    sector.extend_from_slice(b"-\0\0\0\0\0\0\0");
    sector.extend_from_slice(b"-\0\0\0\0\0\0\0");
    sector.extend_from_slice(&160i16.to_le_bytes());
    sector.extend_from_slice(&0i16.to_le_bytes());
    sector.extend_from_slice(&0i16.to_le_bytes());

    // One 14-byte Doom linedef: v1=0, v2=1, one-sided (right=0, left=0xffff).
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
        ("SEGS", segs),
        ("SSECTORS", ssectors),
        ("NODES", nodes),
    ] {
        push_lump(&mut payload, &mut directory, name, bytes);
    }

    let lump_count = 9i32;
    let directory_offset = i32::try_from(12 + payload.len())
        .expect("fixture payload stays within i32 range (the fuzz body caps input length)");

    let mut wad = Vec::new();
    wad.extend_from_slice(b"PWAD");
    wad.extend_from_slice(&lump_count.to_le_bytes());
    wad.extend_from_slice(&directory_offset.to_le_bytes());
    wad.extend_from_slice(&payload);
    wad.extend_from_slice(&directory);
    wad
}

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }
    // Bound the input: the synthetic WAD's directory offsets/sizes are `i32`
    // (`build_map_wad`), so an over-large stream would overflow the `try_from`
    // conversions there — a HARNESS panic that would masquerade as a
    // no-panic-oracle failure of the decoder under test. Real node lumps are
    // far below this cap.
    if data.len() > (1usize << 24) {
        return;
    }

    // Split the input into three slices for SEGS / SSECTORS / the NODES body.
    // The first two bytes pick the SEGS and SSECTORS lengths; the remaining
    // body is carved into those two lumps and the post-signature NODES body.
    // All three are arbitrary so the decoder sees fuzzed record framing on
    // every lump.
    if data.len() < 2 {
        return;
    }
    let body = &data[2..];
    let seg_len = usize::from(data[0]).min(body.len());
    let (segs, rest) = body.split_at(seg_len);
    let ss_len = usize::from(data[1]).min(rest.len());
    let (ssectors, nodes_body) = rest.split_at(ss_len);

    // The NODES lump always carries the 8-byte DeePBSP signature so detection
    // routes to `decode_deepbsp`.
    let mut nodes = b"xNd4\0\0\0\0".to_vec();
    nodes.extend_from_slice(nodes_body);

    let wad_bytes = build_map_wad(segs, ssectors, &nodes);

    // O(input) bound (ADR-0016 §1): each arena element consumes at least six
    // bytes of a lump (SSECTORS record = 6, SEGS = 16, NODES = 32), so the
    // total element count cannot exceed the fuzzer input length.
    let output_bound = data.len() + 8;

    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        // The fixed map bytes around the fuzzed NODES lump are always valid,
        // but the container parse is kept in the loop (rather than hoisted out
        // and shared) so a future change to either parser can't silently stop
        // exercising this path — matching `fuzz_extended_nodes`. The cost is
        // negligible: `Wad::from_bytes` parses only the small fixed directory
        // and loads lumps lazily, so this is O(directory), not O(NODES-lump),
        // regardless of the fuzzed input size.
        let Ok(wad) = Wad::from_bytes_with_options(wad_bytes.clone(), options) else {
            continue;
        };
        for group in wad.map_groups() {
            let Ok(map) = Map::assemble_with_options(&wad, &group, options) else {
                // A framing defect or a strict-mode dangling reference is an
                // expected, clean `Err` — not a panic.
                continue;
            };

            let element_count = map.segs().len() + map.subsectors().len() + map.nodes().len();
            assert!(
                element_count <= output_bound,
                "decoded element count {element_count} exceeds O(input) bound (bound {output_bound})",
            );

            std::hint::black_box(&map);
        }
    }
});
