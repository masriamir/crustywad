//! Identifying one map's lumps within the flat WAD directory (ADR-0015 §1).

use crate::Wad;
use crate::map::graph::MapFormat;

/// Recognized classic/extended map **data** lump names. A lump is a map marker
/// when the lump directly after it is one of these.
const MAP_DATA_LUMPS: &[&str] = &[
    "THINGS", "LINEDEFS", "SIDEDEFS", "VERTEXES", "SEGS", "SSECTORS", "NODES", "SECTORS", "REJECT",
    "BLOCKMAP", "BEHAVIOR", "TEXTMAP",
];

fn is_map_data_lump(name: &str) -> bool {
    MAP_DATA_LUMPS.contains(&name)
}

/// One map's lumps within a WAD: the marker lump plus its associated data
/// lumps, addressed by directory index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapGroup {
    /// Directory index of the map's marker lump (e.g. `E1M1`, `MAP01`).
    pub marker_index: usize,
    /// The map's name, taken from the marker lump.
    pub name: String,
    /// Directory indices of the data lumps belonging to this map, in order.
    pub data_indices: Vec<usize>,
}

/// If the lump at `i` is a map marker (its successor is a recognized data
/// lump), returns the directory index just past its contiguous data-lump run;
/// otherwise `None`. Allocates nothing.
fn marker_run_end(wad: &Wad, i: usize) -> Option<usize> {
    let lumps = wad.lumps();
    if !lumps.get(i + 1).is_some_and(|l| is_map_data_lump(l.name())) {
        return None;
    }
    // If the map is UDMF (its first data lump is TEXTMAP), the run is bounded by
    // the first subsequent ENDMAP (inclusive) rather than by MAP_DATA_LUMPS
    // membership, so intervening port-specific lumps are captured. If ENDMAP is
    // absent, recover best-effort: stop at the next map marker or end-of-directory.
    if lumps.get(i + 1).is_some_and(|l| l.name() == "TEXTMAP") {
        let mut j = i + 2;
        while j < lumps.len() {
            if lumps[j].name() == "ENDMAP" {
                return Some(j + 1); // inclusive of ENDMAP
            }
            // A new map marker (its successor is a recognized data lump) bounds recovery.
            if lumps.get(j + 1).is_some_and(|l| is_map_data_lump(l.name())) {
                return Some(j);
            }
            j += 1;
        }
        return Some(j); // end-of-directory recovery
    }
    let mut j = i + 1;
    while j < lumps.len() && is_map_data_lump(lumps[j].name()) {
        j += 1;
    }
    Some(j)
}

/// Returns whether any of the group's data lumps is named `name`.
// Consumed by `detect_map_format`'s UDMF branch, added in a follow-up task.
#[allow(dead_code)]
pub(crate) fn group_has_lump(wad: &Wad, group: &MapGroup, name: &str) -> bool {
    group
        .data_indices
        .iter()
        .any(|&i| wad.lumps().get(i).is_some_and(|l| l.name() == name))
}

/// Builds the group for the marker at `i` whose data lumps span `i + 1..end`.
fn build_group(wad: &Wad, i: usize, end: usize) -> MapGroup {
    MapGroup {
        marker_index: i,
        name: wad.lumps()[i].name().to_string(),
        data_indices: (i + 1..end).collect(),
    }
}

pub(crate) fn map_groups(wad: &Wad) -> Vec<MapGroup> {
    let len = wad.lumps().len();
    let mut groups = Vec::new();
    let mut i = 0;
    while i < len {
        if let Some(end) = marker_run_end(wad, i) {
            groups.push(build_group(wad, i, end));
            i = end; // skip past the consumed run so data lumps aren't seen as markers
        } else {
            i += 1;
        }
    }
    groups
}

pub(crate) fn map_group(wad: &Wad, name: &str) -> Option<MapGroup> {
    let len = wad.lumps().len();
    let mut i = 0;
    while i < len {
        if let Some(end) = marker_run_end(wad, i) {
            // Build (and allocate) only the matching group, returning early.
            if wad.lumps()[i].name() == name {
                return Some(build_group(wad, i, end));
            }
            i = end;
        } else {
            i += 1;
        }
    }
    None
}

/// Classifies the map format of `group` from its lump names (ADR-0014).
///
/// A `BEHAVIOR` lump marks a Hexen map; otherwise the group is treated as the
/// classic Doom binary layout. UDMF (`TEXTMAP`) classification arrives with the
/// UDMF work — until then a `TEXTMAP` group is reported as [`MapFormat::Doom`]
/// here but refused by [`Map::assemble`][crate::map::Map::assemble].
#[must_use]
pub fn detect_map_format(wad: &Wad, group: &MapGroup) -> MapFormat {
    let has_lump = |name: &str| {
        group
            .data_indices
            .iter()
            .any(|&i| wad.lumps().get(i).is_some_and(|l| l.name() == name))
    };
    if has_lump("BEHAVIOR") {
        MapFormat::Hexen
    } else {
        MapFormat::Doom
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode_i32(value: usize) -> [u8; 4] {
        i32::try_from(value)
            .expect("test fixture values should fit within i32")
            .to_le_bytes()
    }

    /// Builds minimal PWAD bytes from `(name, data)` lump pairs, mirroring the
    /// on-disk layout used by `tests/common/mod.rs::build_wad`: a 12-byte
    /// header (`PWAD`, lump count, directory offset), lump payloads, then
    /// 16-byte directory entries (`filepos`, `size`, 8-byte name).
    fn build_pwad(lumps: &[(&str, &[u8])]) -> Vec<u8> {
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
        wad.extend_from_slice(b"PWAD");
        wad.extend_from_slice(&encode_i32(lumps.len()));
        wad.extend_from_slice(&encode_i32(directory_offset));
        wad.extend_from_slice(&payload);
        wad.extend_from_slice(&directory);
        wad
    }

    #[test]
    fn textmap_run_is_bounded_by_endmap_inclusive() {
        // MAP01 / TEXTMAP / SCRIPTS(port lump) / ENDMAP / (next) MAP02 / TEXTMAP / ENDMAP
        let wad = crate::Wad::from_bytes(build_pwad(&[
            ("MAP01", b"" as &[u8]),
            ("TEXTMAP", b"x"),
            ("SCRIPTS", b"y"),
            ("ENDMAP", b""),
            ("MAP02", b""),
            ("TEXTMAP", b"z"),
            ("ENDMAP", b""),
        ]))
        .unwrap();
        let g = map_group(&wad, "MAP01").unwrap();
        // data_indices covers TEXTMAP, SCRIPTS, and ENDMAP (indices 1,2,3), not MAP02.
        assert!(group_has_lump(&wad, &g, "TEXTMAP"));
        assert!(group_has_lump(&wad, &g, "ENDMAP"));
        assert!(!group_has_lump(&wad, &g, "MAP02"));
    }

    #[test]
    fn textmap_without_endmap_recovers_and_has_no_endmap() {
        let wad = crate::Wad::from_bytes(build_pwad(&[
            ("MAP01", b"" as &[u8]),
            ("TEXTMAP", b"x"),
            ("SCRIPTS", b"y"),
        ]))
        .unwrap();
        let g = map_group(&wad, "MAP01").unwrap();
        assert!(group_has_lump(&wad, &g, "TEXTMAP"));
        assert!(!group_has_lump(&wad, &g, "ENDMAP"));
    }

    #[test]
    fn textmap_without_endmap_recovers_at_next_marker_without_swallowing_it() {
        // No ENDMAP for MAP01, but a following binary map MAP02 must remain its
        // own group — recovery bounds MAP01's run at MAP02, not end-of-directory.
        let wad = crate::Wad::from_bytes(build_pwad(&[
            ("MAP01", b"" as &[u8]),
            ("TEXTMAP", b"x"),
            ("SCRIPTS", b"y"),
            ("MAP02", b""),
            ("THINGS", b""),
            ("LINEDEFS", b""),
        ]))
        .unwrap();
        let groups = map_groups(&wad);
        assert_eq!(
            groups.len(),
            2,
            "MAP02 must be a separate group, not swallowed"
        );
        let g1 = map_group(&wad, "MAP01").unwrap();
        assert!(group_has_lump(&wad, &g1, "TEXTMAP"));
        assert!(!group_has_lump(&wad, &g1, "ENDMAP"));
        assert!(!group_has_lump(&wad, &g1, "MAP02"));
        assert!(!group_has_lump(&wad, &g1, "THINGS"));
        let g2 = map_group(&wad, "MAP02").unwrap();
        assert!(group_has_lump(&wad, &g2, "THINGS"));
        assert!(group_has_lump(&wad, &g2, "LINEDEFS"));
    }
}
