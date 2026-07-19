//! The REJECT builder (ADR-0024 §6, staging §9.1).
//!
//! A `REJECT` lump is an `n × n` sector-visibility bit matrix. An all-zeros
//! table means "never pre-reject any sight line": the engine's reject early-out
//! simply never fires and every sight check falls through to the full BSP trace
//! (verified against Chocolate Doom `p_sight.c` in the ADR-0024 research
//! record). That is functionally correct for every map — it is exactly what
//! `zdbsp` ships (its `--full-reject` is unsupported) — so building REJECT is
//! just emitting the correctly-sized zero table. There is nothing to fail on,
//! hence [`build_reject`] is infallible.

use crate::map::Map;
use crate::map::graph::MapReject;

/// Builds the correctly-sized all-zeros `REJECT` for `map`.
///
/// The table is `ceil(sector_count² / 8)` bytes (matching
/// [`MapReject::parse`]'s own size arithmetic), every bit clear. An all-clear
/// table pre-rejects no sight line, which is always engine-correct (ADR-0024
/// §6): the reject early-out never fires and sight checks fall through to the
/// full trace.
///
/// Infallible. An assembled [`Map`] always has at least one sector (an empty
/// required arena is a fatal assembly error, ADR-0015); a hypothetical
/// zero-sector input simply yields an empty table (`ceil(0 / 8) == 0` bytes),
/// which round-trips as an absent REJECT.
#[must_use]
pub fn build_reject(map: &Map) -> MapReject {
    let sector_count = map.sectors().len();
    // Saturating, mirroring `MapReject::parse`: a pathological standalone count
    // yields a deterministic size rather than overflowing.
    let expected = sector_count.saturating_mul(sector_count).div_ceil(8);
    MapReject {
        sector_count,
        bits: vec![0u8; expected].into(),
    }
}

impl MapReject {
    /// Serializes this table to `REJECT` lump bytes: the stored table verbatim.
    ///
    /// Infallible — a REJECT lump is a flat bit matrix with no offsets or
    /// counts that could overflow. Round-trips exactly through
    /// [`MapReject::parse`] against the same `sector_count` (ADR-0024 §7).
    #[must_use]
    pub fn to_lump_bytes(&self) -> Vec<u8> {
        self.bits.to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Strictness;
    use crate::map::MapWarning;
    use crate::map::graph::{Map, MapFormat, MapSector, TextureRef};

    /// A `Map` with `n` sectors and no other geometry — enough to exercise the
    /// REJECT builder, which reads only the sector count.
    fn map_with_sectors(n: usize) -> Map {
        let sector = MapSector {
            floor_height: 0,
            ceiling_height: 128,
            floor_flat: TextureRef::Name("FLOOR4_8".into()),
            ceiling_flat: TextureRef::Name("CEIL3_5".into()),
            light: 160,
            special: 0,
            tag: 0,
            colors: None,
            flags: 0,
        };
        Map {
            name: "MAP01".into(),
            format: MapFormat::Doom,
            namespace: None,
            vertices: Vec::new(),
            linedefs: Vec::new(),
            sidedefs: Vec::new(),
            sectors: vec![sector; n],
            things: Vec::new(),
            lights: Vec::new(),
            segs: Vec::new(),
            subsectors: Vec::new(),
            nodes: Vec::new(),
            leafs: Vec::new(),
            macros: Vec::new(),
            reject: None,
            blockmap: None,
            warnings: Vec::new(),
        }
    }

    #[test]
    fn three_sectors_gives_two_zero_bytes() {
        // ceil(3² / 8) = ceil(9 / 8) = 2 bytes.
        let reject = build_reject(&map_with_sectors(3));
        assert_eq!(reject.sector_count(), 3);
        assert_eq!(reject.to_lump_bytes(), vec![0u8, 0u8]);
    }

    #[test]
    fn size_matches_parse_arithmetic() {
        // ceil(n² / 8): 1 -> 1, 8 -> 8 (64 / 8), 0 -> 0.
        assert_eq!(build_reject(&map_with_sectors(1)).to_lump_bytes().len(), 1);
        assert_eq!(build_reject(&map_with_sectors(8)).to_lump_bytes().len(), 8);
        assert!(
            build_reject(&map_with_sectors(0))
                .to_lump_bytes()
                .is_empty()
        );
    }

    #[test]
    fn all_bytes_are_zero() {
        let bytes = build_reject(&map_with_sectors(5)).to_lump_bytes();
        // ceil(25 / 8) = 4 bytes, all clear.
        assert_eq!(bytes, vec![0u8; 4]);
    }

    #[test]
    fn round_trips_through_parse_in_strict_mode() {
        // ADR-0024 §7 / Global Constraint 4: re-parsing the built bytes against
        // the owning sector count reconstructs the table exactly, warning-free.
        for n in [1usize, 3, 8] {
            let built = build_reject(&map_with_sectors(n));
            let mut warnings: Vec<MapWarning> = Vec::new();
            let parsed =
                MapReject::parse(&built.to_lump_bytes(), n, Strictness::Strict, &mut warnings)
                    .expect("built REJECT parses")
                    .expect("built REJECT is present");
            assert_eq!(parsed, built);
            assert!(warnings.is_empty());
        }
    }
}
