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

fetch-fixtures:
    python tests/fixtures/fetch_freedoom.py

fuzz:
    @echo "Fuzz targets are planned for a later milestone; see docs/design.md."

bench:
    cargo bench

ci: build test lint doc deny
