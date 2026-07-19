//! Instrument-bank audio lumps (ADR-0023 §2): [`Genmidi`], the fixed-layout
//! OPL instrument bank (`GENMIDI`), and [`Dmxgus`], the line-oriented GUS
//! patch-mapping lump (`DMXGUS`/`DMXGUSC`). Both are bounded reads over
//! `&[u8]`; neither recurses and neither allocates beyond `O(input)`.
//!
//! The engines these mirror perform little or no validation. Vanilla
//! pointer-casts the `GENMIDI` lump with **zero** checks — "DMX does not check
//! header" (`src/i_oplmusic.c:369`) — so a truncated bank is an out-of-bounds
//! read in C; crustywad supplies the extent and magic checks the engine omits
//! (ADR-0023 §5). The GUS parser (`src/gusconf.c:64-153`) reads with `atoi`,
//! which yields `0` on garbage and silently skips short lines; crustywad is
//! deliberately stricter (a malformed data line is a strict error) and stores
//! every well-formed entry — including the reserved-gap ids the engine's
//! range filter drops, which every retail carrier ships (ADR-0023 §2
//! amendment) and which are classified via [`DmxgusEntry::is_gm_mapped`]
//! rather than warned.

use crate::{ParseOptions, Strictness};

use super::{AudioError, AudioWarning};

/// A single OPL operator's six raw register bytes (engine reference:
/// `genmidi_op_t`, `src/i_oplmusic.c:40-49`). Each field is the raw OPL
/// register value as stored on disk; this parser reads and exposes them but
/// does not interpret or program the OPL chip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenmidiOp {
    /// Raw tremolo/vibrato/sustain/KSR/multiplier register byte.
    pub tremolo: u8,
    /// Raw attack/decay-rate register byte.
    pub attack: u8,
    /// Raw sustain/release-rate register byte.
    pub sustain: u8,
    /// Raw waveform-select register byte.
    pub waveform: u8,
    /// Raw key-scale-level register byte.
    pub scale: u8,
    /// Raw output-level register byte.
    pub level: u8,
}

/// One of an instrument's two OPL voices (engine reference: `genmidi_voice_t`,
/// `src/i_oplmusic.c:51-59`): a modulator operator, a feedback byte, a carrier
/// operator, an unused byte, and a signed base-note offset. 16 bytes on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenmidiVoice {
    /// The modulator operator's six register bytes.
    pub modulator: GenmidiOp,
    /// The raw feedback/connection register byte.
    pub feedback: u8,
    /// The carrier operator's six register bytes.
    pub carrier: GenmidiOp,
    /// A byte the format reserves and the engine ignores.
    pub unused: u8,
    /// The signed base-note offset (`i16 LE`) applied to notes played on this
    /// voice.
    pub base_note_offset: i16,
}

/// A single `GENMIDI` instrument record (engine reference:
/// `genmidi_instr_t`, `src/i_oplmusic.c:61-69`). 36 bytes on disk: a `u16 LE`
/// flags word, a fine-tuning byte, a fixed-note byte, and two
/// [`GenmidiVoice`]s.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenmidiInstrument {
    /// The raw flags word. Bit 0 marks a fixed-pitch instrument and bit 2 a
    /// double-voice instrument; see [`is_fixed_pitch`](GenmidiInstrument::is_fixed_pitch)
    /// and [`is_double_voice`](GenmidiInstrument::is_double_voice).
    pub flags: u16,
    /// The raw on-disk fine-tuning byte, stored unconverted. The engine
    /// applies it to the second voice as `(value / 2) - 64` semitone-table
    /// steps (`i_oplmusic.c:844`), making `128` the no-adjustment center —
    /// interpretation is the caller's.
    pub fine_tuning: u8,
    /// The MIDI note number a fixed-pitch instrument always plays.
    pub fixed_note: u8,
    /// The instrument's two OPL voices; the second is only used when
    /// [`is_double_voice`](GenmidiInstrument::is_double_voice) holds.
    pub voices: [GenmidiVoice; 2],
}

impl GenmidiInstrument {
    /// The flags bit (bit 0) marking a fixed-pitch instrument.
    const FLAG_FIXED_PITCH: u16 = 0x0001;
    /// The flags bit (bit 2) marking a double-voice instrument.
    const FLAG_DOUBLE_VOICE: u16 = 0x0004;

    /// Whether this is a fixed-pitch instrument (flags bit 0): every note plays
    /// at [`fixed_note`](GenmidiInstrument::fixed_note).
    #[must_use]
    pub fn is_fixed_pitch(&self) -> bool {
        self.flags & Self::FLAG_FIXED_PITCH != 0
    }

    /// Whether this is a double-voice instrument (flags bit 2): both
    /// [`voices`](GenmidiInstrument::voices) are sounded rather than only the
    /// first.
    #[must_use]
    pub fn is_double_voice(&self) -> bool {
        self.flags & Self::FLAG_DOUBLE_VOICE != 0
    }
}

/// The DMX OPL instrument bank (`GENMIDI`), a fixed 11908-byte layout (engine
/// reference: `src/i_oplmusic.c:40-75`, `:367-375`).
///
/// On-disk layout: an 8-byte magic `#OPL_II#`, then 128 melodic and 47
/// percussion [`GenmidiInstrument`] records (36 bytes each), then 128 melodic
/// and 47 percussion name fields (`[u8; 32]` each, NUL-padded). The engine
/// performs **no** validation — it pointer-casts the lump — so this parser
/// supplies the checks the engine omits (ADR-0023 §5).
///
/// Strict mode requires the **full** 11908-byte extent and the magic: a
/// short lump or a wrong magic is a strict error, recovered leniently with a
/// warning. A **longer** lump parses its first 11908 bytes in **both** modes
/// with a [`AudioWarning::TrailingSlack`] — the module-wide slack policy
/// (`DmxSound`/`PcSpeakerSound` behave the same; ADR-0023 §2 amendment).
/// Lenient recovery never reads past the bytes present: it decodes every
/// complete record, then every complete name field, that fits.
///
/// Names are stored as owned [`String`]s, trimmed at the first NUL and
/// converted with [`String::from_utf8_lossy`] (a non-UTF-8 name field yields
/// the lossy replacement rather than an error).
#[derive(Debug, Clone)]
pub struct Genmidi {
    instruments: Vec<GenmidiInstrument>,
    percussion: Vec<GenmidiInstrument>,
    instrument_names: Vec<String>,
    percussion_names: Vec<String>,
    warnings: Vec<AudioWarning>,
}

impl Genmidi {
    /// The magic bytes at the start of a `GENMIDI` lump.
    const MAGIC: [u8; 8] = *b"#OPL_II#";
    /// The number of melodic instruments (and melodic names).
    const MELODIC: usize = 128;
    /// The number of percussion instruments (and percussion names).
    const PERCUSSION: usize = 47;
    /// The on-disk size of one [`GenmidiInstrument`] record.
    const INSTRUMENT: usize = 36;
    /// The on-disk size of one name field.
    const NAME: usize = 32;
    /// The byte offset at which the name fields begin (past magic + all
    /// records).
    const NAMES_START: usize = 8 + (Self::MELODIC + Self::PERCUSSION) * Self::INSTRUMENT;
    /// The exact on-disk size of a well-formed `GENMIDI` lump.
    const TOTAL: usize = Self::NAMES_START + (Self::MELODIC + Self::PERCUSSION) * Self::NAME;

    /// Parses a `GENMIDI` OPL instrument bank.
    ///
    /// # Errors
    ///
    /// - [`AudioError::TruncatedHeader`] when the lump is shorter than the
    ///   8-byte magic (both modes), or — in strict mode only — shorter than the
    ///   fixed 11908-byte extent (`needed` is `11908`); lenient mode decodes
    ///   every complete record and name that fits and records
    ///   [`AudioWarning::TruncatedBank`].
    /// - [`AudioError::BadMagic`] (strict only) when the leading bytes are not
    ///   `#OPL_II#`; lenient mode records [`AudioWarning::BadMagic`] and
    ///   proceeds — the engine never checks the magic (`i_oplmusic.c:369`).
    ///
    /// A lump longer than 11908 bytes is trailing slack
    /// ([`AudioWarning::TrailingSlack`]) in **both** modes; the first 11908
    /// bytes are parsed and the parse succeeds.
    pub fn parse(bytes: &[u8], options: &ParseOptions) -> Result<Self, AudioError> {
        let len = bytes.len();
        if len < Self::MAGIC.len() {
            return Err(AudioError::TruncatedHeader {
                len,
                needed: Self::MAGIC.len(),
            });
        }

        let mut warnings = Vec::new();

        if bytes[..Self::MAGIC.len()] != Self::MAGIC {
            let found = bytes[..Self::MAGIC.len()].to_vec();
            match options.strictness {
                Strictness::Strict => {
                    return Err(AudioError::BadMagic {
                        expected: &Self::MAGIC,
                        found,
                    });
                }
                Strictness::Lenient => warnings.push(AudioWarning::BadMagic {
                    expected: &Self::MAGIC,
                    found: found.clone(),
                }),
            }
        }

        if len < Self::TOTAL {
            match options.strictness {
                Strictness::Strict => {
                    return Err(AudioError::TruncatedHeader {
                        len,
                        needed: Self::TOTAL,
                    });
                }
                Strictness::Lenient => warnings.push(AudioWarning::TruncatedBank {
                    len,
                    needed: Self::TOTAL,
                }),
            }
        } else if len > Self::TOTAL {
            warnings.push(AudioWarning::TrailingSlack {
                expected: Self::TOTAL,
                lump_len: len,
            });
        }

        // Reads are capped at the fixed extent and floored to complete records
        // and names, so every offset below is in bounds.
        let effective = len.min(Self::TOTAL);
        let record_count =
            (Self::MELODIC + Self::PERCUSSION).min(effective.saturating_sub(8) / Self::INSTRUMENT);
        let name_count = (Self::MELODIC + Self::PERCUSSION)
            .min(effective.saturating_sub(Self::NAMES_START) / Self::NAME);

        let melodic_records = record_count.min(Self::MELODIC);
        let percussion_records = record_count - melodic_records;
        let melodic_names = name_count.min(Self::MELODIC);
        let percussion_names = name_count - melodic_names;

        let mut instruments = Vec::with_capacity(melodic_records);
        for i in 0..melodic_records {
            instruments.push(Self::read_instrument(bytes, 8 + i * Self::INSTRUMENT));
        }
        let mut percussion = Vec::with_capacity(percussion_records);
        for i in 0..percussion_records {
            let off = 8 + (Self::MELODIC + i) * Self::INSTRUMENT;
            percussion.push(Self::read_instrument(bytes, off));
        }

        let mut instrument_names = Vec::with_capacity(melodic_names);
        for i in 0..melodic_names {
            instrument_names.push(Self::read_name(bytes, Self::NAMES_START + i * Self::NAME));
        }
        let mut percussion_names_vec = Vec::with_capacity(percussion_names);
        for i in 0..percussion_names {
            let off = Self::NAMES_START + (Self::MELODIC + i) * Self::NAME;
            percussion_names_vec.push(Self::read_name(bytes, off));
        }

        Ok(Self {
            instruments,
            percussion,
            instrument_names,
            percussion_names: percussion_names_vec,
            warnings,
        })
    }

    /// Reads a 6-byte [`GenmidiOp`] at `off` (caller guarantees `off + 6 <=
    /// len`).
    fn read_op(bytes: &[u8], off: usize) -> GenmidiOp {
        GenmidiOp {
            tremolo: bytes[off],
            attack: bytes[off + 1],
            sustain: bytes[off + 2],
            waveform: bytes[off + 3],
            scale: bytes[off + 4],
            level: bytes[off + 5],
        }
    }

    /// Reads a 16-byte [`GenmidiVoice`] at `off` (caller guarantees `off + 16
    /// <= len`).
    fn read_voice(bytes: &[u8], off: usize) -> GenmidiVoice {
        GenmidiVoice {
            modulator: Self::read_op(bytes, off),
            feedback: bytes[off + 6],
            carrier: Self::read_op(bytes, off + 7),
            unused: bytes[off + 13],
            base_note_offset: i16::from_le_bytes([bytes[off + 14], bytes[off + 15]]),
        }
    }

    /// Reads a 36-byte [`GenmidiInstrument`] at `off` (caller guarantees `off +
    /// 36 <= len`).
    fn read_instrument(bytes: &[u8], off: usize) -> GenmidiInstrument {
        GenmidiInstrument {
            flags: u16::from_le_bytes([bytes[off], bytes[off + 1]]),
            fine_tuning: bytes[off + 2],
            fixed_note: bytes[off + 3],
            voices: [
                Self::read_voice(bytes, off + 4),
                Self::read_voice(bytes, off + 20),
            ],
        }
    }

    /// Reads a 32-byte name field at `off`, trimming at the first NUL and
    /// converting lossily (caller guarantees `off + 32 <= len`).
    fn read_name(bytes: &[u8], off: usize) -> String {
        let field = &bytes[off..off + Self::NAME];
        let end = field.iter().position(|&b| b == 0).unwrap_or(Self::NAME);
        String::from_utf8_lossy(&field[..end]).into_owned()
    }

    /// The melodic instrument records (up to 128; fewer only for a leniently
    /// recovered short lump).
    #[must_use]
    pub fn instruments(&self) -> &[GenmidiInstrument] {
        &self.instruments
    }

    /// The percussion instrument records (up to 47; fewer only for a leniently
    /// recovered short lump).
    #[must_use]
    pub fn percussion(&self) -> &[GenmidiInstrument] {
        &self.percussion
    }

    /// The melodic instrument names, aligned with
    /// [`instruments`](Genmidi::instruments) in a well-formed lump.
    #[must_use]
    pub fn instrument_names(&self) -> &[String] {
        &self.instrument_names
    }

    /// The percussion instrument names, aligned with
    /// [`percussion`](Genmidi::percussion) in a well-formed lump.
    #[must_use]
    pub fn percussion_names(&self) -> &[String] {
        &self.percussion_names
    }

    /// Non-fatal issues recorded during parsing.
    #[must_use]
    pub fn warnings(&self) -> &[AudioWarning] {
        &self.warnings
    }
}

/// A single well-formed `DMXGUS` data line: the melodic/percussion instrument
/// id, the four mapped patch ids (one per GUS RAM tier — 256/512/768/1024 KB),
/// and the patch file name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DmxgusEntry {
    /// The instrument id. The engine maps melodic `0..=127` and percussion
    /// `163..=209` and skips everything else (`gusconf.c:127-131`) — but the
    /// reserved-gap ids (`128`, `155..=162`, `210..=215`) appear in **every**
    /// retail `DMXGUS`/`DMXGUSC` lump (ADR-0023 §2 amendment), so they are
    /// ordinary data, not an anomaly. Classify with
    /// [`is_gm_mapped`](DmxgusEntry::is_gm_mapped).
    pub instrument: u32,
    /// The mapped patch ids at the four GUS RAM tiers (256, 512, 768, 1024 KB).
    pub mappings: [u32; 4],
    /// The patch file name (the sixth field, verbatim after trimming).
    pub patch: String,
}

impl DmxgusEntry {
    /// Whether the engine's id-range filter maps this entry: melodic
    /// `0..=127` or percussion `163..=209` (`gusconf.c:127-131`). Entries in
    /// the reserved gaps (`128`, `155..=162`, `210..=215`) return `false` —
    /// the engine skips them, but every retail lump ships them (ADR-0023 §2
    /// amendment), so they parse as data rather than warning.
    #[must_use]
    pub fn is_gm_mapped(&self) -> bool {
        (0..=127).contains(&self.instrument) || (163..=209).contains(&self.instrument)
    }
}

/// The GUS patch-mapping lump (`DMXGUS`/`DMXGUSC`), a line-oriented text format
/// (engine reference: `src/gusconf.c:64-153`).
///
/// Each line is truncated at the first `#` (comment); a line that is empty or
/// all whitespace after that is skipped silently. A data line splits on `,`
/// into at least six whitespace-trimmed fields: an instrument id, four mapped
/// patch ids, and a patch file name. Fields beyond the sixth are ignored with
/// a warning.
///
/// # Divergence from the engine
///
/// The engine parses ids with `atoi`, which yields `0` on non-numeric input and
/// silently skips a short line. This parser is **deliberately stricter**: an id
/// field that is not a full decimal `u32`, or a data line with fewer than six
/// fields, is a strict [`AudioError::MalformedGusLine`] (lenient skips it with
/// the mirroring warning). The engine's id-range filter (melodic `0..=127`,
/// percussion `163..=209`) is load-bearing for its own array bounds; this
/// parser stores every well-formed entry regardless. Reserved-gap ids are
/// **not** warned — every retail carrier ships them (ADR-0023 §2 amendment) —
/// and are classified via [`DmxgusEntry::is_gm_mapped`] instead.
#[derive(Debug, Clone)]
pub struct Dmxgus {
    entries: Vec<DmxgusEntry>,
    warnings: Vec<AudioWarning>,
}

impl Dmxgus {
    /// The minimum number of comma-separated fields a data line requires.
    const MIN_FIELDS: usize = 6;

    /// Parses a `DMXGUS`/`DMXGUSC` patch-mapping lump.
    ///
    /// The bytes are interpreted as text via [`String::from_utf8_lossy`] and
    /// split on `\n` (a trailing `\r` per line is stripped, tolerating CRLF).
    ///
    /// # Errors
    ///
    /// - [`AudioError::MalformedGusLine`] (strict only) when a data line has
    ///   fewer than six fields or an id field is not a decimal `u32`; lenient
    ///   mode skips the line and records [`AudioWarning::MalformedGusLine`].
    ///
    /// Fields beyond the sixth ([`AudioWarning::ExtraGusFields`]) are a
    /// warning in **both** modes; the parse still succeeds. Reserved-gap
    /// instrument ids are ordinary data — see
    /// [`DmxgusEntry::is_gm_mapped`].
    pub fn parse(bytes: &[u8], options: &ParseOptions) -> Result<Self, AudioError> {
        let text = String::from_utf8_lossy(bytes);
        let mut entries = Vec::new();
        let mut warnings = Vec::new();

        for (idx, raw_line) in text.split('\n').enumerate() {
            let line_number = idx + 1;
            let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);

            // Everything from the first '#' is a comment.
            let content = match line.find('#') {
                Some(i) => &line[..i],
                None => line,
            };
            if content.trim().is_empty() {
                continue;
            }

            let fields: Vec<&str> = content.split(',').map(str::trim).collect();
            if fields.len() < Self::MIN_FIELDS {
                match options.strictness {
                    Strictness::Strict => {
                        return Err(AudioError::MalformedGusLine { line: line_number });
                    }
                    Strictness::Lenient => {
                        warnings.push(AudioWarning::MalformedGusLine { line: line_number });
                        continue;
                    }
                }
            }

            // The first five fields (instrument id + four RAM-tier mappings)
            // must be decimal u32s; field 5 is the patch-name string.
            let mut ids = [0u32; 5];
            let mut malformed = false;
            for (slot, field) in ids.iter_mut().zip(fields.iter()) {
                let Ok(v) = field.parse::<u32>() else {
                    malformed = true;
                    break;
                };
                *slot = v;
            }
            if malformed {
                match options.strictness {
                    Strictness::Strict => {
                        return Err(AudioError::MalformedGusLine { line: line_number });
                    }
                    Strictness::Lenient => {
                        warnings.push(AudioWarning::MalformedGusLine { line: line_number });
                        continue;
                    }
                }
            }

            if fields.len() > Self::MIN_FIELDS {
                warnings.push(AudioWarning::ExtraGusFields {
                    line: line_number,
                    extra: fields.len() - Self::MIN_FIELDS,
                });
            }

            entries.push(DmxgusEntry {
                instrument: ids[0],
                mappings: [ids[1], ids[2], ids[3], ids[4]],
                patch: fields[5].to_owned(),
            });
        }

        Ok(Self { entries, warnings })
    }

    /// The well-formed patch-mapping entries, in file order.
    #[must_use]
    pub fn entries(&self) -> &[DmxgusEntry] {
        &self.entries
    }

    /// Non-fatal issues recorded during parsing.
    #[must_use]
    pub fn warnings(&self) -> &[AudioWarning] {
        &self.warnings
    }
}
