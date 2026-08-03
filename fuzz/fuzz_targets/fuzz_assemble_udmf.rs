#![no_main]
use libfuzzer_sys::fuzz_target;

use crustywad::map::{Map, MapFormat};
use crustywad::{ParseOptions, Wad};

// Minimal PWAD: [MAP01][TEXTMAP=data][ENDMAP]. WAD directory offsets/sizes are
// `i32`, so an input too large to encode returns `None` and the harness skips it
// rather than building a truncated, malformed WAD that wouldn't exercise the
// UDMF pipeline. Every conversion is fallible — no lossy `as` casts.
fn wrap_textmap(data: &[u8]) -> Option<Vec<u8>> {
    let lumps: [(&str, &[u8]); 3] = [("MAP01", b""), ("TEXTMAP", data), ("ENDMAP", b"")];
    let mut payload = Vec::new();
    let mut dir = Vec::new();
    for (name, bytes) in lumps {
        let filepos = i32::try_from(12 + payload.len()).ok()?;
        let size = i32::try_from(bytes.len()).ok()?;
        payload.extend_from_slice(bytes);
        dir.extend_from_slice(&filepos.to_le_bytes());
        dir.extend_from_slice(&size.to_le_bytes());
        let mut n = [0u8; 8];
        for (s, b) in n.iter_mut().zip(name.bytes()) {
            *s = b;
        }
        dir.extend_from_slice(&n);
    }
    let count = i32::try_from(lumps.len()).ok()?;
    let dir_offset = i32::try_from(12 + payload.len()).ok()?;
    let mut wad = Vec::with_capacity(12 + payload.len() + dir.len());
    wad.extend_from_slice(b"PWAD");
    wad.extend_from_slice(&count.to_le_bytes());
    wad.extend_from_slice(&dir_offset.to_le_bytes());
    wad.extend_from_slice(&payload);
    wad.extend_from_slice(&dir);
    Some(wad)
}

fuzz_target!(|data: &[u8]| {
    let Some(wad_bytes) = wrap_textmap(data) else {
        return;
    };
    if let Ok(wad) = Wad::from_bytes_with_options(wad_bytes, ParseOptions::lenient())
        && let Some(group) = wad.map_group("MAP01")
        && let Ok(map) = Map::assemble_with_options(&wad, &group, ParseOptions::lenient())
    {
        if map.format() == MapFormat::Udmf {
            // O(input) invariant (ADR-0016 §1): each Map element derives from a
            // parsed UDMF block, each backed by >= one `{`...`}` in the input text.
            let elements = map.vertices().len()
                + map.linedefs().len()
                + map.sidedefs().len()
                + map.sectors().len()
                + map.things().len();
            assert!(
                elements <= data.len(),
                "element count {elements} exceeds O(input) bound {}",
                data.len()
            );
        }
        std::hint::black_box(&map);
    }
});
