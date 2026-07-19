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
//! [`AudioKind::Mus`], [`AudioKind::Midi`], and [`AudioKind::Wav`] have typed
//! parsers alongside the sound-effect formats: [`MusScore`] decodes the MUS
//! event stream, [`MidiInfo`] indexes standard-MIDI chunks, and [`WavSound`]
//! walks RIFF/WAVE containers (ADR-0023 §2, §4).
//!
//! The instrument banks [`Genmidi`] (the fixed-layout `GENMIDI` OPL bank) and
//! [`Dmxgus`] (the line-oriented `DMXGUS`/`DMXGUSC` patch map) round out the
//! layer (ADR-0023 §2). Unlike the sound formats these have no content magic
//! [`AudioKind::detect`] keys on — they are identified by their reserved lump
//! names — so callers dispatch to [`Genmidi::parse`] / [`Dmxgus::parse`] by
//! name rather than through the classifier.

mod banks;
mod containers;
mod music;
mod sfx;

pub use banks::{Dmxgus, DmxgusEntry, Genmidi, GenmidiInstrument, GenmidiOp, GenmidiVoice};
pub use containers::{MidiInfo, MidiTrack, WavSound};
pub use music::{MusEvent, MusEventKind, MusScore};
pub use sfx::{DmxSound, PcSpeakerSound};

/// The content-detected format of an audio lump (ADR-0023 §1).
///
/// Returned by [`AudioKind::detect`], a pure classifier over lump bytes.
/// Each variant other than [`Unknown`](AudioKind::Unknown) has a typed parser:
/// [`DmxSound`], [`PcSpeakerSound`], [`MusScore`], [`MidiInfo`], and
/// [`WavSound`].
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
    /// The MUS music format (`MUS\x1a` magic). Parsed by [`MusScore`].
    Mus,
    /// A standard MIDI file (`MThd` magic). Indexed by [`MidiInfo`].
    Midi,
    /// A RIFF/WAVE container (`RIFF....WAVE` magic). Parsed by [`WavSound`].
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
    /// The lump's leading magic does not match the format this parser decodes
    /// (MUS `MUS\x1a`, MIDI `MThd`, or WAV `RIFF`/`WAVE`). Unrecoverable in
    /// **both** modes — the bytes are not that format at all.
    #[error("bad magic: expected {expected:?}, found {found:?}")]
    BadMagic {
        /// The magic bytes this parser requires.
        expected: &'static [u8],
        /// The leading bytes read from disk — as many as the expected magic
        /// has, clamped to the lump length, so a mismatch anywhere in the
        /// magic (including past byte 4 of GENMIDI's 8-byte `#OPL_II#`) is
        /// visible in the diagnostics.
        found: Vec<u8>,
    },
    /// A MUS `score_start` offset points past the end of the lump. Strict only
    /// — lenient recovers by yielding an empty event list and records
    /// [`AudioWarning::OffsetOutOfBounds`].
    #[error("offset {offset} is out of bounds for a {lump_len}-byte lump")]
    OffsetOutOfBounds {
        /// The out-of-bounds byte offset.
        offset: usize,
        /// The lump's actual length.
        lump_len: usize,
    },
    /// A MUS event's payload (or its trailing delta-time varint) ran past the
    /// end of the lump. Strict only — lenient keeps the events decoded so far
    /// and records [`AudioWarning::TruncatedEvent`].
    #[error("MUS event stream truncated at offset {offset}")]
    TruncatedEvent {
        /// The byte offset at which the read ran out of bytes.
        offset: usize,
    },
    /// A MUS system event carries a controller outside the valid `10..=14`
    /// range. Strict only — lenient stops at the event and records
    /// [`AudioWarning::InvalidSystemController`].
    #[error("MUS system event controller {controller} is outside 10..=14 (offset {offset})")]
    InvalidSystemController {
        /// The out-of-range controller number.
        controller: u8,
        /// The byte offset of the offending event's descriptor.
        offset: usize,
    },
    /// A MUS change-controller event carries a controller outside the valid
    /// `0..=9` range (`0` is the patch change). Strict only — lenient stops at
    /// the event and records [`AudioWarning::InvalidController`].
    #[error("MUS change-controller {controller} is outside 0..=9 (offset {offset})")]
    InvalidController {
        /// The out-of-range controller number.
        controller: u8,
        /// The byte offset of the offending event's descriptor.
        offset: usize,
    },
    /// A MUS descriptor byte selects an event type this format does not define
    /// (the `0x50` and `0x70` type bits). Strict only — lenient stops at the
    /// event and records [`AudioWarning::UnknownEventType`].
    #[error("MUS unknown event type {event_type:#04x} (offset {offset})")]
    UnknownEventType {
        /// The unrecognized event-type bits (descriptor byte masked with
        /// `0x70`).
        event_type: u8,
        /// The byte offset of the offending event's descriptor.
        offset: usize,
    },
    /// The MUS event stream reached the end of the lump without a score-end
    /// event (the engine reads EOF here and fails). Strict only — lenient
    /// keeps the events decoded so far and records
    /// [`AudioWarning::MissingScoreEnd`].
    #[error("MUS event stream ended at offset {offset} without a score-end event")]
    MissingScoreEnd {
        /// The byte offset at which the stream ended.
        offset: usize,
    },
    /// A MIDI `MThd` header declares a chunk size other than the required `6`.
    /// Strict only — lenient reads the standard six bytes and records
    /// [`AudioWarning::UnexpectedChunkSize`].
    #[error("MIDI header chunk size is {found}; expected {expected}")]
    UnexpectedChunkSize {
        /// The chunk size the format requires (`6`).
        expected: u32,
        /// The raw on-disk chunk size.
        found: u32,
    },
    /// A declared MIDI or WAV chunk length overruns the bytes remaining in the
    /// lump. Strict only — lenient stops the walk and records
    /// [`AudioWarning::ChunkOverrun`].
    #[error("chunk at offset {offset} declares {declared} bytes but only {available} remain")]
    ChunkOverrun {
        /// The byte offset of the offending chunk header.
        offset: usize,
        /// The chunk's declared payload length.
        declared: usize,
        /// The bytes available after the chunk header.
        available: usize,
    },
    /// A WAV `fmt ` chunk is smaller than the 16-byte canonical PCM format
    /// body. Strict only — lenient skips the chunk and records
    /// [`AudioWarning::FmtChunkTooSmall`].
    #[error("WAV fmt chunk is {size} bytes; at least 16 are required")]
    FmtChunkTooSmall {
        /// The declared `fmt ` chunk size.
        size: usize,
    },
    /// A WAV lump ended without a required `fmt ` or `data` chunk. Strict only
    /// — lenient defaults the missing fields to `0` / empty data and records
    /// [`AudioWarning::MissingChunk`].
    #[error("WAV lump is missing the {} chunk", String::from_utf8_lossy(id))]
    MissingChunk {
        /// The identifier of the missing chunk (`fmt ` or `data`).
        id: [u8; 4],
    },
    /// A `DMXGUS` data line has fewer than six comma-separated fields, or an id
    /// field is not a decimal `u32`. Strict only — lenient skips the line and
    /// records [`AudioWarning::MalformedGusLine`]. Deliberately stricter than
    /// the engine's `atoi`, which yields `0` on garbage (`gusconf.c:64-153`).
    #[error("malformed DMXGUS data line {line}")]
    MalformedGusLine {
        /// The 1-based line number of the offending line.
        line: usize,
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
    ///
    /// Only emitted when the structural length invariant holds
    /// (`32 <= length <= lump_len - 8`) — an out-of-range length is
    /// surfaced by [`LengthOutOfRange`](AudioWarning::LengthOutOfRange)
    /// instead, never by both.
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
    /// A MUS instrument list could not be read because it does not fit ahead of
    /// `score_start` or inside the lump; an empty instrument list is used
    /// instead. Recorded in **both** modes (ADR-0023 §2 — the engine never
    /// reads the list).
    #[error("MUS instrument list is unreadable; using an empty list")]
    InstrumentListUnreadable,
    /// A MUS `score_start + score_length` extends past the lump end. Recorded
    /// in **both** modes as a warning only — the engine never uses
    /// `score_length` for bounds (ADR-0023 §2).
    #[error("MUS declared score end {declared_end} exceeds the {lump_len}-byte lump")]
    ScoreLengthOverrun {
        /// The byte offset `score_start + score_length` points at.
        declared_end: usize,
        /// The lump's actual length.
        lump_len: usize,
    },
    /// A MUS `score_start` offset out of bounds was recovered during lenient
    /// parsing: an empty event list is used. Mirrors
    /// [`AudioError::OffsetOutOfBounds`].
    #[error("offset {offset} is out of bounds for a {lump_len}-byte lump; no events decoded")]
    OffsetOutOfBounds {
        /// The out-of-bounds byte offset.
        offset: usize,
        /// The lump's actual length.
        lump_len: usize,
    },
    /// A truncated MUS event was recovered during lenient parsing: decoding
    /// stops, keeping the events read so far. Mirrors
    /// [`AudioError::TruncatedEvent`].
    #[error("MUS event stream truncated at offset {offset}; kept the events decoded so far")]
    TruncatedEvent {
        /// The byte offset at which the read ran out of bytes.
        offset: usize,
    },
    /// A MUS system-event controller outside `10..=14` was recovered during
    /// lenient parsing: decoding stops at the event. Mirrors
    /// [`AudioError::InvalidSystemController`].
    #[error(
        "MUS system event controller {controller} is outside 10..=14 (offset {offset}); stopped decoding"
    )]
    InvalidSystemController {
        /// The out-of-range controller number.
        controller: u8,
        /// The byte offset of the offending event's descriptor.
        offset: usize,
    },
    /// A MUS change-controller outside `0..=9` was recovered during lenient
    /// parsing: decoding stops at the event. Mirrors
    /// [`AudioError::InvalidController`].
    #[error(
        "MUS change-controller {controller} is outside 0..=9 (offset {offset}); stopped decoding"
    )]
    InvalidController {
        /// The out-of-range controller number.
        controller: u8,
        /// The byte offset of the offending event's descriptor.
        offset: usize,
    },
    /// A MUS unknown event type was recovered during lenient parsing: decoding
    /// stops at the event. Mirrors [`AudioError::UnknownEventType`].
    #[error("MUS unknown event type {event_type:#04x} (offset {offset}); stopped decoding")]
    UnknownEventType {
        /// The unrecognized event-type bits (descriptor byte masked with
        /// `0x70`).
        event_type: u8,
        /// The byte offset of the offending event's descriptor.
        offset: usize,
    },
    /// A MUS event stream that ended without a score-end event was recovered
    /// during lenient parsing: the events decoded so far are kept. Mirrors
    /// [`AudioError::MissingScoreEnd`].
    #[error(
        "MUS event stream ended at offset {offset} without a score-end event; kept the events decoded so far"
    )]
    MissingScoreEnd {
        /// The byte offset at which the stream ended.
        offset: usize,
    },
    /// A MIDI `MThd` chunk size other than `6` was recovered during lenient
    /// parsing: the standard six bytes are read. Mirrors
    /// [`AudioError::UnexpectedChunkSize`].
    #[error("MIDI header chunk size is {found}; expected {expected}; read the standard six bytes")]
    UnexpectedChunkSize {
        /// The chunk size the format requires (`6`).
        expected: u32,
        /// The raw on-disk chunk size.
        found: u32,
    },
    /// A declared MIDI or WAV chunk length overrunning the lump was recovered
    /// during lenient parsing: the walk stops. Mirrors
    /// [`AudioError::ChunkOverrun`].
    #[error(
        "chunk at offset {offset} declares {declared} bytes but only {available} remain; stopped the walk"
    )]
    ChunkOverrun {
        /// The byte offset of the offending chunk header.
        offset: usize,
        /// The chunk's declared payload length.
        declared: usize,
        /// The bytes available after the chunk header.
        available: usize,
    },
    /// A non-`MTrk` MIDI chunk was skipped by its declared length. The SMF
    /// specification permits alien chunks, so this is a warning in **both**
    /// modes.
    #[error("skipped a non-MTrk MIDI chunk ({})", String::from_utf8_lossy(id))]
    AlienChunk {
        /// The identifier of the skipped chunk.
        id: [u8; 4],
    },
    /// The number of `MTrk` chunks recorded disagrees with the `MThd` header's
    /// declared track count. Recorded in **both** modes.
    #[error("MIDI header declares {declared} track(s) but {found} MTrk chunk(s) were found")]
    TrackCountMismatch {
        /// The header's declared track count.
        declared: u16,
        /// The number of `MTrk` chunks actually recorded.
        found: usize,
    },
    /// Trailing bytes too short to form a chunk header (fewer than 8) remain at
    /// the end of a MIDI or WAV lump. Recorded in **both** modes.
    #[error("{len} trailing byte(s) too short to form a chunk header")]
    TrailingBytes {
        /// The number of trailing bytes.
        len: usize,
    },
    /// A WAV `RIFF` size field disagrees with the lump length (`8 + riff_size
    /// != lump_len`). Recorded in **both** modes.
    #[error("WAV riff size implies an end of {declared_end} but the lump is {lump_len} bytes")]
    RiffSizeMismatch {
        /// The byte offset `8 + riff_size` points at.
        declared_end: usize,
        /// The lump's actual length.
        lump_len: usize,
    },
    /// A WAV `fmt ` chunk smaller than 16 bytes was skipped during lenient
    /// parsing. Mirrors [`AudioError::FmtChunkTooSmall`].
    #[error("WAV fmt chunk is {size} bytes; at least 16 are required; skipped it")]
    FmtChunkTooSmall {
        /// The declared `fmt ` chunk size.
        size: usize,
    },
    /// A second `fmt ` or `data` chunk was found in a WAV lump; the first wins
    /// and the duplicate is ignored. Recorded in **both** modes.
    #[error("ignored a duplicate WAV {} chunk", String::from_utf8_lossy(id))]
    DuplicateChunk {
        /// The identifier of the duplicated chunk (`fmt ` or `data`).
        id: [u8; 4],
    },
    /// A required WAV `fmt ` or `data` chunk was missing and defaulted during
    /// lenient parsing (fields `0` / empty data). Mirrors
    /// [`AudioError::MissingChunk`].
    #[error(
        "WAV lump is missing the {} chunk; defaulted it",
        String::from_utf8_lossy(id)
    )]
    MissingChunk {
        /// The identifier of the missing chunk (`fmt ` or `data`).
        id: [u8; 4],
    },
    /// A `GENMIDI` lump's leading bytes are not `#OPL_II#`; lenient parsing
    /// proceeds anyway, as the engine never checks the magic
    /// (`i_oplmusic.c:369`). Recorded only in lenient mode — strict fails with
    /// [`AudioError::BadMagic`].
    #[error("bad magic: expected {expected:?}, found {found:?}; parsed anyway")]
    BadMagic {
        /// The magic bytes the format requires.
        expected: &'static [u8],
        /// The leading bytes read from disk — as many as the expected magic
        /// has, clamped to the lump length.
        found: Vec<u8>,
    },
    /// A `GENMIDI` lump is shorter than the fixed 11908-byte extent; lenient
    /// parsing decoded every complete record and name field that fits. Recorded
    /// only in lenient mode — strict fails with [`AudioError::TruncatedHeader`].
    #[error("GENMIDI lump is {len} bytes; {needed} needed for the full bank; decoded what fit")]
    TruncatedBank {
        /// The lump's actual length.
        len: usize,
        /// Bytes required for the complete bank (`11908`).
        needed: usize,
    },
    /// A `DMXGUS` data line was malformed (fewer than six fields, or a
    /// non-numeric id) and skipped during lenient parsing. Mirrors
    /// [`AudioError::MalformedGusLine`].
    #[error("malformed DMXGUS data line {line}; skipped it")]
    MalformedGusLine {
        /// The 1-based line number of the skipped line.
        line: usize,
    },
    /// A `DMXGUS` data line carried more than the six fields the format defines;
    /// the extras were ignored. Recorded in **both** modes.
    #[error("DMXGUS data line {line} has {extra} field(s) beyond the six defined; ignored them")]
    ExtraGusFields {
        /// The 1-based line number of the line.
        line: usize,
        /// How many fields beyond the sixth were present.
        extra: usize,
    },
}
