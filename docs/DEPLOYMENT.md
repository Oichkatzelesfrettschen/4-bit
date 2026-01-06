# Deployment and Operations

## Build
- Debug: `cargo build --workspace --locked`
- Release: `cargo build --workspace --release --locked`
- Clean artifacts: `scripts/clean.sh`

## Run
- GUI: `cargo run -p mcs4-gui -- --rom path/to/rom.bin`

## Logs
- Set `RUST_LOG=info`.

## Artifacts
- Default build output: `target/` (workspace root).
- Coverage output: `coverage/` (see `.cargo/config.toml` if enabled).
