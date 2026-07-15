use std::collections::HashMap;
use std::path::PathBuf;

use proptest::prelude::*;

fn encode_i32(value: usize) -> [u8; 4] {
    i32::try_from(value)
        .expect("test fixture values should fit within i32")
        .to_le_bytes()
}

pub fn build_wad(kind: [u8; 4], lumps: &[(&str, &[u8])]) -> Vec<u8> {
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
    wad.extend_from_slice(&kind);
    wad.extend_from_slice(&encode_i32(lumps.len()));
    wad.extend_from_slice(&encode_i32(directory_offset));
    wad.extend_from_slice(&payload);
    wad.extend_from_slice(&directory);
    wad
}

#[allow(dead_code)]
pub fn lump_map<'a>(pairs: &'a [(&'a str, &'a [u8])]) -> HashMap<&'a str, &'a [u8]> {
    pairs.iter().copied().collect()
}

/// Builds a PWAD whose lumps are the given `(name, data)` pairs in order.
/// Lump names are written into the fixed 8-byte on-disk name field; names
/// longer than 8 bytes are truncated to 8 bytes by `build_wad`.
#[allow(dead_code)]
pub fn build_named_lumps(lumps: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let refs: Vec<(&str, &[u8])> = lumps
        .iter()
        .map(|(name, data)| (*name, data.as_slice()))
        .collect();
    build_wad(*b"PWAD", &refs)
}

/// Builds a single classic-Doom map: marker `name` followed by THINGS,
/// LINEDEFS, SIDEDEFS, VERTEXES, SECTORS lumps carrying the given raw bytes.
#[allow(dead_code)]
pub fn build_doom_map_wad(
    name: &str,
    things: Vec<u8>,
    linedefs: Vec<u8>,
    sidedefs: Vec<u8>,
    vertexes: Vec<u8>,
    sectors: Vec<u8>,
) -> Vec<u8> {
    build_named_lumps(&[
        (name, Vec::new()),
        ("THINGS", things),
        ("LINEDEFS", linedefs),
        ("SIDEDEFS", sidedefs),
        ("VERTEXES", vertexes),
        ("SECTORS", sectors),
    ])
}

/// Builds a single Hexen map: marker `name` followed by THINGS, LINEDEFS,
/// SIDEDEFS, VERTEXES, SECTORS, and a BEHAVIOR lump (which marks the map as
/// Hexen; its bytes are irrelevant to assembly).
#[allow(dead_code)]
pub fn build_hexen_map_wad(
    name: &str,
    things: Vec<u8>,
    linedefs: Vec<u8>,
    sidedefs: Vec<u8>,
    vertexes: Vec<u8>,
    sectors: Vec<u8>,
    behavior: Vec<u8>,
) -> Vec<u8> {
    build_named_lumps(&[
        (name, Vec::new()),
        ("THINGS", things),
        ("LINEDEFS", linedefs),
        ("SIDEDEFS", sidedefs),
        ("VERTEXES", vertexes),
        ("SECTORS", sectors),
        ("BEHAVIOR", behavior),
    ])
}

/// Builds a minimal but complete Hexen map WAD, used both by the assembly test
/// and as the fuzz seed: 2 vertices, 1 sector, 1 sidedef, 1 Hexen thing
/// (tid=7, z=24, special=80, args=[1,2,3,4,5]), and 1 one-sided Hexen linedef
/// (special=13, args=[99,0,0,0,0], left=0xffff).
#[allow(dead_code)]
pub fn hexen_sample_map_bytes() -> Vec<u8> {
    hexen_map_bytes_with_thing_flags(0x0007)
}

/// The same map as [`hexen_sample_map_bytes`], with the thing's on-disk Hexen
/// `flags` word set to `flags` — used to exercise Hexen thing-flag normalization
/// (ADR-0019 §2), where the on-disk bits differ from the graph's Doom layout.
#[allow(dead_code)]
pub fn hexen_map_bytes_with_thing_flags(flags: u16) -> Vec<u8> {
    let vertexes = [0i16, 0, 64, 0] // (0,0) and (64,0)
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect::<Vec<u8>>();
    // Sidedef (30 B): x_off, y_off, upper/lower/middle=8B each, sector=0.
    let mut sidedef = vec![0u8; 4];
    sidedef.extend_from_slice(&[b'-', 0, 0, 0, 0, 0, 0, 0]);
    sidedef.extend_from_slice(&[b'-', 0, 0, 0, 0, 0, 0, 0]);
    sidedef.extend_from_slice(&[b'W', b'A', b'L', b'L', 0, 0, 0, 0]);
    sidedef.extend_from_slice(&0u16.to_le_bytes());
    // Sector (26 B): floor=0, ceil=128, flats 8B each, light=160, special=0, tag=0.
    let mut sector = Vec::new();
    sector.extend_from_slice(&0i16.to_le_bytes());
    sector.extend_from_slice(&128i16.to_le_bytes());
    sector.extend_from_slice(&[b'F', b'L', b'O', b'O', b'R', 0, 0, 0]);
    sector.extend_from_slice(&[b'C', b'E', b'I', b'L', 0, 0, 0, 0]);
    sector.extend_from_slice(&160i16.to_le_bytes());
    sector.extend_from_slice(&0i16.to_le_bytes());
    sector.extend_from_slice(&0i16.to_le_bytes());
    // Hexen thing (20 B): tid=7, x=16, y=16, z=24, angle=90, type=1, `flags`, special=80, args=[1..5].
    let mut thing: Vec<u8> = vec![
        0x07, 0x00, 0x10, 0x00, 0x10, 0x00, 0x18, 0x00, 0x5A, 0x00, 0x01, 0x00,
    ];
    thing.extend_from_slice(&flags.to_le_bytes());
    thing.extend_from_slice(&[0x50, 0x01, 0x02, 0x03, 0x04, 0x05]);
    // Hexen linedef (16 B): start=0,end=1,flags=1,special=13,args=[99,0,0,0,0],right=0,left=0xffff.
    let linedef: Vec<u8> = vec![
        0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x0D, 0x63, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF,
        0xFF,
    ];
    build_hexen_map_wad(
        "MAP01",
        thing,
        linedef,
        sidedef,
        vertexes,
        sector,
        b"ACS\0".to_vec(),
    )
}

/// Generates an ASCII lump name (1–8 chars) and a payload (0–256 bytes).
///
/// The name charset covers upper- and lower-case letters, digits, and underscores —
/// all are valid ASCII of at most 8 bytes, so `WadBuilder::build`/`build_with_options`
/// accept them without a name-related error. Uppercase-only would also be valid, but
/// narrowing the charset would underspecify what the write path actually accepts.
#[allow(dead_code)]
pub fn arb_lump_pair() -> impl Strategy<Value = (String, Vec<u8>)> {
    let name = proptest::string::string_regex("[A-Za-z0-9_]{1,8}").unwrap();
    let data = proptest::collection::vec(any::<u8>(), 0..=256);
    (name, data)
}

/// Generates structurally valid WAD bytes (correct header offsets, ASCII names).
#[allow(dead_code)]
pub fn arb_valid_wad() -> impl Strategy<Value = Vec<u8>> {
    let kind = prop_oneof![Just(*b"IWAD"), Just(*b"PWAD"),];
    let lumps = proptest::collection::vec(arb_lump_pair(), 0..=16);
    (kind, lumps).prop_map(|(k, pairs)| {
        let refs: Vec<(&str, &[u8])> = pairs
            .iter()
            .map(|(n, d)| (n.as_str(), d.as_slice()))
            .collect();
        build_wad(k, &refs)
    })
}

/// Directory named by `env_var`, if that environment variable is set.
///
/// Used by the optional real-IWAD fixture tests to locate a caller-supplied
/// WAD directory (e.g. `CRUSTYWAD_FREEDOOM_DIR`, `CRUSTYWAD_HEXEN_DIR`).
#[allow(dead_code)]
pub fn iwad_dir(env_var: &str) -> Option<PathBuf> {
    std::env::var_os(env_var).map(PathBuf::from)
}

/// Files from `candidates` that exist inside `iwad_dir(env_var)`, returned in listed order.
///
/// Returns an empty `Vec` — the caller should skip its test — when `env_var`
/// is unset, points to a non-directory, or contains none of the candidates;
/// prints a skip note to stderr in each case.
#[allow(dead_code)]
pub fn iwad_files(env_var: &str, candidates: &[&str]) -> Vec<PathBuf> {
    let Some(dir) = iwad_dir(env_var) else {
        eprintln!("skipping fixture test: {env_var} not set");
        return Vec::new();
    };
    if !dir.is_dir() {
        eprintln!(
            "skipping fixture test: {env_var} ({}) is not a directory",
            dir.display()
        );
        return Vec::new();
    }
    let found: Vec<PathBuf> = candidates
        .iter()
        .map(|name| dir.join(name))
        .filter(|path| path.is_file())
        .collect();
    if found.is_empty() {
        eprintln!(
            "skipping fixture test: {env_var}: no fixtures found in {}",
            dir.display()
        );
    }
    found
}

/// Every `*.wad` file (case-insensitive extension) inside `iwad_dir(env_var)`,
/// sorted by path.
///
/// Returns an empty `Vec` — the caller should skip its test — when `env_var`
/// is unset, points to a non-directory, or contains no WAD files; prints a
/// skip note to stderr in each case.
#[allow(dead_code)]
pub fn wad_files(env_var: &str) -> Vec<PathBuf> {
    let Some(dir) = iwad_dir(env_var) else {
        eprintln!("skipping fixture test: {env_var} not set");
        return Vec::new();
    };
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!(
                "skipping fixture test: {env_var} ({}) is not a readable directory: {e}",
                dir.display()
            );
            return Vec::new();
        }
    };
    // Fail fast on an unreadable entry rather than silently dropping it — a
    // sweep that skips part of the directory would pass without actually
    // covering the collection.
    let mut found: Vec<PathBuf> = entries
        .map(|entry| {
            entry
                .unwrap_or_else(|e| {
                    panic!(
                        "{env_var}: failed to read an entry in {}: {e}",
                        dir.display()
                    )
                })
                .path()
        })
        .filter(|p| {
            if !p
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("wad"))
            {
                return false;
            }
            // `Path::is_file` swallows metadata errors (permissions, broken
            // symlinks) as `false`; stat explicitly so those fail loudly too.
            let meta = std::fs::metadata(p)
                .unwrap_or_else(|e| panic!("{env_var}: failed to stat {}: {e}", p.display()));
            meta.is_file()
        })
        .collect();
    found.sort();
    if found.is_empty() {
        eprintln!(
            "skipping fixture test: {env_var}: no WAD files found in {}",
            dir.display()
        );
    }
    found
}

/// A Doom 64 map marker lump is named `MAPxx` (`MAP` + two ASCII digits).
///
/// Shared by the `doom64-tests` fixture test and the `sweep-tests` sweep so
/// the naming rule cannot drift between them.
#[allow(dead_code)]
pub fn is_doom64_map_name(name: &str) -> bool {
    name.len() == 5 && name.starts_with("MAP") && name[3..].bytes().all(|b| b.is_ascii_digit())
}

/// Builds a WAD holding one Doom 64 nested-WAD map lump named `name`.
/// All 9 record sub-lumps `read_doom64_map` expects are present (empty unless
/// supplied), plus the four raw-byte lumps (`REJECT`/`BLOCKMAP`/`LEAFS`/
/// `MACROS`) it carries opaquely, so strict reads succeed.
#[allow(dead_code, clippy::too_many_arguments)]
pub fn build_doom64_map_wad(
    name: &str,
    things: &[u8],
    linedefs: &[u8],
    sidedefs: &[u8],
    vertexes: &[u8],
    sectors: &[u8],
    lights: &[u8],
    segs: &[u8],
    subsectors: &[u8],
    nodes: &[u8],
) -> Vec<u8> {
    let nested = build_wad(
        *b"IWAD",
        &[
            ("THINGS", things),
            ("LINEDEFS", linedefs),
            ("SIDEDEFS", sidedefs),
            ("VERTEXES", vertexes),
            ("SECTORS", sectors),
            ("LIGHTS", lights),
            ("SEGS", segs),
            ("SSECTORS", subsectors),
            ("NODES", nodes),
            ("REJECT", &[]),
            ("BLOCKMAP", &[]),
            ("LEAFS", &[]),
            ("MACROS", &[]),
        ],
    );
    build_named_lumps(&[(name, nested)])
}

/// One Doom 64 vertex: 16.16 fixed-point coordinates.
#[allow(dead_code)]
pub fn d64_vertex(x: f64, y: f64) -> Vec<u8> {
    #[allow(clippy::cast_possible_truncation)]
    let (xf, yf) = ((x * 65536.0) as i32, (y * 65536.0) as i32);
    [xf.to_le_bytes(), yf.to_le_bytes()].concat()
}

/// One Doom 64 linedef (16 bytes).
#[allow(dead_code)]
pub fn d64_linedef(v1: u16, v2: u16, flags: u32, right: u16, left: u16) -> Vec<u8> {
    [
        &v1.to_le_bytes()[..],
        &v2.to_le_bytes(),
        &flags.to_le_bytes(),
        &0u16.to_le_bytes(), // special
        &7u16.to_le_bytes(), // tag
        &right.to_le_bytes(),
        &left.to_le_bytes(),
    ]
    .concat()
}

/// One Doom 64 sidedef (12 bytes) with texture indices.
#[allow(dead_code)]
pub fn d64_sidedef(upper: u16, lower: u16, middle: u16, sector: u16) -> Vec<u8> {
    [
        &0i16.to_le_bytes()[..],
        &0i16.to_le_bytes(),
        &upper.to_le_bytes(),
        &lower.to_le_bytes(),
        &middle.to_le_bytes(),
        &sector.to_le_bytes(),
    ]
    .concat()
}

/// One Doom 64 sector (24 bytes) with flat indices and five color refs.
#[allow(dead_code)]
pub fn d64_sector(floor_tex: u16, ceiling_tex: u16, colors: [u16; 5], flags: u16) -> Vec<u8> {
    let mut b: Vec<u8> = Vec::new();
    b.extend(0i16.to_le_bytes());
    b.extend(128i16.to_le_bytes());
    b.extend(floor_tex.to_le_bytes());
    b.extend(ceiling_tex.to_le_bytes());
    for c in colors {
        b.extend(c.to_le_bytes());
    }
    b.extend(0u16.to_le_bytes()); // special
    b.extend(0u16.to_le_bytes()); // tag
    b.extend(flags.to_le_bytes());
    b
}

/// One Doom 64 thing (14 bytes).
#[allow(dead_code)]
pub fn d64_thing(x: i16, y: i16, z: i16, angle: i16, type_id: i16, flags: i16, id: i16) -> Vec<u8> {
    [x, y, z, angle, type_id, flags, id]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect()
}

/// One LIGHTS record (6 bytes).
#[allow(dead_code)]
pub fn d64_light(r: u8, g: u8, b: u8, tag: u8) -> Vec<u8> {
    vec![r, g, b, tag, 0, 0]
}
