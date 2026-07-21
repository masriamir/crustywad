#![no_main]
//! Fuzzes the uncompressed ZDoom extended-node decoder (`map/extended.rs`,
//! ADR-0025, #326) via the public assembly API.
//!
//! `decode_extended_nodes` itself is crate-private, so this target reaches it
//! the same way a real caller would: build a small, fixed, always-valid Doom
//! map (two vertices, one one-sided linedef, one sidedef, one sector) and
//! substitute the fuzzer's bytes as the `NODES` lump, prefixed with one of
//! the four uncompressed dialect tags (`XNOD`/`XGLN`/`XGL2`/`XGL3`, chosen by
//! the input's first byte). That reliably routes the assembler's
//! extended-node signature check (`assemble.rs`) into the decoder, instead of
//! falling through to the classic `SEGS`/`SSECTORS`/`NODES` triple or the
//! still-gated `Z*`/`xNd4` path (#327/#328) — a raw whole-WAD fuzz target
//! (like `fuzz_assemble_map`) would only reach `map/extended.rs` by chance,
//! since it would also need to randomly assemble a valid map around it.
//! `Map::assemble_with_options` is then called in both [`Strictness`] modes.
//!
//! Oracle: no panic in either mode (ADR-0016 §2 — the decoder is a single
//! forward pass with an explicit budget check per section, never recursive),
//! and the decoded BSP arenas are `O(input)` (ADR-0016 §1) — on a successful
//! decode, `segs().len() + subsectors().len() + nodes().len()` is bounded by
//! the extended-node stream length fed to the decoder (each record consumes
//! several bytes of the stream, so this generous per-item bound holds
//! without needing the exact per-dialect record size).

use libfuzzer_sys::fuzz_target;

use crustywad::map::Map;
use crustywad::{ParseOptions, Wad};

/// Appends one lump's directory entry and payload to `payload`/`directory`.
fn push_lump(payload: &mut Vec<u8>, directory: &mut Vec<u8>, name: &str, bytes: &[u8]) {
    let filepos = 12 + payload.len();
    payload.extend_from_slice(bytes);
    directory.extend_from_slice(
        &i32::try_from(filepos)
            .expect("fixture payload stays well within i32 range")
            .to_le_bytes(),
    );
    directory.extend_from_slice(
        &i32::try_from(bytes.len())
            .expect("fixture lump stays well within i32 range")
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
        i32::try_from(12 + payload.len()).expect("fixture payload stays well within i32 range");

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

    // Cycle through the four decodable uncompressed dialects by the input's
    // first byte; the rest of the input becomes the stream body after the
    // tag.
    let tag: &[u8; 4] = match data[0] % 4 {
        0 => b"XNOD",
        1 => b"XGLN",
        2 => b"XGL2",
        _ => b"XGL3",
    };
    let mut nodes_bytes = Vec::with_capacity(4 + data.len() - 1);
    nodes_bytes.extend_from_slice(tag);
    nodes_bytes.extend_from_slice(&data[1..]);

    let wad_bytes = build_map_wad(&nodes_bytes);

    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
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
                // count, a dangling reference in strict mode, ...) is an
                // expected, clean `Err` — not a panic.
                continue;
            };

            // O(input) bound (ADR-0016 §1): every arena element consumes at
            // least several bytes of the extended-node stream, so a
            // generous per-element bound is the stream's own length.
            let element_count = map.segs().len() + map.subsectors().len() + map.nodes().len();
            assert!(
                element_count <= nodes_bytes.len(),
                "decoded element count {element_count} exceeds O(input) bound (stream length {})",
                nodes_bytes.len()
            );

            std::hint::black_box(&map);
        }
    }
});
