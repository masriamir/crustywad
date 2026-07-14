#![no_main]
use libfuzzer_sys::fuzz_target;

use crustywad::map::Map;
use crustywad::{ParseOptions, Wad};

fuzz_target!(|data: &[u8]| {
    if let Ok(wad) = Wad::from_bytes_with_options(data.to_vec(), ParseOptions::lenient()) {
        for group in wad.map_groups() {
            if let Ok(map) = Map::assemble_with_options(&wad, &group, ParseOptions::lenient()) {
                // Guard against unbounded warning growth: normalize_linedefs
                // (all formats) pushes at most 4 warnings per linedef
                // (start/end vertex, right/left sidedef); normalize_sidedefs
                // at most 1 per sidedef (sector reference); the Doom 64 arm
                // additionally pushes at most 5 per sector (one per colored-
                // lighting reference resolved into `MapSector::colors` from
                // the raw `doom64::Sector.colors`), at most 1 per thing
                // (`type_id` range coercion, shared with the UDMF arm's
                // `coerce_u16` call), and at most 9 container-level
                // `Doom64Warning`s (one per expected nested-WAD sub-lump:
                // THINGS/LINEDEFS/SIDEDEFS/VERTEXES/SECTORS/LIGHTS/SEGS/
                // SSECTORS/NODES — `MissingLump` xor `TrailingBytes`, never
                // both, per lump). The shared `normalize_bsp` (classic and
                // Doom 64 both call it, ADR-0015 §1/ADR-0018) pushes at most
                // 3 per seg (`resolve_required` on start vertex, end vertex,
                // linedef — one `MapWarning` each), at most 1 per subsector
                // (the single seg-range `DanglingReference` check), and at
                // most 2 per node (`resolve_node_child` -> `resolve_required`
                // once per child, right then left) — verified directly
                // against `normalize_bsp` in assemble.rs, not derived from
                // this comment alone. Plus at most 2 more from the classic-
                // only extended-node-encoding gate, which can warn once each
                // for NODES and SSECTORS in lenient mode (Doom 64 has no such
                // gate, so this constant is a safe over-count there).
                let warning_count = map.warnings().len();
                // Saturating arithmetic keeps the bound meaningful even for a
                // pathologically large synthesized map (no overflow).
                let bound = map
                    .linedefs()
                    .len()
                    .saturating_mul(4)
                    .saturating_add(map.sidedefs().len())
                    .saturating_add(map.sectors().len().saturating_mul(5))
                    .saturating_add(map.things().len())
                    .saturating_add(9)
                    .saturating_add(map.segs().len().saturating_mul(3))
                    .saturating_add(map.subsectors().len())
                    .saturating_add(map.nodes().len().saturating_mul(2))
                    .saturating_add(2);
                assert!(
                    warning_count <= bound,
                    "warning count {warning_count} exceeded upper bound {bound}"
                );
                // O(input) allocation invariant (ADR-0016 §1): each arena is
                // decoded from one fixed-size-record lump whose bytes are a
                // subset of the WAD input, so element count <= input_len /
                // min_record_size. The divisors below are the SMALLEST record
                // size per arena across supported formats (Doom linedef 14 <
                // Hexen/Doom64 linedef 16; Doom thing 10 < Doom64 thing 14 <
                // Hexen thing 20; Doom64 sidedef 12 < Doom/Hexen sidedef 30;
                // Doom64 sector 24 < Doom/Hexen sector 26), so the bound holds
                // across all formats reachable via `map_groups`. `lights` is
                // Doom64-only and checked separately below; other formats
                // always report an empty lights table. `segs`/`subsectors`/
                // `nodes` are decoded by the single shared `normalize_bsp`
                // (classic `common::Seg`/`Subsector`/`Node` records, ADR-0015
                // §1) that both the classic binary path and the Doom 64
                // nested-WAD path (ADR-0018) call, so classic and Doom 64
                // share one on-disk layout per arena and each divisor is its
                // own cross-format minimum: `common::Seg` is 12 bytes
                // (2 vertex indices + angle + linedef + direction, all u16,
                // plus an i16 offset); `common::Subsector` is 4 bytes (two
                // u16 fields); `common::Node` is 28 bytes (4 i16 partition
                // fields + two 4-entry i16 bounding boxes + 2 u16 child
                // indices).
                // Bounded per-arena (not summed) so it holds even if a
                // malicious WAD overlaps lumps.
                for (count, record_size, arena) in [
                    (map.vertices().len(), 4, "vertices"),
                    (map.linedefs().len(), 14, "linedefs"),
                    (map.sidedefs().len(), 12, "sidedefs"),
                    (map.sectors().len(), 24, "sectors"),
                    (map.things().len(), 10, "things"),
                    (map.segs().len(), 12, "segs"),
                    (map.subsectors().len(), 4, "subsectors"),
                    (map.nodes().len(), 28, "nodes"),
                ] {
                    assert!(
                        count <= data.len() / record_size,
                        "{arena} count {count} exceeds O(input) bound {}",
                        data.len() / record_size
                    );
                }
                // `Map::lights()` mirrors the engine's table (Doom64 EX
                // P_LoadLights): 256 implicit grayscale entries always precede
                // the LIGHTS lump records (record size 6), so the O(input)
                // bound carries a constant 256-entry offset.
                let lights_len = map.lights().len();
                assert!(
                    lights_len <= 256 + data.len() / 6,
                    "lights count {lights_len} exceeds O(input) bound {}",
                    256 + data.len() / 6
                );
                std::hint::black_box(&map);
            }
        }
    }
});
