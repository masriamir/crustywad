//! Iterative, depth-bounded UDMF parser engine.
//!
//! [`parse_udmf`] drives [`Lexer`] tokens through a flat two-state machine
//! (`ExpectGlobalItem` / `ExpectBlockItem`) tracked by an explicit `depth`
//! counter. There is no recursion anywhere in this module (ADR-0016).

use crate::Limits;

use super::UdmfParseError;
use super::lex::{Lexer, Spanned, Token};
use super::model::{UdmfMap, UdmfVertex};

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
    let mut last_pos: (usize, usize);

    let mut namespace: Option<String> = None;
    let mut vertices = Vec::new();
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
                                namespace = Some(as_str(value)?);
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
                            current_block = Some(BlockState::new(&name));
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
                        if let Some(vertex) = block.finish()? {
                            vertices.push(vertex);
                        }
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

    let namespace = namespace.ok_or_else(|| UdmfParseError::Semantic {
        message: "TEXTMAP is missing a required 'namespace' declaration".to_owned(),
    })?;

    Ok(UdmfMap {
        namespace,
        vertices,
        linedefs: Vec::new(),
        sidedefs: Vec::new(),
        sectors: Vec::new(),
        things: Vec::new(),
    })
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
/// recognized field assignments. Unknown blocks accumulate nothing.
enum BlockState {
    /// A `vertex` block; both fields are required (no spec default).
    Vertex {
        /// The `x` field, if assigned.
        x: Option<f64>,
        /// The `y` field, if assigned.
        y: Option<f64>,
    },
    /// Any block header not otherwise recognized; fields are validated for
    /// syntax but dropped.
    Unknown,
}

impl BlockState {
    /// Selects the accumulator for a block header identifier.
    fn new(header: &str) -> Self {
        match header {
            "vertex" => BlockState::Vertex { x: None, y: None },
            _ => BlockState::Unknown,
        }
    }

    /// Applies a recognized field assignment; unknown fields are dropped.
    fn set_field(&mut self, name: &str, value: &Spanned) -> Result<(), UdmfParseError> {
        match self {
            BlockState::Vertex { x, y } => match name {
                "x" => *x = Some(as_f64(value)?),
                "y" => *y = Some(as_f64(value)?),
                _ => {}
            },
            BlockState::Unknown => {}
        }
        Ok(())
    }

    /// Finalizes the block at `}`, returning the assembled element (if any)
    /// or a [`UdmfParseError::Semantic`] if a required field was never set.
    fn finish(self) -> Result<Option<UdmfVertex>, UdmfParseError> {
        match self {
            BlockState::Vertex { x, y } => {
                let x = x.ok_or_else(|| UdmfParseError::Semantic {
                    message: "vertex block is missing required field 'x'".to_owned(),
                })?;
                let y = y.ok_or_else(|| UdmfParseError::Semantic {
                    message: "vertex block is missing required field 'y'".to_owned(),
                })?;
                Ok(Some(UdmfVertex { x, y }))
            }
            BlockState::Unknown => Ok(None),
        }
    }
}

/// Reads the single value token following `=`, without consuming the
/// trailing `;`.
///
/// Returns the raw [`Spanned`] token so target-type conversion (`as_f64`,
/// `as_str`, and Task 4's `i32`/`bool` counterparts) can match on it
/// directly; only `Int`/`Float`/`Str`/`Bool` are accepted as value tokens.
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

/// Converts a lexed value token to `f64`; any non-`Float` token is a type
/// mismatch.
fn as_f64(spanned: &Spanned) -> Result<f64, UdmfParseError> {
    match spanned.token {
        Token::Float(f) => Ok(f),
        _ => Err(syntax_error(
            spanned.line,
            spanned.column,
            "expected a floating-point value",
        )),
    }
}

/// Converts a lexed value token to `String`; any non-`Str` token is a type
/// mismatch. Only used for the `namespace` global in this task; `i32`/`bool`
/// counterparts (`Token::Int`/`Token::Bool`) land with Task 4's block kinds.
fn as_str(spanned: Spanned) -> Result<String, UdmfParseError> {
    match spanned.token {
        Token::Str(s) => Ok(s),
        _ => Err(syntax_error(
            spanned.line,
            spanned.column,
            "expected a string value",
        )),
    }
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
    use crate::map::udmf::UdmfParseError;

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
        let err = parse_udmf(&text, Limits { max_depth: 8 }).unwrap_err();
        assert!(matches!(
            err,
            UdmfParseError::Syntax { .. } | UdmfParseError::DepthExceeded { .. }
        ));
    }

    #[test]
    fn unknown_block_and_field_are_skipped() {
        let text =
            "namespace=\"doom\"; widget { foo = 1; bar = \"x\"; } vertex { x=1.0; y=2.0; z=9.0; }";
        let m = parse_udmf(text, Limits::default()).unwrap();
        assert_eq!(m.vertices.len(), 1);
        assert_eq!((m.vertices[0].x, m.vertices[0].y), (1.0, 2.0));
    }
}
