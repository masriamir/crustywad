# Dev Container

Provides a reproducible Rust development environment for VS Code and GitHub Codespaces.

## Base image

```
mcr.microsoft.com/devcontainers/rust:1-1.85-bullseye
```

The tag `1-1.85-bullseye` tracks Rust 1.85.x (the project MSRV) on Debian Bullseye. It is a
floating tag maintained by Microsoft and receives security patches without requiring a tag bump.
The `1-1.85` prefix ensures that minor Rust updates within the 1.85 series are picked up, but
a major upgrade (e.g., to 1.86) requires an explicit tag change here and in `devcontainer.json`.

**When to update the tag:** bump the tag here and in `devcontainer.json` when the project MSRV
is raised. Pin to a digest (`@sha256:…`) in the image field if you need fully reproducible
builds; the floating tag is intentional for this development container.

## Tools installed on creation

`postCreateCommand` installs these tools via `cargo install --locked`:

| Tool | Purpose |
|---|---|
| `just` | Task runner (`just build`, `just ci`, etc.) |
| `cargo-llvm-cov` | Code coverage (`just cov`) |
| `cargo-deny` | Dependency audit (`just deny`) |
| `cargo-lefthook` | Git hook runner (used by `postStartCommand`) |

`postStartCommand` runs `cargo lefthook install` to register the pre-commit and pre-push hooks
defined in `lefthook.yml`. This uses the Cargo subcommand provided by `cargo-lefthook`; no
standalone `lefthook` binary is required.

## VS Code extensions

| Extension | Purpose |
|---|---|
| `rust-lang.rust-analyzer` | Rust language server |
| `tamasfe.even-better-toml` | TOML support for `Cargo.toml` |
| `serayuzgur.crates` | Crate version hints in `Cargo.toml` |
| `vadimcn.vscode-lldb` | LLDB-based Rust debugger |
| `fill-labs.dependi` | Dependency management UI |
