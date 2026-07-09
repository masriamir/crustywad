//! Identifying one map's lumps within the flat WAD directory (ADR-0015 §1).

use crate::Wad;

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

pub(crate) fn map_groups(wad: &Wad) -> Vec<MapGroup> {
    let lumps = wad.lumps();
    let mut groups = Vec::new();
    let mut i = 0;
    while i < lumps.len() {
        let next_is_data = lumps.get(i + 1).is_some_and(|l| is_map_data_lump(l.name()));
        if next_is_data {
            let mut j = i + 1;
            let mut data_indices = Vec::new();
            while j < lumps.len() && is_map_data_lump(lumps[j].name()) {
                data_indices.push(j);
                j += 1;
            }
            groups.push(MapGroup {
                marker_index: i,
                name: lumps[i].name().to_string(),
                data_indices,
            });
            i = j; // skip past the consumed run so data lumps aren't seen as markers
        } else {
            i += 1;
        }
    }
    groups
}

pub(crate) fn map_group(wad: &Wad, name: &str) -> Option<MapGroup> {
    map_groups(wad).into_iter().find(|g| g.name == name)
}
