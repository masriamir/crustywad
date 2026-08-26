# ADR-0031: pk3 archive support — container plus maps, zip only

- **Status:** Accepted
- **Date:** 2026-08-25
- **Deciders:** @masriamir
- **Tracking issue:** https://github.com/masriamir/crustywad/issues/445

## Context and problem statement

The first full idgames harvest (#408) quantified the archive formats the
library cannot open: 449 of 15,716 entries (2.86%) contain no `.wad` member,
and 282 of those carry a pk3/pk7/pke — 277 `.pk3`, 5 `.pk7`, 4 `.pke`. pk3 is
1.8% of the corpus and the modern slice of it (steady from 2008 on); pk7 is
0.03%. A pk3 is not a container around a WAD: it is a zip whose directory
layout carries lump semantics for ZDoom-family engines, so "we already handle
zip" (the harvester's central-directory reader) does not carry over.

A local census of 28 pk3s (including GZDoom's own `gzdoom.pk3` and
`game_support.pk3`) found stored and deflate as the only compression methods,
no encryption, member counts from 16 to 15,105, a worst inflation ratio of
205×, and a largest decoded member of ≈ 200 MiB. Maps live in `maps/<NAME>.wad`
(up to 100 per file), root-level embedded `.wad`s are common, several files
are mispackaged behind a wrapper folder, and directory names appear in mixed
case.

## Decision

1. **A feature-gated reader in the library, consumed by the CLI.** The
   `archive` feature adds `crustywad::archive::Archive` beside `Wad`; nothing
   about `Wad`, `Lump`, or `ParseError` changes. `cwad info`, `list`, and
   `validate` accept a pk3 by magic bytes (never by extension); other
   subcommands reject one with a message that names the format. The CLI's
   `archive` feature is default-on, mirroring `extended-nodes-zlib`, so a
   `--no-default-features` build stays decompressor-free.

2. **Container plus maps — no unified lookup.** `Archive` models members
   (full path, declared sizes, method), the namespace GZDoom's directory table
   assigns, the 8-character short name, embedded-WAD detection, and
   `maps/<NAME>.wad` / `.map` enumeration. It hands out `Wad` values for the
   WADs it contains and stops there: no cross-member name resolution, no
   `filter/<game>/`, no `.{id}` resource IDs, no `Namespace`↔`SectionKind`
   bridge, no `.map` (bare `TEXTMAP`) parsing, no `mmap`. Each is a recorded
   follow-up, not a silent omission.

3. **Rules transcribed from GZDoom, cited by function.**
   `filesystem.cpp` `LumpRecord::SetFromLump` — namespace from a `strncmp`
   prefix test on `flats/ textures/ hires/ sprites/ voxels/ colormaps/ acs/
   voices/ patches/ graphics/ sounds/ music/`, root files global, anything
   else hidden; short name = basename, extension stripped, uppercased, eight
   bytes, `^` standing for `\`. `resourcefile.cpp` `FResourceFile::CheckEmbedded`
   / `IsFileInFolder` — a `.wad` at the root, or `<archive-stem>/<file>.wad`,
   is an embedded WAD. `p_openmap.cpp` — `maps/<NAME>.wad` and
   `maps/<NAME>.map` are found by full-path lookup.

4. **Own the zip reader; stored and deflate only.** A hand-rolled
   central-directory parser plus `miniz_oxide`'s length-limited inflater —
   already the crate's optional inflater — adds zero dependencies and keeps
   the ADR-0016 discipline direct: the central directory alone decides what is
   listed, nothing is decoded at open, and every allocation is bounded by
   `Limits::max_archive_members` (default 65,536) and
   `Limits::max_decoded_member_bytes` (default 256 MiB), enforced in both
   strictness modes. lzma/bzip2/xz/implode/shrink/ppmd — which GZDoom accepts —
   are rejected *by name* (`ArchiveError::UnsupportedMethod`), as are
   encrypted members; the census found none of them, and each is another
   decoder's attack surface for no observed benefit. Nesting depth is one.

5. **pk7: designed for, not shipped.** The container seam (`Container`) is a
   private trait with one implementation, so a 7z backend can slot in without
   a public change. Today the 7z signature (`7z\xbc\xaf\x27\x1c`) is recognized
   only to produce `ArchiveError::UnsupportedContainer(ContainerKind::Pk7)`.
   Five corpus entries do not justify an LZMA/7z decoder; revisit on demand.

6. **Strictness split.** Facts the central directory reveals — method,
   encryption, declared size, or a non-ASCII name — are decided at open:
   strict errors; lenient lists the member, records an `ArchiveWarning`, and
   lets `read()` fail on it later. A duplicate member path is not one of
   those facts: zips permit duplicates and GZDoom keeps the later entry — 5 of
   the 24 pk3s in the opt-in sweep collection carry exact same-case
   duplicates, the zip-tool append pattern — so a duplicate is never an error
   in either strictness mode. Both members stay in the table, `member()`
   (which searches in reverse) resolves to the later one, and lenient mode
   additionally records `ArchiveWarning::DuplicatePath`; no
   `ArchiveError::DuplicatePath` variant exists. Facts only extraction reveals
   — local-header mismatch, inflate failure, size or CRC lies — are `read()`
   errors in both modes. A WAD parse failure inside a member is
   `ArchiveError::Wad { path, source }`, so the member path is never lost, and
   a `Member` presented to an `Archive` other than the one that produced it is
   refused as `ArchiveError::ForeignMember` rather than read against whatever
   entry happens to sit at that index there.

## Consequences

- `Limits` gains two fields (`#[non_exhaustive]`, so not a breaking change);
  `ParseOptions` is reused unchanged and a pk3's maps parse exactly like a
  standalone WAD's.
- Every offset computation in the zip reader (central-directory record spans,
  the end-of-central-directory search, local-header offsets) is
  `checked_add`-bounded against attacker-controlled `u64`/`usize` values
  rather than assumed to fit; implementation caught and fixed three
  overflow/panic defects of exactly this shape (plus one non-ASCII `&str`
  byte-index slice on a member path) before they reached `main`, and the
  `fuzz_archive` target below is the standing regression proof.
- The `fuzz_archive` target and the opt-in `pk3-tests` sweep
  (`CRUSTYWAD_PK3_DIR`, `just test-pk3`) are the proof surfaces; the sample
  census above is the derivation for the limit defaults.
- Follow-ups: CLI `extract` for archive members and a `--member`/`--map`
  selector (where #446's picker question lands for the CLI); unified lookup;
  `.map` parsing; reusing this reader in the harvester (a range-read job in a
  separate workspace — a different problem today).
