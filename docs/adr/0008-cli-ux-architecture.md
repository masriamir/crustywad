# ADR-0008: `cwad` CLI UX and architecture

- **Status:** Accepted
- **Date:** 2026-06-14
- **Deciders:** @masriamir
- **Tracking issue:** https://github.com/masriamir/crustywad/issues/34

## Context

`cwad` currently exposes two subcommands (`info`, `list`) built with `clap` and `anyhow`.
Planned additions — `validate`, `diff`, `merge`, `extract`, and further inspection commands —
make it necessary to establish consistent UX conventions before more subcommands are written,
so that every command feels like part of the same tool and machine-readable output can be
plumbed into scripts without breakage.

Five areas need explicit decisions:

1. Output format strategy
2. Exit code conventions
3. Error output routing
4. Subcommand grouping / hierarchy
5. Shell-completion and man-page packaging

## Decision

### 1. Output format strategy

Every subcommand that emits structured data (`info`, `list`, `validate`, `diff`)
must accept a global `--format <FORMAT>` flag (shorthand `-F`) with three values:

| Value   | Description                                           |
|---------|-------------------------------------------------------|
| `human` | Human-readable, column-aligned text (default)         |
| `json`  | Newline-delimited JSON (one object per logical record) |
| `csv`   | RFC 4180 CSV with a header row                        |

`human` is the default so existing `cwad info` / `cwad list` output is unchanged.
`json` and `csv` are designed for piping into `jq`, spreadsheet imports, or CI scripts.

Subcommands that write to a file and only confirm a result (`merge`) emit a single-line
human message to stdout when `--format human` or `--format csv` is used (no tabular data
exists to render as CSV), and a machine-readable `{"ok": true}` /
`{"ok": false, "error": "..."}` envelope when `--format json` is used.

`extract` is a special case: when writing to stdout (`-` or no output path), only the raw
lump bytes are written to stdout — no confirmation text, to avoid corrupting the stream.
The `--format` flag is ignored in this mode; specifying it does not alter stdout output.
Any progress or error messages go to stderr. When writing to a file, `extract` uses the
same confirmation format as `merge` and respects `--format` normally.

Implementation: a `Format` enum is added to the shared CLI layer, parsed once by `clap`,
and passed as a parameter to every output helper. No third-party serialization crate is
pulled in; JSON is hand-formatted for the flat record structures involved. All string
values in hand-formatted JSON must properly escape backslashes, double quotes, and control
characters (U+0000–U+001F) to produce valid JSON even for edge-case lump names. If
`serde`/`serde_json` are needed later an ADR amendment is required first.

### 2. Exit code conventions

| Code | Meaning                                                          |
|------|------------------------------------------------------------------|
| `0`  | Success — all requested operations completed without error       |
| `1`  | Negative result — validation errors found, WADs differ, etc.    |
| `2`  | I/O or parse error — file not found, unreadable or malformed WAD|
| `3`  | Usage error — bad arguments                                      |

`anyhow::Result<()>` in `main` maps to exit code `1` for any error today. The implementation
must switch to `std::process::exit` (or a custom `run()` function returning `ExitCode`) so
that I/O errors and logical failures are distinguishable by callers.

Note: clap's built-in default for parse errors is exit code `2`, not `3`. To avoid
colliding with the I/O-error code, the implementation must intercept clap parse failures
via `Cli::try_parse()`. On error, check `clap::Error::kind()`: `DisplayHelp` and
`DisplayVersion` should call `err.exit()` (which prints and exits `0`); all other error
kinds indicate bad arguments and should call `err.print()` (writes the diagnostic to
stderr) then `std::process::exit(3)`.

`validate` exits `1` when validation errors are found and `0` when the WAD is clean.
`diff` exits `1` when the two WADs differ and `0` when they are identical (matching `diff(1)`
conventions).

### 3. Error output

All diagnostic output (errors, warnings) goes to **stderr**. All result output (records,
summaries, exit messages) goes to **stdout**. This is the Unix convention and must hold for
every subcommand including future ones.

`anyhow` with the `?` operator is kept as the error propagation mechanism in `main.rs`
because it is already the approved dependency for the CLI crate (ADR-0002). The top-level
handler catches `anyhow::Error`, writes `"error: {err:#}"` to stderr, then calls
`std::process::exit` with the appropriate code from decision 2.

Warnings collected by `Wad::warnings()` in lenient mode are printed to stderr as
`"warning: {w}"` after the successful result, mirroring the existing behavior.

No color or ANSI escape codes are added at this stage; a `--color=auto|always|never` flag
can be addressed in a future ADR once a color crate is evaluated.

### 4. Subcommand grouping

Use a **flat namespace**: `cwad <verb> [args]`.

Rationale: the total number of planned subcommands (≤10 in milestone 2) does not warrant an
extra grouping level. Grouped namespaces such as `cwad wad diff` add typing overhead and are
harder to tab-complete without shell integration that is not yet in place. The flat layout
matches tools at a similar stage of development (`wad-tools`, `omgifol`).

Reserved flat verbs for the next milestones:

| Verb       | Purpose                                               |
|------------|-------------------------------------------------------|
| `info`     | Print WAD header (exists)                             |
| `list`     | Print lump directory (exists)                         |
| `validate` | Check WAD and lump consistency                        |
| `diff`     | Compare lump directories of two WADs                  |
| `merge`    | Combine lumps from multiple WADs into one output WAD  |
| `extract`  | Write a named lump to stdout or a file                |
| `dump`     | Hex/binary dump of a lump (debugging aid)             |

If a future milestone introduces a large orthogonal domain (e.g., texture atlas management
with five sub-operations) a grouped namespace can be adopted at that point via a new ADR.

### 5. Packaging: shell completions and man pages

Shell completions are generated at **build time** using `clap_complete` via a `build.rs`
script added to `crustywad-cli`. Completions for `bash`, `zsh`, and `fish` are written to
`$OUT_DIR/completions/`. They are not embedded in the binary.

A `just completions` recipe installs them to `~/.local/share/bash-completion/completions/`,
`~/.zfunc/`, and `~/.config/fish/completions/` respectively on developer machines. CI does
not test completion installation; only that `build.rs` runs without error.

Man pages via `clap_mangen` are deferred until the subcommand surface stabilizes (after
milestone 2). Generating and vendoring man pages before the CLI is stable would create
churn without user benefit. A `just man` recipe and `OUT_DIR/man/` placement are reserved
for when this is adopted.

`clap_complete` is added as a `build-dependency` (not a runtime dependency), so it is not
shipped in the binary and does not affect its size. It is compiled by anyone building from
source, so its minimum supported Rust version must not exceed 1.85.0. `clap_mangen` is
deferred along with man-page generation and is not added until that work begins.

## Consequences

- Every new subcommand must implement the `--format` flag and route output through the
  shared format helper before merging.
- The exit-code convention is a public API contract; breaking changes to codes 0–3 require
  a semver-major bump of `crustywad-cli`. Because `crustywad-cli` currently uses
  `version.workspace = true`, this bump is shared with the library crate; decoupling the
  versions would be a prerequisite for independent CLI releases.
- The flat subcommand namespace reserves the verbs listed above; adding a conflicting verb
  requires updating this ADR.
- `build.rs` is added to `crustywad-cli`, which is currently build-script-free; CI must
  continue to pass after the addition.
- Issues #35–#40 implementing individual subcommands are blocked on this ADR being accepted.
