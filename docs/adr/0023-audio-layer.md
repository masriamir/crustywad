# ADR-0023: The audio layer — DMX sound, MUS music, instrument banks, Raven scripts, and Doom 64 containers

- **Status:** Accepted
- **Date:** 2026-07-18
- **Deciders:** @masriamir
- **Tracking issue:** https://github.com/masriamir/crustywad/issues/299

## Context and problem statement

The roadmap's v0.7.0 milestone opens audio — the last major classic lump
domain after the v0.6.0 graphics layer — and its single draft issue (#158,
"Audio lump support") records no decision about formats, detection, API
shape, or staging. This spike (#299) ran four engine-source research passes
plus an empirical survey of the entire retail collection, and this ADR
records the decisions before implementation begins, per the project's
ADR-before-code process.

- **Classic sound effects** (Chocolate Doom — `src/i_sdlsound.c`,
  `src/i_pcsound.c`, `pcsound/pcsound.c`, `src/i_sound.h`, `src/w_wad.c`):
  the DMX digital-sound (`DS*`) and PC-speaker (`DP*`) byte layouts, the
  engine's exact validation set, and the name-prefix rules.
- **Music and instrument banks** (Chocolate Doom — `src/mus2mid.c`,
  `src/i_oplmusic.c`, `src/gusconf.c`, `src/i_sdlmusic.c`, `src/midifile.c`):
  the MUS header and complete event encoding, the engine's content-detection
  rule, and the GENMIDI/DMXGUS instrument-bank layouts.
- **Raven divergence** (Chocolate Doom — `src/heretic/`, `src/hexen/`,
  `src/hexen/sc_man.c`, `src/hexen/sn_sonix.c`): Heretic and Hexen share the
  DMX/MUS byte formats through the same loaders; the divergence is entirely
  in lump *naming* — plus three Raven-only lumps (`SNDCURVE`, and Hexen's
  `SNDINFO`/`SNDSEQ` text scripts).
- **Doom 64** (svkaiser/Doom64EX including `wadgen`, and Doom64EX-Plus):
  three distinct IWAD generations store audio three different ways; the
  retail KEX remaster (the collection's `DOOM64.WAD`) uses plain WAV and
  MIDI containers inside marker sections.
- **Empirical survey** of all 17 retail WADs (`RETAIL/`): every digital
  sound in the collection satisfies `lump_size == 8 + length_field`
  (1,335 lumps across id, Freedoom, Raven, and Rogue WADs, zero
  exceptions); all 867 `DP*` lumps satisfy `lump_size == 4 +
  length_field`; Freedoom's
  `D_*` music lumps are **standard MIDI, not MUS** (76 lumps); Doom 64's
  `DS_START..DS_END` holds 93 canonical 44-byte-header PCM WAVs
  (92 × 22050 Hz mono 16-bit + 1 × 44100 Hz, `44 + data_length ==
  lump_size` with zero anomalies) and `DM_START..DM_END` holds 24 standard
  MIDI files; and `strife1.wad`'s `DSTAA0` is a *sprite* whose name
  collides with the `DS` prefix — decoding as a classic picture header,
  not a sound.

## Decision

### 1. Content-first identification: `AudioKind::detect`

Audio lumps are identified by **content, never by name**. The survey
produced three independent falsifications of name-based detection:

- Freedoom ships MIDI bytes under the same `D_*` names id ships MUS under —
  the name convention does not determine the content format.
- `strife1.wad`'s sprite `DSTAA0` (and any sprite whose name starts `DS`)
  collides with the sound prefix; only the bytes distinguish them.
- Hexen's sound lump names are data-driven via `SNDINFO` and its music
  names via MAPINFO — there is no name convention to key on at all
  (`src/hexen/sounds.c` stores empty lump names; `src/hexen/s_sound.c:953-1013`
  fills them at runtime).

A new `AudioKind::detect(bytes) -> AudioKind` classifier recognizes, in
order of magic specificity: `MThd` (standard MIDI), `RIFF....WAVE` (WAV),
`MUS\x1a` (MUS), the DMX digital-sound shape (`format == 3` plus the header
arithmetic below), and the PC-speaker shape (`format == 0` plus its
arithmetic); anything else is `Unknown`. Detection is a **classifier, not a
validator** — it never allocates and never errors; the per-format `parse`
constructors (§2) do the validating.

Vanilla's actual rule is documented but deliberately **not** replicated:
the engine sniffs only `MThd` and additionally requires `len < 96 KiB`
(`MAXMIDLENGTH`, `src/i_oplmusic.c:39`, `:1656-1665`), routing everything
else — including a large valid MIDI — into the MUS converter, and it never
checks the MUS magic at all (the check exists only under a compiled-out
`#ifdef CHECK_MUS_HEADER`, `src/mus2mid.c:486-495`). The 96 KiB cap is a
vanilla playback limitation, not a format property; a rustdoc note records
the divergence.

### 2. Typed classic decode in core, dependency-free

Audio decode ships in the core crate with **no feature flag and no new
dependencies** — the same precedent as map records and the classic graphics
tiers (ADR-0022 §3): every parser below is a bounded read over `&[u8]`.
All types follow the shipped gfx idiom (`Picture::parse` in
`crates/crustywad/src/gfx/picture.rs`): `parse(bytes: &[u8], options:
&ParseOptions) -> Result<Self, AudioError>` with lenient-mode warnings
carried inside the returned value and exposed by a `warnings()` accessor.

**`DmxSound`** — the DMX digital-sound lump. On-disk layout (engine read:
`src/i_sdlsound.c:737-772`):

| Offset | Size | Type | Meaning |
|---|---|---|---|
| 0x00 | 2 | u16 LE | format number; `3` for digital sounds |
| 0x02 | 2 | u16 LE | sample rate in Hz (11025 dominant; 22050/16000/44100 observed) |
| 0x04 | 4 | u32 LE | length: byte count after the 8-byte header, **including** both pads |
| 0x08 | 16 | u8[16] | leading pad (engine skips; contents carry no semantics) |
| 0x18 | N−32 | u8[] | unsigned 8-bit mono PCM samples |
| end−16 | 16 | u8[16] | trailing pad (engine skips) |

Strict mode requires the structural invariants: `lump_len >= 8`,
`format == 3`, `32 <= length <= lump_len - 8`. Trailing slack
(`lump_len > 8 + length`) is tolerated with a warning in both modes — the
engine tolerates it silently, and the retail survey shows exact equality
everywhere, so slack is unusual but not malformed. Lenient mode recovers a
length-field overrun by clamping to the available bytes with a warning.
The engine's additional `length <= 48` rejection (`src/i_sdlsound.c:759`)
is a *playability* floor its own comment calls approximate ("the actual
cut-off length seems to vary slightly depending on the sample rate…
needs further investigation", `:753-757`); it is **not** a structural
property, so crustywad surfaces `length <= 48` as a warning in both modes
rather than an error. A zero sample rate is a warning (the engine performs
no guard and would divide by zero downstream; a parser must not
propagate that hazard silently). The samples accessor exposes the
pad-stripped PCM view — offset 24, count `length − 32`, exactly the span
vanilla plays (`data += 16; length -= 32;` then `data + 8`,
`src/i_sdlsound.c:764-772`) — alongside the raw bytes.

**`PcSpeakerSound`** — the PC-speaker lump (engine read:
`src/i_pcsound.c:126-141`): u16 LE format (`0`), u16 LE tone count, then
one tone byte per 1/140 s tick (`:68`, `:95-96`). Strict requires
`lump_len >= 4` — **deliberately stricter than the engine, which reads the
header without any minimum-length check** (an out-of-bounds read on a
sub-4-byte lump) — plus `format == 0` and `count <= lump_len - 4`. Tone
bytes index a 128-entry divisor table (`pcsound/pcsound.c:44-62`); values
`>= 128` (or index 0) render as silence in-engine
(`src/i_pcsound.c:86-93`), so out-of-range tones are a warning, not an
error, in both modes. A zero-count body is valid (engine-accepted).

**`MusScore`** — the MUS music lump. 14-byte header (struct `musheader`,
`src/mus2mid.c:57-65`): 4-byte magic `MUS\x1a`, then five u16 LE fields —
`score_length`, `score_start` (byte offset from lump start to the first
event), `primary_channels`, `secondary_channels`, `instrument_count` —
followed by `instrument_count` u16 LE patch numbers. The reference
converter never reads the instrument list and never uses `score_length`
for bounds (it seeks straight to `score_start` and reads events until
score-end or EOF, `src/mus2mid.c:498-502`); crustywad parses the
instrument list when `14 + 2 × instrument_count <= score_start` and
otherwise records a warning, and cross-checks `score_start <= lump_len`
(strict error — the engine's equivalent is a failed seek) and
`score_start + score_length <= lump_len` (warning only, matching the
engine's indifference to the field).

The event stream decodes to typed events per the reference converter
(`src/mus2mid.c:511-667`): descriptor byte = bit 7 "delta-time follows",
bits 6-4 event type, bits 3-0 channel; six event types (release-key,
press-key with optional velocity byte when bit 7 of the key is set,
pitch-wheel, system-event with controller 10..=14, change-controller with
controller 0 = patch change / 1..=9 = valued controller, score-end); the
15-entry MUS→MIDI controller table; channel 15 = percussion; delta-times
as base-128 big-endian varints. Strict mode fails exactly where the engine
fails: truncated payload, system-event controller outside 10..=14, valued
controller outside 1..=9, or an unknown event type (0x50/0x70). Lenient
mode keeps the events parsed so far and warns — a recovery vanilla does
not have (it is all-or-nothing). Bytes after score-end are ignored, as the
engine ignores them. Event decoding is iterative and bounded by the lump
length; MIDI *generation* is not part of this type (see §6, CLI staging).

**`MidiInfo`** — a shallow standard-MIDI chunk index, not an event parser:
validates the 14-byte `MThd` header (u32 BE chunk size 6, u16 BE format /
track count / division — note the endianness split: WAD directory fields
are LE, SMF content fields are BE) and walks the `MTrk` chunk frames,
bounding each declared chunk length against the remaining lump bytes.
That bound is the point: Doom64 EX's reader allocates from the declared
track count and advances a data pointer by each track's declared BE32
length with **no bound against the lump end**
(`Song_RegisterTracks`, `src/engine/system/i_audio.cc:976-1000`) — the
kind of trusted-count surface ADR-0016 exists to close. Zero-length MIDI
lumps parse successfully as empty (Doom 64's `NOSOUND` placeholder;
EX tolerates them, `i_audio.cc:1053`).

**`WavSound`** — a bounded RIFF/WAVE walk covering the shape Doom 64
ships: `RIFF` + u32 LE riff size + `WAVE`, then chunks (`fmt ` parsed —
format tag, channels, sample rate, bits per sample; `data` located;
unknown chunks skipped by their declared, bounds-checked length). The
retail KEX remaster's 93 sound lumps are all canonical 44-byte-header PCM
(§4); the chunk walk tolerates non-canonical chunk orderings leniently.
This is a container parser for WAD-embedded audio, not a general-purpose
WAV library: compressed format tags are surfaced as data, not decoded.

**`Genmidi`** — the OPL instrument bank. Fixed 11908-byte layout (derived
from the engine's pointer arithmetic, `src/i_oplmusic.c:40-75`, `:367-375`,
and confirmed byte-exact by every retail carrier): 8-byte magic
`#OPL_II#`, 128 + 47 instrument records of 36 bytes (u16 LE flags, tuning
byte, fixed-note byte, two 16-byte voices — each 6-byte modulator op,
feedback byte, 6-byte carrier op, unused byte, i16 LE base-note offset),
then 128 + 47 fixed 32-byte name fields. The engine performs **no
validation at all** — it pointer-casts the lump ("DMX does not check
header", `src/i_oplmusic.c:369`), so a truncated lump is an out-of-bounds
read in C; crustywad supplies the checks the engine omits. Strict requires
exactly 11908 bytes and the magic; lenient accepts a longer lump
(trailing slack) or a wrong magic with warnings, but never reads past
what is present.

**`Dmxgus`** — the GUS patch-mapping lump (`DMXGUS`/`DMXGUSC`), a
line-oriented text format (engine parse: `src/gusconf.c:64-153`): comma
lines of 6 fields (instrument id; mapped patch id at four GUS RAM tiers;
patch file name), `#` comments, lines with fewer fields skipped. The
engine's id-range filter — melodic `0..=127`, percussion `163..=209`,
everything else skipped (`src/gusconf.c:127-131`) — is load-bearing for
its own bounds safety; crustywad stores all well-formed entries and warns
on out-of-range ids rather than silently dropping them. Strict errors on
a malformed data line; lenient skips it with a warning (the engine skips
silently).

### 3. Raven games: same bytes, different names — plus three lumps of their own

Heretic and Hexen sound effects and music use **byte-identical formats**
to Doom — the decoders are shared, game-agnostic modules
(`src/i_sdlsound.c`, `src/i_sdlmusic.c`); the survey confirms it
empirically (350 unprefixed DMX-format lumps across the two IWADs, all
satisfying the same header invariant). The divergence is naming only:
Doom/Strife prepend `ds`/`dp` (`use_sfx_prefix = (mission == doom ||
mission == strife)`, `src/i_sdlsound.c:1067`), Heretic uses bare
compile-time lump names, and Hexen resolves symbolic tag names to lump
names at runtime through `SNDINFO`. Because §1 makes identification
content-first, **the library needs no per-game naming logic**; the naming
conventions are documented in rustdoc and the guide as context for CLI
consumers, not encoded as detection rules.

Three Raven-only lumps get typed parsers:

- **`SndCurve`** — Heretic/Hexen distance-attenuation curve
  (`src/heretic/s_sound.c:571`, `src/hexen/s_sound.c:793`): a raw byte
  table exposed as-is (observed 1600 and 2025 bytes; the length is the
  data, no header exists to validate).
- **`SndInfo`** — Hexen's sound-definition text lump
  (`S_InitScript`, `src/hexen/s_sound.c:953-1013`): `$`-directives
  (`$ARCHIVEPATH` — consumed and ignored; `$MAP <number> <songlump>` —
  map-music assignment, ignored for map 0; unknown `$`-directives
  ignored) and bare `<tagname> <lumpname>` pairs, where a `?` value means
  the literal lump `DEFAULT` — a real lump: the retail `HEXEN.WAD` ships
  a 4667-byte DMX sound named `DEFAULT`, closing what the engine source
  alone leaves ambiguous. Unknown tag names consume their value and warn
  (the engine ignores them silently).
- **`SndSeq`** — Hexen's sound-sequence script
  (`SN_InitSequenceScript`, `src/hexen/sn_sonix.c:177-307`): `:`-prefixed
  sequence names, nine commands (`play`, `playuntildone`, `playtime`,
  `playrepeat`, `delay`, `delayrand`, `volume`, `stopsound`, `end`).
  Everywhere Chocolate Hexen aborts the process — unknown command,
  nested `:`, more than 64 sequences (`SS_MAX_SCRIPTS`), buffer overflow
  — crustywad returns a strict error or a lenient warning-with-skip
  instead. Sequence *name* semantics (the engine matches against a fixed
  21-entry table) are engine policy, not lump structure: crustywad
  parses any sequence name and does not enforce the table.

Both text lumps are tokenized per the engine's `sc_man` scanner semantics
(whitespace-delimited, `;` comments, `"` quoted strings,
`src/hexen/sc_man.c:198-254`) with one deliberate improvement: the
engine truncates any token beyond 63 bytes **silently**
(`sc_man.c:236-250`); crustywad has no fixed token buffer, so nothing
truncates — but tokens longer than the engine's limit get a warning,
since the engine would have seen a different string. The ZDoom-family
`SNDINFO`/`SNDSEQ` extensions (`$random`, `$playersound`, arbitrary
sequence commands, …) are **out of scope**: this layer parses the
vanilla/Chocolate dialect and says so in rustdoc.

Heretic has no `SNDINFO`/`SNDSEQ` — its ambient sequences are hard-coded
C tables (`src/heretic/p_spec.c`), not WAD data, so there is nothing to
parse.

### 4. Doom 64: the `DM_` music section, and container parsing for the KEX remaster

Doom 64 audio exists in three IWAD generations, verified across the two
engine trees and the retail WAD:

1. **Modern Doom64EX-generated IWADs** (wadgen with `USE_SOUNDFONTS`,
   unconditionally defined — `src/engine/wadgen/wadgen.h:47`):
   `DS_START..DS_END` holds 117 standard-MIDI lumps (sound *effects* are
   single-note MIDI against an external `doomsnd.sf2` SoundFont — not WAD
   data at all).
2. **Legacy Doom64EX IWADs** (older wadgen builds): additionally
   `DOOMSND` (a raw `SN64` bank blob) and `DOOMSEQ` (a raw `SSEQ`
   sequence blob) plus `SFX_###` canonical-WAV lumps.
3. **The retail KEX remaster** (the collection's `DOOM64.WAD`, and the
   only variant Doom64EX-Plus supports): `DS_START..DS_END` = 93
   canonical PCM WAVs (`NOSOUND` + `SFX_033`…), and a marker pair the EX
   sources never emit — **`DM_START..DM_END`** — holding the 24 MIDI
   music lumps (`MUSAMB01-20`, `MUSFINAL`, `MUSDONE`, `MUSINTRO`,
   `MUSTITLE`).

Decisions:

- **`SectionKind` gains a `Music` variant** for `DM_START..DM_END`. The
  sections model (`crates/crustywad/src/sections.rs`) already recognizes
  `DS_START..DS_END` as `SectionKind::Sounds` but has no `DM` arm — today
  the remaster's music markers parse as ordinary zero-size lumps. The
  enum is `#[non_exhaustive]`, so the variant is additive (the v0.6.0
  precedent: release-plz computes the bump; an additive variant is a
  plain `feat`).
- **Variants 1 and 3 are fully covered by §2's content-first parsers**:
  everything inside both marker sections is WAV or MIDI, and
  `AudioKind::detect` + `WavSound`/`MidiInfo` handle them without any
  Doom 64-specific decode code. Doom64EX-Plus itself validates this
  approach — it ignores the markers entirely and identifies audio purely
  by content magic across the whole directory
  (`Plus/src/engine/i_audio.c:896-999`).
- **Variant 2's raw blobs (`DOOMSND`/`DOOMSEQ`) are explicitly
  deferred.** wadgen byte-swaps the `SN64`/`SSEQ` structures in place
  during processing and writes the same buffer with no swap-back found,
  so the **on-disk endianness of the emitted legacy blobs is unverified
  from source**, no legacy reader exists in either engine tree to settle
  it, and the collection has no legacy-EX fixture to test against.
  Under the project's verify-format-constants discipline that is a
  disqualifier: shipping a parser whose byte order is a guess is exactly
  the failure mode the discipline exists to prevent. The deferral is
  recorded here and in #158's re-scope, not silently dropped; a legacy
  fixture would reopen it.
- Doom 64 music is **never MUS** — real SMF under `MUS*`-prefixed names,
  the inverse of the classic trap (§1's Freedoom case). Content-first
  detection makes this a non-issue.

### 5. Hardening (ADR-0016) applies wholesale

Every trusted count, offset, or unchecked read surfaced by the research
becomes a bounded, validated, both-modes-tested requirement:

| Data | Engine trust point | This layer's requirement |
|---|---|---|
| DMX `length` field | Checked against lump size, then trusted for the pad arithmetic | `32 <= length <= lump_len - 8` before any slice; samples view derived, never indexed raw |
| `DP*` header | Read with **no** minimum-length check (OOB on < 4 bytes, `src/i_pcsound.c:126-131`) | `lump_len >= 4` required before the header read |
| MUS `score_start` / event stream | Seek + read-until-EOF; no `score_length` bound | `score_start` bounds-checked; event decode bounded by lump length, iterative |
| MIDI track count / chunk lengths | EX allocates from BE16 count, advances by BE32 lengths unbounded (`i_audio.cc:976-1000`) | Every chunk frame bounds-checked against remaining bytes before advancing |
| WAV chunk lengths | (Plus delegates to FMOD) | Declared chunk sizes bounds-checked during the walk |
| GENMIDI | Pointer-cast with zero checks (`i_oplmusic.c:369-375`) | Full 11908-byte extent required (strict) before any field read |
| DMXGUS instrument id | Range filter is the only thing keeping the array write in bounds (`gusconf.c:127-131`) | Entries stored by value; no id-indexed table, so no write to guard — out-of-range ids warn |
| SNDINFO/SNDSEQ | Process-aborting `I_Error` on malformed input; silent 63-byte token truncation | Typed errors/warnings; no fixed buffers; over-limit tokens warn |

**Bounded-work statement.** Every parser in this ADR is `O(input length)`
with allocation proportional to the input: MUS events, MIDI/WAV chunk
frames, DMXGUS entries, and SNDINFO/SNDSEQ tokens are all bounded by the
lump byte count; GENMIDI is fixed-size; nothing recurses. **No new
`Limits` field is needed** — unlike UDMF nesting (`max_depth`) or texture
composition (`max_composite_pixels`), no audio surface can amplify a
small input into a large allocation. `Limits` is untouched.

**Fuzz-target staging**, one per surface, each landing in the PR that
adds the surface (ADR-0016 §3):

1. Sound effects: `DmxSound` + `PcSpeakerSound` (+ `AudioKind::detect`
   as the harness dispatcher).
2. Music and banks: `MusScore` + `MidiInfo` + `WavSound` + `Genmidi` +
   `Dmxgus`.
3. Raven scripts: `SndInfo` + `SndSeq` (+ `SndCurve`, trivially).

Each target carries the no-panic oracle, the `O(input)` output-size
assertion, and a committed seed corpus (seeds must not begin with 8 hex
characters — the corpus-glob lesson from PR #285), wired into
`.github/workflows/fuzz.yml`.

## Staging — the v0.7.0 implementation issues

| # | Issue | Depends on |
|---|---|---|
| 1 | #158 (re-scoped): `AudioKind` detection + `DmxSound` + `PcSpeakerSound`, fuzz target 1, retail sweep anchor | — |
| 2 | new: `MusScore` + `MidiInfo` + `WavSound` + `Genmidi` + `Dmxgus`, fuzz target 2 | #158 (shares `AudioError`/module layout) |
| 3 | new: Raven scripts — `SndInfo`, `SndSeq`, `SndCurve`, fuzz target 3 | #158 |
| 4 | new: Doom 64 — `SectionKind::Music` (`DM_START..DM_END`), Doom 64 audio sweep over the retail IWAD | #2 (WAV/MIDI parsers) |
| 5 | new: CLI — `cwad extract` audio export (DMX → `.wav` container wrap; MUS → `.mus` raw and `--midi` conversion via the typed event stream; WAV/MIDI passthrough), `cwad info`/`list` audio annotations | #2 |

Issue 5's `--midi` conversion is the one elastic scope item: the MUS→MIDI
event mapping is fully specified by §2's typed events (the
`mus2mid` semantics), but if the cycle runs long it can split out without
disturbing the read-path story. The retail sweep anchors (issues 1 and 4)
extend the existing `just test-sweep` contracts: every audio lump in
`RETAIL/` must parse strict-clean — the survey says they will, and the
sweep pins it.

**API sketch** (staging vocabulary; exact names and error-enum shapes are
each issue's latitude, following the shipped gfx idiom of
`parse(bytes, &ParseOptions) -> Result<Self, Error>` with warnings
carried in the value):

```rust
// §1 (issue 1)
pub enum AudioKind { Dmx, PcSpeaker, Mus, Midi, Wav, Unknown }
impl AudioKind {
    /// Content-only classification; never errors, never allocates.
    pub fn detect(bytes: &[u8]) -> AudioKind;
}

// §2 (issues 1-2) — one parse-from-bytes constructor per lump type
impl DmxSound {
    pub fn parse(bytes: &[u8], options: &ParseOptions) -> Result<Self, AudioError>;
    /// Pad-stripped PCM view: offset 24, `length - 32` bytes.
    pub fn samples(&self) -> &[u8];
}

// §3 (issue 3)
impl SndInfo {
    pub fn parse(bytes: &[u8], options: &ParseOptions) -> Result<Self, AudioError>;
}

// §4 (issue 4): SectionKind::Music — additive variant, DM_START..DM_END
```

## Considered options

### Identification strategy

1. **Name/section-based** (prefix `DS`/`DP`/`D_` plus marker sections).
   Rejected on three empirical falsifications (§1): Freedoom's MIDI-bytes
   `D_*` lumps, the `DSTAA0` sprite collision, and Hexen's data-driven
   names.
2. **Engine-mirroring** (`MThd` + the 96 KiB cap, MUS as fallback).
   Rejected: it misroutes any large valid MIDI and blesses arbitrary
   bytes as "MUS"; vanilla's rule conflates detection with a playback
   limitation.
3. **Content-first classifier, validation in the parsers (chosen)** —
   magic/shape sniffing that never errors, with each `parse` constructor
   owning its format's validation under the standard strictness model.

### Doom 64 legacy `DOOMSND`/`DOOMSEQ` blobs

1. **Parse them now.** Rejected: the on-disk endianness is unverifiable
   from the available sources (§4) and no fixture exists — shipping a
   guessed byte order violates the verify-format-constants discipline.
2. **Defer, documented (chosen)** — variants 1 and 3 (every IWAD a user
   is realistically holding) are fully served by the content-first
   WAV/MIDI parsers; the deferral is recorded in this ADR and #158's
   re-scope, reopenable when a legacy fixture surfaces.

### Raven script depth

1. **Defer SNDINFO/SNDSEQ to a later cycle.** Rejected: they are small,
   flat, bounded text formats (nine commands; two directives), they
   complete the Hexen typing story this crate already committed to in
   the map domain, and deferring them would leave Hexen's audio
   *identification* story incomplete (its sfx names live in SNDINFO).
2. **Parse the ZDoom-extended dialect too.** Rejected: the extensions
   are port-specific, much larger, and unexercised by any engine source
   this spike verified; scoping to the vanilla/Chocolate dialect with an
   explicit rustdoc boundary keeps every claim engine-anchored.
3. **Vanilla-dialect SNDINFO/SNDSEQ/SNDCURVE in this cycle (chosen).**

### Conversion scope

1. **No conversion anywhere.** Rejected: `cwad extract` producing raw
   `.lmp` blobs for sounds is a poor CLI story when the DMX→WAV
   container wrap is a 44-byte header over already-parsed PCM, and
   MUS→MIDI is fully determined by the typed event stream this ADR
   already specifies.
2. **A general audio transcoding layer (resampling, encoding).**
   Rejected: out of charter for a WAD I/O crate; brings dependencies.
3. **Minimal, dependency-free container-level export in the CLI issue
   (chosen)** — DMX PCM wrapped in a canonical WAV header; MUS mapped to
   MIDI events per the verified `mus2mid` semantics; WAV/MIDI lumps
   passed through as-is.

## Consequences

- #158 is re-scoped to staging issue 1 and no longer describes the whole
  audio surface; four new issues carry the rest (staging table).
- `SectionKind` gains `Music` — additive on a `#[non_exhaustive]` enum, a
  plain `feat` whose release math release-plz computes (the v0.6.0
  precedent).
- No new dependencies, no new feature flags, no new `Limits` fields: the
  feature-flag four-place sync rule is **not** triggered by this cycle.
- The fuzz workflow gains three targets on the ADR-0016 pattern, each
  landing with its surface.
- The `just test-sweep` retail contracts extend to audio: every audio
  lump in the collection parses strict-clean (1,335 DMX + 867 PC-speaker
  + music + banks + Doom 64's 117 container lumps), pinned by the sweep.
- Doom 64 legacy `DOOMSND`/`DOOMSEQ` parsing is deliberately absent and
  documented as such (§4) — reopening requires a fixture, not a guess.
- Writing/serializing audio lumps is out of scope for this cycle, same
  read-then-write sequencing as graphics (ADR-0022) and maps (ADR-0019).

## More information

- Tracking issue: #299. Re-scoped by this ADR: #158.
- Related ADRs: ADR-0016 (hardening policy — §5 applies it wholesale);
  ADR-0022 (graphics layer — the section API this layer's §4 extends, and
  the in-core/no-feature-flag precedent §2 follows); ADR-0003 (strictness
  model).
- Source anchors: Chocolate Doom master (`src/i_sdlsound.c`,
  `src/i_pcsound.c`, `pcsound/pcsound.c`, `src/mus2mid.c`,
  `src/i_oplmusic.c`, `src/gusconf.c`, `src/i_sdlmusic.c`,
  `src/midifile.c`, `src/heretic/sounds.c`, `src/heretic/s_sound.c`,
  `src/hexen/sounds.c`, `src/hexen/s_sound.c`, `src/hexen/sc_man.c`,
  `src/hexen/sn_sonix.c`, `src/hexen/p_setup.c`); svkaiser/Doom64EX
  (`src/engine/wadgen/wadgen.{h,cc}`, `src/engine/wadgen/sound.{h,cc}`,
  `src/engine/wadgen/wad.cc`, `src/engine/wad/DoomWad.cc`,
  `src/engine/system/i_audio.cc`); Doom64EX-Plus
  (`src/engine/i_audio.c`, `src/engine/doomdef.h`); the empirical survey
  of all 17 `RETAIL/` WADs and the `DS`/`DM` section characterization of
  the 2020 Steam KEX `DOOM64.WAD`; the four spike research reports and
  survey outputs posted to #299.
- Out of scope, documented rather than silently dropped: audio
  **writing**; general transcoding; ZDoom-dialect SNDINFO/SNDSEQ;
  Doom 64 legacy `DOOMSND`/`DOOMSEQ` blobs (§4); Strife's out-of-WAD
  `voices.wad` speech (Strife remains `Later` per epic #241); external
  SoundFont/DLS files (`doomsnd.sf2`, `DOOMSND.DLS`) — not WAD lumps.

## Amendment (2026-07-18, #301): DMXGUS reserved-gap ids are data, not warnings

The first retail music/banks sweep falsified §2's DMXGUS claim that the
parser "warns on out-of-range ids": **every** retail `DMXGUS`/`DMXGUSC`
carrier — 11 of 11, across id, Raven, Rogue, and Freedoom — ships
instrument ids in the reserved gaps the engine's range filter skips
(`128`, `155..=162`, `210..=215` in the standard 190-line DMX file;
Freedoom's variant carries `128` alone). A property universal to retail
data is not an anomaly — the same reasoning as ADR-0022's COLORMAP
amendment. Corrected policy: reserved-gap entries parse as ordinary data
with no warning; the engine's mapped/skipped classification is exposed as
`DmxgusEntry::is_gm_mapped` (melodic `0..=127`, percussion `163..=209`,
per `gusconf.c:127-131`) so callers can reproduce the engine's filter
without the parser editorializing. The sweep's zero-warning contract for
DMXGUS holds under the corrected policy.
