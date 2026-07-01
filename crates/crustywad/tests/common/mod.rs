use std::collections::HashMap;

use proptest::prelude::*;

fn encode_i32(value: usize) -> [u8; 4] {
    i32::try_from(value)
        .expect("test fixture values should fit within i32")
        .to_le_bytes()
}

pub fn build_wad(kind: [u8; 4], lumps: &[(&str, &[u8])]) -> Vec<u8> {
    let mut payload = Vec::new();
    let mut directory = Vec::new();
    let directory_offset = 12 + lumps.iter().map(|(_, bytes)| bytes.len()).sum::<usize>();

    for (name, bytes) in lumps {
        let filepos = 12 + payload.len();
        payload.extend_from_slice(bytes);
        directory.extend_from_slice(&encode_i32(filepos));
        directory.extend_from_slice(&encode_i32(bytes.len()));
        let mut encoded = [0_u8; 8];
        for (slot, byte) in name.as_bytes().iter().take(8).enumerate() {
            encoded[slot] = *byte;
        }
        directory.extend_from_slice(&encoded);
    }

    let mut wad = Vec::new();
    wad.extend_from_slice(&kind);
    wad.extend_from_slice(&encode_i32(lumps.len()));
    wad.extend_from_slice(&encode_i32(directory_offset));
    wad.extend_from_slice(&payload);
    wad.extend_from_slice(&directory);
    wad
}

#[allow(dead_code)]
pub fn lump_map<'a>(pairs: &'a [(&'a str, &'a [u8])]) -> HashMap<&'a str, &'a [u8]> {
    pairs.iter().copied().collect()
}

/// Generates an ASCII lump name (1–8 chars) and a payload (0–256 bytes).
#[allow(dead_code)]
pub fn arb_lump_pair() -> impl Strategy<Value = (String, Vec<u8>)> {
    let name = proptest::string::string_regex("[A-Z0-9_]{1,8}").unwrap();
    let data = proptest::collection::vec(any::<u8>(), 0..=256);
    (name, data)
}

/// Generates structurally valid WAD bytes (correct header offsets, ASCII names).
#[allow(dead_code)]
pub fn arb_valid_wad() -> impl Strategy<Value = Vec<u8>> {
    let kind = prop_oneof![Just(*b"IWAD"), Just(*b"PWAD"),];
    let lumps = proptest::collection::vec(arb_lump_pair(), 0..=16);
    (kind, lumps).prop_map(|(k, pairs)| {
        let refs: Vec<(&str, &[u8])> = pairs
            .iter()
            .map(|(n, d)| (n.as_str(), d.as_slice()))
            .collect();
        build_wad(k, &refs)
    })
}
