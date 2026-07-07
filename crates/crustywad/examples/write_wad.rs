//! Builds a WAD from scratch, then round-trips it through `Wad::to_builder()`.
//!
//! Run with: `cargo run -p crustywad --example write_wad --features write`

use crustywad::{Wad, WadBuilder, WadKind, WriteOptions};

fn main() {
    // Build a new PWAD with two lumps.
    let bytes = WadBuilder::new(WadKind::Pwad)
        .add_lump("MAP01", b"")
        .add_lump("TEST", vec![1, 2, 3, 4])
        .build()
        .expect("valid lump names and sizes");

    println!("built {} bytes", bytes.len());

    // Round-trip: parse it back, add a lump, and rebuild.
    let wad = Wad::from_bytes(bytes).expect("just-built WAD parses");
    let mut builder = wad.to_builder();
    builder.add_lump("EXTRA", b"more data");
    let rebuilt = builder.build().expect("still valid");
    let rebuilt_len = rebuilt.len();
    let rebuilt_lump_count = Wad::from_bytes(rebuilt)
        .expect("rebuilt WAD parses")
        .lump_count();

    println!("rebuilt {rebuilt_len} bytes with {rebuilt_lump_count} lumps");

    // Lenient mode tolerates an over-length name, truncating it and returning a warning.
    let (_, warnings) = WadBuilder::new(WadKind::Pwad)
        .add_lump("VERYLONGNAME", b"data")
        .build_with_options(&WriteOptions::lenient())
        .expect("lenient mode recovers from the over-length name");
    for warning in &warnings {
        println!("warning: {warning}");
    }
}
