//! MUS -> MIDI conversion (issue #304), a faithful port of Chocolate Doom's
//! `mus2mid.c` (Ben Ryves, 2006) driven by the library's typed [`MusScore`]
//! event stream rather than a re-read of the raw lump.
//!
//! The reference converter is at
//! `/chocolate-doom/src/mus2mid.c`; every constant, event
//! code, and byte layout below carries a `mus2mid.c:LINE` citation to the
//! revision this port was verified against. Because the input is the already
//! bounds-checked, typed [`MusScore`] (its `SystemEvent`/`ChangeController`
//! controller ranges are guaranteed by the library parser), this converter is
//! total: it never fails and, per ADR-0016, allocates `O(events)`, iterates
//! (no recursion), and cannot panic on any typed event.
//!
//! One deliberate divergence from the C source: the reference's `WriteTime`
//! accumulates the variable-length quantity in a 32-bit `buffer`
//! (`mus2mid.c:104-136`), which overflows for delta times that need five VLQ
//! bytes (values `>= 2^28`). [`write_varlen`] instead emits a correct standard
//! MIDI VLQ for the full `u32` range. The output is byte-identical to the
//! reference for every value the reference encodes correctly; it only differs
//! where the reference would corrupt its own output.

use crustywad::audio::{MusEventKind, MusScore};

/// Standard MIDI type-0 header plus the track header (`mus2mid.c:68-77`).
///
/// `MThd`, header size 6, MIDI type 0, one track, resolution `0x0046` (70),
/// then `MTrk` and a four-byte placeholder for the track length that
/// [`convert`] backfills. This is exactly the 22-byte `midiheader[]` array the
/// engine writes verbatim (`mus2mid.c:507`).
const MIDI_HEADER: [u8; 22] = [
    b'M', b'T', b'h', b'd', // main header
    0x00, 0x00, 0x00, 0x06, // header size (6)
    0x00, 0x00, // MIDI type (0)
    0x00, 0x01, // number of tracks (1)
    0x00, 0x46, // resolution (70)
    b'M', b'T', b'r', b'k', // start of track
    0x00, 0x00, 0x00, 0x00, // placeholder for track length
];

/// Byte offset of the track-length placeholder inside [`MIDI_HEADER`]
/// (`mus2mid.c:676` seeks to 18 to backfill it).
const TRACK_LENGTH_OFFSET: usize = 18;

/// The 15-entry MUS-controller -> MIDI-controller table (`mus2mid.c:94-98`).
///
/// Index 0 is the patch-change slot (handled specially, never read here);
/// indices `1..=9` are the valued controllers; indices `10..=14` are the
/// valueless system controllers. The library guarantees a MUS
/// `ChangeController` carries controller `0..=9` and a `SystemEvent` carries
/// `10..=14`, so every index this module forms is in range.
const CONTROLLER_MAP: [u8; 15] = [
    0x00, 0x20, 0x01, 0x07, 0x0A, 0x0B, 0x5B, 0x5D, 0x40, 0x43, 0x78, 0x7B, 0x7E, 0x7F, 0x79,
];

/// MIDI percussion channel (`MIDI_PERCUSSION_CHAN`, `mus2mid.c:30`).
const MIDI_PERCUSSION_CHAN: u8 = 9;
/// MUS percussion channel (`MUS_PERCUSSION_CHAN`, `mus2mid.c:31`).
const MUS_PERCUSSION_CHAN: u8 = 15;
/// Number of MUS/MIDI channels (`NUM_CHANNELS`, `mus2mid.c:28`).
const NUM_CHANNELS: usize = 16;

// MIDI event status nibbles (`mus2mid.c:47-53`).
/// `midi_releasekey` — note-off (`mus2mid.c:47`).
const MIDI_RELEASE_KEY: u8 = 0x80;
/// `midi_presskey` — note-on (`mus2mid.c:48`).
const MIDI_PRESS_KEY: u8 = 0x90;
/// `midi_changecontroller` (`mus2mid.c:50`).
const MIDI_CHANGE_CONTROLLER: u8 = 0xB0;
/// `midi_changepatch` (`mus2mid.c:51`).
const MIDI_CHANGE_PATCH: u8 = 0xC0;
/// `midi_pitchwheel` (`mus2mid.c:53`).
const MIDI_PITCH_WHEEL: u8 = 0xE0;

/// The all-notes-off controller emitted on a channel's first allocation — the
/// "`D_DDTBLU` disease" fix (`mus2mid.c:408`, controller `0x7b`).
const ALL_NOTES_OFF: u8 = 0x7B;

/// Appends `value` to `out` as a standard MIDI variable-length quantity.
///
/// Seven bits per byte, big-endian, every byte but the last carrying the high
/// continuation bit. This matches the reference `WriteTime` (`mus2mid.c:104-136`)
/// for all values it encodes correctly, and additionally handles values
/// `>= 2^28` (five VLQ bytes) that the reference's 32-bit accumulator would
/// overflow — see the module note.
fn write_varlen(out: &mut Vec<u8>, value: u32) {
    // At most five 7-bit groups cover a u32. Fill from the least-significant
    // group upward, then emit the used suffix big-endian.
    let mut bytes = [0u8; 5];
    bytes[4] = (value & 0x7F) as u8;
    let mut i = 4;
    let mut v = value >> 7;
    while v != 0 {
        i -= 1;
        bytes[i] = ((v & 0x7F) as u8) | 0x80;
        v >>= 7;
    }
    out.extend_from_slice(&bytes[i..]);
}

/// Incremental MUS -> MIDI converter state, mirroring the reference's file-scoped
/// statics (`mus2mid.c:80-100`).
struct Converter {
    /// Track event bytes accumulated after the header; its length is the
    /// reference's `tracksize` (`mus2mid.c:92`).
    track: Vec<u8>,
    /// Per-MUS-channel MIDI channel allocation; `-1` means unallocated
    /// (`channel_map`, `mus2mid.c:100`, initialized at `mus2mid.c:474-477`).
    channel_map: [i32; NUM_CHANNELS],
    /// Per-MIDI-channel cached note velocity, initialized to 127
    /// (`channelvelocities`, `mus2mid.c:80-84`).
    velocities: [u8; NUM_CHANNELS],
    /// Accumulated delta time awaiting the next event (`queuedtime`,
    /// `mus2mid.c:88`).
    queued_time: u32,
}

impl Converter {
    fn new() -> Self {
        Self {
            track: Vec::new(),
            channel_map: [-1; NUM_CHANNELS],
            velocities: [127; NUM_CHANNELS],
            queued_time: 0,
        }
    }

    /// Flushes the queued delta time as a VLQ and resets it (`WriteTime`,
    /// `mus2mid.c:104-136`). Every event emission is prefixed by this.
    fn write_time(&mut self) {
        write_varlen(&mut self.track, self.queued_time);
        self.queued_time = 0;
    }

    /// Allocates the next free MIDI channel, skipping the percussion channel
    /// (`AllocateMIDIChannel`, `mus2mid.c:350-382`).
    fn allocate_midi_channel(&self) -> u8 {
        let max = self.channel_map.iter().copied().max().unwrap_or(-1);
        let mut result = max + 1;
        if result == i32::from(MIDI_PERCUSSION_CHAN) {
            result += 1;
        }
        // At most 15 melodic MUS channels (0..=14) ever allocate, mapping to
        // MIDI channels 0..=15 skipping 9, so `result` stays within a u8 and a
        // valid channel nibble.
        u8::try_from(result).unwrap_or(MIDI_PERCUSSION_CHAN + 1)
    }

    /// Maps a MUS channel to its MIDI channel, allocating (and emitting the
    /// first-use all-notes-off controller) on demand (`GetMIDIChannel`,
    /// `mus2mid.c:387-414`).
    fn get_midi_channel(&mut self, mus_channel: u8) -> u8 {
        if mus_channel == MUS_PERCUSSION_CHAN {
            return MIDI_PERCUSSION_CHAN;
        }
        let idx = usize::from(mus_channel); // 0..=15 from the descriptor nibble
        if self.channel_map[idx] == -1 {
            let allocated = self.allocate_midi_channel();
            self.channel_map[idx] = i32::from(allocated);
            // First use of the channel: send "all notes off" (mus2mid.c:405-409).
            self.write_change_controller_valueless(allocated, ALL_NOTES_OFF);
        }
        u8::try_from(self.channel_map[idx]).unwrap_or(0)
    }

    /// Note-on with an explicit velocity (`WritePressKey`, `mus2mid.c:158-191`).
    fn write_press_key(&mut self, channel: u8, key: u8, velocity: u8) {
        self.write_time();
        self.track.push(MIDI_PRESS_KEY | channel);
        self.track.push(key & 0x7F);
        self.track.push(velocity & 0x7F);
    }

    /// Note-off, encoded as status `0x80` with a zero velocity byte
    /// (`WriteReleaseKey`, `mus2mid.c:194-226`).
    fn write_release_key(&mut self, channel: u8, key: u8) {
        self.write_time();
        self.track.push(MIDI_RELEASE_KEY | channel);
        self.track.push(key & 0x7F);
        self.track.push(0);
    }

    /// Pitch bend: `wheel = value * 64`, split LSB then MSB, 7 bits each
    /// (`WritePitchWheel`, `mus2mid.c:229-260`, called with `key * 64` at
    /// `mus2mid.c:571`).
    fn write_pitch_wheel(&mut self, channel: u8, value: u8) {
        let wheel = u16::from(value) * 64;
        self.write_time();
        self.track.push(MIDI_PITCH_WHEEL | channel);
        self.track.push((wheel & 0x7F) as u8);
        self.track.push(((wheel >> 7) & 0x7F) as u8);
    }

    /// Patch (instrument) change (`WriteChangePatch`, `mus2mid.c:263-288`).
    fn write_change_patch(&mut self, channel: u8, patch: u8) {
        self.write_time();
        self.track.push(MIDI_CHANGE_PATCH | channel);
        self.track.push(patch & 0x7F);
    }

    /// Valued controller change; a value with bit 7 set clamps to `0x7F` (the
    /// vanilla-DOOM quirk fix, `WriteChangeController_Valued`,
    /// `mus2mid.c:292-337`, clamp at `mus2mid.c:319-327`).
    fn write_change_controller_valued(&mut self, channel: u8, control: u8, value: u8) {
        self.write_time();
        self.track.push(MIDI_CHANGE_CONTROLLER | channel);
        self.track.push(control & 0x7F);
        let value = if value & 0x80 != 0 { 0x7F } else { value };
        self.track.push(value);
    }

    /// Valueless controller change — a valued change with value 0
    /// (`WriteChangeController_Valueless`, `mus2mid.c:340-346`).
    fn write_change_controller_valueless(&mut self, channel: u8, control: u8) {
        self.write_change_controller_valued(channel, control, 0);
    }

    /// End-of-track meta event, prefixed by the final queued delta
    /// (`WriteEndTrack`, `mus2mid.c:140-156`, `endtrack = {0xFF, 0x2F, 0x00}`).
    fn write_end_track(&mut self) {
        self.write_time();
        self.track.extend_from_slice(&[0xFF, 0x2F, 0x00]);
    }
}

/// Converts a parsed [`MusScore`] to a format-0 standard MIDI file.
///
/// The returned bytes are a complete SMF: [`MIDI_HEADER`] with the track length
/// backfilled, followed by the single `MTrk` event stream. Conversion walks the
/// typed events to `ScoreEnd` (`mus2mid.c:504-667`); it never fails and never
/// panics on any typed event.
#[must_use]
pub fn convert(score: &MusScore) -> Vec<u8> {
    let mut c = Converter::new();

    for event in score.events() {
        // ScoreEnd terminates the stream; the reference stops here too
        // (mus2mid.c:634-636). We break before channel allocation so a
        // ScoreEnd's (musically meaningless) channel nibble never triggers a
        // spurious channel allocation.
        if matches!(event.kind, MusEventKind::ScoreEnd) {
            break;
        }

        // The MIDI channel is resolved first, exactly as the reference calls
        // GetMIDIChannel before its event switch (mus2mid.c:524); a first-use
        // channel therefore emits its all-notes-off controller ahead of this
        // event, consuming the queued delta.
        let channel = c.get_midi_channel(event.channel);

        match event.kind {
            MusEventKind::ReleaseKey { note } => c.write_release_key(channel, note),
            MusEventKind::PressKey { note, velocity } => {
                // An explicit velocity updates the per-channel cache; its
                // absence reuses the cached value (mus2mid.c:548-559). The
                // library already masks the velocity to 7 bits.
                let velocity = match velocity {
                    Some(v) => {
                        c.velocities[usize::from(channel)] = v & 0x7F;
                        v & 0x7F
                    }
                    None => c.velocities[usize::from(channel)],
                };
                c.write_press_key(channel, note, velocity);
            }
            MusEventKind::PitchWheel { value } => c.write_pitch_wheel(channel, value),
            MusEventKind::SystemEvent { controller } => {
                // Guaranteed 10..=14 by the library; indexing is in range.
                let control = CONTROLLER_MAP[usize::from(controller)];
                c.write_change_controller_valueless(channel, control);
            }
            MusEventKind::ChangeController { controller, value } => {
                if controller == 0 {
                    // Controller 0 is the patch change (mus2mid.c:608-615).
                    c.write_change_patch(channel, value);
                } else {
                    // Guaranteed 1..=9 by the library; indexing is in range.
                    let control = CONTROLLER_MAP[usize::from(controller)];
                    c.write_change_controller_valued(channel, control, value);
                }
            }
            // Handled above; kept for exhaustiveness.
            MusEventKind::ScoreEnd => break,
        }

        // The delta following this event becomes the queued time flushed before
        // the next event (mus2mid.c:648-666). Saturating, matching the
        // library's own accumulation of `delay`.
        c.queued_time = c.queued_time.saturating_add(event.delay);
    }

    c.write_end_track();

    let mut out = MIDI_HEADER.to_vec();
    let track_size = u32::try_from(c.track.len()).unwrap_or(u32::MAX);
    out[TRACK_LENGTH_OFFSET..TRACK_LENGTH_OFFSET + 4].copy_from_slice(&track_size.to_be_bytes());
    out.extend_from_slice(&c.track);
    out
}

#[cfg(test)]
mod tests {
    use super::{convert, write_varlen};
    use crustywad::ParseOptions;
    use crustywad::audio::MusScore;

    /// Exercises every event kind and both channel-mapping rules in one
    /// score: melodic channel 2 (lazily allocated to MIDI 0, emitting the
    /// first-use all-notes-off), percussion channel 15 (fixed MIDI 9, no
    /// allocation), an explicit velocity updating the per-channel cache, a
    /// velocity-less press reusing it, a patch change (controller 0), a
    /// valued controller with bit 7 set (clamped to 0x7F), a valueless
    /// system event, and a pitch wheel.
    /// Ten melodic MUS channels allocate MIDI channels 0..=8 and then 10 —
    /// the allocator must skip percussion channel 9
    /// (`AllocateMIDIChannel`, `mus2mid.c:376-379`).
    #[test]
    fn tenth_melodic_channel_allocation_skips_percussion() {
        let mut events = Vec::new();
        for ch in 0u8..10 {
            events.extend_from_slice(&[0x10 | ch, 0xBC, 0x64]);
        }
        events.push(0x60);
        let mut mus = Vec::new();
        mus.extend_from_slice(&[0x4D, 0x55, 0x53, 0x1A]);
        mus.extend_from_slice(&u16::try_from(events.len()).unwrap().to_le_bytes());
        mus.extend_from_slice(&14u16.to_le_bytes());
        mus.extend_from_slice(&10u16.to_le_bytes());
        mus.extend_from_slice(&0u16.to_le_bytes());
        mus.extend_from_slice(&0u16.to_le_bytes());
        mus.extend_from_slice(&events);
        let score = MusScore::parse(&mus, &ParseOptions::strict()).expect("fixture parses");

        let out = convert(&score);
        let track = &out[super::MIDI_HEADER.len()..];
        // The tenth press (MUS channel 9) lands on MIDI channel 10, not 9.
        assert!(track.windows(4).any(|w| w == [0x00, 0x9A, 0x3C, 0x64]));
        // Nothing melodic ever writes to MIDI channel 9's note-on status.
        assert!(!track.windows(4).any(|w| w == [0x00, 0x99, 0x3C, 0x64]));
    }

    #[test]
    fn converts_every_event_kind_and_channel_rule() {
        let mut mus = Vec::new();
        mus.extend_from_slice(&[0x4D, 0x55, 0x53, 0x1A]);
        mus.extend_from_slice(&19u16.to_le_bytes()); // score_length
        mus.extend_from_slice(&14u16.to_le_bytes()); // score_start
        mus.extend_from_slice(&2u16.to_le_bytes()); // primary channels
        mus.extend_from_slice(&0u16.to_le_bytes());
        mus.extend_from_slice(&0u16.to_le_bytes()); // no instrument list
        mus.extend_from_slice(&[
            0x12, 0xBC, 0x64, // press ch2 note 60 velocity 100
            0x2F, 0x40, // pitch wheel ch15 value 0x40
            0x32, 0x0A, // system event ch2 controller 10
            0x42, 0x00, 0x05, // patch change ch2 patch 5
            0x42, 0x03, 0x90, // valued controller 3 value 0x90 (bit 7 set)
            0x12, 0x3C, // press ch2 note 60, no velocity (cache reuse)
            0x82, 0x3C, 0x46, // release ch2 note 60, delta 70
            0x60, // score end
        ]);
        let score = MusScore::parse(&mus, &ParseOptions::strict()).expect("fixture parses");

        let out = convert(&score);
        let (header, track) = out.split_at(super::MIDI_HEADER.len());
        assert_eq!(
            &header[..super::TRACK_LENGTH_OFFSET],
            &super::MIDI_HEADER[..super::TRACK_LENGTH_OFFSET]
        );

        // Each expected event, derived against mus2mid.c:
        let expected: Vec<u8> = vec![
            0x00, 0xB0, 0x7B, 0x00, // first use of MIDI ch 0: all notes off
            0x00, 0x90, 0x3C, 0x64, // note on ch0 note 60 vel 100
            0x00, 0xE9, 0x00, 0x20, // pitch wheel MIDI ch9: 0x40*64=4096 -> LSB 0, MSB 0x20
            0x00, 0xB0, 0x78, 0x00, // system event 10 -> CC 0x78, value 0
            0x00, 0xC0, 0x05, // patch change ch0 -> 5
            0x00, 0xB0, 0x07, 0x7F, // controller 3 -> CC 0x07, 0x90 clamps to 0x7F
            0x00, 0x90, 0x3C, 0x64, // velocity-less press reuses cached 100
            0x00, 0x80, 0x3C, 0x00, // note off ch0 note 60 (velocity 0)
            0x46, 0xFF, 0x2F, 0x00, // delta 70 then end-of-track
        ];
        assert_eq!(track, expected.as_slice());

        // The backfilled track length matches the actual track bytes.
        let len_bytes = &header[super::TRACK_LENGTH_OFFSET..super::TRACK_LENGTH_OFFSET + 4];
        assert_eq!(
            u32::from_be_bytes(len_bytes.try_into().unwrap()),
            u32::try_from(track.len()).unwrap()
        );
    }

    /// Encodes a single value to its own buffer for assertion.
    fn vlq(value: u32) -> Vec<u8> {
        let mut out = Vec::new();
        write_varlen(&mut out, value);
        out
    }

    #[test]
    fn vlq_zero() {
        assert_eq!(vlq(0), vec![0x00]);
    }

    #[test]
    fn vlq_max_single_byte() {
        // 127 is the largest value that fits in one VLQ byte.
        assert_eq!(vlq(127), vec![0x7F]);
    }

    #[test]
    fn vlq_first_two_byte() {
        // 128 rolls over into two bytes: 0x81 0x00.
        assert_eq!(vlq(128), vec![0x81, 0x00]);
    }

    #[test]
    fn vlq_two_hundred() {
        // 200 = 0xC8: groups 0x01, 0x48 -> 0x81 0x48.
        assert_eq!(vlq(200), vec![0x81, 0x48]);
    }

    #[test]
    fn vlq_max_two_byte() {
        // 16383 = 0x3FFF is the largest two-byte VLQ: 0xFF 0x7F.
        assert_eq!(vlq(16383), vec![0xFF, 0x7F]);
    }

    #[test]
    fn vlq_first_three_byte() {
        // 16384 = 0x4000 rolls into three bytes: 0x81 0x80 0x00.
        assert_eq!(vlq(16384), vec![0x81, 0x80, 0x00]);
    }

    #[test]
    fn vlq_saturated_u32_max() {
        // u32::MAX = 0xFFFFFFFF needs five VLQ bytes: groups (high->low)
        // 0x0F,0x7F,0x7F,0x7F,0x7F with continuation bits set on all but the
        // last -> 0x8F 0xFF 0xFF 0xFF 0x7F. (The reference's 32-bit `buffer`
        // overflows here; this correct encoder does not.)
        assert_eq!(vlq(u32::MAX), vec![0x8F, 0xFF, 0xFF, 0xFF, 0x7F]);
    }

    #[test]
    fn vlq_2_28_boundary() {
        // 2^28 = 0x10000000 is the smallest five-byte value.
        assert_eq!(vlq(1 << 28), vec![0x81, 0x80, 0x80, 0x80, 0x00]);
    }
}
