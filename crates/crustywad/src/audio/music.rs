//! The MUS music format (ADR-0023 §2): [`MusScore`], a bounded, typed decode
//! of the DMX MUS event stream (engine reference: chocolate-doom
//! `src/mus2mid.c:57-667`). This is an event decoder, not a MIDI generator —
//! the MUS→MIDI controller mapping and file emission are CLI staging (#304),
//! deliberately out of scope here.
//!
//! Decoding is a single bounded pass over `&[u8]`: the header is fixed, the
//! event stream is walked iteratively from `score_start` to the lump end (the
//! reference converter ignores `score_length` for bounds — it seeks straight
//! to `score_start` and reads until score-end or EOF), and every read is
//! bounds-checked. Lenient-mode warnings are carried inside the returned value
//! and exposed by [`MusScore::warnings`], following the shipped gfx idiom.

use crate::{ParseOptions, Strictness};

use super::{AudioError, AudioWarning};

/// The typed kind of a single MUS event (engine reference:
/// `src/mus2mid.c:511-667`). The descriptor byte's bits 6-4 select the kind;
/// bits 3-0 are the channel (carried by [`MusEvent::channel`]) and bit 7
/// signals a trailing delta-time (carried by [`MusEvent::delay`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MusEventKind {
    /// Release a key (type `0x00`): stop the note. Payload is one byte; the
    /// note is its low 7 bits.
    ReleaseKey {
        /// The note number (0-127).
        note: u8,
    },
    /// Press a key (type `0x10`): start the note. The payload key byte's
    /// low 7 bits are the note; when its high bit is set a second byte
    /// follows whose low 7 bits are the velocity.
    PressKey {
        /// The note number (0-127).
        note: u8,
        /// The velocity (0-127) when the key byte carried one, else `None`
        /// (the engine reuses the channel's last volume).
        velocity: Option<u8>,
    },
    /// Bend the pitch wheel (type `0x20`): payload is one raw byte (the MUS
    /// wheel value, `0..=255`, mapped to a 14-bit MIDI bend downstream).
    PitchWheel {
        /// The raw pitch-wheel value as read from disk.
        value: u8,
    },
    /// A MUS system event (type `0x30`): payload is one controller byte, which
    /// the format constrains to `10..=14` (all-sounds-off, all-notes-off,
    /// mono, poly, reset-all-controllers).
    SystemEvent {
        /// The controller number (`10..=14`).
        controller: u8,
    },
    /// A change-controller event (type `0x40`): payload is a controller byte
    /// and a value byte. Controller `0` is a patch (instrument) change; `1..=9`
    /// are valued controllers.
    ChangeController {
        /// The controller number (`0` = patch change, `1..=9` valued).
        controller: u8,
        /// The controller value.
        value: u8,
    },
    /// The score-end event (type `0x60`): terminates the stream. Bytes after
    /// it are ignored, as the engine ignores them.
    ScoreEnd,
}

/// A single decoded MUS event: its channel, typed [`MusEventKind`], and the
/// delta-time that follows it in the stream (`0` when the descriptor byte's
/// high bit was clear).
///
/// The delta-time is a base-128 big-endian varint accumulated with saturating
/// arithmetic — a hostile varint that would exceed [`u32::MAX`] saturates
/// rather than wrapping or panicking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MusEvent {
    /// The MUS channel (descriptor bits 3-0; channel 15 is percussion).
    pub channel: u8,
    /// The typed event kind.
    pub kind: MusEventKind,
    /// The delta-time (in MUS ticks) that follows this event, or `0` when the
    /// descriptor byte carried no delta. Accumulated with saturating
    /// arithmetic.
    pub delay: u32,
}

/// A parsed MUS music lump (engine reference: struct `musheader`,
/// `src/mus2mid.c:57-65`).
///
/// On-disk layout: 4-byte magic `MUS\x1a`, then five `u16 LE` header fields —
/// `score_length`, `score_start` (byte offset from the lump start to the first
/// event), `primary_channels`, `secondary_channels`, `instrument_count` —
/// followed by `instrument_count` `u16 LE` patch numbers, and finally the
/// event stream at `score_start`.
///
/// The reference converter never reads the instrument list and never uses
/// `score_length` for bounds; this parser reads the instruments when they fit
/// ahead of `score_start`, cross-checks `score_start` against the lump length
/// (a strict error — the engine's equivalent is a failed seek), and treats a
/// `score_start + score_length` overrun as a warning only (matching the
/// engine's indifference to the field).
#[derive(Debug, Clone)]
pub struct MusScore {
    score_length: u16,
    score_start: u16,
    primary_channels: u16,
    secondary_channels: u16,
    instruments: Vec<u16>,
    events: Vec<MusEvent>,
    warnings: Vec<AudioWarning>,
}

impl MusScore {
    /// The fixed header size in bytes.
    const HEADER: usize = 14;
    /// The MUS magic bytes (`MUS\x1a`).
    const MAGIC: [u8; 4] = [0x4D, 0x55, 0x53, 0x1A];

    /// Parses a MUS music lump.
    ///
    /// # Errors
    ///
    /// - [`AudioError::TruncatedHeader`] when the lump is shorter than the
    ///   14-byte header (both modes).
    /// - [`AudioError::BadMagic`] when the leading four bytes are not
    ///   `MUS\x1a` (both modes — the bytes are not a MUS lump at all).
    /// - [`AudioError::OffsetOutOfBounds`] (strict only) when `score_start`
    ///   points past the lump end; lenient mode records
    ///   [`AudioWarning::OffsetOutOfBounds`] and yields an empty event list.
    /// - [`AudioError::TruncatedEvent`], [`AudioError::InvalidSystemController`],
    ///   [`AudioError::InvalidController`], [`AudioError::UnknownEventType`], and
    ///   [`AudioError::MissingScoreEnd`] (strict only) when the event stream is
    ///   malformed. Lenient mode keeps the events decoded so far and records the
    ///   mirroring [`AudioWarning`], never failing past the header and magic
    ///   checks.
    ///
    /// An unreadable instrument list ([`AudioWarning::InstrumentListUnreadable`])
    /// and a `score_start + score_length` overrun
    /// ([`AudioWarning::ScoreLengthOverrun`]) are warnings in **both** modes;
    /// the parse still succeeds.
    pub fn parse(bytes: &[u8], options: &ParseOptions) -> Result<Self, AudioError> {
        let len = bytes.len();
        if len < Self::HEADER {
            return Err(AudioError::TruncatedHeader {
                len,
                needed: Self::HEADER,
            });
        }
        if bytes[..4] != Self::MAGIC {
            return Err(AudioError::BadMagic {
                expected: &Self::MAGIC,
                found: bytes[..4].to_vec(),
            });
        }

        let score_length = u16::from_le_bytes([bytes[4], bytes[5]]);
        let score_start = u16::from_le_bytes([bytes[6], bytes[7]]);
        let primary_channels = u16::from_le_bytes([bytes[8], bytes[9]]);
        let secondary_channels = u16::from_le_bytes([bytes[10], bytes[11]]);
        let instrument_count = u16::from_le_bytes([bytes[12], bytes[13]]);

        let mut warnings = Vec::new();

        // Instrument list: readable only when the whole table fits ahead of the
        // event stream and inside the lump. Otherwise an empty list + warning.
        let instr_end = Self::HEADER + 2 * usize::from(instrument_count);
        let instruments = if instr_end <= usize::from(score_start) && instr_end <= len {
            let mut list = Vec::with_capacity(usize::from(instrument_count));
            let mut off = Self::HEADER;
            for _ in 0..instrument_count {
                list.push(u16::from_le_bytes([bytes[off], bytes[off + 1]]));
                off += 2;
            }
            list
        } else {
            warnings.push(AudioWarning::InstrumentListUnreadable);
            Vec::new()
        };

        let score_start_usize = usize::from(score_start);
        let mut events = Vec::new();

        if score_start_usize > len {
            match options.strictness {
                Strictness::Strict => {
                    return Err(AudioError::OffsetOutOfBounds {
                        offset: score_start_usize,
                        lump_len: len,
                    });
                }
                Strictness::Lenient => {
                    warnings.push(AudioWarning::OffsetOutOfBounds {
                        offset: score_start_usize,
                        lump_len: len,
                    });
                }
            }
            return Ok(Self {
                score_length,
                score_start,
                primary_channels,
                secondary_channels,
                instruments,
                events,
                warnings,
            });
        }

        // score_length overrun is a warning in both modes — the engine never
        // uses the field for bounds.
        let declared_end = score_start_usize + usize::from(score_length);
        if declared_end > len {
            warnings.push(AudioWarning::ScoreLengthOverrun {
                declared_end,
                lump_len: len,
            });
        }

        Self::decode_events(
            bytes,
            options,
            score_start_usize,
            &mut events,
            &mut warnings,
        )?;

        Ok(Self {
            score_length,
            score_start,
            primary_channels,
            secondary_channels,
            instruments,
            events,
            warnings,
        })
    }

    /// Decodes the event stream from `start` to the lump end, appending to
    /// `events`. In strict mode a malformed event returns the matching
    /// [`AudioError`]; in lenient mode it records the mirroring
    /// [`AudioWarning`] and stops, keeping the events already decoded.
    #[allow(clippy::too_many_lines)]
    fn decode_events(
        bytes: &[u8],
        options: &ParseOptions,
        start: usize,
        events: &mut Vec<MusEvent>,
        warnings: &mut Vec<AudioWarning>,
    ) -> Result<(), AudioError> {
        let len = bytes.len();
        let mut pos = start;

        // Reads one byte, advancing `pos`. On EOF it either returns a strict
        // truncation error or records the lenient warning and stops the walk.
        macro_rules! next_byte {
            () => {{
                if pos >= len {
                    match options.strictness {
                        Strictness::Strict => {
                            return Err(AudioError::TruncatedEvent { offset: pos });
                        }
                        Strictness::Lenient => {
                            warnings.push(AudioWarning::TruncatedEvent { offset: pos });
                            return Ok(());
                        }
                    }
                }
                let byte = bytes[pos];
                pos += 1;
                byte
            }};
        }

        loop {
            if pos >= len {
                // The stream ended without a score-end event. The engine reads
                // EOF here and fails; strict mirrors that, lenient warns.
                match options.strictness {
                    Strictness::Strict => return Err(AudioError::MissingScoreEnd { offset: pos }),
                    Strictness::Lenient => {
                        warnings.push(AudioWarning::MissingScoreEnd { offset: pos });
                        return Ok(());
                    }
                }
            }

            let event_offset = pos;
            let descriptor = next_byte!();
            let delta_follows = descriptor & 0x80 != 0;
            let event_type = descriptor & 0x70;
            let channel = descriptor & 0x0F;

            let kind = match event_type {
                0x00 => {
                    let note = next_byte!() & 0x7F;
                    MusEventKind::ReleaseKey { note }
                }
                0x10 => {
                    let key = next_byte!();
                    let velocity = if key & 0x80 != 0 {
                        Some(next_byte!() & 0x7F)
                    } else {
                        None
                    };
                    MusEventKind::PressKey {
                        note: key & 0x7F,
                        velocity,
                    }
                }
                0x20 => MusEventKind::PitchWheel {
                    value: next_byte!(),
                },
                0x30 => {
                    let controller = next_byte!();
                    if !(10..=14).contains(&controller) {
                        match options.strictness {
                            Strictness::Strict => {
                                return Err(AudioError::InvalidSystemController {
                                    controller,
                                    offset: event_offset,
                                });
                            }
                            Strictness::Lenient => {
                                warnings.push(AudioWarning::InvalidSystemController {
                                    controller,
                                    offset: event_offset,
                                });
                                return Ok(());
                            }
                        }
                    }
                    MusEventKind::SystemEvent { controller }
                }
                0x40 => {
                    let controller = next_byte!();
                    let value = next_byte!();
                    if controller > 9 {
                        match options.strictness {
                            Strictness::Strict => {
                                return Err(AudioError::InvalidController {
                                    controller,
                                    offset: event_offset,
                                });
                            }
                            Strictness::Lenient => {
                                warnings.push(AudioWarning::InvalidController {
                                    controller,
                                    offset: event_offset,
                                });
                                return Ok(());
                            }
                        }
                    }
                    MusEventKind::ChangeController { controller, value }
                }
                0x60 => MusEventKind::ScoreEnd,
                other => match options.strictness {
                    Strictness::Strict => {
                        return Err(AudioError::UnknownEventType {
                            event_type: other,
                            offset: event_offset,
                        });
                    }
                    Strictness::Lenient => {
                        warnings.push(AudioWarning::UnknownEventType {
                            event_type: other,
                            offset: event_offset,
                        });
                        return Ok(());
                    }
                },
            };

            if matches!(kind, MusEventKind::ScoreEnd) {
                events.push(MusEvent {
                    channel,
                    kind,
                    delay: 0,
                });
                return Ok(());
            }

            // Delta-time: a base-128 big-endian varint, present only when the
            // descriptor's high bit was set. Saturating arithmetic caps a
            // hostile varint at u32::MAX rather than wrapping.
            let mut delay = 0u32;
            if delta_follows {
                loop {
                    let byte = next_byte!();
                    delay = delay
                        .saturating_mul(128)
                        .saturating_add(u32::from(byte & 0x7F));
                    if byte & 0x80 == 0 {
                        break;
                    }
                }
            }

            events.push(MusEvent {
                channel,
                kind,
                delay,
            });
        }
    }

    /// The declared `score_length` header field (byte length of the event
    /// stream). The engine never uses it for bounds; this parser only
    /// cross-checks it for the [`AudioWarning::ScoreLengthOverrun`] warning.
    #[must_use]
    pub fn score_length(&self) -> u16 {
        self.score_length
    }

    /// The `score_start` header field: the byte offset from the lump start to
    /// the first event.
    #[must_use]
    pub fn score_start(&self) -> u16 {
        self.score_start
    }

    /// The declared primary channel count.
    #[must_use]
    pub fn primary_channels(&self) -> u16 {
        self.primary_channels
    }

    /// The declared secondary channel count.
    #[must_use]
    pub fn secondary_channels(&self) -> u16 {
        self.secondary_channels
    }

    /// The instrument (patch) numbers, empty when the instrument list could
    /// not be read (see [`AudioWarning::InstrumentListUnreadable`]).
    #[must_use]
    pub fn instruments(&self) -> &[u16] {
        &self.instruments
    }

    /// The decoded event stream. A terminating [`MusEventKind::ScoreEnd`] is
    /// included as the final event when the stream reached one.
    #[must_use]
    pub fn events(&self) -> &[MusEvent] {
        &self.events
    }

    /// Non-fatal issues recorded during parsing.
    #[must_use]
    pub fn warnings(&self) -> &[AudioWarning] {
        &self.warnings
    }
}
