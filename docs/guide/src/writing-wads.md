# Writing WAD Files

`WadBuilder`, behind the `write` feature flag, builds a new WAD from scratch
or re-serializes an existing one.

```toml
crustywad = { version = "0.3", features = ["write"] }
```

## Building from scratch

```rust
use crustywad::{WadBuilder, WadKind};

let bytes = WadBuilder::new(WadKind::Pwad)
    .add_lump("MAP01", b"")
    .add_lump("TEST", vec![1, 2, 3, 4])
    .build()
    .unwrap();

assert!(crustywad::Wad::from_bytes(bytes).is_ok());
```

Lumps are added in order with `add_lump(name, data)`. Name and size
validation, along with offset (`filepos`, `infotableofs`) computation, are
deferred entirely to `build()` / `build_with_options()` — callers never
supply offsets directly.

## Round-tripping an existing WAD

Use `Wad::to_builder()` to load a WAD, modify it, and re-serialize:

```rust
use crustywad::Wad;

# let mut bytes = Vec::new();
# bytes.extend_from_slice(b"IWAD");
# bytes.extend_from_slice(&1_i32.to_le_bytes());
# bytes.extend_from_slice(&16_i32.to_le_bytes());
# bytes.extend_from_slice(&[1, 2, 3, 4]);
# bytes.extend_from_slice(&12_i32.to_le_bytes());
# bytes.extend_from_slice(&4_i32.to_le_bytes());
# bytes.extend_from_slice(b"TEST\0\0\0\0");
let wad = Wad::from_bytes(bytes)?;

let mut builder = wad.to_builder();
builder.add_lump("EXTRA", b"more data");
let rebuilt = builder.build()?;

assert_eq!(Wad::from_bytes(rebuilt)?.lump_count(), 2);
# Ok::<(), Box<dyn std::error::Error>>(())
```

All lump data is copied into the builder during the conversion, so memory
usage roughly doubles for the duration.

## Writing UDMF maps

Use `write_udmf()` to serialize an assembled `Map` into a UDMF `TEXTMAP` string,
or `add_udmf_map()` to add a complete map group to a `WadBuilder`. Both are
available with the `write` feature:

```rust
use crustywad::{Wad, WadBuilder, WadKind, WriteOptions};
use crustywad::map::{Map, add_udmf_map, write_udmf};

# // Assemble a Map to write out (here from a small in-memory UDMF WAD).
# let textmap = concat!(
#     "namespace = \"doom\";\n",
#     "vertex { x = 0; y = 0; }\n",
#     "vertex { x = 8; y = 0; }\n",
#     "linedef { v1 = 0; v2 = 1; sidefront = 0; }\n",
#     "sidedef { sector = 0; }\n",
#     "sector { texturefloor = \"F\"; textureceiling = \"C\"; }\n",
# );
# let mut src = WadBuilder::new(WadKind::Pwad);
# src.add_lump("MAP01", b"");
# src.add_lump("TEXTMAP", textmap.as_bytes().to_vec());
# src.add_lump("ENDMAP", b"");
# let wad = Wad::from_bytes(src.build()?)?;
# let group = wad.map_group("MAP01").unwrap();
let map: Map = Map::assemble(&wad, &group)?;

// Serialize the map to a UDMF TEXTMAP string:
let (textmap_out, _warnings) = write_udmf(&map, &WriteOptions::strict())?;
assert!(textmap_out.starts_with("namespace"));

// Or add a complete map group (MAP01 + TEXTMAP + ENDMAP) to a builder:
let mut builder = WadBuilder::new(WadKind::Pwad);
add_udmf_map(&mut builder, "MAP01", &map, &WriteOptions::strict())?;
let bytes = builder.build()?;
# assert!(!bytes.is_empty());
# Ok::<(), Box<dyn std::error::Error>>(())
```

Fields are emitted only when they differ from UDMF spec defaults; float coordinates
are narrowed to integer form when whole (e.g. `64.0` is written as `64`). The
[Strict vs. lenient write validation](#strict-vs-lenient-write-validation) section
below covers the `WriteOptions` modes; see [Map records](map-records.md) for the
`Map` graph types these APIs consume.

## Strict vs. lenient write validation

`build()` always uses strict validation. `build_with_options()` takes a
`WriteOptions` for either mode, and returns any collected `WriteWarning`s
alongside the bytes:

```rust
use crustywad::{WadBuilder, WadKind, WriteOptions};

let (bytes, warnings) = WadBuilder::new(WadKind::Pwad)
    .add_lump("VERYLONGNAME", b"data")
    .build_with_options(&WriteOptions::lenient())
    .unwrap();

assert!(!warnings.is_empty()); // name was truncated to 8 bytes
assert!(crustywad::Wad::from_bytes(bytes).is_ok());
```

| Condition | Strict | Lenient |
|---|---|---|
| Lump name longer than 8 bytes | `WriteError::NameTooLong` | `WriteWarning::NameTruncated`, truncated to 8 bytes |
| Name contains a NUL byte | `WriteError::NulInName` | Same (both modes) |
| Non-ASCII name | `WriteError::NonAsciiName` | Same (both modes) |
| `WadKind::Unknown` magic | `WriteError::UnknownMagicStrict` | `WriteWarning::UnknownMagic`, written unchanged |
| Lump data larger than `i32::MAX` bytes | `WriteError::LumpTooLarge` | Same (both modes) |
| Lump count exceeds `i32::MAX` | `WriteError::TooManyLumps` | Same (both modes) |
| Computed offset exceeds `i32::MAX` | `WriteError::OffsetOverflow` | Same (both modes) |

## Error handling

`build()` returns `Result<Vec<u8>, WriteError>`. `build_with_options()`
returns `Result<(Vec<u8>, Vec<WriteWarning>), WriteError>` — the warnings
vector is only ever non-empty in lenient mode.

See [Data flow](data-flow.md) for the write pipeline flowchart and the
strict/lenient write mode comparison, and [Data model](data-model.md) for how
`WadBuilder` and its supporting types relate to `Wad`.

## Runnable example

`crates/crustywad/examples/write_wad.rs` runs the scenarios above end to end:

```bash
cargo run -p crustywad --example write_wad --features write
```
