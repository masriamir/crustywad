//! Integration + round-trip tests for UDMF (`TEXTMAP`) writing (#60).
#![cfg(feature = "write")]

mod common;

use crustywad::map::udmf::{UdmfWriteError, UdmfWriteWarning};
use crustywad::map::{Map, write_udmf};
use crustywad::{ParseOptions, Wad, WriteOptions};

/// A complete one-of-each-block UDMF map (mirrors the assembly test fixture).
const FULL_MAP: &str = concat!(
    "namespace = \"doom\";\n",
    "vertex { x = 0.0; y = 0.0; }\n",
    "vertex { x = 64.0; y = 0.0; }\n",
    "linedef { v1 = 0; v2 = 1; sidefront = 0; }\n",
    "sidedef { sector = 0; }\n",
    "sector { texturefloor = \"F\"; textureceiling = \"C\"; }\n",
    "thing { x = 16.0; y = 16.0; type = 1; }\n",
);

fn assemble_udmf(text: &str) -> Map {
    let bytes = common::build_named_lumps(&[
        ("MAP01", vec![]),
        ("TEXTMAP", text.as_bytes().to_vec()),
        ("ENDMAP", vec![]),
    ]);
    let wad = Wad::from_bytes_with_options(bytes, ParseOptions::default()).unwrap();
    let group = wad.map_group("MAP01").unwrap();
    Map::assemble_with_options(&wad, &group, ParseOptions::default()).unwrap()
}

#[test]
fn writes_namespace_and_vertices() {
    let map = assemble_udmf(FULL_MAP);
    let (text, warnings) = write_udmf(&map, &WriteOptions::strict()).unwrap();
    assert!(warnings.is_empty());
    // Namespace header first, then integer-narrowed vertex coordinates.
    assert!(text.starts_with("namespace = \"doom\";\n"), "got:\n{text}");
    assert!(text.contains("vertex { x = 0; y = 0; }\n"), "got:\n{text}");
    assert!(text.contains("vertex { x = 64; y = 0; }\n"), "got:\n{text}");
}

#[test]
fn lenient_preserves_existing_namespace_without_warnings() {
    // A map that already carries a namespace keeps it verbatim in lenient mode
    // and produces no warnings. (The missing/empty-namespace default and error
    // paths are covered by their own tests below.)
    let map = assemble_udmf(FULL_MAP);
    let (text, warnings) = write_udmf(&map, &WriteOptions::lenient()).unwrap();
    assert!(text.starts_with("namespace = \"doom\";"));
    assert!(warnings.is_empty());
}

#[test]
fn writes_linedef_flags_sideback_and_special() {
    // Two-sided line with blocking+twosided flags, a special+arg, and an id.
    let text = concat!(
        "namespace = \"doom\";\n",
        "vertex { x = 0; y = 0; }\n",
        "vertex { x = 8; y = 0; }\n",
        "linedef { v1 = 0; v2 = 1; sidefront = 0; sideback = 1; id = 7;\n",
        "  blocking = true; twosided = true; special = 13; arg0 = 99; }\n",
        "sidedef { sector = 0; }\n",
        "sidedef { sector = 0; }\n",
        "sector { texturefloor = \"F\"; textureceiling = \"C\"; }\n",
    );
    let map = assemble_udmf(text);
    let (out, _) = write_udmf(&map, &WriteOptions::strict()).unwrap();
    let line = out.lines().find(|l| l.starts_with("linedef")).unwrap();
    assert!(line.contains("v1 = 0; v2 = 1; sidefront = 0; "), "{line}");
    assert!(line.contains("sideback = 1; "), "{line}");
    assert!(line.contains("id = 7; "), "{line}");
    assert!(line.contains("special = 13; "), "{line}");
    assert!(line.contains("arg0 = 99; "), "{line}");
    assert!(line.contains("blocking = true; "), "{line}");
    assert!(line.contains("twosided = true; "), "{line}");
    // Defaults omitted:
    assert!(!line.contains("arg1"), "{line}");
    assert!(!line.contains("dontpegtop"), "{line}");
}

#[test]
fn writes_one_sided_linedef_omits_sideback_and_id() {
    let map = assemble_udmf(FULL_MAP);
    let (out, _) = write_udmf(&map, &WriteOptions::strict()).unwrap();
    let line = out.lines().find(|l| l.starts_with("linedef")).unwrap();
    assert!(!line.contains("sideback = "), "{line}"); // left is None
    assert!(!line.contains("id = "), "{line}"); // id == -1 default
    assert!(!line.contains("special = "), "{line}"); // special == 0
}

#[test]
fn writes_sidedef_textures_and_offsets_omitting_defaults() {
    let text = concat!(
        "namespace = \"doom\";\n",
        "vertex { x = 0; y = 0; }\n",
        "vertex { x = 8; y = 0; }\n",
        "linedef { v1 = 0; v2 = 1; sidefront = 0; }\n",
        "sidedef { sector = 0; offsetx = 4; texturetop = \"BRICK\"; }\n",
        "sector { texturefloor = \"F\"; textureceiling = \"C\"; heightceiling = 128; lightlevel = 200; id = 5; }\n",
    );
    let map = assemble_udmf(text);
    let (out, _) = write_udmf(&map, &WriteOptions::strict()).unwrap();

    let side = out.lines().find(|l| l.starts_with("sidedef")).unwrap();
    assert!(side.contains("sector = 0; "), "{side}");
    assert!(side.contains("offsetx = 4; "), "{side}");
    assert!(side.contains("texturetop = \"BRICK\"; "), "{side}");
    assert!(!side.contains("offsety = "), "{side}"); // 0 default
    assert!(!side.contains("texturebottom = "), "{side}"); // "-"/"" default

    let sector = out.lines().find(|l| l.starts_with("sector")).unwrap();
    assert!(
        sector.contains("texturefloor = \"F\"; textureceiling = \"C\"; "),
        "{sector}"
    );
    assert!(sector.contains("heightceiling = 128; "), "{sector}");
    assert!(sector.contains("lightlevel = 200; "), "{sector}");
    assert!(sector.contains("id = 5; "), "{sector}");
    assert!(!sector.contains("heightfloor = "), "{sector}"); // 0 default
    assert!(!sector.contains("special = "), "{sector}"); // 0 default
}

#[test]
fn omits_lightlevel_at_udmf_default_160() {
    let text = concat!(
        "namespace = \"doom\";\n",
        "vertex { x = 0; y = 0; }\n",
        "vertex { x = 8; y = 0; }\n",
        "linedef { v1 = 0; v2 = 1; sidefront = 0; }\n",
        "sidedef { sector = 0; }\n",
        "sector { texturefloor = \"F\"; textureceiling = \"C\"; lightlevel = 160; }\n",
    );
    let map = assemble_udmf(text);
    let (out, _) = write_udmf(&map, &WriteOptions::strict()).unwrap();
    let sector = out.lines().find(|l| l.starts_with("sector")).unwrap();
    assert!(!sector.contains("lightlevel = "), "{sector}");
}

#[test]
fn explicitly_empty_texture_round_trips_distinct_from_default() {
    // An explicit `texturetop = ""` is preserved by the read side as an empty
    // string, distinct from the `"-"` default. The writer must emit it (not omit
    // it as if it were the default) so it survives a write -> read round-trip.
    let text = concat!(
        "namespace = \"doom\";\n",
        "vertex { x = 0; y = 0; }\n",
        "vertex { x = 8; y = 0; }\n",
        "linedef { v1 = 0; v2 = 1; sidefront = 0; }\n",
        "sidedef { sector = 0; texturetop = \"\"; }\n",
        "sector { texturefloor = \"F\"; textureceiling = \"C\"; }\n",
    );
    let map = assemble_udmf(text);
    assert_eq!(map.sidedefs()[0].upper, "");
    let (out, _) = write_udmf(&map, &WriteOptions::strict()).unwrap();
    let side = out.lines().find(|l| l.starts_with("sidedef")).unwrap();
    assert!(side.contains("texturetop = \"\"; "), "{side}");
    // `texturebottom`/`texturemiddle` default to `"-"` and stay omitted.
    assert!(!side.contains("texturebottom"), "{side}");
    let reparsed = assemble_udmf(&out);
    assert_eq!(reparsed.sidedefs()[0].upper, "");
}

#[test]
fn writes_thing_fields_omitting_defaults() {
    let text = concat!(
        "namespace = \"doom\";\n",
        "vertex { x = 0; y = 0; }\n",
        "vertex { x = 8; y = 0; }\n",
        "linedef { v1 = 0; v2 = 1; sidefront = 0; }\n",
        "sidedef { sector = 0; }\n",
        "sector { texturefloor = \"F\"; textureceiling = \"C\"; }\n",
        "thing { x = 1.5; y = 2; type = 3001; angle = 90; id = 5; special = 80; arg0 = 1; height = 24; }\n",
    );
    let map = assemble_udmf(text);
    let (out, _) = write_udmf(&map, &WriteOptions::strict()).unwrap();
    let t = out.lines().find(|l| l.starts_with("thing")).unwrap();
    assert!(t.contains("x = 1.5; "), "{t}"); // non-whole float preserved
    assert!(t.contains("y = 2; "), "{t}"); // whole float narrowed
    assert!(t.contains("type = 3001; "), "{t}");
    assert!(t.contains("angle = 90; "), "{t}");
    assert!(t.contains("id = 5; "), "{t}");
    assert!(t.contains("special = 80; "), "{t}");
    assert!(t.contains("arg0 = 1; "), "{t}");
    assert!(t.contains("height = 24; "), "{t}");
    assert!(!t.contains("arg1"), "{t}"); // 0 default
}

// Thing-flag mapping and non-finite handling are covered by unit tests inside
// write.rs (they need direct `Map`/`Writer` construction that the public
// assembly path cannot easily produce for flags/NaN). See Step 3b.

use crustywad::map::add_udmf_map;
use crustywad::{WadBuilder, WadKind};

fn assert_maps_eq(a: &Map, b: &Map) {
    assert_eq!(a.namespace(), b.namespace());
    assert_eq!(a.format(), b.format());
    assert_eq!(a.vertices(), b.vertices());
    assert_eq!(a.linedefs(), b.linedefs());
    assert_eq!(a.sidedefs(), b.sidedefs());
    assert_eq!(a.sectors(), b.sectors());
    assert_eq!(a.things(), b.things());
}

#[test]
fn write_then_read_round_trips_the_map() {
    let original = assemble_udmf(FULL_MAP);
    let (text, warnings) = write_udmf(&original, &WriteOptions::strict()).unwrap();
    assert!(warnings.is_empty());
    let reparsed = assemble_udmf(&text);
    assert_maps_eq(&original, &reparsed);
}

#[test]
fn add_udmf_map_builds_a_readable_udmf_wad() {
    let original = assemble_udmf(FULL_MAP);
    let mut builder = WadBuilder::new(WadKind::Pwad);
    let warnings = add_udmf_map(&mut builder, "MAP01", &original, &WriteOptions::strict()).unwrap();
    assert!(warnings.is_empty());
    let bytes = builder.build().unwrap();

    let wad = Wad::from_bytes_with_options(bytes, ParseOptions::default()).unwrap();
    // The three group lumps are present and named.
    let names: Vec<&str> = wad.lumps().iter().map(crustywad::Lump::name).collect();
    assert_eq!(names, vec!["MAP01", "TEXTMAP", "ENDMAP"]);
    let group = wad.map_group("MAP01").unwrap();
    let reassembled = Map::assemble_with_options(&wad, &group, ParseOptions::default()).unwrap();
    assert_maps_eq(&original, &reassembled);
}

use proptest::prelude::*;

proptest! {
    // Round-trip arbitrary scalar values through a fixed map structure: build
    // UDMF text with random field values, assemble, write, re-assemble, and
    // assert the two Maps are identical (write reproduces the source exactly).
    #[test]
    fn round_trip_arbitrary_scalars(
        vx in -32768.0f64..32768.0,
        vy in -32768.0f64..32768.0,
        floor in -32768i32..32768,
        ceil in -32768i32..32768,
        light in 0i32..=255,
        angle in 0i32..360,
        ty in 1i32..=32767,
    ) {
        // Single multi-line literal with inline captures (no concat!, no explicit
        // args) — clippy-clean and unambiguous. Braces are doubled for UDMF blocks.
        let text = format!(
"namespace = \"doom\";
vertex {{ x = {vx}; y = {vy}; }}
vertex {{ x = 0; y = 0; }}
linedef {{ v1 = 0; v2 = 1; sidefront = 0; }}
sidedef {{ sector = 0; }}
sector {{ texturefloor = \"F\"; textureceiling = \"C\"; heightfloor = {floor}; heightceiling = {ceil}; lightlevel = {light}; }}
thing {{ x = 0; y = 0; type = {ty}; angle = {angle}; }}
");
        let original = assemble_udmf(&text);
        let (out, warnings) = write_udmf(&original, &WriteOptions::strict()).unwrap();
        prop_assert!(warnings.is_empty());
        let reparsed = assemble_udmf(&out);
        prop_assert_eq!(original.vertices(), reparsed.vertices());
        prop_assert_eq!(original.sectors(), reparsed.sectors());
        prop_assert_eq!(original.things(), reparsed.things());
        prop_assert_eq!(original.linedefs(), reparsed.linedefs());
        prop_assert_eq!(original.sidedefs(), reparsed.sidedefs());
    }
}

// --- Coverage: `map.namespace()` is `None` (a binary Doom map). ---

fn doom_vertex(x: i16, y: i16) -> Vec<u8> {
    [x.to_le_bytes(), y.to_le_bytes()].concat()
}

fn doom_sector() -> Vec<u8> {
    // 26 bytes: floor(i16), ceiling(i16), floor_flat(8), ceiling_flat(8), light(i16), special(i16), tag(i16).
    let mut b = Vec::new();
    b.extend(0i16.to_le_bytes());
    b.extend(128i16.to_le_bytes());
    b.extend(b"FLOOR\0\0\0");
    b.extend(b"CEIL\0\0\0\0");
    b.extend(160i16.to_le_bytes());
    b.extend(0i16.to_le_bytes());
    b.extend(0i16.to_le_bytes());
    b
}

fn doom_sidedef(offset_y: i16, sector: u16) -> Vec<u8> {
    // 30 bytes: x_off(i16), y_off(i16), upper(8), lower(8), middle(8), sector(u16).
    let mut b = Vec::new();
    b.extend(0i16.to_le_bytes());
    b.extend(offset_y.to_le_bytes());
    b.extend(b"-\0\0\0\0\0\0\0");
    b.extend(b"-\0\0\0\0\0\0\0");
    b.extend(b"WALL\0\0\0\0");
    b.extend(sector.to_le_bytes());
    b
}

fn doom_linedef(v1: u16, v2: u16, right: u16, left: u16) -> Vec<u8> {
    // 14 bytes: v1, v2, flags, special, tag, sidefront, sideback.
    [
        v1.to_le_bytes(),
        v2.to_le_bytes(),
        0u16.to_le_bytes(),
        0u16.to_le_bytes(),
        0u16.to_le_bytes(),
        right.to_le_bytes(),
        left.to_le_bytes(),
    ]
    .concat()
}

fn assemble_doom_map() -> Map {
    let bytes = common::build_doom_map_wad(
        "E1M1",
        /* things */ Vec::new(),
        /* linedefs */ doom_linedef(0, 1, 0, 0xffff),
        /* sidedefs */ doom_sidedef(0, 0),
        /* vertexes */ [doom_vertex(0, 0), doom_vertex(64, 0)].concat(),
        /* sectors */ doom_sector(),
    );
    let wad = Wad::from_bytes(bytes).unwrap();
    let group = wad.map_group("E1M1").unwrap();
    Map::assemble(&wad, &group).unwrap()
}

#[test]
fn none_namespace_strict_defaults_without_warning() {
    let map = assemble_doom_map();
    assert_eq!(map.namespace(), None);
    let (text, warnings) = write_udmf(&map, &WriteOptions::strict()).unwrap();
    assert!(text.starts_with("namespace = \"doom\";"), "got:\n{text}");
    assert!(warnings.is_empty());
}

#[test]
fn none_namespace_lenient_defaults_with_warning() {
    let map = assemble_doom_map();
    let (text, warnings) = write_udmf(&map, &WriteOptions::lenient()).unwrap();
    assert!(text.starts_with("namespace = \"doom\";"), "got:\n{text}");
    assert_eq!(
        warnings,
        vec![UdmfWriteWarning::NamespaceDefaulted { used: "doom" }]
    );
}

#[test]
fn doom_sourced_linedef_omits_id_zero() {
    // A Doom map's line has `id == 0` ("no id" in the graph). Because the source
    // is not UDMF, the writer must NOT emit `id = 0` (which would assert a real
    // UDMF line id of 0) — it omits it. A genuine UDMF line with `id = 0` is
    // still emitted (covered by the UDMF-path tests above).
    let map = assemble_doom_map();
    assert_eq!(map.format(), crustywad::map::MapFormat::Doom);
    let (out, _) = write_udmf(&map, &WriteOptions::strict()).unwrap();
    let line = out.lines().find(|l| l.starts_with("linedef")).unwrap();
    assert!(!line.contains("id = "), "{line}");
}

// --- Coverage: `map.namespace()` is `Some("")` (an explicitly empty UDMF namespace). ---

const EMPTY_NAMESPACE_MAP: &str = concat!(
    "namespace = \"\";\n",
    "vertex { x = 0; y = 0; }\n",
    "vertex { x = 64; y = 0; }\n",
    "linedef { v1 = 0; v2 = 1; sidefront = 0; }\n",
    "sidedef { sector = 0; }\n",
    "sector { texturefloor = \"F\"; textureceiling = \"C\"; }\n",
);

#[test]
fn empty_namespace_strict_errors() {
    let map = assemble_udmf(EMPTY_NAMESPACE_MAP);
    assert_eq!(map.namespace(), Some(""));
    let err = write_udmf(&map, &WriteOptions::strict()).unwrap_err();
    assert_eq!(err, UdmfWriteError::EmptyNamespace);
}

#[test]
fn empty_namespace_lenient_defaults_with_warning() {
    let map = assemble_udmf(EMPTY_NAMESPACE_MAP);
    let (text, warnings) = write_udmf(&map, &WriteOptions::lenient()).unwrap();
    assert!(text.starts_with("namespace = \"doom\";"), "got:\n{text}");
    assert_eq!(
        warnings,
        vec![UdmfWriteWarning::NamespaceDefaulted { used: "doom" }]
    );
}

// --- Coverage: sidedef `offsety` and sector `special`, neither exercised above. ---

#[test]
fn writes_sidedef_offsety_and_sector_special() {
    let text = concat!(
        "namespace = \"doom\";\n",
        "vertex { x = 0; y = 0; }\n",
        "vertex { x = 8; y = 0; }\n",
        "linedef { v1 = 0; v2 = 1; sidefront = 0; }\n",
        "sidedef { sector = 0; offsety = 5; }\n",
        "sector { texturefloor = \"F\"; textureceiling = \"C\"; special = 9; }\n",
    );
    let map = assemble_udmf(text);
    let (out, _) = write_udmf(&map, &WriteOptions::strict()).unwrap();

    let side = out.lines().find(|l| l.starts_with("sidedef")).unwrap();
    assert!(side.contains("offsety = 5; "), "{side}");

    let sector = out.lines().find(|l| l.starts_with("sector")).unwrap();
    assert!(sector.contains("special = 9; "), "{sector}");
}

// --- Hexen -> UDMF: thing flags must not come out inverted (ADR-0019 §2). ---

/// A Hexen map's thing flags are normalized into the Doom/Boom-MBF layout at
/// assembly, so `write_udmf` — which accepts Hexen maps — emits the *correct*
/// game-mode booleans for them. Hexen's bits are positive and sit at
/// `0x0100`/`0x0200`/`0x0400`; before normalization they were copied through
/// verbatim and this thing (present in all three modes) serialized as present in
/// none.
#[test]
fn hexen_thing_present_in_all_modes_writes_all_three_udmf_game_modes() {
    // All skills (0x0007) + single (0x0100) + co-op (0x0200) + deathmatch (0x0400).
    let bytes = common::hexen_map_bytes_with_thing_flags(0x0007 | 0x0100 | 0x0200 | 0x0400);
    let wad = Wad::from_bytes(bytes).unwrap();
    let group = wad.map_group("MAP01").unwrap();
    let map = Map::assemble(&wad, &group).unwrap();

    let (out, _) = write_udmf(&map, &WriteOptions::strict()).unwrap();
    assert!(out.starts_with("namespace = \"hexen\";"), "got:\n{out}");
    let thing = out.lines().find(|l| l.starts_with("thing")).unwrap();
    assert!(thing.contains("single = true; "), "{thing}");
    assert!(thing.contains("dm = true; "), "{thing}");
    assert!(thing.contains("coop = true; "), "{thing}");
    // All skills carried across; no `friend` (Hexen has no such flag).
    assert!(thing.contains("skill1 = true; skill2 = true; "), "{thing}");
    assert!(!thing.contains("friend"), "{thing}");
}
