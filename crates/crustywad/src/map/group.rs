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

/// Directory indices of one classic GL-node group's four data lumps, located
/// via [`gl_group_for`].
///
/// Consumed by
/// [`assemble_with_options`](crate::map::graph::Map::assemble_with_options),
/// which reads the four lumps' bytes by these indices and decodes them (#324).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GlGroup {
    /// Directory index of the `GL_VERT` lump.
    pub(crate) vert: usize,
    /// Directory index of the `GL_SEGS` lump.
    pub(crate) segs: usize,
    /// Directory index of the `GL_SSECT` lump.
    pub(crate) ssect: usize,
    /// Directory index of the `GL_NODES` lump.
    pub(crate) nodes: usize,
}

/// Recognized GL data-lump names within a group's run. Any other
/// `GL_`-prefixed lump is a group marker (`GL_<name>` / `GL_LEVEL`) and ends
/// the run — this is what keeps adjacent groups (an in-WAD map's own
/// `GL_<name2>` marker, or back-to-back `.gwa` groups) from bleeding into
/// each other.
const GL_DATA_LUMPS: &[&str] = &["GL_VERT", "GL_SEGS", "GL_SSECT", "GL_NODES", "GL_PVS"];

/// Collects a classic GL-node group's four required data lumps from the
/// contiguous run starting at `marker_index + 1`. Recognizes `GL_VERT`,
/// `GL_SEGS`, `GL_SSECT`, and `GL_NODES` in any order (first occurrence
/// wins), and tolerates an interleaved `GL_PVS` lump without using it. The
/// run stops at the first lump whose name is not in [`GL_DATA_LUMPS`] —
/// whether because it isn't `GL_`-prefixed at all, or because it's a
/// different group's marker.
///
/// Shared by [`gl_group_for`] (anchored, in-WAD lookup) and
/// [`gl_group_in_gl_wad`] (by-name, `.gwa` lookup, #342) so both honor the
/// same run-termination rule.
fn collect_gl_run(wad: &Wad, marker_index: usize) -> Option<GlGroup> {
    let lumps = wad.lumps();
    let (mut vert, mut segs, mut ssect, mut nodes) = (None, None, None, None);
    let mut j = marker_index + 1;
    while let Some(lump) = lumps.get(j) {
        let name = lump.name();
        if !GL_DATA_LUMPS.contains(&name) {
            break;
        }
        match name {
            "GL_VERT" => {
                vert.get_or_insert(j);
            }
            "GL_SEGS" => {
                segs.get_or_insert(j);
            }
            "GL_SSECT" => {
                ssect.get_or_insert(j);
            }
            "GL_NODES" => {
                nodes.get_or_insert(j);
            }
            _ => {}
        }
        j += 1;
    }

    Some(GlGroup {
        vert: vert?,
        segs: segs?,
        ssect: ssect?,
        nodes: nodes?,
    })
}

/// Locates the classic `GL_<map_name>` node group belonging to the specific
/// `map` instance in `wad`.
///
/// Classic GL nodes live under a `GL_<mapname>` marker lump (e.g. `GL_MAP01`),
/// followed by a contiguous run of `GL_*` lumps that includes `GL_VERT`,
/// `GL_SEGS`, `GL_SSECT`, and `GL_NODES` in any order.
///
/// The lookup is anchored to `map`: the scan starts immediately after `map`'s
/// own marker/data-lump run (not from the top of the directory) and stops as
/// soon as another map's marker is encountered. This ensures a WAD with
/// duplicate map names (two `MAP01` groups) or a stray earlier `GL_<name>`
/// lump cannot cross-associate GL data between map instances — the GL group
/// returned always belongs to *this* map's own run, never an earlier or later
/// same-named map's.
///
/// Returns `None` if the marker name would exceed the 8-byte WAD lump-name
/// limit (no such lump could exist), if no `GL_<map_name>` marker is found
/// before the next map marker (or end of directory), or if any of the four
/// required lumps is missing from its run.
///
/// Called by
/// [`assemble_with_options`](crate::map::graph::Map::assemble_with_options) on
/// the binary-map path to locate the group before decoding it (#324).
pub(crate) fn gl_group_for(wad: &Wad, map: &MapGroup) -> Option<GlGroup> {
    let marker_name = format!("GL_{}", map.name);
    if marker_name.len() > 8 {
        return None;
    }

    let lumps = wad.lumps();
    // Anchor the lookup to THIS map instance: scan from immediately after the
    // map's own lump run and stop at the next map marker, so a duplicate map
    // name or a stray earlier `GL_<name>` cannot cross-associate GL data
    // between map instances.
    let start = map
        .data_indices
        .iter()
        .copied()
        .max()
        .unwrap_or(map.marker_index)
        + 1;
    let mut i = start;
    let marker_index = loop {
        match lumps.get(i) {
            None => return None,
            Some(lump) if lump.name() == marker_name => break i,
            // Another map's marker before the GL marker => this map has no GL group.
            Some(_) if marker_run_end(wad, i).is_some() => return None,
            Some(_) => i += 1,
        }
    };

    collect_gl_run(wad, marker_index)
}

/// Locates the classic GL-node group for `map_name` inside a sibling `.gwa`
/// `Wad`.
///
/// A `.gwa` has no map groups of its own — it is a flat sequence of GL
/// groups, each introduced by one of two marker forms and immediately
/// followed by its data-lump run:
///
/// 1. `GL_<map_name>` — a lump named e.g. `GL_MAP01`, matched by name. Only
///    possible when `GL_` + the map name is at most 8 bytes.
/// 2. `GL_LEVEL` — a lump literally named `GL_LEVEL` whose text contents
///    carry a `LEVEL=<map_name>` line (glBSP's `KEYWORD=VALUE` form, used
///    when the map name doesn't fit form 1).
///
/// Unlike [`gl_group_for`], this is a plain by-name scan of the whole
/// directory rather than one anchored to a specific map instance — a `.gwa`
/// has no map markers to anchor to in the first place.
///
/// Returns `None` if no marker matching `map_name` is found, or if any of
/// the four required data lumps (`GL_VERT`/`GL_SEGS`/`GL_SSECT`/`GL_NODES`)
/// is missing from the matched marker's run before the next group marker
/// (or end of directory).
///
/// This is the locator half of the `.gwa` read path (#342); it has no
/// non-test caller yet — the public API that opens a sibling `.gwa` `Wad`
/// and wires this lookup into map assembly lands in a follow-up task on the
/// same issue — hence the explicit `allow` below.
#[allow(dead_code)]
pub(crate) fn gl_group_in_gl_wad(gl_wad: &Wad, map_name: &str) -> Option<GlGroup> {
    let lumps = gl_wad.lumps();
    let gl_name = format!("GL_{map_name}");
    let mut marker_index = None;
    for (i, lump) in lumps.iter().enumerate() {
        let name = lump.name();
        let is_match = (gl_name.len() <= 8 && name == gl_name)
            || (name == "GL_LEVEL"
                && gl_wad
                    .lump_bytes(i)
                    .is_some_and(|bytes| gl_level_matches(bytes, map_name)));
        if is_match {
            marker_index = Some(i);
            break;
        }
    }
    collect_gl_run(gl_wad, marker_index?)
}

/// True if a `GL_LEVEL` marker's text contents contain a `LEVEL=<map_name>`
/// line (glBSP's `KEYWORD=VALUE` form). The comparison is case-sensitive,
/// matching glBSP's uppercase output. Never panics: non-UTF-8 contents parse
/// to an empty string rather than erroring.
fn gl_level_matches(marker_bytes: &[u8], map_name: &str) -> bool {
    let text = core::str::from_utf8(marker_bytes).unwrap_or("");
    text.lines().any(|line| {
        line.strip_prefix("LEVEL=")
            .is_some_and(|value| value.trim_end_matches(['\r', '\0']).trim() == map_name)
    })
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

    /// Minimal real binary-map data-lump run following a `MAPxx` marker, used
    /// so `map_group`/`map_groups` recognize the marker as a genuine map
    /// group (required now that `gl_group_for` anchors to a `MapGroup`
    /// rather than scanning the whole WAD by name).
    const MIN_MAP_DATA_LUMPS: &[(&str, &[u8])] = &[
        ("THINGS", b""),
        ("LINEDEFS", b""),
        ("SIDEDEFS", b""),
        ("VERTEXES", b""),
        ("SEGS", b""),
        ("SSECTORS", b""),
        ("NODES", b""),
        ("SECTORS", b""),
    ];

    #[test]
    fn locates_in_wad_gl_group() {
        let mut lumps: Vec<(&str, &[u8])> = vec![("MAP01", b"" as &[u8])];
        lumps.extend_from_slice(MIN_MAP_DATA_LUMPS);
        lumps.extend_from_slice(&[
            ("GL_MAP01", b""),
            ("GL_VERT", b"gNd2"),
            ("GL_SEGS", b""),
            ("GL_SSECT", b""),
            ("GL_NODES", b""),
        ]);
        let wad = crate::Wad::from_bytes(build_pwad(&lumps)).unwrap();
        let group = map_group(&wad, "MAP01").expect("MAP01 group");
        let g = gl_group_for(&wad, &group).expect("gl group");
        assert_eq!(wad.lumps()[g.vert].name(), "GL_VERT");
        assert_eq!(wad.lumps()[g.segs].name(), "GL_SEGS");
        assert_eq!(wad.lumps()[g.ssect].name(), "GL_SSECT");
        assert_eq!(wad.lumps()[g.nodes].name(), "GL_NODES");
    }

    #[test]
    fn gl_group_run_terminates_at_non_gl_lump_mid_directory() {
        // The GL_ run is followed by a non-GL_ lump (THINGS) mid-directory,
        // with more lumps after it — the contiguous-run loop in `gl_group_for`
        // must stop at the `!starts_with("GL_")` break, not run to end of
        // directory. An unrecognized `GL_`-prefixed lump (`GL_PVS`) is also
        // included in the run, exercising the wildcard match arm.
        //
        // Note: this fixture places all four required lumps *before* THINGS,
        // so it does not by itself distinguish the `break` from a mutant that
        // removes it (both reach the same `Some(GlGroup)`); see
        // `gl_group_run_break_excludes_lump_past_non_gl_boundary` below for a
        // fixture where the two diverge.
        let mut lumps: Vec<(&str, &[u8])> = vec![("MAP01", b"" as &[u8])];
        lumps.extend_from_slice(MIN_MAP_DATA_LUMPS);
        lumps.extend_from_slice(&[
            ("GL_MAP01", b""),
            ("GL_VERT", b"gNd2"),
            ("GL_PVS", b""),
            ("GL_SEGS", b""),
            ("GL_SSECT", b""),
            ("GL_NODES", b""),
            ("THINGS", b""),
            ("LINEDEFS", b""),
        ]);
        let wad = crate::Wad::from_bytes(build_pwad(&lumps)).unwrap();
        let group = map_group(&wad, "MAP01").expect("MAP01 group");
        let g = gl_group_for(&wad, &group).expect("gl group");
        assert_eq!(wad.lumps()[g.vert].name(), "GL_VERT");
        assert_eq!(wad.lumps()[g.segs].name(), "GL_SEGS");
        assert_eq!(wad.lumps()[g.ssect].name(), "GL_SSECT");
        assert_eq!(wad.lumps()[g.nodes].name(), "GL_NODES");
    }

    /// Real regression guard for the `!starts_with("GL_")` `break` in
    /// `gl_group_for`'s contiguous-run loop.
    ///
    /// Places only 3 of the 4 required lumps (`GL_VERT`/`GL_SEGS`/`GL_SSECT`)
    /// before a non-`GL_` boundary lump (`THINGS`), with the 4th
    /// (`GL_NODES`) *after* it. With the `break` intact, the loop stops at
    /// `THINGS` and never observes `GL_NODES`, so `nodes` stays `None` and
    /// `gl_group_for` correctly returns `None` (the run is incomplete).
    ///
    /// If the `break` is deleted, the loop instead keeps scanning past
    /// `THINGS`, reaches `GL_NODES`, and `gl_group_for` wrongly returns
    /// `Some(GlGroup { .. })` — so this test fails without the `break`,
    /// unlike `gl_group_run_terminates_at_non_gl_lump_mid_directory` above
    /// (where all four required lumps already precede the boundary, so
    /// `get_or_insert` fills them the same way whether or not the loop
    /// stops there). Verified by hand: deleting the `break` makes this test
    /// fail (see the fix report for the full trace).
    #[test]
    fn gl_group_run_break_excludes_lump_past_non_gl_boundary() {
        let mut lumps: Vec<(&str, &[u8])> = vec![("MAP01", b"" as &[u8])];
        lumps.extend_from_slice(MIN_MAP_DATA_LUMPS);
        lumps.extend_from_slice(&[
            ("GL_MAP01", b""),
            ("GL_VERT", b"gNd2"),
            ("GL_SEGS", b""),
            ("GL_SSECT", b""),
            ("THINGS", b""),
            ("GL_NODES", b""),
        ]);
        let wad = crate::Wad::from_bytes(build_pwad(&lumps)).unwrap();
        let group = map_group(&wad, "MAP01").expect("MAP01 group");
        assert!(
            gl_group_for(&wad, &group).is_none(),
            "GL_NODES lies past the THINGS boundary, so the run must be \
             treated as missing it"
        );
    }

    #[test]
    fn gl_group_absent_returns_none() {
        let mut lumps: Vec<(&str, &[u8])> = vec![("MAP01", b"" as &[u8])];
        lumps.extend_from_slice(MIN_MAP_DATA_LUMPS);
        let wad = crate::Wad::from_bytes(build_pwad(&lumps)).unwrap();
        let group = map_group(&wad, "MAP01").expect("MAP01 group");
        assert!(gl_group_for(&wad, &group).is_none());
    }

    #[test]
    fn gl_group_for_overlong_name_returns_none() {
        // "GL_" + "ABCDEF" (9 bytes) exceeds the 8-byte lump name limit, so
        // no such lump could ever exist — must short-circuit to `None`
        // without scanning. The marker name itself is a valid 6-byte name,
        // and needs a data lump after it so `map_group` recognizes it.
        let wad = crate::Wad::from_bytes(build_pwad(&[("ABCDEF", b"" as &[u8]), ("THINGS", b"")]))
            .unwrap();
        let group = map_group(&wad, "ABCDEF").expect("ABCDEF group");
        assert!(gl_group_for(&wad, &group).is_none());
    }

    #[test]
    fn gwa_locator_gl_name_marker() {
        let wad = crate::Wad::from_bytes(build_pwad(&[
            ("GL_MAP01", b"" as &[u8]),
            ("GL_VERT", b"gNd2"),
            ("GL_SEGS", b""),
            ("GL_SSECT", b""),
            ("GL_NODES", b""),
        ]))
        .unwrap();
        let g = gl_group_in_gl_wad(&wad, "MAP01").expect("gl group");
        assert_eq!(wad.lumps()[g.vert].name(), "GL_VERT");
        assert_eq!(wad.lumps()[g.segs].name(), "GL_SEGS");
        assert_eq!(wad.lumps()[g.ssect].name(), "GL_SSECT");
        assert_eq!(wad.lumps()[g.nodes].name(), "GL_NODES");
    }

    #[test]
    fn gwa_locator_gl_level_marker() {
        let wad = crate::Wad::from_bytes(build_pwad(&[
            ("GL_LEVEL", b"LEVEL=MAP01\n" as &[u8]),
            ("GL_VERT", b"gNd2"),
            ("GL_SEGS", b""),
            ("GL_SSECT", b""),
            ("GL_NODES", b""),
        ]))
        .unwrap();
        let g = gl_group_in_gl_wad(&wad, "MAP01").expect("gl group");
        assert_eq!(wad.lumps()[g.vert].name(), "GL_VERT");
        assert!(
            gl_group_in_gl_wad(&wad, "MAP02").is_none(),
            "LEVEL=MAP01 must not match a different requested map name"
        );
    }

    #[test]
    fn gwa_locator_stops_at_next_group_marker() {
        // Two back-to-back .gwa groups: MAP01's own GL_NODES must be found,
        // not borrowed from MAP02's run.
        let wad = crate::Wad::from_bytes(build_pwad(&[
            ("GL_MAP01", b"" as &[u8]),
            ("GL_VERT", b"AAAA"),
            ("GL_SEGS", b""),
            ("GL_SSECT", b""),
            ("GL_NODES", b""),
            ("GL_MAP02", b""),
            ("GL_VERT", b"BBBBBBBB"),
            ("GL_SEGS", b""),
            ("GL_SSECT", b""),
            ("GL_NODES", b""),
        ]))
        .unwrap();
        let g1 = gl_group_in_gl_wad(&wad, "MAP01").expect("MAP01 gl group");
        let g2 = gl_group_in_gl_wad(&wad, "MAP02").expect("MAP02 gl group");
        assert_eq!(wad.lump_bytes(g1.vert), Some(b"AAAA".as_slice()));
        assert_eq!(wad.lump_bytes(g2.vert), Some(b"BBBBBBBB".as_slice()));
        assert_ne!(g1.vert, g2.vert);
        assert_ne!(g1.nodes, g2.nodes);
    }

    #[test]
    fn gwa_locator_missing_nodes_before_next_marker_returns_none() {
        // MAP01's run is missing GL_NODES before MAP02's marker begins — the
        // scan must not cross into MAP02's run and borrow ITS GL_NODES.
        let wad = crate::Wad::from_bytes(build_pwad(&[
            ("GL_MAP01", b"" as &[u8]),
            ("GL_VERT", b"AAAA"),
            ("GL_SEGS", b""),
            ("GL_SSECT", b""),
            ("GL_MAP02", b""),
            ("GL_VERT", b"BBBBBBBB"),
            ("GL_SEGS", b""),
            ("GL_SSECT", b""),
            ("GL_NODES", b""),
        ]))
        .unwrap();
        assert!(
            gl_group_in_gl_wad(&wad, "MAP01").is_none(),
            "run must stop at MAP02's marker rather than borrowing its GL_NODES"
        );
    }

    #[test]
    fn gwa_locator_absent_returns_none() {
        let wad = crate::Wad::from_bytes(build_pwad(&[("GL_MAP01", b"" as &[u8])])).unwrap();
        assert!(gl_group_in_gl_wad(&wad, "MAP02").is_none());
    }

    #[test]
    fn gl_group_anchored_to_map_instance_with_duplicate_names() {
        // Two MAP01 map instances, each followed by its OWN GL group with
        // distinct contents (different GL_VERT byte lengths). Under the old
        // global-by-name scan, `gl_group_for` would return the FIRST
        // GL_MAP01 group in the WAD for both map instances — this test
        // fails under that behavior, since it asserts the second instance
        // resolves to the second (distinct) GL group, not the first.
        let mut lumps: Vec<(&str, &[u8])> = vec![("MAP01", b"" as &[u8])];
        lumps.extend_from_slice(MIN_MAP_DATA_LUMPS);
        lumps.extend_from_slice(&[
            ("GL_MAP01", b""),
            ("GL_VERT", b"AAAA"),
            ("GL_SEGS", b""),
            ("GL_SSECT", b""),
            ("GL_NODES", b""),
        ]);
        lumps.push(("MAP01", b"" as &[u8]));
        lumps.extend_from_slice(MIN_MAP_DATA_LUMPS);
        lumps.extend_from_slice(&[
            ("GL_MAP01", b""),
            ("GL_VERT", b"BBBBBBBB"),
            ("GL_SEGS", b""),
            ("GL_SSECT", b""),
            ("GL_NODES", b""),
        ]);
        let wad = crate::Wad::from_bytes(build_pwad(&lumps)).unwrap();

        let groups: Vec<MapGroup> = map_groups(&wad)
            .into_iter()
            .filter(|g| g.name == "MAP01")
            .collect();
        assert_eq!(groups.len(), 2, "expected two distinct MAP01 map instances");

        let gl1 = gl_group_for(&wad, &groups[0]).expect("first MAP01's gl group");
        let gl2 = gl_group_for(&wad, &groups[1]).expect("second MAP01's gl group");

        // Indices 9-13: GL_MAP01(9)/GL_VERT(10)/GL_SEGS(11)/GL_SSECT(12)/GL_NODES(13).
        assert_eq!(wad.lump_bytes(gl1.vert), Some(b"AAAA".as_slice()));
        // Indices 23-27: GL_MAP01(23)/GL_VERT(24)/GL_SEGS(25)/GL_SSECT(26)/GL_NODES(27).
        assert_eq!(wad.lump_bytes(gl2.vert), Some(b"BBBBBBBB".as_slice()));
        assert_ne!(
            gl1.vert, gl2.vert,
            "each map instance must get its OWN gl group"
        );
        assert_ne!(gl1.segs, gl2.segs);
        assert_ne!(gl1.ssect, gl2.ssect);
        assert_ne!(gl1.nodes, gl2.nodes);
    }
}
