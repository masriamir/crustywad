//! Iterative, depth-bounded UDMF parser engine.
//!
//! [`parse_udmf`] drives [`Lexer`] tokens through a flat two-state machine
//! (`ExpectGlobalItem` / `ExpectBlockItem`) tracked by an explicit `depth`
//! counter. There is no recursion anywhere in this module (ADR-0016).

use crate::Limits;

use super::UdmfParseError;
use super::lex::{Lexer, Spanned, Token};
use super::model::{
    UdmfAssignment, UdmfLinedef, UdmfMap, UdmfSector, UdmfSidedef, UdmfThing, UdmfUnknownBlock,
    UdmfValue, UdmfVertex,
};

/// Parses UDMF `TEXTMAP` text into a typed, un-normalized [`UdmfMap`].
///
/// This is a lexical + grammatical + per-field-default pass only; it does not
/// resolve vertex/sidedef/sector cross-references (that is map assembly, PR B).
/// `limits.max_depth` bounds block nesting (defense-in-depth; legal UDMF never
/// nests beyond depth 1).
///
/// # Errors
/// Returns [`UdmfParseError`] if `text` is not valid UDMF syntax, if nesting
/// exceeds `limits.max_depth`, if a block omits a field with no valid spec
/// default, or if the document lacks a `namespace` declaration.
pub fn parse_udmf(text: &str, limits: Limits) -> Result<UdmfMap, UdmfParseError> {
    let mut lexer = Lexer::new(text);
    let mut state = State::ExpectGlobalItem;
    let mut depth: usize = 0;
    // Initial value covers the zero-token (empty-string) case; every other
    // path overwrites it before it is read.
    let mut last_pos: (usize, usize) = (1, 1);

    let mut namespace: Option<String> = None;
    let mut global_extras = ExtrasBuilder::default();
    let mut collections = Collections::default();
    let mut current_block: Option<BlockState> = None;

    while let Some(spanned) = lexer.next_spanned()? {
        last_pos = (spanned.line, spanned.column);

        match state {
            State::ExpectGlobalItem => match spanned.token {
                Token::Ident(name) => {
                    let next = require_next(&mut lexer, last_pos)?;
                    last_pos = (next.line, next.column);
                    match next.token {
                        Token::Equals => {
                            let value = read_value(&mut lexer, &mut last_pos)?;
                            expect_semicolon(&mut lexer, &mut last_pos)?;
                            if name == "namespace" {
                                namespace = Some(as_str(&value)?);
                            } else {
                                global_extras.set(&name, to_value(&value)?);
                            }
                        }
                        Token::LBrace => {
                            let new_depth = depth + 1;
                            if new_depth > limits.max_depth {
                                return Err(UdmfParseError::DepthExceeded {
                                    max_depth: limits.max_depth,
                                    line: next.line,
                                    column: next.column,
                                });
                            }
                            depth = new_depth;
                            current_block = Some(BlockState::new(name));
                            state = State::ExpectBlockItem;
                        }
                        _ => {
                            return Err(syntax_error(
                                next.line,
                                next.column,
                                "expected '=' or '{' after identifier",
                            ));
                        }
                    }
                }
                _ => {
                    return Err(syntax_error(
                        spanned.line,
                        spanned.column,
                        "expected an identifier at file scope",
                    ));
                }
            },
            State::ExpectBlockItem => match spanned.token {
                Token::Ident(name) => {
                    let eq = require_next(&mut lexer, last_pos)?;
                    last_pos = (eq.line, eq.column);
                    if !matches!(eq.token, Token::Equals) {
                        return Err(syntax_error(
                            eq.line,
                            eq.column,
                            "expected '=' after field name",
                        ));
                    }
                    let value = read_value(&mut lexer, &mut last_pos)?;
                    expect_semicolon(&mut lexer, &mut last_pos)?;
                    if let Some(block) = current_block.as_mut() {
                        block.set_field(&name, &value)?;
                    }
                }
                Token::RBrace => {
                    depth = depth.saturating_sub(1);
                    state = State::ExpectGlobalItem;
                    if let Some(block) = current_block.take() {
                        collections.push(block.finish()?);
                    }
                }
                _ => {
                    return Err(syntax_error(
                        spanned.line,
                        spanned.column,
                        "expected a field name or '}' inside a block",
                    ));
                }
            },
        }
    }

    check_block_closed(current_block.is_some(), last_pos)?;

    let namespace = namespace.ok_or_else(|| UdmfParseError::Semantic {
        message: "TEXTMAP is missing a required 'namespace' declaration".to_owned(),
    })?;

    Ok(collections.into_map(namespace, global_extras.finish()))
}

/// The two states of the flat parser loop, keyed by `depth`.
enum State {
    /// At file scope (`depth == 0`): expects a global assignment or a block
    /// header.
    ExpectGlobalItem,
    /// Inside a block (`depth == 1`): expects a field assignment or `}`.
    ExpectBlockItem,
}

/// Per-block accumulator, built from spec defaults and overwritten by
/// recognized field assignments. Unknown blocks accumulate their header name
/// and every field verbatim for lossless round-trip (ADR-0027).
enum BlockState {
    /// A `vertex` block.
    Vertex(VertexBuilder),
    /// A `linedef` block.
    Linedef(LinedefBuilder),
    /// A `sidedef` block.
    Sidedef(SidedefBuilder),
    /// A `sector` block.
    Sector(SectorBuilder),
    /// A `thing` block.
    Thing(ThingBuilder),
    /// Any block header not otherwise recognized; its header name and fields
    /// are retained verbatim (ADR-0027).
    Unknown {
        /// The block header identifier, folded to ASCII lowercase.
        name: String,
        /// The block's assignments, retained in first-assignment order.
        fields: ExtrasBuilder,
    },
}

/// The typed element produced by [`BlockState::finish`], or an
/// [`UdmfUnknownBlock`] retaining an unrecognized block header verbatim.
enum BlockResult {
    /// A finished `vertex` block.
    Vertex(UdmfVertex),
    /// A finished `linedef` block.
    Linedef(UdmfLinedef),
    /// A finished `sidedef` block.
    Sidedef(UdmfSidedef),
    /// A finished `sector` block.
    Sector(UdmfSector),
    /// A finished `thing` block.
    Thing(UdmfThing),
    /// A retained unrecognized block, in declaration order (ADR-0027).
    Unknown(UdmfUnknownBlock),
}

/// Accumulates finished blocks into their typed collections, kept separate
/// from the parser's locals so `parse_udmf` stays under the pedantic
/// line-count lint.
#[derive(Default)]
struct Collections {
    /// The map's vertices, in declaration order.
    vertices: Vec<UdmfVertex>,
    /// The map's linedefs, in declaration order.
    linedefs: Vec<UdmfLinedef>,
    /// The map's sidedefs, in declaration order.
    sidedefs: Vec<UdmfSidedef>,
    /// The map's sectors, in declaration order.
    sectors: Vec<UdmfSector>,
    /// The map's things, in declaration order.
    things: Vec<UdmfThing>,
    /// Unrecognized blocks, in declaration order (ADR-0027).
    unknown_blocks: Vec<UdmfUnknownBlock>,
}

impl Collections {
    /// Routes a finished block into its typed collection; unrecognized
    /// blocks (`BlockResult::Unknown`) are retained in `unknown_blocks`.
    fn push(&mut self, result: BlockResult) {
        match result {
            BlockResult::Vertex(v) => self.vertices.push(v),
            BlockResult::Linedef(l) => self.linedefs.push(l),
            BlockResult::Sidedef(s) => self.sidedefs.push(s),
            BlockResult::Sector(s) => self.sectors.push(s),
            BlockResult::Thing(t) => self.things.push(t),
            BlockResult::Unknown(b) => self.unknown_blocks.push(b),
        }
    }

    /// Consumes the accumulated collections into a finished [`UdmfMap`].
    fn into_map(self, namespace: String, global_extras: Vec<UdmfAssignment>) -> UdmfMap {
        UdmfMap {
            namespace,
            vertices: self.vertices,
            linedefs: self.linedefs,
            sidedefs: self.sidedefs,
            sectors: self.sectors,
            things: self.things,
            unknown_blocks: self.unknown_blocks,
            global_extras,
        }
    }
}

impl BlockState {
    /// Selects the accumulator for a block header identifier. Takes the header
    /// by value so an unrecognized header is retained without a clone (the
    /// recognized arms simply drop it); the lexer has already lowercased it.
    fn new(header: String) -> Self {
        match header.as_str() {
            "vertex" => BlockState::Vertex(VertexBuilder::default()),
            "linedef" => BlockState::Linedef(LinedefBuilder::default()),
            "sidedef" => BlockState::Sidedef(SidedefBuilder::default()),
            "sector" => BlockState::Sector(SectorBuilder::default()),
            "thing" => BlockState::Thing(ThingBuilder::default()),
            _ => BlockState::Unknown {
                name: header,
                fields: ExtrasBuilder::default(),
            },
        }
    }

    /// Applies a recognized field assignment; unknown fields are retained in
    /// the block's `extras`.
    fn set_field(&mut self, name: &str, value: &Spanned) -> Result<(), UdmfParseError> {
        match self {
            BlockState::Vertex(builder) => builder.set_field(name, value)?,
            BlockState::Linedef(builder) => builder.set_field(name, value)?,
            BlockState::Sidedef(builder) => builder.set_field(name, value)?,
            BlockState::Sector(builder) => builder.set_field(name, value)?,
            BlockState::Thing(builder) => builder.set_field(name, value)?,
            BlockState::Unknown { fields, .. } => fields.set(name, to_value(value)?),
        }
        Ok(())
    }

    /// Finalizes the block at `}`, returning the assembled element (if any)
    /// or a [`UdmfParseError::Semantic`] if a required field was never set.
    fn finish(self) -> Result<BlockResult, UdmfParseError> {
        match self {
            BlockState::Vertex(builder) => Ok(BlockResult::Vertex(builder.finish()?)),
            BlockState::Linedef(builder) => Ok(BlockResult::Linedef(builder.finish()?)),
            BlockState::Sidedef(builder) => Ok(BlockResult::Sidedef(builder.finish()?)),
            BlockState::Sector(builder) => Ok(BlockResult::Sector(builder.finish()?)),
            BlockState::Thing(builder) => Ok(BlockResult::Thing(builder.finish()?)),
            BlockState::Unknown { name, fields } => Ok(BlockResult::Unknown(UdmfUnknownBlock {
                name,
                fields: fields.finish(),
            })),
        }
    }
}

/// Returns a [`UdmfParseError::Semantic`] naming a missing required field.
fn missing_field(block: &str, field: &str) -> UdmfParseError {
    UdmfParseError::Semantic {
        message: format!("{block} block is missing required field '{field}'"),
    }
}

/// Sets or clears bit `bit` of `flags` depending on `value`.
fn set_flag_bit(flags: &mut u32, bit: u32, value: bool) {
    if value {
        *flags |= 1 << bit;
    } else {
        *flags &= !(1 << bit);
    }
}

/// Converts a value token (post-`read_value`, so one of the four value
/// shapes) to its retained [`UdmfValue`].
///
/// # Errors
/// Returns [`UdmfParseError::Syntax`] if `spanned` is not a value literal;
/// `read_value` already rejects non-literal tokens, so this arm is defensive.
fn to_value(spanned: &Spanned) -> Result<UdmfValue, UdmfParseError> {
    match &spanned.token {
        Token::Bool(b) => Ok(UdmfValue::Bool(*b)),
        Token::Int(i) => Ok(UdmfValue::Int(*i)),
        Token::Float(f) => Ok(UdmfValue::Float(*f)),
        Token::Str(s) => Ok(UdmfValue::Str(s.clone())),
        _ => Err(syntax_error(
            spanned.line,
            spanned.column,
            "expected a value literal",
        )),
    }
}

/// Ordered extras accumulator: first-assignment position, last-assignment
/// value, O(1) per duplicate (a linear scan would be quadratic on
/// adversarial many-field blocks, violating ADR-0016's bounded-work rule).
#[derive(Default)]
struct ExtrasBuilder {
    /// Retained assignments in first-assignment order.
    entries: Vec<UdmfAssignment>,
    /// Field name -> index into `entries`.
    index: std::collections::HashMap<String, usize>,
}

impl ExtrasBuilder {
    /// Records `name = value`, overwriting an earlier value in place.
    fn set(&mut self, name: &str, value: UdmfValue) {
        if let Some(&i) = self.index.get(name) {
            self.entries[i].value = value;
        } else {
            self.index.insert(name.to_owned(), self.entries.len());
            self.entries.push(UdmfAssignment {
                name: name.to_owned(),
                value,
            });
        }
    }

    /// Consumes the accumulator into the retained, ordered assignment list.
    fn finish(self) -> Vec<UdmfAssignment> {
        self.entries
    }
}

/// Accumulator for a `vertex` block; both coordinate fields are required (no
/// spec default).
#[derive(Default)]
struct VertexBuilder {
    /// The `x` field, if assigned.
    x: Option<f64>,
    /// The `y` field, if assigned.
    y: Option<f64>,
    /// Assignments other than `x`/`y`, retained in `extras`.
    extras: ExtrasBuilder,
}

impl VertexBuilder {
    /// Applies a recognized field assignment; unknown fields are retained in
    /// `extras`.
    fn set_field(&mut self, name: &str, value: &Spanned) -> Result<(), UdmfParseError> {
        match name {
            "x" => self.x = Some(as_f64(value)?),
            "y" => self.y = Some(as_f64(value)?),
            _ => self.extras.set(name, to_value(value)?),
        }
        Ok(())
    }

    /// Finalizes the block, erroring if `x`/`y` were never assigned.
    fn finish(self) -> Result<UdmfVertex, UdmfParseError> {
        let x = self.x.ok_or_else(|| missing_field("vertex", "x"))?;
        let y = self.y.ok_or_else(|| missing_field("vertex", "y"))?;
        Ok(UdmfVertex {
            x,
            y,
            extras: self.extras.finish(),
        })
    }
}

/// Accumulator for a `linedef` block, seeded with spec defaults.
struct LinedefBuilder {
    /// The `v1` field, if assigned (required; no default).
    v1: Option<i32>,
    /// The `v2` field, if assigned (required; no default).
    v2: Option<i32>,
    /// The `sidefront` field, if assigned (required; no default).
    sidefront: Option<i32>,
    /// The `sideback` field (default -1).
    sideback: i32,
    /// The `id` field (default -1).
    id: i32,
    /// The `special` field (default 0).
    special: i32,
    /// The `arg0..arg4` fields (default 0 each).
    args: [i32; 5],
    /// The nine Doom-mapped boolean fields, packed into bits 0–8.
    flags: u32,
    /// Assignments not held by a typed field, retained in `extras`.
    extras: ExtrasBuilder,
}

impl Default for LinedefBuilder {
    fn default() -> Self {
        LinedefBuilder {
            v1: None,
            v2: None,
            sidefront: None,
            sideback: -1,
            id: -1,
            special: 0,
            args: [0; 5],
            flags: 0,
            extras: ExtrasBuilder::default(),
        }
    }
}

impl LinedefBuilder {
    /// Applies a recognized field assignment; unmodeled recognized fields
    /// (activation/Strife booleans) and unknown fields are retained in
    /// `extras`.
    fn set_field(&mut self, name: &str, value: &Spanned) -> Result<(), UdmfParseError> {
        match name {
            "v1" => self.v1 = Some(as_i32(value)?),
            "v2" => self.v2 = Some(as_i32(value)?),
            "sidefront" => self.sidefront = Some(as_i32(value)?),
            "sideback" => self.sideback = as_i32(value)?,
            "id" => self.id = as_i32(value)?,
            "special" => self.special = as_i32(value)?,
            "arg0" => self.args[0] = as_i32(value)?,
            "arg1" => self.args[1] = as_i32(value)?,
            "arg2" => self.args[2] = as_i32(value)?,
            "arg3" => self.args[3] = as_i32(value)?,
            "arg4" => self.args[4] = as_i32(value)?,
            "blocking" => set_flag_bit(&mut self.flags, 0, as_bool(value)?),
            "blockmonsters" => set_flag_bit(&mut self.flags, 1, as_bool(value)?),
            "twosided" => set_flag_bit(&mut self.flags, 2, as_bool(value)?),
            "dontpegtop" => set_flag_bit(&mut self.flags, 3, as_bool(value)?),
            "dontpegbottom" => set_flag_bit(&mut self.flags, 4, as_bool(value)?),
            "secret" => set_flag_bit(&mut self.flags, 5, as_bool(value)?),
            "blocksound" => set_flag_bit(&mut self.flags, 6, as_bool(value)?),
            "dontdraw" => set_flag_bit(&mut self.flags, 7, as_bool(value)?),
            "mapped" => set_flag_bit(&mut self.flags, 8, as_bool(value)?),
            _ => self.extras.set(name, to_value(value)?),
        }
        Ok(())
    }

    /// Finalizes the block, applying the `sideback` -1 -> `None` normalization.
    fn finish(self) -> Result<UdmfLinedef, UdmfParseError> {
        let v1 = self.v1.ok_or_else(|| missing_field("linedef", "v1"))?;
        let v2 = self.v2.ok_or_else(|| missing_field("linedef", "v2"))?;
        let sidefront = self
            .sidefront
            .ok_or_else(|| missing_field("linedef", "sidefront"))?;
        Ok(UdmfLinedef {
            v1,
            v2,
            sidefront,
            sideback: if self.sideback == -1 {
                None
            } else {
                Some(self.sideback)
            },
            id: self.id,
            special: self.special,
            args: self.args,
            flags: self.flags,
            extras: self.extras.finish(),
        })
    }
}

/// Accumulator for a `sidedef` block, seeded with spec defaults.
struct SidedefBuilder {
    /// The `offsetx` field (default 0).
    offsetx: i32,
    /// The `offsety` field (default 0).
    offsety: i32,
    /// The `texturetop` field (default `"-"`).
    texturetop: String,
    /// The `texturebottom` field (default `"-"`).
    texturebottom: String,
    /// The `texturemiddle` field (default `"-"`).
    texturemiddle: String,
    /// The `sector` field, if assigned (required; no default).
    sector: Option<i32>,
    /// Assignments not held by a typed field, retained in `extras`.
    extras: ExtrasBuilder,
}

impl Default for SidedefBuilder {
    fn default() -> Self {
        SidedefBuilder {
            offsetx: 0,
            offsety: 0,
            texturetop: "-".to_owned(),
            texturebottom: "-".to_owned(),
            texturemiddle: "-".to_owned(),
            sector: None,
            extras: ExtrasBuilder::default(),
        }
    }
}

impl SidedefBuilder {
    /// Applies a recognized field assignment; unknown fields are retained in
    /// `extras`.
    fn set_field(&mut self, name: &str, value: &Spanned) -> Result<(), UdmfParseError> {
        match name {
            "offsetx" => self.offsetx = as_i32(value)?,
            "offsety" => self.offsety = as_i32(value)?,
            "texturetop" => self.texturetop = as_str(value)?,
            "texturebottom" => self.texturebottom = as_str(value)?,
            "texturemiddle" => self.texturemiddle = as_str(value)?,
            "sector" => self.sector = Some(as_i32(value)?),
            _ => self.extras.set(name, to_value(value)?),
        }
        Ok(())
    }

    /// Finalizes the block, erroring if `sector` was never assigned.
    fn finish(self) -> Result<UdmfSidedef, UdmfParseError> {
        let sector = self
            .sector
            .ok_or_else(|| missing_field("sidedef", "sector"))?;
        Ok(UdmfSidedef {
            offsetx: self.offsetx,
            offsety: self.offsety,
            texturetop: self.texturetop,
            texturebottom: self.texturebottom,
            texturemiddle: self.texturemiddle,
            sector,
            extras: self.extras.finish(),
        })
    }
}

/// Accumulator for a `sector` block, seeded with spec defaults.
struct SectorBuilder {
    /// The `heightfloor` field (default 0).
    heightfloor: i32,
    /// The `heightceiling` field (default 0).
    heightceiling: i32,
    /// The `texturefloor` field, if assigned (required; no default).
    texturefloor: Option<String>,
    /// The `textureceiling` field, if assigned (required; no default).
    textureceiling: Option<String>,
    /// The `lightlevel` field (default 160).
    lightlevel: i32,
    /// The `special` field (default 0).
    special: i32,
    /// The `id` field (default 0).
    id: i32,
    /// Assignments not held by a typed field, retained in `extras`.
    extras: ExtrasBuilder,
}

impl Default for SectorBuilder {
    fn default() -> Self {
        SectorBuilder {
            heightfloor: 0,
            heightceiling: 0,
            texturefloor: None,
            textureceiling: None,
            lightlevel: 160,
            special: 0,
            id: 0,
            extras: ExtrasBuilder::default(),
        }
    }
}

impl SectorBuilder {
    /// Applies a recognized field assignment; unknown fields are retained in
    /// `extras`.
    fn set_field(&mut self, name: &str, value: &Spanned) -> Result<(), UdmfParseError> {
        match name {
            "heightfloor" => self.heightfloor = as_i32(value)?,
            "heightceiling" => self.heightceiling = as_i32(value)?,
            "texturefloor" => self.texturefloor = Some(as_str(value)?),
            "textureceiling" => self.textureceiling = Some(as_str(value)?),
            "lightlevel" => self.lightlevel = as_i32(value)?,
            "special" => self.special = as_i32(value)?,
            "id" => self.id = as_i32(value)?,
            _ => self.extras.set(name, to_value(value)?),
        }
        Ok(())
    }

    /// Finalizes the block, erroring if `texturefloor`/`textureceiling` were
    /// never assigned.
    fn finish(self) -> Result<UdmfSector, UdmfParseError> {
        let texturefloor = self
            .texturefloor
            .ok_or_else(|| missing_field("sector", "texturefloor"))?;
        let textureceiling = self
            .textureceiling
            .ok_or_else(|| missing_field("sector", "textureceiling"))?;
        Ok(UdmfSector {
            heightfloor: self.heightfloor,
            heightceiling: self.heightceiling,
            texturefloor,
            textureceiling,
            lightlevel: self.lightlevel,
            special: self.special,
            id: self.id,
            extras: self.extras.finish(),
        })
    }
}

/// Accumulator for a `thing` block, seeded with spec defaults.
///
/// The ten skill/multiplayer booleans are collected as plain fields rather
/// than packed incrementally into `flags`: `skill1`/`skill2` and
/// `skill4`/`skill5` must be OR-folded into a single Doom bit each at
/// [`finish`][ThingBuilder::finish], which an incremental `set_flag_bit` call
/// per field cannot express (a later `false` would clear a bit set by an
/// earlier `true` in the same pair).
///
/// **Every boolean defaults to `false`**, per the UDMF spec: *"All flags
/// default to false."* The spec's separate note that *"suggested editor
/// defaults for all skill, gamemode, and player class flags is true rather
/// than the UDMF default of false"* is a hint for what an editor pre-fills in
/// its UI — it is **not** parse semantics, and must not be read as one.
///
/// The ten booleans are dual-stored (ADR-0027): each assignment both updates
/// its typed field (for the `flags` fold below) and is recorded verbatim in
/// `extras` via [`ExtrasBuilder::set`]. `flags` is a derived Doom-engine
/// projection of the recognized booleans; `extras` is the authoritative,
/// round-trippable record of what the UDMF text actually assigned.
#[allow(clippy::struct_excessive_bools)]
#[derive(Default)]
struct ThingBuilder {
    /// The `x` field, if assigned (required; no default).
    x: Option<f64>,
    /// The `y` field, if assigned (required; no default).
    y: Option<f64>,
    /// The `height` field (default 0).
    height: f64,
    /// The `angle` field, raw degrees (default 0).
    angle: i32,
    /// The `type` field, if assigned (required; no default).
    type_id: Option<i32>,
    /// The `id` field (default 0).
    id: i32,
    /// The `special` field (default 0).
    special: i32,
    /// The `arg0..arg4` fields (default 0 each).
    args: [i32; 5],
    /// The `skill1` field (default false).
    skill1: bool,
    /// The `skill2` field (default false).
    skill2: bool,
    /// The `skill3` field (default false).
    skill3: bool,
    /// The `skill4` field (default false).
    skill4: bool,
    /// The `skill5` field (default false).
    skill5: bool,
    /// The `ambush` field (default false).
    ambush: bool,
    /// The `single` field (default false).
    single: bool,
    /// The `dm` field (default false).
    dm: bool,
    /// The `coop` field (default false).
    coop: bool,
    /// The `friend` field (default false).
    friend: bool,
    /// Assignments not held by a typed field, retained in `extras`.
    extras: ExtrasBuilder,
}

impl ThingBuilder {
    /// Applies a recognized field assignment. The skill/multiplayer booleans are
    /// packed into `flags` at [`finish`][Self::finish] AND recorded verbatim in
    /// `extras` (ADR-0027 dual-store: `flags` is derived, `extras` is
    /// authoritative); unmodeled recognized fields (`class1`–`class3`,
    /// `dormant`, `standing`, Strife booleans) and unknown fields are retained
    /// in `extras` only.
    fn set_field(&mut self, name: &str, value: &Spanned) -> Result<(), UdmfParseError> {
        match name {
            "x" => self.x = Some(as_f64(value)?),
            "y" => self.y = Some(as_f64(value)?),
            "height" => self.height = as_f64(value)?,
            "angle" => self.angle = as_i32(value)?,
            "type" => self.type_id = Some(as_i32(value)?),
            "id" => self.id = as_i32(value)?,
            "special" => self.special = as_i32(value)?,
            "arg0" => self.args[0] = as_i32(value)?,
            "arg1" => self.args[1] = as_i32(value)?,
            "arg2" => self.args[2] = as_i32(value)?,
            "arg3" => self.args[3] = as_i32(value)?,
            "arg4" => self.args[4] = as_i32(value)?,
            "skill1" => {
                self.skill1 = as_bool(value)?;
                self.extras.set(name, UdmfValue::Bool(self.skill1));
            }
            "skill2" => {
                self.skill2 = as_bool(value)?;
                self.extras.set(name, UdmfValue::Bool(self.skill2));
            }
            "skill3" => {
                self.skill3 = as_bool(value)?;
                self.extras.set(name, UdmfValue::Bool(self.skill3));
            }
            "skill4" => {
                self.skill4 = as_bool(value)?;
                self.extras.set(name, UdmfValue::Bool(self.skill4));
            }
            "skill5" => {
                self.skill5 = as_bool(value)?;
                self.extras.set(name, UdmfValue::Bool(self.skill5));
            }
            "ambush" => {
                self.ambush = as_bool(value)?;
                self.extras.set(name, UdmfValue::Bool(self.ambush));
            }
            "single" => {
                self.single = as_bool(value)?;
                self.extras.set(name, UdmfValue::Bool(self.single));
            }
            "dm" => {
                self.dm = as_bool(value)?;
                self.extras.set(name, UdmfValue::Bool(self.dm));
            }
            "coop" => {
                self.coop = as_bool(value)?;
                self.extras.set(name, UdmfValue::Bool(self.coop));
            }
            "friend" => {
                self.friend = as_bool(value)?;
                self.extras.set(name, UdmfValue::Bool(self.friend));
            }
            _ => self.extras.set(name, to_value(value)?),
        }
        Ok(())
    }

    /// Finalizes the block, erroring if `x`/`y`/`type` were never assigned.
    fn finish(self) -> Result<UdmfThing, UdmfParseError> {
        let x = self.x.ok_or_else(|| missing_field("thing", "x"))?;
        let y = self.y.ok_or_else(|| missing_field("thing", "y"))?;
        let type_id = self.type_id.ok_or_else(|| missing_field("thing", "type"))?;
        // Pack per ADR-0019. The skill pairs are OR-folded (Doom has one bit per
        // pair); Doom's game-mode bits are negative ("not in X"), so they are
        // the inverse of UDMF's positive single/dm/coop booleans. Those three
        // default to `false` like every other UDMF flag, so a `thing` block
        // that names none of them packs to `0x70` — present in no game mode,
        // which is exactly what the spec says an unflagged thing means.
        let mut flags = 0u32;
        set_flag_bit(&mut flags, 0, self.skill1 || self.skill2);
        set_flag_bit(&mut flags, 1, self.skill3);
        set_flag_bit(&mut flags, 2, self.skill4 || self.skill5);
        set_flag_bit(&mut flags, 3, self.ambush);
        set_flag_bit(&mut flags, 4, !self.single);
        set_flag_bit(&mut flags, 5, !self.dm);
        set_flag_bit(&mut flags, 6, !self.coop);
        set_flag_bit(&mut flags, 7, self.friend);
        Ok(UdmfThing {
            x,
            y,
            height: self.height,
            angle: self.angle,
            type_id,
            id: self.id,
            special: self.special,
            args: self.args,
            flags,
            extras: self.extras.finish(),
        })
    }
}

/// Reads the single value token following `=`, without consuming the
/// trailing `;`.
///
/// Returns the raw [`Spanned`] token so target-type conversion (`as_f64`,
/// `as_str`, `as_i32`, `as_bool`) can match on it directly; only
/// `Int`/`Float`/`Str`/`Bool` are accepted as value tokens.
fn read_value(
    lexer: &mut Lexer<'_>,
    last_pos: &mut (usize, usize),
) -> Result<Spanned, UdmfParseError> {
    let spanned = require_next(lexer, *last_pos)?;
    *last_pos = (spanned.line, spanned.column);
    match spanned.token {
        Token::Int(_) | Token::Float(_) | Token::Str(_) | Token::Bool(_) => Ok(spanned),
        _ => Err(syntax_error(
            spanned.line,
            spanned.column,
            "expected a value literal",
        )),
    }
}

/// Consumes the `;` that terminates an assignment.
fn expect_semicolon(
    lexer: &mut Lexer<'_>,
    last_pos: &mut (usize, usize),
) -> Result<(), UdmfParseError> {
    let spanned = require_next(lexer, *last_pos)?;
    *last_pos = (spanned.line, spanned.column);
    match spanned.token {
        Token::Semicolon => Ok(()),
        _ => Err(syntax_error(spanned.line, spanned.column, "expected ';'")),
    }
}

/// Reads the next token, mapping end-of-input to a [`UdmfParseError::Syntax`]
/// at the last-seen position.
fn require_next(
    lexer: &mut Lexer<'_>,
    last_pos: (usize, usize),
) -> Result<Spanned, UdmfParseError> {
    lexer
        .next_spanned()?
        .ok_or_else(|| syntax_error(last_pos.0, last_pos.1, "unexpected end of input"))
}

/// Converts a lexed value token to `f64`; any non-numeric token is a type
/// mismatch.
///
/// A bare `Token::Int` is also accepted and widened to `f64`: real UDMF
/// editors commonly write whole-number coordinates (e.g. `x = 128;`) without
/// a decimal point, and every UDMF implementation coerces this the same way.
/// This does not relax integer-typed fields, which still reject `Token::Float`.
fn as_f64(spanned: &Spanned) -> Result<f64, UdmfParseError> {
    match spanned.token {
        Token::Float(f) => Ok(f),
        // Coordinate magnitudes are small (well within f64's 53-bit exact
        // integer range), so this widening never loses precision in practice.
        #[allow(clippy::cast_precision_loss)]
        Token::Int(i) => Ok(i as f64),
        _ => Err(syntax_error(
            spanned.line,
            spanned.column,
            "expected a floating-point value",
        )),
    }
}

/// Converts a lexed value token to `String`; any non-`Str` token is a type
/// mismatch.
fn as_str(spanned: &Spanned) -> Result<String, UdmfParseError> {
    match &spanned.token {
        Token::Str(s) => Ok(s.clone()),
        _ => Err(syntax_error(
            spanned.line,
            spanned.column,
            "expected a string value",
        )),
    }
}

/// Converts a lexed value token to `i32`; only a bare `Token::Int` is
/// accepted (a `Token::Float` is rejected — an integer field never truncates
/// a float literal). An `i64` literal outside `i32`'s range is a
/// [`UdmfParseError::Semantic`], not a syntax error: the token is
/// well-formed, but cannot be represented in the target field.
fn as_i32(spanned: &Spanned) -> Result<i32, UdmfParseError> {
    match spanned.token {
        Token::Int(i) => i32::try_from(i).map_err(|_| UdmfParseError::Semantic {
            message: format!(
                "integer value {i} at line {}, column {} does not fit in a 32-bit field",
                spanned.line, spanned.column
            ),
        }),
        _ => Err(syntax_error(
            spanned.line,
            spanned.column,
            "expected an integer value",
        )),
    }
}

/// Converts a lexed value token to `bool`; any non-`Bool` token is a type
/// mismatch.
fn as_bool(spanned: &Spanned) -> Result<bool, UdmfParseError> {
    match spanned.token {
        Token::Bool(b) => Ok(b),
        _ => Err(syntax_error(
            spanned.line,
            spanned.column,
            "expected a boolean value",
        )),
    }
}

/// Rejects end-of-input while a block is still open (unbalanced braces).
///
/// `last_pos` is the last known source position, used to anchor the error
/// when there is no further input to point at.
fn check_block_closed(block_open: bool, last_pos: (usize, usize)) -> Result<(), UdmfParseError> {
    if block_open {
        return Err(syntax_error(
            last_pos.0,
            last_pos.1,
            "unexpected end of input inside block",
        ));
    }
    Ok(())
}

/// Builds a [`UdmfParseError::Syntax`] at `line`/`column` with `message`.
fn syntax_error(line: usize, column: usize, message: &str) -> UdmfParseError {
    UdmfParseError::Syntax {
        line,
        column,
        message: message.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_udmf;
    use crate::Limits;
    use crate::map::udmf::{UdmfParseError, UdmfValue};

    #[test]
    fn parses_namespace_and_vertices() {
        let text =
            "namespace = \"doom\";\nvertex { x = 1.0; y = -2.0; }\nvertex { x = 3.5; y = 4.5; }";
        let m = parse_udmf(text, Limits::default()).unwrap();
        assert_eq!(m.namespace, "doom");
        assert_eq!(m.vertices.len(), 2);
        assert_eq!((m.vertices[0].x, m.vertices[0].y), (1.0, -2.0));
        assert_eq!((m.vertices[1].x, m.vertices[1].y), (3.5, 4.5));
    }

    #[test]
    fn missing_namespace_is_semantic() {
        let err = parse_udmf("vertex { x = 1.0; y = 2.0; }", Limits::default()).unwrap_err();
        assert!(matches!(err, UdmfParseError::Semantic { .. }));
    }

    #[test]
    fn to_value_rejects_a_non_literal_token_defensively() {
        // `read_value` filters non-literal tokens before `to_value` ever
        // sees one; exercise the defensive arm directly on the private
        // helper so its contract is pinned.
        let spanned = crate::map::udmf::lex::Spanned {
            token: crate::map::udmf::lex::Token::LBrace,
            line: 3,
            column: 7,
        };
        let err = super::to_value(&spanned).unwrap_err();
        assert_eq!(
            err,
            UdmfParseError::Syntax {
                line: 3,
                column: 7,
                message: "expected a value literal".to_owned(),
            }
        );
    }

    #[test]
    fn vertex_missing_required_field_is_semantic() {
        let err =
            parse_udmf("namespace=\"doom\"; vertex { x = 1.0; }", Limits::default()).unwrap_err();
        assert!(matches!(err, UdmfParseError::Semantic { .. }));
    }

    #[test]
    fn nested_block_is_syntax_error() {
        let err = parse_udmf(
            "namespace=\"doom\"; vertex { x=1.0; { } }",
            Limits::default(),
        )
        .unwrap_err();
        assert!(matches!(err, UdmfParseError::Syntax { .. }));
    }

    #[test]
    fn depth_limit_rejects_brace_run() {
        // A run of `{` must be rejected (either Syntax at depth 1 or DepthExceeded),
        // never a stack overflow.
        let text = format!("namespace=\"doom\"; x {}", "{".repeat(500));
        let err = parse_udmf(&text, Limits::new().with_max_depth(8)).unwrap_err();
        assert!(matches!(
            err,
            UdmfParseError::Syntax { .. } | UdmfParseError::DepthExceeded { .. }
        ));
    }

    #[test]
    fn unterminated_block_at_eof_is_syntax_error() {
        // No closing brace on the (only) block.
        let e1 = parse_udmf(
            "namespace=\"doom\"; vertex { x = 1.0; y = 2.0;",
            crate::Limits::default(),
        )
        .unwrap_err();
        assert!(matches!(e1, UdmfParseError::Syntax { .. }), "got {e1:?}");
        // EOF immediately after `{`.
        let e2 = parse_udmf("namespace=\"doom\"; vertex {", crate::Limits::default()).unwrap_err();
        assert!(matches!(e2, UdmfParseError::Syntax { .. }), "got {e2:?}");
        // Valid first block, truncated second block.
        let e3 = parse_udmf(
            "namespace=\"doom\"; vertex { x=1.0; y=2.0; } vertex { x=3.0;",
            crate::Limits::default(),
        )
        .unwrap_err();
        assert!(matches!(e3, UdmfParseError::Syntax { .. }), "got {e3:?}");
    }

    #[test]
    fn float_field_accepts_integer_literal() {
        let m = parse_udmf(
            "namespace=\"doom\"; vertex { x = 128; y = -64; }",
            crate::Limits::default(),
        )
        .unwrap();
        assert_eq!((m.vertices[0].x, m.vertices[0].y), (128.0, -64.0));
    }

    #[test]
    fn unknown_block_and_field_are_retained() {
        let text =
            "namespace=\"doom\"; widget { foo = 1; bar = \"x\"; } vertex { x=1.0; y=2.0; z=9.0; }";
        let m = parse_udmf(text, Limits::default()).unwrap();
        assert_eq!(m.vertices.len(), 1);
        assert_eq!((m.vertices[0].x, m.vertices[0].y), (1.0, 2.0));
        // The unrecognized `widget` block is retained verbatim (ADR-0027).
        assert_eq!(m.unknown_blocks.len(), 1);
        let block = &m.unknown_blocks[0];
        assert_eq!(block.name, "widget");
        assert_eq!(block.fields.len(), 2);
        assert_eq!(block.fields[0].name, "foo");
        assert_eq!(block.fields[0].value, UdmfValue::Int(1));
        assert_eq!(block.fields[1].name, "bar");
        assert_eq!(block.fields[1].value, UdmfValue::Str("x".to_owned()));
    }

    #[test]
    fn parses_full_one_of_each_block_with_defaults() {
        let text = concat!(
            "namespace = \"doom\";\n",
            "vertex { x = 0.0; y = 0.0; }\n",
            "vertex { x = 64.0; y = 0.0; }\n",
            "sidedef { sector = 0; }\n",
            "sector { texturefloor = \"FLOOR1\"; textureceiling = \"CEIL1\"; }\n",
            "linedef { v1 = 0; v2 = 1; sidefront = 0; blocking = true; twosided = true; }\n",
            "thing { x = 32.0; y = 16.0; type = 1; }\n",
        );
        let m = parse_udmf(text, crate::Limits::default()).unwrap();

        // sidedef defaults
        let sd = &m.sidedefs[0];
        assert_eq!((sd.offsetx, sd.offsety, sd.sector), (0, 0, 0));
        assert_eq!(
            (
                sd.texturetop.as_str(),
                sd.texturebottom.as_str(),
                sd.texturemiddle.as_str()
            ),
            ("-", "-", "-")
        );

        // sector defaults
        let sc = &m.sectors[0];
        assert_eq!(
            (
                sc.heightfloor,
                sc.heightceiling,
                sc.lightlevel,
                sc.special,
                sc.id
            ),
            (0, 0, 160, 0, 0)
        );
        assert_eq!(
            (sc.texturefloor.as_str(), sc.textureceiling.as_str()),
            ("FLOOR1", "CEIL1")
        );

        // linedef: sideback default -1 -> None; flags bits 0 and 2 set (blocking|twosided)
        let ld = &m.linedefs[0];
        assert_eq!((ld.v1, ld.v2, ld.sidefront), (0, 1, 0));
        assert_eq!(ld.sideback, None);
        assert_eq!(ld.id, -1);
        assert_eq!(ld.special, 0);
        assert_eq!(ld.args, [0; 5]);
        assert_eq!(ld.flags, 0b0000_0101);

        // thing defaults
        let th = &m.things[0];
        assert_eq!((th.x, th.y, th.height), (32.0, 16.0, 0.0));
        assert_eq!((th.angle, th.type_id, th.id, th.special), (0, 1, 0, 0));
        assert_eq!(th.args, [0; 5]);
    }

    #[test]
    fn linedef_explicit_sideback_and_args() {
        let text = "namespace=\"doom\"; linedef { v1=0; v2=1; sidefront=0; sideback=5; special=80; arg0=1; arg2=3; id=7; }";
        let m = parse_udmf(text, crate::Limits::default()).unwrap();
        let ld = &m.linedefs[0];
        assert_eq!(ld.sideback, Some(5));
        assert_eq!(ld.special, 80);
        assert_eq!(ld.args, [1, 0, 3, 0, 0]);
        assert_eq!(ld.id, 7);
    }

    #[test]
    fn sidedef_missing_sector_is_semantic() {
        let err = parse_udmf(
            "namespace=\"doom\"; sidedef { offsetx = 1; }",
            crate::Limits::default(),
        )
        .unwrap_err();
        assert!(matches!(
            err,
            crate::map::udmf::UdmfParseError::Semantic { .. }
        ));
    }

    #[test]
    fn thing_type_overflow_is_semantic() {
        let err = parse_udmf(
            "namespace=\"doom\"; thing { x=0.0; y=0.0; type=9999999999; }",
            crate::Limits::default(),
        )
        .unwrap_err();
        assert!(matches!(
            err,
            crate::map::udmf::UdmfParseError::Semantic { .. }
        ));
    }

    #[test]
    fn max_depth_zero_yields_depth_exceeded() {
        // With `max_depth == 0`, the first `{` (new depth 1 > 0) is rejected by
        // the depth guard itself, deterministically exercising `DepthExceeded`.
        let err = parse_udmf(
            "namespace=\"doom\"; x {",
            crate::Limits::new().with_max_depth(0),
        )
        .unwrap_err();
        assert!(
            matches!(
                err,
                crate::map::udmf::UdmfParseError::DepthExceeded { max_depth: 0, .. }
            ),
            "got {err:?}"
        );
    }

    #[test]
    fn integer_field_rejects_float_literal() {
        // A `float` literal assigned to an `i32`-typed field (`sidedef.sector`)
        // is a syntax error — the reverse of the accepted int->float widening.
        let err = parse_udmf(
            "namespace=\"doom\"; sidedef { sector = 0.5; }",
            crate::Limits::default(),
        )
        .unwrap_err();
        assert!(
            matches!(err, crate::map::udmf::UdmfParseError::Syntax { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn global_identifier_without_equals_or_lbrace_is_syntax_error() {
        // At file scope, an identifier must be followed by `=` (assignment)
        // or `{` (block header); anything else is rejected.
        let err = parse_udmf("namespace \"doom\";", Limits::default()).unwrap_err();
        match err {
            UdmfParseError::Syntax { message, .. } => {
                assert_eq!(message, "expected '=' or '{' after identifier");
            }
            other => panic!("expected Syntax error, got {other:?}"),
        }
    }

    #[test]
    fn global_non_namespace_assignment_is_retained() {
        // A top-level assignment to an identifier other than `namespace` is
        // accepted and retained verbatim in `global_extras` (ADR-0027); only
        // `namespace` is promoted to its dedicated field.
        let text = "namespace=\"doom\"; author=\"me\"; vertex { x=1.0; y=2.0; }";
        let m = parse_udmf(text, Limits::default()).unwrap();
        assert_eq!(m.namespace, "doom");
        assert_eq!(m.vertices.len(), 1);
        assert_eq!(m.global_extras.len(), 1);
        assert_eq!(m.global_extras[0].name, "author");
        assert_eq!(m.global_extras[0].value, UdmfValue::Str("me".to_owned()));
    }

    #[test]
    fn block_item_missing_equals_is_syntax_error() {
        let err = parse_udmf(
            "namespace=\"doom\"; vertex { x 1.0; y=2.0; }",
            Limits::default(),
        )
        .unwrap_err();
        assert!(matches!(err, UdmfParseError::Syntax { .. }), "got {err:?}");
    }

    #[test]
    fn vertex_missing_x_is_semantic() {
        // The existing `vertex_missing_required_field_is_semantic` test only
        // omits `y`; this covers the `x`-specific missing-field arm.
        let err =
            parse_udmf("namespace=\"doom\"; vertex { y = 1.0; }", Limits::default()).unwrap_err();
        match err {
            UdmfParseError::Semantic { message } => {
                assert!(message.contains('x'), "message was: {message}");
            }
            other => panic!("expected Semantic error, got {other:?}"),
        }
    }

    #[test]
    fn linedef_boolean_flags_false_and_all_set_correctly() {
        let text = concat!(
            "namespace=\"doom\"; linedef { v1=0; v2=1; sidefront=0;",
            " blocking=false; blockmonsters=true; twosided=false;",
            " dontpegtop=true; dontpegbottom=true; secret=true;",
            " blocksound=true; dontdraw=true; mapped=true;",
            " unknownfield=1; }",
        );
        let m = parse_udmf(text, Limits::default()).unwrap();
        // bit0 blocking=false, bit1 blockmonsters=true, bit2 twosided=false,
        // bits 3-8 (dontpegtop..mapped) all true.
        assert_eq!(m.linedefs[0].flags, 0b1_1111_1010);
    }

    #[test]
    fn sidedef_unknown_field_is_retained() {
        let m = parse_udmf(
            "namespace=\"doom\"; sidedef { sector = 3; extra = \"x\"; }",
            Limits::default(),
        )
        .unwrap();
        assert_eq!(m.sidedefs[0].sector, 3);
        assert_eq!(m.sidedefs[0].extras.len(), 1);
        assert_eq!(m.sidedefs[0].extras[0].name, "extra");
        assert_eq!(
            m.sidedefs[0].extras[0].value,
            UdmfValue::Str("x".to_owned())
        );
    }

    #[test]
    fn sector_unknown_field_is_retained() {
        let m = parse_udmf(
            concat!(
                "namespace=\"doom\"; sector { texturefloor=\"F\";",
                " textureceiling=\"C\"; extra = 5; }",
            ),
            Limits::default(),
        )
        .unwrap();
        assert_eq!(m.sectors[0].texturefloor, "F");
        assert_eq!(m.sectors[0].extras.len(), 1);
        assert_eq!(m.sectors[0].extras[0].name, "extra");
        assert_eq!(m.sectors[0].extras[0].value, UdmfValue::Int(5));
    }

    #[test]
    fn thing_all_optional_fields_and_unknown_field_is_retained() {
        let text = concat!(
            "namespace=\"doom\"; thing { x=1.0; y=2.0; height=8.0; angle=90;",
            " type=5; id=9; special=3; arg0=1; arg1=2; arg2=3; arg3=4; arg4=5;",
            " extra=\"kept\"; }",
        );
        let m = parse_udmf(text, Limits::default()).unwrap();
        let th = &m.things[0];
        assert_eq!((th.height, th.angle), (8.0, 90));
        assert_eq!((th.id, th.special), (9, 3));
        assert_eq!(th.args, [1, 2, 3, 4, 5]);
        assert_eq!(th.extras.len(), 1);
        assert_eq!(th.extras[0].name, "extra");
        assert_eq!(th.extras[0].value, UdmfValue::Str("kept".to_owned()));
    }

    #[test]
    fn thing_flags_pack_into_doom_bit_layout() {
        let text = r#"
            namespace = "doom";
            thing { x = 0; y = 0; type = 1; skill1 = true; skill3 = true;
                    skill5 = true; ambush = true; single = false; dm = false;
                    coop = false; friend = true; }
        "#;
        let map = parse_udmf(text, Limits::default()).unwrap();
        // skill1|skill2 -> bit0, skill3 -> bit1, skill4|skill5 -> bit2,
        // ambush -> bit3, !single -> bit4, !dm -> bit5, !coop -> bit6, friend -> bit7.
        assert_eq!(map.things[0].flags, 0b1111_1111);
    }

    #[test]
    fn unflagged_thing_appears_in_no_game_mode() {
        // The UDMF spec: "All flags default to false." (Its suggested *editor*
        // default of true is a UI hint, not parse semantics.) `single`/`dm`/
        // `coop` are therefore false here, and Doom's bits are their negation,
        // so bits 4/5/6 are all set: 0x70 — the thing appears in no game mode.
        let text = r#"
            namespace = "doom";
            thing { x = 0; y = 0; type = 1; }
        "#;
        let map = parse_udmf(text, Limits::default()).unwrap();
        assert_eq!(map.things[0].flags, 0x70);
    }

    #[test]
    fn thing_skill_pairs_are_or_folded() {
        // skill1 and skill2 collapse to one Doom bit; setting either sets bit 0.
        // Order matters: a naive set-then-clear would leave bit 0 clear here.
        // Bits 4/5/6 are set because single/dm/coop default to false (above).
        let text = r#"
            namespace = "doom";
            thing { x = 0; y = 0; type = 1; skill1 = true; skill2 = false; }
        "#;
        let map = parse_udmf(text, Limits::default()).unwrap();
        assert_eq!(map.things[0].flags, 0b0111_0001);
    }

    #[test]
    fn thing_game_mode_flags_are_the_inverse_of_dooms_bits() {
        // The positive UDMF booleans clear Doom's negative "not in X" bits.
        let text = r#"
            namespace = "doom";
            thing { x = 0; y = 0; type = 1; single = true; dm = true; coop = true; }
        "#;
        let map = parse_udmf(text, Limits::default()).unwrap();
        assert_eq!(map.things[0].flags, 0);
    }

    #[test]
    fn value_position_rejects_non_literal_token() {
        // An identifier is not a valid value literal.
        let err = parse_udmf("namespace = vertex;", Limits::default()).unwrap_err();
        assert!(matches!(err, UdmfParseError::Syntax { .. }), "got {err:?}");
    }

    #[test]
    fn float_field_rejects_non_numeric_value() {
        let err = parse_udmf(
            "namespace=\"doom\"; vertex { x = true; y = 1.0; }",
            Limits::default(),
        )
        .unwrap_err();
        assert!(matches!(err, UdmfParseError::Syntax { .. }), "got {err:?}");
    }

    #[test]
    fn string_field_rejects_non_string_value() {
        let err = parse_udmf(
            "namespace=\"doom\"; sidedef { sector = 0; texturetop = 5; }",
            Limits::default(),
        )
        .unwrap_err();
        assert!(matches!(err, UdmfParseError::Syntax { .. }), "got {err:?}");
    }

    #[test]
    fn bool_field_rejects_non_bool_value() {
        let err = parse_udmf(
            "namespace=\"doom\"; linedef { v1=0; v2=1; sidefront=0; blocking = 1; }",
            Limits::default(),
        )
        .unwrap_err();
        assert!(matches!(err, UdmfParseError::Syntax { .. }), "got {err:?}");
    }

    use proptest::prelude::*;

    proptest! {
        #[test]
        fn never_panics_and_element_count_is_bounded_by_input(input in ".*") {
            if let Ok(m) = parse_udmf(&input, crate::Limits::default()) {
                let elements = m.vertices.len() + m.linedefs.len()
                    + m.sidedefs.len() + m.sectors.len() + m.things.len();
                // Each produced element requires at least one `{`...`}` pair, so
                // the element count is bounded by the input length (O(input)).
                prop_assert!(elements <= input.len());
            }
        }
    }
}
