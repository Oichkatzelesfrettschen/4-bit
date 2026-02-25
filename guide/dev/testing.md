# Testing

```bash
cargo test --workspace
cargo clippy --all-targets -- -D warnings
```

## Test Coverage

```bash
cargo llvm-cov --workspace --all-features --lcov --output-path coverage/lcov.info
```

{{#include ../../docs/CLAIMS_TO_TESTS.md}}
