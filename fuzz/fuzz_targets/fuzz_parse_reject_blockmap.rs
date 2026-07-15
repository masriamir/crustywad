#![no_main]
use libfuzzer_sys::fuzz_target;

use crustywad::Strictness;
use crustywad::map::{MapBlockmap, MapReject};

// First four input bytes pick the sector/linedef counts; the rest is the
// lump payload, fed to both parsers in both strictness modes. Oracles per
// ADR-0016: no panic, and output bounded by the input — the REJECT table
// stores at most the payload's bytes (virtual padding), the BLOCKMAP block
// table is bounded by the offset table that physically fit in the payload,
// and warnings are bounded by one per block plus one whole-lump warning
// per parser.
fuzz_target!(|data: &[u8]| {
    if data.len() < 4 {
        return;
    }
    let sector_count = usize::from(u16::from_le_bytes([data[0], data[1]]));
    let linedef_count = usize::from(u16::from_le_bytes([data[2], data[3]]));
    let payload = &data[4..];
    let words = payload.len() / 2;

    for strictness in [Strictness::Strict, Strictness::Lenient] {
        let mut warnings = Vec::new();
        if let Ok(Some(reject)) = MapReject::parse(payload, sector_count, strictness, &mut warnings)
        {
            assert_eq!(reject.sector_count(), sector_count);
        }
        assert!(warnings.len() <= 1);

        let mut warnings = Vec::new();
        if let Ok(Some(blockmap)) =
            MapBlockmap::parse(payload, linedef_count, strictness, &mut warnings)
        {
            let blocks = blockmap.columns() * blockmap.rows();
            assert!(4 + blocks <= words);
            assert!(warnings.len() <= blocks);
        } else {
            assert!(warnings.len() <= 1);
        }
    }
});
