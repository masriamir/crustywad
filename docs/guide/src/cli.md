# CLI Usage

The `cwad` binary ships with the `crustywad-cli` crate and provides quick WAD
inspection from the command line.

## Installation

Build and install from the workspace:

```bash
cargo install --path crates/crustywad-cli
```

Or run directly without installing:

```bash
cargo run -p crustywad-cli -- <subcommand> [options] <file.wad>
```

## Subcommands

### info

Print the WAD kind and lump count.

```text
$ cwad info doom.wad
kind: Iwad
lumps: 1264
```

### list

Print the full lump directory with index, file offset, byte size, and name.

```text
$ cwad list doom.wad
0000       12     1160 PLAYPAL
0001     1172     4096 COLORMAP
0002     5268        0 ENDOOM
...
```

Column order: `index  filepos  size  name`.

## Global flags

| Flag | Description |
|---|---|
| `--lenient` | Use lenient parsing instead of strict |

### Lenient mode example

```bash
cwad --lenient info damaged.wad
```

Warnings from lenient parsing are written to stderr:

```text
warning: invalid magic bytes: "\xde\xad\xbe\xef"
kind: Unknown([222, 173, 190, 239])
lumps: 3
```

