//! Classic sound-effect lumps (ADR-0023 §2): the DMX digital-sound format
//! ([`DmxSound`]) and the PC-speaker format ([`PcSpeakerSound`]). Both are
//! bounded reads over `&[u8]` with lenient-mode warnings carried inside the
//! returned value, following the shipped gfx idiom.

use crate::{ParseOptions, Strictness};

use super::{AudioError, AudioWarning};

/// A DMX digital-sound lump (`DS*`; engine read: `src/i_sdlsound.c:737-772`).
///
/// On-disk layout: `u16 LE` format (`3`) | `u16 LE` sample rate in Hz |
/// `u32 LE` length (the byte count after the 8-byte header, **including**
/// both 16-byte pads) | 16-byte leading pad | unsigned 8-bit mono PCM
/// samples | 16-byte trailing pad.
///
/// [`samples`](DmxSound::samples) exposes the pad-stripped PCM view — lump
/// offset 24, count `length - 32` — exactly the span vanilla plays
/// (`data += 16; length -= 32;`, `src/i_sdlsound.c:764-772`);
/// [`payload`](DmxSound::payload) exposes the whole effective body including
/// the pads.
#[derive(Debug, Clone)]
pub struct DmxSound {
    sample_rate: u16,
    length: u32,
    payload: Vec<u8>,
    warnings: Vec<AudioWarning>,
}

impl DmxSound {
    /// The fixed header size in bytes.
    const HEADER: usize = 8;
    /// The combined size of the leading and trailing pads — the minimum a
    /// declared length must reach to carry any samples.
    const PAD_TOTAL: u32 = 32;

    /// Parses a DMX digital-sound lump.
    ///
    /// # Errors
    ///
    /// - [`AudioError::TruncatedHeader`] when the lump is shorter than the
    ///   8-byte header (both modes).
    /// - [`AudioError::UnexpectedFormat`] when the format field is not `3`
    ///   (both modes).
    /// - [`AudioError::LengthOutOfRange`] (strict only) when the declared
    ///   length violates `32 <= length <= lump_len - 8`. Lenient mode
    ///   recovers — an overrun clamps to the available bytes, a length below
    ///   32 yields empty samples — and records
    ///   [`AudioWarning::LengthOutOfRange`].
    ///
    /// Trailing slack, a declared length at or below the 48-byte playability
    /// floor, and a zero sample rate are warnings in **both** modes
    /// ([`AudioWarning::TrailingSlack`], [`AudioWarning::PlayabilityFloor`],
    /// [`AudioWarning::ZeroSampleRate`]); the parse still succeeds. The
    /// floor warning is only emitted when the length invariant holds — a
    /// length that is both out of range and below the floor is surfaced by
    /// [`AudioError::LengthOutOfRange`] /
    /// [`AudioWarning::LengthOutOfRange`] alone.
    pub fn parse(bytes: &[u8], options: &ParseOptions) -> Result<Self, AudioError> {
        let len = bytes.len();
        if len < Self::HEADER {
            return Err(AudioError::TruncatedHeader {
                len,
                needed: Self::HEADER,
            });
        }
        let format = u16::from_le_bytes([bytes[0], bytes[1]]);
        if format != 3 {
            return Err(AudioError::UnexpectedFormat {
                expected: 3,
                found: format,
            });
        }
        let sample_rate = u16::from_le_bytes([bytes[2], bytes[3]]);
        let length = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let available = len - Self::HEADER;

        let mut warnings = Vec::new();

        // Invariant: 32 <= length <= available. When it holds, the declared
        // length is authoritative; when it does not, strict fails and lenient
        // clamps to `min(length, available)` — an under-min length that fits
        // keeps its own value, and anything larger than the lump clamps to
        // what is present.
        let length_usize = length as usize;
        let invariant_holds = length >= Self::PAD_TOTAL && length_usize <= available;
        let effective = if invariant_holds {
            length_usize
        } else {
            match options.strictness {
                Strictness::Strict => {
                    return Err(AudioError::LengthOutOfRange {
                        length,
                        min: Self::PAD_TOTAL,
                        available,
                    });
                }
                Strictness::Lenient => {
                    warnings.push(AudioWarning::LengthOutOfRange {
                        length,
                        min: Self::PAD_TOTAL,
                        available,
                    });
                    length_usize.min(available)
                }
            }
        };

        if Self::HEADER + effective < len {
            warnings.push(AudioWarning::TrailingSlack {
                expected: Self::HEADER + effective,
                lump_len: len,
            });
        }
        // The playability floor is a property of a valid length; an
        // out-of-range length is already surfaced by `LengthOutOfRange`.
        if invariant_holds && length <= 48 {
            warnings.push(AudioWarning::PlayabilityFloor { length });
        }
        if sample_rate == 0 {
            warnings.push(AudioWarning::ZeroSampleRate);
        }

        let payload = bytes[Self::HEADER..Self::HEADER + effective].to_vec();
        Ok(Self {
            sample_rate,
            length,
            payload,
            warnings,
        })
    }

    /// The declared sample rate in Hz (11025 dominant; 22050/16000/44100
    /// observed). A value of `0` is surfaced as
    /// [`AudioWarning::ZeroSampleRate`].
    #[must_use]
    pub fn sample_rate(&self) -> u16 {
        self.sample_rate
    }

    /// The declared length field (bytes after the header, including both
    /// pads) as read from disk — not the recovered effective length.
    #[must_use]
    pub fn length(&self) -> u32 {
        self.length
    }

    /// The pad-stripped PCM view: the effective payload with its leading and
    /// trailing 16-byte pads removed — lump offset 24, count
    /// `effective_length - 32`, exactly the span vanilla plays. Empty when
    /// the effective length is degenerate (below the 32-byte pad minimum).
    #[must_use]
    pub fn samples(&self) -> &[u8] {
        if self.payload.len() >= Self::PAD_TOTAL as usize {
            &self.payload[16..self.payload.len() - 16]
        } else {
            &[]
        }
    }

    /// The effective payload: every byte after the header up to the effective
    /// length, pads included.
    #[must_use]
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    /// Non-fatal issues recorded during parsing.
    #[must_use]
    pub fn warnings(&self) -> &[AudioWarning] {
        &self.warnings
    }
}

/// A PC-speaker sound lump (`DP*`; engine read: `src/i_pcsound.c:126-141`).
///
/// On-disk layout: `u16 LE` format (`0`) | `u16 LE` tone count | one tone
/// byte per 1/140-second tick. Each tone byte indexes a 128-entry divisor
/// table; values `>= 128` render as silence in-engine (ADR-0023 §2).
#[derive(Debug, Clone)]
pub struct PcSpeakerSound {
    count: u16,
    tones: Vec<u8>,
    warnings: Vec<AudioWarning>,
}

impl PcSpeakerSound {
    /// The fixed header size in bytes.
    const HEADER: usize = 4;

    /// Parses a PC-speaker sound lump.
    ///
    /// # Errors
    ///
    /// - [`AudioError::TruncatedHeader`] when the lump is shorter than the
    ///   4-byte header (both modes). This is **deliberately stricter than
    ///   the engine**, which reads the header with no minimum-length check —
    ///   an out-of-bounds read on a sub-4-byte lump (`src/i_pcsound.c:126-131`).
    /// - [`AudioError::UnexpectedFormat`] when the format field is not `0`
    ///   (both modes).
    /// - [`AudioError::LengthOutOfRange`] (strict only) when the tone count
    ///   exceeds `lump_len - 4`. Lenient mode clamps the tones to the
    ///   available bytes and records [`AudioWarning::LengthOutOfRange`].
    ///
    /// Trailing slack and tone bytes `>= 128` are warnings in **both** modes
    /// ([`AudioWarning::TrailingSlack`], [`AudioWarning::OutOfRangeTones`]);
    /// a zero tone count is valid and warning-free.
    pub fn parse(bytes: &[u8], options: &ParseOptions) -> Result<Self, AudioError> {
        let len = bytes.len();
        if len < Self::HEADER {
            return Err(AudioError::TruncatedHeader {
                len,
                needed: Self::HEADER,
            });
        }
        let format = u16::from_le_bytes([bytes[0], bytes[1]]);
        if format != 0 {
            return Err(AudioError::UnexpectedFormat {
                expected: 0,
                found: format,
            });
        }
        let count = u16::from_le_bytes([bytes[2], bytes[3]]);
        let available = len - Self::HEADER;

        let mut warnings = Vec::new();

        let effective = if usize::from(count) <= available {
            usize::from(count)
        } else {
            match options.strictness {
                Strictness::Strict => {
                    return Err(AudioError::LengthOutOfRange {
                        length: u32::from(count),
                        min: 0,
                        available,
                    });
                }
                Strictness::Lenient => {
                    warnings.push(AudioWarning::LengthOutOfRange {
                        length: u32::from(count),
                        min: 0,
                        available,
                    });
                    available
                }
            }
        };

        if Self::HEADER + effective < len {
            warnings.push(AudioWarning::TrailingSlack {
                expected: Self::HEADER + effective,
                lump_len: len,
            });
        }

        let tones = bytes[Self::HEADER..Self::HEADER + effective].to_vec();
        let out_of_range = tones.iter().filter(|&&t| t >= 128).count();
        if out_of_range > 0 {
            warnings.push(AudioWarning::OutOfRangeTones {
                count: out_of_range,
            });
        }

        Ok(Self {
            count,
            tones,
            warnings,
        })
    }

    /// The declared tone count as read from disk — not the recovered
    /// effective count.
    #[must_use]
    pub fn declared_count(&self) -> u16 {
        self.count
    }

    /// The tone bytes (each a divisor-table index), clamped to the bytes
    /// actually present when the declared count overran.
    #[must_use]
    pub fn tones(&self) -> &[u8] {
        &self.tones
    }

    /// Non-fatal issues recorded during parsing.
    #[must_use]
    pub fn warnings(&self) -> &[AudioWarning] {
        &self.warnings
    }
}
