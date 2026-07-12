#!/usr/bin/env python3
"""Export every supported HDL module and validate it with local tools."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
BEHAVIORAL_CHIPS = (
    "4004",
    "4001",
    "4002",
    "4003",
    "4008",
    "4009",
    "3216",
    "3226",
    "3205",
    "3404",
    "2101",
    "4040",
    "4101",
    "4201",
    "4289",
    "4308",
    "4207",
    "4209",
    "4211",
    "4265",
    "4316",
    "4702",
)
FPGA_CHIPS = (
    "4004",
    "4001",
    "4002",
    "4003",
    "3205",
    "4101",
    "4040",
    "4308",
    "4265",
    "4702",
    "4316",
    "4269",
    "2102",
    "1302",
    "2316",
)
CHIPS_BY_FLAVOR = {
    "behavioral": BEHAVIORAL_CHIPS,
    "fpga": FPGA_CHIPS,
}
LINT_EXCEPTION_PATH = REPO_ROOT / "docs" / "evidence" / "hdl_lint_exceptions.json"
GOWIN_DIRECTORY = REPO_ROOT / "mcs4-emu" / "crates" / "mcs4-fpga" / "gowin"
WARNING_PATTERN = re.compile(r"%Warning-([A-Z0-9_]+):")


@dataclass(frozen=True)
class ExportedModule:
    flavor: str
    chip: str
    top_module: str
    source_path: Path
    manifest_path: Path


def expected_module_name(flavor: str, chip: str) -> str:
    """Return the generated module name for one typed export request."""
    prefix = f"i{chip}"
    return prefix if flavor == "behavioral" else f"{prefix}_fpga"


def parse_warning_codes(output: str) -> set[str]:
    """Return the stable Verilator warning codes present in one diagnostic stream."""
    return set(WARNING_PATTERN.findall(output))


def load_warning_allowlist(path: Path = LINT_EXCEPTION_PATH) -> dict[tuple[str, str], frozenset[str]]:
    """Load exact Verilator exceptions with durable evidence and tracking fields."""
    document = json.loads(path.read_text(encoding="utf-8"))
    if document.get("schema_version") != 1 or document.get("tool") != "Verilator":
        raise RuntimeError(f"invalid HDL lint exception schema: {path}")

    allowlist = {}
    for entry in document.get("exceptions", []):
        flavor = entry.get("flavor")
        module = entry.get("module")
        warning_codes = entry.get("warning_codes")
        if flavor not in CHIPS_BY_FLAVOR or not isinstance(module, str):
            raise RuntimeError(f"invalid HDL lint exception target: {entry!r}")
        if not isinstance(warning_codes, list) or not warning_codes:
            raise RuntimeError(f"invalid HDL lint exception warnings: {entry!r}")
        if not all(isinstance(code, str) and code.isupper() for code in warning_codes):
            raise RuntimeError(f"invalid HDL lint warning code: {entry!r}")
        if not all(isinstance(entry.get(field), str) and entry[field] for field in ("reason", "tracking", "evidence_state")):
            raise RuntimeError(f"missing HDL lint exception evidence: {entry!r}")
        key = (flavor, module)
        if key in allowlist:
            raise RuntimeError(f"duplicate HDL lint exception: {flavor} {module}")
        allowlist[key] = frozenset(warning_codes)
    return allowlist


def expected_module_keys() -> set[tuple[str, str]]:
    """Return every module key that this validator emits."""
    return {
        (flavor, expected_module_name(flavor, chip))
        for flavor, chips in CHIPS_BY_FLAVOR.items()
        for chip in chips
    }


def unexpected_warning_codes(
    flavor: str,
    top_module: str,
    output: str,
    warning_allowlist: dict[tuple[str, str], frozenset[str]],
) -> set[str]:
    """Return warning codes not explicitly accepted for this module contract."""
    allowed = warning_allowlist.get((flavor, top_module), frozenset())
    return parse_warning_codes(output) - allowed


def missing_warning_codes(
    flavor: str,
    top_module: str,
    output: str,
    warning_allowlist: dict[tuple[str, str], frozenset[str]],
) -> set[str]:
    """Return stale exception codes absent from the current tool diagnostic."""
    allowed = warning_allowlist.get((flavor, top_module), frozenset())
    return allowed - parse_warning_codes(output)


def require_tool(name: str) -> str:
    """Resolve one required executable or stop with an actionable message."""
    resolved = shutil.which(name)
    if resolved is None:
        raise RuntimeError(f"required tool is not on PATH: {name}")
    return resolved


def run_checked(command: list[str], *, cwd: Path) -> subprocess.CompletedProcess[str]:
    """Run one local command and expose complete diagnostics on failure."""
    result = subprocess.run(  # noqa: S603 - fixed local tools execute without a shell
        command,
        cwd=cwd,
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        command_text = " ".join(command)
        raise RuntimeError(
            f"command failed ({result.returncode}): {command_text}\n"
            f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}"
        )
    return result


def export_module(binary: str, flavor: str, chip: str, output_dir: Path) -> ExportedModule:
    """Write one typed module and verify that its provenance manifest matches."""
    top_module = expected_module_name(flavor, chip)
    source_path = output_dir / f"{top_module}.v"
    manifest_path = Path(f"{source_path}.manifest.json")
    run_checked(
        [
            binary,
            "--chip",
            chip,
            "--flavor",
            flavor,
            "--output",
            str(source_path),
        ],
        cwd=REPO_ROOT,
    )
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    digest = hashlib.sha256(source_path.read_bytes()).hexdigest()
    if manifest["output_sha256"] != digest:
        raise RuntimeError(f"manifest hash mismatch for {source_path}")
    if manifest["request"] != {"chip": f"i{chip}", "flavor": flavor}:
        raise RuntimeError(f"manifest request mismatch for {source_path}")
    return ExportedModule(flavor, chip, top_module, source_path, manifest_path)


def validate_module(
    iverilog: str,
    verilator: str,
    artifact: ExportedModule,
    warning_allowlist: dict[tuple[str, str], frozenset[str]],
) -> None:
    """Compile and lint one isolated generated module."""
    run_checked(
        [
            iverilog,
            "-g2012",
            "-Wall",
            "-s",
            artifact.top_module,
            "-tnull",
            str(artifact.source_path),
        ],
        cwd=REPO_ROOT,
    )
    result = run_checked(
        [
            verilator,
            "--lint-only",
            "--Wall",
            "--Wno-fatal",
            "--top-module",
            artifact.top_module,
            str(artifact.source_path),
        ],
        cwd=REPO_ROOT,
    )
    unexpected = unexpected_warning_codes(
        artifact.flavor,
        artifact.top_module,
        result.stderr,
        warning_allowlist,
    )
    missing = missing_warning_codes(
        artifact.flavor,
        artifact.top_module,
        result.stderr,
        warning_allowlist,
    )
    if unexpected or missing:
        details = []
        if unexpected:
            details.append(f"unapproved={','.join(sorted(unexpected))}")
        if missing:
            details.append(f"stale={','.join(sorted(missing))}")
        raise RuntimeError(
            f"Verilator warning contract failed for {artifact.top_module}: {'; '.join(details)}\n{result.stderr}"
        )


def validate_gowin_system(
    iverilog: str,
    verilator: str,
    artifacts: list[ExportedModule],
    output_dir: Path,
) -> None:
    """Lint and simulate the shared Gowin system against fresh FPGA exports."""
    vvp = require_tool("vvp")
    fpga_sources = {
        artifact.top_module: artifact.source_path
        for artifact in artifacts
        if artifact.flavor == "fpga"
    }
    required_modules = ("i4004_fpga", "i4001_fpga", "i4002_fpga")
    missing_modules = [module for module in required_modules if module not in fpga_sources]
    if missing_modules:
        raise RuntimeError(f"missing fresh FPGA export(s): {', '.join(missing_modules)}")

    shared_sources = [
        GOWIN_DIRECTORY / "mcs4_system_core.v",
        GOWIN_DIRECTORY / "mcs4_top.v",
        GOWIN_DIRECTORY / "mcs4_system_sim_top.v",
        GOWIN_DIRECTORY / "clock_gen.v",
        GOWIN_DIRECTORY / "uart_hw.v",
        GOWIN_DIRECTORY / "uart_bridge.v",
        GOWIN_DIRECTORY / "rom_bsram.v",
        GOWIN_DIRECTORY / "ram_bsram.v",
    ]
    missing_sources = [path for path in shared_sources if not path.is_file()]
    if missing_sources:
        rendered = ", ".join(str(path) for path in missing_sources)
        raise RuntimeError(f"missing Gowin system source(s): {rendered}")

    system_sources = [*shared_sources, *(fpga_sources[module] for module in required_modules)]
    lint_sources = [
        GOWIN_DIRECTORY / "mcs4_system_core.v",
        GOWIN_DIRECTORY / "mcs4_top.v",
        GOWIN_DIRECTORY / "clock_gen.v",
        GOWIN_DIRECTORY / "uart_hw.v",
        GOWIN_DIRECTORY / "uart_bridge.v",
        GOWIN_DIRECTORY / "rom_bsram.v",
        GOWIN_DIRECTORY / "ram_bsram.v",
        *(fpga_sources[module] for module in required_modules),
    ]
    lint = run_checked(
        [
            verilator,
            "--lint-only",
            "--Wall",
            "--top-module",
            "mcs4_top",
            *(str(path) for path in lint_sources),
        ],
        cwd=REPO_ROOT,
    )
    warnings = parse_warning_codes(lint.stderr)
    if warnings:
        raise RuntimeError(
            f"Gowin system Verilator contract has warning(s): {','.join(sorted(warnings))}\n{lint.stderr}"
        )

    monitor_image = output_dir / "monitor_rom.hex"
    shutil.copyfile(GOWIN_DIRECTORY / "monitor_rom.hex", monitor_image)
    executable = output_dir / "tb_mcs4_system"
    compile = run_checked(
        [
            iverilog,
            "-g2012",
            "-Wall",
            "-Wno-timescale",
            "-s",
            "tb_mcs4_system",
            "-o",
            str(executable),
            str(GOWIN_DIRECTORY / "tb_mcs4_system.v"),
            *(str(path) for path in system_sources),
        ],
        cwd=REPO_ROOT,
    )
    if compile.stderr.strip():
        raise RuntimeError(f"Gowin system Icarus contract has diagnostics:\n{compile.stderr}")
    simulation = run_checked([vvp, str(executable)], cwd=output_dir)
    if "SYSTEM_TEST_PASS" not in simulation.stdout:
        raise RuntimeError(f"Gowin system simulation omitted its success marker:\n{simulation.stdout}")


def validate(output_dir: Path) -> list[ExportedModule]:
    """Build the exporter and validate every supported module in a clean directory."""
    iverilog = require_tool("iverilog")
    verilator = require_tool("verilator")
    warning_allowlist = load_warning_allowlist()
    unknown_entries = set(warning_allowlist) - expected_module_keys()
    if unknown_entries:
        rendered = ", ".join(f"{flavor}/{module}" for flavor, module in sorted(unknown_entries))
        raise RuntimeError(f"HDL lint exception has no generated module: {rendered}")
    run_checked(
        ["cargo", "build", "--locked", "-p", "mcs4-fpga", "--bin", "mcs4-fpga-export"],
        cwd=REPO_ROOT,
    )
    binary = REPO_ROOT / "target" / "debug" / "mcs4-fpga-export"
    if not binary.is_file():
        raise RuntimeError(f"exporter binary is missing after build: {binary}")

    artifacts = []
    for flavor, chips in CHIPS_BY_FLAVOR.items():
        flavor_dir = output_dir / flavor
        for chip in chips:
            artifact = export_module(str(binary), flavor, chip, flavor_dir)
            validate_module(iverilog, verilator, artifact, warning_allowlist)
            artifacts.append(artifact)
    validate_gowin_system(iverilog, verilator, artifacts, output_dir)
    return artifacts


def parse_arguments() -> argparse.Namespace:
    """Parse local-only verifier arguments."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--output-dir",
        type=Path,
        help="retained generated HDL directory; default uses a temporary directory",
    )
    return parser.parse_args()


def main() -> int:
    """Run validation and report the exact generated module count."""
    arguments = parse_arguments()
    if arguments.output_dir is None:
        with tempfile.TemporaryDirectory(prefix="mcs4-hdl-") as temporary:
            artifacts = validate(Path(temporary))
    else:
        arguments.output_dir.mkdir(parents=True, exist_ok=True)
        artifacts = validate(arguments.output_dir)
    print(f"Validated {len(artifacts)} generated HDL modules.")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except RuntimeError as error:
        print(f"verify_hdl_exports: {error}", file=sys.stderr)
        raise SystemExit(1) from error
