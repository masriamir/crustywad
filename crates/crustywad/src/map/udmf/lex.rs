//! UDMF tokenizer.
//!
//! [`Lexer`] scans UDMF `TEXTMAP` source text into a flat stream of
//! [`Spanned`] [`Token`]s. The scan is a single non-recursive loop over the
//! input's characters, tracking 1-based line/column positions as it goes.
//! This module is crate-internal; [`super::parse`][crate::map::udmf] (Task 3)
//! consumes the token stream to build the UDMF AST.

use super::UdmfParseError;

/// A single lexical token produced by [`Lexer`].
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Token {
    /// `{`
    LBrace,
    /// `}`
    RBrace,
    /// `=`
    Equals,
    /// `;`
    Semicolon,
    /// An identifier or keyword, e.g. a block name or field key.
    Ident(String),
    /// A `true`/`false` literal.
    Bool(bool),
    /// A signed integer literal.
    Int(i64),
    /// A floating-point literal.
    Float(f64),
    /// A double-quoted string literal with escapes resolved.
    Str(String),
}

/// A [`Token`] paired with the 1-based line and column of its first character.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Spanned {
    /// The token itself.
    pub(crate) token: Token,
    /// 1-based source line of the token's first character.
    pub(crate) line: usize,
    /// 1-based source column of the token's first character.
    pub(crate) column: usize,
}

/// A flat, non-recursive tokenizer over UDMF source text.
///
/// `Lexer` holds a `Vec<char>` view of the input plus a cursor index and the
/// current 1-based line/column. Each call to [`Lexer::next_spanned`] advances
/// the cursor past exactly one token (skipping whitespace and comments
/// first) and returns it.
pub(crate) struct Lexer<'a> {
    /// Retained for potential future use (e.g. byte-offset reporting); the
    /// scan itself operates over `chars`.
    _input: &'a str,
    /// The input, decoded into a random-accessible character buffer.
    chars: Vec<char>,
    /// Index of the next unconsumed character in `chars`.
    pos: usize,
    /// 1-based line of the next unconsumed character.
    line: usize,
    /// 1-based column of the next unconsumed character.
    column: usize,
}

impl<'a> Lexer<'a> {
    /// Creates a lexer over `input`.
    pub(crate) fn new(input: &'a str) -> Self {
        Self {
            _input: input,
            chars: input.chars().collect(),
            pos: 0,
            line: 1,
            column: 1,
        }
    }

    /// Returns the character at `pos` without consuming it.
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    /// Returns the character at `pos + offset` without consuming it.
    fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.pos + offset).copied()
    }

    /// Consumes and returns the character at `pos`, advancing `line`/`column`.
    fn advance(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += 1;
        if c == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(c)
    }

    /// Skips whitespace, `//` line comments, and `/* */` block comments.
    ///
    /// Returns an error if a block comment is left unterminated.
    fn skip_trivia(&mut self) -> Result<(), UdmfParseError> {
        loop {
            match self.peek() {
                Some(c) if c.is_whitespace() => {
                    self.advance();
                }
                Some('/') if self.peek_at(1) == Some('/') => {
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.advance();
                    }
                }
                Some('/') if self.peek_at(1) == Some('*') => {
                    let (start_line, start_column) = (self.line, self.column);
                    self.advance();
                    self.advance();
                    let mut closed = false;
                    while let Some(c) = self.peek() {
                        if c == '*' && self.peek_at(1) == Some('/') {
                            self.advance();
                            self.advance();
                            closed = true;
                            break;
                        }
                        self.advance();
                    }
                    if !closed {
                        return Err(UdmfParseError::Syntax {
                            line: start_line,
                            column: start_column,
                            message: "unterminated block comment".to_owned(),
                        });
                    }
                }
                _ => break,
            }
        }
        Ok(())
    }

    /// Scans a double-quoted string literal, resolving `\"` and `\\` escapes.
    ///
    /// The opening quote must already have been consumed by the caller.
    fn scan_string(
        &mut self,
        start_line: usize,
        start_column: usize,
    ) -> Result<Token, UdmfParseError> {
        let mut value = String::new();
        loop {
            match self.advance() {
                None | Some('\n') => {
                    return Err(UdmfParseError::Syntax {
                        line: start_line,
                        column: start_column,
                        message: "unterminated string literal".to_owned(),
                    });
                }
                Some('"') => return Ok(Token::Str(value)),
                Some('\\') => match self.advance() {
                    Some('"') => value.push('"'),
                    Some('\\') => value.push('\\'),
                    Some('n') => value.push('\n'),
                    Some('t') => value.push('\t'),
                    Some(other) => value.push(other),
                    None => {
                        return Err(UdmfParseError::Syntax {
                            line: start_line,
                            column: start_column,
                            message: "unterminated string literal".to_owned(),
                        });
                    }
                },
                Some(c) => value.push(c),
            }
        }
    }

    /// Scans an identifier/keyword starting at the already-consumed first
    /// character `first`, mapping `true`/`false` to [`Token::Bool`].
    fn scan_ident(&mut self, first: char) -> Token {
        let mut ident = String::new();
        ident.push(first);
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == '_' {
                ident.push(c);
                self.advance();
            } else {
                break;
            }
        }
        match ident.as_str() {
            "true" => Token::Bool(true),
            "false" => Token::Bool(false),
            _ => Token::Ident(ident),
        }
    }

    /// Scans a numeric literal starting at the already-consumed first
    /// character `first`, producing [`Token::Int`] or [`Token::Float`]
    /// depending on whether a fraction/exponent is present.
    fn scan_number(
        &mut self,
        first: char,
        start_line: usize,
        start_column: usize,
    ) -> Result<Token, UdmfParseError> {
        let mut text = String::new();
        text.push(first);
        let mut is_float = first == '.';

        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                text.push(c);
                self.advance();
            } else if c == '.' && !is_float {
                is_float = true;
                text.push(c);
                self.advance();
            } else if (c == 'e' || c == 'E')
                && (matches!(self.peek_at(1), Some(d) if d.is_ascii_digit())
                    || matches!(self.peek_at(1), Some(s) if (s == '+' || s == '-') && matches!(self.peek_at(2), Some(d) if d.is_ascii_digit())))
            {
                is_float = true;
                text.push(c);
                self.advance();
                if let Some(sign @ ('+' | '-')) = self.peek() {
                    text.push(sign);
                    self.advance();
                }
            } else {
                break;
            }
        }

        if is_float {
            text.parse::<f64>()
                .map(Token::Float)
                .map_err(|_| UdmfParseError::Syntax {
                    line: start_line,
                    column: start_column,
                    message: format!("invalid float literal '{text}'"),
                })
        } else {
            text.parse::<i64>()
                .map(Token::Int)
                .map_err(|_| UdmfParseError::Syntax {
                    line: start_line,
                    column: start_column,
                    message: format!("invalid integer literal '{text}'"),
                })
        }
    }

    /// Scans and returns the next token, or `Ok(None)` at end of input.
    ///
    /// # Errors
    /// Returns [`UdmfParseError::Syntax`] if a string is unterminated, a
    /// block comment is unterminated, a numeric literal is malformed, or a
    /// character does not start any recognized token.
    pub(crate) fn next_spanned(&mut self) -> Result<Option<Spanned>, UdmfParseError> {
        self.skip_trivia()?;

        let (line, column) = (self.line, self.column);
        let Some(c) = self.advance() else {
            return Ok(None);
        };

        let token = match c {
            '{' => Token::LBrace,
            '}' => Token::RBrace,
            '=' => Token::Equals,
            ';' => Token::Semicolon,
            '"' => self.scan_string(line, column)?,
            '-' | '+' if matches!(self.peek(), Some(d) if d.is_ascii_digit() || d == '.') => {
                // Pass the sign as the leading character so the full signed
                // literal is parsed in one step. Parsing the magnitude and then
                // negating would reject `i64::MIN`, whose magnitude
                // (9223372036854775808) exceeds `i64::MAX`.
                self.scan_number(c, line, column)?
            }
            c if c.is_ascii_digit() || c == '.' => self.scan_number(c, line, column)?,
            c if c.is_ascii_alphabetic() || c == '_' => self.scan_ident(c),
            other => {
                return Err(UdmfParseError::Syntax {
                    line,
                    column,
                    message: format!("unexpected character '{other}'"),
                });
            }
        };

        Ok(Some(Spanned {
            token,
            line,
            column,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::{Lexer, Token};

    fn lex_all(input: &str) -> Vec<Token> {
        let mut lx = Lexer::new(input);
        let mut out = Vec::new();
        while let Some(s) = lx.next_spanned().unwrap() {
            out.push(s.token);
        }
        out
    }

    #[test]
    fn lexes_assignment_tokens() {
        assert_eq!(
            lex_all("namespace = \"doom\" ;"),
            vec![
                Token::Ident("namespace".into()),
                Token::Equals,
                Token::Str("doom".into()),
                Token::Semicolon,
            ]
        );
    }

    #[test]
    fn lexes_block_and_numbers_and_bools() {
        assert_eq!(
            lex_all("vertex { x = -1; y = 2.5; b = true; }"),
            vec![
                Token::Ident("vertex".into()),
                Token::LBrace,
                Token::Ident("x".into()),
                Token::Equals,
                Token::Int(-1),
                Token::Semicolon,
                Token::Ident("y".into()),
                Token::Equals,
                Token::Float(2.5),
                Token::Semicolon,
                Token::Ident("b".into()),
                Token::Equals,
                Token::Bool(true),
                Token::Semicolon,
                Token::RBrace,
            ]
        );
    }

    #[test]
    fn lexes_i64_boundaries() {
        // `i64::MIN`'s magnitude exceeds `i64::MAX`, so it must be parsed as a
        // single signed literal rather than magnitude-then-negate.
        assert_eq!(
            lex_all("-9223372036854775808 9223372036854775807 +42"),
            vec![Token::Int(i64::MIN), Token::Int(i64::MAX), Token::Int(42)]
        );
    }

    #[test]
    fn skips_line_and_block_comments() {
        assert_eq!(
            lex_all("a // comment\n = /* x */ 1 ;"),
            vec![
                Token::Ident("a".into()),
                Token::Equals,
                Token::Int(1),
                Token::Semicolon
            ]
        );
    }

    #[test]
    fn tracks_line_and_column() {
        let mut lx = Lexer::new("a\n  b");
        let a = lx.next_spanned().unwrap().unwrap();
        assert_eq!((a.line, a.column), (1, 1));
        let b = lx.next_spanned().unwrap().unwrap();
        assert_eq!((b.line, b.column), (2, 3));
    }

    #[test]
    fn handles_string_escapes() {
        assert_eq!(lex_all(r#""a\"b\\c""#), vec![Token::Str("a\"b\\c".into())]);
    }

    #[test]
    fn unterminated_string_is_syntax_error() {
        let mut lx = Lexer::new("\"abc");
        assert!(lx.next_spanned().is_err());
    }
}
