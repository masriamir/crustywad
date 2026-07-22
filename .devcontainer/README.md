# Dev Container

Provides a consistent Rust development environment for VS Code and GitHub Codespaces.

## Base image

```
mcr.microsoft.com/devcontainers/rust:1-bookworm
```

The `devcontainers/rust` image is **not tagged by Rust version** — its tags track the
devcontainer image version and the Debian base, not a Rust toolchain. `1-bookworm` is a floating
tag (devcontainer image v1 on Debian 12 "Bookworm"), maintained by Microsoft, that receives
security and toolchain updates without a tag bump. It ships a current stable Rust toolchain
(1.96.1 at the time of writing), comfortably above the project MSRV (1.94.0); the image tag does
not select a specific Rust minor.

**MSRV guarantee:** the image's stable Rust tracks ahead of the project's rolling N-3 MSRV floor,
so the container builds the workspace without any Rust pin. If you need an *exact* toolchain, add
a `rust-toolchain.toml` or the `ghcr.io/devcontainers/features/rust` feature — the image tag
cannot do it.

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
