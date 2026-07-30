//! UDMF (`TEXTMAP`) writing: serialize an assembled [`Map`] back to UDMF text
//! (ADR-0017 §#60). Requires the `write` feature.
//!
//! [`write_udmf`] produces the `TEXTMAP` string; [`add_udmf_map`] adds a complete
//! `MAPxx` + `TEXTMAP` + `ENDMAP` group to a [`WadBuilder`]. Fields are emitted
//! only when they differ from their UDMF spec default; `f64` coordinates narrow
//! to integer form when whole. The source of truth is the [`Map`] graph, so only
//! standardized, modeled fields are written; for lossless round-trip of `comment`
//! fields, `user_*` fields, and port extensions, write from the parsed
//! [`UdmfMap`] via [`UdmfMap::to_textmap`] instead (ADR-0027).

use std::fmt::Write as _;

use crate::Strictness;
use crate::map::Map;
use crate::map::graph::{
    MapFormat, MapLinedef, MapSector, MapSidedef, MapThing, MapVertex, TextureRef, linedef_id_unset,
};
use crate::write::{WadBuilder, WriteOptions};

use super::model::{
    UdmfAssignment, UdmfLinedef, UdmfMap, UdmfSector, UdmfSidedef, UdmfThing, UdmfUnknownBlock,
    UdmfValue, UdmfVertex,
};

/// Message for the infallible `write!`-into-`String` calls.
const INFALLIBLE: &str = "writing to a String never fails";

/// UDMF linedef flag booleans by their linedef-`flags` bit (reverse of the read
/// mapping in `udmf/parse.rs`). Shared by both writers: the assembled-[`Map`]
/// path ([`Writer::push_linedef`]) and the lossless [`UdmfMap::to_textmap`]
/// path, whose `flags` bit layouts are identical (ADR-0027).
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
    /// A linedef has no front sidedef (`MapLinedef::right == None` — whether
    /// from the binary `0xffff` sentinel (ADR-0020) or a lenient-mode recovery
    /// of a dangling UDMF `sidefront`), which UDMF cannot represent — the spec
    /// gives `sidefront` no valid default (strict mode; lenient writes
    /// `sidefront = -1` and warns).
    #[error("linedef #{index} has no front sidedef, which UDMF cannot represent")]
    NoFrontSide {
        /// The 0-based linedef index.
        index: usize,
    },
    /// A [`TextureRef::Index`] reached the writer; assembly resolves a Doom
    /// 64 texture hash to a name only when the outer WAD carries a
    /// `T_START..T_END` section (ADR-0022 §4), so a leftover `Index` has no
    /// name this writer can invent (both strictness modes; ADR-0021 §5).
    #[error("unresolvable texture index for {field} in {block} #{index}")]
    UnresolvedTextureIndex {
        /// The block kind (`"sidedef"` or `"sector"`).
        block: &'static str,
        /// The field name (e.g. `"texturetop"`, `"texturefloor"`).
        field: &'static str,
        /// The 0-based block index.
        index: usize,
    },
    /// The map's source format has no writer policy at all — a defensive
    /// fallback for a future [`MapFormat`] variant this writer has never
    /// heard of (returned in **both** strictness modes: there is no policy
    /// to even attempt recovery under). A [`MapFormat::Doom64`] map is
    /// **not** this variant — it writes under the tier-3 colored-lighting
    /// policy instead (see
    /// [`UnrepresentableField`][Self::UnrepresentableField] /
    /// [`UdmfWriteWarning::ColoredLightingDropped`]; ADR-0021 §5 amendment 3).
    #[error("cannot write a {format:?}-sourced map")]
    UnsupportedSourceFormat {
        /// The assembled map's source format.
        format: MapFormat,
    },
    /// A field carried data this writer's target format has no slot for
    /// (strict; lenient drops and warns). Currently produced only for a
    /// Doom 64 map's colored lighting (`block: "sector", field: "colors"`;
    /// ADR-0021 §5 amendment 3), mirroring
    /// [`DoomWriteError::UnrepresentableField`](crate::map::doom::DoomWriteError::UnrepresentableField).
    #[error("{block} #{index} has a {field} value, which UDMF cannot represent")]
    UnrepresentableField {
        /// The block kind.
        block: &'static str,
        /// The field name (e.g. `"colors"`).
        field: &'static str,
        /// The 0-based element index.
        index: usize,
    },
}

impl UdmfWriteError {
    /// Whether re-running the write with [`WriteOptions::lenient`] recovers
    /// from this error, turning it into a [`UdmfWriteWarning`]-carrying
    /// success.
    ///
    /// Returns `false` for the errors produced identically in both strictness
    /// modes — [`UnresolvedTextureIndex`][Self::UnresolvedTextureIndex] and
    /// [`UnsupportedSourceFormat`][Self::UnsupportedSourceFormat] — where
    /// suggesting lenient mode would mislead.
    #[must_use]
    pub fn is_lenient_recoverable(&self) -> bool {
        match self {
            Self::NonFiniteCoordinate { .. }
            | Self::EmptyNamespace
            | Self::NoFrontSide { .. }
            | Self::UnrepresentableField { .. } => true,
            Self::UnresolvedTextureIndex { .. } | Self::UnsupportedSourceFormat { .. } => false,
        }
    }
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
        /// The namespace written instead: `"hexen"` for a [`MapFormat::Hexen`]
        /// map, `"doom"` otherwise.
        used: &'static str,
    },
    /// A linedef with no front sidedef (unrepresentable in UDMF — the spec
    /// gives `sidefront` no valid default) was written as `sidefront = -1`,
    /// which ports tolerate at load time (ADR-0020).
    #[error("linedef #{index} has no front sidedef; wrote sidefront = -1")]
    NoFrontSideDefaulted {
        /// The 0-based linedef index.
        index: usize,
    },
    /// A Doom 64 map's colored lighting (sector color references and the
    /// engine-model lights table) has no UDMF slot and was dropped. Emitted
    /// at most once per map (lenient mode only — strict mode instead returns
    /// [`UdmfWriteError::UnrepresentableField`] naming `block: "sector",
    /// field: "colors"`; ADR-0021 §5 amendment 3).
    #[error(
        "the map's Doom 64 colored lighting (sector color references and lights table) has no UDMF slot and was dropped"
    )]
    ColoredLightingDropped,
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

/// Resolves a [`TextureRef`] to a name, or fails: assembly already resolved
/// every Doom 64 texture hash it could (ADR-0022 §4), so a leftover `Index`
/// reaching this writer has no name to invent — not a recoverable defect
/// (ADR-0021 §5) — it errors in **both** strictness modes.
fn texture_name<'a>(
    block: &'static str,
    field: &'static str,
    index: usize,
    tex: &'a TextureRef,
) -> Result<&'a str, UdmfWriteError> {
    tex.as_name().ok_or(UdmfWriteError::UnresolvedTextureIndex {
        block,
        field,
        index,
    })
}

/// Formats a finite `f64` so the text re-lexes as a single numeric token and
/// re-parses to an equal value. Whole values within `i64` emit as bare
/// integer digits (`i64 → f64` is exact for any integer obtained from a
/// whole in-range `f64`); everything else emits Rust's shortest-round-trip
/// `{:?}` form, which always carries a `.` or exponent — both accepted by
/// the UDMF lexer — and re-parses to the identical `f64` by construction.
/// A bare-digits emission of a huge whole value (e.g. `1e300` under `{}`)
/// would re-lex as an integer and overflow `i64` (ADR-0027 §2).
fn fmt_roundtrip_float(value: f64) -> String {
    const I64_MIN_F: f64 = -9_223_372_036_854_775_808.0; // -2^63, exact
    const I64_MAX_BOUND: f64 = 9_223_372_036_854_775_808.0; // 2^63, exact
    if value.fract() == 0.0 && (I64_MIN_F..I64_MAX_BOUND).contains(&value) {
        // Guarded by the range check above.
        #[allow(clippy::cast_possible_truncation)]
        return format!("{}", value as i64);
    }
    format!("{value:?}")
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
            return Ok(fmt_roundtrip_float(value));
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

    fn push_linedef(
        &mut self,
        index: usize,
        l: &MapLinedef,
        format: MapFormat,
    ) -> Result<(), UdmfWriteError> {
        // UDMF gives `sidefront` no valid default, so a frontless line
        // (`right: None`, whatever its source — see `MapLinedef::right`;
        // ADR-0020) is unrepresentable: strict errors, lenient writes the
        // port-tolerated `-1` and warns.
        self.out.push_str("linedef { ");
        write!(self.out, "v1 = {}; v2 = {}; ", l.start.0, l.end.0).expect(INFALLIBLE);
        match l.right {
            // Write the arena index directly — no numeric narrowing (a
            // hand-constructed `Map`'s public fields can never panic here)
            // and no per-linedef allocation.
            Some(r) => write!(self.out, "sidefront = {}; ", r.0).expect(INFALLIBLE),
            None => match self.strictness {
                // The partially written buffer is discarded with the error.
                Strictness::Strict => return Err(UdmfWriteError::NoFrontSide { index }),
                Strictness::Lenient => {
                    self.warnings
                        .push(UdmfWriteWarning::NoFrontSideDefaulted { index });
                    self.out.push_str("sidefront = -1; ");
                }
            },
        }
        if let Some(back) = l.left {
            write!(self.out, "sideback = {}; ", back.0).expect(INFALLIBLE);
        }
        // Omitting the source's "no id" sentinel (`linedef_id_unset`) keeps a
        // Doom/Hexen line from being written as a real UDMF `id = 0`, and
        // preserves a genuine UDMF `id = 0`. The rule is defined once, in
        // `map::graph`, and shared with `map::doom::write`.
        if l.id != linedef_id_unset(format) {
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
        for (bit, name) in LINEDEF_FLAGS {
            if l.flags & (1 << bit) != 0 {
                write!(self.out, "{name} = true; ").expect(INFALLIBLE);
            }
        }
        self.out.push_str("}\n");
        Ok(())
    }

    fn push_sidedef(&mut self, index: usize, s: &MapSidedef) -> Result<(), UdmfWriteError> {
        self.out.push_str("sidedef { ");
        write!(self.out, "sector = {}; ", s.sector.0).expect(INFALLIBLE);
        if s.x_offset != 0 {
            write!(self.out, "offsetx = {}; ", s.x_offset).expect(INFALLIBLE);
        }
        if s.y_offset != 0 {
            write!(self.out, "offsety = {}; ", s.y_offset).expect(INFALLIBLE);
        }
        for (key, field, tex) in [
            ("texturetop", "texturetop", &s.upper),
            ("texturebottom", "texturebottom", &s.lower),
            ("texturemiddle", "texturemiddle", &s.middle),
        ] {
            let name = texture_name("sidedef", field, index, tex)?;
            // Emit whenever the texture differs from the UDMF default `"-"`.
            // An explicitly-empty texture (`""`) is preserved distinct from the
            // default by the read side, so it must be emitted to round-trip.
            if name != "-" {
                write!(self.out, "{key} = {}; ", escape_udmf_string(name)).expect(INFALLIBLE);
            }
        }
        self.out.push_str("}\n");
        Ok(())
    }

    fn push_sector(&mut self, index: usize, s: &MapSector) -> Result<(), UdmfWriteError> {
        self.out.push_str("sector { ");
        write!(
            self.out,
            "texturefloor = {}; textureceiling = {}; ",
            escape_udmf_string(texture_name(
                "sector",
                "texturefloor",
                index,
                &s.floor_flat
            )?),
            escape_udmf_string(texture_name(
                "sector",
                "textureceiling",
                index,
                &s.ceiling_flat
            )?)
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
        Ok(())
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
        // Thing flags: the exact inverse of the read-side packing (ADR-0019) —
        // bit 0 -> skill1+skill2, bit 1 -> skill3, bit 2 -> skill4+skill5,
        // bit 3 -> ambush, bit 7 -> friend.
        //
        // Every UDMF flag defaults to `false` (spec: "All flags default to
        // false"), so `false` is the value that gets omitted. Doom's game-mode
        // bits are negative ("not in X") while UDMF's are positive, so the
        // *positive* key is emitted when Doom's bit is CLEAR: bit 4 clear ->
        // `single = true`, bit 5 clear -> `dm = true`, bit 6 clear ->
        // `coop = true`. Omitting them when the bit is set correctly means
        // "false" — a spec-conformant reader then keeps the thing out of that
        // mode. Emitting nothing at all (the old behavior, which assumed a
        // default of `true`) makes every converted thing spawn nowhere.
        //
        // Bits above 7 have no UDMF boolean and are not emitted.
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
        if f & 0x0010 == 0 {
            self.out.push_str("single = true; ");
        }
        if f & 0x0020 == 0 {
            self.out.push_str("dm = true; ");
        }
        if f & 0x0040 == 0 {
            self.out.push_str("coop = true; ");
        }
        if f & 0x0080 != 0 {
            self.out.push_str("friend = true; ");
        }
        self.out.push_str("}\n");
        Ok(())
    }
}

/// Formats a retained [`UdmfValue`] as UDMF source text.
///
/// A [`UdmfValue::Float`] is emitted via Rust's shortest-round-trip `{:?}`
/// form, which always carries a `.` or exponent — so it re-lexes as a
/// [`Token::Float`][super::lex::Token::Float] and re-parses to the identical
/// [`UdmfValue::Float`]. Unlike the typed coordinate fields (which pass through
/// [`fmt_roundtrip_float`] and may narrow a whole value to bare integer
/// digits, harmless there because the typed-float readers widen an integer
/// literal), a retained extra must preserve its exact `UdmfValue` variant: a
/// whole float emitted as bare digits would re-lex as an integer and violate
/// the ADR-0027 round-trip (`Float(8.0)` vs `Int(8)`).
fn fmt_value(value: &UdmfValue) -> String {
    match value {
        UdmfValue::Bool(b) => b.to_string(),
        UdmfValue::Int(i) => i.to_string(),
        UdmfValue::Float(f) => format!("{f:?}"),
        UdmfValue::Str(s) => escape_udmf_string(s),
    }
}

/// Appends `name = value; ` pairs for a retained extras list, in retained
/// order, with no default elision (every extra is emitted verbatim; ADR-0027).
fn push_extras(out: &mut String, extras: &[UdmfAssignment]) {
    for a in extras {
        write!(out, "{} = {}; ", a.name, fmt_value(&a.value)).expect(INFALLIBLE);
    }
}

/// Appends one canonical `vertex { … }` line. Both coordinates are required
/// and always emitted via [`fmt_roundtrip_float`], then the retained extras.
fn push_udmf_vertex(out: &mut String, v: &UdmfVertex) {
    write!(
        out,
        "vertex {{ x = {}; y = {}; ",
        fmt_roundtrip_float(v.x),
        fmt_roundtrip_float(v.y)
    )
    .expect(INFALLIBLE);
    push_extras(out, &v.extras);
    out.push_str("}\n");
}

/// Appends one canonical `linedef { … }` line: the three required refs, then
/// each non-default typed field, then the nine [`LINEDEF_FLAGS`] booleans that
/// are set, then the retained extras.
fn push_udmf_linedef(out: &mut String, l: &UdmfLinedef) {
    write!(
        out,
        "linedef {{ v1 = {}; v2 = {}; sidefront = {}; ",
        l.v1, l.v2, l.sidefront
    )
    .expect(INFALLIBLE);
    if let Some(back) = l.sideback {
        write!(out, "sideback = {back}; ").expect(INFALLIBLE);
    }
    // `UdmfLinedef.id`'s UDMF spec default is a plain -1 (unlike the
    // assembled-`Map` path, which keys off `linedef_id_unset(format)`).
    if l.id != -1 {
        write!(out, "id = {}; ", l.id).expect(INFALLIBLE);
    }
    if l.special != 0 {
        write!(out, "special = {}; ", l.special).expect(INFALLIBLE);
    }
    for (i, arg) in l.args.iter().enumerate() {
        if *arg != 0 {
            write!(out, "arg{i} = {arg}; ").expect(INFALLIBLE);
        }
    }
    for (bit, name) in LINEDEF_FLAGS {
        if l.flags & (1 << bit) != 0 {
            write!(out, "{name} = true; ").expect(INFALLIBLE);
        }
    }
    push_extras(out, &l.extras);
    out.push_str("}\n");
}

/// Appends one canonical `sidedef { … }` line: the required `sector`, then the
/// non-default offsets and any texture differing from the UDMF `"-"` default,
/// then the retained extras.
fn push_udmf_sidedef(out: &mut String, s: &UdmfSidedef) {
    write!(out, "sidedef {{ sector = {}; ", s.sector).expect(INFALLIBLE);
    if s.offsetx != 0 {
        write!(out, "offsetx = {}; ", s.offsetx).expect(INFALLIBLE);
    }
    if s.offsety != 0 {
        write!(out, "offsety = {}; ", s.offsety).expect(INFALLIBLE);
    }
    for (key, tex) in [
        ("texturetop", &s.texturetop),
        ("texturebottom", &s.texturebottom),
        ("texturemiddle", &s.texturemiddle),
    ] {
        if tex != "-" {
            write!(out, "{key} = {}; ", escape_udmf_string(tex)).expect(INFALLIBLE);
        }
    }
    push_extras(out, &s.extras);
    out.push_str("}\n");
}

/// Appends one canonical `sector { … }` line: the two required flat textures,
/// then each non-default typed field (`lightlevel` elided against the UDMF
/// default 160), then the retained extras.
fn push_udmf_sector(out: &mut String, s: &UdmfSector) {
    write!(
        out,
        "sector {{ texturefloor = {}; textureceiling = {}; ",
        escape_udmf_string(&s.texturefloor),
        escape_udmf_string(&s.textureceiling)
    )
    .expect(INFALLIBLE);
    if s.heightfloor != 0 {
        write!(out, "heightfloor = {}; ", s.heightfloor).expect(INFALLIBLE);
    }
    if s.heightceiling != 0 {
        write!(out, "heightceiling = {}; ", s.heightceiling).expect(INFALLIBLE);
    }
    if s.lightlevel != 160 {
        write!(out, "lightlevel = {}; ", s.lightlevel).expect(INFALLIBLE);
    }
    if s.special != 0 {
        write!(out, "special = {}; ", s.special).expect(INFALLIBLE);
    }
    if s.id != 0 {
        write!(out, "id = {}; ", s.id).expect(INFALLIBLE);
    }
    push_extras(out, &s.extras);
    out.push_str("}\n");
}

/// Appends one canonical `thing { … }` line: the required `x`/`y`/`type`, then
/// each non-default typed field, then the retained extras. `flags` is **never**
/// emitted — it is a derived projection of the dual-stored skill/multiplayer
/// booleans, which round-trip through `extras` and re-derive `flags` on reparse
/// (ADR-0027).
fn push_udmf_thing(out: &mut String, t: &UdmfThing) {
    write!(
        out,
        "thing {{ x = {}; y = {}; type = {}; ",
        fmt_roundtrip_float(t.x),
        fmt_roundtrip_float(t.y),
        t.type_id
    )
    .expect(INFALLIBLE);
    if t.height != 0.0 {
        write!(out, "height = {}; ", fmt_roundtrip_float(t.height)).expect(INFALLIBLE);
    }
    if t.angle != 0 {
        write!(out, "angle = {}; ", t.angle).expect(INFALLIBLE);
    }
    if t.id != 0 {
        write!(out, "id = {}; ", t.id).expect(INFALLIBLE);
    }
    if t.special != 0 {
        write!(out, "special = {}; ", t.special).expect(INFALLIBLE);
    }
    for (i, arg) in t.args.iter().enumerate() {
        if *arg != 0 {
            write!(out, "arg{i} = {arg}; ").expect(INFALLIBLE);
        }
    }
    push_extras(out, &t.extras);
    out.push_str("}\n");
}

/// Appends one retained unrecognized block, `<name> { <fields> }`, verbatim in
/// declaration order (ADR-0027).
fn push_udmf_unknown_block(out: &mut String, b: &UdmfUnknownBlock) {
    write!(out, "{} {{ ", b.name).expect(INFALLIBLE);
    push_extras(out, &b.fields);
    out.push_str("}\n");
}

impl UdmfMap {
    /// Serializes this document to canonical UDMF `TEXTMAP` text.
    ///
    /// Infallible by construction: every value a parsed `UdmfMap` can hold is
    /// representable (the lexer rejects non-finite floats, and string escaping
    /// covers every character). Canonical form — blocks grouped by kind,
    /// spec-default typed fields elided, extras always emitted — with the
    /// ADR-0027 semantic round-trip guarantee: re-parsing the output yields a
    /// value equal to `self`.
    ///
    /// Emission order: `namespace`, global extras, vertices, linedefs,
    /// sidedefs, sectors, things, then unknown blocks. Within each element the
    /// standardized fields are written at their non-default values first
    /// (required fields always), then that element's retained extras. A
    /// thing's `flags` is never emitted — it is a derived projection of the
    /// dual-stored skill/multiplayer booleans, which round-trip through the
    /// element's extras instead.
    #[must_use]
    pub fn to_textmap(&self) -> String {
        let mut out = String::new();
        writeln!(out, "namespace = {};", escape_udmf_string(&self.namespace)).expect(INFALLIBLE);
        for a in &self.global_extras {
            writeln!(out, "{} = {};", a.name, fmt_value(&a.value)).expect(INFALLIBLE);
        }
        for v in &self.vertices {
            push_udmf_vertex(&mut out, v);
        }
        for l in &self.linedefs {
            push_udmf_linedef(&mut out, l);
        }
        for s in &self.sidedefs {
            push_udmf_sidedef(&mut out, s);
        }
        for s in &self.sectors {
            push_udmf_sector(&mut out, s);
        }
        for t in &self.things {
            push_udmf_thing(&mut out, t);
        }
        for b in &self.unknown_blocks {
            push_udmf_unknown_block(&mut out, b);
        }
        out
    }
}

/// Serializes an assembled map to UDMF `TEXTMAP` text.
///
/// Fields are emitted only when they differ from their UDMF spec default;
/// `f64` coordinates narrow to integer form when whole. See the module docs.
///
/// # Errors
/// - [`UdmfWriteError::UnrepresentableField`] — includes `map.format()`
///   being [`MapFormat::Doom64`] in strict mode: colored lighting (sector
///   color references and the engine-model lights table) has no UDMF slot
///   (`block: "sector", field: "colors"`, `index: 0`; ADR-0021 §5 amendment
///   3). Lenient mode instead drops it and returns
///   [`UdmfWriteWarning::ColoredLightingDropped`].
/// - [`UdmfWriteError::UnsupportedSourceFormat`] — an unrecognized future
///   [`MapFormat`] variant this writer has no policy for (returned in
///   **both** strictness modes).
/// - [`UdmfWriteError::EmptyNamespace`] — `map.namespace()` is `Some("")` (strict).
/// - [`UdmfWriteError::NonFiniteCoordinate`] — a coordinate/height is NaN or ∞ (strict).
/// - [`UdmfWriteError::NoFrontSide`] — a linedef has no front sidedef, which
///   UDMF cannot represent (strict).
/// - [`UdmfWriteError::UnresolvedTextureIndex`] — a sidedef/sector carries a
///   [`TextureRef::Index`] left unresolved by assembly (returned in **both**
///   strictness modes; ADR-0021 §5, ADR-0022 §4).
pub fn write_udmf(
    map: &Map,
    opts: &WriteOptions,
) -> Result<(String, Vec<UdmfWriteWarning>), UdmfWriteError> {
    match map.format() {
        MapFormat::Doom | MapFormat::Hexen | MapFormat::Udmf => {}
        MapFormat::Doom64 => {
            // Colored lighting (sector color refs + the engine-model lights
            // table) has no slot in this target — tier-3 data loss
            // (ADR-0019). The texture half of the old blanket gate is gone:
            // refs resolve to names at assembly when a texture section is
            // present (ADR-0022 §4); any leftover `Index` still hits
            // `UnresolvedTextureIndex` below, in both modes.
            if opts.strictness == Strictness::Strict {
                return Err(UdmfWriteError::UnrepresentableField {
                    block: "sector",
                    field: "colors",
                    index: 0,
                });
            }
        }
        // A future `MapFormat` variant this writer has never heard of:
        // reject in both modes rather than silently mis-writing. Every
        // variant known to this crate is already matched above, so this arm
        // is unreachable today — it exists purely as a defensive fallback
        // for when a new variant is added.
        #[allow(unreachable_patterns)]
        format => return Err(UdmfWriteError::UnsupportedSourceFormat { format }),
    }

    let mut w = Writer::new(opts.strictness);
    if map.format() == MapFormat::Doom64 {
        w.warnings.push(UdmfWriteWarning::ColoredLightingDropped);
    }

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
            // A binary-format map has no namespace declaration of its own, so
            // derive the reserved UDMF namespace matching its source format
            // (ADR-0019). `MapFormat` is #[non_exhaustive]; anything without a
            // reserved namespace falls back to "doom".
            let derived = match map.format() {
                MapFormat::Hexen => "hexen",
                _ => "doom",
            };
            if opts.strictness == Strictness::Lenient {
                w.warnings
                    .push(UdmfWriteWarning::NamespaceDefaulted { used: derived });
            }
            derived
        }
    };
    writeln!(w.out, "namespace = {};", escape_udmf_string(namespace)).expect(INFALLIBLE);

    for (i, v) in map.vertices().iter().enumerate() {
        w.push_vertex(i, v)?;
    }

    for (i, l) in map.linedefs().iter().enumerate() {
        w.push_linedef(i, l, map.format())?;
    }

    for (i, s) in map.sidedefs().iter().enumerate() {
        w.push_sidedef(i, s)?;
    }

    for (i, s) in map.sectors().iter().enumerate() {
        w.push_sector(i, s)?;
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
/// Same as [`write_udmf`]: [`UdmfWriteError::EmptyNamespace`],
/// [`UdmfWriteError::NonFiniteCoordinate`],
/// [`UdmfWriteError::NoFrontSide`], and
/// [`UdmfWriteError::UnrepresentableField`] (strict mode); and
/// [`UdmfWriteError::UnresolvedTextureIndex`] and
/// [`UdmfWriteError::UnsupportedSourceFormat`] (both modes).
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

/// Adds a complete UDMF map group — the `name` marker lump, a `TEXTMAP`
/// lump holding [`UdmfMap::to_textmap`]'s output, and an `ENDMAP` lump — to
/// `builder`. Infallible, unlike the [`Map`]-sourced [`add_udmf_map`]: a
/// parsed [`UdmfMap`] contains only representable values (ADR-0027).
///
/// The caller invokes [`WadBuilder::build`] afterward (which returns
/// [`WriteError`](crate::WriteError)).
pub fn add_udmf_textmap(builder: &mut WadBuilder, name: &str, map: &UdmfMap) {
    builder.add_lump(name, b"");
    builder.add_lump("TEXTMAP", map.to_textmap().into_bytes());
    builder.add_lump("ENDMAP", b"");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WadKind;
    use crate::map::MapFormat;
    use crate::map::graph::{
        MapSector, MapSidedef, MapThing, MapVertex, SectorIdx, Special, TextureRef,
    };

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
            lights: Vec::new(),
            segs: Vec::new(),
            subsectors: Vec::new(),
            nodes: Vec::new(),
            gl_vertices: Vec::new(),
            gl_segs: Vec::new(),
            gl_subsectors: Vec::new(),
            gl_nodes: Vec::new(),
            leafs: Vec::new(),
            macros: Vec::new(),
            reject: None,
            blockmap: None,
            warnings: Vec::new(),
        }
    }

    /// Builds a minimal `Map` with a single sidedef/sector pair (other arenas
    /// empty), for exercising the texture-resolving write paths.
    fn tiny_map() -> Map {
        Map {
            name: "MAP01".to_string(),
            format: MapFormat::Udmf,
            namespace: Some("doom".to_string()),
            vertices: Vec::new(),
            linedefs: Vec::new(),
            sidedefs: vec![MapSidedef {
                sector: SectorIdx(0),
                x_offset: 0,
                y_offset: 0,
                upper: TextureRef::Name("-".into()),
                lower: TextureRef::Name("-".into()),
                middle: TextureRef::Name("WALL".into()),
            }],
            sectors: vec![MapSector {
                floor_height: 0,
                ceiling_height: 128,
                floor_flat: TextureRef::Name("FLOOR".into()),
                ceiling_flat: TextureRef::Name("CEIL".into()),
                light: 160,
                special: 0,
                tag: 0,
                colors: None,
                flags: 0,
            }],
            things: Vec::new(),
            lights: Vec::new(),
            segs: Vec::new(),
            subsectors: Vec::new(),
            nodes: Vec::new(),
            gl_vertices: Vec::new(),
            gl_segs: Vec::new(),
            gl_subsectors: Vec::new(),
            gl_nodes: Vec::new(),
            leafs: Vec::new(),
            macros: Vec::new(),
            reject: None,
            blockmap: None,
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
        // Bit 4 ("not in single player") is set, so the positive `single` key
        // is omitted — and omission means `false`, the UDMF default.
        assert!(!out.contains("single"), "{out}");
        // Bits 5/6 are clear, so the thing *is* in dm and co-op, which the
        // spec's false-by-default flags require us to say explicitly.
        assert!(out.contains("dm = true; "), "{out}");
        assert!(out.contains("coop = true; "), "{out}");
    }

    /// The headline conversion case: an ordinary Doom thing (`0x07` — all
    /// skills, all game modes) must name every game mode it appears in.
    /// Emitting nothing here would make the thing spawn in no mode at all,
    /// because the UDMF spec defaults every flag to `false`.
    #[test]
    fn ordinary_doom_thing_names_every_game_mode() {
        let mut w = Writer::new(Strictness::Strict);
        w.push_thing(0, &thing(0x0007)).unwrap();
        let out = &w.out;
        assert!(out.contains("single = true; "), "{out}");
        assert!(out.contains("dm = true; "), "{out}");
        assert!(out.contains("coop = true; "), "{out}");
        assert!(out.contains("skill1 = true; skill2 = true; "), "{out}");
        assert!(out.contains("skill3 = true; "), "{out}");
        assert!(out.contains("skill4 = true; skill5 = true; "), "{out}");
    }

    #[test]
    fn zero_flags_emit_the_game_modes_but_no_skills() {
        let mut w = Writer::new(Strictness::Strict);
        w.push_thing(0, &thing(0)).unwrap();
        assert!(!w.out.contains("skill"), "{}", w.out);
        assert!(!w.out.contains("ambush"), "{}", w.out);
        assert!(!w.out.contains("friend"), "{}", w.out);
        // No Doom "not in X" bit is set, so the thing appears in every game
        // mode — which UDMF states positively.
        assert!(w.out.contains("single = true; "), "{}", w.out);
        assert!(w.out.contains("dm = true; "), "{}", w.out);
        assert!(w.out.contains("coop = true; "), "{}", w.out);
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
    fn hexen_map_gets_hexen_namespace() {
        let mut map = map_with(vec![MapVertex { x: 0.0, y: 0.0 }], vec![]);
        map.format = MapFormat::Hexen;
        map.namespace = None;
        let (text, warnings) = write_udmf(&map, &WriteOptions::lenient()).unwrap();
        assert!(text.starts_with("namespace = \"hexen\";"), "{text}");
        assert_eq!(
            warnings,
            vec![UdmfWriteWarning::NamespaceDefaulted { used: "hexen" }]
        );
    }

    #[test]
    fn doom_map_gets_doom_namespace() {
        let mut map = map_with(vec![MapVertex { x: 0.0, y: 0.0 }], vec![]);
        map.format = MapFormat::Doom;
        map.namespace = None;
        let (text, _) = write_udmf(&map, &WriteOptions::lenient()).unwrap();
        assert!(text.starts_with("namespace = \"doom\";"), "{text}");
    }

    #[test]
    fn thing_flag_mapping_covers_boom_mbf_bits() {
        let mut w = Writer::new(Strictness::Strict);
        w.push_thing(0, &thing(0x0020 | 0x0040 | 0x0080)).unwrap();
        let out = &w.out;
        // "not in deathmatch" / "not in co-op" are set, so both keys are
        // omitted (= false). Bit 4 is clear, so single-player is stated.
        assert!(!out.contains("dm"), "{out}");
        assert!(!out.contains("coop"), "{out}");
        assert!(out.contains("single = true; "), "{out}");
        assert!(out.contains("friend = true; "), "{out}");
    }

    /// Doom → UDMF → Doom is the identity over every one of the 256 mapped
    /// thing-flag values. This is the property the wrong `single`/`dm`/`coop`
    /// defaults silently preserved (reader and writer agreed with each other
    /// while both disagreed with the spec); it must still hold now that both
    /// sides match the spec.
    #[test]
    fn doom_thing_flags_round_trip_through_udmf_for_every_value() {
        use crate::Limits;
        use crate::map::udmf::parse_udmf;

        for f in 0u32..256 {
            let mut w = Writer::new(Strictness::Strict);
            w.push_thing(0, &thing(f)).unwrap();
            let text = format!("namespace = \"doom\";\n{}", w.out);
            let parsed = parse_udmf(&text, Limits::default()).unwrap();
            assert_eq!(parsed.things[0].flags, f, "flags {f:#04x} did not survive");
        }
    }

    /// A leftover `TextureRef::Index` (one assembly could not resolve to a
    /// name, ADR-0022 §4) is rejected in both strictness modes — there is no
    /// honest recovery (ADR-0021 §5).
    #[test]
    fn texture_index_is_rejected_in_both_modes() {
        let mut map = tiny_map();
        map.sidedefs[0].middle = TextureRef::Index(42);
        for opts in [WriteOptions::strict(), WriteOptions::lenient()] {
            let err = write_udmf(&map, &opts).unwrap_err();
            assert!(matches!(
                err,
                UdmfWriteError::UnresolvedTextureIndex {
                    block: "sidedef",
                    index: 0,
                    ..
                }
            ));
        }
    }

    /// The mirror case on the sector arena: a Doom 64 floor/ceiling texture
    /// index is likewise rejected in both modes.
    #[test]
    fn sector_texture_index_is_rejected_in_both_modes() {
        let mut map = tiny_map();
        map.sectors[0].floor_flat = TextureRef::Index(7);
        for opts in [WriteOptions::strict(), WriteOptions::lenient()] {
            let err = write_udmf(&map, &opts).unwrap_err();
            assert!(matches!(
                err,
                UdmfWriteError::UnresolvedTextureIndex {
                    block: "sector",
                    index: 0,
                    ..
                }
            ));
        }
    }

    /// The mirror case on the sector arena's *ceiling* field specifically:
    /// `floor_flat` resolves fine, so the failure surfaces from the
    /// `textureceiling` call site rather than being short-circuited by the
    /// preceding `texturefloor` argument.
    #[test]
    fn sector_ceiling_texture_index_is_rejected_in_both_modes() {
        let mut map = tiny_map();
        map.sectors[0].ceiling_flat = TextureRef::Index(9);
        for opts in [WriteOptions::strict(), WriteOptions::lenient()] {
            let err = write_udmf(&map, &opts).unwrap_err();
            assert!(matches!(
                err,
                UdmfWriteError::UnresolvedTextureIndex {
                    block: "sector",
                    field: "textureceiling",
                    index: 0,
                }
            ));
        }
    }

    /// A Doom 64-sourced map's colored lighting has no UDMF slot: strict
    /// refuses it (tier-3 loss, ADR-0021 §5 amendment 3), lenient drops it
    /// and warns exactly once. Texture resolution is a separate concern
    /// (ADR-0022 §4) — `tiny_map`'s refs are already `TextureRef::Name`, so
    /// this test isolates the lighting policy.
    #[test]
    fn doom64_sourced_map_lighting_is_tier3_loss() {
        let mut map = tiny_map();
        map.format = MapFormat::Doom64;

        let err = write_udmf(&map, &WriteOptions::strict()).unwrap_err();
        assert_eq!(
            err,
            UdmfWriteError::UnrepresentableField {
                block: "sector",
                field: "colors",
                index: 0
            }
        );
        assert_eq!(
            err.to_string(),
            "sector #0 has a colors value, which UDMF cannot represent"
        );

        let (text, warnings) = write_udmf(&map, &WriteOptions::lenient()).unwrap();
        assert!(text.contains("WALL"));
        assert_eq!(
            warnings
                .iter()
                .filter(|w| matches!(w, UdmfWriteWarning::ColoredLightingDropped))
                .count(),
            1
        );
        assert_eq!(
            UdmfWriteWarning::ColoredLightingDropped.to_string(),
            "the map's Doom 64 colored lighting (sector color references and lights table) has no UDMF slot and was dropped"
        );
    }

    #[test]
    fn fmt_roundtrip_float_covers_extremes() {
        // (input, expected text) — whole-in-range values emit integer digits.
        assert_eq!(fmt_roundtrip_float(3.0), "3");
        assert_eq!(fmt_roundtrip_float(-0.0), "0");
        assert_eq!(fmt_roundtrip_float(2.0_f64.powi(60)), "1152921504606846976");
        assert_eq!(fmt_roundtrip_float(0.5), "0.5");
        // Out-of-i64-range whole float must NOT emit bare digits (they would
        // re-lex as an integer and overflow i64): shortest-roundtrip form.
        assert_eq!(fmt_roundtrip_float(1e300), "1e300");
        assert_eq!(fmt_roundtrip_float(1e-300), "1e-300");
    }

    #[test]
    // Exact `==` is the point of this test: the interface guarantee is
    // "bit-identical or `==`-equal", so a fuzzy comparison would defeat it.
    #[allow(clippy::float_cmp)]
    fn fmt_roundtrip_float_output_reparses_equal() {
        for v in [
            3.0,
            -0.0,
            0.5,
            -12345.678,
            1e300,
            -1e300,
            1e-300,
            2.0_f64.powi(60),
            9.007_199_254_740_993e15, // 2^53 + 1 as parsed: exercises the exactness edge
            f64::MAX,
            f64::MIN_POSITIVE,
        ] {
            let text = format!(
                "namespace = \"doom\";\nvertex {{ x = {}; y = 0.0; }}",
                fmt_roundtrip_float(v)
            );
            let map = crate::map::udmf::parse_udmf(&text, crate::Limits::default())
                .unwrap_or_else(|e| panic!("output for {v:?} failed to reparse: {e}"));
            assert_eq!(map.vertices[0].x, v, "value {v:?} did not round-trip");
        }
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
