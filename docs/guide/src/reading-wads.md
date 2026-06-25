# Reading WAD Files

## Constructors

`Wad` provides several constructors depending on your input source:

| Constructor | Source | Notes |
|---|---|---|
| `Wad::from_bytes(bytes)` | `Vec<u8>` / `&[u8]` / byte array | No file I/O |
| `Wad::from_bytes_with_options(bytes, opts)` | Same, with custom options | |
| `Wad::from_path(path)` | File path | Reads file into heap |
| `Wad::from_path_with_options(path, opts)` | File path + options | |
| `Wad::from_path_mapped(path)` | File path | Memory-mapped; requires `mmap` feature |
| `Wad::from_path_mapped_with_options(path, opts)` | File path + options | Requires `mmap` feature |

## Accessing lumps

```rust
use crustywad::Wad;

// Build a minimal IWAD with one lump for illustration.
let mut bytes = Vec::new();
bytes.extend_from_slice(b"IWAD");
bytes.extend_from_slice(&1_i32.to_le_bytes());   // numlumps = 1
bytes.extend_from_slice(&16_i32.to_le_bytes());  // infotableofs = 16
bytes.extend_from_slice(&[1, 2, 3, 4]);           // lump data at offset 12
bytes.extend_from_slice(&12_i32.to_le_bytes());  // directory: filepos = 12
bytes.extend_from_slice(&4_i32.to_le_bytes());   // directory: size = 4
bytes.extend_from_slice(b"TEST\0\0\0\0");         // directory: name

let wad = Wad::from_bytes(bytes)?;

// By index.
if let Some(lump) = wad.lump(0) {
    println!("lump[0]: {} ({} bytes at {})", lump.name(), lump.size(), lump.filepos());
}

// By name.
if let Some(lump) = wad.lump_by_name("TEST") {
    let data: &[u8] = wad.lump_data(lump);
    println!("TEST lump is {} bytes", data.len());
}

// Iterate all lumps.
for lump in wad.lumps() {
    println!("{}: {} bytes", lump.name(), lump.size());
}

// Raw bytes by index (returns None if index is out of range).
if let Some(bytes) = wad.lump_bytes(0) {
    println!("raw bytes: {:?}", bytes);
}
# Ok::<(), crustywad::ParseError>(())
```

## Strict vs. lenient parsing

By default `Wad::from_bytes` and `Wad::from_path` use **strict** mode: the first
validation error stops parsing and returns a `ParseError`.

Use **lenient** mode when you want best-effort recovery from malformed files:

```rust
use crustywad::{ParseOptions, Wad};

let options = ParseOptions::lenient();
let result = Wad::from_path_with_options("questionable.wad", options)?;

// Inspect any non-fatal issues collected during parsing.
for warning in result.warnings() {
    eprintln!("warning: {warning}");
}
# Ok::<(), crustywad::ParseError>(())
```

You can also use the shorthand constructors:

```rust
use crustywad::ParseOptions;

let strict  = ParseOptions::strict();   // same as default
let lenient = ParseOptions::lenient();
```

### What each mode does

| Condition | Strict | Lenient |
|---|---|---|
| Invalid magic bytes | `ParseError` | `ParseWarning`, `WadKind::Unknown` |
| Negative `numlumps` or `infotableofs` | `ParseError` | `ParseWarning`, value clamped to 0 |
| Directory extends past end-of-file | `ParseError` | `ParseWarning`, truncated to available entries |
| Lump data out of bounds | `ParseError` | `ParseWarning`, range clamped |
| Non-ASCII lump name | `ParseError` | `ParseWarning`, lossy UTF-8 decoding |

## WAD kind

```rust
use crustywad::{Wad, WadKind};

# let mut bytes = Vec::new();
# bytes.extend_from_slice(b"IWAD");
# bytes.extend_from_slice(&1_i32.to_le_bytes());
# bytes.extend_from_slice(&16_i32.to_le_bytes());
# bytes.extend_from_slice(&[1, 2, 3, 4]);
# bytes.extend_from_slice(&12_i32.to_le_bytes());
# bytes.extend_from_slice(&4_i32.to_le_bytes());
# bytes.extend_from_slice(b"TEST\0\0\0\0");
let wad = Wad::from_bytes(bytes)?;

match wad.kind() {
    WadKind::Iwad => println!("IWAD — base game data"),
    WadKind::Pwad => println!("PWAD — patch or add-on"),
    WadKind::Unknown(magic) => println!("Unknown magic: {:?}", magic),
}
# Ok::<(), crustywad::ParseError>(())
```
