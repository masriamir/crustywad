//! Strife flag-bit constants for interpreting raw map-record values
//! (ADR-0028 §4).
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
/// [sic]).
pub const ML_TRANSPARENT1: u16 = 2048;

/// Linedef flag bit 12 (`0x1000`): translucency variant 2 (`ML_TRANSPARENT2`); same
/// engine-side uncertainty as [`ML_TRANSPARENT1`].
pub const ML_TRANSPARENT2: u16 = 4096;

#[cfg(test)]
mod tests {
    use super::*;

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
