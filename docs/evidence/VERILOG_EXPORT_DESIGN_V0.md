# Verilog Export Design for Gate-Level Netlists (v0)

**Date**: 2026-01-29
**Status**: DESIGN DOCUMENT
**Target**: Phase 5 implementation
**Purpose**: Export gate-level netlists to synthesizable Verilog HDL

---

## Executive Summary

This document defines the architecture for converting gate-level netlists (gates_v0 format) into synthesizable Verilog HDL modules suitable for FPGA implementation. The goal is to enable hardware verification and FPGA-based emulation of extracted Intel MCS-4 chip circuits.

---

## Input Format: gates_v0

### Structure

```json
{
  "schema_version": 0,
  "chip": "4001",
  "description": "Gate-level netlist",
  "gates": [
    {
      "gate_type": "INV",
      "inputs": [1236],
      "outputs": [2593],
      "transistors": [257, 525],
      "confidence": 1.0
    },
    {
      "gate_type": "NAND",
      "inputs": [1234, 5678],
      "outputs": [9012],
      "transistors": [100, 200],
      "confidence": 0.8
    }
  ],
  "statistics": {
    "total_gates": 150,
    "inverters": 80,
    "nand_gates": 40,
    "nor_gates": 20,
    "transmission_gates": 10
  }
}
```

### Gate Types Supported

- **INV**: Inverter (1 input, 1 output)
- **NAND**: N-input NAND gate (2+ inputs, 1 output)
- **NOR**: N-input NOR gate (2+ inputs, 1 output)
- **TGATE**: Transmission gate (2 control inputs, bidirectional data)
- **PASS**: Pass transistor (1 control, 2 data terminals)

---

## Output Format: Verilog HDL

### Module Structure

```verilog
// Auto-generated from gate-level netlist
// Chip: 4001
// Generated: 2026-01-29
// Tool: gate_to_verilog_v0.py

module i4001_gates (
    // Primary inputs
    input wire CLK1,
    input wire CLK2,
    input wire RESET,
    input wire SYNC,
    input wire [3:0] D_PAD,

    // Primary outputs
    output wire [3:0] IO,

    // Power
    input wire VDD,
    input wire VSS
);

    // Internal wire declarations
    wire n1236;
    wire n2593;
    wire n2883;
    // ... (all internal nodes)

    // Gate instantiations
    inv g0 (.A(n1236), .Y(n2593));
    nand2 g1 (.A(n1234), .B(n5678), .Y(n9012));
    nor2 g2 (.A(n100), .B(n200), .Y(n300));
    // ... (all gates)

endmodule

// Primitive gate library
module inv (
    input wire A,
    output wire Y
);
    assign Y = ~A;
endmodule

module nand2 (
    input wire A,
    input wire B,
    output wire Y
);
    assign Y = ~(A & B);
endmodule

module nor2 (
    input wire A,
    input wire B,
    output wire Y
);
    assign Y = ~(A | B);
endmodule

module tgate (
    input wire EN,
    input wire ENB,
    inout wire A,
    inout wire B
);
    // Transmission gate (bidirectional)
    assign A = EN ? B : 1'bz;
    assign B = EN ? A : 1'bz;
endmodule
```

---

## Design Decisions

### 1. Node Naming Convention

**Decision**: Use `nXXXX` for internal nodes, preserve anchor names for I/O

**Examples**:
```verilog
wire n1236;        // Internal node (from netlist node ID 1236)
wire CLK1;         // Anchor/I/O signal (preserved name)
wire D0_PAD;       // Pad signal (preserved name)
```

**Rationale**:
- Simple, unambiguous naming
- Preserves human-readable anchor names
- Avoids conflicts with Verilog keywords

### 2. Gate Library Strategy

**Decision**: Inline primitive library at end of file

**Alternatives Considered**:
- Separate library file (more modular, but requires include path)
- Use Verilog built-in gates (less portable across tools)
- Technology-mapped cells (requires synthesis library)

**Chosen Approach**: Inline behavioral primitives
- Self-contained (single .v file)
- Portable across all Verilog simulators
- Easy to modify for specific FPGA libraries

### 3. Port Mapping

**Decision**: Extract I/O from anchor nodes in manifest

**Strategy**:
1. Parse subcircuit manifest to identify anchors
2. Classify anchors as input, output, or bidirectional
3. Generate port list in logical order (CLK, RESET, data, control)
4. Add VDD/VSS as explicit inputs (for FPGA power-on reset)

**Example Mapping** (4001 ROM):
```verilog
module i4001_gates (
    // Clock inputs
    input wire CLK1,
    input wire CLK2,

    // Control inputs
    input wire RESET,
    input wire SYNC,
    input wire CM,
    input wire CL,

    // Data bus (bidirectional)
    inout wire [3:0] D,

    // I/O ports (bidirectional)
    inout wire [3:0] IO,

    // Power
    input wire VDD,
    input wire VSS
);
```

### 4. Bidirectional Signals

**Decision**: Use `inout` for transmission gates and I/O pads

**Challenge**: Transmission gates and pads are bidirectional
**Solution**: Model with `inout` wires and tri-state buffers

```verilog
// Bidirectional I/O pad
inout wire IO0;
wire io0_out;
wire io0_in;
wire io0_oe;  // Output enable

assign IO0 = io0_oe ? io0_out : 1'bz;
assign io0_in = IO0;
```

### 5. Unconnected Nodes

**Decision**: Tie unconnected inputs to VSS (ground)

**Rationale**:
- Prevents floating inputs (synthesis warning)
- Matches expected NMOS behavior (weak pull-down)
- Conservative default for unused logic

```verilog
// Tie unused inputs to ground
assign unused_node = VSS;
```

---

## Testbench Generation

### Basic Testbench Template

```verilog
// Testbench for i4001_gates
`timescale 1ns/1ps

module tb_i4001_gates;
    // DUT signals
    reg CLK1, CLK2;
    reg RESET, SYNC;
    reg [3:0] D_PAD;
    wire [3:0] IO;
    reg VDD, VSS;

    // DUT instantiation
    i4001_gates dut (
        .CLK1(CLK1),
        .CLK2(CLK2),
        .RESET(RESET),
        .SYNC(SYNC),
        .D_PAD(D_PAD),
        .IO(IO),
        .VDD(VDD),
        .VSS(VSS)
    );

    // Clock generation (2 MHz for MCS-4)
    initial begin
        CLK1 = 0;
        forever #250 CLK1 = ~CLK1;  // 2 MHz
    end

    initial begin
        CLK2 = 0;
        forever #250 CLK2 = ~CLK2;  // 2 MHz, 90-degree phase shift
    end

    // Test sequence
    initial begin
        // Initialize
        VDD = 1;
        VSS = 0;
        RESET = 1;
        SYNC = 0;
        D_PAD = 4'b0000;

        // Reset sequence
        #1000 RESET = 0;
        #1000 RESET = 1;

        // Apply test vectors
        #1000 D_PAD = 4'b1010;
        #1000 SYNC = 1;
        #1000 SYNC = 0;

        // Monitor outputs
        $monitor("Time=%0t IO=%b", $time, IO);

        // Run for 10us
        #10000 $finish;
    end

    // Waveform dump
    initial begin
        $dumpfile("tb_i4001_gates.vcd");
        $dumpvars(0, tb_i4001_gates);
    end
endmodule
```

---

## Synthesis Considerations

### Timing Constraints

**Clock Period**: 500ns (2 MHz for MCS-4)

```tcl
# Timing constraints for Xilinx Vivado (.xdc)
create_clock -period 500.0 -name CLK1 [get_ports CLK1]
create_clock -period 500.0 -name CLK2 [get_ports CLK2]

set_input_delay -clock CLK1 10.0 [get_ports {D[*] RESET SYNC}]
set_output_delay -clock CLK1 10.0 [get_ports {IO[*]}]
```

### Resource Utilization Estimates

**4001 ROM** (256x8 ROM + 4-bit I/O):
- Gates: ~150 (from extraction)
- LUTs: ~200 (4-input LUTs)
- FFs: ~50 (for I/O registers)
- Block RAM: 1 (for ROM storage)

**4004 CPU** (based on subcircuit extraction):
- Gates: ~437 (CLK1 subcircuit alone)
- LUTs: ~600
- FFs: ~100
- Total chip: ~2000 LUTs (estimate)

**Target FPGAs**:
- Lattice iCE40HX4K: 3,520 LUTs (sufficient for single chip)
- Xilinx Spartan-7 (XC7S15): 12,800 LUTs (sufficient for full MCS-4 system)

### Technology Mapping

**FPGA-Specific Optimization**:
- Map small NANDs/NORs to LUTs
- Use BRAM for ROM storage (256x8 fits in one BRAM18)
- Use DSP blocks for arithmetic (if beneficial)

---

## Implementation Plan

### Phase 5 Task 38: gate_to_verilog_v0.py

**Step 1: Parser** (Day 1)
- Load gates_v0 JSON format
- Extract gate list, node list, I/O mapping
- Validate schema version

**Step 2: Node Analysis** (Day 2)
- Build node connectivity graph
- Identify primary inputs/outputs from anchors
- Detect bidirectional signals
- Find unconnected nodes

**Step 3: Verilog Generation** (Day 3)
- Generate module declaration with ports
- Generate wire declarations
- Generate gate instantiations
- Generate primitive library
- Handle special cases (transmission gates, bidirectional)

**Step 4: Testbench Generation** (Day 4)
- Generate basic testbench template
- Add clock generation
- Add reset sequence
- Add waveform dumping

**Step 5: Validation** (Day 5)
- Run through Icarus Verilog (iverilog) for syntax check
- Run through Verilator for lint check
- Compare gate count with input netlist
- Generate synthesis report

---

## Validation Strategy

### Syntax Validation

```bash
# Check Verilog syntax
iverilog -t null -Wall i4001_gates.v
verilator --lint-only i4001_gates.v
```

### Functional Simulation

```bash
# Simulate with Icarus Verilog
iverilog -o tb_i4001 tb_i4001_gates.v i4001_gates.v
vvp tb_i4001
gtkwave tb_i4001.vcd
```

### Synthesis Validation

```bash
# Synthesize with Yosys (open-source)
yosys -p "read_verilog i4001_gates.v; synth_ice40 -top i4001_gates; write_json i4001_synth.json"

# Or Xilinx Vivado
vivado -mode tcl -source synth_i4001.tcl
```

---

## File Organization

### Output Structure

```
docs/evidence/verilog_v0/
├── 4001/
│   ├── i4001_gates.v           (synthesizable module)
│   ├── tb_i4001_gates.v        (testbench)
│   ├── i4001_gates.sdc         (timing constraints)
│   └── README.md               (usage notes)
├── 4002/
│   ├── i4002_gates.v
│   └── ...
├── 4003/
│   └── ...
└── 4004/
    └── ...
```

---

## Future Enhancements

### Post-Phase 5

1. **Hierarchical Modules**: Export functional blocks as separate modules
2. **Technology Mapping**: Direct instantiation of FPGA primitives (LUT6, BRAM)
3. **Formal Verification**: SystemVerilog Assertions (SVA) for properties
4. **Power Analysis**: Activity-based power estimation
5. **Floorplan Constraints**: FPGA placement hints for critical paths

---

## References

- [Gate-level extraction](extract_gates_v0.py)
- [FPGA synthesis workflow](FPGA_SYNTHESIS_WORKFLOW_V0.md) (to be created)
- [Verilog IEEE 1364-2005 Standard](https://ieeexplore.ieee.org/document/1620780)

---

**Author**: Claude Sonnet 4.5 (claude-sonnet-4-5-20250929)
**Date**: 2026-01-29
**Status**: DESIGN COMPLETE - READY FOR IMPLEMENTATION
