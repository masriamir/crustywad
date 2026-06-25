# Feature Flags

## mmap

**Default:** disabled

Enable memory-mapped file loading:

```toml
[dependencies]
crustywad = { version = "0.1", features = ["mmap"] }
```

This unlocks two additional constructors:

- `Wad::from_path_mapped(path)` — strict parsing
- `Wad::from_path_mapped_with_options(path, options)` — custom options

```rust
# #[cfg(feature = "mmap")]
# {
use crustywad::Wad;

let wad = Wad::from_path_mapped("doom.wad")?;
println!("{} lumps", wad.lump_count());
# Ok::<(), crustywad::ParseError>(())
# }
```

### When to use mmap

Memory-mapped loading is useful for large WADs when you only need to access a
subset of lumps. The OS maps the file into the address space without copying all
bytes into heap memory upfront — pages are faulted in on demand.

For small WADs or when you will access most lumps, `Wad::from_path` (which reads
into a `Vec<u8>`) is equally fast and has simpler lifetime semantics.

### Safety note

The mapping is held for the lifetime of the `Wad`. Truncating or writing to the
file from another process while the `Wad` is alive is unsupported and may cause
`SIGBUS` on Unix; on Windows the mapping prevents truncation but concurrent writes may produce inconsistent reads.

## freedoom-tests

**Default:** disabled

Enables optional integration tests that parse locally downloaded FreeDoom WAD
fixtures. This flag is only useful for contributors running the full test suite:

```bash
just fetch-fixtures
CRUSTYWAD_FREEDOOM_DIR=tests/fixtures/freedoom cargo test --all-features
```

The tests are skipped gracefully when `CRUSTYWAD_FREEDOOM_DIR` is not set, so
passing `--all-features` in CI is safe without fetching fixtures first.

