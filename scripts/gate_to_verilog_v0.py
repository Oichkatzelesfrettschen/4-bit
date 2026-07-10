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


@dataclass
class SignalPort:
    """Chip pad signal anchored to a gate-netlist node.

    Multiple schematic names can share one physical pad node (e.g. POC and
    RESET on the 4004); `aliases` retains the full name set while `name`
    carries the emitted port identifier.
    """

    name: str
    node: int
    direction: str  # "input" | "output"
    aliases: tuple[str, ...]


def load_signal_ports(chip: str, netlists_dir: Path, nodes: set[int], driven: set[int]) -> list[SignalPort]:
    """Derive module ports from netlist_v1 signal anchors.

    A signal becomes a port when its layout_node exists in the gate
    netlist's node set. Direction is structural: a node driven by any gate
    output is a chip output; a node only consumed by gate inputs is a chip
    input. Signals sharing a node collapse into one port (aliases kept,
    plain names preferred over *_PAD variants).
    """
    netlist_path = netlists_dir / f"{chip}_netlist_v1.json"
    if not netlist_path.exists():
        print(f"  Warning: netlist_v1 not found, no ports derived: {netlist_path}")
        return []

    power_rails = {"VDD", "VSS", "GND", "VCC", "VDD_PAD", "VSS_PAD"}
    doc = json.loads(netlist_path.read_text(encoding="utf-8"))
    by_node: dict[int, list[str]] = {}
    for sig in doc.get("signals", []):
        node = sig.get("layout_node")
        name = sig.get("name")
        if node is None or not name:
            continue
        if name.upper() in power_rails:
            # Rails are the module's fixed VDD/VSS ports, not signal ports.
            continue
        if node not in nodes:
            print(f"  Note: signal {name} anchors node {node} absent from gate netlist; skipped")
            continue
        by_node.setdefault(node, []).append(name)

    ports = []
    for node, names in sorted(by_node.items()):
        # Prefer plain names over *_PAD variants, then shortest, then lexical.
        canonical = sorted(names, key=lambda n: (n.endswith("_PAD"), len(n), n))[0]
        direction = "output" if node in driven else "input"
        ports.append(SignalPort(canonical, node, direction, tuple(sorted(names))))
    ports.sort(key=lambda p: (p.direction, p.name))
    return ports


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
    ports: list[SignalPort],
) -> str:
    """Generate Verilog module from gate list and pad-signal ports."""
    lines = []

    # Header comment
    lines.append("// Auto-generated from gate-level netlist")
    lines.append(f"// Chip: {chip}")
    lines.append("// Tool: gate_to_verilog_v0.py")
    lines.append("// Ports derived from netlist_v1 signal anchors (layout_node)")
    lines.append("")

    module_name = f"i{chip}_gates"
    lines.append(f"module {module_name} (")
    lines.append("    input wire VDD,")
    port_decls = ["    input wire VSS"]
    for port in ports:
        alias_note = ""
        if len(port.aliases) > 1:
            alias_note = f"  // aliases: {', '.join(port.aliases)}"
        port_decls.append(f"    {port.direction} wire {port.name},{alias_note}")
    # The final port line carries no trailing comma; re-order so VSS stays
    # second and the last derived port closes the list.
    if ports:
        last = port_decls.pop()
        # strip the trailing comma of the last port, keep any alias comment
        if "," in last:
            head, _, tail = last.partition(",")
            last = head + tail
        port_decls.append(last)
        port_decls[0] += ","
    lines.extend(port_decls)
    lines.append(");")
    lines.append("")

    # Wire declarations
    lines.append("    // Internal wires")
    for node in sorted(nodes):
        lines.append(f"    wire n{node};")
    lines.append("")

    # Pad bindings between derived ports and their netlist nodes
    if ports:
        lines.append("    // Pad bindings (signal anchor -> gate-netlist node)")
        for port in ports:
            if port.direction == "input":
                lines.append(f"    assign n{port.node} = {port.name};")
            else:
                lines.append(f"    assign {port.name} = n{port.node};")
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


CLOCK_NAMES = {"CLK1", "CLK2", "CLOCK", "PHI1", "PHI2"}
RESET_NAMES = {"RESET", "POC", "POC_PAD", "CL"}


def classify_input(port: SignalPort) -> str:
    """Classify an input port for stimulus generation."""
    alias_set = set(port.aliases) | {port.name}
    if alias_set & CLOCK_NAMES:
        return "clock2" if {"CLK2", "PHI2"} & alias_set else "clock1"
    if alias_set & RESET_NAMES:
        return "reset"
    return "data"


def generate_testbench(chip: str, module_name: str, ports: list[SignalPort]) -> str:
    """Generate a testbench driving two-phase clocks, reset, and data.

    The MCS-4 family runs on non-overlapping phi1/phi2 with a 1350 ns
    period; single-clock parts (4003 shift register) get one clock at the
    same period. Data inputs walk a deterministic pattern so every input
    toggles; output values are displayed at end of simulation for eyeball
    and waveform comparison against the behavioral model.
    """
    inputs = [p for p in ports if p.direction == "input"]
    outputs = [p for p in ports if p.direction == "output"]
    clock1 = [p for p in inputs if classify_input(p) == "clock1"]
    clock2 = [p for p in inputs if classify_input(p) == "clock2"]
    resets = [p for p in inputs if classify_input(p) == "reset"]
    data = [p for p in inputs if classify_input(p) == "data"]

    lines = [
        "// Testbench (auto-generated)",
        f"// Module: {module_name}",
        "// Two-phase non-overlapping clock, reset pulse, walking data pattern",
        "",
        "`timescale 1ns/1ps",
        "",
        f"module tb_{module_name};",
        "    reg VDD, VSS;",
    ]
    for port in inputs:
        lines.append(f"    reg {port.name};")
    for port in outputs:
        lines.append(f"    wire {port.name};")
    lines.append("")
    lines.append(f"    {module_name} dut (")
    conns = ["        .VDD(VDD)", "        .VSS(VSS)"]
    conns.extend(f"        .{p.name}({p.name})" for p in inputs + outputs)
    lines.append(",\n".join(conns))
    lines.append("    );")
    lines.append("")
    lines.append("    // Two-phase non-overlapping clock, 1350 ns period:")
    lines.append("    // phi1 high 0-540, dead 540-675, phi2 high 675-1215, dead 1215-1350.")
    lines.append("    integer cycle;")
    lines.append("    initial begin")
    lines.append("        VDD = 1;")
    lines.append("        VSS = 0;")
    for port in clock1 + clock2:
        lines.append(f"        {port.name} = 0;")
    for port in resets:
        lines.append(f"        {port.name} = 1;")
    for i, port in enumerate(data):
        lines.append(f"        {port.name} = {i % 2};")
    lines.append("")
    lines.append("        for (cycle = 0; cycle < 32; cycle = cycle + 1) begin")
    for port in clock1:
        lines.append(f"            {port.name} = 1;")
    lines.append("            #540;")
    for port in clock1:
        lines.append(f"            {port.name} = 0;")
    lines.append("            #135;")
    for port in clock2:
        lines.append(f"            {port.name} = 1;")
    lines.append("            #540;")
    for port in clock2:
        lines.append(f"            {port.name} = 0;")
    lines.append("            #135;")
    if resets:
        lines.append("            if (cycle == 1) begin")
        for port in resets:
            lines.append(f"                {port.name} = 0;")
        lines.append("            end")
    if data:
        lines.append("            // Walk the data inputs so every one toggles")
        for i, port in enumerate(data):
            lines.append(f"            {port.name} = (cycle >> {i % 5}) & 1;")
    lines.append("        end")
    lines.append("")
    if outputs:
        names = ", ".join(f"{p.name}=%b" for p in outputs)
        args = ", ".join(p.name for p in outputs)
        lines.append(f'        $display("{module_name} final: {names}", {args});')
    lines.append("        $finish;")
    lines.append("    end")
    lines.append("")
    lines.append("    // Waveform dump")
    lines.append("    initial begin")
    lines.append(f'        $dumpfile("tb_{module_name}.vcd");')
    lines.append(f"        $dumpvars(0, tb_{module_name});")
    lines.append("    end")
    lines.append("endmodule")
    lines.append("")
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
        "--netlists-dir",
        type=Path,
        default=ROOT / "docs" / "evidence" / "netlists_v1",
        help="netlist_v1 directory supplying signal anchors for ports",
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
            print("  Creating placeholder output")

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

        # Ports from signal anchors; direction from structural drivenness
        driven = set()
        for gate in gates:
            driven.update(gate.outputs)
        ports = load_signal_ports(chip, args.netlists_dir, nodes, driven)

        # Generate Verilog
        module_name = f"i{chip}_gates"
        verilog_code = generate_verilog_module(chip, gates, nodes, ports)

        # Write output
        output_dir = args.output_dir / chip
        output_dir.mkdir(parents=True, exist_ok=True)

        verilog_file = output_dir / f"{module_name}.v"
        verilog_file.write_text(verilog_code, encoding="utf-8")

        print(f"  Generated: {verilog_file.name}")
        print(f"    Gates: {len(gates)}")
        print(f"    Nodes: {len(nodes)}")
        n_in = sum(1 for p in ports if p.direction == "input")
        n_out = len(ports) - n_in
        print(f"    Ports: {n_in} inputs, {n_out} outputs (from signal anchors)")

        # Generate testbench if requested
        if args.generate_testbench:
            testbench = generate_testbench(chip, module_name, ports)
            tb_file = output_dir / f"tb_{module_name}.v"
            tb_file.write_text(testbench, encoding="utf-8")
            print(f"  Generated: {tb_file.name}")

        print("")

    print("Verilog export complete")
    print("")
    print("NOTE: Port directions are structural (driven node -> output);")
    print("functional per-chip test vectors remain future work.")


if __name__ == "__main__":
    main()
