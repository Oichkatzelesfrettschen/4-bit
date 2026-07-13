#!/usr/bin/env python3
"""Capture shared-signal agreement and disagreement for one common MCS-4 stimulus."""

from __future__ import annotations

import argparse
import json
import os
import tempfile
from collections import Counter
from pathlib import Path
from typing import Any

MAX_MISMATCH_RECORDS = 32


def parse_int(value: str) -> int:
    parsed = int(value)
    if parsed < 0:
        raise argparse.ArgumentTypeError("value must be non-negative")
    return parsed


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--behavioral", type=Path, required=True)
    parser.add_argument("--fpga", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument(
        "--phase-map",
        choices=["identity", "previous"],
        default="identity",
        help="How to align behavioral and FPGA phase samples (default: identity)",
    )
    parser.add_argument(
        "--max-mismatch-observations",
        type=parse_int,
        default=0,
        help=(
            "Observed mismatches allowed before equivalence is considered failed"
            " (0 means exact agreement required)"
        ),
    )
    return parser.parse_args()


def read_frames(path: Path) -> list[dict[str, Any]]:
    frames: list[dict[str, Any]] = []
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        if not line:
            continue
        try:
            frame = json.loads(line)
        except json.JSONDecodeError as error:
            raise ValueError(f"{path}:{line_number}: invalid JSONL frame: {error}") from error
        if not isinstance(frame, dict):
            raise ValueError(f"{path}:{line_number}: frame must be an object")
        frames.append(frame)
    if not frames:
        raise ValueError(f"{path}: trace contains no frames")
    return frames


def frame_provenance(frame: dict[str, Any], label: str) -> tuple[str, str, str]:
    provenance = frame.get("provenance")
    if not isinstance(provenance, dict):
        raise ValueError(f"{label}: missing provenance object")
    stimulus_sha256 = provenance.get("stimulus_sha256")
    stimulus_kind = provenance.get("stimulus_kind")
    model_id = provenance.get("model_id")
    if not isinstance(stimulus_sha256, str) or len(stimulus_sha256) != 64:
        raise ValueError(f"{label}: missing SHA-256 stimulus identity")
    if stimulus_kind != "scenario-json":
        raise ValueError(f"{label}: stimulus representation is not scenario-json")
    if not isinstance(model_id, str) or not model_id:
        raise ValueError(f"{label}: missing model identity")
    return stimulus_sha256, stimulus_kind, model_id


def signals(frame: dict[str, Any], label: str) -> dict[str, Any]:
    raw_signals = frame.get("signals")
    if not isinstance(raw_signals, list):
        raise ValueError(f"{label}: missing signals array")
    result: dict[str, Any] = {}
    for signal in raw_signals:
        if not isinstance(signal, dict):
            raise ValueError(f"{label}: signal must be an object")
        path = signal.get("path")
        if not isinstance(path, str) or not path:
            raise ValueError(f"{label}: signal path is invalid")
        if path in result:
            raise ValueError(f"{label}: duplicate signal path {path}")
        result[path] = signal.get("value")
    return result


def control_signal_is_active(value: Any) -> bool:
    if not isinstance(value, dict):
        return False
    if value.get("kind") == "bits":
        return True
    return value.get("kind") == "logic" and value.get("value") == "one"


def normalize_phase_signal(path: str, value: Any, phase_map: str, trace_role: str) -> Any:
    if path != "mcs4.phase" or not isinstance(value, dict):
        return value
    if value.get("kind") != "bits":
        return value
    width = value.get("width")
    raw_value = value.get("value")
    if not isinstance(width, int) or not isinstance(raw_value, int):
        return value
    if width <= 0 or width > 63:
        return value
    modulo = 1 << width
    if phase_map == "identity":
        return value
    if phase_map == "previous" and trace_role == "fpga":
        normalized = (raw_value - 1) % modulo
        return {
            **value,
            "value": normalized,
        }
    return value


def normalize_signal(path: str, value: Any, phase_map: str, trace_role: str) -> Any:
    return normalize_phase_signal(
        path,
        value,
        phase_map,
        trace_role,
    )


def compare(
    behavioral: list[dict[str, Any]],
    fpga: list[dict[str, Any]],
    *,
    phase_map: str,
    max_mismatch_observations: int,
) -> dict[str, Any]:
    if len(behavioral) != len(fpga):
        raise ValueError("behavioral and FPGA traces have different frame counts")
    behavioral_identity = frame_provenance(behavioral[0], "behavioral frame 1")
    fpga_identity = frame_provenance(fpga[0], "FPGA frame 1")
    if behavioral_identity[:2] != fpga_identity[:2]:
        raise ValueError("behavioral and FPGA traces do not share one exact stimulus")

    matching = Counter()
    mismatching = Counter()
    behavioral_active_by_path = Counter()
    fpga_active_by_path = Counter()
    mismatching_observations = []
    shared_paths: set[str] = set()
    mismatching_frames = 0
    for index, (behavioral_frame, fpga_frame) in enumerate(
        zip(behavioral, fpga, strict=True), start=1
    ):
        for label, frame, identity in (
            (f"behavioral frame {index}", behavioral_frame, behavioral_identity),
            (f"FPGA frame {index}", fpga_frame, fpga_identity),
        ):
            actual_identity = frame_provenance(frame, label)
            if actual_identity[:2] != identity[:2]:
                raise ValueError(f"{label}: stimulus identity changes within one trace")
        if behavioral_frame.get("run_id") != fpga_frame.get("run_id"):
            raise ValueError(f"frame {index}: run identity differs")
        if behavioral_frame.get("sequence") != fpga_frame.get("sequence"):
            raise ValueError(f"frame {index}: sequence differs")
        behavioral_signals = signals(behavioral_frame, f"behavioral frame {index}")
        fpga_signals = signals(fpga_frame, f"FPGA frame {index}")
        frame_has_mismatch = False
        for path in sorted(behavioral_signals.keys() & fpga_signals.keys()):
            shared_paths.add(path)
            if path in {"mcs4.control.rom", "mcs4.control.ram"}:
                if control_signal_is_active(behavioral_signals[path]):
                    behavioral_active_by_path[path] += 1
                if control_signal_is_active(fpga_signals[path]):
                    fpga_active_by_path[path] += 1
            behavioral_value = normalize_signal(
                path,
                behavioral_signals[path],
                phase_map,
                "behavioral",
            )
            fpga_value = normalize_signal(path, fpga_signals[path], phase_map, "fpga")
            if behavioral_value == fpga_value:
                matching[path] += 1
            else:
                mismatching[path] += 1
                frame_has_mismatch = True
                if len(mismatching_observations) < MAX_MISMATCH_RECORDS:
                    mismatching_observations.append(
                        {
                            "frame": index,
                            "path": path,
                            "behavioral": behavioral_signals[path],
                            "fpga": fpga_signals[path],
                            "behavioral_normalized": behavioral_value,
                            "fpga_normalized": fpga_value,
                        }
                    )
        if frame_has_mismatch:
            mismatching_frames += 1
    if not shared_paths:
        raise ValueError("traces expose no common hierarchy-qualified signals")
    matching_count = sum(matching.values())
    mismatching_count = sum(mismatching.values())
    return {
        "schema_version": 1,
        "contract": "mcs4-common-stimulus",
        "stimulus_sha256": behavioral_identity[0],
        "stimulus_kind": behavioral_identity[1],
        "behavioral_model": behavioral_identity[2],
        "fpga_model": fpga_identity[2],
        "frame_count": len(behavioral),
        "shared_paths": sorted(shared_paths),
        "matching_observations": matching_count,
        "mismatching_observations": mismatching_count,
        "mismatching_frames": mismatching_frames,
        "mismatch_examples": mismatching_observations,
        "phase_map": phase_map,
        "max_mismatch_observations": max_mismatch_observations,
        "matching_by_path": dict(sorted(matching.items())),
        "mismatching_by_path": dict(sorted(mismatching.items())),
        "behavioral_active_by_path": dict(sorted(behavioral_active_by_path.items())),
        "fpga_active_by_path": dict(sorted(fpga_active_by_path.items())),
        "exact_equivalence": mismatching_count == 0,
        "within_mismatch_budget": mismatching_count <= max_mismatch_observations,
    }


def write_report(path: Path, report: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    encoded = (json.dumps(report, indent=2, sort_keys=True) + "\n").encode("utf-8")
    with tempfile.NamedTemporaryFile(
        dir=path.parent, prefix=f".{path.name}.", delete=False
    ) as output:
        temporary_path = Path(output.name)
        output.write(encoded)
        output.flush()
        os.fsync(output.fileno())
    try:
        os.replace(temporary_path, path)
    except BaseException:
        temporary_path.unlink(missing_ok=True)
        raise


def main() -> int:
    arguments = parse_arguments()
    try:
        report = compare(
            read_frames(arguments.behavioral),
            read_frames(arguments.fpga),
            phase_map=arguments.phase_map,
            max_mismatch_observations=arguments.max_mismatch_observations,
        )
        write_report(arguments.report, report)
    except (OSError, ValueError) as error:
        raise SystemExit(f"compare_common_stimulus_traces: {error}") from error
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
