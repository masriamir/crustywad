use std::path::PathBuf;

/// Encodes a `usize` as a little-endian `i32`, panicking if the value overflows.
fn encode_i32(value: usize) -> [u8; 4] {
    i32::try_from(value)
        .expect("bench fixture value must fit in i32")
        .to_le_bytes()
}

/// Builds a minimal but structurally valid WAD byte buffer.
///
/// `kind` is the 4-byte magic (e.g. `*b"IWAD"` or `*b"PWAD"`).
/// `lumps` is a slice of `(name, data)` pairs; names are truncated to 8 bytes.
pub fn build_wad(kind: [u8; 4], lumps: &[(&str, &[u8])]) -> Vec<u8> {
    let mut payload = Vec::new();
    let mut directory = Vec::new();
    let directory_offset = 12 + lumps.iter().map(|(_, b)| b.len()).sum::<usize>();

    for (name, bytes) in lumps {
        let filepos = 12 + payload.len();
        payload.extend_from_slice(bytes);
        directory.extend_from_slice(&encode_i32(filepos));
        directory.extend_from_slice(&encode_i32(bytes.len()));
        let mut encoded = [0u8; 8];
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

/// Returns the path to a Freedoom WAD directory, or `None` when the env var is unset.
///
/// Benchmarks skip gracefully when this returns `None`.
pub fn freedoom_wad_path() -> Option<PathBuf> {
    std::env::var_os("CRUSTYWAD_FREEDOOM_DIR").map(PathBuf::from)
}

/// 10 PWAD lumps × 256 bytes each (~2.6 KB total).
pub fn small_wad() -> Vec<u8> {
    let payload = vec![0u8; 256];
    let lumps: Vec<(&str, &[u8])> = (0..10).map(|_| ("BENCH", payload.as_slice())).collect();
    build_wad(*b"PWAD", &lumps)
}

/// 100 PWAD lumps × 4 KiB each (~401 KB total).
pub fn medium_wad() -> Vec<u8> {
    let payload = vec![0u8; 4096];
    let lumps: Vec<(&str, &[u8])> = (0..100).map(|_| ("BENCH", payload.as_slice())).collect();
    build_wad(*b"PWAD", &lumps)
}

/// 1 000 PWAD lumps × 16 KiB each (~16 MB total).
pub fn large_wad() -> Vec<u8> {
    let payload = vec![0u8; 16384];
    let lumps: Vec<(&str, &[u8])> = (0..1000).map(|_| ("BENCH", payload.as_slice())).collect();
    build_wad(*b"PWAD", &lumps)
}
