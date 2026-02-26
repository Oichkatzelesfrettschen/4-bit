# Contributing

## Setup

1. Clone the repository and ensure you have the nightly Rust toolchain:
   ```sh
   git clone https://github.com/Oichkatzelesfrettschen/4-bit.git
   cd 4-bit
   rustup show  # should pick up rust-toolchain.toml automatically
   ```
2. Install system dependencies (see `mcs4-emu/INSTALLATION.md`).
3. Verify the build: `cargo build --workspace && cargo test --workspace`

## Workflow

- Create a feature branch per change: `feature/x`, `fix/x`, or `docs/x`.
- Keep PRs focused: one topic per PR.
- Update documentation alongside code changes.
- Ensure all three status files agree after milestone work (see convention in `claude.md`).

## Commit Conventions

- Use Conventional Commits: `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`.
- Keep messages imperative and concise ("Add 4308 bus protocol tests").
- ASCII-only commit messages; no Unicode or emoji.

## Code Style

- Run `cargo fmt --all` before committing.
- All code must pass `cargo clippy --all-targets -- -D warnings` with zero warnings.
- Warnings are treated as errors across the workspace (`-D warnings` in `.cargo/config.toml`).
- Use standard Rust idioms; follow patterns in existing chip implementations (e.g., `i4001.rs`).

## Testing Requirements

- Every new feature or chip implementation must include unit tests.
- Target >=90% coverage for new code.
- Run the full suite: `cargo test --workspace`
- Property-based tests (proptest) encouraged for decode paths and instruction execution.
- Tests must not panic on any valid or invalid input (see `fuzz_test.rs` pattern).

## Evidence and Claims

- All accuracy claims must cite primary sources (Intel datasheets, manuals).
- New chip implementations should reference the relevant datasheet section.
- Evidence artifacts go under `docs/evidence/` with provenance documented in
  `docs/evidence/photomicrograph_permissions.md`.

## PR Review Checklist

- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy --all-targets -- -D warnings` passes
- [ ] `cargo test --workspace` passes with no regressions
- [ ] New tests added for new functionality
- [ ] Documentation updated if behavior changes
- [ ] No secrets or credentials in diff
