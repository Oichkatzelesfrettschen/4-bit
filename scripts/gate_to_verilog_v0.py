#!/usr/bin/env python3
"""
Convert gate-level netlists to synthesizable Verilog HDL.

Reads gates_v0 JSON format and generates:
- Synthesizable Verilog module
- Testbench template
- Timing constraints (optional)

Supports: INV, NAND, NOR gate primitives. The single-type pMOS Intel
process yields no complementary transmission gates, so no classifier in
this pipeline emits a TGATE/PASS gate type; an unsupported type raises
instead of emitting an unresolvable primitive.
"""

from __future__ import annotations

import argparse
import json
import os
import tempfile
from collections import defaultdict
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


class GateContractError(ValueError):
    """Raised when retained gate evidence cannot form a valid HDL artifact."""


@dataclass(frozen=True)
class OutputConeStatus:
    """Structural resolution status for one exported output cone."""

    port: SignalPort
    reachable_gate_ids: tuple[int, ...]
    undriven_nodes: tuple[int, ...]
    multiply_driven_nodes: tuple[tuple[int, int], ...]
    cyclic_nodes: tuple[int, ...]

    @property
    def is_resolved(self) -> bool:
        """Return true when the output has one complete input-driven cone."""
        return not (
            self.undriven_nodes
            or self.multiply_driven_nodes
            or self.cyclic_nodes
        )

    def to_dict(self) -> dict[str, Any]:
        """Return a stable machine-readable representation."""
        return {
            "port": self.port.name,
            "node": self.port.node,
            "reachable_gate_count": len(self.reachable_gate_ids),
            "reachable_gate_ids": list(self.reachable_gate_ids),
            "undriven_nodes": list(self.undriven_nodes),
            "multiply_driven_nodes": [
                {"node": node, "driver_count": driver_count}
                for node, driver_count in self.multiply_driven_nodes
            ],
            "cyclic_nodes": list(self.cyclic_nodes),
            "resolved": self.is_resolved,
        }


@dataclass(frozen=True)
class GateExportContract:
    """Structural delivery contract for generated gate HDL.

    The contract proves only that every exported output cone resolves through
    declared input ports. It does not establish chip-level behavioral or
    electrical equivalence.
    """

    chip: str
    gate_count: int
    input_ports: tuple[SignalPort, ...]
    output_ports: tuple[SignalPort, ...]
    gate_errors: tuple[str, ...]
    output_cones: tuple[OutputConeStatus, ...]
    disconnected_gate_count: int
    global_multiply_driven_nodes: tuple[tuple[int, int], ...]

    @property
    def is_delivery_ready(self) -> bool:
        """Return true when the exported HDL surface has a closed contract."""
        return bool(self.output_ports) and not self.gate_errors and all(
            cone.is_resolved for cone in self.output_cones
        )

    def to_dict(self) -> dict[str, Any]:
        """Return a stable machine-readable representation."""
        return {
            "chip": self.chip,
            "gate_count": self.gate_count,
            "input_ports": [port.name for port in self.input_ports],
            "output_ports": [port.name for port in self.output_ports],
            "gate_errors": list(self.gate_errors),
            "output_cones": [cone.to_dict() for cone in self.output_cones],
            "disconnected_gate_count": self.disconnected_gate_count,
            "global_multiply_driven_nodes": [
                {"node": node, "driver_count": driver_count}
                for node, driver_count in self.global_multiply_driven_nodes
            ],
            "delivery_ready": self.is_delivery_ready,
        }


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


def validate_gate_shapes(gates: list[Gate]) -> tuple[str, ...]:
    """Return schema errors that make a gate-level HDL export invalid."""
    accepted_arities = {
        "INV": {1},
        "NAND": {2, 3},
        "NOR": {2, 3},
    }
    errors = []
    if not gates:
        errors.append("gate source contains no gates")

    for gate in gates:
        if gate.gate_type not in accepted_arities:
            errors.append(
                f"g{gate.gate_id} uses unsupported gate type {gate.gate_type!r}"
            )
            continue
        if len(gate.inputs) not in accepted_arities[gate.gate_type]:
            errors.append(
                f"g{gate.gate_id} {gate.gate_type} has {len(gate.inputs)} inputs; "
                f"expected {sorted(accepted_arities[gate.gate_type])}"
            )
        if len(gate.outputs) != 1:
            errors.append(
                f"g{gate.gate_id} has {len(gate.outputs)} outputs; expected 1"
            )
        if not all(isinstance(node, int) for node in gate.inputs + gate.outputs):
            errors.append(f"g{gate.gate_id} contains a non-integer node identifier")

    return tuple(errors)


def analyze_output_cone(
    port: SignalPort,
    input_nodes: set[int],
    driver_by_node: dict[int, list[Gate]],
) -> OutputConeStatus:
    """Trace an exported output back to declared inputs and report breaks.

    A generated top module is useful only when each exported output has one
    driver chain whose leaves are declared input ports. Disconnected gates
    outside those cones remain extraction debt, but do not make an otherwise
    observable output non-deterministic.
    """
    reachable_gate_ids: set[int] = set()
    undriven_nodes: set[int] = set()
    multiply_driven_nodes: dict[int, int] = {}
    cyclic_nodes: set[int] = set()

    def trace_node(node: int, active_path: set[int]) -> None:
        if node in input_nodes:
            return

        drivers = driver_by_node.get(node, [])
        if not drivers:
            undriven_nodes.add(node)
            return
        if len(drivers) != 1:
            multiply_driven_nodes[node] = len(drivers)
            return
        if node in active_path:
            cyclic_nodes.add(node)
            return

        gate = drivers[0]
        reachable_gate_ids.add(gate.gate_id)
        next_path = active_path | {node}
        for input_node in gate.inputs:
            trace_node(input_node, next_path)

    trace_node(port.node, set())
    return OutputConeStatus(
        port=port,
        reachable_gate_ids=tuple(sorted(reachable_gate_ids)),
        undriven_nodes=tuple(sorted(undriven_nodes)),
        multiply_driven_nodes=tuple(sorted(multiply_driven_nodes.items())),
        cyclic_nodes=tuple(sorted(cyclic_nodes)),
    )


def analyze_gate_export_contract(
    chip: str,
    gates: list[Gate],
    ports: list[SignalPort],
) -> GateExportContract:
    """Evaluate whether generated HDL has deterministic exported outputs."""
    input_ports = tuple(port for port in ports if port.direction == "input")
    output_ports = tuple(port for port in ports if port.direction == "output")
    gate_errors = list(validate_gate_shapes(gates))
    input_nodes = {port.node for port in input_ports}
    driver_by_node: dict[int, list[Gate]] = defaultdict(list)
    for gate in gates:
        if len(gate.outputs) == 1 and isinstance(gate.outputs[0], int):
            driver_by_node[gate.outputs[0]].append(gate)

    output_cones = ()
    if not gate_errors:
        output_cones = tuple(
            analyze_output_cone(port, input_nodes, driver_by_node)
            for port in output_ports
        )
    reachable_gate_ids = {
        gate_id
        for cone in output_cones
        for gate_id in cone.reachable_gate_ids
    }
    if not output_ports:
        gate_errors.append("no exported output port derives from signal anchors")

    global_multiply_driven_nodes = tuple(
        sorted(
            (node, len(drivers))
            for node, drivers in driver_by_node.items()
            if len(drivers) > 1
        )
    )
    return GateExportContract(
        chip=chip,
        gate_count=len(gates),
        input_ports=input_ports,
        output_ports=output_ports,
        gate_errors=tuple(gate_errors),
        output_cones=output_cones,
        disconnected_gate_count=len(gates) - len(reachable_gate_ids),
        global_multiply_driven_nodes=global_multiply_driven_nodes,
    )


def format_nodes(nodes: tuple[int, ...] | list[int] | set[int]) -> str:
    """Format node identifiers as stable HDL names."""
    return ", ".join(f"n{node}" for node in nodes)


def contract_failure_summary(contract: GateExportContract) -> str:
    """Return the first deterministic reason an export is not deliverable."""
    if contract.gate_errors:
        return contract.gate_errors[0]
    for cone in contract.output_cones:
        if cone.undriven_nodes:
            return (
                f"output {cone.port.name} reaches undriven nodes "
                f"{format_nodes(cone.undriven_nodes)}"
            )
        if cone.multiply_driven_nodes:
            node, driver_count = cone.multiply_driven_nodes[0]
            return (
                f"output {cone.port.name} reaches n{node} with "
                f"{driver_count} drivers"
            )
        if cone.cyclic_nodes:
            return (
                f"output {cone.port.name} reaches cycle nodes "
                f"{format_nodes(cone.cyclic_nodes)}"
            )
    return "export contract is incomplete"


def print_contract(contract: GateExportContract) -> None:
    """Print an actionable structural contract report."""
    status = "PASS" if contract.is_delivery_ready else "FAIL"
    print(
        f"  Export contract: {status} "
        f"({len(contract.input_ports)} inputs, {len(contract.output_ports)} outputs, "
        f"{contract.disconnected_gate_count} disconnected gates)"
    )
    for error in contract.gate_errors:
        print(f"    Error: {error}")
    for cone in contract.output_cones:
        cone_status = "PASS" if cone.is_resolved else "FAIL"
        print(
            f"    {cone.port.name} (n{cone.port.node}): {cone_status}; "
            f"{len(cone.reachable_gate_ids)} reachable gates"
        )
        if cone.undriven_nodes:
            print(f"      Undriven nodes: {format_nodes(cone.undriven_nodes)}")
        for node, driver_count in cone.multiply_driven_nodes:
            print(f"      Multiple drivers: n{node} has {driver_count}")
        if cone.cyclic_nodes:
            print(f"      Cycle nodes: {format_nodes(cone.cyclic_nodes)}")


def generate_verilog_module(
    chip: str,
    gates: list[Gate],
    nodes: set[int],
    ports: list[SignalPort],
    contract: GateExportContract,
) -> str:
    """Generate Verilog module from gate list and pad-signal ports."""
    shape_errors = validate_gate_shapes(gates)
    if shape_errors:
        raise GateContractError(shape_errors[0])

    lines = []

    # Header comment
    lines.append("// Auto-generated from gate-level netlist")
    lines.append(f"// Chip: {chip}")
    lines.append("// Tool: gate_to_verilog_v0.py")
    lines.append("// Ports derived from netlist_v1 signal anchors (layout_node)")
    if contract.is_delivery_ready:
        lines.append("// Export contract: exported output cones resolve through declared inputs.")
    else:
        lines.append(
            "// Export contract: incomplete; "
            f"{contract_failure_summary(contract)}."
        )
    lines.append("// This artifact does not establish chip-level functional equivalence.")
    lines.append("")
    lines.append("`timescale 1ns/1ps")
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
    }

    if gate.gate_type not in gate_type_map:
        raise ValueError(f"unsupported gate type {gate.gate_type!r} for gate g{inst_id}")

    prim_name = gate_type_map[gate.gate_type]
    inst_name = f"g{inst_id}"

    # Build port connections
    connections = []

    if gate.gate_type == "INV":
        # Single input, single output
        connections.append(f".A(n{gate.inputs[0]})")
        connections.append(f".Y(n{gate.outputs[0]})")
    else:
        # NAND/NOR: multiple inputs, single output
        for i, inp in enumerate(gate.inputs):
            port_name = chr(ord("A") + i)  # A, B, C, ...
            connections.append(f".{port_name}(n{inp})")
        connections.append(f".Y(n{gate.outputs[0]})")

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
    ]
    return lines


MAX_EXHAUSTIVE_TESTBENCH_INPUTS = 12


def verilog_string(value: str) -> str:
    """Escape one literal for a generated Verilog string."""
    return value.replace("\\", "\\\\").replace('"', '\\"')


def generate_incomplete_testbench(
    module_name: str, contract: GateExportContract
) -> str:
    """Generate an explicit failing bench for incomplete retained evidence."""
    reason = verilog_string(contract_failure_summary(contract))
    return "\n".join(
        [
            "// Testbench unavailable for an incomplete gate-HDL export.",
            f"// Module: {module_name}",
            f"// Reason: {reason}",
            "",
            "`timescale 1ns/1ps",
            "",
            f"module tb_{module_name};",
            "    initial begin",
            f'        $fatal(1, "{module_name} is not delivery-ready: {reason}");',
            "    end",
            "endmodule",
            "",
        ]
    )


def generate_testbench(
    chip: str,
    module_name: str,
    ports: list[SignalPort],
    contract: GateExportContract,
) -> str:
    """Generate an exhaustive resolution test for a closed output contract.

    The oracle rejects X and Z values on every exported output after every
    binary input vector. It establishes structural determinism only; behavior
    still requires an independent vector source or hardware evidence.
    """
    del chip
    if not contract.is_delivery_ready:
        return generate_incomplete_testbench(module_name, contract)

    inputs = [port for port in ports if port.direction == "input"]
    outputs = [port for port in ports if port.direction == "output"]
    if len(inputs) > MAX_EXHAUSTIVE_TESTBENCH_INPUTS:
        raise GateContractError(
            f"{module_name} has {len(inputs)} inputs; exhaustive generated testbenches "
            f"support at most {MAX_EXHAUSTIVE_TESTBENCH_INPUTS}"
        )

    vector_count = 1 << len(inputs)
    lines = [
        "// Testbench (auto-generated)",
        f"// Module: {module_name}",
        "// Exhaustive binary input vectors with an X/Z resolution oracle.",
        "// This testbench does not establish chip-level functional equivalence.",
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
    lines.extend(
        [
            "",
            "    task require_known;",
            "        input value;",
            "        input [8*64-1:0] signal_name;",
            "        begin",
            "            if ((value !== 1'b0) && (value !== 1'b1)) begin",
            '                $display("FAIL: unresolved output %0s=%b", signal_name, value);',
            "                $fatal(1);",
            "            end",
            "        end",
            "    endtask",
            "",
            f"    {module_name} dut (",
        ]
    )
    connections = ["        .VDD(VDD)", "        .VSS(VSS)"]
    connections.extend(f"        .{port.name}({port.name})" for port in inputs + outputs)
    lines.append(",\n".join(connections))
    lines.extend(
        [
            "    );",
            "",
            "    integer vector;",
            "    initial begin",
            "        VDD = 1'b1;",
            "        VSS = 1'b0;",
            f"        for (vector = 0; vector < {vector_count}; vector = vector + 1) begin",
        ]
    )
    for input_index, port in enumerate(inputs):
        lines.append(f"            {port.name} = (vector >> {input_index}) & 1'b1;")
    lines.append("            #1;")
    for port in outputs:
        lines.append(f'            require_known({port.name}, "{port.name}");')
    lines.extend(
        [
            "        end",
            f'        $display("PASS: {module_name} resolves all {vector_count} input vectors");',
            "        $finish;",
            "    end",
            "",
            "    initial begin",
            f'        $dumpfile("tb_{module_name}.vcd");',
            f"        $dumpvars(0, tb_{module_name});",
            "    end",
            "endmodule",
            "",
        ]
    )
    return "\n".join(lines)


def atomic_write_text(path: Path, content: str) -> None:
    """Publish one generated text artifact without exposing a partial file."""
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_path = tempfile.mkstemp(
        prefix=f".{path.name}.",
        dir=path.parent,
        text=True,
    )
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8", newline="\n") as stream:
            stream.write(content)
        os.replace(temporary_path, path)
    except BaseException:
        Path(temporary_path).unlink(missing_ok=True)
        raise


def load_gates(chip: str, input_dir: Path) -> tuple[dict[str, Any], list[Gate]]:
    """Load one gates_v0 document and reject missing or malformed input."""
    gates_file = input_dir / chip / f"{chip}_gates_v0.json"
    if not gates_file.is_file():
        raise GateContractError(f"{chip}: gates file not found: {gates_file}")

    netlist = parse_gates_netlist(gates_file)
    if not isinstance(netlist, dict):
        raise GateContractError(f"{chip}: gate netlist root is not an object")
    raw_gates = netlist.get("gates")
    if not isinstance(raw_gates, list):
        raise GateContractError(f"{chip}: gates field is not a list")

    gates = []
    for gate_id, raw_gate in enumerate(raw_gates):
        if not isinstance(raw_gate, dict):
            raise GateContractError(f"{chip}: gate {gate_id} is not an object")
        gate_type = raw_gate.get("gate_type")
        inputs = raw_gate.get("inputs")
        outputs = raw_gate.get("outputs")
        if not isinstance(gate_type, str):
            raise GateContractError(f"{chip}: gate {gate_id} has no string gate_type")
        if not isinstance(inputs, list) or not isinstance(outputs, list):
            raise GateContractError(
                f"{chip}: gate {gate_id} inputs and outputs must both be lists"
            )
        gates.append(Gate(gate_type, inputs, outputs, gate_id))

    shape_errors = validate_gate_shapes(gates)
    if shape_errors:
        raise GateContractError(f"{chip}: {shape_errors[0]}")
    return netlist, gates


def write_contract_report(path: Path, contracts: list[GateExportContract]) -> None:
    """Write a deterministic contract report after every chip is analyzed."""
    content = json.dumps(
        {
            "schema_version": 1,
            "tool": "scripts/gate_to_verilog_v0.py",
            "contracts": [contract.to_dict() for contract in contracts],
        },
        indent=2,
        sort_keys=True,
    )
    atomic_write_text(path, f"{content}\n")


def render_exports(
    exports: list[tuple[str, list[Gate], set[int], list[SignalPort], GateExportContract]],
    output_dir: Path,
    include_testbenches: bool,
) -> list[tuple[Path, str]]:
    """Render every selected artifact before any destination file changes."""
    rendered_exports = []
    for chip, gates, nodes, ports, contract in exports:
        module_name = f"i{chip}_gates"
        chip_output_dir = output_dir / chip
        verilog_code = generate_verilog_module(
            chip,
            gates,
            nodes,
            ports,
            contract,
        )
        rendered_exports.append(
            (chip_output_dir / f"{module_name}.v", verilog_code)
        )
        if include_testbenches:
            testbench = generate_testbench(chip, module_name, ports, contract)
            rendered_exports.append(
                (chip_output_dir / f"tb_{module_name}.v", testbench)
            )
    return rendered_exports


def check_generated_exports(
    exports: list[tuple[str, list[Gate], set[int], list[SignalPort], GateExportContract]],
    output_dir: Path,
) -> tuple[str, ...]:
    """Return drift errors for generated module and testbench evidence files."""
    errors = []
    for output_path, expected_content in render_exports(
        exports,
        output_dir,
        include_testbenches=True,
    ):
        if not output_path.is_file():
            errors.append(f"generated artifact is missing: {output_path}")
            continue
        actual_content = output_path.read_text(encoding="utf-8")
        if actual_content != expected_content:
            errors.append(
                f"generated artifact is stale: {output_path}; "
                "run gate_to_verilog_v0.py --generate-testbench"
            )
    return tuple(errors)


def main() -> int:
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
        help="Generate a resolution-oracle testbench for each module",
    )
    parser.add_argument(
        "--check-export-contract",
        action="store_true",
        help="Check exported output cones and exit nonzero when one is incomplete",
    )
    parser.add_argument(
        "--check-generated",
        action="store_true",
        help="Check generated Verilog and testbenches against retained source evidence",
    )
    parser.add_argument(
        "--contract-report",
        type=Path,
        help="Write a JSON report for every analyzed gate export contract",
    )

    args = parser.parse_args()
    if (args.check_export_contract or args.check_generated) and args.generate_testbench:
        parser.error("check options do not generate testbenches")

    print("=== Gate-Level to Verilog Export ===")
    print("")

    exports: list[
        tuple[str, list[Gate], set[int], list[SignalPort], GateExportContract]
    ] = []
    for chip in args.chips:
        print(f"Processing {chip}...")
        try:
            netlist, gates = load_gates(chip, args.input_dir)
        except (GateContractError, OSError, json.JSONDecodeError) as error:
            parser.error(str(error))

        netlist_path = args.netlists_dir / f"{chip}_netlist_v1.json"
        if not netlist_path.is_file():
            parser.error(f"{chip}: netlist_v1 signal anchors not found: {netlist_path}")

        nodes = extract_nodes(netlist)
        driven = {output for gate in gates for output in gate.outputs}
        ports = load_signal_ports(chip, args.netlists_dir, nodes, driven)
        contract = analyze_gate_export_contract(chip, gates, ports)

        print(f"    Gates: {len(gates)}")
        print(f"    Nodes: {len(nodes)}")
        n_in = sum(1 for p in ports if p.direction == "input")
        n_out = len(ports) - n_in
        print(f"    Ports: {n_in} inputs, {n_out} outputs (from signal anchors)")
        print_contract(contract)
        exports.append((chip, gates, nodes, ports, contract))
        print("")

    contracts = [export[4] for export in exports]
    if args.contract_report:
        write_contract_report(args.contract_report, contracts)
        print(f"Wrote contract report: {args.contract_report}")

    if args.check_export_contract or args.check_generated:
        check_passed = True
        if args.check_export_contract:
            if all(contract.is_delivery_ready for contract in contracts):
                print("Gate export contract check passed")
            else:
                print("Gate export contract check failed")
                check_passed = False
        if args.check_generated:
            generated_errors = check_generated_exports(exports, args.output_dir)
            if generated_errors:
                print("Generated gate HDL check failed")
                for error in generated_errors:
                    print(f"  Error: {error}")
                check_passed = False
            else:
                print("Generated gate HDL matches retained source evidence")
        return 0 if check_passed else 1

    rendered_exports = render_exports(
        exports,
        args.output_dir,
        include_testbenches=args.generate_testbench,
    )

    for output_path, content in rendered_exports:
        atomic_write_text(output_path, content)
        print(f"  Generated: {output_path.relative_to(args.output_dir)}")

    print("Verilog export complete")
    print("")
    print("NOTE: The export contract proves structural resolution only.")
    print("Independent vectors or hardware evidence establish chip behavior.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
