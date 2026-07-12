"""Contract and simulator tests for gate-level Verilog export."""

from __future__ import annotations

import json
import shutil
import subprocess
import sys
from pathlib import Path

import pytest

import extract_python_callgraph
import gate_to_verilog_v0 as gate_to_verilog

REPO_ROOT = Path(__file__).resolve().parents[2]
IVERILOG = shutil.which("iverilog")
VVP = shutil.which("vvp")


def _port(name: str, node: int, direction: str) -> gate_to_verilog.SignalPort:
    """Create a minimal signal-anchor port for a synthetic gate graph."""
    return gate_to_verilog.SignalPort(name, node, direction, (name,))


def _complete_nand_contract() -> tuple[list[gate_to_verilog.Gate], list[gate_to_verilog.SignalPort], gate_to_verilog.GateExportContract]:
    """Build one closed two-input NAND cone for focused exporter tests."""
    gates = [gate_to_verilog.Gate("NAND", [1, 2], [3], 0)]
    ports = [
        _port("A", 1, "input"),
        _port("B", 2, "input"),
        _port("Y", 3, "output"),
    ]
    contract = gate_to_verilog.analyze_gate_export_contract("unit", gates, ports)
    return gates, ports, contract


def test_contract_accepts_closed_output_cone_and_reports_disconnected_gates() -> None:
    """A closed exported cone remains usable despite unrelated extracted gates."""
    gates, ports, contract = _complete_nand_contract()
    gates.append(gate_to_verilog.Gate("NAND", [10, 11], [12], 1))
    contract = gate_to_verilog.analyze_gate_export_contract("unit", gates, ports)

    assert contract.is_delivery_ready is True
    assert contract.disconnected_gate_count == 1
    assert contract.output_cones[0].reachable_gate_ids == (0,)


def test_python_callgraph_extracts_nested_and_module_direct_calls() -> None:
    """AST extraction preserves direct lexical calls without claiming dispatch."""
    source = """
def leaf():
    return 1

def outer():
    def nested():
        return leaf()
    nested()
    leaf()
    object.method()

outer()
"""
    callgraph = extract_python_callgraph.build_callgraph(source, "unit.py")

    assert "<module> -> outer" in callgraph
    assert "outer -> leaf" in callgraph
    assert "outer -> outer.nested" in callgraph
    assert "outer.nested -> leaf" in callgraph
    assert "outer -> [external] .method" in callgraph


def test_contract_reports_undriven_and_multiple_driver_output_cones() -> None:
    """Unbound leaves and competing drivers prevent a delivery claim."""
    undriven_gates = [gate_to_verilog.Gate("NAND", [1, 2], [3], 0)]
    undriven_contract = gate_to_verilog.analyze_gate_export_contract(
        "unit",
        undriven_gates,
        [_port("A", 1, "input"), _port("Y", 3, "output")],
    )
    assert undriven_contract.is_delivery_ready is False
    assert undriven_contract.output_cones[0].undriven_nodes == (2,)

    competing_gates = [
        gate_to_verilog.Gate("NAND", [1, 2], [3], 0),
        gate_to_verilog.Gate("NOR", [1, 2], [3], 1),
    ]
    competing_contract = gate_to_verilog.analyze_gate_export_contract(
        "unit",
        competing_gates,
        [_port("A", 1, "input"), _port("B", 2, "input"), _port("Y", 3, "output")],
    )
    assert competing_contract.is_delivery_ready is False
    assert competing_contract.output_cones[0].multiply_driven_nodes == ((3, 2),)


def test_contract_rejects_malformed_gate_shape() -> None:
    """Emitter arity and output-count constraints match the primitive library."""
    malformed_contract = gate_to_verilog.analyze_gate_export_contract(
        "unit",
        [gate_to_verilog.Gate("NAND", [1], [3, 4], 0)],
        [_port("A", 1, "input"), _port("Y", 3, "output")],
    )

    assert malformed_contract.is_delivery_ready is False
    assert "g0 NAND has 1 inputs; expected [2, 3]" in malformed_contract.gate_errors
    assert "g0 has 2 outputs; expected 1" in malformed_contract.gate_errors


def test_generated_testbenches_distinguish_closed_and_incomplete_contracts() -> None:
    """Closed models get X/Z assertions; incomplete models fail explicitly."""
    gates, ports, complete_contract = _complete_nand_contract()
    complete_testbench = gate_to_verilog.generate_testbench(
        "unit",
        "iunit_gates",
        ports,
        complete_contract,
    )
    assert "task require_known;" in complete_testbench
    assert "#1;" in complete_testbench
    assert "vector < 4" in complete_testbench

    incomplete_contract = gate_to_verilog.analyze_gate_export_contract(
        "unit",
        gates,
        [_port("A", 1, "input"), _port("Y", 3, "output")],
    )
    incomplete_testbench = gate_to_verilog.generate_testbench(
        "unit",
        "iunit_gates",
        [_port("A", 1, "input"), _port("Y", 3, "output")],
        incomplete_contract,
    )
    assert "$fatal(1" in incomplete_testbench
    assert "not delivery-ready" in incomplete_testbench


def test_generated_artifact_check_detects_missing_and_stale_outputs(tmp_path: Path) -> None:
    """Delivery preflight binds retained Verilog files to their generator inputs."""
    gates, ports, contract = _complete_nand_contract()
    exports = [("unit", gates, {1, 2, 3}, ports, contract)]

    missing_errors = gate_to_verilog.check_generated_exports(exports, tmp_path)
    assert len(missing_errors) == 2
    assert all("is missing" in error for error in missing_errors)

    for output_path, content in gate_to_verilog.render_exports(
        exports,
        tmp_path,
        include_testbenches=True,
    ):
        output_path.parent.mkdir(parents=True, exist_ok=True)
        output_path.write_text(content, encoding="utf-8")
    assert gate_to_verilog.check_generated_exports(exports, tmp_path) == ()

    module_path = tmp_path / "unit" / "iunit_gates.v"
    module_path.write_text("stale", encoding="utf-8")
    stale_errors = gate_to_verilog.check_generated_exports(exports, tmp_path)
    assert len(stale_errors) == 1
    assert "is stale" in stale_errors[0]


@pytest.mark.skipif(
    IVERILOG is None or VVP is None,
    reason="iverilog and vvp are required for the generated HDL simulator test",
)
def test_generated_hdl_compiles_and_resolves_all_synthetic_input_vectors(tmp_path: Path) -> None:
    """Icarus runs the generated X/Z oracle against a closed NAND cone."""
    assert IVERILOG is not None
    assert VVP is not None
    gates, ports, contract = _complete_nand_contract()
    module = gate_to_verilog.generate_verilog_module(
        "unit",
        gates,
        {1, 2, 3},
        ports,
        contract,
    )
    testbench = gate_to_verilog.generate_testbench(
        "unit",
        "iunit_gates",
        ports,
        contract,
    )
    module_path = tmp_path / "iunit_gates.v"
    testbench_path = tmp_path / "tb_iunit_gates.v"
    executable_path = tmp_path / "iunit"
    module_path.write_text(module, encoding="utf-8")
    testbench_path.write_text(testbench, encoding="utf-8")

    compile_result = subprocess.run(  # noqa: S603 - fixed tools and temporary test artifacts
        [
            IVERILOG,
            "-g2012",
            "-Wall",
            "-s",
            "tb_iunit_gates",
            "-o",
            str(executable_path),
            str(testbench_path),
            str(module_path),
        ],
        check=False,
        capture_output=True,
        cwd=tmp_path,
        text=True,
    )
    assert compile_result.returncode == 0, compile_result.stderr
    simulation_result = subprocess.run(  # noqa: S603 - fixed tool and temporary test artifact
        [VVP, str(executable_path)],
        check=False,
        capture_output=True,
        cwd=tmp_path,
        text=True,
    )
    assert simulation_result.returncode == 0, simulation_result.stderr
    assert "PASS: iunit_gates resolves all 4 input vectors" in simulation_result.stdout


def test_cli_rejects_missing_gate_input_before_creating_output(tmp_path: Path) -> None:
    """A missing source file no longer creates a passing placeholder module."""
    input_dir = tmp_path / "input"
    netlists_dir = tmp_path / "netlists"
    output_dir = tmp_path / "output"
    netlists_dir.mkdir()
    (netlists_dir / "unit_netlist_v1.json").write_text(
        json.dumps({"signals": []}),
        encoding="utf-8",
    )

    result = subprocess.run(  # noqa: S603 - invokes the repository-local script under test
        [
            sys.executable,
            str(REPO_ROOT / "scripts" / "gate_to_verilog_v0.py"),
            "--chips",
            "unit",
            "--input-dir",
            str(input_dir),
            "--netlists-dir",
            str(netlists_dir),
            "--output-dir",
            str(output_dir),
        ],
        check=False,
        capture_output=True,
        text=True,
    )

    assert result.returncode != 0
    assert "gates file not found" in result.stderr
    assert output_dir.exists() is False


def test_cli_rejects_missing_signal_anchors_before_creating_output(tmp_path: Path) -> None:
    """A gate graph without netlist_v1 anchors cannot create an anonymous top."""
    input_dir = tmp_path / "input"
    output_dir = tmp_path / "output"
    gates_path = input_dir / "unit" / "unit_gates_v0.json"
    gates_path.parent.mkdir(parents=True)
    gates_path.write_text(
        json.dumps(
            {
                "gates": [
                    {"gate_type": "NAND", "inputs": [1, 2], "outputs": [3]}
                ]
            }
        ),
        encoding="utf-8",
    )

    result = subprocess.run(  # noqa: S603 - invokes the repository-local script under test
        [
            sys.executable,
            str(REPO_ROOT / "scripts" / "gate_to_verilog_v0.py"),
            "--chips",
            "unit",
            "--input-dir",
            str(input_dir),
            "--netlists-dir",
            str(tmp_path / "missing-netlists"),
            "--output-dir",
            str(output_dir),
        ],
        check=False,
        capture_output=True,
        text=True,
    )

    assert result.returncode != 0
    assert "signal anchors not found" in result.stderr
    assert output_dir.exists() is False


def test_retained_gate_evidence_has_truthful_delivery_classification() -> None:
    """The only closed retained exported cone is 4003 Q4 at this evidence revision."""
    contracts = {}
    for chip in ("4001", "4002", "4003", "4004"):
        netlist, gates = gate_to_verilog.load_gates(
            chip,
            REPO_ROOT / "docs" / "evidence" / "gates_v0",
        )
        nodes = gate_to_verilog.extract_nodes(netlist)
        driven = {output for gate in gates for output in gate.outputs}
        ports = gate_to_verilog.load_signal_ports(
            chip,
            REPO_ROOT / "docs" / "evidence" / "netlists_v1",
            nodes,
            driven,
        )
        contracts[chip] = gate_to_verilog.analyze_gate_export_contract(
            chip,
            gates,
            ports,
        )

    assert contracts["4003"].is_delivery_ready is True
    assert [cone.port.name for cone in contracts["4003"].output_cones] == ["Q4"]
    assert all(
        contracts[chip].is_delivery_ready is False
        for chip in ("4001", "4002", "4004")
    )
