# CLI Flow

> **Audience:** Library users

The `cwad` binary exposes seven subcommands: `info`, `list`, `validate`, `diff`, and `extract` (read-only) plus `merge` and `build` (write-path, require the `write` feature). `--lenient` and `--format` (`-F`; `human` default, `json`, or `csv`) are global flags that apply to every subcommand. Warnings are always written to stderr; normal output routes through `--format`. Argument-parsing failures (via `clap`) exit `3` before any subcommand runs, regardless of which subcommand was given.

## Read-path dispatch

`info`, `list`, `validate`, `diff`, and `extract` all read one or more WADs via `Wad::from_path_with_options` under `ParseOptions::strict()`/`lenient()`. `validate` handles its load `Result` explicitly so it can route both outcomes through `--format`; the other read subcommands propagate load failures via `anyhow`'s `?`, which `main` catches and turns into exit `2`. `extract` additionally exits `2` if a requested `--lump NAME` isn't found in the WAD (a separate explicit check after a successful load).

```mermaid
flowchart TD
    A["cwad [--lenient] [--format FMT] <subcommand>"]
    B{"--lenient flag?"}
    C["ParseOptions::strict()\n(default)"]
    D["ParseOptions::lenient()"]
    A --> B
    B -- "absent" --> C
    B -- "present" --> D
    C & D --> E{"subcommand"}

    E -- "extract" --> X0{"output dir exists?"}
    X0 -- "no" --> ERR2A["stderr: error\nexit 2"]
    X0 -- "yes" --> LOAD

    E -- "info / list" --> LOAD["Wad::from_path_with_options(path, opts)"]
    E -- "validate" --> LOADV["Wad::from_path_with_options(path, opts)\n(Result handled explicitly, not via ?)"]
    E -- "diff" --> LOAD2["Wad::from_path_with_options\nfor file1, then file2"]

    LOAD --> R1{"Ok?"}
    R1 -- "no" --> ERR2B["stderr: error message\nexit 2 (propagated via anyhow ?)"]
    R1 -- "yes" --> S1{"subcommand"}

    LOADV --> RV{"Ok?"}
    RV -- "no" --> FMTERRV["--format routed:\nhuman -> stderr\njson/csv -> stdout ok:false\nexit 2"]
    RV -- "yes" --> FMTOKV["--format routed:\nhuman/json/csv ok:true\nexit 0"]

    LOAD2 --> R2{"both Ok?"}
    R2 -- "no" --> ERR2B
    R2 -- "yes" --> DCALC["diff lumps by name\n(per-name data-sequence comparison)"]

    S1 -- "info" --> OUT1["--format routed:\nkind, lump count, data size, maps\nexit 0"]
    S1 -- "list" --> OUT2["--format routed:\nlump directory\n(index, filepos, size, name)\nexit 0"]
    S1 -- "extract" --> OUT3{"--lump NAME given\nand not found?"}
    OUT3 -- "yes" --> ERR2C["stderr: lump not found\nexit 2"]
    OUT3 -- "no" --> OUT3B["extract matching lumps,\nsanitize filenames,\n--format routed per-file output\nexit 0"]

    DCALC --> DHAS{"any differences?"}
    DHAS -- "no" --> ZERO["exit 0, no output"]
    DHAS -- "yes" --> OUT4["--format routed:\nkind + name per difference\n(json = NDJSON)\nexit 1"]

    OUT1 & OUT2 & FMTOKV & OUT3B --> WARN["stderr: one line per ParseWarning\n(lenient mode; empty in strict)"]
```

## Write-path dispatch

`merge` and `build` construct a WAD via `WadBuilder` and require the `write` feature. `--lenient` selects `WriteOptions::strict()`/`lenient()` for the build step, distinct from (but analogous to) the read-side `ParseOptions`. `WriteError` from `build_with_options` is a usage/data error and exits `3`; I/O failures (reading input files, writing the output file) still propagate via `?` and exit `2`, mirroring the read-path exit codes.

```mermaid
flowchart TD
    A["cwad [--lenient] [--format FMT] merge|build ..."]
    B{"--lenient flag?"}
    C["WriteOptions::strict()\n(default)"]
    D["WriteOptions::lenient()"]
    A --> B
    B -- "absent" --> C
    B -- "present" --> D
    C & D --> E{"subcommand"}

    E -- "merge" --> M1["for each input path:\nWad::from_path_with_options(path, ParseOptions)\nadd_lump for every lump into WadBuilder"]
    M1 --> M2{"all inputs Ok?"}
    M2 -- "no" --> ERR2["stderr: error\nexit 2 (propagated via anyhow ?)"]
    M2 -- "yes" --> BUILD

    E -- "build" --> B1["for each NAME=FILE spec:\nsplit on '=', read file, add_lump"]
    B1 --> B2{"spec malformed or\nname/file empty?"}
    B2 -- "yes" --> ERR3A["stderr: invalid lump spec\nexit 3"]
    B2 -- "no" --> B3{"file read Ok?"}
    B3 -- "no" --> ERR2
    B3 -- "yes" --> BUILD

    BUILD["builder.build_with_options(write_opts)"]
    BUILD --> BOK{"Ok((bytes, warnings))?"}
    BOK -- "no, Err(WriteError)" --> ERR3B["stderr: build error\nexit 3 (usage/data error, not I/O)"]
    BOK -- "yes" --> WARN["stderr: one line per WriteWarning\n(lenient mode; empty in strict)"]
    WARN --> WFILE["fs::write(output, bytes)"]
    WFILE --> WOK{"Ok?"}
    WOK -- "no" --> ERR2
    WOK -- "yes" --> DONE{"subcommand"}

    DONE -- "merge" --> M_EXIT["exit 0\n(no confirmation output)"]
    DONE -- "build" --> B_EXIT["--format routed:\nhuman/csv 'wrote ... lumps: N'\njson ok:true,lumps:N\nexit 0"]
```
