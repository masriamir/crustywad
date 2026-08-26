//! Pure pk3 path rules, transcribed from `GZDoom`:
//!
//! - namespace from the first path component (`filesystem.cpp`,
//!   `LumpRecord::SetFromLump`'s `strncmp` table);
//! - short name = basename, extension stripped, uppercased, 8 bytes, with
//!   `^` → `\` in the sprites, voxels, and hires namespaces only (same
//!   function; `\` cannot appear in a zip name);
//! - embedded WAD = `.wad` at the root, or `<archive-stem>/<file>.wad`
//!   (`resourcefile.cpp`, `FResourceFile::CheckEmbedded` / `IsFileInFolder`);
//! - maps = `maps/<NAME>.wad` / `maps/<NAME>.map` looked up by full path
//!   (`p_openmap.cpp`).
//!
//! Everything here is a function of the path string alone, so it is
//! unit-tested here and never touches the container.

use super::{MapKind, Namespace};

/// `GZDoom`'s directory → namespace table, in its `strncmp` order.
const DIRECTORIES: [(&str, Namespace); 12] = [
    ("flats", Namespace::Flats),
    ("textures", Namespace::Textures),
    ("hires", Namespace::Hires),
    ("sprites", Namespace::Sprites),
    ("voxels", Namespace::Voxels),
    ("colormaps", Namespace::Colormaps),
    ("acs", Namespace::Acs),
    ("voices", Namespace::Voices),
    ("patches", Namespace::Patches),
    ("graphics", Namespace::Graphics),
    ("sounds", Namespace::Sounds),
    ("music", Namespace::Music),
];

/// The directory name a namespace maps from.
pub(crate) fn directory_of(namespace: Namespace) -> Option<&'static str> {
    DIRECTORIES
        .iter()
        .find(|(_, ns)| *ns == namespace)
        .map(|(dir, _)| *dir)
}

/// Normalizes a raw zip name: `\` → `/` and no leading `/`.
pub(crate) fn normalize_path(raw: &str) -> String {
    let flipped = raw.replace('\\', "/");
    flipped.trim_start_matches('/').to_string()
}

/// The namespace the first path component selects.
pub(crate) fn namespace_of(path: &str) -> Namespace {
    let Some((first, _)) = path.split_once('/') else {
        return Namespace::Global;
    };
    DIRECTORIES
        .iter()
        .find(|(dir, _)| first.eq_ignore_ascii_case(dir))
        .map_or(Namespace::Hidden, |(_, ns)| *ns)
}

/// The engine's short name for an ASCII path, or `None` for hidden members
/// and empty basenames.
pub(crate) fn short_name_of(path: &str, namespace: Namespace) -> Option<String> {
    if namespace == Namespace::Hidden {
        return None;
    }
    let base = path.rsplit('/').next().unwrap_or(path);
    let stem = match base.rfind('.') {
        Some(dot) => &base[..dot],
        None => base,
    };
    if stem.is_empty() {
        return None;
    }
    // `SetFromLump` runs its `memchr(shortName, '^')` replacement only under
    // `if (Namespace == ns_sprites || Namespace == ns_voxels ||
    // Namespace == ns_hires)`, because `^` stands in for the `\` of a sprite
    // frame character and only those three namespaces carry sprite-shaped
    // names. Everywhere else a `^` is an ordinary name character.
    let caret_is_backslash = matches!(
        namespace,
        Namespace::Sprites | Namespace::Voxels | Namespace::Hires
    );
    let name: String = stem
        .chars()
        .take(8)
        .map(|c| {
            if caret_is_backslash && c == '^' {
                '\\'
            } else {
                c.to_ascii_uppercase()
            }
        })
        .collect();
    Some(name)
}

/// Whether `path` ends in `.wad`, ASCII-case-insensitively. Compares bytes
/// so a multi-byte character near the end can never split a `&str` slice.
pub(crate) fn has_wad_extension(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 4 && bytes[bytes.len() - 4..].eq_ignore_ascii_case(b".wad")
}

/// `GZDoom`'s `CheckEmbedded`: a `.wad` at the root, or exactly
/// `<archive_name>/<file>.wad`.
pub(crate) fn is_embedded_wad(path: &str, archive_name: Option<&str>) -> bool {
    if !has_wad_extension(path) {
        return false;
    }
    match path.split_once('/') {
        None => true,
        Some((first, rest)) => {
            archive_name.is_some_and(|name| first.eq_ignore_ascii_case(name)) && !rest.contains('/')
        }
    }
}

/// `maps/<NAME>.wad` → `(NAME, Wad)`, `maps/<NAME>.map` → `(NAME, Textmap)`.
pub(crate) fn map_of(path: &str) -> Option<(String, MapKind)> {
    let (first, rest) = path.split_once('/')?;
    if !first.eq_ignore_ascii_case("maps") || rest.contains('/') {
        return None;
    }
    let dot = rest.rfind('.')?;
    let (stem, ext) = rest.split_at(dot);
    if stem.is_empty() {
        return None;
    }
    let kind = if ext.eq_ignore_ascii_case(".wad") {
        MapKind::Wad
    } else if ext.eq_ignore_ascii_case(".map") {
        MapKind::Textmap
    } else {
        return None;
    };
    Some((stem.to_ascii_uppercase(), kind))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_flips_backslashes_and_strips_a_leading_slash() {
        assert_eq!(normalize_path("maps\\MAP01.wad"), "maps/MAP01.wad");
        assert_eq!(normalize_path("/sprites/TROOA1.png"), "sprites/TROOA1.png");
        assert_eq!(normalize_path("MAPINFO.txt"), "MAPINFO.txt");
    }

    #[test]
    fn namespace_comes_from_the_first_component_case_insensitively() {
        assert_eq!(namespace_of("flats/FLOOR0_1.png"), Namespace::Flats);
        assert_eq!(namespace_of("TEXTURES/FILTER/x.png"), Namespace::Textures);
        assert_eq!(namespace_of("hires/a.png"), Namespace::Hires);
        assert_eq!(namespace_of("SPRITES/TROOA1.png"), Namespace::Sprites);
        assert_eq!(namespace_of("voxels/a.kvx"), Namespace::Voxels);
        assert_eq!(namespace_of("colormaps/a.lmp"), Namespace::Colormaps);
        assert_eq!(namespace_of("acs/lib.o"), Namespace::Acs);
        assert_eq!(namespace_of("voices/a.ogg"), Namespace::Voices);
        assert_eq!(namespace_of("patches/WALL00_1.png"), Namespace::Patches);
        assert_eq!(namespace_of("graphics/TITLEPIC.png"), Namespace::Graphics);
        assert_eq!(namespace_of("sounds/DSPISTOL.wav"), Namespace::Sounds);
        assert_eq!(namespace_of("music/D_E1M1.mid"), Namespace::Music);
        assert_eq!(namespace_of("MAPINFO.txt"), Namespace::Global);
        assert_eq!(namespace_of("maps/MAP01.wad"), Namespace::Hidden);
        assert_eq!(namespace_of("zscript/actors.zs"), Namespace::Hidden);
        assert_eq!(
            namespace_of("Beautiful-Doom/sprites/x.png"),
            Namespace::Hidden
        );
    }

    #[test]
    fn directory_of_round_trips_the_table() {
        assert_eq!(directory_of(Namespace::Sprites), Some("sprites"));
        assert_eq!(directory_of(Namespace::Global), None);
        assert_eq!(directory_of(Namespace::Hidden), None);
    }

    #[test]
    fn short_name_strips_extension_uppercases_and_truncates() {
        assert_eq!(
            short_name_of("MAPINFO.txt", Namespace::Global).as_deref(),
            Some("MAPINFO")
        );
        assert_eq!(
            short_name_of("sprites/trooa1.png", Namespace::Sprites).as_deref(),
            Some("TROOA1")
        );
        assert_eq!(
            short_name_of("graphics/verylongname.png", Namespace::Graphics).as_deref(),
            Some("VERYLONG")
        );
        assert_eq!(
            short_name_of("sprites/TROOA2A8^.png", Namespace::Sprites).as_deref(),
            Some("TROOA2A8")
        );
        assert_eq!(
            short_name_of("sprites/PLAYA^1.png", Namespace::Sprites).as_deref(),
            Some("PLAYA\\1")
        );
        assert_eq!(
            short_name_of("textures/no_ext", Namespace::Textures).as_deref(),
            Some("NO_EXT")
        );
        // Only the *last* dot ends the stem, so an inner dot survives.
        assert_eq!(
            short_name_of("graphics/a.b.png", Namespace::Graphics).as_deref(),
            Some("A.B")
        );
        assert_eq!(short_name_of("maps/MAP01.wad", Namespace::Hidden), None);
        assert_eq!(short_name_of("sprites/", Namespace::Sprites), None);
    }

    #[test]
    fn caret_becomes_a_backslash_only_in_sprites_voxels_and_hires() {
        assert_eq!(
            short_name_of("sprites/PLAYA^1.png", Namespace::Sprites).as_deref(),
            Some("PLAYA\\1")
        );
        assert_eq!(
            short_name_of("voxels/A^B.kvx", Namespace::Voxels).as_deref(),
            Some("A\\B")
        );
        assert_eq!(
            short_name_of("hires/A^B.png", Namespace::Hires).as_deref(),
            Some("A\\B")
        );
        // Every other namespace keeps the `^` verbatim.
        assert_eq!(
            short_name_of("graphics/M^1.png", Namespace::Graphics).as_deref(),
            Some("M^1")
        );
        assert_eq!(
            short_name_of("A^B.lmp", Namespace::Global).as_deref(),
            Some("A^B")
        );
    }

    #[test]
    fn embedded_wad_is_root_or_stem_folder() {
        assert!(is_embedded_wad("extra.wad", None));
        assert!(is_embedded_wad("EXTRA.WAD", None));
        assert!(!is_embedded_wad("maps/MAP01.wad", None));
        assert!(!is_embedded_wad("myproject/extra.wad", None));
        assert!(is_embedded_wad("myproject/extra.wad", Some("myproject")));
        assert!(is_embedded_wad("MyProject/extra.wad", Some("myproject")));
        assert!(!is_embedded_wad(
            "myproject/sub/extra.wad",
            Some("myproject")
        ));
        assert!(!is_embedded_wad("other/extra.wad", Some("myproject")));
        assert!(!is_embedded_wad("readme.txt", Some("myproject")));
        assert!(!is_embedded_wad("extra.wad.bak", None));
    }

    #[test]
    fn embedded_wad_check_never_panics_on_non_ascii_tails() {
        assert!(!is_embedded_wad("\u{1F600}x", None));
        assert!(!is_embedded_wad("gr\u{00e4}fik.png", None));
        assert!(is_embedded_wad("d\u{00e4}ta.wad", None));
    }

    #[test]
    fn maps_are_single_component_wad_or_map_under_maps() {
        assert_eq!(
            map_of("maps/MAP01.wad"),
            Some(("MAP01".to_string(), MapKind::Wad))
        );
        assert_eq!(
            map_of("MAPS/e1m1.WAD"),
            Some(("E1M1".to_string(), MapKind::Wad))
        );
        assert_eq!(
            map_of("maps/MAP02.map"),
            Some(("MAP02".to_string(), MapKind::Textmap))
        );
        // The stem is everything before the last dot, inner dots included.
        assert_eq!(
            map_of("maps/E1M1.v2.wad"),
            Some(("E1M1.V2".to_string(), MapKind::Wad))
        );
        assert_eq!(map_of("maps/sub/MAP03.wad"), None);
        assert_eq!(map_of("maps/.wad"), None);
        assert_eq!(map_of("maps/readme.txt"), None);
        assert_eq!(map_of("MAP01.wad"), None);
    }
}
