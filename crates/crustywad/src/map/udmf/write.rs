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
use crate::map::graph::{MapLinedef, MapSector, MapSidedef, MapThing, MapVertex};
use crate::write::WriteOptions;

/// Message for the infallible `write!`-into-`String` calls.
const INFALLIBLE: &str = "writing to a String never fails";

/// An error that prevents writing a map to UDMF text.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum UdmfWriteError {
    /// A coordinate or height was NaN or infinite and cannot be a valid UDMF
    /// float (strict mode; lenient replaces it with `0.0` and warns).
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
    /// A non-finite coordinate/height was replaced with `0.0`.
    #[error("non-finite {field} in {block} #{index} replaced with 0.0")]
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

/// Quotes and escapes `s` as a UDMF string literal (mirrors the lexer, which
/// resolves `\"` and `\\`). Backslash is escaped first so an escaped quote is
/// not double-escaped.
fn escape_udmf_string(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
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

    fn push_linedef(&mut self, l: &MapLinedef) {
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
        if l.id != -1 {
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
            if !tex.is_empty() && tex != "-" {
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
        if t.angle != 0 {
            write!(self.out, "angle = {}; ", t.angle).expect(INFALLIBLE);
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
        // Thing flags: Doom bits -> UDMF booleans (single/dm/coop default true).
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
        w.push_linedef(l);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::graph::{MapThing, Special};

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
    fn escape_handles_quote_and_backslash() {
        assert_eq!(escape_udmf_string(r#"a"b\c"#), r#""a\"b\\c""#);
    }
}
