# Gate HDL Export Contract

## Scope

`scripts/gate_to_verilog_v0.py` converts retained `gates_v0` evidence into
Verilog. The source graph is often partial: a signal anchor can be absent,
an output can have multiple extracted drivers, and unrelated gate fragments
can remain disconnected. Compilation alone cannot distinguish those cases
from a usable hardware model.

The export contract classifies each generated top module before simulation or
synthesis uses it. It establishes structural resolution of the declared
output surface only. It does not establish Intel-chip behavior, electrical
fidelity, timing closure, pin correctness, or board operation.

## Contract

For every declared output port, the checker traces its gate cone backward.
The cone passes only when all of these conditions hold:

1. The source contains one or more well-formed supported gates.
2. Every reachable gate uses a supported primitive with its exact arity and
   exactly one output.
3. Every reachable node has exactly one gate driver, unless it is a declared
   top-level input.
4. Every dependency leaf is a declared top-level input.
5. The cone contains no combinational cycle.

Gates outside every exported output cone remain reported as disconnected
extraction debt. They do not turn a resolved output cone into an unknown
simulation result, and they do not establish full-chip coverage.

Run the check directly:

~~~sh
python3 scripts/gate_to_verilog_v0.py --chips 4003 \
  --check-export-contract --check-generated
~~~

The command exits nonzero for an incomplete export. It also accepts
`--contract-report PATH` to write a deterministic JSON diagnostic for a
bounded analysis run. `--check-generated` also rejects a stale retained
Verilog module or testbench before delivery uses it.

## Call-path capture

The export path has three captured static layers:

1. `cflow --main=main scripts/gate_to_verilog_v0.py` exits zero but emits no
   graph and reports C-parser diagnostics for Python syntax. It supplies no
   usable Python call evidence.
2. `cscope` builds an index over the Python file, but its definitions and
   callee queries are token matches. It associates unrelated lexical uses and
   does not resolve Python scope or dispatch.
3. `scripts/extract_python_callgraph.py` parses Python AST and records 26
   defined functions with 29 same-file direct-call edges. Its map records the
   load, contract, render, testbench, and atomic-publication path.

The current direct path is:

~~~text
main
  -> load_gates -> parse_gates_netlist, validate_gate_shapes
  -> extract_nodes, load_signal_ports
  -> analyze_gate_export_contract -> analyze_output_cone
  -> check_generated_exports -> render_exports
  -> generate_verilog_module -> generate_gate_instance
  -> generate_testbench -> generated resolution oracle or explicit failure
  -> atomic_write_text
~~~

`scripts/callgraph_capture.sh` records the AST map plus the cflow and cscope
probe outputs in every full capture. The AST map remains lexical: imports,
dynamic dispatch, reflection, decorators, and higher-order calls remain
external boundaries. The focused current capture is retained under
`target/gate-hdl-contract-capture-20260711-v3/`. It records source and
artifact SHA-256 values, the cflow and cscope limitation statuses, a
26-function 29-edge AST map, a passing 4003 contract and Icarus run, and the expected
nonzero 4001 contract result.

## Measured retained-artifact state

| Chip | Exported surface | Contract result | Evidence-bound interpretation |
|---|---|---|---|
| 4001 | CL, CM, D2, D3 | fail | CL reaches undriven n1236, CM reaches n1228, and D2/D3 reach n39. The artifact is not a deliverable model. |
| 4002 | no output anchor | fail | The retained signal anchors expose no observable top-level output. The artifact is not a deliverable model. |
| 4003 | Q4 | pass | Q4 traces through one NAND gate to declared inputs Q2 and Q6. Five unrelated extracted gates remain disconnected. |
| 4004 | CLK1, CMRAM0, D0_PAD | fail | Each exported node has multiple drivers in the retained graph: 128, 21, and 21 respectively. The artifact is not a deliverable model. |

All eight retained Verilog artifacts, module plus testbench for each chip,
match the current generator and source evidence under `--check-generated`.
The three failures therefore describe retained extraction limits rather than
stale generated files.

The 4003 pass means the named Q4 cone resolves for all binary combinations of
the declared inputs. It does not prove that the retained cone is a complete
Intel 4003 model or that the names and directions match a physical pinout.

## Behavioral 4003 source boundary

The Rust I4003 model and the generated behavioral i4003 and i4003_fpga
modules implement a separate primary-source contract from the MCS-40 Users
Manual: CP rising edges shift data, E low exposes parallel outputs, E high
drives parallel outputs to VSS, and serial output remains independent of E.
The retained OCR locator is
docs/evidence/ocr/mcs40_users_manual.txt lines 8855-8859 and 8888-8899.
The generic behavioral module initializes its shift register to zero to
represent the retained power-on-clear behavior at lines 8974-8977. The
FPGA-safe module provides the same state through its host reset. Target
synthesis still requires a target-specific reset or initialization check.

This correction does not add a behavior-vector corpus for i4003_gates. The
gate artifact still resolves only Q4 through extracted Q2 and Q6 inputs. No
claim of behavioral RTL-to-gate equivalence, electrical equivalence, pin
equivalence, synthesis, or board behavior follows from this separation.

## Generated testbench behavior

For a contract-passing module with at most 12 declared inputs, the generator
emits an exhaustive binary-vector testbench. After each vector it waits one
simulation time unit and rejects an output unless it is exactly zero or one.
The testbench therefore fails on both X and Z values.

For a contract-failing module, the generator emits a testbench that calls
`$fatal` at time zero with the first structural failure. This replaces the
previous false success mode where a testbench displayed X values and called
`$finish` with exit zero.

The 4003 bench executes eight vectors and reports:

~~~text
PASS: i4003_gates resolves all 8 input vectors
~~~

The 4001, 4002, and 4004 benches fail intentionally. Their nonzero result is
the current evidence result, not a regression in a previously validated chip
model.

## Delivery gate

`mcs4-emu/crates/mcs4-fpga/Makefile` runs `gate_contract` before gate-mode
simulation and iCE40 or Spartan-7 synthesis. As a result:

- `make -C mcs4-emu/crates/mcs4-fpga sim CHIP=4003 MODE=gate` runs the
  resolution testbench.
- `make -C mcs4-emu/crates/mcs4-fpga sim CHIP=4001 MODE=gate` stops before
  compilation and reports the undriven source nodes.
- `just gate-contract` keeps the passing 4003 structural surface in the
  repository verification path.

This gate prevents an incomplete retained extraction from producing a
bitstream or a passing simulation that reviewers could mistake for a hardware
validation result.

## Remaining evidence work

1. Reconcile pad anchors and layout nodes for every absent interface signal.
2. Re-extract gates from a connectivity representation that preserves rail,
   driver, and transistor-role evidence.
3. Add independently sourced per-chip behavior vectors before asserting
   logical equivalence.
4. Compare behavioral RTL, generated gate HDL, synthesis netlist, and an
   attended board probe against the same vector corpus.
