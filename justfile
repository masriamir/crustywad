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

# Compile-check the guide's Rust code samples. The pages are pulled into the crate as
# doctests (src/guide_doctests.rs, gated on doctest + the guide-doctests feature +
# build.rs's has_guide_sources cfg; --all-features enables the feature) and compiled with
# every feature enabled, so API drift in a snippet fails CI. `mdbook test` cannot do this:
# it only passes `-L` to rustdoc, never `--extern`, and has no way to enable Cargo features.
guide-test:
    cargo test -p crustywad --doc --all-features

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
# The default must be absolute: cargo runs the test binary with its CWD at the
# package root (crates/crustywad), so a workspace-relative path never resolves and
# the fixture tests skip silently instead of failing.
test-freedoom dir=(justfile_directory() / "tests/fixtures/freedoom"):
    CRUSTYWAD_FREEDOOM_DIR="{{dir}}" cargo test -p crustywad --features freedoom-tests,write

# Sweep a local retail-WAD collection: assemble every map of every WAD in both
# strictness modes (#254). Nothing is fetched — supply your own WADs; the repo's
# gitignored RETAIL/ directory is the default. Use an absolute dir (cargo runs
# the test binary with its CWD at the package root, so a relative path resolves
# against crates/crustywad and a missed directory only prints a stderr skip note).
test-sweep dir=(justfile_directory() / "RETAIL") extdir=(justfile_directory() / "RETAIL-EXT"):
    CRUSTYWAD_SWEEP_DIR="{{dir}}" CRUSTYWAD_SWEEP_EXTENDED_DIR="{{extdir}}" cargo test -p crustywad --features sweep-tests,extended-nodes-zlib --test sweep -- --nocapture

# Run a fuzz target. The fuzz/ sub-workspace pins nightly via rust-toolchain.toml.
fuzz target="fuzz_wad_strict":
    cd fuzz && cargo fuzz run {{target}}

# Run all benchmarks with all features enabled.
bench:
    cargo bench --all-features --benches
    @echo "Criterion HTML report: target/criterion/report/index.html"

# Run benchmarks then open the HTML report in the default browser.
bench-open:
    cargo bench --all-features --benches
    {{ if os() == "macos" { "open" } else if os_family() == "windows" { "explorer" } else { "xdg-open" } }} target/criterion/report/index.html

# Doc drift checks: living-doc anchor strings present in all three doc files
# (ADR-0007), and every documented `crustywad = "X.Y.Z"` pin still resolving to
# the crate's actual version (after a minor bump a stale 0.x pin does not resolve
# for readers at all — see #235). Patch bumps still resolve and need no change.
docs-sync:
    python3 scripts/check_doc_anchors.py
    python3 scripts/check_doc_versions.py

# Pre-push gate (fail-fast): cheapest checks first so failures surface in
# seconds, not after a long compile. Omits `build` (subsumed by `test`; CI has
# no standalone build job either) and `deny` (a dependency-graph audit whose
# outcome code edits never change). Run `ci-full` before releases and on
# branches that touch Cargo.toml/Cargo.lock.
ci: docs-sync lint test doc

# The pre-push gate plus the workspace build and dependency audit — full
# parity with every recipe this justfile can mirror from CI.
ci-full: build test lint doc deny docs-sync
