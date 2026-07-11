//! Integration tests for the public UDMF text-map parser.

use crustywad::Limits;
use crustywad::map::udmf::{UdmfParseError, parse_udmf};

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
    let text = format!("namespace=\"doom\"; x {}", "{".repeat(64));
    let err = parse_udmf(&text, Limits { max_depth: 4 }).unwrap_err();
    assert!(matches!(
        err,
        UdmfParseError::Syntax { .. } | UdmfParseError::DepthExceeded { .. }
    ));
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
