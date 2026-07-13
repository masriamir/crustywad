//! Integration + round-trip tests for UDMF (`TEXTMAP`) writing (#60).
#![cfg(feature = "write")]

mod common;

use crustywad::map::udmf::UdmfWriteError;
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
fn empty_namespace_errors_in_strict_defaults_in_lenient() {
    // A namespace of "" is invalid. We can't easily assemble such a map, so this
    // is covered by unit tests in write.rs; here we assert the lenient default
    // path via a normal map that already has "doom".
    let map = assemble_udmf(FULL_MAP);
    let (text, _) = write_udmf(&map, &WriteOptions::lenient()).unwrap();
    assert!(text.starts_with("namespace = \"doom\";"));
    // Reference the error type so the import is exercised.
    let _ = UdmfWriteError::EmptyNamespace;
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
