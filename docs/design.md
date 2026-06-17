# Design

## Goals

- Provide a small, safe Rust library for Doom WAD file I/O.
- Start with reliable header and lump-directory reading.
- Keep the API ready for future write support, async I/O, zero-copy parsing, and optional memory mapping.

## Non-goals

- Full map graph assembly in this milestone.
- Write support in this milestone.
- Async runtime integration in this milestone.

## Data model

A WAD contains a 12-byte header, lump data blobs, and a directory of 16-byte lump entries. The header stores the byte offset at which the directory begins; in practice the directory sits after all lump data at the end of the file. `crustywad` models the file as owned bytes plus validated metadata for the parsed header and lump directory.

### WAD File Format

The diagram below shows the on-disk layout of a WAD file. The header is always at offset 0 and is exactly 12 bytes. Lump data blobs can appear anywhere in the file; each directory entry's `filepos` and `size` fields locate the blob. The lump directory sits at the byte offset stored in `infotableofs` (typically at the end of the file). Each directory entry is exactly 16 bytes and describes one lump.

```mermaid
flowchart TD
    subgraph Header["Header - 12 bytes at offset 0"]
        magic["magic\n4 bytes\n'IWAD' or 'PWAD'"]
        numlumps["numlumps\n4 bytes i32\nlump count"]
        infotableofs["infotableofs\n4 bytes i32\ndirectory offset"]
    end
    subgraph Data["Lump Data Blobs (variable)"]
        lump0["lump 0 data\n(variable)"]
        lump1["lump 1 data\n(variable)"]
        lumpN["... lump N data\n(variable)"]
    end
    subgraph Dir["Lump Directory - N x 16 bytes at infotableofs"]
        entry0["entry 0\nfilepos(4) + size(4) + name(8)"]
        entry1["entry 1\nfilepos(4) + size(4) + name(8)"]
        entryN["... entry N-1\nfilepos(4) + size(4) + name(8)"]
    end
    Header --> Data
    Data --> Dir
```

## Read pipeline

1. Read the file into owned bytes.
2. Parse the header with `binrw` using little-endian integers.
3. Validate or recover header fields according to `ParseOptions`.
4. Parse the lump directory.
5. Clamp invalid lump ranges in lenient mode and collect warnings.

### Read Pipeline Flowchart

The flowchart below traces how bytes become a `Wad`. `Strictness` only affects semantic validation: strict mode returns `Err(ParseError)` immediately; lenient mode pushes a `ParseWarning` and continues. Binary decode errors from `binrw` — for both the header and directory entries — are always fatal regardless of mode.

```mermaid
flowchart TD
    A["Input bytes\n(from_bytes / from_path / from_path_mapped)"]
    B["binrw reads RawHeader\n(12 bytes, little-endian)"]
    C{Header OK?}
    D["Err(ParseError::Header)"]
    E{Magic valid?\n'IWAD' / 'PWAD'}
    F{Strictness?}
    G["Err(ParseError::InvalidMagic)"]
    H["warn ParseWarning::InvalidMagic\nkind = WadKind::Unknown"]
    I["Validate numlumps / infotableofs\n(coerce_i32: negative values invalid)"]
    J{Values non-negative?}
    K["Err(ParseError::NegativeValue)"]
    L["warn ParseWarning::NegativeValue\nclamp to 0"]
    M["Compute directory span\n(numlumps x 16 bytes)"]
    N{Directory within buffer?}
    O["Err(ParseError::OutOfBounds)"]
    P["warn ParseWarning::OutOfBounds\ntruncate to available entries"]
    Q["Parse N x RawDirectoryEntry\n(16 bytes each, little-endian)"]
    R["validate_entry: check filepos/size/name\nlump-directory overlap\nper-entry strict/lenient branch"]
    S["Ok(Wad)\n+ warnings (may be empty)"]

    A --> B
    B --> C
    C -- "binrw error" --> D
    C -- "ok" --> E
    E -- "yes" --> I
    E -- "no" --> F
    F -- "Strict" --> G
    F -- "Lenient" --> H
    H --> I
    I --> J
    J -- "yes" --> M
    J -- "no" --> F2{Strictness?}
    F2 -- "Strict" --> K
    F2 -- "Lenient" --> L
    L --> M
    M --> N
    N -- "yes" --> Q
    N -- "no" --> F3{Strictness?}
    F3 -- "Strict" --> O
    F3 -- "Lenient" --> P
    P --> Q
    Q --> R
    R --> S
```

## Strict vs. lenient parsing

`Strictness::Strict` treats malformed magic, negative counts, out-of-range offsets, oversized lumps, and non-ASCII names as hard errors.

`Strictness::Lenient` keeps parsing when possible, returning a `Wad` plus collected warnings. In lenient mode, invalid directory sizes are truncated to the number of complete entries that fit in the buffer and invalid lump byte ranges are clamped into a safe slice.

### Strict vs. Lenient Mode Comparison

The sequence diagram below shows how the same malformed WAD (bad magic bytes) flows through each mode. Strict mode returns an error immediately; lenient mode records a warning and proceeds to produce a usable `Wad`.

```mermaid
sequenceDiagram
    participant Caller
    participant Parser
    participant Warnings

    Note over Caller,Warnings: Input: WAD bytes with magic = XWAD (not IWAD/PWAD)

    rect rgb(255, 230, 230)
        Note over Caller,Parser: Strict mode (ParseOptions::strict())
        Caller->>Parser: Wad::from_bytes_with_options(bytes, ParseOptions::strict())
        Parser->>Parser: read RawHeader, magic = XWAD
        Parser->>Parser: magic != IWAD/PWAD, Strictness::Strict
        Parser-->>Caller: Err(ParseError::InvalidMagic)
    end

    rect rgb(230, 255, 230)
        Note over Caller,Warnings: Lenient mode (ParseOptions::lenient())
        Caller->>Parser: Wad::from_bytes_with_options(bytes, ParseOptions::lenient())
        Parser->>Parser: read RawHeader, magic = XWAD
        Parser->>Parser: magic != IWAD/PWAD, Strictness::Lenient
        Parser->>Warnings: push ParseWarning::InvalidMagic
        Parser->>Parser: kind = WadKind::Unknown
        Parser->>Parser: continue parsing numlumps, infotableofs, directory
        Parser-->>Caller: Ok(Wad) with warnings
        Caller->>Caller: wad.warnings() includes InvalidMagic
    end
```

## Map record parsing

### Map Record Parsing Flowchart

`parse_records::<T>` turns raw lump bytes into a typed vector using `binrw`. The generic parameter `T` may be any map record type (`Thing`, `Linedef`, `Sidedef`, `Vertex`, `Seg`, `Subsector`, `Node`, `Sector`) that implements `BinRead<Args<'_> = ()>`. Zero-sized types (`size_of::<T>() == 0`) are handled as a special case before the modulo check: an empty buffer yields an empty `Vec`, and a non-empty buffer is an unconditional `TrailingBytes` error. For all other types, records are read sequentially until the cursor reaches the end of the slice.

```mermaid
flowchart TD
    A["Input: raw lump bytes\ne.g. THINGS lump data"]
    B["Caller specifies record type T\nfor parse_records, e.g. T = Thing"]
    ZST{T is zero-sized?}
    ZST_EMPTY{bytes is empty?}
    ZST_OK["Ok, empty Vec"]
    ZST_ERR["Err, TrailingBytes\noffset = 0"]
    C{exact multiple\nof record size?}
    D["Err, TrailingBytes\noffset = last complete record end"]
    E["Allocate Vec\ncapacity = bytes.len() / size_of T"]
    F{more bytes\nto read?}
    G["binrw reads one T\nlittle-endian fixed-size struct"]
    H{binrw ok?}
    I["Err, MapParseError::Binrw"]
    J["push T into Vec"]
    K["Ok, Vec of T\ne.g. Vec of Thing or Vec of Linedef"]

    A --> B
    B --> ZST
    ZST -- "yes" --> ZST_EMPTY
    ZST_EMPTY -- "yes" --> ZST_OK
    ZST_EMPTY -- "no" --> ZST_ERR
    ZST -- "no" --> C
    C -- "no" --> D
    C -- "yes" --> E
    E --> F
    F -- "yes" --> G
    G --> H
    H -- "error" --> I
    H -- "ok" --> J
    J --> F
    F -- "no" --> K

    subgraph examples["Concrete T examples"]
        T1["Thing\n10 bytes: x i16, y i16, angle u16\ntype_id u16, flags u16"]
        T2["Linedef\n14 bytes: 7 x u16"]
        T3["Vertex\n4 bytes: x i16, y i16"]
        T4["Sector\n26 bytes: heights i16, textures Name8"]
    end

    K --> T1
    K --> T2
    K --> T3
    K --> T4
```

## Feature plan

- `mmap`: enables `Wad::from_path_mapped[_with_options]` for read-only memory-mapped file loading via `memmap2`; `from_path` always reads into memory regardless of this flag.
- `freedoom-tests`: optional integration tests that inspect downloaded FreeDoom fixtures.
- Future `async`: alternate I/O constructors without changing the in-memory parse model.
- Future zero-copy: borrowed views over validated bytes.

## Milestones

1. Header and directory parsing
2. Map lump record parsing
3. Graphics and patches
4. Texture composition
5. Audio lumps
6. Writing support

## Testing strategy

- Synthetic WAD builders for offline unit and integration tests.
- Optional FreeDoom fixture coverage for real-world inputs.
- `proptest` for parser invariants.
- Future fuzzing and criterion benchmarks once the API surface expands.
