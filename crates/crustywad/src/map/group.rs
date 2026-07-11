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
    let mut j = i + 1;
    while j < lumps.len() && is_map_data_lump(lumps[j].name()) {
        j += 1;
    }
    Some(j)
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
