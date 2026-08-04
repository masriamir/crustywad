//! Strife flag-bit constants for interpreting raw map-record values
//! (ADR-0028 §4), plus normalized dialogue-lump parsing (ADR-0028 §5).
//!
//! Strife's binary map records are byte-identical to Doom's; only the
//! *meaning* of flag bits, specials, and thing types differs. These constants
//! name the Strife-specific and Strife-divergent bits so a consumer holding a
//! map with [`Map::game()`][crate::map::Map::game] `== Some(WadGame::Strife)`
//! can read its raw `flags` fields correctly. The `Map` graph itself does not
//! decode game semantics (ADR-0014 non-goal; typed views are tracked
//! separately).
//!
//! The headline divergence: **thing-flag bit 3 (`0x0008`) is AMBUSH in Doom
//! but STAND in Strife**, with AMBUSH relocated to bit 5 (`0x0020`).
//! Doom-common bits — the skill gates in bits 0-2 (`0x0001`-`0x0004`) and
//! bit 4 (`0x0010`, not-in-single-player, which Strife merely names
//! `MTF_NOTSINGLE`) — keep their Doom meanings and are deliberately not
//! duplicated here.
//!
//! All values verified against Chocolate Strife
//! (`chocolate-doom` @ `353cf5001dfd`, `src/strife/doomdef.h:78-97` and
//! `src/strife/doomdata.h:134-144`); per-constant caveats quote the engine's
//! own uncertainty comments where they exist.

use binrw::BinRead;

use crate::{ParseOptions, Strictness};

/// Thing flag bit 3 (`0x0008`): the thing is a *standing* NPC (`MTF_STAND`).
///
/// Divergent: in Doom this bit is `MTF_AMBUSH` (deaf); Strife moves AMBUSH to
/// [`MTF_AMBUSH`] (bit 5, `0x0020`).
pub const MTF_STAND: u16 = 8;

/// Thing flag bit 5 (`0x0020`): deaf monster / does not react to sound
/// (`MTF_AMBUSH`).
///
/// Strife relocates Doom's bit-3 (`0x0008`) AMBUSH here.
pub const MTF_AMBUSH: u16 = 32;

/// Thing flag bit 6 (`0x0040`): the thing is friendly to players
/// (`MTF_FRIEND`).
pub const MTF_FRIEND: u16 = 64;

/// Thing flag bit 7 (`0x0080`): unidentified (`MTF_UNKNOWN1`) — the engine source
/// itself says "TODO - identify".
pub const MTF_UNKNOWN1: u16 = 128;

/// Thing flag bit 8 (`0x0100`): the thing is translucent (`MTF_TRANSLUCENT`, mapped to
/// `MF_SHADOW` at spawn). The engine source is unsure of the exact degree
/// ("STRIFE-TODO: But how much?").
pub const MTF_TRANSLUCENT: u16 = 256;

/// Thing flag bit 9 (`0x0200`): alternate translucency (`MTF_MVIS`, mapped to
/// `MF_MVIS`). The engine source's own comment: "thing is more - or less? -
/// translucent - STRIFE-TODO". Unused by retail map data.
pub const MTF_MVIS: u16 = 512;

/// Thing flag bit 10 (`0x0400`): unidentified (`MTF_UNKNOWN2`) — engine source "TODO -
/// identify"; unused by retail map data.
pub const MTF_UNKNOWN2: u16 = 1024;

/// Linedef flag bit 9 (`0x0200`): jump-over railing (`ML_JUMPOVER`). The behavior name
/// is the engine's own guess ("jump over rails?").
pub const ML_JUMPOVER: u16 = 512;

/// Linedef flag bit 10 (`0x0400`): blocks flying things (`ML_BLOCKFLOATERS`).
pub const ML_BLOCKFLOATERS: u16 = 1024;

/// Linedef flag bit 11 (`0x0800`): translucency variant 1 (`ML_TRANSPARENT1`); the
/// engine source is unsure of the percentage ("25% or 75% transcluency?"
/// \[sic\]).
pub const ML_TRANSPARENT1: u16 = 2048;

/// Linedef flag bit 12 (`0x1000`): translucency variant 2 (`ML_TRANSPARENT2`); same
/// engine-side uncertainty as [`ML_TRANSPARENT1`].
pub const ML_TRANSPARENT2: u16 = 4096;

/// Retail dialogue record size in bytes (0x5EC; Chocolate Strife
/// `ORIG_MAPDIALOG_SIZE`, `src/strife/p_dialog.c:48` @ 353cf50).
pub const RETAIL_DIALOG_RECORD_SIZE: usize = 1516;
/// Demo/teaser dialogue record size in bytes (0x5D0; Strife: Veteran Edition
/// `DEMO_MAPDIALOG_SIZE`, `src/strife/p_dialog.c:56` @ ac2381d).
pub const DEMO_DIALOG_RECORD_SIZE: usize = 1488;

/// Which on-disk dialogue layout a lump carried (ADR-0028 §5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogueFormat {
    /// The 1516-byte retail layout.
    Retail,
    /// The 1488-byte demo/teaser layout (selected only when the lump length
    /// is not a retail multiple; Strife: Veteran Edition's heuristic).
    Demo,
}

/// One choice a dialogue offers the player (identical in both layouts).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogueChoice {
    /// Item given when the choice succeeds.
    pub give_item: i32,
    /// Items required for success.
    pub need_items: [i32; 3],
    /// Amounts of each required item.
    pub need_amounts: [i32; 3],
    /// Choice caption (fixed NUL-padded bytes, not guaranteed terminated).
    pub text: [u8; 32],
    /// Message shown on success (fixed NUL-padded bytes).
    pub text_ok: [u8; 80],
    /// Next dialogue to jump to.
    pub next: i32,
    /// Mission-objective number; nonzero references a `LOGnn` text lump.
    pub objective: i32,
    /// Message shown on failure (fixed NUL-padded bytes).
    pub text_no: [u8; 80],
}

/// One normalized dialogue record — both on-disk layouts decode into this
/// (the crate's extended-nodes pattern: private dialect readers, one public
/// representation).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogueRecord {
    /// Script ID of the speaking thing type.
    pub speaker_id: i32,
    /// Item dropped when the speaker is killed.
    pub drop_item: i32,
    /// Items required to see this dialogue; `None` in the demo layout (the
    /// engine zero-fills these — `[0, 0, 0]` — when loading demo lumps).
    pub check_items: Option<[i32; 3]>,
    /// Conversation to jump to; `None` in the demo layout (engine fills 0).
    pub jump_to_conversation: Option<i32>,
    /// Speaker name (fixed NUL-padded bytes, not guaranteed terminated).
    pub name: [u8; 16],
    /// Voice lump name. Retail: raw bytes. Demo: the engine's `VOCnn`
    /// reconstruction from the stored number — all-NUL when the number is
    /// zero or negative, truncated to 7 characters like `M_snprintf`.
    pub voice: [u8; 8],
    /// Backdrop patch lump name; `None` in the demo layout (engine fills NULs).
    pub backpic: Option<[u8; 8]>,
    /// Main message text (fixed NUL-padded bytes).
    pub text: [u8; 320],
    /// The five choices (unused slots are all-zero records on disk).
    pub choices: [DialogueChoice; 5],
}

/// Errors from [`parse_dialogue`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DialogueError {
    /// Strict mode: the lump length is an exact multiple of neither record
    /// size.
    #[error(
        "dialogue lump length {len} matches neither the 1516-byte retail nor the 1488-byte demo record"
    )]
    LengthMatchesNoFormat {
        /// The offending lump length.
        len: usize,
    },
    /// Record decoding failed. Defensively unreachable: every field is a
    /// fixed-size integer or byte array read from a length-validated buffer,
    /// so `binrw` cannot hit end-of-input — kept so the [`parse_records`]
    /// seam stays honest rather than panicking on a broken invariant.
    ///
    /// [`parse_records`]: crate::map::parse_records
    #[error("dialogue record decoding failed: {0}")]
    Records(#[from] crate::map::MapParseError),
}

/// Non-fatal issues from lenient [`parse_dialogue`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum DialogueWarning {
    /// The lump length was not an exact record multiple; lenient parsing kept
    /// the whole records and ignored the rest (the engine floor-divides the
    /// same way).
    #[error(
        "dialogue lump has {trailing} trailing byte(s) after {kept} whole {format:?} record(s); ignored during lenient parsing"
    )]
    TrailingBytes {
        /// Whole records kept.
        kept: usize,
        /// Trailing bytes ignored.
        trailing: usize,
        /// The layout the kept records were decoded as.
        format: DialogueFormat,
    },
}

/// On-disk demo/teaser dialogue record (1488 bytes; strife-ve
/// `P_ParseDemoDialogLump` @ ac2381d — `voice_number` sits before `name`,
/// and `check_items`/`jump_to_conversation`/`backpic` do not exist on disk).
#[derive(Debug, Clone, Copy, BinRead)]
#[br(little)]
struct RawDemoDialogue {
    speaker_id: i32,
    drop_item: i32,
    voice_number: i32,
    name: [u8; 16],
    text: [u8; 320],
    choices: [RawChoice; 5],
}

/// On-disk retail dialogue record (1516 bytes). Private: decodes into
/// [`DialogueRecord`].
#[derive(Debug, Clone, Copy, BinRead)]
#[br(little)]
struct RawRetailDialogue {
    speaker_id: i32,
    drop_item: i32,
    check_items: [i32; 3],
    jump_to_conversation: i32,
    name: [u8; 16],
    voice: [u8; 8],
    backpic: [u8; 8],
    text: [u8; 320],
    choices: [RawChoice; 5],
}

/// On-disk choice record (228 bytes, shared by both layouts).
#[derive(Debug, Clone, Copy, BinRead)]
#[br(little)]
struct RawChoice {
    give_item: i32,
    need_items: [i32; 3],
    need_amounts: [i32; 3],
    text: [u8; 32],
    text_ok: [u8; 80],
    next: i32,
    objective: i32,
    text_no: [u8; 80],
}

fn normalize_choice(raw: RawChoice) -> DialogueChoice {
    DialogueChoice {
        give_item: raw.give_item,
        need_items: raw.need_items,
        need_amounts: raw.need_amounts,
        text: raw.text,
        text_ok: raw.text_ok,
        next: raw.next,
        objective: raw.objective,
        text_no: raw.text_no,
    }
}

fn normalize_retail(raw: &RawRetailDialogue) -> DialogueRecord {
    DialogueRecord {
        speaker_id: raw.speaker_id,
        drop_item: raw.drop_item,
        check_items: Some(raw.check_items),
        jump_to_conversation: Some(raw.jump_to_conversation),
        name: raw.name,
        voice: raw.voice,
        backpic: Some(raw.backpic),
        text: raw.text,
        choices: raw.choices.map(normalize_choice),
    }
}

/// The engine's demo voice reconstruction: `M_snprintf(voice, 8, "VOC%d", n)`
/// for positive `n` (truncating to 7 characters + NUL), all-NUL otherwise.
fn demo_voice(number: i32) -> [u8; 8] {
    let mut voice = [0_u8; 8];
    if number > 0 {
        let s = format!("VOC{number}");
        let take = s.len().min(7);
        voice[..take].copy_from_slice(&s.as_bytes()[..take]);
    }
    voice
}

fn normalize_demo(raw: &RawDemoDialogue) -> DialogueRecord {
    DialogueRecord {
        speaker_id: raw.speaker_id,
        drop_item: raw.drop_item,
        check_items: None,
        jump_to_conversation: None,
        name: raw.name,
        voice: demo_voice(raw.voice_number),
        backpic: None,
        text: raw.text,
        choices: raw.choices.map(normalize_choice),
    }
}

/// Parses a Strife dialogue lump (`SCRIPT00`–`SCRIPT99`) into normalized
/// records (ADR-0028 §5).
///
/// The layout is selected by the lump length, mirroring Strife: Veteran
/// Edition's `P_getDialogFormat`: an exact multiple of 1516 bytes is the
/// retail layout (this includes the empty lump — zero records), else an
/// exact multiple of 1488 bytes is the demo layout. In strict mode any other
/// length is [`DialogueError::LengthMatchesNoFormat`]; in lenient mode the
/// longest whole-record retail prefix is kept (possibly zero records) and a
/// [`DialogueWarning::TrailingBytes`] records what was ignored.
///
/// # Errors
///
/// Strict mode: [`DialogueError::LengthMatchesNoFormat`] when the length
/// divides neither record size. Both modes: the defensively-unreachable
/// [`DialogueError::Records`] (see its documentation).
pub fn parse_dialogue(
    bytes: &[u8],
    options: &ParseOptions,
) -> Result<(Vec<DialogueRecord>, DialogueFormat, Vec<DialogueWarning>), DialogueError> {
    let len = bytes.len();
    let mut warnings = Vec::new();
    let (kept, format) = if len.is_multiple_of(RETAIL_DIALOG_RECORD_SIZE) {
        (bytes, DialogueFormat::Retail)
    } else if len.is_multiple_of(DEMO_DIALOG_RECORD_SIZE) {
        (bytes, DialogueFormat::Demo)
    } else if options.strictness == Strictness::Lenient {
        let kept_len = len / RETAIL_DIALOG_RECORD_SIZE * RETAIL_DIALOG_RECORD_SIZE;
        warnings.push(DialogueWarning::TrailingBytes {
            kept: kept_len / RETAIL_DIALOG_RECORD_SIZE,
            trailing: len - kept_len,
            format: DialogueFormat::Retail,
        });
        (&bytes[..kept_len], DialogueFormat::Retail)
    } else {
        return Err(DialogueError::LengthMatchesNoFormat { len });
    };
    let records = match format {
        DialogueFormat::Retail => crate::map::parse_records::<RawRetailDialogue>(kept)?
            .iter()
            .map(normalize_retail)
            .collect(),
        DialogueFormat::Demo => crate::map::parse_records::<RawDemoDialogue>(kept)?
            .iter()
            .map(normalize_demo)
            .collect(),
    };
    Ok((records, format, warnings))
}

/// The dialogue lump name for a map number: the engine looks up
/// `"script%02d"` of the current map (`SCRIPT07` for MAP07), with `SCRIPT00`
/// (map number 0) doubling as the global fallback lump. Returns [`None`] for
/// map numbers above 99 — three digits cannot fit the 8-byte lump name field.
#[must_use]
pub fn script_lump_name(map_number: u8) -> Option<String> {
    (map_number <= 99).then(|| format!("SCRIPT{map_number:02}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_lump_names_cover_the_two_digit_range() {
        assert_eq!(script_lump_name(0).as_deref(), Some("SCRIPT00"));
        assert_eq!(script_lump_name(7).as_deref(), Some("SCRIPT07"));
        assert_eq!(script_lump_name(99).as_deref(), Some("SCRIPT99"));
        assert_eq!(script_lump_name(100), None);
        assert_eq!(script_lump_name(255), None);
    }

    #[test]
    fn bit_values_match_chocolate_strife() {
        // src/strife/doomdef.h:78-97 @ 353cf50
        assert_eq!(
            [
                MTF_STAND,
                MTF_AMBUSH,
                MTF_FRIEND,
                MTF_UNKNOWN1,
                MTF_TRANSLUCENT,
                MTF_MVIS,
                MTF_UNKNOWN2
            ],
            [8, 32, 64, 128, 256, 512, 1024]
        );
        // src/strife/doomdata.h:134-144 @ 353cf50
        assert_eq!(
            [
                ML_JUMPOVER,
                ML_BLOCKFLOATERS,
                ML_TRANSPARENT1,
                ML_TRANSPARENT2
            ],
            [512, 1024, 2048, 4096]
        );
    }

    #[test]
    fn strife_bits_are_disjoint_where_they_must_be() {
        // The divergent thing bits never collide with each other.
        let bits = [
            MTF_STAND,
            MTF_AMBUSH,
            MTF_FRIEND,
            MTF_UNKNOWN1,
            MTF_TRANSLUCENT,
            MTF_MVIS,
            MTF_UNKNOWN2,
        ];
        let or = bits.iter().fold(0_u16, |acc, b| acc | b);
        assert_eq!(u32::from(or).count_ones() as usize, bits.len());
    }
}
