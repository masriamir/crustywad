# Data Model

> **Audience:** Library users

## WAD on-disk layout

The header is always at offset 0 and is exactly 12 bytes. Lump data blobs can appear anywhere in the file; each directory entry's `filepos` and `size` fields locate the blob. The lump directory sits at the byte offset stored in `infotableofs` (typically at the end of the file). Each directory entry is exactly 16 bytes and describes one lump.

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
    Header -- "infotableofs" --> Dir
```

## Rust type relationships

The class diagram below shows the public API types in `crustywad` and how they relate to each other. Constructors return `Result<Wad, ParseError>`; in lenient mode the returned `Wad` carries zero or more `ParseWarning` values accessible via `Wad::warnings`. Methods marked `[mmap]` are only available when the `mmap` feature flag is enabled.

```mermaid
classDiagram
    class Wad {
        +from_bytes(bytes) Result~Wad, ParseError~
        +from_path(path) Result~Wad, ParseError~
        +from_path_mapped(path) Result~Wad, ParseError~ [mmap]
        +from_path_mapped_with_options(path, opts) Result~Wad, ParseError~ [mmap]
        +kind() WadKind
        +header() WadHeader
        +lump_count() usize
        +lumps() Lump[]
        +lump(index) Option~Lump~
        +lump_by_name(name) Option~Lump~
        +lump_bytes(index) Option~u8[]~
        +warnings() ParseWarning[]
        +into_bytes() Vec~u8~
    }
    class WadHeader {
        +kind WadKind
        +num_lumps usize
        +info_table_offset usize
    }
    class Lump {
        +name() str
        +filepos() usize
        +size() usize
    }
    class WadKind {
        <<enumeration>>
        Iwad
        Pwad
        Unknown([u8; 4])
    }
    class ParseOptions {
        +strictness Strictness
        +strict() ParseOptions
        +lenient() ParseOptions
    }
    class Strictness {
        <<enumeration>>
        Strict
        Lenient
    }
    class ParseError {
        <<enumeration>>
        Io
        Header
        Directory
        InvalidMagic
        NegativeValue
        OutOfBounds
        NonAsciiName
        Overflow
    }
    class ParseWarning {
        <<enumeration>>
        InvalidMagic
        NegativeValue
        OutOfBounds
        NonAsciiName
        Overflow
    }
    class MapParseError {
        <<enumeration>>
        TrailingBytes
        Binrw
    }

    Wad "1" --> "1" WadHeader : has
    Wad "1" --> "0..*" Lump : contains
    Wad "1" --> "0..*" ParseWarning : collects
    WadHeader --> WadKind : kind
    ParseOptions --> Strictness : strictness
    Wad ..> ParseOptions : constructed with
    Wad ..> ParseError : returns on failure
```
