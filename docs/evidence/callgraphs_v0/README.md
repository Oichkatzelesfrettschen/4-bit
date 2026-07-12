# Call-Graph Evidence Bundle (callgraphs_v0)

Structural call/dependency maps for the load-bearing execution paths of the
workspace, generated from source with per-language static tooling. cflow and
cscope parse C; this workspace contains Rust, Python, and Verilog, so each
language uses the tool that actually parses it:

| language | tool | artifact |
|----------|------|----------|
| Rust | cargo-modules 0.x (`dependencies`/`structure`) | `mcs4-core_module_deps.dot`, `mcs4-core_structure.txt`, `mcs4-chips_structure.txt` |
| Rust (fn-level) | curated source trace (grep + read) | `solver_path_callmap.md` |
| Python | `scripts/extract_python_callgraph.py` AST direct-call extractor | `gate_to_verilog_v0_callgraph.txt`, `autofill_manual_readings_ocr_v1_callgraph.txt` |
| shell pipeline | stage order read from `scripts/ci_schematic_pipeline_v0.sh` | `solver_path_callmap.md` (pipeline section) |

Regeneration:

```sh
cargo modules dependencies --package mcs4-core --lib --no-externs > mcs4-core_module_deps.dot
cargo modules structure --package mcs4-core --lib > mcs4-core_structure.txt
cargo modules structure --package mcs4-chips --lib > mcs4-chips_structure.txt
python3 scripts/extract_python_callgraph.py scripts/gate_to_verilog_v0.py \
  docs/evidence/callgraphs_v0/gate_to_verilog_v0_callgraph.txt
```

The regenerated gate-export edge list marks calls to names defined outside the
module as `[external]`; older retained edge lists use `[ext]`. Internal edges
use lexical qualified names. cflow and cscope remain captured as C-oriented
lexical probes, but their Python output is not a semantic call graph. Verilog
module instantiation hierarchy is flat (gate-level netlists instantiate only
primitive cells), so no separate Verilog graph is emitted; the mcs4-fpga system
hierarchy is `mcs4_top -> {clock_gen, i4004_fpga, i4001_fpga, i4002_fpga,
uart_bridge -> {uart_tx, uart_rx}, rom_bsram, ram_bsram}` per
`mcs4-emu/crates/mcs4-fpga/gowin/mcs4_top.v`.
