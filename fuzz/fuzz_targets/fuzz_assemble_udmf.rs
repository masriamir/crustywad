#![no_main]
use libfuzzer_sys::fuzz_target;

use crustywad::map::{Map, MapFormat};
use crustywad::{ParseOptions, Wad};

// Minimal PWAD: [MAP01][TEXTMAP=data][ENDMAP].
fn wrap_textmap(data: &[u8]) -> Vec<u8> {
    let lumps: [(&str, &[u8]); 3] = [("MAP01", b""), ("TEXTMAP", data), ("ENDMAP", b"")];
    let mut payload = Vec::new();
    let mut dir = Vec::new();
    for (name, bytes) in lumps {
        let filepos = 12 + payload.len();
        payload.extend_from_slice(bytes);
        dir.extend_from_slice(&(filepos as i32).to_le_bytes());
        dir.extend_from_slice(&(bytes.len() as i32).to_le_bytes());
        let mut n = [0u8; 8];
        for (s, b) in n.iter_mut().zip(name.bytes()) { *s = b; }
        dir.extend_from_slice(&n);
    }
    let mut wad = Vec::with_capacity(12 + payload.len() + dir.len());
    wad.extend_from_slice(b"PWAD");
    wad.extend_from_slice(&(lumps.len() as i32).to_le_bytes());
    wad.extend_from_slice(&((12 + payload.len()) as i32).to_le_bytes());
    wad.extend_from_slice(&payload);
    wad.extend_from_slice(&dir);
    wad
}

fuzz_target!(|data: &[u8]| {
    let wad_bytes = wrap_textmap(data);
    if let Ok(wad) = Wad::from_bytes_with_options(wad_bytes, ParseOptions::lenient()) {
        if let Some(group) = wad.map_group("MAP01") {
            if let Ok(map) = Map::assemble_with_options(&wad, &group, ParseOptions::lenient()) {
                if map.format() == MapFormat::Udmf {
                    // O(input) invariant (ADR-0016 §1): each Map element derives from a
                    // parsed UDMF block, each backed by >= one `{`...`}` in the input text.
                    let elements = map.vertices().len() + map.linedefs().len()
                        + map.sidedefs().len() + map.sectors().len() + map.things().len();
                    assert!(elements <= data.len(),
                        "element count {elements} exceeds O(input) bound {}", data.len());
                }
                std::hint::black_box(&map);
            }
        }
    }
});
