//! Raven script lumps (ADR-0023 §3): [`SndCurve`] (the Heretic/Hexen
//! distance-attenuation byte table), [`SndInfo`] (Hexen's sound-definition
//! text lump), and [`SndSeq`] (Hexen's sound-sequence script). All three are
//! bounded reads over `&[u8]` with lenient-mode warnings carried inside the
//! returned value, following the shipped gfx/audio idiom.
//!
//! # Tokenizer
//!
//! [`SndInfo`] and [`SndSeq`] are text formats tokenized per Hexen's `sc_man`
//! scanner (engine reference: `src/hexen/sc_man.c:198-254`). The semantics are
//! replicated, not the fixed buffer:
//!
//! - Bytes are interpreted as text via [`String::from_utf8_lossy`].
//! - A token is a run of bytes `> 32`; any byte `<= 32` separates tokens, and
//!   newlines are counted for 1-based line diagnostics.
//! - `;` starts a comment that runs to the end of the line (it also terminates
//!   an unquoted token, matching the engine's `*ScriptPtr != ASCII_COMMENT`
//!   token loop).
//! - `"` opens a quoted string that ends at the next `"` (or the end of the
//!   lump); the quotes are delimiters, not part of the token.
//!
//! The engine silently truncates any token beyond `MAX_STRING_SIZE - 1 = 63`
//! bytes (`sc_man.c:236-250`). This parser has no fixed buffer, so nothing
//! truncates — but a token longer than 63 bytes would have been read
//! differently in-engine, so every such token is flagged with a single
//! aggregate [`AudioWarning::OversizedTokens`] in **both** modes (the
//! [`AudioWarning::OutOfRangeTones`] aggregation pattern).
//!
//! The ZDoom-family `SNDINFO`/`SNDSEQ` extensions (`$random`, `$playersound`,
//! arbitrary sequence commands, …) are **out of scope**: this layer parses the
//! vanilla/Chocolate dialect only (ADR-0023 §3).

use crate::{ParseOptions, Strictness};

use super::{AudioError, AudioWarning};

/// The longest token, in bytes, the engine's `MAX_STRING_SIZE - 1` buffer would
/// have preserved; a longer token is read verbatim here but flagged.
const MAX_TOKEN_LEN: usize = 63;

/// A single `sc_man` token: its text and the 1-based line it began on.
#[derive(Clone)]
struct Token {
    /// The token text, with surrounding quotes (if any) already stripped.
    text: String,
    /// The 1-based line the token began on, for diagnostics.
    line: usize,
}

/// Decodes one raw token slice into a [`Token`], counting it against the
/// engine's 63-byte buffer by its **raw byte length** — the length the
/// engine's `sc_man` buffer would have measured — before any lossy UTF-8
/// decoding (a non-UTF-8 byte expands to a replacement character, which must
/// not affect the oversized accounting).
fn push_token(raw: &[u8], line: usize, tokens: &mut Vec<Token>, oversized: &mut usize) {
    if raw.len() > MAX_TOKEN_LEN {
        *oversized += 1;
    }
    tokens.push(Token {
        text: String::from_utf8_lossy(raw).into_owned(),
        line,
    });
}

/// Tokenizes `bytes` per the `sc_man` scanner semantics, returning the tokens
/// and the count of tokens that exceeded [`MAX_TOKEN_LEN`] **raw bytes**.
///
/// The scan walks the original lump bytes — exactly what the engine reads —
/// and each token slice is decoded (lossily) on its own, so token boundaries,
/// line counting, and the oversized check are all independent of UTF-8
/// validity.
fn tokenize(bytes: &[u8]) -> (Vec<Token>, usize) {
    let n = bytes.len();

    let mut tokens = Vec::new();
    let mut oversized = 0usize;
    let mut line = 1usize;
    let mut i = 0usize;

    while i < n {
        let b = bytes[i];
        if b == b'\n' {
            line += 1;
            i += 1;
            continue;
        }
        if b <= 32 {
            i += 1;
            continue;
        }
        if b == b';' {
            // Comment: skip to (but not past) the end of the line.
            while i < n && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }

        let start_line = line;
        if b == b'"' {
            // Quoted string: from just after the opening quote to the next
            // quote (or the end of the lump). Newlines inside keep the line
            // count accurate.
            i += 1;
            let start = i;
            while i < n && bytes[i] != b'"' {
                if bytes[i] == b'\n' {
                    line += 1;
                }
                i += 1;
            }
            push_token(&bytes[start..i], start_line, &mut tokens, &mut oversized);
            if i < n {
                i += 1; // consume the closing quote
            }
            continue;
        }

        // Unquoted token: a run of bytes `> 32` up to the next whitespace or
        // comment marker.
        let start = i;
        while i < n && bytes[i] > 32 && bytes[i] != b';' {
            i += 1;
        }
        push_token(&bytes[start..i], start_line, &mut tokens, &mut oversized);
    }

    (tokens, oversized)
}

/// Resolves a recoverable script diagnostic under the active strictness: strict
/// mode returns `Err(error)` (propagated via `?`); lenient mode pushes `warning`
/// and returns `Ok(())` so the caller can continue.
fn resolve(
    strictness: Strictness,
    warnings: &mut Vec<AudioWarning>,
    error: AudioError,
    warning: AudioWarning,
) -> Result<(), AudioError> {
    match strictness {
        Strictness::Strict => Err(error),
        Strictness::Lenient => {
            warnings.push(warning);
            Ok(())
        }
    }
}

/// The Heretic/Hexen distance-attenuation curve (`SNDCURVE`; engine reference:
/// `src/heretic/s_sound.c:571`, `src/hexen/s_sound.c:793`).
///
/// The lump is a raw byte table indexed by distance — there is no header to
/// validate, so any length parses in **both** modes with no warnings. The
/// engine simply reads scaled bytes straight out of it. The observed retail
/// lengths are 1600 bytes (Heretic) and 2025 bytes (Hexen); those are data
/// points, not constraints — any length is accepted.
#[derive(Debug, Clone)]
pub struct SndCurve {
    bytes: Vec<u8>,
    warnings: Vec<AudioWarning>,
}

impl SndCurve {
    /// Parses a `SNDCURVE` attenuation table.
    ///
    /// The whole lump is the table; the bytes are copied verbatim.
    ///
    /// # Errors
    ///
    /// This function never returns an error — a `SNDCURVE` lump has no header
    /// or structural invariant to violate, and any length is valid in both
    /// modes. The [`Result`] return keeps the signature uniform with the other
    /// audio parsers.
    pub fn parse(bytes: &[u8], options: &ParseOptions) -> Result<Self, AudioError> {
        let _ = options;
        Ok(Self {
            bytes: bytes.to_vec(),
            warnings: Vec::new(),
        })
    }

    /// The raw attenuation table (an owned copy of the whole lump).
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Non-fatal issues recorded during parsing (always empty today; retained
    /// for API uniformity with the other audio parsers).
    #[must_use]
    pub fn warnings(&self) -> &[AudioWarning] {
        &self.warnings
    }
}

/// A single bare `<tagname> <lumpname>` pair from a `SNDINFO` lump.
///
/// crustywad stores every pair verbatim: it has no `S_sfx` table, so the
/// engine's "unknown tag name" filtering (`src/hexen/s_sound.c:985-1013`) is
/// engine policy, not lump structure, and no pair is dropped or warned on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SndInfoEntry {
    /// The symbolic sound tag name (stored verbatim, case preserved).
    pub tag: String,
    /// The lump name the tag maps to, stored verbatim — including the literal
    /// `?`, which the engine resolves to the lump `DEFAULT` (see
    /// [`resolved_lump`](SndInfoEntry::resolved_lump)).
    pub lump: String,
}

impl SndInfoEntry {
    /// The resolved lump name: `DEFAULT` when [`lump`](SndInfoEntry::lump) is
    /// the literal `?`, else the raw value.
    ///
    /// The engine maps a `?` value to the literal lump `DEFAULT`
    /// (`src/hexen/s_sound.c:985-993`) — and the retail `HEXEN.WAD` ships a real
    /// 4667-byte DMX sound named `DEFAULT`, closing what the engine source alone
    /// leaves ambiguous.
    #[must_use]
    pub fn resolved_lump(&self) -> &str {
        if self.lump == "?" {
            "DEFAULT"
        } else {
            &self.lump
        }
    }
}

/// Hexen's sound-definition text lump (`SNDINFO`; engine reference:
/// `S_InitScript`, `src/hexen/s_sound.c:953-1013`).
///
/// The token stream carries two kinds of `$`-directive and a sequence of bare
/// `<tagname> <lumpname>` pairs:
///
/// - `$ARCHIVEPATH <value>` — the value is consumed and stored nowhere (the
///   engine ignores it too, using it only for its own asset pipeline).
/// - `$MAP <number> <songlump>` — a map-music assignment. `number` is a full
///   decimal `u32`; a `number` of `0` is consumed and dropped silently (engine
///   behavior). Non-zero assignments are stored in file order and exposed by
///   [`map_songs`](SndInfo::map_songs).
/// - Any **other** `$`-directive is silently ignored and consumes **no**
///   following value — the engine `continue`s past it without reading one
///   (`src/hexen/s_sound.c:976`), and the retail Raven IWADs ship `$REGISTERED`
///   under exactly this path (ADR-0023 §3: "unknown `$`-directives ignored").
/// - Every bare `<tag> <lump>` pair is stored via [`entries`](SndInfo::entries).
///
/// # Errors and recovery
///
/// A directive or bare tag whose required value(s) run out at the end of the
/// lump ([`AudioError::SndInfoMissingValue`]) or a `$MAP` whose number is not a
/// decimal `u32` ([`AudioError::SndInfoBadMapNumber`]) is a strict error; lenient
/// mode records the mirror warning and skips the incomplete directive/tag.
#[derive(Debug, Clone)]
pub struct SndInfo {
    map_songs: Vec<(u32, String)>,
    entries: Vec<SndInfoEntry>,
    warnings: Vec<AudioWarning>,
}

impl SndInfo {
    /// Parses a `SNDINFO` sound-definition lump.
    ///
    /// # Errors
    ///
    /// - [`AudioError::SndInfoMissingValue`] (strict only) when `$ARCHIVEPATH`,
    ///   `$MAP`, or a bare tag reaches the end of the lump before its required
    ///   value(s); lenient mode records [`AudioWarning::SndInfoMissingValue`]
    ///   and drops the incomplete item.
    /// - [`AudioError::SndInfoBadMapNumber`] (strict only) when a `$MAP` number
    ///   token is not a decimal `u32`; lenient mode records
    ///   [`AudioWarning::SndInfoBadMapNumber`] and skips the `$MAP` and its
    ///   song-lump token.
    ///
    /// A token longer than 63 bytes is an [`AudioWarning::OversizedTokens`]
    /// aggregate in **both** modes; the parse still succeeds. Unknown
    /// `$`-directives are silently ignored in both modes (engine behavior).
    pub fn parse(bytes: &[u8], options: &ParseOptions) -> Result<Self, AudioError> {
        let (tokens, oversized) = tokenize(bytes);
        let mut map_songs = Vec::new();
        let mut entries = Vec::new();
        let mut warnings = Vec::new();
        if oversized > 0 {
            warnings.push(AudioWarning::OversizedTokens { count: oversized });
        }

        let strictness = options.strictness;
        // Builds the mirrored missing-value error/warning pair for `token`.
        let missing = |token: &Token| {
            (
                AudioError::SndInfoMissingValue {
                    keyword: token.text.clone(),
                    line: token.line,
                },
                AudioWarning::SndInfoMissingValue {
                    keyword: token.text.clone(),
                    line: token.line,
                },
            )
        };

        let mut idx = 0usize;
        while idx < tokens.len() {
            let token = tokens[idx].clone();
            idx += 1;

            if let Some(directive) = token.text.strip_prefix('$') {
                if directive.eq_ignore_ascii_case("ARCHIVEPATH") {
                    // Consume and drop the single value token.
                    if idx >= tokens.len() {
                        let (e, w) = missing(&token);
                        resolve(strictness, &mut warnings, e, w)?;
                        continue;
                    }
                    idx += 1;
                } else if directive.eq_ignore_ascii_case("MAP") {
                    // $MAP <number> <songlump>.
                    if idx >= tokens.len() {
                        let (e, w) = missing(&token);
                        resolve(strictness, &mut warnings, e, w)?;
                        continue;
                    }
                    let number_text = tokens[idx].text.clone();
                    let number_line = tokens[idx].line;
                    idx += 1;

                    let Ok(number) = number_text.parse::<u32>() else {
                        // Skip the song-lump token too (both tokens skipped).
                        if idx < tokens.len() {
                            idx += 1;
                        }
                        resolve(
                            strictness,
                            &mut warnings,
                            AudioError::SndInfoBadMapNumber {
                                value: number_text.clone(),
                                line: number_line,
                            },
                            AudioWarning::SndInfoBadMapNumber {
                                value: number_text,
                                line: number_line,
                            },
                        )?;
                        continue;
                    };

                    if idx >= tokens.len() {
                        let (e, w) = missing(&token);
                        resolve(strictness, &mut warnings, e, w)?;
                        continue;
                    }
                    let song = tokens[idx].text.clone();
                    idx += 1;
                    // Map 0 is consumed and dropped silently (engine behavior).
                    if number != 0 {
                        map_songs.push((number, song));
                    }
                }
                // Any other `$`-directive is silently ignored and consumes no
                // value (engine `continue`, ADR-0023 §3).
                continue;
            }

            // A bare `<tag> <lump>` pair.
            if idx >= tokens.len() {
                let (e, w) = missing(&token);
                resolve(strictness, &mut warnings, e, w)?;
                continue;
            }
            let lump = tokens[idx].text.clone();
            idx += 1;
            entries.push(SndInfoEntry {
                tag: token.text,
                lump,
            });
        }

        Ok(Self {
            map_songs,
            entries,
            warnings,
        })
    }

    /// The non-zero `$MAP` music assignments, as `(map number, song lump)`
    /// pairs in file order.
    #[must_use]
    pub fn map_songs(&self) -> &[(u32, String)] {
        &self.map_songs
    }

    /// The bare `<tag> <lump>` pairs, in file order.
    #[must_use]
    pub fn entries(&self) -> &[SndInfoEntry] {
        &self.entries
    }

    /// Non-fatal issues recorded during parsing.
    #[must_use]
    pub fn warnings(&self) -> &[AudioWarning] {
        &self.warnings
    }
}

/// A single typed `SNDSEQ` command (engine reference: `SN_InitSequenceScript`,
/// `src/hexen/sn_sonix.c:177-307`).
///
/// Sound arguments are stored verbatim as tag-name strings — the engine's
/// fatal unknown-sound lookup is engine policy, not lump structure. Numeric
/// arguments are full decimal `u32` values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SndSeqCommand {
    /// `play <sound>`: start a sound.
    Play(
        /// The sound tag name.
        String,
    ),
    /// `playuntildone <sound>`: play a sound and wait for it to finish.
    PlayUntilDone(
        /// The sound tag name.
        String,
    ),
    /// `playtime <sound> <tics>`: play a sound for a fixed number of tics.
    PlayTime {
        /// The sound tag name.
        sound: String,
        /// The play duration in tics.
        tics: u32,
    },
    /// `playrepeat <sound>`: play a sound on a loop.
    PlayRepeat(
        /// The sound tag name.
        String,
    ),
    /// `delay <tics>`: pause for a fixed number of tics.
    Delay(
        /// The delay in tics.
        u32,
    ),
    /// `delayrand <min> <max>`: pause for a random number of tics in a range.
    DelayRand {
        /// The minimum delay in tics.
        min: u32,
        /// The maximum delay in tics.
        max: u32,
    },
    /// `volume <n>`: set the sequence volume.
    Volume(
        /// The volume level.
        u32,
    ),
    /// `stopsound <sound>`: play a stop sound and end the current looped sound.
    StopSound(
        /// The sound tag name.
        String,
    ),
    /// `end`: terminate the sequence. Stored as the final command when present.
    End,
}

/// One `SNDSEQ` sound sequence: a `:`-prefixed name and its ordered commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SndSeqSequence {
    /// The sequence name (the token text after the leading `:`; any name
    /// parses — the engine's fixed 21-entry translate table is engine policy,
    /// ADR-0023 §3).
    pub name: String,
    /// The sequence's commands in file order; a terminated sequence ends with
    /// [`SndSeqCommand::End`].
    pub commands: Vec<SndSeqCommand>,
}

/// A diagnostic produced while parsing a single `SNDSEQ` command, resolved to a
/// strict [`AudioError`] or a lenient [`AudioWarning`] by the caller.
enum SeqDiag {
    /// An unrecognized command token.
    Unknown { command: String, line: usize },
    /// A command reached the end of the lump before all its arguments.
    Missing { command: String, line: usize },
    /// A command's numeric argument is not a decimal `u32`.
    BadNumber {
        command: String,
        value: String,
        line: usize,
    },
}

impl SeqDiag {
    /// The strict-mode error for this diagnostic.
    fn into_error(self) -> AudioError {
        match self {
            SeqDiag::Unknown { command, line } => {
                AudioError::SndSeqUnknownCommand { command, line }
            }
            SeqDiag::Missing { command, line } => {
                AudioError::SndSeqMissingArgument { command, line }
            }
            SeqDiag::BadNumber {
                command,
                value,
                line,
            } => AudioError::SndSeqBadNumber {
                command,
                value,
                line,
            },
        }
    }

    /// The lenient-mode warning for this diagnostic.
    fn into_warning(self) -> AudioWarning {
        match self {
            SeqDiag::Unknown { command, line } => {
                AudioWarning::SndSeqUnknownCommand { command, line }
            }
            SeqDiag::Missing { command, line } => {
                AudioWarning::SndSeqMissingArgument { command, line }
            }
            SeqDiag::BadNumber {
                command,
                value,
                line,
            } => AudioWarning::SndSeqBadNumber {
                command,
                value,
                line,
            },
        }
    }
}

/// Consumes and returns the next token's text as a string argument, or a
/// [`SeqDiag::Missing`] if the stream ended.
fn take_string(
    tokens: &[Token],
    idx: &mut usize,
    command: &str,
    line: usize,
) -> Result<String, SeqDiag> {
    match tokens.get(*idx) {
        Some(token) => {
            *idx += 1;
            Ok(token.text.clone())
        }
        None => Err(SeqDiag::Missing {
            command: command.to_owned(),
            line,
        }),
    }
}

/// Consumes the next token as a decimal `u32` argument, or a
/// [`SeqDiag::Missing`]/[`SeqDiag::BadNumber`] on end-of-stream or non-numeric
/// input.
fn take_number(
    tokens: &[Token],
    idx: &mut usize,
    command: &str,
    line: usize,
) -> Result<u32, SeqDiag> {
    match tokens.get(*idx) {
        None => Err(SeqDiag::Missing {
            command: command.to_owned(),
            line,
        }),
        Some(token) => {
            *idx += 1;
            token.text.parse::<u32>().map_err(|_| SeqDiag::BadNumber {
                command: command.to_owned(),
                value: token.text.clone(),
                line: token.line,
            })
        }
    }
}

/// Parses one command from `token` and any argument tokens it consumes.
fn parse_command(
    token: &Token,
    tokens: &[Token],
    idx: &mut usize,
) -> Result<SndSeqCommand, SeqDiag> {
    let command = token.text.clone();
    let line = token.line;
    match command.to_ascii_lowercase().as_str() {
        "play" => Ok(SndSeqCommand::Play(take_string(
            tokens, idx, &command, line,
        )?)),
        "playuntildone" => Ok(SndSeqCommand::PlayUntilDone(take_string(
            tokens, idx, &command, line,
        )?)),
        "playrepeat" => Ok(SndSeqCommand::PlayRepeat(take_string(
            tokens, idx, &command, line,
        )?)),
        "stopsound" => Ok(SndSeqCommand::StopSound(take_string(
            tokens, idx, &command, line,
        )?)),
        "playtime" => {
            let sound = take_string(tokens, idx, &command, line)?;
            let tics = take_number(tokens, idx, &command, line)?;
            Ok(SndSeqCommand::PlayTime { sound, tics })
        }
        "delay" => Ok(SndSeqCommand::Delay(take_number(
            tokens, idx, &command, line,
        )?)),
        "volume" => Ok(SndSeqCommand::Volume(take_number(
            tokens, idx, &command, line,
        )?)),
        "delayrand" => {
            let min = take_number(tokens, idx, &command, line)?;
            let max = take_number(tokens, idx, &command, line)?;
            Ok(SndSeqCommand::DelayRand { min, max })
        }
        "end" => Ok(SndSeqCommand::End),
        _ => Err(SeqDiag::Unknown { command, line }),
    }
}

/// Hexen's sound-sequence script (`SNDSEQ`; engine reference:
/// `SN_InitSequenceScript`, `src/hexen/sn_sonix.c:177-307`).
///
/// A sequence begins at a token starting with `:` (the remainder is its name)
/// and runs until an `end` command, collecting the nine typed
/// [`SndSeqCommand`]s. Everywhere Chocolate Hexen aborts the process, this
/// parser returns a strict error or performs lenient warning-with-recovery
/// (ADR-0023 §3):
///
/// - a `:` inside an open sequence — strict [`AudioError::SndSeqNestedSequence`];
///   lenient closes the open sequence and starts the new one;
/// - an unrecognized command token — strict [`AudioError::SndSeqUnknownCommand`];
///   lenient skips it;
/// - a command outside any sequence — strict
///   [`AudioError::SndSeqCommandOutsideSequence`]; lenient skips it;
/// - a command missing its argument(s) at the end of the lump — strict
///   [`AudioError::SndSeqMissingArgument`]; lenient drops the partial command;
/// - a non-numeric argument — strict [`AudioError::SndSeqBadNumber`]; lenient
///   skips the command;
/// - the lump ending with a sequence still open — strict
///   [`AudioError::SndSeqUnterminatedSequence`]; lenient keeps the partial
///   sequence.
///
/// The engine's `SS_MAX_SCRIPTS = 64` and temp-buffer caps are **not** enforced:
/// allocation is bounded by the input length, so no separate limit is needed.
#[derive(Debug, Clone)]
pub struct SndSeq {
    sequences: Vec<SndSeqSequence>,
    warnings: Vec<AudioWarning>,
}

impl SndSeq {
    /// Parses a `SNDSEQ` sound-sequence lump.
    ///
    /// # Errors
    ///
    /// Returns the strict-mode [`AudioError`] variants listed on the type when
    /// the corresponding malformed construct is encountered in
    /// [`Strictness::Strict`]; [`Strictness::Lenient`] records the mirror
    /// [`AudioWarning`] and recovers as documented. A token longer than 63 bytes
    /// is an [`AudioWarning::OversizedTokens`] aggregate in **both** modes.
    pub fn parse(bytes: &[u8], options: &ParseOptions) -> Result<Self, AudioError> {
        let (tokens, oversized) = tokenize(bytes);
        let mut sequences = Vec::new();
        let mut warnings = Vec::new();
        if oversized > 0 {
            warnings.push(AudioWarning::OversizedTokens { count: oversized });
        }

        let strictness = options.strictness;
        let mut open: Option<SndSeqSequence> = None;
        let mut open_line = 0usize;
        let mut idx = 0usize;

        while idx < tokens.len() {
            let token = tokens[idx].clone();
            idx += 1;

            if let Some(name) = token.text.strip_prefix(':') {
                if let Some(sequence) = open.take() {
                    resolve(
                        strictness,
                        &mut warnings,
                        AudioError::SndSeqNestedSequence {
                            name: name.to_owned(),
                            line: token.line,
                        },
                        AudioWarning::SndSeqNestedSequence {
                            name: name.to_owned(),
                            line: token.line,
                        },
                    )?;
                    // Lenient recovery: close the previously open sequence.
                    sequences.push(sequence);
                }
                open = Some(SndSeqSequence {
                    name: name.to_owned(),
                    commands: Vec::new(),
                });
                open_line = token.line;
                continue;
            }

            if open.is_none() {
                resolve(
                    strictness,
                    &mut warnings,
                    AudioError::SndSeqCommandOutsideSequence {
                        token: token.text.clone(),
                        line: token.line,
                    },
                    AudioWarning::SndSeqCommandOutsideSequence {
                        token: token.text,
                        line: token.line,
                    },
                )?;
                continue;
            }

            match parse_command(&token, &tokens, &mut idx) {
                Ok(command) => {
                    let is_end = matches!(command, SndSeqCommand::End);
                    // `open` is `Some` here (the block above returns or
                    // continues otherwise), so `if let` never falls through.
                    if let Some(sequence) = open.as_mut() {
                        sequence.commands.push(command);
                    }
                    if is_end {
                        if let Some(sequence) = open.take() {
                            sequences.push(sequence);
                        }
                    }
                }
                Err(diag) => match strictness {
                    Strictness::Strict => return Err(diag.into_error()),
                    Strictness::Lenient => warnings.push(diag.into_warning()),
                },
            }
        }

        if let Some(sequence) = open.take() {
            resolve(
                strictness,
                &mut warnings,
                AudioError::SndSeqUnterminatedSequence {
                    name: sequence.name.clone(),
                    line: open_line,
                },
                AudioWarning::SndSeqUnterminatedSequence {
                    name: sequence.name.clone(),
                    line: open_line,
                },
            )?;
            // Lenient recovery: keep the partial sequence.
            sequences.push(sequence);
        }

        Ok(Self {
            sequences,
            warnings,
        })
    }

    /// The parsed sound sequences, in file order.
    #[must_use]
    pub fn sequences(&self) -> &[SndSeqSequence] {
        &self.sequences
    }

    /// Non-fatal issues recorded during parsing.
    #[must_use]
    pub fn warnings(&self) -> &[AudioWarning] {
        &self.warnings
    }
}
