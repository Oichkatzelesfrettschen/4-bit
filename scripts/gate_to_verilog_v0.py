#!/usr/bin/env python3
"""
Convert gate-level netlists to synthesizable Verilog HDL.

Reads gates_v0 JSON format and generates:
- Synthesizable Verilog module
- Testbench template
- Timing constraints (optional)

Supports: INV, NAND, NOR, TGATE gate primitives
"""

from __future__ import annotations

import argparse
import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]


@dataclass
class Gate:
    """Parsed gate from netlist."""

    gate_type: str
    inputs: list[int]
    outputs: list[int]
    gate_id: int


@dataclass
class VerilogModule:
    """Verilog module representation."""

    name: str
    ports: list[tuple[str, str, str]]  # (direction, width, name)
    wires: set[int]
    gates: list[Gate]


def parse_gates_netlist(netlist_path: Path) -> dict[str, Any]:
    """Load and parse gates_v0 JSON."""
    return json.loads(netlist_path.read_text(encoding="utf-8"))


def extract_nodes(netlist: dict[str, Any]) -> set[int]:
    """Extract all unique node IDs from gate netlist."""
    nodes = set()
    for gate in netlist.get("gates", []):
        nodes.update(gate.get("inputs", []))
        nodes.update(gate.get("outputs", []))
    return nodes


def generate_verilog_module(
    chip: str,
    gates: list[Gate],
    nodes: set[int],
) -> str:
    """Generate Verilog module from gate list."""
    lines = []

    # Header comment
    lines.append("// Auto-generated from gate-level netlist")
    lines.append(f"// Chip: {chip}")
    lines.append("// Tool: gate_to_verilog_v0.py")
    lines.append("")

    # Module declaration (simplified - no I/O for now)
    module_name = f"i{chip}_gates"
    lines.append(f"module {module_name} (")
    lines.append("    input wire VDD,")
    lines.append("    input wire VSS")
    lines.append(");")
    lines.append("")

    # Wire declarations
    lines.append("    // Internal wires")
    for node in sorted(nodes):
        lines.append(f"    wire n{node};")
    lines.append("")

    # Gate instantiations
    lines.append("    // Gate instances")
    for i, gate in enumerate(gates):
        gate_inst = generate_gate_instance(gate, i)
        lines.append(f"    {gate_inst}")
    lines.append("")

    lines.append("endmodule")
    lines.append("")

    # Append primitive library
    lines.extend(generate_primitive_library())

    return "\n".join(lines)


def generate_gate_instance(gate: Gate, inst_id: int) -> str:
    """Generate Verilog instantiation for single gate."""
    gate_type_map = {
        "INV": "inv",
        "NAND": f"nand{len(gate.inputs)}",
        "NOR": f"nor{len(gate.inputs)}",
        "TGATE": "tgate",
        "PASS": "pass_trans",
    }

    prim_name = gate_type_map.get(gate.gate_type, "unknown")
    inst_name = f"g{inst_id}"

    # Build port connections
    connections = []

    if gate.gate_type in ["INV"]:
        # Single input, single output
        connections.append(f".A(n{gate.inputs[0]})")
        connections.append(f".Y(n{gate.outputs[0]})")
    elif gate.gate_type in ["NAND", "NOR"]:
        # Multiple inputs, single output
        for i, inp in enumerate(gate.inputs):
            port_name = chr(ord("A") + i)  # A, B, C, ...
            connections.append(f".{port_name}(n{inp})")
        connections.append(f".Y(n{gate.outputs[0]})")
    elif gate.gate_type == "TGATE":
        # Transmission gate (2 control, bidirectional data)
        connections.append(f".EN(n{gate.inputs[0]})")
        connections.append(f".ENB(n{gate.inputs[1]})")
        connections.append(f".A(n{gate.outputs[0]})")
        connections.append(f".B(n{gate.outputs[1] if len(gate.outputs) > 1 else gate.outputs[0]})")

    return f"{prim_name} {inst_name} ({', '.join(connections)});"


def generate_primitive_library() -> list[str]:
    """Generate inline Verilog primitive library."""
    lines = [
        "// ========================================",
        "// Primitive Gate Library",
        "// ========================================",
        "",
        "// Inverter",
        "module inv (",
        "    input wire A,",
        "    output wire Y",
        ");",
        "    assign Y = ~A;",
        "endmodule",
        "",
        "// 2-input NAND",
        "module nand2 (",
        "    input wire A,",
        "    input wire B,",
        "    output wire Y",
        ");",
        "    assign Y = ~(A & B);",
        "endmodule",
        "",
        "// 3-input NAND",
        "module nand3 (",
        "    input wire A,",
        "    input wire B,",
        "    input wire C,",
        "    output wire Y",
        ");",
        "    assign Y = ~(A & B & C);",
        "endmodule",
        "",
        "// 2-input NOR",
        "module nor2 (",
        "    input wire A,",
        "    input wire B,",
        "    output wire Y",
        ");",
        "    assign Y = ~(A | B);",
        "endmodule",
        "",
        "// 3-input NOR",
        "module nor3 (",
        "    input wire A,",
        "    input wire B,",
        "    input wire C,",
        "    output wire Y",
        ");",
        "    assign Y = ~(A | B | C);",
        "endmodule",
        "",
        "// Transmission gate (bidirectional)",
        "module tgate (",
        "    input wire EN,",
        "    input wire ENB,",
        "    inout wire A,",
        "    inout wire B",
        ");",
        "    assign A = EN ? B : 1'bz;",
        "    assign B = EN ? A : 1'bz;",
        "endmodule",
        "",
    ]
    return lines


def generate_testbench(chip: str, module_name: str) -> str:
    """Generate basic testbench template."""
    lines = [
        "// Testbench template (auto-generated)",
        f"// Module: {module_name}",
        "",
        "`timescale 1ns/1ps",
        "",
        f"module tb_{module_name};",
        "    // DUT signals",
        "    reg VDD, VSS;",
        "",
        "    // DUT instantiation",
        f"    {module_name} dut (",
        "        .VDD(VDD),",
        "        .VSS(VSS)",
        "    );",
        "",
        "    // Initialize",
        "    initial begin",
        "        VDD = 1;",
        "        VSS = 0;",
        "",
        "        // TODO: Add test stimulus",
        "",
        "        #10000 $finish;",
        "    end",
        "",
        "    // Waveform dump",
        "    initial begin",
        f'        $dumpfile("tb_{module_name}.vcd");',
        f"        $dumpvars(0, tb_{module_name});",
        "    end",
        "endmodule",
        "",
    ]
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(description="Gate-level netlist to Verilog")
    parser.add_argument(
        "--chips",
        nargs="+",
        default=["4001", "4002", "4003", "4004"],
        help="Chips to process",
    )
    parser.add_argument(
        "--input-dir",
        type=Path,
        default=ROOT / "docs" / "evidence" / "gates_v0",
        help="Input gates directory",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=ROOT / "docs" / "evidence" / "verilog_v0",
        help="Output Verilog directory",
    )
    parser.add_argument(
        "--generate-testbench",
        action="store_true",
        help="Generate testbench for each module",
    )

    args = parser.parse_args()

    args.output_dir.mkdir(parents=True, exist_ok=True)

    print("=== Gate-Level to Verilog Export ===")
    print("")

    for chip in args.chips:
        print(f"Processing {chip}...")

        # Input path
        gates_file = args.input_dir / chip / f"{chip}_gates_v0.json"

        if not gates_file.exists():
            print(f"  Warning: Gates file not found: {gates_file}")
            print(f"  Creating placeholder output")

            # Create placeholder netlist
            netlist = {
                "schema_version": 0,
                "chip": chip,
                "gates": [],
                "statistics": {"total_gates": 0},
            }
        else:
            # Load actual netlist
            netlist = parse_gates_netlist(gates_file)

        # Extract nodes and gates
        nodes = extract_nodes(netlist)
        gates = [
            Gate(
                gate_type=g["gate_type"],
                inputs=g["inputs"],
                outputs=g["outputs"],
                gate_id=i,
            )
            for i, g in enumerate(netlist.get("gates", []))
        ]

        # Generate Verilog
        module_name = f"i{chip}_gates"
        verilog_code = generate_verilog_module(chip, gates, nodes)

        # Write output
        output_dir = args.output_dir / chip
        output_dir.mkdir(parents=True, exist_ok=True)

        verilog_file = output_dir / f"{module_name}.v"
        verilog_file.write_text(verilog_code, encoding="utf-8")

        print(f"  Generated: {verilog_file.name}")
        print(f"    Gates: {len(gates)}")
        print(f"    Nodes: {len(nodes)}")

        # Generate testbench if requested
        if args.generate_testbench:
            testbench = generate_testbench(chip, module_name)
            tb_file = output_dir / f"tb_{module_name}.v"
            tb_file.write_text(testbench, encoding="utf-8")
            print(f"  Generated: {tb_file.name}")

        print("")

    print("Verilog export complete")
    print("")
    print("NOTE: Generated modules are simplified (no I/O ports)")
    print("TODO: Extract I/O from anchor nodes in subcircuit manifests")


if __name__ == "__main__":
    main()
