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
cwad [OPTIONS] <COMMAND>
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

Check whether a WAD file parses without errors and exits with the
appropriate code (see [Exit codes](#exit-codes)).

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

#### Deep validation

`--deep` goes beyond the header and directory: after the container parses,
every map in the WAD is assembled — all four formats, including Doom 64
nested-WAD maps — with per-map errors and warnings reported. Validation
continues past a failing map so one corrupt map cannot mask another.

```text
$ cwad validate --deep doom.wad
ok: doom.wad (36 map(s) validated)
```

On a WAD whose `E1M1` has a corrupt lump:

```text
$ cwad validate --deep broken.wad
error: map E1M1: failed to decode LINEDEFS records: record stream ended mid-record at byte offset 0
error: broken.wad: 1 of 2 map(s) failed validation
```

Per-map diagnostics go to stderr; the exit code is `1` if any map fails —
ADR-0008's "validation errors found" code, distinct from `2` (the container
itself is unreadable or malformed). The
strictness flag applies: under `--lenient`, recoverable per-map issues become
warnings on stderr and the exit code stays `0`. In JSON format, `--deep` emits
one newline-delimited record per map (`{"map":"E1M1","ok":true,"warnings":0}`
or `{"map":"E1M1","ok":false,"error":"..."}`) followed by the usual summary
object; in CSV it emits a `map,ok,error` table instead of the shallow
`ok`/`true` pair.

### merge

Combine multiple WAD files into one, writing lumps in the order the input
files are given.

```text
$ cwad merge base.wad patch.wad --output combined.wad
```

Use `--kind` to set the output WAD kind (`iwad` or `pwad`; default `pwad`).
Lump-name or size validation failures during the write exit `3`.

### diff

Compare two WAD files lump by lump: same lump names, same count of each name,
and same data for each occurrence. Directory order of distinct lump names
does not matter; for a name that appears more than once, the sequence of
occurrences is compared in directory order. Exits `0` if identical, `1` if
any differences are found, or `2` on I/O or parse error.

```text
$ cwad diff doom.wad doom-modified.wad
Only in doom.wad:  DEMO1
Changed:           E1M1
```

### extract

Extract lumps from a WAD file into a directory (which must already exist).
Extracts every lump by default, or only the occurrences of one lump name via
`--lump`/`-l`. Each lump is written as `<SANITIZED_NAME>.bin`; when two or
more lumps sanitize to the same filename, later ones get a `_1`, `_2`, ...
suffix.

```text
$ cwad extract doom.wad --output ./out
PLAYPAL.bin
COLORMAP.bin
...
```

### build

Build a new WAD file from `NAME=FILE` lump specifications, added to the
output in the order listed.

```text
$ cwad build --output custom.wad E1M1=e1m1.lmp PLAYPAL=playpal.lmp
wrote custom.wad: kind=Pwad lumps: 2
```

Use `--kind iwad` to build an IWAD instead of the default PWAD. Lump-name or
size validation failures exit `3`.

#### Building nodes: `build --nodes`

Pass `--nodes` to rebuild, after packing, every **Doom**-format map group in
the output with classic engine-playable node lumps via the
[`add_doom_map_with_nodes`](building-nodes.md) one-shot — the BSP tree
(`SEGS`/`SSECTORS`/`NODES`), the collision `BLOCKMAP`, and the all-clear
`REJECT`:

```text
$ cwad build --nodes -o playable.wad MAP01=map01.lmp THINGS=things.lmp ...
wrote playable.wad: kind=Pwad lumps: 11
```

All of a rebuilt Doom group's classic node lumps — `SEGS`/`SSECTORS`/`NODES`,
the `REJECT` visibility table, and the `BLOCKMAP` — are overwritten with the
newly built ones, whether they were packed as empty placeholders or already
held data. **Hexen** (#352), **Doom 64** (#353), and **UDMF** (#354)
map groups are not yet supported by `--nodes` and are passed through
unchanged with a note on stderr; non-map lumps always pass through
unchanged; if no Doom map group is found, `--nodes` is a no-op and prints a
note. `--nodes` builds classic node lumps only — there is no `--node-format`
flag on `build` (unlike `convert --nodes`, which also supports `xnod`, and
`znod` when `cwad` is built with the `extended-nodes-zlib` feature). The
global `--lenient` flag applies to the node build too — a strict-mode build
failure exits `3`, and, when the error is one lenient mode can recover, hints
to re-run with `--lenient`. See [Building nodes](building-nodes.md) for the
full picture.

### convert

Convert every map in a WAD between the classic Doom binary format and UDMF,
replacing each map's lump run in place; non-map lumps, and maps already in
the target format, pass through unchanged in directory order.

```text
$ cwad convert doom.wad -o udmf.wad --to udmf
wrote udmf.wad: converted 1 map to udmf
```

`--to` is required and takes `doom` or `udmf`. Use `--map NAME` to convert
only the named map (e.g. `--map MAP01`) and pass every other map through
unchanged; omit it to convert every map in the WAD. A `--map NAME` that
matches no map in the WAD is an error (exit `3`), not a no-op. Use `--kind`
to set the output WAD kind (`iwad` or `pwad`; default `pwad`).

#### Building nodes: `--nodes`

By default, `--to doom` emits empty `SEGS`/`SSECTORS`/`NODES`/`REJECT`/`BLOCKMAP`
lumps and always prints a `NodesNotBuilt` warning to stderr — playable on the
ZDoom family (which rebuilds nodes at load) but not on vanilla ports. Pass
`--nodes` to **build** those lumps for real, so the output is engine-playable
everywhere with no external nodebuilder pass:

```text
$ cwad convert udmf.wad -o doom.wad --to doom --nodes --lenient
wrote doom.wad: converted 1 map to doom
```

`--nodes` builds the classic 16-bit node lumps via the
[`nodebuild`](building-nodes.md) pipeline (`add_doom_map_with_nodes`): the BSP
tree, the collision `BLOCKMAP`, and the all-clear `REJECT`. The
`NodesNotBuilt` warning is then gone (the nodes exist). The global `--lenient`
flag applies to the build too — it is often needed for real maps, whose
geometry can contain the engine-tolerated mixed-sector fan that strict mode
rejects (see [Building nodes](building-nodes.md#the-tolerated-mixed-sector-fan)).

`--nodes` only affects classic Doom output; combined with `--to udmf` it has no
effect (UDMF has no binary node lumps) and prints a note to stderr:

```text
$ cwad convert doom.wad -o out.wad --to udmf --nodes
note: --nodes has no effect with --to udmf (UDMF has no binary node lumps); ignoring
```

`--node-format <classic|xnod|znod>` selects the on-disk form of the nodes
`--nodes` builds; default `classic`. It has no effect without `--nodes`; a
non-`classic` value (`xnod`/`znod`) passed without `--nodes` prints a note on
stderr and is ignored. `xnod` writes an uncompressed
ZDoom extended-node stream in `NODES`; `znod` writes the zlib-compressed
form and requires `cwad` built with the `extended-nodes-zlib` feature
(on by default) — without it, `znod` exits `3` with a clear error rather than
a clap parse failure. See
[Choosing the on-disk node format](building-nodes.md#choosing-the-on-disk-node-format).

The ZDoom non-GL extended and compressed node formats (`xnod`/`znod`) are
supported via `--node-format`; GL nodes (`XGL2`/`XGL3`) remain out of scope
for `--nodes` and are deferred to a future issue — for those, still run an
external nodebuilder (`zdbsp`, `bsp`, ...) over the output. See
[Building nodes](building-nodes.md) for the full picture.

**Strict mode refuses data loss.** Converting a typical ZDoom-namespace UDMF
map (linedef `args`, thing `height`/`id`/`special`, ...) to `doom` exits `3`,
naming the offending field on stderr:

```text
$ cwad convert udmf.wad -o doom.wad --to doom
error: cannot convert map MAP01 to doom: thing #0 has a height value, which the Doom format cannot represent
note: re-run with --lenient to accept the data loss
```

This is intended, not a bug: `--to doom` succeeding is the answer to "does
this map fit in the Doom format?" Pass the global `--lenient` flag to accept
the loss and convert anyway; each dropped or rounded field is then reported
as a warning on stderr instead. See [Converting maps](converting-maps.md)
for the full loss policy.

**A converted map keeps only the lumps its target format defines.** A
converted group is rebuilt from the assembled map: the marker plus `TEXTMAP`
and `ENDMAP` (`--to udmf`), or the marker plus the classic
`THINGS`/`LINEDEFS`/`SIDEDEFS`/`VERTEXES`/`SECTORS` run and the empty node
lumps (`--to doom`). Any *other* lump that lived inside the map group —
`BEHAVIOR` (compiled ACS), `SCRIPTS`, `ZNODES`, `DIALOGUE`, GL node lumps —
is dropped. It is not passed through: compiled ACS is bound to the source
map's specials and node lumps describe the source geometry, so carrying
either into a converted map would produce something that looks intact and is
subtly broken. Dropping it is data loss, and is treated like any other:

```text
$ cwad convert hexen.wad -o udmf.wad --to udmf
error: cannot convert map MAP01 to udmf: it contains lump(s) that cannot be carried into the converted map: BEHAVIOR
note: re-run with --lenient to convert anyway and drop them
```

With `--lenient` the conversion proceeds and each dropped lump is named in a
warning on stderr. A map already in the target format is not converted, so
nothing in its group is dropped.

Exits `0` on success, `2` on I/O or parse error, `3` if a map cannot be
assembled, cannot be converted without loss in strict mode, or if `--map
NAME` matches no map in the WAD.

## Global options

| Flag | Short | Description |
|---|---|---|
| `--lenient` | — | Use lenient parsing instead of strict when reading a WAD; attempts best-effort recovery for non-fatal issues and emits warnings to stderr. For `build`, also uses lenient instead of strict validation when writing |
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

All subcommands accept the `--format` / `-F` flag, but `merge` does not
currently produce any structured stdout output — it only writes the merged
file, and warnings/errors still go to stderr regardless of format.

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
| `1` | Negative result — the two WADs differ (`diff`), or `validate --deep` found map validation errors |
| `2` | I/O error or parse error (malformed WAD, missing file, etc.); for `extract`, also a nonexistent `--output` directory or a `--lump` name not found |
| `3` | Usage error (unknown subcommand, invalid flag value, missing required argument, or a lump-name/size validation failure when writing for `build`, `merge`, or `convert` — note a non-ASCII lump name decodes under a lenient *read* but is rejected on *write* in both strictness modes); for `convert`, also a map that fails to assemble, a map that cannot be converted without loss in strict mode (including a group lump such as `BEHAVIOR` that the target format cannot carry), or a `--map NAME` that matches no map in the WAD; for `build --nodes`, also a Doom map group that fails to assemble or a node build that fails in strict mode |

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
