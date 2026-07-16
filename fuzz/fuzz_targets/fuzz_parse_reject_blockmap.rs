#![no_main]
use libfuzzer_sys::fuzz_target;

use crustywad::Strictness;
use crustywad::map::{MapBlockmap, MapReject};

// First four input bytes pick the sector/linedef counts; the rest is the
// lump payload, fed to both parsers in both strictness modes. Oracles per
// ADR-0016: no panic, and output bounded by the input — the REJECT table
// stores at most the payload's bytes (virtual padding) and warns at most
// once; the BLOCKMAP block table is bounded by the offset table that
// physically fit in the payload, with warnings bounded by two per block on
// the Ok(Some) path and by one on the whole-lump discard path (the two
// paths are mutually exclusive — see the per-assert comments below).
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
            // At most 2 per block: an unterminated list is truncated
            // (warning 1) and the truncated span may still carry a
            // dangling linedef that empties the block (warning 2); every
            // other lenient recovery `continue`s after its single warning.
            assert!(warnings.len() <= 2 * blocks);
        } else {
            // Not `Ok(Some(_))` is reachable only via a pre-block-loop
            // `malformed` check (empty/too-short/non-positive
            // dimensions/offset-table-overflow) or a strict-mode `Err` from
            // inside the loop. The block loop itself never returns
            // `Ok(None)`, so per-block warnings can never combine with this
            // branch: lenient `malformed` pushes exactly 1 warning before
            // returning `Ok(None)`, and every strict `Err` (pre-loop or
            // in-loop) returns before pushing any warning. So 1 is the true
            // bound, not just a loose cap.
            assert!(warnings.len() <= 1);
        }
    }
});
