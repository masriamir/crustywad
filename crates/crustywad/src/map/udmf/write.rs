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
use crate::map::graph::MapVertex;
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

    Ok((w.out, w.warnings))
}
