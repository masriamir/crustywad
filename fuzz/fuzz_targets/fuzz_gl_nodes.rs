#![no_main]
//! Fuzzes the classic GL node group decoder (`GL_VERT`/`GL_SEGS`/`GL_SSECT`/
//! `GL_NODES`, `map/gl.rs`, #324) via the public assembly API.
//!
//! `decode_gl_group` is crate-private, so this target reaches it the same way a
//! real caller would: build a small, fixed, always-valid Doom map (two
//! vertices, one one-sided linedef, one sidedef, one sector) whose vanilla
//! `SEGS`/`SSECTORS`/`NODES` lumps are EMPTY — an empty vanilla BSP assembles
//! cleanly in both `Strictness` modes (there are no records to normalize), so
//! assembly reaches the GL step in both modes without erroring on the vanilla
//! graph. A `GL_MAP01` marker lump (empty payload) then triggers
//! `gl_group_for`'s detection of a classic GL node group, followed by four
//! lumps whose bytes are entirely fuzzer-controlled: `GL_VERT`, `GL_SEGS`,
//! `GL_SSECT`, `GL_NODES`.
//!
//! The fuzzer input is split into four slices: the first three bytes each pick
//! one of `GL_VERT`'s, `GL_SEGS`'s, and `GL_SSECT`'s lengths (capped to the
//! remaining body), generalizing `fuzz_deepbsp`'s two-byte split; the
//! remaining body becomes `GL_NODES`. All four lumps — including the
//! `GL_VERT` magic bytes — are fuzzer bytes, so the target exercises
//! `detect_gl_version` (V2/V3/V5, and the refused V1/V4 paths), the V3
//! `gNd3`-header-stripping path on `GL_SEGS`/`GL_SSECT`, and all four record
//! decoders. `Map::assemble_with_options` is then called in both
//! [`Strictness`] modes.
//!
//! Oracle: no panic in either mode (ADR-0016 §2 — `decode_gl_group` is a
//! sequential, iterative decode: `detect_gl_version`, then four fixed-record
//! passes (`as_chunks`/`chunks_exact`/`parse_records`) over the lumps in
//! dependency order;
//! the GL BSP tree is stored, not walked, so no crafted input can recurse),
//! and the decoded GL arenas are `O(input)` (ADR-0016 §1) — on a successful
//! decode, `gl_vertices().len() + gl_segs().len() + gl_subsectors().len() +
//! gl_nodes().len()` is bounded by the total fuzzer input length. Every GL
//! record consumes at least four bytes of *some* lump (the smallest, a V2
//! `GL_SSECT` record, is 4 bytes; `GL_VERT` records are 8; V2 `GL_SEGS`
//! records are 10; V2/V3 `GL_NODES` records are 28; V3/V5 `GL_SSECT` records
//! are 8; V3/V5 `GL_SEGS` and V5 `GL_NODES` records are larger still), and the
//! four lumps together hold at most `data.len()` bytes (they are carved out of
//! `data` alongside the three length-prefix bytes), so the combined element
//! count across all four arenas cannot exceed the fuzzer input length.
//!
//! Corpus seeds (`fuzz/corpus/fuzz_gl_nodes/`) match this harness's input
//! model: three leading length bytes (`GL_VERT`, `GL_SEGS`, `GL_SSECT`
//! lengths) then the four lump bodies in order. Non-hex seed names describe
//! their shape (`seed_v2_square.bin` a valid single-node V2 GL group —
//! `tests/gl_nodes.rs`'s `v2_gl_lumps` fixture re-encoded in this harness's
//! input model; `seed_refused_v4.bin` a `gNd4`-tagged `GL_VERT` that is
//! refused rather than decoded).

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

/// Builds a minimal, always-valid Doom-format map PWAD (`MAP01`) with an EMPTY
/// vanilla BSP (`SEGS`/`SSECTORS`/`NODES`) and a `GL_MAP01` node group whose
/// four lumps are the fuzzer-controlled GL records.
fn build_map_wad(gl_vert: &[u8], gl_segs: &[u8], gl_ssect: &[u8], gl_nodes: &[u8]) -> Vec<u8> {
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
        ("SEGS", &[][..]),
        ("SSECTORS", &[][..]),
        ("NODES", &[][..]),
        ("GL_MAP01", &[][..]),
        ("GL_VERT", gl_vert),
        ("GL_SEGS", gl_segs),
        ("GL_SSECT", gl_ssect),
        ("GL_NODES", gl_nodes),
    ] {
        push_lump(&mut payload, &mut directory, name, bytes);
    }

    let lump_count = 14i32;
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
    if data.len() < 3 {
        return;
    }
    // Bound the input: the synthetic WAD's directory offsets/sizes are `i32`
    // (`build_map_wad`), so an over-large stream would overflow the `try_from`
    // conversions there — a HARNESS panic that would masquerade as a
    // no-panic-oracle failure of the decoder under test. Real GL node lumps
    // are far below this cap.
    if data.len() > (1usize << 24) {
        return;
    }

    // Split the input into four slices for GL_VERT / GL_SEGS / GL_SSECT /
    // GL_NODES. The first three bytes pick the GL_VERT, GL_SEGS, and GL_SSECT
    // lengths; the remaining body is carved into those three lumps and the
    // GL_NODES lump. All four are arbitrary — including the GL_VERT magic
    // bytes — so the decoder sees fuzzed version detection and record framing
    // on every lump.
    let body = &data[3..];
    let vert_len = usize::from(data[0]).min(body.len());
    let (gl_vert, rest) = body.split_at(vert_len);
    let segs_len = usize::from(data[1]).min(rest.len());
    let (gl_segs, rest) = rest.split_at(segs_len);
    let ssect_len = usize::from(data[2]).min(rest.len());
    let (gl_ssect, gl_nodes) = rest.split_at(ssect_len);

    let wad_bytes = build_map_wad(gl_vert, gl_segs, gl_ssect, gl_nodes);

    // O(input) bound (ADR-0016 §1): every GL record consumes at least four
    // bytes of a lump (the smallest, a V2 GL_SSECT record, is 4 bytes), and
    // the four lumps together hold at most `data.len()` bytes (they're carved
    // out of `data` alongside the three length-prefix bytes), so the combined
    // decoded element count across all four arenas cannot exceed the fuzzer
    // input length.
    let output_bound = data.len() + 8;

    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        // The fixed map bytes around the fuzzed GL lumps are always valid, but
        // the container parse is kept in the loop (rather than hoisted out
        // and shared) so a future change to either parser can't silently stop
        // exercising this path — matching `fuzz_deepbsp`/`fuzz_extended_nodes`.
        // The cost is negligible: `Wad::from_bytes_with_options` parses only
        // the small fixed directory and loads lumps lazily, so this is
        // O(directory), not O(GL-lump), regardless of the fuzzed input size.
        let Ok(wad) = Wad::from_bytes_with_options(wad_bytes.clone(), options) else {
            continue;
        };
        for group in wad.map_groups() {
            let Ok(map) = Map::assemble_with_options(&wad, &group, options) else {
                // A framing defect or a strict-mode dangling reference is an
                // expected, clean `Err` — not a panic.
                continue;
            };

            let element_count = map.gl_vertices().len()
                + map.gl_segs().len()
                + map.gl_subsectors().len()
                + map.gl_nodes().len();
            assert!(
                element_count <= output_bound,
                "decoded GL element count {element_count} exceeds O(input) bound (bound {output_bound})",
            );

            std::hint::black_box(&map);
        }
    }
});
