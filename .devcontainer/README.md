# Dev Container

Provides a consistent Rust development environment for VS Code and GitHub Codespaces.

## Base image

```
mcr.microsoft.com/devcontainers/rust:1-bookworm
```

`1-bookworm` is a floating tag (devcontainer image v1 on Debian 12 "Bookworm"), maintained by
Microsoft, that receives security and toolchain updates without a tag bump. What the tag encodes
is the **devcontainer image version and the Debian base** — it does not select a specific Rust
minor. The image ships a recent stable Rust toolchain that tracks ahead of the project MSRV, so it
satisfies the rolling N-3 floor without any pin.

**Rust toolchain:** the image ships a recent stable Rust that should sit comfortably above the
project's rolling N-3 MSRV floor — but because `1-bookworm` is a floating tag, treat that as an
expectation, not a hard guarantee. For a strictly pinned or reproducible toolchain, add a
`rust-toolchain.toml` or the `ghcr.io/devcontainers/features/rust` feature — the image tag itself
does not select a Rust version.

**When to update the tag:** only to change the Debian base (e.g. a future `1-trixie`) or the
devcontainer image major version — **not** when the MSRV is raised, since the tag carries no Rust
version. Pin to a digest (`@sha256:…`) in the image field if you need fully reproducible builds.

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
