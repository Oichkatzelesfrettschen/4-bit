# Transistor-Level Simulation

The transistor-level engine uses SPICE-class methods to simulate the actual
pMOS circuits extracted from die photomicrographs.

## Architecture

- `process/` -- Semiconductor material properties (10um pMOS SGT)
- `device/` -- Shichman-Hodges Level 1 pMOS transistor model
- `circuit/` -- Netlist bridge (JSON to CircuitGraph)
- `solver/` -- MNA matrix, Newton-Raphson DC, backward Euler transient

## Process Parameters

{{#include ../../docs/evidence/process_parameters_v0.md}}
