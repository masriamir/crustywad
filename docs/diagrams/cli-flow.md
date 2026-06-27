# CLI Flow

> **Audience:** Library users

The `cwad` binary exposes two subcommands. The `--lenient` flag is global and switches the parser to lenient mode for the entire invocation. Warnings are always written to stderr; normal output goes to stdout.

```mermaid
flowchart TD
    A["cwad [--lenient] &lt;subcommand&gt; &lt;path&gt;"]
    B{"--lenient flag?"}
    C["ParseOptions::strict()\n(default)"]
    D["ParseOptions::lenient()"]

    A --> B
    B -- "absent" --> C
    B -- "present" --> D
    C & D --> E{"subcommand"}

    E -- "info" --> F["Wad::from_path_with_options(path, opts)"]
    E -- "list" --> F

    F --> G{"parse result"}
    G -- "Err(ParseError)" --> H["stderr: error message\nexit code 1"]
    G -- "Ok(Wad)" --> I{"subcommand"}

    I -- "info" --> J["stdout: kind\nstdout: lump count"]
    I -- "list" --> K["stdout: index · filepos · size · name\n(one row per lump)"]

    J & K --> L["stderr: one line per ParseWarning\n(lenient mode; empty in strict)"]
    L --> M["exit code 0"]
```
