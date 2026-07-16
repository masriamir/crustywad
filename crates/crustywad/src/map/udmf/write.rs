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
use crate::map::graph::{
    MapFormat, MapLinedef, MapSector, MapSidedef, MapThing, MapVertex, TextureRef, linedef_id_unset,
};
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
    /// A [`TextureRef::Index`] reached the writer; a Doom 64 texture index
    /// cannot be written as a name until the texture layer (#156/#157) can
    /// resolve it (both strictness modes; ADR-0021 §5).
    #[error("unresolvable texture index for {field} in {block} #{index}")]
    UnresolvedTextureIndex {
        /// The block kind (`"sidedef"` or `"sector"`).
        block: &'static str,
        /// The field name (e.g. `"texturetop"`, `"texturefloor"`).
        field: &'static str,
        /// The 0-based block index.
        index: usize,
    },
    /// The map's source format cannot be expressed by this writer — a Doom 64
    /// map's texture indices and colored lighting have no classic/UDMF
    /// representation until the texture layer (#156/#157) exists (both
    /// strictness modes; ADR-0021 §5).
    #[error("cannot write a {format:?}-sourced map")]
    UnsupportedSourceFormat {
        /// The assembled map's source format.
        format: MapFormat,
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
            Self::NonFiniteCoordinate { .. } | Self::EmptyNamespace | Self::NoFrontSide { .. } => {
                true
            }
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

/// Resolves a [`TextureRef`] to a name, or fails: a Doom 64 texture index has
/// no name until the texture layer (#156/#157) exists, so this is not a
/// recoverable defect (ADR-0021 §5) — it errors in **both** strictness modes.
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
        for (bit, name) in Self::LINEDEF_FLAGS {
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

/// Serializes an assembled map to UDMF `TEXTMAP` text.
///
/// Fields are emitted only when they differ from their UDMF spec default;
/// `f64` coordinates narrow to integer form when whole. See the module docs.
///
/// # Errors
/// - [`UdmfWriteError::UnsupportedSourceFormat`] — `map.format()` is
///   [`MapFormat::Doom64`] (returned in **both** strictness modes; ADR-0021 §5).
/// - [`UdmfWriteError::EmptyNamespace`] — `map.namespace()` is `Some("")` (strict).
/// - [`UdmfWriteError::NonFiniteCoordinate`] — a coordinate/height is NaN or ∞ (strict).
/// - [`UdmfWriteError::NoFrontSide`] — a linedef has no front sidedef, which
///   UDMF cannot represent (strict).
/// - [`UdmfWriteError::UnresolvedTextureIndex`] — a sidedef/sector carries a
///   [`TextureRef::Index`] (returned in **both** strictness modes; ADR-0021 §5).
pub fn write_udmf(
    map: &Map,
    opts: &WriteOptions,
) -> Result<(String, Vec<UdmfWriteWarning>), UdmfWriteError> {
    if map.format() == MapFormat::Doom64 {
        return Err(UdmfWriteError::UnsupportedSourceFormat {
            format: map.format(),
        });
    }

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
/// [`UdmfWriteError::NonFiniteCoordinate`], and
/// [`UdmfWriteError::NoFrontSide`] (strict mode); and
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
            leafs: Vec::new(),
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
            leafs: Vec::new(),
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

    /// A Doom 64 texture index has no name until the texture layer (#156/#157)
    /// exists, so a `TextureRef::Index` is rejected in both strictness modes —
    /// there is no honest recovery (ADR-0021 §5).
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

    /// A Doom 64-sourced map has no UDMF representation (texture indices,
    /// colored lighting) until the texture layer (#156/#157) exists, so it is
    /// rejected in both strictness modes (ADR-0021 §5), before namespace
    /// derivation or any per-field handling runs.
    #[test]
    fn doom64_sourced_map_is_rejected_in_both_modes() {
        let mut map = tiny_map();
        map.format = MapFormat::Doom64;
        for opts in [WriteOptions::strict(), WriteOptions::lenient()] {
            let err = write_udmf(&map, &opts).unwrap_err();
            assert_eq!(
                err,
                UdmfWriteError::UnsupportedSourceFormat {
                    format: MapFormat::Doom64
                }
            );
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
