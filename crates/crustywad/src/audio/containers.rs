//! Doom 64 container audio (ADR-0023 §2, §4): [`MidiInfo`], a shallow
//! standard-MIDI chunk index, and [`WavSound`], a bounded RIFF/WAVE walk.
//! Both are container parsers for WAD-embedded audio, not codecs — they locate
//! and bounds-check chunk frames without decoding MIDI events or PCM.
//!
//! Every declared chunk length is bounds-checked against the remaining lump
//! before the walk advances. That bound is the point of these types: Doom64
//! EX's reader allocates from the declared track count and advances a data
//! pointer by each track's declared big-endian length with **no bound against
//! the lump end** (`Song_RegisterTracks`, `src/engine/system/i_audio.cc:976-1000`)
//! — the trusted-count surface ADR-0016 exists to close (ADR-0023 §5).

use crate::{ParseOptions, Strictness};

use super::{AudioError, AudioWarning};

/// The span of one `MTrk` chunk's payload inside a MIDI lump, recorded by
/// [`MidiInfo`]. [`MidiInfo`] borrows nothing — a track is described only by
/// its offset and length, so callers slice the original lump bytes themselves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MidiTrack {
    /// The byte offset of the track payload from the lump start (just past the
    /// 8-byte `MTrk` chunk header).
    pub offset: usize,
    /// The track payload length in bytes (the chunk's declared, bounds-checked
    /// length).
    pub length: usize,
}

/// A shallow standard-MIDI (SMF) chunk index (engine reference:
/// `src/midifile.c`; Doom 64 container use per ADR-0023 §4). This validates
/// the 14-byte `MThd` header and walks the chunk frames recording each `MTrk`
/// span; it is **not** an event parser.
///
/// # Zero-length placeholder
///
/// A **zero-length** lump parses successfully as empty ([`format`](MidiInfo::format)
/// `0`, no tracks, no warnings): this is Doom 64's `NOSOUND` placeholder, which
/// Doom64 EX tolerates (`i_audio.cc:1053`). Document and expect this special
/// case — it is not an error.
///
/// # Endianness
///
/// Note the split from the rest of the crate: WAD directory fields are
/// little-endian, but SMF content fields (chunk sizes, format, track count,
/// division) are **big-endian**.
///
/// [`MidiInfo`] borrows nothing from the input: track payloads are described by
/// [`MidiTrack`] offset/length pairs, not copied.
#[derive(Debug, Clone)]
pub struct MidiInfo {
    format: u16,
    declared_tracks: u16,
    division: u16,
    tracks: Vec<MidiTrack>,
    warnings: Vec<AudioWarning>,
}

impl MidiInfo {
    /// The fixed `MThd` header size in bytes.
    const HEADER: usize = 14;
    /// The `MThd` magic bytes.
    const MAGIC: [u8; 4] = *b"MThd";
    /// The `MTrk` chunk identifier.
    const MTRK: [u8; 4] = *b"MTrk";
    /// The chunk-frame header size: a 4-byte id plus a `u32` length.
    const CHUNK_HEADER: usize = 8;
    /// The chunk size the `MThd` header always declares.
    const MTHD_CHUNK_SIZE: u32 = 6;

    /// Parses (indexes) a standard-MIDI lump.
    ///
    /// # Errors
    ///
    /// - A **zero-length** lump is not an error — see the type-level
    ///   documentation for the `NOSOUND` placeholder.
    /// - [`AudioError::TruncatedHeader`] when `0 < len < 14` (both modes).
    /// - [`AudioError::BadMagic`] when the leading four bytes are not `MThd`
    ///   (both modes).
    /// - [`AudioError::UnexpectedChunkSize`] (strict only) when the `MThd`
    ///   chunk size is not `6`; lenient records
    ///   [`AudioWarning::UnexpectedChunkSize`] and reads the standard six bytes.
    /// - [`AudioError::ChunkOverrun`] (strict only) when a declared chunk
    ///   length overruns the remaining lump; lenient records
    ///   [`AudioWarning::ChunkOverrun`] and stops the walk.
    ///
    /// A non-`MTrk` chunk ([`AudioWarning::AlienChunk`]), a recorded `MTrk`
    /// count that disagrees with the header
    /// ([`AudioWarning::TrackCountMismatch`]), and trailing bytes too short for
    /// a chunk header ([`AudioWarning::TrailingBytes`]) are warnings in
    /// **both** modes; the parse still succeeds.
    #[allow(clippy::too_many_lines)]
    pub fn parse(bytes: &[u8], options: &ParseOptions) -> Result<Self, AudioError> {
        let len = bytes.len();

        // Doom 64's zero-length NOSOUND placeholder: an empty, warning-free
        // index (ADR-0023 §4).
        if len == 0 {
            return Ok(Self {
                format: 0,
                declared_tracks: 0,
                division: 0,
                tracks: Vec::new(),
                warnings: Vec::new(),
            });
        }
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

        let mut warnings = Vec::new();

        let chunk_size = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        if chunk_size != Self::MTHD_CHUNK_SIZE {
            match options.strictness {
                Strictness::Strict => {
                    return Err(AudioError::UnexpectedChunkSize {
                        expected: Self::MTHD_CHUNK_SIZE,
                        found: chunk_size,
                    });
                }
                Strictness::Lenient => warnings.push(AudioWarning::UnexpectedChunkSize {
                    expected: Self::MTHD_CHUNK_SIZE,
                    found: chunk_size,
                }),
            }
        }

        let format = u16::from_be_bytes([bytes[8], bytes[9]]);
        let declared_tracks = u16::from_be_bytes([bytes[10], bytes[11]]);
        let division = u16::from_be_bytes([bytes[12], bytes[13]]);

        let mut tracks = Vec::new();
        let mut stopped = false;
        // The first chunk frame starts after the declared MThd chunk, not at
        // a fixed offset: a lenient-accepted extended header (chunk size > 6)
        // would otherwise misalign the walk and read header bytes as a frame.
        // A declaration below the 6 standard bytes keeps the standard layout
        // (the fields above were read regardless, matching the engine). A
        // declared header size overrunning the lump gets the same diagnostic
        // as any other overrunning chunk length rather than a silent clamp.
        // Strict mode never reaches this arm — any non-6 size already failed
        // as `UnexpectedChunkSize` above.
        let mut pos = if chunk_size > Self::MTHD_CHUNK_SIZE {
            let declared = usize::try_from(chunk_size).unwrap_or(usize::MAX);
            if let Some(start) = declared.checked_add(8).filter(|&start| start <= len) {
                start
            } else {
                warnings.push(AudioWarning::ChunkOverrun {
                    offset: 0,
                    declared,
                    available: len - Self::CHUNK_HEADER,
                });
                stopped = true;
                len
            }
        } else {
            Self::HEADER
        };

        while pos + Self::CHUNK_HEADER <= len {
            let id = [bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]];
            let chunk_len = u32::from_be_bytes([
                bytes[pos + 4],
                bytes[pos + 5],
                bytes[pos + 6],
                bytes[pos + 7],
            ]) as usize;
            let available = len - (pos + Self::CHUNK_HEADER);
            if chunk_len > available {
                match options.strictness {
                    Strictness::Strict => {
                        return Err(AudioError::ChunkOverrun {
                            offset: pos,
                            declared: chunk_len,
                            available,
                        });
                    }
                    Strictness::Lenient => {
                        warnings.push(AudioWarning::ChunkOverrun {
                            offset: pos,
                            declared: chunk_len,
                            available,
                        });
                        stopped = true;
                        break;
                    }
                }
            }

            let payload = pos + Self::CHUNK_HEADER;
            if id == Self::MTRK {
                tracks.push(MidiTrack {
                    offset: payload,
                    length: chunk_len,
                });
            } else {
                warnings.push(AudioWarning::AlienChunk { id });
            }
            pos = payload + chunk_len;
        }

        // Trailing bytes too short to be a chunk header (only meaningful when
        // the walk ended by reaching the lump, not by an overrun stop).
        if !stopped && pos < len {
            let remainder = len - pos;
            if remainder < Self::CHUNK_HEADER {
                warnings.push(AudioWarning::TrailingBytes { len: remainder });
            }
        }

        if tracks.len() != usize::from(declared_tracks) {
            warnings.push(AudioWarning::TrackCountMismatch {
                declared: declared_tracks,
                found: tracks.len(),
            });
        }

        Ok(Self {
            format,
            declared_tracks,
            division,
            tracks,
            warnings,
        })
    }

    /// The SMF format field (`0` single-track, `1` simultaneous, `2`
    /// independent). Also `0` for the zero-length placeholder.
    #[must_use]
    pub fn format(&self) -> u16 {
        self.format
    }

    /// The track count declared in the `MThd` header. May disagree with the
    /// number of `MTrk` chunks actually recorded (see
    /// [`AudioWarning::TrackCountMismatch`]).
    #[must_use]
    pub fn declared_tracks(&self) -> u16 {
        self.declared_tracks
    }

    /// The SMF division field (ticks per quarter note, or an SMPTE code).
    #[must_use]
    pub fn division(&self) -> u16 {
        self.division
    }

    /// The recorded `MTrk` track spans, each an offset/length into the original
    /// lump bytes.
    #[must_use]
    pub fn tracks(&self) -> &[MidiTrack] {
        &self.tracks
    }

    /// Non-fatal issues recorded during parsing.
    #[must_use]
    pub fn warnings(&self) -> &[AudioWarning] {
        &self.warnings
    }
}

/// A bounded RIFF/WAVE walk over a WAD-embedded WAV lump (ADR-0023 §2, §4;
/// the shape Doom 64's KEX remaster ships: 93 canonical PCM sounds).
///
/// On-disk layout: `RIFF` | `u32 LE` riff size | `WAVE`, then a sequence of
/// chunks (`fmt ` parsed for the PCM format fields; `data` located and copied;
/// unknown chunks skipped by their declared, bounds-checked length; odd chunk
/// sizes padded to even per the RIFF specification).
///
/// This is a container parser, not a general-purpose WAV library: a
/// compressed or non-PCM format tag is surfaced as data, not decoded.
#[derive(Debug, Clone)]
pub struct WavSound {
    format_tag: u16,
    channels: u16,
    sample_rate: u32,
    byte_rate: u32,
    block_align: u16,
    bits_per_sample: u16,
    data: Vec<u8>,
    warnings: Vec<AudioWarning>,
}

impl WavSound {
    /// The fixed `RIFF`/`WAVE` preamble size in bytes.
    const HEADER: usize = 12;
    /// The chunk-frame header size: a 4-byte id plus a `u32` length.
    const CHUNK_HEADER: usize = 8;
    /// The minimum `fmt ` chunk body size (canonical PCM `WAVEFORMAT`).
    const FMT_MIN: usize = 16;
    /// The `fmt ` chunk identifier.
    const FMT: [u8; 4] = *b"fmt ";
    /// The `data` chunk identifier.
    const DATA: [u8; 4] = *b"data";

    /// Parses (indexes) a RIFF/WAVE lump.
    ///
    /// # Errors
    ///
    /// - [`AudioError::TruncatedHeader`] when the lump is shorter than the
    ///   12-byte `RIFF`/`WAVE` preamble (both modes).
    /// - [`AudioError::BadMagic`] when bytes `0..4` are not `RIFF` or bytes
    ///   `8..12` are not `WAVE` (both modes).
    /// - [`AudioError::ChunkOverrun`] (strict only) when a declared chunk
    ///   length overruns the remaining lump; lenient records
    ///   [`AudioWarning::ChunkOverrun`] and stops the walk.
    /// - [`AudioError::FmtChunkTooSmall`] (strict only) when a `fmt ` chunk is
    ///   smaller than 16 bytes; lenient records
    ///   [`AudioWarning::FmtChunkTooSmall`] and skips it.
    /// - [`AudioError::MissingChunk`] (strict only) when the walk finds no
    ///   `fmt ` or no `data` chunk; lenient records
    ///   [`AudioWarning::MissingChunk`] and defaults the fields to `0` / empty
    ///   data.
    ///
    /// A riff-size field that disagrees with the lump length
    /// ([`AudioWarning::RiffSizeMismatch`]), a duplicate `fmt `/`data` chunk
    /// ([`AudioWarning::DuplicateChunk`]), and trailing bytes too short for a
    /// chunk header ([`AudioWarning::TrailingBytes`]) are warnings in **both**
    /// modes; the parse still succeeds.
    #[allow(clippy::too_many_lines)]
    pub fn parse(bytes: &[u8], options: &ParseOptions) -> Result<Self, AudioError> {
        let len = bytes.len();
        if len < Self::HEADER {
            return Err(AudioError::TruncatedHeader {
                len,
                needed: Self::HEADER,
            });
        }
        if bytes[..4] != *b"RIFF" {
            return Err(AudioError::BadMagic {
                expected: b"RIFF",
                found: bytes[..4].to_vec(),
            });
        }
        if bytes[8..12] != *b"WAVE" {
            return Err(AudioError::BadMagic {
                expected: b"WAVE",
                found: bytes[8..12].to_vec(),
            });
        }

        let mut warnings = Vec::new();

        let riff_size = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
        let declared_end = Self::CHUNK_HEADER.saturating_add(riff_size);
        if declared_end != len {
            warnings.push(AudioWarning::RiffSizeMismatch {
                declared_end,
                lump_len: len,
            });
        }

        let mut format_tag = 0u16;
        let mut channels = 0u16;
        let mut sample_rate = 0u32;
        let mut byte_rate = 0u32;
        let mut block_align = 0u16;
        let mut bits_per_sample = 0u16;
        let mut data = Vec::new();
        let mut fmt_parsed = false;
        let mut fmt_seen = false;
        let mut data_seen = false;

        let mut pos = Self::HEADER;
        let mut stopped = false;

        while pos + Self::CHUNK_HEADER <= len {
            let id = [bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]];
            let size = u32::from_le_bytes([
                bytes[pos + 4],
                bytes[pos + 5],
                bytes[pos + 6],
                bytes[pos + 7],
            ]) as usize;
            let available = len - (pos + Self::CHUNK_HEADER);
            if size > available {
                match options.strictness {
                    Strictness::Strict => {
                        return Err(AudioError::ChunkOverrun {
                            offset: pos,
                            declared: size,
                            available,
                        });
                    }
                    Strictness::Lenient => {
                        warnings.push(AudioWarning::ChunkOverrun {
                            offset: pos,
                            declared: size,
                            available,
                        });
                        stopped = true;
                        break;
                    }
                }
            }

            let payload = pos + Self::CHUNK_HEADER;
            if id == Self::FMT {
                fmt_seen = true;
                if fmt_parsed {
                    warnings.push(AudioWarning::DuplicateChunk { id });
                } else if size < Self::FMT_MIN {
                    match options.strictness {
                        Strictness::Strict => {
                            return Err(AudioError::FmtChunkTooSmall { size });
                        }
                        Strictness::Lenient => {
                            warnings.push(AudioWarning::FmtChunkTooSmall { size });
                        }
                    }
                } else {
                    format_tag = u16::from_le_bytes([bytes[payload], bytes[payload + 1]]);
                    channels = u16::from_le_bytes([bytes[payload + 2], bytes[payload + 3]]);
                    sample_rate = u32::from_le_bytes([
                        bytes[payload + 4],
                        bytes[payload + 5],
                        bytes[payload + 6],
                        bytes[payload + 7],
                    ]);
                    byte_rate = u32::from_le_bytes([
                        bytes[payload + 8],
                        bytes[payload + 9],
                        bytes[payload + 10],
                        bytes[payload + 11],
                    ]);
                    block_align = u16::from_le_bytes([bytes[payload + 12], bytes[payload + 13]]);
                    bits_per_sample =
                        u16::from_le_bytes([bytes[payload + 14], bytes[payload + 15]]);
                    fmt_parsed = true;
                }
            } else if id == Self::DATA {
                if data_seen {
                    warnings.push(AudioWarning::DuplicateChunk { id });
                } else {
                    data = bytes[payload..payload + size].to_vec();
                    data_seen = true;
                }
            }

            // Advance past the payload, honoring RIFF's even-alignment pad.
            // A missing pad byte at end-of-lump is tolerated deliberately:
            // real-world writers commonly omit the final pad, and the byte
            // carries no data. An omitted *interior* pad desynchronizes any
            // RIFF reader; the walk's bounds checks keep that failure mode
            // safe (it surfaces as overrun/duplicate/missing-chunk
            // diagnostics rather than an out-of-bounds read).
            pos = payload + size;
            if size % 2 == 1 && pos < len {
                pos += 1;
            }
        }

        if !stopped && pos < len {
            let remainder = len - pos;
            if remainder < Self::CHUNK_HEADER {
                warnings.push(AudioWarning::TrailingBytes { len: remainder });
            }
        }

        // `MissingChunk` means no `fmt ` chunk existed at all; a present-but-
        // malformed one is already surfaced as `FmtChunkTooSmall`, and doubling
        // it with a "missing" warning would misdescribe the lump.
        if !fmt_seen {
            match options.strictness {
                Strictness::Strict => return Err(AudioError::MissingChunk { id: Self::FMT }),
                Strictness::Lenient => warnings.push(AudioWarning::MissingChunk { id: Self::FMT }),
            }
        }
        if !data_seen {
            match options.strictness {
                Strictness::Strict => return Err(AudioError::MissingChunk { id: Self::DATA }),
                Strictness::Lenient => warnings.push(AudioWarning::MissingChunk { id: Self::DATA }),
            }
        }

        Ok(Self {
            format_tag,
            channels,
            sample_rate,
            byte_rate,
            block_align,
            bits_per_sample,
            data,
            warnings,
        })
    }

    /// The `fmt ` chunk's format tag (`1` = PCM; other values are compressed
    /// or non-PCM formats surfaced as data, not decoded). `0` when no `fmt `
    /// chunk was parsed.
    #[must_use]
    pub fn format_tag(&self) -> u16 {
        self.format_tag
    }

    /// The channel count from the `fmt ` chunk (`0` when unparsed).
    #[must_use]
    pub fn channels(&self) -> u16 {
        self.channels
    }

    /// The sample rate in Hz from the `fmt ` chunk (`0` when unparsed).
    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// The byte rate from the `fmt ` chunk (`0` when unparsed).
    #[must_use]
    pub fn byte_rate(&self) -> u32 {
        self.byte_rate
    }

    /// The block alignment from the `fmt ` chunk (`0` when unparsed).
    #[must_use]
    pub fn block_align(&self) -> u16 {
        self.block_align
    }

    /// The bits-per-sample from the `fmt ` chunk (`0` when unparsed).
    #[must_use]
    pub fn bits_per_sample(&self) -> u16 {
        self.bits_per_sample
    }

    /// The `data` chunk contents (an owned copy, `O(input)`), empty when no
    /// `data` chunk was located.
    #[must_use]
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Non-fatal issues recorded during parsing.
    #[must_use]
    pub fn warnings(&self) -> &[AudioWarning] {
        &self.warnings
    }
}
