# Reading pk3 Archives

With the `archive` feature enabled, `crustywad::archive::Archive` reads pk3
(zip) resource archives — the container ZDoom-family engines use alongside
WADs. It lists members with the namespace their directory selects, derives
the engine's 8-character short names, finds embedded WADs and `maps/*.wad`
maps, and hands out `Wad` values for the WADs inside, so a pk3's maps parse
exactly like a standalone WAD's (ADR-0031).

```toml
[dependencies]
crustywad = { version = "0.9.5", features = ["archive"] }
```

## Opening an archive

Detection is by magic bytes, never extension: `.pk3`, `.zip`, `.pke`, and
`.ipk3` all open the same way, and a 7z-based `.pk7` is refused with an error
that names the format.

```rust,no_run
use crustywad::archive::{Archive, MapKind, Namespace};

let archive = Archive::from_path("mod.pk3")?;

for member in archive.members() {
    println!(
        "{:<40} {:?} {:?} {} bytes",
        member.path(),
        member.namespace(),
        member.short_name(),
        member.size()
    );
}

// `maps/MAP01.wad` and friends, in directory order.
for map in archive.maps() {
    if map.kind() == MapKind::Wad {
        let member = &archive.members()[map.member_index()];
        let wad = archive.wad(member)?;
        println!("{}: {} map group(s)", map.name(), wad.map_groups().len());
    }
}

// A `.wad` at the archive root is an embedded WAD the engine loads whole.
for member in archive.embedded_wads() {
    let wad = archive.wad(member)?;
    println!("embedded {}: {} lumps", member.path(), wad.lump_count());
}

let sprites = archive
    .members()
    .iter()
    .filter(|m| m.namespace() == Namespace::Sprites)
    .count();
println!("{sprites} sprite members");
# Ok::<(), crustywad::archive::ArchiveError>(())
```

## What is and is not modeled

| Modeled | Deferred (ADR-0031 §2) |
|---|---|
| Members, sizes, compression method | Cross-member lookup by short name + namespace |
| Namespace from the first directory (GZDoom's table) | `filter/<game>/` handling and `.{id}` resource IDs |
| Short name: basename, extension stripped, uppercased, 8 bytes, `^` → `\` | Bridging `Namespace` to the WAD-side `SectionKind` |
| Embedded WADs: root `.wad`, or `<stem>/<file>.wad` (see `Archive::with_name`) | Parsing bare `maps/*.map` `TEXTMAP` members (listed as `MapKind::Textmap`) |
| `maps/<NAME>.wad` / `.map` enumeration | pk7 (7z) decoding — the seam exists, the decoder does not |

## Limits and strictness

Nothing is decoded when an archive opens; the central directory alone decides
what is listed. Two `Limits` fields bound the work, in **both** strictness
modes: `max_archive_members` (default 65,536) caps the declared member count
before the member table is allocated, and `max_decoded_member_bytes` (default
256 MiB) caps a single `read()`. Only stored and deflate members can be read;
any other method, and any encrypted member, is rejected by name.

In strict mode those central-directory facts fail `from_bytes`. In lenient
mode the member is still listed, an `ArchiveWarning` is recorded, and
`read()` fails on that member alone — so `cwad list` can still show the
layout of a pk3 packed with lzma. A duplicate member path is different: zips
permit duplicates and GZDoom keeps the later entry, so a duplicate is never
an error in either strictness mode — both members stay listed, `member()`
resolves to the later one, and lenient mode additionally records
`ArchiveWarning::DuplicatePath`. Facts only extraction reveals (a local
header disagreeing with the directory, a corrupt deflate stream, a size or
CRC-32 lie) are `read()` errors in both modes.

A `Member` is only valid against the `Archive` that produced it: passing one
to `read`/`wad` on a different `Archive` fails with
`ArchiveError::ForeignMember` rather than reading whatever entry happens to
occupy that index there.

```rust
use crustywad::archive::Archive;
use crustywad::{Limits, ParseOptions};

let mut options = ParseOptions::lenient();
options.limits = Limits::new()
    .with_max_archive_members(10_000)
    .with_max_decoded_member_bytes(64 << 20);
// `Archive::from_bytes_with_options(bytes, options)` then applies them.
let _ = options;
```
