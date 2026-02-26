# Deployment and Operations

## Build

```sh
# Debug build (faster compilation, debug info)
cargo build --workspace --locked

# Release build (optimized, stripped)
cargo build --workspace --release --locked

# Zen3-optimized build (AMD Zen 3 CPUs)
RUSTFLAGS="-C target-cpu=znver3" cargo build --workspace --profile zen3-release

# Clean all build artifacts
cargo clean
```

## Run

### GUI emulator
```sh
cargo run -p mcs4-gui -- --rom path/to/rom.bin
```

### Fixture runner (no GUI)
```sh
cargo run -p mcs4-gui -- --mode fixture --system mcs4 --fixture src_wrm_rdm --cycles 12
```

With ROM port input for RDR fixtures:
```sh
cargo run -p mcs4-gui -- --mode fixture --system mcs4 --fixture rdr_test \
  --rom-io-input 12 --rom-io-chip 0
```

## Testing

```sh
# Full test suite
cargo test --workspace

# Single crate
cargo test -p mcs4-chips

# With SIMD feature (requires nightly)
cargo test -p mcs4-chips --features simd

# Coverage report (requires cargo-llvm-cov)
cargo llvm-cov --workspace --all-features --lcov --output-path coverage/lcov.info
```

## Linting and Formatting

```sh
# Format check
cargo fmt --all -- --check

# Clippy (warnings as errors)
cargo clippy --all-targets -- -D warnings

# Full lint pass (alias)
cargo clippy-all
```

## Benchmarks

```sh
# Standard benchmarks
cargo bench --workspace --locked

# With AVX2 optimization
RUSTFLAGS="-C target-cpu=x86-64-v3" cargo bench --workspace

# Performance regression check (Python)
python3 scripts/benchmark_emulator_v0.py \
  --output docs/evidence/benchmarks_ci.json \
  --baseline docs/evidence/benchmarks_baseline_v0.json \
  --fail-on-regression
```

## Logs

Set `RUST_LOG` for tracing output:
```sh
RUST_LOG=info cargo run -p mcs4-gui
RUST_LOG=mcs4_core=debug cargo test -p mcs4-core
```

## Artifacts

| Path | Description |
|------|-------------|
| `target/` | Build output (workspace root) |
| `coverage/` | Coverage reports (lcov) |
| `docs/evidence/` | OCR, benchmarks, extraction artifacts |
| `mcs4-emu/crates/mcs4-system/fixtures/` | ROM test fixtures (.hex) |

## CI

CI runs on every push to `main` and on pull requests. Jobs:
- **build-test**: format check, clippy, full test suite
- **coverage**: generates lcov report
- **bench**: runs benchmarks with regression detection
