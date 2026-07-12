# Load-Bearing Path Call Maps

Function-level call maps for the three execution paths that carry the
project's correctness claims: the transient/DC solver core, the
chip-to-solver bridge, and the schematic evidence pipeline. Anchors cite
the defining file and line at the revision this bundle was generated from.

## 1. Transient solver path (mcs4-core)

`TransientSolver::run` (`src/solver/transient.rs:300`) is the entry point.

```
TransientSolver::run
|- free-node scan over CircuitGraph.nodes (is_fixed filter)
|- DeviceModel construction per transistor
|    |- PmosLevel1::new          (Enhancement kind)
|    `- DepletionLoadModel::new  (Depletion kind)
|- DC operating point as initial condition (DcSolver::solve, dc_op.rs:133)
`- time-step loop
   |- estimate_lte (transient.rs:670)          # TRBDF2 order control
   |- companion_models_with_method (transient.rs:258)
   |    |- capacitor_geq / capacitor_ieq        (BackwardEuler)
   |    `- trap_capacitor_geq / trap_capacitor_ieq (Trapezoidal)
   |- backend select: step_num_free >= SPARSE_THRESHOLD
   |    |- SparseMnaSystem::new -> SparseMnaSystem::solve (sparse_matrix.rs:245)
   |    `- MnaSystem::new       -> MnaSystem::solve       (matrix.rs:211)
   `- LTE-based dt adaptation (ratio clamp 0.5..2.0 against config.lte_tol)
```

Sibling entry points sharing the MNA backends: `DcSolver::solve`
(`solver/dc_op.rs:133`), `AcSolver::solve` (`solver/ac.rs:172`),
`run_temperature_sweep` (`solver/temp_sweep.rs:109`),
`run_sensitivity_analysis` (`solver/sensitivity.rs:330`).
The legacy `NodalSolver` (`nodal_solver.rs:228 solve`, `:360 solve_robust`)
stamps directly (`stamp_conductance`/`stamp_transconductance`/
`stamp_current_source`/`stamp_voltage_source`) without the companion-model
machinery.

## 2. Chip-to-solver bridge path (mcs4-chips -> mcs4-core)

Each of the four core chips implements `ChipSolverBridge` with the same
shape (4001 at `i4001.rs:203`, 4002 at `i4002.rs:297`, 4003 at
`i4003.rs:159`, 4040 at `i4040/solver_bridge.rs:12`):

```
<Chip as ChipSolverBridge>::subcircuit(name)
|- repo-root discovery: env!("CARGO_MANIFEST_DIR").ancestors().nth(3)
|- mcs4_core::layout_netlist::load_netlist_v1 (layout_netlist.rs:56)
|    `- serde_json deserialize -> NetlistV1
`- mcs4_core::circuit::netlist_bridge::netlist_v1_to_circuit
     `- CircuitGraph (consumable by every solver in section 1)
```

## 3. Schematic evidence pipeline (Python/shell)

Stage order as executed by `scripts/ci_schematic_pipeline_v0.sh`:

```
audit_schematic_layout_anchors_v1.py
-> report_pad_anchor_consistency_v0.py
-> check_anchor_incidence_v0.py        (per chip 4001/4002/4003/4004)
-> check_anchor_uniqueness_v0.py       (4001/4002/4003)
```

Downstream generation path (manual, feeds docs/evidence/verilog_v0):

```
extract_gates_v0.py -> gates_v0/<chip>/ JSON
-> gate_to_verilog_v0.py
   main -> load_gates -> parse_gates_netlist, validate_gate_shapes
        -> extract_nodes, load_signal_ports
        -> analyze_gate_export_contract -> analyze_output_cone
        -> check_generated_exports -> render_exports
        -> generate_verilog_module -> generate_gate_instance, generate_primitive_library
        -> generate_testbench -> resolution oracle or explicit $fatal
```

Per-function edge lists for the Python stages live beside this file
(`gate_to_verilog_v0_callgraph.txt`, `autofill_manual_readings_ocr_v1_callgraph.txt`).
