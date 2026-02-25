# MCS-4/MCS-40 Emulator

A transistor-level accurate emulator of Intel's first microprocessor family.

This project simulates the MCS-4 (4004) and MCS-40 (4040) chip families at
multiple levels of abstraction:

- **Cycle-accurate**: instruction-level simulation matching documented timing
- **Gate-level**: logic gate simulation with propagation delay modeling
- **Transistor-level**: SPICE-class simulation using real semiconductor physics

The transistor-level engine uses process parameters from Intel's 10um pMOS
silicon-gate technology (1971) and works with transistor netlists extracted
from die photomicrographs.

## Quick Start

```bash
cargo test --workspace
cargo doc --workspace --no-deps --open
```

## Project Status

See the [Roadmap](./reference/roadmap.md) for current completion status.
