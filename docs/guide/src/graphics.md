# Graphics

`crustywad::gfx` decodes the classic Doom graphics lumps: the picture format used by
patches and sprites, the raw 64×64 flat format, and the `PLAYPAL`/`COLORMAP` palette
lumps. This is "tier 1" of ADR-0022 §3's three-tier plan (raw typed lumps); tier 2
(TEXTUREx/PNAMES composition) is [#157](https://github.com/masriamir/crustywad/issues/157).

The module is **dependency-free and lives in the core crate with no feature flag** —
the same precedent map parsing set: a format this central to the WAD ecosystem does not
need a format-specific gate (ADR-0022 §3).

## The four lump types

- **`Picture`** (patches and sprites): an 8-byte header of four little-endian `i16`
  fields (`width`, `height`, `left_offset`, `top_offset`), followed by exactly `width`
  little-endian `i32` column offsets counted from the start of the lump. Each column is
  a chain of posts: `top_delta` (`u8`; `0xFF` terminates the chain), `length` (`u8`), a
  padding byte, `length` pixel bytes, and a trailing padding byte. `top_delta` is plain,
  not cumulative — vanilla has no DeePsea-style "tall patch" handling (ADR-0022 §3).
- **`Playpal`**: `N × 768` bytes (256 RGB entries per palette), with no count field on
  disk — the palette count is derived from the lump's length (`len / 768`). Strict mode
  rejects a length that is not a positive multiple of 768; lenient mode truncates the
  remainder and warns (ADR-0022 §3).
- **`Colormap`**: `N × 256` bytes (`NUMCOLORMAPS` is a vanilla compile-time constant of
  32, not a value read from the lump, and the engine loads the lump with no size check).
  Strict mode requires a whole number of 256-byte tables totaling at least 8192 bytes (the
  32-table floor); lenient mode zero-pads a short lump to 8192 or truncates a long one's
  trailing partial table (ADR-0022 §3, corrected by the §3 amendment). Retail lumps carry
  34 tables — id, Freedoom, Raven, and Rogue all ship 8704 bytes — and every table is
  exposed via `tables()`.
- **`Flat`**: a raw 64×64 blob, at least 4096 bytes — an assumption vanilla makes only at
  render time, never validated against the lump's actual length at load. Strict mode
  requires a whole number of 64-byte rows totaling at least 4096 bytes (accepting
  Heretic's 4160-byte and Hexen's 8192-byte retail flats); lenient mode keeps the actual
  bytes and warns (ADR-0022 §3, corrected by the §3 amendment).

## Strictness policy

Every lump type follows the crate-wide strict/lenient contract
(`ParseOptions::strict()`/`ParseOptions::lenient()`): strict mode returns the first
`GfxError` encountered; lenient mode recovers with a best-effort value and records the
matching `GfxWarning` — with one exception: a picture lump under 8 bytes has no
header to recover from and errors in both modes.

| Condition | Strict | Lenient |
|---|---|---|
| Picture lump under 8 bytes (no header to recover from) | `GfxError::TruncatedPicture` | Error in both modes |
| Picture lump under `8 + width × 4` bytes (offset table truncated) | `GfxError::TruncatedPicture` | Width clamped to the offsets present; `GfxWarning::TruncatedPicture` |
| Negative picture width/height | `GfxError::NegativeDimension` | Clamped to 0; `GfxWarning::NegativeDimension` |
| Column offset outside the lump (including a negative offset) | `GfxError::ColumnOffsetOutOfBounds` | Column left empty; `GfxWarning::ColumnOffsetOutOfBounds` |
| Post chain runs past the lump end without a `0xFF` terminator | `GfxError::UnterminatedColumn` | Posts fully read so far are kept; `GfxWarning::UnterminatedColumn` |
| A post's rows exceed the picture height | `GfxError::PostOutOfBounds` | Clipped to the picture height (dropped if entirely out of bounds); `GfxWarning::PostOutOfBounds` |
| Cumulative post-chain bytes consumed exceed the lump length (aliased column offsets) | `GfxError::ExcessivePostData` | Remaining columns left empty; `GfxWarning::ExcessivePostData` |
| `PLAYPAL` length not a positive multiple of 768 | `GfxError::PlaypalSize` | Remainder truncated (zero palettes for a zero-length lump); `GfxWarning::PlaypalSize` |
| `COLORMAP` length not a 256-byte multiple of at least 8192 | `GfxError::ColormapSize` | Zero-padded to 8192 (short) or truncated to whole tables (long); `GfxWarning::ColormapSize` |
| `Flat` length not a 64-byte multiple of at least 4096 | `GfxError::FlatSize` | Actual bytes kept as parsed (`to_indexed` pads or truncates to 4096); `GfxWarning::FlatSize` |

The consumed-bytes budget behind `ExcessivePostData` is a hardening addition beyond the
spec's plain post-chain description (ADR-0016 §1): cumulative bytes actually consumed
across all posts and columns (`4 + pixel length` per post) is capped at the lump length,
closing an `O(width × length)` blowup that aliased column offsets would otherwise allow.

## Worked example

```rust
use crustywad::{ParseOptions, SectionKind, Wad};
use crustywad::gfx::Picture;

# fn run(wad: &Wad) -> Result<(), Box<dyn std::error::Error>> {
let sections = wad.sections()?;
let Some(palette) = wad.playpal()? else {
    return Ok(()); // no PLAYPAL in this WAD
};

for section in sections.of_kind(SectionKind::Sprites) {
    for i in section.lumps.clone() {
        let bytes = wad.lump_bytes(i).expect("valid lump index");
        if bytes.is_empty() {
            continue; // nested sub-namespace marker
        }
        let pic = Picture::parse(bytes, &ParseOptions::strict())?;
        let rgba = pic.to_rgba(&palette.palettes()[0]);
        // `rgba.pixels` is `width * height * 4` bytes, row-major RGBA8.
        let _ = rgba;
    }
}
# Ok(())
# }
```

`Picture::to_indexed` produces an `IndexedImage` (palette indices plus a coverage mask —
posts don't have to cover every row of every column); `Picture::to_rgba` composes that
with a `Palette` in one step. `Flat` has the same `to_indexed`/`to_rgba` pair, always
fully covered since a flat has no post gaps.

## Doom 64 graphics

Doom 64's texture, sprite, and gfx lumps are complete PNG files, not this format
(ADR-0022 §3/§5). They are decoded separately behind the `doom64-gfx` feature
([#282](https://github.com/masriamir/crustywad/issues/282)) once that feature lands.

## Texture composition

`crustywad::gfx::TextureSet` is "tier 2" of ADR-0022 §3's three-tier plan: assembling
`TEXTURE1`/`TEXTURE2` texture definitions plus the `PNAMES` patch-name table and their
resolved `Picture` lumps into named, multi-patch composite images — reimplementing the
contract of vanilla's `R_GenerateComposite`/`R_GenerateLookup`.

### Worked example

```rust
use crustywad::{ParseOptions, Wad};

# fn run(wad: &Wad) -> Result<(), Box<dyn std::error::Error>> {
let Some(set) = wad.texture_set()? else {
    return Ok(()); // no TEXTURE1/TEXTURE2 in this WAD
};
let Some(index) = set.find("STARTAN2") else {
    return Ok(()); // this WAD doesn't define the texture
};
let (image, warnings) = set.compose(index, &ParseOptions::strict())?;
assert!(warnings.is_empty()); // strict mode: no warnings ever accompany Ok
// `image` is an `IndexedImage` (palette indices + a coverage mask); apply a
// palette in the same step with `compose_rgba` instead when RGBA8 is wanted:
let palette = wad.playpal()?.map(|p| p.palettes()[0].clone());
if let Some(palette) = palette {
    let (rgba, _) = set.compose_rgba(index, &ParseOptions::strict(), &palette)?;
    let _ = rgba; // width * height * 4 bytes, row-major RGBA8
}
# Ok(())
# }
```

`Wad::texture_set()` (strict) / `Wad::texture_set_with_options()` (either mode) build the
set once; `TextureSet::compose`/`compose_rgba` are then called per texture, as many times
as needed — the resolved patch pictures are shared across every `compose` call.

### Build strictness policy

Building the set parses `TEXTURE1`/`TEXTURE2` (see the [Strictness policy](#strictness-policy)
table above for those rows) and `PNAMES`, then resolves and validates every patch reference:

| Condition | Strict | Lenient |
|---|---|---|
| `TEXTUREx` present but no `PNAMES` lump exists | `GfxError::MissingPnames` | Set built with an empty name table; `GfxWarning::MissingPnames` |
| A patch reference indexes past the resolved `PNAMES` table (including a negative index) | `GfxError::PatchIndexOutOfBounds` | Reference ignored; `GfxWarning::PatchIndexOutOfBounds` — suppressed when the `PNAMES` lump is absent entirely (`MissingPnames` already explains every reference) |
| A resolved patch name matches no lump in the WAD | `GfxError::UnresolvedPatchName` | Patch left unresolved; `GfxWarning::UnresolvedPatchName` |
| A resolved patch lump fails to parse as a `Picture` | `GfxError::PatchPictureFailed` | Patch left unresolved; `GfxWarning::PatchPictureFailed` |

**The PWAD reality.** Patch names resolve through the crate's first-match
[`Wad::lump_by_name`] after uppercasing (vanilla uppercases its own search name too).
A PWAD that references patches shipped only in the base IWAD therefore cannot resolve
them here — multi-WAD merge (loading a PWAD layered over its IWAD) is out of scope for
a single `Wad` — so a retail PWAD's strict `texture_set()` commonly fails with
`GfxError::UnresolvedPatchName` even though the WAD is perfectly well-formed; building
leniently instead recovers with those patches left unresolved (composing them draws
holes rather than failing).

### Compose strictness policy

`TextureSet::compose` composites one texture already validated at build time:

| Condition | Strict | Lenient |
|---|---|---|
| Negative composed width/height | `GfxError::NegativeDimension` | Clamped to 0 (see the picture `NegativeDimension` row above) |
| `width × height` exceeds [`Limits::max_composite_pixels`] | `GfxError::CompositeTooLarge` in **both** modes | `GfxError::CompositeTooLarge` in **both** modes |
| A composited column has no contributing patch (the Medusa case) | `GfxError::MedusaColumn` | Column(s) left as holes; `GfxWarning::MedusaColumns` |

`Limits::max_composite_pixels` (default `1 << 24`) bounds the pixel buffer a single
`compose` call allocates. A `TEXTUREx` header can declare a 32767×32767 canvas (nearly
1 GiB) from a 30-byte lump, so this cap is enforced in **both** strictness modes — the
same DoS-cap exception to the strict/lenient contract that the UDMF nesting-depth limit
uses (ADR-0016): an oversized composite is a resource-exhaustion risk, not a recoverable
parse anomaly, so lenient mode does not clamp past it and instead returns the same error
strict mode does.

**Medusa policy vs. vanilla (ADR-0022 §3).** Vanilla's `R_GenerateLookup` handles a
texture column with no contributing patch by printing "column without a patch" and
**returning early from the entire function**, leaving every later column's composite
state uninitialized — a silent, partial, engine-visible bug (the well-known "Medusa
effect"). A stricter `I_Error` abort exists in the vanilla source but is commented out.
`compose` instead treats the Medusa case as a `Strictness::Strict` error and a
`Strictness::Lenient` warning-with-hole: the column decodes with an explicit gap rather
than either aborting the whole texture or silently leaving other columns corrupt —
deliberately better than either of vanilla's two behaviors. Dead patch references (an
unresolved or out-of-bounds `PNAMES` index) never count as contributors, even though
vanilla's own column-contributor count includes them regardless of lookup failure; only
*live*, resolved patches count here, which is what the explicit-holes model requires.

## What's next

Doom 64's graphics are a different family entirely — see the [Doom 64
graphics](#doom-64-graphics) note above; PNG decoding lands behind the
`doom64-gfx` feature ([#282](https://github.com/masriamir/crustywad/issues/282)).

[`Wad::lump_by_name`]: https://docs.rs/crustywad/latest/crustywad/struct.Wad.html#method.lump_by_name
[`Limits::max_composite_pixels`]: https://docs.rs/crustywad/latest/crustywad/struct.Limits.html#structfield.max_composite_pixels
