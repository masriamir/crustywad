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

## Synopsis

```text
cwad [OPTIONS] <SUBCOMMAND> <FILE>
```

## Subcommands

### info

Print the WAD kind (`Iwad` or `Pwad`) and total lump count.

```text
$ cwad info doom.wad
kind:  Iwad
lumps: 1264
```

### list

Print the full lump directory. Each line contains the zero-based index, the
file offset (`filepos`), the byte size, and the lump name.

```text
$ cwad list doom.wad
0000       12     1160 PLAYPAL
0001     1172     4096 COLORMAP
0002     5268        0 ENDOOM
...
```

Column order: `index  filepos  size  name`.

### validate

Check whether a WAD file parses without errors. Prints a result line to
stdout and exits with the appropriate code (see [Exit codes](#exit-codes)).

```text
$ cwad validate doom.wad
ok: doom.wad
```

On a corrupt file:

```text
$ cwad validate broken.wad
error: broken.wad: invalid WAD magic
```

The error message goes to stderr in human format; the exit code is `2`.

## Global options

| Flag | Short | Description |
|---|---|---|
| `--lenient` | — | Use lenient parsing instead of strict; emits warnings to stderr instead of failing |
| `--format <FORMAT>` | `-F` | Output format: `human` (default), `json`, or `csv` |
| `--help` | `-h` | Print help and exit `0` |
| `--version` | `-V` | Print version and exit `0` |

### Lenient mode

In lenient mode `cwad` attempts best-effort recovery and prints warnings to
stderr for any non-fatal issues encountered.

```bash
cwad --lenient info damaged.wad
```

Example output when the WAD magic is unrecognized:

```text
kind:  Unknown([88, 87, 65, 68])
lumps: 3
warning: unrecognized WAD magic `XWAD`
```

## Output formats

All three subcommands support the `--format` / `-F` flag.

### human (default)

Human-readable text written to stdout. Warnings and errors go to stderr.

### json

Newline-delimited JSON (one object per record). Useful for scripting and
piping into tools like `jq`.

```bash
cwad -F json info doom.wad
```

```json
{"kind":"Iwad","lumps":1264}
```

```bash
cwad -F json list doom.wad
```

```json
{"index":0,"filepos":12,"size":1160,"name":"PLAYPAL"}
{"index":1,"filepos":1172,"size":4096,"name":"COLORMAP"}
```

```bash
cwad -F json validate doom.wad
```

```json
{"ok":true}
```

On parse failure the `validate` subcommand writes `{"ok":false,"error":"..."}` to stdout and exits `2`.

### csv

RFC 4180 CSV with a header row. Field values that contain commas, quotes, or
newlines are wrapped in double-quotes with internal quotes doubled.

```bash
cwad -F csv info doom.wad
```

```text
kind,lumps
Iwad,1264
```

```bash
cwad -F csv list doom.wad
```

```text
index,filepos,size,name
0,12,1160,PLAYPAL
1,1172,4096,COLORMAP
```

```bash
cwad -F csv validate doom.wad
```

```text
ok
true
```

## Exit codes

| Code | Meaning |
|---|---|
| `0` | Success |
| `2` | I/O error or parse error (malformed WAD, missing file, etc.) |
| `3` | Usage error (unknown subcommand, invalid flag value, missing required argument) |

## Man page

A man page (`cwad.1`) is generated into `$OUT_DIR/man/` at build time via
`clap_mangen`. To install it system-wide after building the crate, copy the
generated file to the appropriate man directory, for example:

```bash
install -m 644 \
  "$(cargo build -p crustywad-cli --message-format=json \
      | jq -r 'select(.reason=="build-script-executed") | .out_dir')/man/cwad.1" \
  /usr/local/share/man/man1/cwad.1
mandb
```

## Shell completions

Completion scripts for bash, zsh, and fish are generated into
`$OUT_DIR/completions/` at build time via `clap_complete`. Source the
appropriate script for your shell to enable tab completion for `cwad`
subcommands and flags.
