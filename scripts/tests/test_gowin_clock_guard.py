"""Verify that Gowin programming refuses unreviewed board clock state."""

from __future__ import annotations

import hashlib
import json
import shutil
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
FPGA_DIRECTORY = ROOT / "mcs4-emu" / "crates" / "mcs4-fpga"
CLOCK_VALIDATOR = ROOT / "scripts" / "verify_gowin_clock_contract.py"


def make_binary() -> str:
    """Resolve the host Make executable before invoking the fixed target."""
    executable = shutil.which("make")
    if executable is None:
        raise RuntimeError("make is required for Gowin clock guard tests")
    return str(Path(executable).resolve())


def run_make(target: str, *variables: str) -> subprocess.CompletedProcess[str]:
    """Run one fixed Make target without invoking a vendor or programmer tool."""
    return subprocess.run(  # noqa: S603 - fixed repository-local Makefile target
        [make_binary(), "-C", str(FPGA_DIRECTORY), target, *variables],
        check=False,
        capture_output=True,
        text=True,
    )


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_clock_contract_inputs(tmp_path: Path) -> dict[str, Path]:
    source = tmp_path / "source.v"
    source.write_text("module source; endmodule\n", encoding="ascii")
    constraints = tmp_path / "mcs4_gowin.cst"
    constraints.write_text(
        'IO_LOC "sys_clk_in" 19;\nIO_PORT "sys_clk_in" IO_TYPE=LVCMOS33;\n',
        encoding="ascii",
    )
    evidence = tmp_path / "clock-evidence.json"
    evidence.write_text(
        json.dumps(
            {
                "schema_version": 1,
                "target": {
                    "device": "GW1N-LV2LQ100C6/I5",
                    "top_module": "mcs4_top",
                    "board": "test-board",
                    "revision": "test-revision",
                },
                "review": {
                    "record_id": "test-clock-route",
                    "reviewer": "test-reviewer",
                    "reviewed_at": "2026-07-11",
                },
                "clock": {
                    "port": "sys_clk_in",
                    "io_loc": "19",
                    "io_type": "LVCMOS33",
                    "frequency_hz": 27_000_000,
                    "measured_frequency_hz": 27_000_000,
                    "timing_frequency_hz": 27_000_000,
                    "duty_cycle_percent": 50,
                    "measurement_method": "bench-counter",
                },
                "artifacts": {"constraints_sha256": sha256(constraints)},
                "source_files": [{"path": "source.v", "sha256": sha256(source)}],
            },
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )
    return {
        "source": source,
        "constraints": constraints,
        "evidence": evidence,
        "timing_constraints": tmp_path / "mcs4_sys_clk.sdc",
        "contract": tmp_path / "gowin_clock_contract.json",
        "report": tmp_path / "mcs4_system.rpt.txt",
        "timing_paths": tmp_path / "mcs4_system.timing_paths",
        "bitstream": tmp_path / "mcs4_system.fs",
        "build_evidence": tmp_path / "gowin_build_evidence.json",
    }


def run_clock_validator(tmp_path: Path, stage: str, paths: dict[str, Path]) -> subprocess.CompletedProcess[str]:
    command = [
        "python3",
        str(CLOCK_VALIDATOR),
        "--stage",
        stage,
        "--evidence",
        paths["evidence"].name,
        "--constraints",
        paths["constraints"].name,
        "--expected-device",
        "GW1N-LV2LQ100C6/I5",
        "--expected-top-module",
        "mcs4_top",
        "--timing-constraints",
        paths["timing_constraints"].name,
        "--source",
        paths["source"].name,
    ]
    if stage in {"source", "programming"}:
        command.extend(["--contract", paths["contract"].name])
    if stage == "programming":
        command.extend(
            [
                "--synthesis-report",
                paths["report"].name,
                "--timing-paths",
                paths["timing_paths"].name,
                "--bitstream",
                paths["bitstream"].name,
                "--build-evidence",
                paths["build_evidence"].name,
            ]
        )
    return subprocess.run(  # noqa: S603 - fixed validator with test-local arguments
        command, cwd=tmp_path, check=False, capture_output=True, text=True
    )


def test_clock_guard_rejects_missing_evidence_before_synthesis() -> None:
    """Programming stops before any synthesis or programmer command without evidence."""
    result = run_make("gowin_prog")

    assert result.returncode != 0
    assert "GOWIN_CLOCK_EVIDENCE must name a reviewed sys_clk_in board-route record" in result.stdout
    assert "Generating FPGA Verilog" not in result.stdout
    assert "openFPGALoader" not in result.stdout


def test_clock_guard_requires_matching_constraint_after_evidence(tmp_path: Path) -> None:
    """A descriptive record alone cannot bypass the unassigned clock constraint."""
    evidence = tmp_path / "clock-route.json"
    evidence.write_text(
        json.dumps(
            {
                "schema_version": 1,
                "target": {
                    "device": "GW1N-LV2LQ100C6/I5",
                    "top_module": "mcs4_top",
                    "board": "test-board",
                    "revision": "test-revision",
                },
                "review": {
                    "record_id": "test-clock-route",
                    "reviewer": "test-reviewer",
                    "reviewed_at": "2026-07-11",
                },
                "clock": {
                    "port": "sys_clk_in",
                    "io_loc": "19",
                    "io_type": "LVCMOS33",
                    "frequency_hz": 27_000_000,
                    "measured_frequency_hz": 27_000_000,
                    "timing_frequency_hz": 27_000_000,
                    "duty_cycle_percent": 50,
                    "measurement_method": "bench-counter",
                },
                "artifacts": {
                    "constraints_sha256": sha256(FPGA_DIRECTORY / "constraints" / "mcs4_gowin.cst")
                },
                "source_files": [
                    {"path": "unavailable.v", "sha256": "0" * 64}
                ],
            }
        )
        + "\n",
        encoding="utf-8",
    )

    result = run_make("gowin_clock_guard", f"GOWIN_CLOCK_EVIDENCE={evidence}")

    assert result.returncode != 0
    assert "constraints do not assign sys_clk_in" in result.stderr


def test_clock_contract_generates_exact_sdc_and_records_post_synthesis_hashes(tmp_path: Path) -> None:
    """A deployment contract binds the reviewed route to source, SDC, report, and bitstream bytes."""
    paths = write_clock_contract_inputs(tmp_path)

    preflight = run_clock_validator(tmp_path, "preflight", paths)
    assert preflight.returncode == 0, preflight.stderr
    assert paths["timing_constraints"].read_text(encoding="ascii") == (
        "# Generated from the reviewed Gowin board-clock contract.\n"
        "create_clock -name sys_clk_in -period 37.037037037 -waveform {0 18.518518519} "
        "[get_ports {sys_clk_in}]\n"
    )

    source = run_clock_validator(tmp_path, "source", paths)
    assert source.returncode == 0, source.stderr
    paths["report"].write_text(
        "<Timing Constraints File>: mcs4_sys_clk.sdc\n"
        "sys_clk_in | - | 19/3 | Y | in | IOL16[A] | - | LVCMOS33 | NA\n",
        encoding="ascii",
    )
    paths["timing_paths"].write_text("=====\nSETUP\n0.250\n", encoding="ascii")
    paths["bitstream"].write_bytes(b"bitstream")

    programming = run_clock_validator(tmp_path, "programming", paths)
    assert programming.returncode == 0, programming.stderr
    build_evidence = json.loads(paths["build_evidence"].read_text(encoding="utf-8"))
    assert build_evidence["bitstream_sha256"] == sha256(paths["bitstream"])
    assert build_evidence["synthesis_report_sha256"] == sha256(paths["report"])


def test_clock_contract_rejects_report_without_active_timing_constraints(tmp_path: Path) -> None:
    """A PnR report that omits an active SDC cannot authorize programming."""
    paths = write_clock_contract_inputs(tmp_path)
    assert run_clock_validator(tmp_path, "preflight", paths).returncode == 0
    assert run_clock_validator(tmp_path, "source", paths).returncode == 0
    paths["report"].write_text("<Timing Constraints File>: ---\n", encoding="ascii")
    paths["timing_paths"].write_text("=====\nSETUP\n0.250\n", encoding="ascii")
    paths["bitstream"].write_bytes(b"bitstream")

    programming = run_clock_validator(tmp_path, "programming", paths)

    assert programming.returncode != 0
    assert "does not identify active timing constraints" in programming.stderr
