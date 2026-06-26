build:
    cargo build --workspace --all-features

test:
    cargo test --workspace --all-features

lint:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets --all-features -- -D warnings

fmt:
    cargo fmt --all

doc:
    cargo doc --workspace --all-features --no-deps

cov:
    cargo llvm-cov --workspace --all-features --lcov --output-path lcov.info

deny:
    cargo deny check

# Download Freedoom fixtures. Override the release with e.g. `just fetch-fixtures version=v0.14.0`.
fetch-fixtures version="":
    python tests/fixtures/fetch_freedoom.py {{ if version != "" { "--version " + version } else { "" } }}

# Build the core library with the mmap feature enabled.
build-mmap:
    cargo build -p crustywad --features mmap

# Test the core library with the mmap feature enabled.
test-mmap:
    cargo test -p crustywad --features mmap

# Run Freedoom fixture tests. Requires CRUSTYWAD_FREEDOOM_DIR to be set and fixtures fetched.
test-freedoom:
    cargo test --workspace --all-features

fuzz:
    @echo "Fuzz targets are planned for a later milestone; see docs/design.md."

bench:
    cargo bench

ci: build test lint doc deny
