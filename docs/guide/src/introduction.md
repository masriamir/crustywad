# Introduction

`crustywad` is a safe, documented Rust library for reading Doom WAD files. A WAD
("Where's All the Data?") is the container format that id Software's Doom engine
uses to store maps, graphics, audio, and other game assets.

## What crustywad provides

- Parse WAD headers and lump directories from bytes or files.
- Look up lumps by index or name and access their raw bytes.
- Decode typed map-record lumps (`Thing`, `Linedef`, `Sidedef`, `Vertex`, `Seg`,
  `Subsector`, `Node`, `Sector`).
- Choose between **strict** parsing (fail fast on bad data) and **lenient** parsing
  (best-effort recovery with collected warnings).
- Optional memory-mapped loading for large WADs via the `mmap` feature flag.
- A small `cwad` CLI binary for dogfooding and quick inspection.

## When to use it

Use `crustywad` when you need to:

- Extract lump data from IWAD or PWAD files for further processing.
- Build Doom map editors, converters, or analysis tools in Rust.
- Inspect WAD structure programmatically without a full game engine.

## WAD format overview

Every WAD starts with a 12-byte header:

| Field | Size | Description |
|---|---|---|
| Magic | 4 bytes | `IWAD` (game data) or `PWAD` (patch) |
| `numlumps` | 4 bytes (i32 LE) | Number of lump directory entries |
| `infotableofs` | 4 bytes (i32 LE) | Byte offset of the lump directory |

The lump directory follows the lump data. Each directory entry is 16 bytes:

| Field | Size | Description |
|---|---|---|
| `filepos` | 4 bytes (i32 LE) | Byte offset of lump data |
| `size` | 4 bytes (i32 LE) | Byte length of lump data |
| `name` | 8 bytes | NUL-padded ASCII name |

See the [Doom Wiki](https://doomwiki.org/wiki/WAD) for the full unofficial spec.

## Next steps

Start with [Getting Started](getting-started.md) to add `crustywad` to your
