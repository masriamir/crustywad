//! Integration tests for the public UDMF text-map parser.

use crustywad::Limits;
use crustywad::map::udmf::{UdmfParseError, UdmfValue, parse_udmf};

const MINIMAL: &str = concat!(
    "namespace = \"doom\";\n",
    "vertex { x = 0.0; y = 0.0; }\n",
    "vertex { x = 64.0; y = 0.0; }\n",
    "linedef { v1 = 0; v2 = 1; sidefront = 0; }\n",
    "sidedef { sector = 0; }\n",
    "sector { texturefloor = \"F\"; textureceiling = \"C\"; }\n",
    "thing { x = 0.0; y = 0.0; type = 1; }\n",
);

#[test]
fn parses_a_minimal_valid_map() {
    let m = parse_udmf(MINIMAL, Limits::default()).unwrap();
    assert_eq!(m.namespace, "doom");
    assert_eq!(m.vertices.len(), 2);
    assert_eq!(m.linedefs.len(), 1);
    assert_eq!(m.sidedefs.len(), 1);
    assert_eq!(m.sectors.len(), 1);
    assert_eq!(m.things.len(), 1);
}

// `vertices[0].x` is parsed from the hex literal `0x10` (16), an exactly
// f64-representable integer, so strict float equality is safe here — not a
// precision-sensitive comparison.
#[allow(clippy::float_cmp)]
#[test]
fn parses_mixed_case_identifiers_and_hex_integers() {
    let text = concat!(
        "Namespace = \"doom\";\n",
        "Vertex { X = 0x10; Y = 0; }\n",
        "Linedef { v1 = 0; v2 = 0; sidefront = 0; }\n",
        "SideDef { sector = 0; }\n",
        "Sector { texturefloor = \"F\"; textureceiling = \"C\"; }\n",
        "Thing { x = 0; y = 0; type = 1; }\n",
    );
    let m = parse_udmf(text, Limits::default()).expect("mixed-case + hex map parses");
    assert_eq!(m.namespace, "doom");
    assert_eq!(m.vertices.len(), 1);
    assert_eq!(m.vertices[0].x, 16.0);
}

#[test]
fn syntax_error_reports_position() {
    let err = parse_udmf("namespace = \"doom\" vertex { }", Limits::default()).unwrap_err();
    // Missing `;` after the namespace value.
    assert!(matches!(err, UdmfParseError::Syntax { .. }));
}

#[test]
fn semantic_error_on_missing_required_field() {
    let err = parse_udmf("namespace=\"doom\"; vertex { x = 0.0; }", Limits::default()).unwrap_err();
    assert!(matches!(err, UdmfParseError::Semantic { .. }));
}

#[test]
fn depth_exceeded_on_configured_limit() {
    // With `max_depth: 0`, the very first `{` (new depth 1 > 0) trips the
    // depth guard deterministically, before the "blocks don't nest" `Syntax`
    // catch-all ever gets a chance to fire (that would require a *second*
    // `{`, which this input never reaches).
    let err = parse_udmf(
        "namespace=\"doom\"; vertex {",
        Limits::new().with_max_depth(0),
    )
    .unwrap_err();
    assert!(
        matches!(err, UdmfParseError::DepthExceeded { max_depth: 0, .. }),
        "got {err:?}"
    );
}

/// Validates the committed `fuzz_parse_udmf` seed corpus: the two valid seeds
/// must parse `Ok` with their intended element counts, and the malformed seed
/// must error. This guards against seed rot (e.g. a seed that silently stops
/// exercising the `Ok` path because it became invalid).
#[test]
fn committed_fuzz_seeds_parse_as_intended() {
    let dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/corpus/fuzz_parse_udmf");
    // The fuzz corpus lives outside this crate and is not shipped in the
    // packaged/published crate. This is a repo-only guard against seed rot, so
    // skip gracefully when the corpus directory is absent (e.g. running the
    // tests from a packaged crate tarball).
    if !dir.exists() {
        return;
    }
    let seed = |name: &str| {
        let path = dir.join(name);
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
    };

    // seed_minimal: a valid one-of-each map.
    let min = parse_udmf(&seed("seed_minimal"), Limits::default()).expect("seed_minimal parses");
    assert_eq!(min.vertices.len(), 2);
    assert_eq!(min.things.len(), 1);

    // seed_blocks: a valid two-of-each map.
    let blocks = parse_udmf(&seed("seed_blocks"), Limits::default()).expect("seed_blocks parses");
    assert_eq!(blocks.vertices.len(), 2);
    assert_eq!(blocks.linedefs.len(), 2);
    assert_eq!(blocks.sidedefs.len(), 2);
    assert_eq!(blocks.sectors.len(), 2);
    assert_eq!(blocks.things.len(), 2);

    // seed_malformed: must exercise an error path.
    assert!(parse_udmf(&seed("seed_malformed"), Limits::default()).is_err());
}

#[test]
fn unrecognized_fields_are_retained_as_extras_per_block_kind() {
    let text = r#"
        namespace = "zdoom";
        vertex { x = 1.0; y = 2.0; zfloor = 8.5; }
        linedef { v1 = 0; v2 = 0; sidefront = 0; playercross = true; comment = "hi"; }
        sidedef { sector = 0; scalex_top = 2.0; }
        sector { texturefloor = "F"; textureceiling = "C"; user_note = "mine"; }
        thing { x = 0.0; y = 0.0; type = 1; dormant = true; }
    "#;
    let map = parse_udmf(text, Limits::default()).unwrap();
    assert_eq!(map.vertices[0].extras.len(), 1);
    assert_eq!(map.vertices[0].extras[0].name, "zfloor");
    assert_eq!(map.vertices[0].extras[0].value, UdmfValue::Float(8.5));
    assert_eq!(map.linedefs[0].extras.len(), 2);
    assert_eq!(map.linedefs[0].extras[0].name, "playercross");
    assert_eq!(map.linedefs[0].extras[0].value, UdmfValue::Bool(true));
    assert_eq!(
        map.linedefs[0].extras[1].value,
        UdmfValue::Str("hi".to_owned())
    );
    assert_eq!(map.sidedefs[0].extras[0].name, "scalex_top");
    assert_eq!(map.sectors[0].extras[0].name, "user_note");
    assert_eq!(map.things[0].extras[0].name, "dormant");
}

#[test]
fn duplicate_extras_keep_first_position_and_last_value() {
    let text = r#"
        namespace = "doom";
        vertex { x = 0.0; user_a = 1; user_b = 2; user_a = 3; y = 0.0; }
    "#;
    let map = parse_udmf(text, Limits::default()).unwrap();
    let extras = &map.vertices[0].extras;
    assert_eq!(extras.len(), 2);
    assert_eq!(extras[0].name, "user_a");
    assert_eq!(extras[0].value, UdmfValue::Int(3));
    assert_eq!(extras[1].name, "user_b");
}

#[test]
fn extras_names_are_lowercased_and_hex_values_typed_as_int() {
    let text = "namespace = \"doom\";\nvertex { x = 0.0; y = 0.0; User_Flag = 0x1A; }";
    let map = parse_udmf(text, Limits::default()).unwrap();
    assert_eq!(map.vertices[0].extras[0].name, "user_flag");
    assert_eq!(map.vertices[0].extras[0].value, UdmfValue::Int(26));
}
