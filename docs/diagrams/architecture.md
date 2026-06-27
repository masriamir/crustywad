# Architecture

> **Audience:** Library users and contributors

## Workspace layout

The workspace contains two crates. `crustywad-cli` depends on `crustywad`; the library has no dependency on the CLI.

```mermaid
graph TD
    subgraph lib["crustywad  (library crate)"]
        lrs["lib.rs\nWad · WadHeader · Lump\nParseOptions · Strictness"]
        ers["error.rs\nParseError · ParseWarning"]
        mrs["map.rs\nThing · Linedef · Sidedef · Vertex\nSeg · Subsector · Node · Sector"]
        mmrs["mmap.rs\n(feature: mmap only)"]
    end
    subgraph cli["crustywad-cli  (binary crate)"]
        mains["main.rs\ncwad — info · list subcommands"]
    end
    cli -->|"cargo dependency"| lib
    mmrs -. "feature = mmap\nadds memmap2 dependency" .-> memmap2(["memmap2\n(external crate)"])
```

## Feature flags

```mermaid
graph LR
    lib["crustywad"]
    lib -. "mmap" .-> mmap["Wad::from_path_mapped\nWad::from_path_mapped_with_options\nzero-copy loading via memmap2"]
    lib -. "freedoom-tests" .-> ft["integration tests against\nlocal Freedoom WAD fixtures\n(test-only, no runtime dependency)"]
```
