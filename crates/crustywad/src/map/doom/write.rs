//! Doom binary map writing: serialize an assembled [`Map`] into the classic
//! `THINGS`/`LINEDEFS`/`SIDEDEFS`/`VERTEXES`/`SECTORS` lumps (ADR-0019).
//! Requires the `write` feature.
//!
//! [`write_doom_map`] produces the five lump byte buffers; [`add_doom_map`] adds
//! a complete map group to a [`WadBuilder`], including the zero-length
//! `SEGS`/`SSECTORS`/`NODES`/`REJECT`/`BLOCKMAP` placeholders.
//!
//! # Data loss
//!
//! The Doom format is narrower than the [`Map`] graph, so conversion is lossy in
//! three tiers (ADR-0019):
//!
//! 1. **Structurally impossible** — more than 65,536 vertices/sectors or 65,535
//!    sidedefs cannot be indexed by Doom's `u16` references. This is a
//!    [`DoomWriteError::TooManyElements`] in *both* strictness modes.
//! 2. **Value loss** — fractional or out-of-range coordinates, out-of-range
//!    integers, oversized names. Strict errors; lenient rounds/clamps/truncates
//!    and warns.
//! 3. **No slot in the format** — linedef `args[1..=4]` and `id`, thing
//!    `special`/`args`/`height`/`id`. Strict errors; lenient drops and warns.
//!
//! **Nodes are never built.** The emitted node lumps are empty and every call
//! returns [`DoomWriteWarning::NodesNotBuilt`]: run an external nodebuilder
//! (`zdbsp`, `bsp`, …) before the map is playable.
//!
//! # Name fidelity
//!
//! A texture/flat name survives conversion byte-for-byte **only if it is valid
//! UTF-8 and NUL-clean** — valid UTF-8 up to its first NUL, with nothing but
//! NUL padding after it. That holds for every name in practice (they are
//! ASCII), but not unconditionally, and the limit is the graph, not this
//! writer: [`Name8`] does keep the raw `[u8; 8]`, whereas [`MapSidedef`] and
//! [`MapSector`] store `String`, filled on read via
//! [`Name8::as_str_lossy`](Name8::as_str_lossy) — which trims at the first NUL
//! and decodes with `String::from_utf8_lossy`. So an on-disk
//! `b"\x81OCK\0\0\0\0"` reaches the graph as `"\u{FFFD}OCK"` and is written
//! back as `EF BF BD 4F 43 4B 00 00` — different bytes, no warning — and an
//! 8-byte all-invalid name becomes a 24-byte replacement-character string that
//! then fails as [`DoomWriteError::NameTooLong`] in strict mode. Bytes after
//! the NUL terminator (which real IWADs do contain) are dropped on read for
//! the same reason. This is read-time normalization, not conversion loss; only
//! a name longer than 8 bytes is tier-2 loss below.
//!
//! # Round-tripping
//!
//! Doom → UDMF → Doom is byte-identical **for maps whose linedef flags fit the
//! nine standard bits (0–8), whose thing flags fit the eight mapped bits (0–7),
//! and whose thing angles are already in `0..360`**. Outside that envelope the
//! UDMF leg loses data, so the returned Doom lumps differ from the originals:
//!
//! - A linedef flag bit ≥ 9 (e.g. Boom's `passuse`, `0x200`) has no UDMF boolean
//!   in [`write_udmf`](crate::map::udmf::write_udmf), which emits only the nine
//!   standard flags, so it is dropped.
//! - A thing flag bit ≥ 8 likewise has no UDMF boolean (the eight mapped bits
//!   are skill 1–5, ambush, multiplayer-only, and the Boom/MBF dm/co-op/friend
//!   bits), so it is dropped.
//! - A thing `angle` ≥ 360 is normalized modulo 360 on the way out to UDMF.
//!
//! **UDMF → Doom → UDMF is not reversible** — coordinate rounding and tier-3
//! drops are one-way.

use std::io::Cursor;

use binrw::{BinWrite, BinWriterExt};

use crate::Strictness;
use crate::map::common::{Name8, Sector, Sidedef, Vertex};
use crate::map::doom::{Linedef, Thing};
use crate::map::graph::{
    Map, MapFormat, MapLinedef, MapSector, MapSidedef, MapThing, MapVertex, TextureRef,
    linedef_id_unset,
};
use crate::write::{WadBuilder, WriteOptions};

/// Message for the infallible `BinWrite`-into-`Vec` calls.
const INFALLIBLE: &str = "writing to a Vec never fails";

/// Doom's `u16` vertex/sector references address at most 65,536 elements.
const MAX_INDEXED: usize = 65_536;
/// Sidedefs get one fewer, because `0xFFFF` is the "no sidedef" sentinel.
const MAX_SIDEDEFS: usize = 65_535;
/// The largest addressable vertex/sector index (`MAX_INDEXED - 1`).
const MAX_INDEX: i64 = 0xFFFF;
/// The largest addressable sidedef index (`MAX_SIDEDEFS - 1`). One below the
/// `0xFFFF` "no sidedef" sentinel, so a real index can never collide with it.
const MAX_SIDEDEF_INDEX: i64 = 0xFFFE;
/// Doom's "no sidedef" sentinel, stored in a one-sided linedef's `left_sidedef`.
const NO_SIDEDEF: u16 = 0xFFFF;

/// An error that prevents writing a map to the Doom binary format.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum DoomWriteError {
    /// An arena holds more elements than Doom's `u16` indices can address.
    /// Returned in **both** strictness modes — there is no honest recovery.
    #[error("{kind} count {count} exceeds the Doom format maximum of {max}")]
    TooManyElements {
        /// The arena name (e.g. `"vertices"`).
        kind: &'static str,
        /// The actual element count.
        count: usize,
        /// The maximum Doom can index.
        max: usize,
    },
    /// An `x`/`y` coordinate (of a vertex or a thing) was NaN or infinite
    /// (strict; lenient writes `0`). Doom has no thing `height` field — the
    /// graph's `height` is dropped by this writer, so it is never checked here.
    #[error("non-finite {field} in {block} #{index}")]
    NonFiniteCoordinate {
        /// The block kind (e.g. `"vertex"`, `"thing"`).
        block: &'static str,
        /// The field name (e.g. `"x"`).
        field: &'static str,
        /// The 0-based element index.
        index: usize,
    },
    /// A coordinate was not a whole number, and Doom stores `i16` (strict;
    /// lenient rounds to nearest, half away from zero).
    #[error("fractional {field} {value} in {block} #{index} cannot be stored as an i16")]
    FractionalCoordinate {
        /// The block kind.
        block: &'static str,
        /// The field name.
        field: &'static str,
        /// The 0-based element index.
        index: usize,
        /// The offending value.
        value: f64,
    },
    /// A value was outside its Doom on-disk field's range (strict; lenient clamps).
    #[error("{field} value {value} in {block} #{index} is out of range for the Doom format")]
    ValueOutOfRange {
        /// The block kind.
        block: &'static str,
        /// The field name.
        field: &'static str,
        /// The 0-based element index.
        index: usize,
        /// The offending value.
        value: i64,
    },
    /// A field carried data the Doom format has no slot for (strict; lenient drops).
    #[error("{block} #{index} has a {field} value, which the Doom format cannot represent")]
    UnrepresentableField {
        /// The block kind.
        block: &'static str,
        /// The field name (e.g. `"arg1"`, `"height"`).
        field: &'static str,
        /// The 0-based element index.
        index: usize,
    },
    /// A texture or flat name exceeded Doom's 8-byte name field (strict; lenient truncates).
    #[error("name {name:?} is {len} bytes; Doom names are at most 8 bytes")]
    NameTooLong {
        /// The offending name.
        name: String,
        /// Its length in bytes.
        len: usize,
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
    /// [`DoomWriteWarning::ColoredLightingDropped`]; ADR-0021 §5 amendment 3).
    #[error("cannot write a {format:?}-sourced map")]
    UnsupportedSourceFormat {
        /// The assembled map's source format.
        format: MapFormat,
    },
}

impl DoomWriteError {
    /// Whether re-running the write with [`WriteOptions::lenient`] recovers
    /// from this error, turning it into a [`DoomWriteWarning`]-carrying
    /// success.
    ///
    /// Returns `false` for the errors produced identically in both strictness
    /// modes — [`TooManyElements`][Self::TooManyElements],
    /// [`UnresolvedTextureIndex`][Self::UnresolvedTextureIndex], and
    /// [`UnsupportedSourceFormat`][Self::UnsupportedSourceFormat] — where
    /// suggesting lenient mode would mislead.
    #[must_use]
    pub fn is_lenient_recoverable(&self) -> bool {
        match self {
            Self::NonFiniteCoordinate { .. }
            | Self::FractionalCoordinate { .. }
            | Self::ValueOutOfRange { .. }
            | Self::UnrepresentableField { .. }
            | Self::NameTooLong { .. } => true,
            Self::TooManyElements { .. }
            | Self::UnresolvedTextureIndex { .. }
            | Self::UnsupportedSourceFormat { .. } => false,
        }
    }
}

/// A non-fatal issue recovered while writing a map to the Doom binary format in
/// lenient mode — except [`NodesNotBuilt`][DoomWriteWarning::NodesNotBuilt],
/// which is always returned.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum DoomWriteWarning {
    /// The node lumps were emitted empty; run an external nodebuilder before
    /// the map is playable. Returned by every [`add_doom_map`] /
    /// [`write_doom_map`] call, in both strictness modes.
    #[error("node lumps (SEGS/SSECTORS/NODES/REJECT/BLOCKMAP) were not built; run a nodebuilder")]
    NodesNotBuilt,
    /// A non-finite coordinate/height was replaced with `0`.
    #[error("non-finite {field} in {block} #{index} replaced with 0")]
    NonFiniteReplaced {
        /// The block kind.
        block: &'static str,
        /// The field name.
        field: &'static str,
        /// The 0-based element index.
        index: usize,
    },
    /// A fractional coordinate was rounded to the nearest whole map unit.
    #[error("fractional {field} {from} in {block} #{index} rounded to {to}")]
    CoordinateRounded {
        /// The block kind.
        block: &'static str,
        /// The field name.
        field: &'static str,
        /// The 0-based element index.
        index: usize,
        /// The original value.
        from: f64,
        /// The value written.
        to: i16,
    },
    /// A `flags` value wider than Doom's 16-bit field was **truncated** to its
    /// low 16 bits. Bit fields truncate rather than clamp: clamping would turn
    /// one stray high bit into all sixteen Doom flags at once.
    #[error("{field} value {from} in {block} #{index} truncated to {to}")]
    ValueTruncated {
        /// The block kind.
        block: &'static str,
        /// The field name.
        field: &'static str,
        /// The 0-based element index.
        index: usize,
        /// The original value.
        from: i64,
        /// The value written.
        to: i64,
    },
    /// An out-of-range value was clamped to the Doom field's range.
    #[error("{field} value {from} in {block} #{index} clamped to {to}")]
    ValueClamped {
        /// The block kind.
        block: &'static str,
        /// The field name.
        field: &'static str,
        /// The 0-based element index.
        index: usize,
        /// The original value.
        from: i64,
        /// The value written.
        to: i64,
    },
    /// A field the Doom format cannot represent was dropped.
    #[error("{field} on {block} #{index} was dropped (not representable in the Doom format)")]
    FieldDropped {
        /// The block kind.
        block: &'static str,
        /// The field name.
        field: &'static str,
        /// The 0-based element index.
        index: usize,
    },
    /// A texture or flat name was truncated to 8 bytes.
    #[error("name {name:?} was truncated to 8 bytes")]
    NameTruncated {
        /// The offending name.
        name: String,
    },
    /// A Doom 64 map's colored lighting (sector color references and the
    /// engine-model lights table) has no slot in the Doom format and was
    /// dropped. Emitted at most once per map (lenient mode only — strict
    /// mode instead returns [`DoomWriteError::UnrepresentableField`] naming
    /// `block: "sector", field: "colors"`; ADR-0021 §5 amendment 3).
    #[error(
        "the map's Doom 64 colored lighting (sector color references and lights table) has no Doom slot and was dropped"
    )]
    ColoredLightingDropped,
}

/// The five serialized Doom map data lumps, in canonical order.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct DoomMapLumps {
    /// The serialized `THINGS` lump.
    pub things: Vec<u8>,
    /// The serialized `LINEDEFS` lump.
    pub linedefs: Vec<u8>,
    /// The serialized `SIDEDEFS` lump.
    pub sidedefs: Vec<u8>,
    /// The serialized `VERTEXES` lump.
    pub vertexes: Vec<u8>,
    /// The serialized `SECTORS` lump.
    pub sectors: Vec<u8>,
}

/// Narrows a `Map` into Doom records, accumulating lenient-mode warnings.
///
/// `pub(crate)` so the `nodebuild` feature's builders can reuse the exact same
/// coordinate/index narrowing pass (ADR-0024 §3) rather than restating the
/// write path's three-tier decision table. Callers construct one with
/// [`Narrower::new`], run the `narrow_*` passes, and read back the accumulated
/// [`warnings`](Self::warnings) field.
pub(crate) struct Narrower {
    /// The lenient-mode warnings accumulated so far. Starts empty:
    /// [`write_doom_map`] pushes [`DoomWriteWarning::NodesNotBuilt`] as its
    /// first warning (the node lumps it emits are empty), whereas the
    /// `nodebuild` builders — which *do* build those lumps — must not carry
    /// that warning (ADR-0024 §3, §9 Global Constraint 9).
    pub(crate) warnings: Vec<DoomWriteWarning>,
    strictness: Strictness,
}

impl Narrower {
    pub(crate) fn new(strictness: Strictness) -> Self {
        Self {
            warnings: Vec::new(),
            strictness,
        }
    }

    /// Narrows an `f64` coordinate to `i16`: non-finite → 0, fractional →
    /// rounded (half away from zero), out-of-range → clamped. Each step errors
    /// in strict mode and warns in lenient mode.
    fn coord(
        &mut self,
        block: &'static str,
        field: &'static str,
        index: usize,
        value: f64,
    ) -> Result<i16, DoomWriteError> {
        if !value.is_finite() {
            return match self.strictness {
                Strictness::Strict => Err(DoomWriteError::NonFiniteCoordinate {
                    block,
                    field,
                    index,
                }),
                Strictness::Lenient => {
                    self.warnings.push(DoomWriteWarning::NonFiniteReplaced {
                        block,
                        field,
                        index,
                    });
                    Ok(0)
                }
            };
        }

        // `round` is half-away-from-zero, which is what map editors do.
        let rounded = value.round();
        let (min, max) = (f64::from(i16::MIN), f64::from(i16::MAX));
        let clamped = rounded.clamp(min, max);
        // Safe: `clamped` is finite and within i16's range by construction.
        #[allow(clippy::cast_possible_truncation)]
        let out = clamped as i16;

        // Exact whole-number test. An `f64::EPSILON` comparison would call a
        // tiny fractional value such as `1e-16` whole and silently round it to
        // `0` even in strict mode — precisely the silent loss this policy
        // exists to prevent. `value` is finite here (guarded above), so
        // `fract()` is well-defined; comparing it against zero is exact.
        if value.fract() != 0.0 {
            match self.strictness {
                Strictness::Strict => {
                    return Err(DoomWriteError::FractionalCoordinate {
                        block,
                        field,
                        index,
                        value,
                    });
                }
                Strictness::Lenient => self.warnings.push(DoomWriteWarning::CoordinateRounded {
                    block,
                    field,
                    index,
                    from: value,
                    to: out,
                }),
            }
        }

        // Ordering comparison rather than a float equality/epsilon test: the
        // clamp fired exactly when `rounded` fell outside `i16`'s range.
        if rounded < min || rounded > max {
            // Safe: `rounded` is finite; the float→int cast saturates, which is
            // exactly the clamp we want for the *reported* value.
            #[allow(clippy::cast_possible_truncation)]
            let reported = rounded as i64;
            match self.strictness {
                Strictness::Strict => {
                    return Err(DoomWriteError::ValueOutOfRange {
                        block,
                        field,
                        index,
                        value: reported,
                    });
                }
                Strictness::Lenient => self.warnings.push(DoomWriteWarning::ValueClamped {
                    block,
                    field,
                    index,
                    from: reported,
                    to: i64::from(out),
                }),
            }
        }

        Ok(out)
    }

    /// Narrows an integer to `[min, max]`: strict errors, lenient clamps and warns.
    fn int(
        &mut self,
        block: &'static str,
        field: &'static str,
        index: usize,
        value: i64,
        min: i64,
        max: i64,
    ) -> Result<i64, DoomWriteError> {
        if value >= min && value <= max {
            return Ok(value);
        }
        match self.strictness {
            Strictness::Strict => Err(DoomWriteError::ValueOutOfRange {
                block,
                field,
                index,
                value,
            }),
            Strictness::Lenient => {
                let clamped = value.clamp(min, max);
                self.warnings.push(DoomWriteWarning::ValueClamped {
                    block,
                    field,
                    index,
                    from: value,
                    to: clamped,
                });
                Ok(clamped)
            }
        }
    }

    /// Narrows an integer to `[min, max]` and casts it to `i16`.
    fn int16(
        &mut self,
        block: &'static str,
        field: &'static str,
        index: usize,
        value: i32,
    ) -> Result<i16, DoomWriteError> {
        let ok = self.int(
            block,
            field,
            index,
            i64::from(value),
            i64::from(i16::MIN),
            i64::from(i16::MAX),
        )?;
        Ok(i16::try_from(ok).expect("clamped to i16's range"))
    }

    /// Narrows an unsigned integer to Doom's 16-bit numeric fields (`special`,
    /// `sector_tag`), which are read back as `u16`. Out-of-range values clamp
    /// in lenient mode — the right recovery for a *magnitude*.
    fn uint16(
        &mut self,
        block: &'static str,
        field: &'static str,
        index: usize,
        value: i64,
    ) -> Result<u16, DoomWriteError> {
        let ok = self.int(block, field, index, value, 0, 0xFFFF)?;
        Ok(u16::try_from(ok).expect("clamped to 0..=0xFFFF"))
    }

    /// Narrows a `flags` bit field to Doom's 16-bit on-disk field by
    /// **truncation** (`& 0xFFFF`), not clamping. A bit field is not a
    /// magnitude: clamping `0x1_0001` to `0xFFFF` would turn one stray high bit
    /// into *all sixteen* Doom flags (blocking, secret, two-sided, …), while
    /// truncation simply drops the bits Doom has no room for and keeps the ones
    /// it does. Strict still errors; lenient warns with
    /// [`DoomWriteWarning::ValueTruncated`].
    fn flags16(
        &mut self,
        block: &'static str,
        field: &'static str,
        index: usize,
        value: i64,
    ) -> Result<u16, DoomWriteError> {
        if let Ok(v) = u16::try_from(value) {
            return Ok(v);
        }
        match self.strictness {
            Strictness::Strict => Err(DoomWriteError::ValueOutOfRange {
                block,
                field,
                index,
                value,
            }),
            Strictness::Lenient => {
                let truncated = u16::try_from(value & 0xFFFF).expect("masked to 0..=0xFFFF");
                self.warnings.push(DoomWriteWarning::ValueTruncated {
                    block,
                    field,
                    index,
                    from: value,
                    to: i64::from(truncated),
                });
                Ok(truncated)
            }
        }
    }

    /// Reports a field the Doom format cannot represent, when it carries a
    /// non-default value: strict errors, lenient drops and warns.
    fn drop_field(
        &mut self,
        block: &'static str,
        field: &'static str,
        index: usize,
        is_set: bool,
    ) -> Result<(), DoomWriteError> {
        if !is_set {
            return Ok(());
        }
        match self.strictness {
            Strictness::Strict => Err(DoomWriteError::UnrepresentableField {
                block,
                field,
                index,
            }),
            Strictness::Lenient => {
                self.warnings.push(DoomWriteWarning::FieldDropped {
                    block,
                    field,
                    index,
                });
                Ok(())
            }
        }
    }

    /// Encodes a texture/flat name into Doom's NUL-padded 8-byte field: strict
    /// rejects anything longer, lenient truncates to the first 8 bytes and warns.
    fn name8(&mut self, name: &str) -> Result<Name8, DoomWriteError> {
        let bytes = name.as_bytes();
        if bytes.len() > 8 {
            match self.strictness {
                Strictness::Strict => {
                    return Err(DoomWriteError::NameTooLong {
                        name: name.to_string(),
                        len: bytes.len(),
                    });
                }
                Strictness::Lenient => self.warnings.push(DoomWriteWarning::NameTruncated {
                    name: name.to_string(),
                }),
            }
        }
        let mut out = [0u8; 8];
        let n = bytes.len().min(8);
        out[..n].copy_from_slice(&bytes[..n]);
        Ok(Name8(out))
    }

    /// Narrows an arena index to a `u16` reference, bounded by `max` — `0xFFFF`
    /// for vertex/sector references, `0xFFFE` for sidedef references (`0xFFFF`
    /// is the "no sidedef" sentinel).
    fn index(
        &mut self,
        block: &'static str,
        field: &'static str,
        elem: usize,
        idx: usize,
        max: i64,
    ) -> Result<u16, DoomWriteError> {
        let value = i64::try_from(idx).unwrap_or(i64::MAX);
        let ok = self.int(block, field, elem, value, 0, max)?;
        // Safe: clamped to 0..=max above, and max <= 0xFFFF.
        Ok(u16::try_from(ok).expect("clamped to a u16 index"))
    }
}

/// Serializes a slice of `BinWrite` records into a lump byte buffer.
fn encode<T>(records: &[T]) -> Vec<u8>
where
    T: for<'a> BinWrite<Args<'a> = ()>,
{
    let mut cursor = Cursor::new(Vec::new());
    for r in records {
        cursor.write_le(r).expect(INFALLIBLE);
    }
    cursor.into_inner()
}

/// Narrows the vertex arena. Doom stores `i16` coordinates.
///
/// `pub(crate)` so the `nodebuild` feature's blockmap/node builders can narrow
/// vertices through the identical pass (ADR-0024 §3) before rasterizing or
/// partitioning on the `i16` geometry the engine will actually read.
pub(crate) fn narrow_vertices(
    n: &mut Narrower,
    raw: &[MapVertex],
) -> Result<Vec<Vertex>, DoomWriteError> {
    let mut out = Vec::with_capacity(raw.len());
    for (i, v) in raw.iter().enumerate() {
        out.push(Vertex {
            x: n.coord("vertex", "x", i, v.x)?,
            y: n.coord("vertex", "y", i, v.y)?,
        });
    }
    Ok(out)
}

/// Narrows the linedef arena. Doom keeps `special` + a single sector tag
/// (`args[0]`); `args[1..=4]` and `id` have no slot (ADR-0019 tier 3).
///
/// `format` selects the graph's "no id" sentinel, which is source-dependent —
/// see [`linedef_id_unset`].
fn narrow_linedefs(
    n: &mut Narrower,
    raw: &[MapLinedef],
    format: MapFormat,
) -> Result<Vec<Linedef>, DoomWriteError> {
    let unset = linedef_id_unset(format);
    let mut out = Vec::with_capacity(raw.len());
    for (i, l) in raw.iter().enumerate() {
        for (arg_i, field) in [(1, "arg1"), (2, "arg2"), (3, "arg3"), (4, "arg4")] {
            n.drop_field("linedef", field, i, l.special.args[arg_i] != 0)?;
        }
        // Only a *real* id is tier-3 loss; the sentinel is the absence of one.
        n.drop_field("linedef", "id", i, l.id != unset)?;

        out.push(Linedef {
            start_vertex: n.index("linedef", "v1", i, l.start.0, MAX_INDEX)?,
            end_vertex: n.index("linedef", "v2", i, l.end.0, MAX_INDEX)?,
            flags: n.flags16("linedef", "flags", i, i64::from(l.flags))?,
            special_type: n.uint16("linedef", "special", i, i64::from(l.special.special))?,
            sector_tag: n.uint16("linedef", "arg0", i, i64::from(l.special.args[0]))?,
            // `NO_SIDEDEF` (0xFFFF) is Doom's "no sidedef" sentinel for either
            // field (ADR-0020); a real sidedef index is capped at
            // MAX_SIDEDEF_INDEX (0xFFFE), so the two can never collide.
            right_sidedef: match l.right {
                Some(s) => n.index("linedef", "sidefront", i, s.0, MAX_SIDEDEF_INDEX)?,
                None => NO_SIDEDEF,
            },
            left_sidedef: match l.left {
                Some(s) => n.index("linedef", "sideback", i, s.0, MAX_SIDEDEF_INDEX)?,
                None => NO_SIDEDEF,
            },
        });
    }
    Ok(out)
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
) -> Result<&'a str, DoomWriteError> {
    tex.as_name().ok_or(DoomWriteError::UnresolvedTextureIndex {
        block,
        field,
        index,
    })
}

/// Narrows the sidedef arena. Doom stores `i16` offsets and 8-byte texture names.
fn narrow_sidedefs(n: &mut Narrower, raw: &[MapSidedef]) -> Result<Vec<Sidedef>, DoomWriteError> {
    let mut out = Vec::with_capacity(raw.len());
    for (i, s) in raw.iter().enumerate() {
        out.push(Sidedef {
            x_offset: n.int16("sidedef", "offsetx", i, s.x_offset)?,
            y_offset: n.int16("sidedef", "offsety", i, s.y_offset)?,
            upper_texture: n.name8(texture_name("sidedef", "texturetop", i, &s.upper)?)?,
            lower_texture: n.name8(texture_name("sidedef", "texturebottom", i, &s.lower)?)?,
            middle_texture: n.name8(texture_name("sidedef", "texturemiddle", i, &s.middle)?)?,
            sector: n.index("sidedef", "sector", i, s.sector.0, MAX_INDEX)?,
        });
    }
    Ok(out)
}

/// Narrows the sector arena. Every Doom sector field is `i16`.
fn narrow_sectors(n: &mut Narrower, raw: &[MapSector]) -> Result<Vec<Sector>, DoomWriteError> {
    let mut out = Vec::with_capacity(raw.len());
    for (i, s) in raw.iter().enumerate() {
        out.push(Sector {
            floor_height: n.int16("sector", "heightfloor", i, s.floor_height)?,
            ceiling_height: n.int16("sector", "heightceiling", i, s.ceiling_height)?,
            floor_texture: n.name8(texture_name("sector", "texturefloor", i, &s.floor_flat)?)?,
            ceiling_texture: n.name8(texture_name(
                "sector",
                "textureceiling",
                i,
                &s.ceiling_flat,
            )?)?,
            light_level: n.int16("sector", "lightlevel", i, s.light)?,
            special_type: n.int16("sector", "special", i, s.special)?,
            tag: n.int16("sector", "id", i, s.tag)?,
        });
    }
    Ok(out)
}

/// Narrows the thing arena. Doom things carry no special, args, height, or tid
/// (ADR-0019 tier 3), and only the low 16 flag bits.
fn narrow_things(n: &mut Narrower, raw: &[MapThing]) -> Result<Vec<Thing>, DoomWriteError> {
    let mut out = Vec::with_capacity(raw.len());
    for (i, t) in raw.iter().enumerate() {
        n.drop_field("thing", "height", i, t.height != 0.0)?;
        n.drop_field("thing", "id", i, t.id != 0)?;
        n.drop_field("thing", "special", i, t.special.special != 0)?;
        n.drop_field("thing", "args", i, t.special.args != [0; 5])?;

        out.push(Thing {
            x: n.coord("thing", "x", i, t.x)?,
            y: n.coord("thing", "y", i, t.y)?,
            angle: t.angle,
            type_id: t.type_id,
            flags: n.flags16("thing", "flags", i, i64::from(t.flags))?,
        });
    }
    Ok(out)
}

/// Serializes an assembled map into the five Doom binary map data lumps.
///
/// The returned warnings always include [`DoomWriteWarning::NodesNotBuilt`]:
/// this function does not build nodes. See the module docs for the full
/// three-tier data-loss policy.
///
/// # Errors
/// - [`DoomWriteError::UnrepresentableField`] — includes `map.format()`
///   being [`MapFormat::Doom64`] in strict mode: colored lighting (sector
///   color references and the engine-model lights table) has no slot in the
///   Doom format (`block: "sector", field: "colors"`, `index: 0`; ADR-0021
///   §5 amendment 3). Lenient mode instead drops it and returns
///   [`DoomWriteWarning::ColoredLightingDropped`].
/// - [`DoomWriteError::UnsupportedSourceFormat`] — an unrecognized future
///   [`MapFormat`] variant this writer has no policy for (returned in
///   **both** strictness modes).
/// - [`DoomWriteError::TooManyElements`] — an arena exceeds Doom's `u16` index
///   space (returned in **both** strictness modes).
/// - [`DoomWriteError::UnresolvedTextureIndex`] — a sidedef/sector carries a
///   [`TextureRef::Index`] left unresolved by assembly (returned in **both**
///   strictness modes; ADR-0021 §5, ADR-0022 §4).
/// - In strict mode only: [`DoomWriteError::NonFiniteCoordinate`],
///   [`DoomWriteError::FractionalCoordinate`], [`DoomWriteError::ValueOutOfRange`],
///   [`DoomWriteError::UnrepresentableField`], [`DoomWriteError::NameTooLong`].
pub fn write_doom_map(
    map: &Map,
    opts: &WriteOptions,
) -> Result<(DoomMapLumps, Vec<DoomWriteWarning>), DoomWriteError> {
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
                return Err(DoomWriteError::UnrepresentableField {
                    block: "sector",
                    field: "colors",
                    index: 0,
                });
            }
        }
        // A future `MapFormat` variant this writer has never heard of:
        // reject in both modes rather than silently mis-writing. `MapFormat`
        // is `#[non_exhaustive]` for downstream crates, but every variant
        // known to this crate is already matched above, so this arm is
        // unreachable today — it exists purely as a defensive fallback for
        // when a new variant is added.
        #[allow(unreachable_patterns)]
        format => return Err(DoomWriteError::UnsupportedSourceFormat { format }),
    }

    for (kind, count, max) in [
        ("vertices", map.vertices().len(), MAX_INDEXED),
        ("sectors", map.sectors().len(), MAX_INDEXED),
        ("sidedefs", map.sidedefs().len(), MAX_SIDEDEFS),
    ] {
        if count > max {
            return Err(DoomWriteError::TooManyElements { kind, count, max });
        }
    }

    let mut n = Narrower::new(opts.strictness);
    // Nodes are never built by this path, in either mode — say so up front, as
    // the first warning every call returns (this seeding moved out of
    // `Narrower::new` so the `nodebuild` builders, which reuse the narrower but
    // *do* build nodes, never inherit it; ADR-0024 §3).
    n.warnings.push(DoomWriteWarning::NodesNotBuilt);
    if map.format() == MapFormat::Doom64 {
        n.warnings.push(DoomWriteWarning::ColoredLightingDropped);
    }
    let vertices = narrow_vertices(&mut n, map.vertices())?;
    let linedefs = narrow_linedefs(&mut n, map.linedefs(), map.format())?;
    let sidedefs = narrow_sidedefs(&mut n, map.sidedefs())?;
    let sectors = narrow_sectors(&mut n, map.sectors())?;
    let things = narrow_things(&mut n, map.things())?;

    let lumps = DoomMapLumps {
        things: encode(&things),
        linedefs: encode(&linedefs),
        sidedefs: encode(&sidedefs),
        vertexes: encode(&vertices),
        sectors: encode(&sectors),
    };
    Ok((lumps, n.warnings))
}

/// Serializes `map` and adds a complete Doom map group to `builder`: the `name`
/// marker, the five data lumps, and zero-length `SEGS`, `SSECTORS`, `NODES`,
/// `REJECT`, and `BLOCKMAP` placeholders in canonical order.
///
/// **The map is not playable until an external nodebuilder processes it** — the
/// returned warnings always include [`DoomWriteWarning::NodesNotBuilt`].
///
/// The caller invokes [`WadBuilder::build`] afterward (which returns
/// [`WriteError`](crate::WriteError)).
///
/// # Errors
/// Same as [`write_doom_map`].
pub fn add_doom_map(
    builder: &mut WadBuilder,
    name: &str,
    map: &Map,
    opts: &WriteOptions,
) -> Result<Vec<DoomWriteWarning>, DoomWriteError> {
    let (lumps, warnings) = write_doom_map(map, opts)?;
    builder.add_lump(name, b"");
    builder.add_lump("THINGS", lumps.things);
    builder.add_lump("LINEDEFS", lumps.linedefs);
    builder.add_lump("SIDEDEFS", lumps.sidedefs);
    builder.add_lump("VERTEXES", lumps.vertexes);
    builder.add_lump("SEGS", b"");
    builder.add_lump("SSECTORS", b"");
    builder.add_lump("NODES", b"");
    builder.add_lump("SECTORS", lumps.sectors);
    builder.add_lump("REJECT", b"");
    builder.add_lump("BLOCKMAP", b"");
    Ok(warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::graph::{
        Map, MapFormat, MapLinedef, MapSector, MapSidedef, MapThing, MapVertex, SectorIdx,
        SidedefIdx, Special, VertexIdx,
    };
    use crate::map::parse_records;
    use crate::write::WriteOptions;

    /// A one-linedef, one-sector map with a single thing — the smallest graph
    /// that exercises every arena.
    fn tiny_map() -> Map {
        Map {
            name: "MAP01".into(),
            format: MapFormat::Udmf,
            namespace: Some("doom".into()),
            vertices: vec![MapVertex { x: 0.0, y: 0.0 }, MapVertex { x: 64.0, y: 0.0 }],
            linedefs: vec![MapLinedef {
                start: VertexIdx(0),
                end: VertexIdx(1),
                right: Some(SidedefIdx(0)),
                left: None,
                flags: 0b1,
                special: Special {
                    special: 11,
                    args: [7, 0, 0, 0, 0],
                },
                // A UDMF-sourced linedef with no id carries the UDMF spec
                // default, `-1` — *not* `0`, which is a Doom/Hexen graph
                // sentinel and, for a UDMF map, a genuine id.
                id: -1,
            }],
            sidedefs: vec![MapSidedef {
                sector: SectorIdx(0),
                x_offset: 4,
                y_offset: -8,
                upper: TextureRef::Name("-".into()),
                lower: TextureRef::Name("-".into()),
                middle: TextureRef::Name("STARTAN3".into()),
            }],
            sectors: vec![MapSector {
                floor_height: 0,
                ceiling_height: 128,
                floor_flat: TextureRef::Name("FLOOR4_8".into()),
                ceiling_flat: TextureRef::Name("CEIL3_5".into()),
                light: 160,
                special: 0,
                tag: 0,
                colors: None,
                flags: 0,
            }],
            things: vec![MapThing {
                x: 32.0,
                y: 32.0,
                angle: 90,
                type_id: 1,
                flags: 0b0111,
                id: 0,
                height: 0.0,
                special: Special {
                    special: 0,
                    args: [0; 5],
                },
            }],
            lights: vec![],
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
            game: None,
            warnings: vec![],
        }
    }

    #[test]
    fn writes_the_five_data_lumps() {
        let (lumps, warnings) = write_doom_map(&tiny_map(), &WriteOptions::strict()).unwrap();

        let vertices: Vec<crate::map::Vertex> = parse_records(&lumps.vertexes).unwrap();
        assert_eq!((vertices[0].x, vertices[1].x), (0, 64));

        let linedefs: Vec<crate::map::doom::Linedef> = parse_records(&lumps.linedefs).unwrap();
        assert_eq!(linedefs[0].start_vertex, 0);
        assert_eq!(linedefs[0].special_type, 11);
        // Doom has no args; args[0] is the sector tag.
        assert_eq!(linedefs[0].sector_tag, 7);
        // A one-sided line's absent left sidedef is the 0xFFFF sentinel.
        assert_eq!(linedefs[0].left_sidedef, 0xFFFF);

        let sidedefs: Vec<crate::map::Sidedef> = parse_records(&lumps.sidedefs).unwrap();
        assert_eq!(sidedefs[0].middle_texture.as_str_lossy(), "STARTAN3");
        assert_eq!(sidedefs[0].y_offset, -8);

        let things: Vec<crate::map::doom::Thing> = parse_records(&lumps.things).unwrap();
        assert_eq!((things[0].x, things[0].y, things[0].angle), (32, 32, 90));

        // Nodes are never built; the caller is always told so, in both modes.
        assert_eq!(warnings, vec![DoomWriteWarning::NodesNotBuilt]);
    }

    #[test]
    fn fractional_coordinate_errors_in_strict() {
        let mut map = tiny_map();
        map.vertices[0].x = 0.5;
        let err = write_doom_map(&map, &WriteOptions::strict()).unwrap_err();
        assert_eq!(
            err,
            DoomWriteError::FractionalCoordinate {
                block: "vertex",
                field: "x",
                index: 0,
                value: 0.5
            }
        );
    }

    #[test]
    fn fractional_coordinate_rounds_in_lenient() {
        let mut map = tiny_map();
        map.vertices[0].x = 0.5;
        let (lumps, warnings) = write_doom_map(&map, &WriteOptions::lenient()).unwrap();
        let vertices: Vec<crate::map::Vertex> = parse_records(&lumps.vertexes).unwrap();
        // Half away from zero.
        assert_eq!(vertices[0].x, 1);
        assert!(warnings.contains(&DoomWriteWarning::CoordinateRounded {
            block: "vertex",
            field: "x",
            index: 0,
            from: 0.5,
            to: 1
        }));
    }

    #[test]
    fn out_of_range_coordinate_clamps_in_lenient() {
        let mut map = tiny_map();
        map.vertices[0].x = 100_000.0;
        let (lumps, warnings) = write_doom_map(&map, &WriteOptions::lenient()).unwrap();
        let vertices: Vec<crate::map::Vertex> = parse_records(&lumps.vertexes).unwrap();
        assert_eq!(vertices[0].x, i16::MAX);
        assert!(warnings.iter().any(|w| matches!(
            w,
            DoomWriteWarning::ValueClamped {
                block: "vertex",
                field: "x",
                ..
            }
        )));
    }

    #[test]
    fn nonzero_linedef_args_are_unrepresentable_in_strict() {
        let mut map = tiny_map();
        map.linedefs[0].special.args = [0, 1, 0, 0, 0];
        let err = write_doom_map(&map, &WriteOptions::strict()).unwrap_err();
        assert_eq!(
            err,
            DoomWriteError::UnrepresentableField {
                block: "linedef",
                field: "arg1",
                index: 0
            }
        );
    }

    #[test]
    fn nonzero_linedef_args_are_dropped_in_lenient() {
        let mut map = tiny_map();
        map.linedefs[0].special.args = [0, 1, 0, 0, 0];
        let (lumps, warnings) = write_doom_map(&map, &WriteOptions::lenient()).unwrap();
        let linedefs: Vec<crate::map::doom::Linedef> = parse_records(&lumps.linedefs).unwrap();
        assert_eq!(linedefs[0].special_type, 11);
        assert!(warnings.contains(&DoomWriteWarning::FieldDropped {
            block: "linedef",
            field: "arg1",
            index: 0
        }));
    }

    #[test]
    fn thing_height_and_tid_are_unrepresentable_in_strict() {
        let mut map = tiny_map();
        map.things[0].height = 16.0;
        let err = write_doom_map(&map, &WriteOptions::strict()).unwrap_err();
        assert_eq!(
            err,
            DoomWriteError::UnrepresentableField {
                block: "thing",
                field: "height",
                index: 0
            }
        );

        let mut map = tiny_map();
        map.things[0].id = 42;
        let err = write_doom_map(&map, &WriteOptions::strict()).unwrap_err();
        assert_eq!(
            err,
            DoomWriteError::UnrepresentableField {
                block: "thing",
                field: "id",
                index: 0
            }
        );
    }

    #[test]
    fn long_texture_name_errors_in_strict_and_truncates_in_lenient() {
        let mut map = tiny_map();
        map.sidedefs[0].middle = TextureRef::Name("TOOLONGNAME".into());

        let err = write_doom_map(&map, &WriteOptions::strict()).unwrap_err();
        assert_eq!(
            err,
            DoomWriteError::NameTooLong {
                name: "TOOLONGNAME".into(),
                len: 11
            }
        );

        let (lumps, warnings) = write_doom_map(&map, &WriteOptions::lenient()).unwrap();
        let sidedefs: Vec<crate::map::Sidedef> = parse_records(&lumps.sidedefs).unwrap();
        assert_eq!(sidedefs[0].middle_texture.as_str_lossy(), "TOOLONGN");
        assert!(warnings.contains(&DoomWriteWarning::NameTruncated {
            name: "TOOLONGNAME".into()
        }));
    }

    /// A leftover `TextureRef::Index` (one assembly could not resolve to a
    /// name, ADR-0022 §4) is rejected in both strictness modes — there is no
    /// honest recovery (ADR-0021 §5).
    #[test]
    fn texture_index_is_rejected_in_both_modes() {
        let mut map = tiny_map();
        map.sidedefs[0].middle = TextureRef::Index(42);
        for opts in [WriteOptions::strict(), WriteOptions::lenient()] {
            let err = write_doom_map(&map, &opts).unwrap_err();
            assert!(matches!(
                err,
                DoomWriteError::UnresolvedTextureIndex {
                    block: "sidedef",
                    index: 0,
                    ..
                }
            ));
        }
    }

    /// The mirror case on the sector arena's *ceiling* field specifically:
    /// `floor_flat` resolves fine, so the failure surfaces from the
    /// `ceiling_texture` call site rather than being short-circuited by an
    /// earlier field.
    #[test]
    fn sector_ceiling_texture_index_is_rejected_in_both_modes() {
        let mut map = tiny_map();
        map.sectors[0].ceiling_flat = TextureRef::Index(9);
        for opts in [WriteOptions::strict(), WriteOptions::lenient()] {
            let err = write_doom_map(&map, &opts).unwrap_err();
            assert!(matches!(
                err,
                DoomWriteError::UnresolvedTextureIndex {
                    block: "sector",
                    field: "textureceiling",
                    index: 0,
                }
            ));
        }
    }

    /// A Doom 64-sourced map's colored lighting has no slot in the Doom
    /// format: strict refuses it (tier-3 loss, ADR-0021 §5 amendment 3),
    /// lenient drops it and warns exactly once. Texture resolution is a
    /// separate concern (ADR-0022 §4) — `tiny_map`'s refs are already
    /// `TextureRef::Name`, so this test isolates the lighting policy.
    #[test]
    fn doom64_sourced_map_lighting_is_tier3_loss() {
        let mut map = tiny_map();
        map.format = MapFormat::Doom64;

        let err = write_doom_map(&map, &WriteOptions::strict()).unwrap_err();
        assert_eq!(
            err,
            DoomWriteError::UnrepresentableField {
                block: "sector",
                field: "colors",
                index: 0
            }
        );

        let (lumps, warnings) = write_doom_map(&map, &WriteOptions::lenient()).unwrap();
        let sidedefs: Vec<crate::map::Sidedef> = parse_records(&lumps.sidedefs).unwrap();
        assert_eq!(sidedefs[0].middle_texture.as_str_lossy(), "STARTAN3");
        assert_eq!(
            warnings
                .iter()
                .filter(|w| matches!(w, DoomWriteWarning::ColoredLightingDropped))
                .count(),
            1
        );
    }

    #[test]
    fn non_finite_coordinate_errors_in_strict() {
        let mut map = tiny_map();
        map.things[0].x = f64::NAN;
        let err = write_doom_map(&map, &WriteOptions::strict()).unwrap_err();
        assert_eq!(
            err,
            DoomWriteError::NonFiniteCoordinate {
                block: "thing",
                field: "x",
                index: 0
            }
        );
    }

    #[test]
    fn too_many_vertices_errors_in_both_modes() {
        let mut map = tiny_map();
        map.vertices = vec![MapVertex { x: 0.0, y: 0.0 }; 65_537];
        for opts in [WriteOptions::strict(), WriteOptions::lenient()] {
            let err = write_doom_map(&map, &opts).unwrap_err();
            assert_eq!(
                err,
                DoomWriteError::TooManyElements {
                    kind: "vertices",
                    count: 65_537,
                    max: 65_536
                }
            );
        }
    }

    #[test]
    fn non_finite_coordinate_replaced_in_lenient() {
        let mut map = tiny_map();
        map.things[0].x = f64::NAN;
        map.vertices[1].y = f64::INFINITY;
        let (lumps, warnings) = write_doom_map(&map, &WriteOptions::lenient()).unwrap();

        let things: Vec<crate::map::doom::Thing> = parse_records(&lumps.things).unwrap();
        assert_eq!(things[0].x, 0);
        let vertices: Vec<crate::map::Vertex> = parse_records(&lumps.vertexes).unwrap();
        assert_eq!(vertices[1].y, 0);

        assert!(warnings.contains(&DoomWriteWarning::NonFiniteReplaced {
            block: "thing",
            field: "x",
            index: 0
        }));
        assert!(warnings.contains(&DoomWriteWarning::NonFiniteReplaced {
            block: "vertex",
            field: "y",
            index: 1
        }));
    }

    #[test]
    fn out_of_range_coordinate_errors_in_strict() {
        let mut map = tiny_map();
        map.vertices[0].y = -100_000.0;
        let err = write_doom_map(&map, &WriteOptions::strict()).unwrap_err();
        assert_eq!(
            err,
            DoomWriteError::ValueOutOfRange {
                block: "vertex",
                field: "y",
                index: 0,
                value: -100_000
            }
        );
    }

    #[test]
    fn out_of_range_sector_values_error_in_strict_and_clamp_in_lenient() {
        let mut map = tiny_map();
        map.sectors[0].ceiling_height = 100_000;
        let err = write_doom_map(&map, &WriteOptions::strict()).unwrap_err();
        assert_eq!(
            err,
            DoomWriteError::ValueOutOfRange {
                block: "sector",
                field: "heightceiling",
                index: 0,
                value: 100_000
            }
        );

        let (lumps, warnings) = write_doom_map(&map, &WriteOptions::lenient()).unwrap();
        let sectors: Vec<crate::map::Sector> = parse_records(&lumps.sectors).unwrap();
        assert_eq!(sectors[0].ceiling_height, i16::MAX);
        assert!(warnings.contains(&DoomWriteWarning::ValueClamped {
            block: "sector",
            field: "heightceiling",
            index: 0,
            from: 100_000,
            to: i64::from(i16::MAX)
        }));
    }

    #[test]
    fn out_of_range_sidedef_offset_clamps_in_lenient() {
        let mut map = tiny_map();
        map.sidedefs[0].x_offset = -100_000;
        let (lumps, warnings) = write_doom_map(&map, &WriteOptions::lenient()).unwrap();
        let sidedefs: Vec<crate::map::Sidedef> = parse_records(&lumps.sidedefs).unwrap();
        assert_eq!(sidedefs[0].x_offset, i16::MIN);
        assert!(warnings.contains(&DoomWriteWarning::ValueClamped {
            block: "sidedef",
            field: "offsetx",
            index: 0,
            from: -100_000,
            to: i64::from(i16::MIN)
        }));
    }

    /// A bit field truncates, it does not clamp: clamping `0x1_0001` to
    /// `0xFFFF` would turn one stray high bit into every Doom linedef flag
    /// (blocking, secret, two-sided, …). Strict still refuses the loss.
    #[test]
    fn out_of_range_linedef_flags_error_in_strict_and_truncate_in_lenient() {
        let mut map = tiny_map();
        map.linedefs[0].flags = 0x1_0001;
        let err = write_doom_map(&map, &WriteOptions::strict()).unwrap_err();
        assert_eq!(
            err,
            DoomWriteError::ValueOutOfRange {
                block: "linedef",
                field: "flags",
                index: 0,
                value: 0x1_0001
            }
        );

        let (lumps, warnings) = write_doom_map(&map, &WriteOptions::lenient()).unwrap();
        let linedefs: Vec<crate::map::doom::Linedef> = parse_records(&lumps.linedefs).unwrap();
        assert_eq!(linedefs[0].flags, 0x0001, "the low 16 bits, not 0xFFFF");
        assert!(warnings.contains(&DoomWriteWarning::ValueTruncated {
            block: "linedef",
            field: "flags",
            index: 0,
            from: 0x1_0001,
            to: 0x0001
        }));
    }

    /// The same for thing flags — the bits Doom can hold survive, the rest are
    /// dropped and reported.
    #[test]
    fn out_of_range_thing_flags_error_in_strict_and_truncate_in_lenient() {
        let mut map = tiny_map();
        map.things[0].flags = 0x8000_0007;
        let err = write_doom_map(&map, &WriteOptions::strict()).unwrap_err();
        assert_eq!(
            err,
            DoomWriteError::ValueOutOfRange {
                block: "thing",
                field: "flags",
                index: 0,
                value: 0x8000_0007
            }
        );

        let (lumps, warnings) = write_doom_map(&map, &WriteOptions::lenient()).unwrap();
        let things: Vec<crate::map::doom::Thing> = parse_records(&lumps.things).unwrap();
        assert_eq!(things[0].flags, 0x0007);
        assert!(warnings.contains(&DoomWriteWarning::ValueTruncated {
            block: "thing",
            field: "flags",
            index: 0,
            from: 0x8000_0007,
            to: 0x0007
        }));
    }

    /// Truncation is confined to the bit fields: a `special` or `tag` is a
    /// magnitude, and out-of-range magnitudes still clamp.
    #[test]
    fn linedef_special_and_tag_still_clamp() {
        let mut map = tiny_map();
        map.linedefs[0].special.special = 0x1_0001;
        let (lumps, warnings) = write_doom_map(&map, &WriteOptions::lenient()).unwrap();
        let linedefs: Vec<crate::map::doom::Linedef> = parse_records(&lumps.linedefs).unwrap();
        assert_eq!(linedefs[0].special_type, 0xFFFF);
        assert!(warnings.contains(&DoomWriteWarning::ValueClamped {
            block: "linedef",
            field: "special",
            index: 0,
            from: 0x1_0001,
            to: 0xFFFF
        }));
    }

    #[test]
    fn out_of_range_index_errors_in_strict() {
        let mut map = tiny_map();
        // A sector reference beyond Doom's u16 index space.
        map.sidedefs[0].sector = SectorIdx(0x1_0000);
        let err = write_doom_map(&map, &WriteOptions::strict()).unwrap_err();
        assert_eq!(
            err,
            DoomWriteError::ValueOutOfRange {
                block: "sidedef",
                field: "sector",
                index: 0,
                value: 0x1_0000
            }
        );
    }

    #[test]
    fn thing_special_and_args_are_dropped_in_lenient() {
        let mut map = tiny_map();
        map.things[0].special = Special {
            special: 80,
            args: [1, 2, 0, 0, 0],
        };
        map.things[0].height = 16.0;
        map.things[0].id = 42;
        let (lumps, warnings) = write_doom_map(&map, &WriteOptions::lenient()).unwrap();

        let things: Vec<crate::map::doom::Thing> = parse_records(&lumps.things).unwrap();
        // The Doom record keeps only what it has slots for.
        assert_eq!((things[0].x, things[0].y, things[0].type_id), (32, 32, 1));

        for field in ["height", "id", "special", "args"] {
            assert!(
                warnings.contains(&DoomWriteWarning::FieldDropped {
                    block: "thing",
                    field,
                    index: 0
                }),
                "{field} should have been reported as dropped"
            );
        }
    }

    #[test]
    fn linedef_id_is_unrepresentable_in_strict() {
        let mut map = tiny_map();
        map.linedefs[0].id = 3;
        let err = write_doom_map(&map, &WriteOptions::strict()).unwrap_err();
        assert_eq!(
            err,
            DoomWriteError::UnrepresentableField {
                block: "linedef",
                field: "id",
                index: 0
            }
        );
    }

    /// The graph's "no id" sentinel is source-dependent: `-1` for a UDMF map,
    /// `0` for a Doom/Hexen one. Treating a UDMF `-1` as a real id would make
    /// Doom → UDMF → Doom fail on the first linedef of every map.
    #[test]
    fn udmf_linedef_id_sentinel_is_not_data() {
        let mut map = tiny_map();
        map.format = MapFormat::Udmf;
        map.linedefs[0].id = -1;

        // Strict: the sentinel is the *absence* of an id, so this must succeed.
        let (_, warnings) = write_doom_map(&map, &WriteOptions::strict()).unwrap();
        assert_eq!(warnings, vec![DoomWriteWarning::NodesNotBuilt]);

        // Lenient: no bogus "dropped" warning for data that never existed.
        let (_, warnings) = write_doom_map(&map, &WriteOptions::lenient()).unwrap();
        assert!(
            !warnings.contains(&DoomWriteWarning::FieldDropped {
                block: "linedef",
                field: "id",
                index: 0
            }),
            "the UDMF -1 sentinel must not be reported as a dropped id: {warnings:?}"
        );
    }

    /// The mirror of the above: a *genuine* UDMF id must still be reported. The
    /// sentinel must not swallow real ids — including `id = 0`, which is a real
    /// id for a UDMF map even though it is Doom's sentinel.
    #[test]
    fn genuine_udmf_linedef_id_is_still_unrepresentable() {
        for id in [7, 0] {
            let mut map = tiny_map();
            map.format = MapFormat::Udmf;
            map.linedefs[0].id = id;

            assert_eq!(
                write_doom_map(&map, &WriteOptions::strict()).unwrap_err(),
                DoomWriteError::UnrepresentableField {
                    block: "linedef",
                    field: "id",
                    index: 0
                },
                "UDMF id = {id} is real data and must be rejected in strict mode"
            );

            let (_, warnings) = write_doom_map(&map, &WriteOptions::lenient()).unwrap();
            assert!(
                warnings.contains(&DoomWriteWarning::FieldDropped {
                    block: "linedef",
                    field: "id",
                    index: 0
                }),
                "UDMF id = {id} must be reported as dropped in lenient mode"
            );
        }
    }

    /// A Doom/Hexen-sourced map uses `0` as its "no id" sentinel, so a
    /// Doom → Doom write is clean.
    #[test]
    fn doom_linedef_id_sentinel_is_zero() {
        for format in [MapFormat::Doom, MapFormat::Hexen] {
            let mut map = tiny_map();
            map.format = format;
            map.linedefs[0].id = 0;
            let (_, warnings) = write_doom_map(&map, &WriteOptions::strict()).unwrap();
            assert_eq!(
                warnings,
                vec![DoomWriteWarning::NodesNotBuilt],
                "{format:?}"
            );
        }
    }

    /// `0xFFFF` is the largest index a Doom `u16` vertex/sector reference can
    /// hold (there is no sentinel in that space), so it must survive untouched.
    #[test]
    fn max_vertex_and_sector_index_round_trip_unclamped() {
        let mut map = tiny_map();
        map.linedefs[0].start = VertexIdx(0xFFFF);
        map.sidedefs[0].sector = SectorIdx(0xFFFF);

        let (lumps, warnings) = write_doom_map(&map, &WriteOptions::strict()).unwrap();
        let linedefs: Vec<crate::map::doom::Linedef> = parse_records(&lumps.linedefs).unwrap();
        let sidedefs: Vec<crate::map::Sidedef> = parse_records(&lumps.sidedefs).unwrap();

        assert_eq!(linedefs[0].start_vertex, 0xFFFF);
        assert_eq!(sidedefs[0].sector, 0xFFFF);
        // Neither was clamped nor rejected.
        assert_eq!(warnings, vec![DoomWriteWarning::NodesNotBuilt]);
    }

    /// A sidedef reference gets one fewer: `0xFFFF` is the "no sidedef"
    /// sentinel, so a real index must never reach it. `0xFFFE` is the boundary.
    #[test]
    fn sidedef_index_stops_one_below_the_no_sidedef_sentinel() {
        // 0xFFFE is legal and untouched, on both the front and back reference.
        let mut map = tiny_map();
        map.linedefs[0].right = Some(SidedefIdx(0xFFFE));
        map.linedefs[0].left = Some(SidedefIdx(0xFFFE));
        let (lumps, warnings) = write_doom_map(&map, &WriteOptions::strict()).unwrap();
        let linedefs: Vec<crate::map::doom::Linedef> = parse_records(&lumps.linedefs).unwrap();
        assert_eq!(linedefs[0].right_sidedef, 0xFFFE);
        assert_eq!(linedefs[0].left_sidedef, 0xFFFE);
        assert_ne!(linedefs[0].left_sidedef, NO_SIDEDEF);
        assert_eq!(warnings, vec![DoomWriteWarning::NodesNotBuilt]);

        // 0xFFFF would collide with the sentinel: strict rejects it...
        let mut map = tiny_map();
        map.linedefs[0].right = Some(SidedefIdx(0xFFFF));
        assert_eq!(
            write_doom_map(&map, &WriteOptions::strict()).unwrap_err(),
            DoomWriteError::ValueOutOfRange {
                block: "linedef",
                field: "sidefront",
                index: 0,
                value: 0xFFFF
            }
        );

        // ...and lenient clamps it to 0xFFFE, never emitting the sentinel as a
        // real reference.
        let (lumps, warnings) = write_doom_map(&map, &WriteOptions::lenient()).unwrap();
        let linedefs: Vec<crate::map::doom::Linedef> = parse_records(&lumps.linedefs).unwrap();
        assert_eq!(linedefs[0].right_sidedef, 0xFFFE);
        assert_ne!(linedefs[0].right_sidedef, NO_SIDEDEF);
        assert!(warnings.contains(&DoomWriteWarning::ValueClamped {
            block: "linedef",
            field: "sidefront",
            index: 0,
            from: 0xFFFF,
            to: 0xFFFE
        }));
    }

    /// A frontless linedef (`right: None`; ADR-0020) serializes as
    /// `NO_SIDEDEF` on the front field — symmetric with the back — with no
    /// error or clamp in either mode, so a frontless binary map round-trips
    /// losslessly.
    #[test]
    fn frontless_linedef_writes_the_sentinel() {
        let mut map = tiny_map();
        map.linedefs[0].right = None;
        map.linedefs[0].left = None;
        for opts in [WriteOptions::strict(), WriteOptions::lenient()] {
            let (lumps, warnings) = write_doom_map(&map, &opts).unwrap();
            let linedefs: Vec<crate::map::doom::Linedef> = parse_records(&lumps.linedefs).unwrap();
            assert_eq!(linedefs[0].right_sidedef, NO_SIDEDEF);
            assert_eq!(linedefs[0].left_sidedef, NO_SIDEDEF);
            assert_eq!(warnings, vec![DoomWriteWarning::NodesNotBuilt]);
        }
    }

    /// A vertex/sector reference is capped at `0xFFFF`, one *above* the sidedef
    /// cap — the two bounds are genuinely different, not a copy of each other.
    #[test]
    fn vertex_index_above_the_sidedef_cap_is_still_accepted() {
        let mut map = tiny_map();
        // 0xFFFF is rejected for a sidedef reference but legal for a vertex one.
        map.linedefs[0].end = VertexIdx(usize::try_from(MAX_SIDEDEF_INDEX).unwrap() + 1);
        let (lumps, _) = write_doom_map(&map, &WriteOptions::strict()).unwrap();
        let linedefs: Vec<crate::map::doom::Linedef> = parse_records(&lumps.linedefs).unwrap();
        assert_eq!(i64::from(linedefs[0].end_vertex), MAX_INDEX);
    }

    /// A tiny fractional value must not be silently swallowed by an epsilon
    /// comparison: `1e-16` is not a whole number, so strict must reject it and
    /// lenient must warn — rounding it to `0` unannounced is silent data loss.
    #[test]
    fn tiny_fractional_coordinate_is_not_swallowed() {
        let mut map = tiny_map();
        map.vertices[0].x = 1e-16;

        assert_eq!(
            write_doom_map(&map, &WriteOptions::strict()).unwrap_err(),
            DoomWriteError::FractionalCoordinate {
                block: "vertex",
                field: "x",
                index: 0,
                value: 1e-16
            }
        );

        let (lumps, warnings) = write_doom_map(&map, &WriteOptions::lenient()).unwrap();
        let vertices: Vec<crate::map::Vertex> = parse_records(&lumps.vertexes).unwrap();
        assert_eq!(vertices[0].x, 0);
        assert!(warnings.contains(&DoomWriteWarning::CoordinateRounded {
            block: "vertex",
            field: "x",
            index: 0,
            from: 1e-16,
            to: 0
        }));
    }

    #[test]
    fn two_sided_linedef_writes_a_real_back_sidedef_index() {
        let mut map = tiny_map();
        map.sidedefs.push(map.sidedefs[0].clone());
        map.linedefs[0].left = Some(SidedefIdx(1));
        let (lumps, _) = write_doom_map(&map, &WriteOptions::strict()).unwrap();
        let linedefs: Vec<crate::map::doom::Linedef> = parse_records(&lumps.linedefs).unwrap();
        assert_eq!(linedefs[0].left_sidedef, 1);
        // A real index can never be the 0xFFFF "no sidedef" sentinel; the
        // boundary itself is exercised by
        // `sidedef_index_stops_one_below_the_no_sidedef_sentinel`.
        assert_ne!(linedefs[0].left_sidedef, NO_SIDEDEF);
    }

    #[test]
    fn too_many_sectors_and_sidedefs_error_in_both_modes() {
        for opts in [WriteOptions::strict(), WriteOptions::lenient()] {
            let mut map = tiny_map();
            map.sectors = vec![map.sectors[0].clone(); MAX_INDEXED + 1];
            assert_eq!(
                write_doom_map(&map, &opts).unwrap_err(),
                DoomWriteError::TooManyElements {
                    kind: "sectors",
                    count: MAX_INDEXED + 1,
                    max: MAX_INDEXED
                }
            );

            let mut map = tiny_map();
            map.sidedefs = vec![map.sidedefs[0].clone(); MAX_SIDEDEFS + 1];
            assert_eq!(
                write_doom_map(&map, &opts).unwrap_err(),
                DoomWriteError::TooManyElements {
                    kind: "sidedefs",
                    count: MAX_SIDEDEFS + 1,
                    max: MAX_SIDEDEFS
                }
            );
        }
    }

    #[test]
    fn error_messages_are_human_readable() {
        assert_eq!(
            DoomWriteError::TooManyElements {
                kind: "vertices",
                count: 70_000,
                max: MAX_INDEXED
            }
            .to_string(),
            "vertices count 70000 exceeds the Doom format maximum of 65536"
        );
        assert_eq!(
            DoomWriteError::NonFiniteCoordinate {
                block: "thing",
                field: "x",
                index: 1
            }
            .to_string(),
            "non-finite x in thing #1"
        );
        assert_eq!(
            DoomWriteError::FractionalCoordinate {
                block: "vertex",
                field: "y",
                index: 2,
                value: 0.5
            }
            .to_string(),
            "fractional y 0.5 in vertex #2 cannot be stored as an i16"
        );
        assert_eq!(
            DoomWriteError::ValueOutOfRange {
                block: "sector",
                field: "id",
                index: 3,
                value: 99_999
            }
            .to_string(),
            "id value 99999 in sector #3 is out of range for the Doom format"
        );
        assert_eq!(
            DoomWriteError::UnrepresentableField {
                block: "thing",
                field: "height",
                index: 4
            }
            .to_string(),
            "thing #4 has a height value, which the Doom format cannot represent"
        );
        assert_eq!(
            DoomWriteError::NameTooLong {
                name: "TOOLONGNAME".into(),
                len: 11
            }
            .to_string(),
            "name \"TOOLONGNAME\" is 11 bytes; Doom names are at most 8 bytes"
        );
        assert_eq!(
            DoomWriteError::UnresolvedTextureIndex {
                block: "sidedef",
                field: "texturetop",
                index: 0
            }
            .to_string(),
            "unresolvable texture index for texturetop in sidedef #0"
        );
    }

    #[test]
    fn warning_messages_are_human_readable() {
        assert_eq!(
            DoomWriteWarning::NodesNotBuilt.to_string(),
            "node lumps (SEGS/SSECTORS/NODES/REJECT/BLOCKMAP) were not built; run a nodebuilder"
        );
        assert_eq!(
            DoomWriteWarning::NonFiniteReplaced {
                block: "vertex",
                field: "x",
                index: 0
            }
            .to_string(),
            "non-finite x in vertex #0 replaced with 0"
        );
        assert_eq!(
            DoomWriteWarning::CoordinateRounded {
                block: "vertex",
                field: "x",
                index: 0,
                from: 0.5,
                to: 1
            }
            .to_string(),
            "fractional x 0.5 in vertex #0 rounded to 1"
        );
        assert_eq!(
            DoomWriteWarning::ValueClamped {
                block: "sector",
                field: "lightlevel",
                index: 0,
                from: 99_999,
                to: 32_767
            }
            .to_string(),
            "lightlevel value 99999 in sector #0 clamped to 32767"
        );
        assert_eq!(
            DoomWriteWarning::ValueTruncated {
                block: "linedef",
                field: "flags",
                index: 0,
                from: 0x1_0001,
                to: 0x0001
            }
            .to_string(),
            "flags value 65537 in linedef #0 truncated to 1"
        );
        assert_eq!(
            DoomWriteWarning::FieldDropped {
                block: "linedef",
                field: "arg1",
                index: 0
            }
            .to_string(),
            "arg1 on linedef #0 was dropped (not representable in the Doom format)"
        );
        assert_eq!(
            DoomWriteWarning::NameTruncated {
                name: "TOOLONGNAME".into()
            }
            .to_string(),
            "name \"TOOLONGNAME\" was truncated to 8 bytes"
        );
        assert_eq!(
            DoomWriteWarning::ColoredLightingDropped.to_string(),
            "the map's Doom 64 colored lighting (sector color references and lights table) has no Doom slot and was dropped"
        );
    }

    #[test]
    fn add_doom_map_propagates_write_errors() {
        use crate::WadKind;
        let mut map = tiny_map();
        map.vertices[0].x = 0.5;
        let mut builder = WadBuilder::new(WadKind::Pwad);
        let err = add_doom_map(&mut builder, "MAP01", &map, &WriteOptions::strict()).unwrap_err();
        assert_eq!(
            err,
            DoomWriteError::FractionalCoordinate {
                block: "vertex",
                field: "x",
                index: 0,
                value: 0.5
            }
        );
    }

    #[test]
    fn add_doom_map_emits_marker_data_and_empty_node_lumps() {
        use crate::{Wad, WadKind};
        let mut builder = WadBuilder::new(WadKind::Pwad);
        let warnings =
            add_doom_map(&mut builder, "MAP01", &tiny_map(), &WriteOptions::strict()).unwrap();
        assert_eq!(warnings, vec![DoomWriteWarning::NodesNotBuilt]);

        let wad = Wad::from_bytes(builder.build().unwrap()).unwrap();
        let names: Vec<&str> = wad.lumps().iter().map(crate::Lump::name).collect();
        assert_eq!(
            names,
            vec![
                "MAP01", "THINGS", "LINEDEFS", "SIDEDEFS", "VERTEXES", "SEGS", "SSECTORS", "NODES",
                "SECTORS", "REJECT", "BLOCKMAP"
            ]
        );
        // The node lumps are placeholders: present, canonical order, zero length.
        for name in ["SEGS", "SSECTORS", "NODES", "REJECT", "BLOCKMAP"] {
            let lump = wad.lumps().iter().find(|l| l.name() == name).unwrap();
            assert_eq!(lump.size(), 0, "{name} must be empty");
        }
    }
}
