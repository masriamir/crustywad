# Data Flow

> **Audience:** Contributors

## Read pipeline

`Strictness` only affects semantic validation: strict mode returns `Err(ParseError)` immediately; lenient mode pushes a `ParseWarning` and continues. Binary decode errors from `binrw` — for both the header and directory entries — are always fatal regardless of mode.

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
    I["Validate numlumps / infotableofs\n(coerce_i32: negative values → error or clamp)"]
    J{Values non-negative?}
    K["Err(ParseError::NegativeValue)"]
    L["warn ParseWarning::NegativeValue\nclamp to 0"]
    M["Compute directory span\n(numlumps x 16 bytes)"]
    MOVF{dir_span overflows?}
    F4{Strictness?}
    OVF_E["Err(ParseError::Overflow)"]
    OVF_W["warn ParseWarning::Overflow\ndir_span saturated"]
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
    M --> MOVF
    MOVF -- "yes" --> F4
    F4 -- "Strict" --> OVF_E
    F4 -- "Lenient" --> OVF_W
    OVF_W --> N
    MOVF -- "no" --> N
    N -- "yes" --> Q
    N -- "no" --> F3{Strictness?}
    F3 -- "Strict" --> O
    F3 -- "Lenient" --> P
    P --> Q
    Q --> R
    R --> S
```

## Strict vs. lenient mode

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

`parse_records::<T>` turns raw lump bytes into a typed vector using `binrw`. The generic parameter `T` may be any map record type (`Thing`, `Linedef`, `Sidedef`, `Vertex`, `Seg`, `Subsector`, `Node`, `Sector`) that implements `BinRead<Args<'_> = ()>`. An empty buffer always yields an empty `Vec`. Otherwise the function parses the first record and measures how many bytes `BinRead` consumed (`record_size = cursor.position()`); this avoids relying on `size_of::<T>()`, which reflects in-memory layout rather than on-disk size. If `record_size == 0` the type has no on-disk representation and any non-empty input is a `TrailingBytes` error. If the total length is not an exact multiple of `record_size`, the remaining partial bytes are a `TrailingBytes` error.

```mermaid
flowchart TD
    A["Input: raw lump bytes\ne.g. THINGS lump data"]
    B["Caller specifies record type T\nfor parse_records, e.g. T = Thing"]
    ZST{T is zero-sized?}
    ZST_EMPTY{bytes is empty?}
    ZST_OK["Ok, empty Vec"]
    ZST_ERR["Err(MapParseError::TrailingBytes)\noffset = 0"]
    C{exact multiple\nof record size?}
    D["Err(MapParseError::TrailingBytes)\noffset = last complete record end"]
    E["Allocate Vec\ncapacity = bytes.len() / record_size\npush first record"]
    F{more bytes\nto read?}
    G["binrw reads one T\nlittle-endian fixed-size struct"]
    H{binrw ok?}
    I["Err(MapParseError::Binrw)"]
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
        T4["Sector\n26 bytes: floor_height i16, ceiling_height i16\nfloor_texture Name8, ceiling_texture Name8\nlight_level i16, special_type i16, tag i16"]
    end

    K --> T1
    K --> T2
    K --> T3
    K --> T4
```
