//! Reads a synthetic WAD from memory and inspects its lumps.
//!
//! Run with: `cargo run -p crustywad --example read_wad`

use crustywad::Wad;

fn main() {
    // Build a minimal WAD in memory: one lump named "TEST".
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"IWAD");
    bytes.extend_from_slice(&1_i32.to_le_bytes()); // numlumps
    bytes.extend_from_slice(&16_i32.to_le_bytes()); // infotableofs
    bytes.extend_from_slice(&[1, 2, 3, 4]); // lump data at offset 12
    bytes.extend_from_slice(&12_i32.to_le_bytes()); // directory: filepos
    bytes.extend_from_slice(&4_i32.to_le_bytes()); // directory: size
    bytes.extend_from_slice(b"TEST\0\0\0\0"); // directory: name

    let wad = Wad::from_bytes(bytes).expect("valid synthetic WAD");

    println!("kind: {:?}", wad.kind());
    println!("lump count: {}", wad.lump_count());

    for lump in wad.lumps() {
        let data = wad.lump_data(lump);
        println!("  {} — {} bytes: {data:?}", lump.name(), lump.size());
    }
}
