//! UDMF (`TEXTMAP`) writing: serialize an assembled [`Map`] back to UDMF text
//! (ADR-0017 §#60). Requires the `write` feature.
//!
//! [`write_udmf`] produces the `TEXTMAP` string; [`add_udmf_map`] adds a complete
//! `MAPxx` + `TEXTMAP` + `ENDMAP` group to a [`WadBuilder`]. Fields are emitted
//! only when they differ from their UDMF spec default; `f64` coordinates narrow
//! to integer form when whole. The source of truth is the [`Map`] graph, so only
//! standardized, modeled fields are written (comments/custom fields are not
//! round-tripped — they are dropped on read).

use std::fmt::Write as _;

use crate::Strictness;
use crate::map::Map;
use crate::map::graph::{MapFormat, MapLinedef, MapSector, MapSidedef, MapThing, MapVertex};
use crate::write::{WadBuilder, WriteOptions};

/// Message for the infallible `write!`-into-`String` calls.
const INFALLIBLE: &str = "writing to a String never fails";

/// An error that prevents writing a map to UDMF text.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum UdmfWriteError {
    /// A coordinate or height was NaN or infinite and cannot be a valid UDMF
    /// float (strict mode; lenient replaces it with `0` and warns).
    #[error("non-finite {field} in {block} #{index}")]
    NonFiniteCoordinate {
        /// The block kind (e.g. `"vertex"`, `"thing"`).
        block: &'static str,
        /// The field name (e.g. `"x"`, `"height"`).
        field: &'static str,
        /// The 0-based block index.
        index: usize,
    },
    /// `map.namespace()` was `Some("")`, which is not a valid UDMF namespace
    /// (strict mode; lenient falls back to `"doom"` and warns).
    #[error("map namespace is empty")]
    EmptyNamespace,
}

/// A non-fatal issue recovered while writing a map to UDMF text in lenient mode.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum UdmfWriteWarning {
    /// A non-finite coordinate/height was replaced with `0`.
    #[error("non-finite {field} in {block} #{index} replaced with 0")]
    NonFiniteReplaced {
        /// The block kind.
        block: &'static str,
        /// The field name.
        field: &'static str,
        /// The 0-based block index.
        index: usize,
    },
    /// The map had no (or an empty) namespace; `used` was written instead.
    #[error("map had no namespace; wrote {used:?}")]
    NamespaceDefaulted {
        /// The namespace written instead (always `"doom"`).
        used: &'static str,
    },
}

/// Quotes and escapes `s` as a UDMF string literal, mirroring every escape the
/// lexer resolves (`\\`, `\"`, `\n`, `\t`) so any string round-trips. Backslash
/// is escaped first so the backslashes introduced for the other escapes are not
/// themselves doubled.
fn escape_udmf_string(s: &str) -> String {
    format!(
        "\"{}\"",
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\t', "\\t")
    )
}

/// Accumulates UDMF text and lenient-mode warnings.
struct Writer {
    out: String,
    warnings: Vec<UdmfWriteWarning>,
    strictness: Strictness,
}

impl Writer {
    fn new(strictness: Strictness) -> Self {
        Self {
            out: String::new(),
            warnings: Vec::new(),
            strictness,
        }
    }

    /// Formats a finite float; on NaN/∞ errors (strict) or returns `"0"` and
    /// records a [`UdmfWriteWarning::NonFiniteReplaced`] (lenient).
    fn fmt_float(
        &mut self,
        block: &'static str,
        field: &'static str,
        index: usize,
        value: f64,
    ) -> Result<String, UdmfWriteError> {
        if value.is_finite() {
            return Ok(format!("{value}"));
        }
        match self.strictness {
            Strictness::Strict => Err(UdmfWriteError::NonFiniteCoordinate {
                block,
                field,
                index,
            }),
            Strictness::Lenient => {
                self.warnings.push(UdmfWriteWarning::NonFiniteReplaced {
                    block,
                    field,
                    index,
                });
                Ok("0".to_string())
            }
        }
    }

    fn push_vertex(&mut self, index: usize, v: &MapVertex) -> Result<(), UdmfWriteError> {
        let x = self.fmt_float("vertex", "x", index, v.x)?;
        let y = self.fmt_float("vertex", "y", index, v.y)?;
        writeln!(self.out, "vertex {{ x = {x}; y = {y}; }}").expect(INFALLIBLE);
        Ok(())
    }

    /// UDMF linedef flag booleans by their `MapLinedef.flags` bit (reverse of the
    /// read mapping in `udmf/parse.rs`).
    const LINEDEF_FLAGS: [(u32, &str); 9] = [
        (0, "blocking"),
        (1, "blockmonsters"),
        (2, "twosided"),
        (3, "dontpegtop"),
        (4, "dontpegbottom"),
        (5, "secret"),
        (6, "blocksound"),
        (7, "dontdraw"),
        (8, "mapped"),
    ];

    fn push_linedef(&mut self, l: &MapLinedef, format: MapFormat) {
        self.out.push_str("linedef { ");
        write!(
            self.out,
            "v1 = {}; v2 = {}; sidefront = {}; ",
            l.start.0, l.end.0, l.right.0
        )
        .expect(INFALLIBLE);
        if let Some(back) = l.left {
            write!(self.out, "sideback = {}; ", back.0).expect(INFALLIBLE);
        }
        // The "no id" sentinel differs by source: UDMF's spec default is -1,
        // while Doom/Hexen maps use 0 (the graph convention). Omitting the
        // source's sentinel keeps a Doom/Hexen line from being written as a real
        // UDMF `id = 0` and preserves a genuine UDMF `id = 0`.
        let id_unset = if format == MapFormat::Udmf { -1 } else { 0 };
        if l.id != id_unset {
            write!(self.out, "id = {}; ", l.id).expect(INFALLIBLE);
        }
        if l.special.special != 0 {
            write!(self.out, "special = {}; ", l.special.special).expect(INFALLIBLE);
        }
        for (i, arg) in l.special.args.iter().enumerate() {
            if *arg != 0 {
                write!(self.out, "arg{i} = {arg}; ").expect(INFALLIBLE);
            }
        }
        for (bit, name) in Self::LINEDEF_FLAGS {
            if l.flags & (1 << bit) != 0 {
                write!(self.out, "{name} = true; ").expect(INFALLIBLE);
            }
        }
        self.out.push_str("}\n");
    }

    fn push_sidedef(&mut self, s: &MapSidedef) {
        self.out.push_str("sidedef { ");
        write!(self.out, "sector = {}; ", s.sector.0).expect(INFALLIBLE);
        if s.x_offset != 0 {
            write!(self.out, "offsetx = {}; ", s.x_offset).expect(INFALLIBLE);
        }
        if s.y_offset != 0 {
            write!(self.out, "offsety = {}; ", s.y_offset).expect(INFALLIBLE);
        }
        for (key, tex) in [
            ("texturetop", &s.upper),
            ("texturebottom", &s.lower),
            ("texturemiddle", &s.middle),
        ] {
            // Emit whenever the texture differs from the UDMF default `"-"`.
            // An explicitly-empty texture (`""`) is preserved distinct from the
            // default by the read side, so it must be emitted to round-trip.
            if tex != "-" {
                write!(self.out, "{key} = {}; ", escape_udmf_string(tex)).expect(INFALLIBLE);
            }
        }
        self.out.push_str("}\n");
    }

    fn push_sector(&mut self, s: &MapSector) {
        self.out.push_str("sector { ");
        write!(
            self.out,
            "texturefloor = {}; textureceiling = {}; ",
            escape_udmf_string(&s.floor_flat),
            escape_udmf_string(&s.ceiling_flat)
        )
        .expect(INFALLIBLE);
        if s.floor_height != 0 {
            write!(self.out, "heightfloor = {}; ", s.floor_height).expect(INFALLIBLE);
        }
        if s.ceiling_height != 0 {
            write!(self.out, "heightceiling = {}; ", s.ceiling_height).expect(INFALLIBLE);
        }
        if s.light != 160 {
            write!(self.out, "lightlevel = {}; ", s.light).expect(INFALLIBLE);
        }
        if s.special != 0 {
            write!(self.out, "special = {}; ", s.special).expect(INFALLIBLE);
        }
        if s.tag != 0 {
            write!(self.out, "id = {}; ", s.tag).expect(INFALLIBLE);
        }
        self.out.push_str("}\n");
    }

    fn push_thing(&mut self, index: usize, t: &MapThing) -> Result<(), UdmfWriteError> {
        let x = self.fmt_float("thing", "x", index, t.x)?;
        let y = self.fmt_float("thing", "y", index, t.y)?;
        self.out.push_str("thing { ");
        write!(self.out, "x = {x}; y = {y}; type = {}; ", t.type_id).expect(INFALLIBLE);
        if t.height != 0.0 {
            let h = self.fmt_float("thing", "height", index, t.height)?;
            write!(self.out, "height = {h}; ").expect(INFALLIBLE);
        }
        // Normalize to the conventional UDMF 0..=359 range. UDMF assembly re-reads
        // angles through `rem_euclid(360)`, so emitting a raw Doom angle >= 360
        // (preserved verbatim in the graph) would wrap on re-read; normalizing
        // here keeps the emitted UDMF in-range and the serialized output stable
        // across repeated write/read cycles.
        let angle = t.angle % 360;
        if angle != 0 {
            write!(self.out, "angle = {angle}; ").expect(INFALLIBLE);
        }
        if t.id != 0 {
            write!(self.out, "id = {}; ", t.id).expect(INFALLIBLE);
        }
        if t.special.special != 0 {
            write!(self.out, "special = {}; ", t.special.special).expect(INFALLIBLE);
        }
        for (i, arg) in t.special.args.iter().enumerate() {
            if *arg != 0 {
                write!(self.out, "arg{i} = {arg}; ").expect(INFALLIBLE);
            }
        }
        // Thing flags: map the Doom flag bits to the UDMF booleans this writer
        // emits — `skill1`..`skill5`, `ambush`, and `single` (`single = false`
        // for the "multiplayer only" bit, since UDMF's `single`/`dm`/`coop`
        // default to `true`). These are emitted from a Doom/Hexen-sourced map's
        // flags; UDMF-sourced maps have `flags == 0` (UDMF thing flags are not
        // modeled on the read/assembly path — normalization sets them to 0), so
        // this mapping is currently one-way within the crate.
        let f = t.flags;
        if f & 0x0001 != 0 {
            self.out.push_str("skill1 = true; skill2 = true; ");
        }
        if f & 0x0002 != 0 {
            self.out.push_str("skill3 = true; ");
        }
        if f & 0x0004 != 0 {
            self.out.push_str("skill4 = true; skill5 = true; ");
        }
        if f & 0x0008 != 0 {
            self.out.push_str("ambush = true; ");
        }
        if f & 0x0010 != 0 {
            self.out.push_str("single = false; ");
        }
        self.out.push_str("}\n");
        Ok(())
    }
}

/// Serializes an assembled map to UDMF `TEXTMAP` text.
///
/// Fields are emitted only when they differ from their UDMF spec default;
/// `f64` coordinates narrow to integer form when whole. See the module docs.
///
/// # Errors
/// - [`UdmfWriteError::EmptyNamespace`] — `map.namespace()` is `Some("")` (strict).
/// - [`UdmfWriteError::NonFiniteCoordinate`] — a coordinate/height is NaN or ∞ (strict).
pub fn write_udmf(
    map: &Map,
    opts: &WriteOptions,
) -> Result<(String, Vec<UdmfWriteWarning>), UdmfWriteError> {
    let mut w = Writer::new(opts.strictness);

    let namespace = match map.namespace() {
        Some("") => match opts.strictness {
            Strictness::Strict => return Err(UdmfWriteError::EmptyNamespace),
            Strictness::Lenient => {
                w.warnings
                    .push(UdmfWriteWarning::NamespaceDefaulted { used: "doom" });
                "doom"
            }
        },
        Some(ns) => ns,
        None => {
            if opts.strictness == Strictness::Lenient {
                w.warnings
                    .push(UdmfWriteWarning::NamespaceDefaulted { used: "doom" });
            }
            "doom"
        }
    };
    writeln!(w.out, "namespace = {};", escape_udmf_string(namespace)).expect(INFALLIBLE);

    for (i, v) in map.vertices().iter().enumerate() {
        w.push_vertex(i, v)?;
    }

    for l in map.linedefs() {
        w.push_linedef(l, map.format());
    }

    for s in map.sidedefs() {
        w.push_sidedef(s);
    }

    for s in map.sectors() {
        w.push_sector(s);
    }

    for (i, t) in map.things().iter().enumerate() {
        w.push_thing(i, t)?;
    }

    Ok((w.out, w.warnings))
}

/// Serializes `map` and adds a complete UDMF map group — the `name` marker
/// lump, a `TEXTMAP` lump, and an `ENDMAP` lump — to `builder`.
///
/// The caller invokes [`WadBuilder::build`] afterward (which returns
/// [`WriteError`](crate::WriteError)).
///
/// # Errors
/// Same as [`write_udmf`]: [`UdmfWriteError::EmptyNamespace`] and
/// [`UdmfWriteError::NonFiniteCoordinate`] (strict mode).
pub fn add_udmf_map(
    builder: &mut WadBuilder,
    name: &str,
    map: &Map,
    opts: &WriteOptions,
) -> Result<Vec<UdmfWriteWarning>, UdmfWriteError> {
    let (text, warnings) = write_udmf(map, opts)?;
    builder.add_lump(name, b"");
    builder.add_lump("TEXTMAP", text.into_bytes());
    builder.add_lump("ENDMAP", b"");
    Ok(warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WadKind;
    use crate::map::MapFormat;
    use crate::map::graph::{MapThing, MapVertex, Special};

    /// Builds a minimal `Map` with the given vertices/things (other arenas empty)
    /// for exercising `write_udmf`'s error-propagation paths directly.
    fn map_with(vertices: Vec<MapVertex>, things: Vec<MapThing>) -> Map {
        Map {
            name: "MAP01".to_string(),
            format: MapFormat::Udmf,
            namespace: Some("doom".to_string()),
            vertices,
            linedefs: Vec::new(),
            sidedefs: Vec::new(),
            sectors: Vec::new(),
            things,
            warnings: Vec::new(),
        }
    }

    fn thing(flags: u32) -> MapThing {
        MapThing {
            x: 0.0,
            y: 0.0,
            angle: 0,
            type_id: 1,
            flags,
            id: 0,
            height: 0.0,
            special: Special {
                special: 0,
                args: [0; 5],
            },
        }
    }

    #[test]
    fn thing_flag_mapping_covers_all_bits() {
        let mut w = Writer::new(Strictness::Strict);
        w.push_thing(0, &thing(0x0001 | 0x0002 | 0x0004 | 0x0008 | 0x0010))
            .unwrap();
        let out = &w.out;
        assert!(out.contains("skill1 = true; skill2 = true; "));
        assert!(out.contains("skill3 = true; "));
        assert!(out.contains("skill4 = true; skill5 = true; "));
        assert!(out.contains("ambush = true; "));
        assert!(out.contains("single = false; "));
    }

    #[test]
    fn no_thing_flags_emitted_when_zero() {
        let mut w = Writer::new(Strictness::Strict);
        w.push_thing(0, &thing(0)).unwrap();
        assert!(!w.out.contains("skill"));
        assert!(!w.out.contains("ambush"));
        assert!(!w.out.contains("single"));
    }

    #[test]
    fn non_finite_coordinate_errors_in_strict() {
        let mut w = Writer::new(Strictness::Strict);
        let mut t = thing(0);
        t.x = f64::NAN;
        let err = w.push_thing(0, &t).unwrap_err();
        assert_eq!(
            err,
            UdmfWriteError::NonFiniteCoordinate {
                block: "thing",
                field: "x",
                index: 0
            }
        );
    }

    #[test]
    fn non_finite_coordinate_replaced_in_lenient() {
        let mut w = Writer::new(Strictness::Lenient);
        let mut t = thing(0);
        t.height = f64::INFINITY;
        t.x = 1.0;
        t.y = 2.0;
        w.push_thing(0, &t).unwrap();
        assert!(w.out.contains("height = 0; "));
        assert_eq!(
            w.warnings,
            vec![UdmfWriteWarning::NonFiniteReplaced {
                block: "thing",
                field: "height",
                index: 0
            }]
        );
    }

    #[test]
    fn thing_angle_is_normalized_modulo_360() {
        // A raw angle >= 360 is normalized to the conventional 0..360 range.
        let mut w = Writer::new(Strictness::Strict);
        let mut t = thing(0);
        t.angle = 450;
        w.push_thing(0, &t).unwrap();
        assert!(w.out.contains("angle = 90; "), "{}", w.out);

        // 360 normalizes to 0 (the default) and is omitted.
        let mut w2 = Writer::new(Strictness::Strict);
        let mut t2 = thing(0);
        t2.angle = 360;
        w2.push_thing(0, &t2).unwrap();
        assert!(!w2.out.contains("angle"), "{}", w2.out);
    }

    #[test]
    fn escape_handles_quote_and_backslash() {
        assert_eq!(escape_udmf_string(r#"a"b\c"#), r#""a\"b\\c""#);
    }

    #[test]
    fn escape_handles_newline_and_tab() {
        // Every escape the lexer resolves must be produced so any string
        // round-trips: backslash, quote, newline, and tab.
        assert_eq!(escape_udmf_string("a\nb\tc"), r#""a\nb\tc""#);
        // A backslash preceding an escaped char is not double-escaped.
        assert_eq!(escape_udmf_string("\\\n"), r#""\\\n""#);
    }

    #[test]
    fn write_udmf_propagates_non_finite_vertex_x_strict() {
        let map = map_with(
            vec![MapVertex {
                x: f64::NAN,
                y: 0.0,
            }],
            vec![],
        );
        let err = write_udmf(&map, &WriteOptions::strict()).unwrap_err();
        assert_eq!(
            err,
            UdmfWriteError::NonFiniteCoordinate {
                block: "vertex",
                field: "x",
                index: 0
            }
        );
    }

    #[test]
    fn write_udmf_propagates_non_finite_vertex_y_strict() {
        let map = map_with(
            vec![MapVertex {
                x: 0.0,
                y: f64::INFINITY,
            }],
            vec![],
        );
        let err = write_udmf(&map, &WriteOptions::strict()).unwrap_err();
        assert_eq!(
            err,
            UdmfWriteError::NonFiniteCoordinate {
                block: "vertex",
                field: "y",
                index: 0
            }
        );
    }

    #[test]
    fn write_udmf_propagates_non_finite_thing_y_strict() {
        let mut t = thing(0);
        t.y = f64::NAN;
        let map = map_with(vec![MapVertex { x: 0.0, y: 0.0 }], vec![t]);
        let err = write_udmf(&map, &WriteOptions::strict()).unwrap_err();
        assert_eq!(
            err,
            UdmfWriteError::NonFiniteCoordinate {
                block: "thing",
                field: "y",
                index: 0
            }
        );
    }

    #[test]
    fn write_udmf_propagates_non_finite_thing_height_strict() {
        let mut t = thing(0);
        t.height = f64::NAN;
        let map = map_with(vec![MapVertex { x: 0.0, y: 0.0 }], vec![t]);
        let err = write_udmf(&map, &WriteOptions::strict()).unwrap_err();
        assert_eq!(
            err,
            UdmfWriteError::NonFiniteCoordinate {
                block: "thing",
                field: "height",
                index: 0
            }
        );
    }

    #[test]
    fn add_udmf_map_propagates_non_finite_error() {
        let map = map_with(
            vec![MapVertex {
                x: f64::NAN,
                y: 0.0,
            }],
            vec![],
        );
        let mut builder = WadBuilder::new(WadKind::Pwad);
        let err = add_udmf_map(&mut builder, "MAP01", &map, &WriteOptions::strict()).unwrap_err();
        assert_eq!(
            err,
            UdmfWriteError::NonFiniteCoordinate {
                block: "vertex",
                field: "x",
                index: 0
            }
        );
    }
}
