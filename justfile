# justfile -- common recipes for the MCS-4/MCS-40 emulator workspace
# WHY: Provides single-command shortcuts for lint, test, format, and docs.
# WHAT: Recipes wrap cargo commands with consistent flags for reproducibility.
# HOW: Run `just <recipe>` from the repo root. Requires `just` (https://github.com/casey/just).

# Check types without building
check:
    cargo check --workspace --locked

# Run all tests
test:
    cargo test --workspace

# Run clippy with warnings as errors
lint:
    cargo clippy --all-targets -- -D warnings

# Check formatting (no writes)
fmt:
    cargo fmt --all -- --check

# Fix formatting (in place)
fmt-fix:
    cargo fmt --all

# Build documentation
doc:
    cargo doc --workspace --no-deps

# Run full verification: fmt, lint, test, doc
full: fmt lint test doc
    @echo "All checks passed."
