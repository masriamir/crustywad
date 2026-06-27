# Diagrams

Mermaid diagrams for the `crustywad` project, organized by audience.

## For library users

| Diagram | Description |
|---|---|
| [Architecture](architecture.md) | Workspace structure, crate layout, and feature flags |
| [Data model](data-model.md) | WAD on-disk layout and public API type relationships |
| [CLI flow](cli-flow.md) | `cwad` subcommand dispatch, output routing, and exit codes |

## For contributors

| Diagram | Description |
|---|---|
| [Data flow](data-flow.md) | Parse pipeline — how raw bytes become a `Wad` |
| [Lump hierarchy](lump-hierarchy.md) | Lump type taxonomy — map lumps, namespace markers, and special lumps |
