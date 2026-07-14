//! Identifying one map's lumps within the flat WAD directory (ADR-0015 §1).

use crate::Wad;
use crate::map::doom64::{is_doom64_map_lump, is_doom64_map_name};
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
    // A UDMF map (its first data lump is TEXTMAP) is delimited by TEXTMAP ...
    // ENDMAP, so the run is bounded by the first subsequent ENDMAP (inclusive)
    // rather than by MAP_DATA_LUMPS membership — every intervening lump (classic
    // data lumps like REJECT/BLOCKMAP/BEHAVIOR *and* port lumps like ZNODES) is
    // captured up to ENDMAP.
    //
    // A UDMF map has no reliable internal terminator other than ENDMAP, so the
    // "successor is a data lump" next-marker heuristic (used for binary maps) is
    // deliberately NOT applied here: an intervening lump that happens to precede
    // a data lump must not be mistaken for a new map marker (which would truncate
    // the run and spawn phantom groups). For a malformed map with no ENDMAP, the
    // only unambiguous boundary is the next map's TEXTMAP (its marker is the lump
    // just before it) or end-of-directory — so such a map absorbs trailing lumps
    // up to the next UDMF map or the end of the directory.
    if lumps.get(i + 1).is_some_and(|l| l.name() == "TEXTMAP") {
        let mut j = i + 2;
        while j < lumps.len() {
            match lumps[j].name() {
                "ENDMAP" => return Some(j + 1), // inclusive of ENDMAP
                // The next UDMF map's data begins at `j`; its marker is `j - 1`.
                "TEXTMAP" => return Some(j - 1),
                _ => j += 1,
            }
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
        // A Doom 64 map is a nested WAD inside its MAPxx marker lump — no
        // classic data-lump run follows it (ADR-0021 §1).
        let lump = &wad.lumps()[i];
        if is_doom64_map_name(lump.name()) && is_doom64_map_lump(wad.lump_data(lump)) {
            groups.push(MapGroup {
                marker_index: i,
                name: lump.name().to_string(),
                data_indices: Vec::new(),
            });
            i += 1;
            continue;
        }
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
        // A Doom 64 map is a nested WAD inside its MAPxx marker lump — no
        // classic data-lump run follows it (ADR-0021 §1).
        let lump = &wad.lumps()[i];
        if is_doom64_map_name(lump.name()) && is_doom64_map_lump(wad.lump_data(lump)) {
            if lump.name() == name {
                return Some(MapGroup {
                    marker_index: i,
                    name: lump.name().to_string(),
                    data_indices: Vec::new(),
                });
            }
            i += 1;
            continue;
        }
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
/// The marker lump is checked first, under the same dual condition grouping
/// uses (ADR-0021 §1): a `MAPxx` name **and** nested `IWAD`/`PWAD` magic mark
/// a Doom 64 map — so a classically named marker whose bytes happen to start
/// with WAD magic stays a classic map, symmetric with `map_groups`. Otherwise,
/// a `TEXTMAP` lump marks a UDMF map; otherwise a `BEHAVIOR` lump marks a
/// Hexen map; otherwise the group is treated as the classic Doom binary
/// layout.
#[must_use]
pub fn detect_map_format(wad: &Wad, group: &MapGroup) -> MapFormat {
    let marker = &wad.lumps()[group.marker_index];
    if is_doom64_map_name(marker.name()) && is_doom64_map_lump(wad.lump_data(marker)) {
        return MapFormat::Doom64;
    }
    if group_has_lump(wad, group, "TEXTMAP") {
        return MapFormat::Udmf;
    }
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
    fn textmap_without_endmap_recovers_at_next_udmf_map() {
        // No ENDMAP for MAP01, but a following UDMF map (MAP02/TEXTMAP/…) is an
        // unambiguous boundary: MAP01's run ends just before MAP02, and MAP02 is
        // its own group. (Its own TEXTMAP is what makes MAP02 recognizable — a
        // following *binary* map without a TEXTMAP would instead be absorbed,
        // since a no-ENDMAP UDMF run has no reliable binary-marker boundary.)
        let wad = crate::Wad::from_bytes(build_pwad(&[
            ("MAP01", b"" as &[u8]),
            ("TEXTMAP", b"x"),
            ("SCRIPTS", b"y"),
            ("MAP02", b""),
            ("TEXTMAP", b"z"),
            ("ENDMAP", b""),
        ]))
        .unwrap();
        let groups = map_groups(&wad);
        assert_eq!(
            groups.len(),
            2,
            "MAP01 and the next UDMF map MAP02 are separate"
        );
        let g1 = map_group(&wad, "MAP01").unwrap();
        assert!(group_has_lump(&wad, &g1, "TEXTMAP"));
        assert!(group_has_lump(&wad, &g1, "SCRIPTS"));
        assert!(!group_has_lump(&wad, &g1, "ENDMAP"));
        assert!(!group_has_lump(&wad, &g1, "MAP02"));
        let g2 = map_group(&wad, "MAP02").unwrap();
        assert!(group_has_lump(&wad, &g2, "TEXTMAP"));
        assert!(group_has_lump(&wad, &g2, "ENDMAP"));
    }

    #[test]
    fn textmap_run_captures_intervening_binary_lumps_up_to_endmap() {
        // A ZDoom-style UDMF map carries auxiliary lumps between TEXTMAP and
        // ENDMAP — some classic data lumps (REJECT/BLOCKMAP/BEHAVIOR) and some
        // port lumps that are NOT in MAP_DATA_LUMPS (ZNODES). The run must extend
        // to ENDMAP, NOT truncate at the first intervening lump whose successor
        // happens to be a data lump.
        let wad = crate::Wad::from_bytes(build_pwad(&[
            ("MAP01", b"" as &[u8]),
            ("TEXTMAP", b"x"),
            ("ZNODES", b"z"),
            ("REJECT", b"r"),
            ("BLOCKMAP", b"b"),
            ("BEHAVIOR", b""),
            ("ENDMAP", b""),
            ("MAP02", b""),
            ("THINGS", b""),
        ]))
        .unwrap();
        let groups = map_groups(&wad);
        assert_eq!(
            groups.len(),
            2,
            "MAP01 (through ENDMAP) and MAP02 are separate"
        );
        let g = map_group(&wad, "MAP01").unwrap();
        for lump in [
            "TEXTMAP", "ZNODES", "REJECT", "BLOCKMAP", "BEHAVIOR", "ENDMAP",
        ] {
            assert!(group_has_lump(&wad, &g, lump), "MAP01 must contain {lump}");
        }
        assert!(!group_has_lump(&wad, &g, "MAP02"));
    }

    #[test]
    fn textmap_without_endmap_followed_by_data_lumps_makes_no_phantom_markers() {
        // A malformed UDMF map (no ENDMAP) followed only by data lumps: recovery
        // must NOT pick a data lump (REJECT/BLOCKMAP) as the bound, which would
        // both drop it and later be mis-read as a standalone "REJECT" marker.
        // With no genuine next marker, the whole tail is one recovered group.
        let wad = crate::Wad::from_bytes(build_pwad(&[
            ("MAP01", b"" as &[u8]),
            ("TEXTMAP", b"x"),
            ("REJECT", b"r"),
            ("BLOCKMAP", b"b"),
        ]))
        .unwrap();
        let groups = map_groups(&wad);
        assert_eq!(groups.len(), 1, "no phantom REJECT/BLOCKMAP marker groups");
        let g = map_group(&wad, "MAP01").unwrap();
        assert!(group_has_lump(&wad, &g, "REJECT"));
        assert!(group_has_lump(&wad, &g, "BLOCKMAP"));
    }
}
