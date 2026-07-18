//! Classic audio lumps (ADR-0023): the DMX digital-sound (`DS*`) and
//! PC-speaker (`DP*`) sound-effect formats, plus content-first
//! identification via [`AudioKind::detect`]. Decoding is dependency-free and
//! lives in the core crate — the same precedent as map records and the
//! classic graphics tiers (ADR-0022 §3): no feature flag, no new
//! dependencies, every parser a bounded read over `&[u8]`.
//!
//! # Content-first identification
//!
//! Audio lumps are identified by **content, never by name** (ADR-0023 §1).
//! The retail survey produced three independent falsifications of name-based
//! detection:
//!
//! - Freedoom ships standard-MIDI bytes under the same `D_*` names id ships
//!   MUS under — the name convention does not determine the content format.
//! - `strife1.wad`'s sprite `DSTAA0` (and any sprite whose name starts `DS`)
//!   collides with the sound-effect prefix; only the bytes distinguish a
//!   picture header from a sound.
//! - Hexen's sound-effect names are data-driven through its `SNDINFO` lump
//!   and its music names through MAPINFO — there is no name convention to
//!   key on at all.
//!
//! [`AudioKind::detect`] is therefore a pure classifier keyed on content:
//! it never allocates and never errors. The per-format `parse` constructors
//! ([`DmxSound::parse`], [`PcSpeakerSound::parse`]) own their format's
//! validation under the standard strictness model.
//!
//! Vanilla's actual detection rule is documented but deliberately **not**
//! replicated: the engine sniffs only `MThd` and additionally requires
//! `len < 96 KiB` (`MAXMIDLENGTH`), routing everything else — including a
//! large valid MIDI — into the MUS converter, and it never checks the MUS
//! magic at all. That 96 KiB cap is a vanilla playback limitation, not a
//! format property, so it is not reproduced here (ADR-0023 §1).
//!
//! Detection is a heuristic: the PC-speaker shape in particular is weak —
//! an all-zeros 8-byte lump satisfies it — so classification only proposes a
//! format; the parsers do the validating.
//!
//! [`AudioKind::Mus`], [`AudioKind::Midi`], and [`AudioKind::Wav`] are
//! recognized here so callers can route lumps today, but their typed parsers
//! land with ADR-0023 stage 2 (#301).

mod sfx;

pub use sfx::{DmxSound, PcSpeakerSound};

/// The content-detected format of an audio lump (ADR-0023 §1).
///
/// Returned by [`AudioKind::detect`], a pure classifier over lump bytes.
/// Only [`Dmx`](AudioKind::Dmx) and [`PcSpeaker`](AudioKind::PcSpeaker) have
/// typed parsers in this stage; the music/container variants are recognized
/// so callers can route lumps, with their parsers arriving in ADR-0023
/// stage 2 (#301).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AudioKind {
    /// The DMX digital-sound format (`format == 3`): unsigned 8-bit mono PCM
    /// behind an 8-byte header, used by `DS*` sound effects. Parsed by
    /// [`DmxSound`].
    Dmx,
    /// The PC-speaker format (`format == 0`): a run of tone bytes behind a
    /// 4-byte header, used by `DP*` sound effects. Parsed by
    /// [`PcSpeakerSound`].
    PcSpeaker,
    /// The MUS music format (`MUS\x1a` magic). Recognized here; typed parser
    /// lands with ADR-0023 stage 2 (#301).
    Mus,
    /// A standard MIDI file (`MThd` magic). Recognized here; typed parser
    /// lands with ADR-0023 stage 2 (#301).
    Midi,
    /// A RIFF/WAVE container (`RIFF....WAVE` magic). Recognized here; typed
    /// parser lands with ADR-0023 stage 2 (#301).
    Wav,
    /// The bytes match no recognized audio format.
    Unknown,
}

impl AudioKind {
    /// Classifies a lump by content alone: never errors, never allocates
    /// (ADR-0023 §1). Recognizes, in order of magic specificity, standard
    /// MIDI (`MThd`), WAV (`RIFF....WAVE`), MUS (`MUS\x1a`), the DMX
    /// digital-sound shape (`format == 3` plus its header arithmetic), and
    /// the PC-speaker shape (`format == 0` plus its arithmetic); anything
    /// else is [`AudioKind::Unknown`].
    ///
    /// This is a classifier, not a validator — a returned [`Dmx`](AudioKind::Dmx)
    /// or [`PcSpeaker`](AudioKind::PcSpeaker) means the bytes satisfy the
    /// shape checks a strict `parse` also requires, so a strict parse of the
    /// matching type succeeds; the weaker music/container magics only mean
    /// the leading bytes match.
    #[must_use]
    pub fn detect(bytes: &[u8]) -> AudioKind {
        let len = bytes.len();

        if len >= 4 && bytes[..4] == *b"MThd" {
            return AudioKind::Midi;
        }
        if len >= 12 && bytes[..4] == *b"RIFF" && bytes[8..12] == *b"WAVE" {
            return AudioKind::Wav;
        }
        if len >= 4 && bytes[..4] == [0x4D, 0x55, 0x53, 0x1A] {
            return AudioKind::Mus;
        }
        if len >= 8 {
            let format = u16::from_le_bytes([bytes[0], bytes[1]]);
            let length = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
            if format == 3 && length >= 32 && (length as usize) <= len - 8 {
                return AudioKind::Dmx;
            }
        }
        if len >= 4 {
            let format = u16::from_le_bytes([bytes[0], bytes[1]]);
            let count = u16::from_le_bytes([bytes[2], bytes[3]]);
            if format == 0 && usize::from(count) <= len - 4 {
                return AudioKind::PcSpeaker;
            }
        }
        AudioKind::Unknown
    }
}

/// A fatal problem decoding a classic audio lump in strict mode; the
/// recoverable variant's lenient recovery is described on the matching
/// [`AudioWarning`].
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    /// The lump is shorter than the format's fixed header (8 bytes for DMX,
    /// 4 bytes for PC-speaker). Unrecoverable in **both** modes — there is
    /// no header to read.
    #[error("audio lump is {len} bytes; {needed} needed for the header")]
    TruncatedHeader {
        /// The lump's actual length.
        len: usize,
        /// Bytes required for the fixed header.
        needed: usize,
    },
    /// The lump's format field is not the value this parser decodes (DMX
    /// requires `3`, PC-speaker requires `0`). Unrecoverable in **both**
    /// modes — the bytes are not that format at all.
    #[error("audio format is {found}; expected {expected}")]
    UnexpectedFormat {
        /// The format value this parser requires.
        expected: u16,
        /// The raw on-disk format value.
        found: u16,
    },
    /// A declared length or tone count falls outside its structural bounds
    /// (DMX: `32 <= length <= lump_len - 8`; PC-speaker:
    /// `count <= lump_len - 4`). Strict only — lenient recovers by clamping
    /// an overrun to the available bytes, or yielding empty samples for a
    /// DMX length below the 32-byte pad minimum, recording the mirror
    /// [`AudioWarning::LengthOutOfRange`].
    #[error("declared length {length} is out of range (min {min}, {available} available)")]
    LengthOutOfRange {
        /// The declared length or tone count.
        length: u32,
        /// The minimum the format requires (`32` for DMX, `0` for
        /// PC-speaker).
        min: u32,
        /// The bytes available after the header.
        available: usize,
    },
}

/// A non-fatal issue recovered while decoding a classic audio lump; the
/// recoverable strict [`AudioError`] is mirrored by
/// [`LengthOutOfRange`](AudioWarning::LengthOutOfRange), while the remaining
/// variants describe anomalies tolerated in **both** strictness modes.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AudioWarning {
    /// A declared length or tone count out of range was recovered during
    /// lenient parsing: a DMX/PC-speaker overrun clamps to the available
    /// bytes, and a DMX length below the 32-byte pad minimum yields empty
    /// samples. Mirrors [`AudioError::LengthOutOfRange`].
    #[error(
        "declared length {length} is out of range (min {min}, {available} available); clamped during lenient parsing"
    )]
    LengthOutOfRange {
        /// The declared length or tone count.
        length: u32,
        /// The minimum the format requires (`32` for DMX, `0` for
        /// PC-speaker).
        min: u32,
        /// The bytes available after the header.
        available: usize,
    },
    /// The lump is longer than its header plus declared length; the trailing
    /// bytes are ignored, as the engine ignores them. Recorded in **both**
    /// modes — the retail survey shows exact equality everywhere, so slack
    /// is unusual but not malformed (ADR-0023 §2).
    #[error("lump is {lump_len} bytes but the header and declared length end at {expected}")]
    TrailingSlack {
        /// The byte offset the header plus declared length ends at.
        expected: usize,
        /// The lump's actual length.
        lump_len: usize,
    },
    /// A DMX lump's declared length is at or below the engine's `48`-byte
    /// playability floor (`src/i_sdlsound.c:759`) — vanilla rejects it as
    /// unplayable, though its own comment calls the cut-off approximate.
    /// Recorded in **both** modes: the floor is a playability heuristic, not
    /// a structural property (ADR-0023 §2), so the lump still parses.
    #[error("DMX declared length {length} is at or below the playability floor of 48 bytes")]
    PlayabilityFloor {
        /// The declared length.
        length: u32,
    },
    /// A DMX lump's sample-rate field is `0`; the engine performs no guard
    /// and would divide by zero downstream, so a parser must not propagate
    /// the hazard silently (ADR-0023 §2). Recorded in **both** modes.
    #[error("DMX sample rate is zero")]
    ZeroSampleRate,
    /// A PC-speaker lump carries tone bytes outside the 128-entry divisor
    /// table (values `>= 128`), which the engine renders as silence
    /// (ADR-0023 §2). Aggregated to one warning carrying the total count so
    /// the output stays `O(1)`. Recorded in **both** modes.
    #[error("{count} PC-speaker tone byte(s) index outside the 128-entry divisor table")]
    OutOfRangeTones {
        /// How many tone bytes were `>= 128`.
        count: usize,
    },
}
