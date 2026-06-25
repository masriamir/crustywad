# Getting Started

## Adding crustywad to your project

Add the crate to your `Cargo.toml`:

```toml
[dependencies]
crustywad = "0.1"
```

Enable optional features as needed:

```toml
[dependencies]
crustywad = { version = "0.1", features = ["mmap"] }
```

## Basic usage

Parse a WAD from an in-memory byte slice:

```rust
use crustywad::Wad;

// Minimal valid IWAD with zero lumps.
let bytes: &[u8] = &[
    b'I', b'W', b'A', b'D',  // magic
    0, 0, 0, 0,               // numlumps = 0
    12, 0, 0, 0,              // infotableofs = 12
];

let wad = Wad::from_bytes(bytes)?;
println!("kind:  {:?}", wad.kind());
println!("lumps: {}", wad.lump_count());
# Ok::<(), crustywad::ParseError>(())
```

## Loading from a file

```rust
use crustywad::Wad;

let wad = Wad::from_path("doom.wad")?;
println!("{} lumps", wad.lump_count());
# Ok::<(), crustywad::ParseError>(())
```

## Handling errors

All parse functions return `Result<Wad, ParseError>`. The `ParseError` type covers
I/O failures, invalid magic bytes, negative field values, out-of-bounds directory
offsets, and non-ASCII lump names. See `crustywad::ParseError` in the API docs for
the full variant list.

## What's next?

- [Reading WAD Files](reading-wads.md) — access lumps and choose a parse mode.
