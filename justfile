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

# Build the mdBook user guide. Requires mdbook and mdbook-mermaid; see tools/Cargo.toml for
# pinned versions. mdbook-mermaid install generates mermaid.min.js / mermaid-init.js (gitignored).
guide:
    mdbook-mermaid install docs/guide
    mdbook build docs/guide

cov:
    cargo llvm-cov --workspace --all-features --lcov --output-path lcov.info

deny:
    cargo deny check

# Download Freedoom fixtures. Override the release with e.g. `just fetch-fixtures version=v0.14.0`.
fetch-fixtures version="":
    python3 tests/fixtures/fetch_freedoom.py {{ if version != "" { "--version " + version } else { "" } }}

# Test the full workspace with all features enabled (alias for discoverability).
test-all-features: test

# Build the core library with the mmap feature enabled.
build-mmap:
    cargo build -p crustywad --features mmap

# Test the core library with the mmap feature enabled.
test-mmap:
    cargo test -p crustywad --features mmap

# Run Freedoom fixture tests. Tests skip gracefully when fixtures are missing.
# Fetch fixtures first with `just fetch-fixtures`, then override the directory if needed:
# just test-freedoom dir=/path/to/freedoom
test-freedoom dir="tests/fixtures/freedoom":
    CRUSTYWAD_FREEDOOM_DIR="{{dir}}" cargo test -p crustywad --features freedoom-tests

fuzz:
    @echo "Fuzz targets are planned for a later milestone; see docs/design.md."

bench:
    cargo bench

# Check that living-doc anchor strings are present in all three doc files (ADR-0007).
docs-sync:
    python3 scripts/check_doc_anchors.py

ci: build test lint doc deny docs-sync
