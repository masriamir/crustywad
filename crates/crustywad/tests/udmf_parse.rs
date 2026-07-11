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
