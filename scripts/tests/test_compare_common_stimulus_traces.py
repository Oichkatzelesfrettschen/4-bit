import json
from pathlib import Path

import pytest
from scripts.compare_common_stimulus_traces import compare, read_frames, write_report


def frame(
    model_id: str,
    phase: int,
    *,
    pc: int = 0,
    rom_control: dict | None = None,
    ram_control: dict | None = None,
) -> dict:
    signals = [
        {"path": "mcs4.phase", "value": {"kind": "bits", "width": 3, "value": phase}},
        {"path": "mcs4.cpu.pc", "value": {"kind": "bits", "width": 12, "value": pc}},
    ]
    if rom_control is not None:
        signals.append({"path": "mcs4.control.rom", "value": rom_control})
    if ram_control is not None:
        signals.append({"path": "mcs4.control.ram", "value": ram_control})
    return {
        "run_id": 2,
        "sequence": 1,
        "provenance": {
            "model_id": model_id,
            "stimulus_sha256": "a" * 64,
            "stimulus_kind": "scenario-json",
        },
        "signals": signals,
    }


def compare_frames(
    behavioral: list[dict], fpga: list[dict], *, phase_map: str = "identity"
) -> dict:
    return compare(
        behavioral,
        fpga,
        phase_map=phase_map,
        max_mismatch_observations=0,
    )


def test_compare_records_identity_agreement_and_disagreement() -> None:
    report = compare_frames(
        [frame("mcs4-behavioral", 0, pc=0)],
        [frame("mcs4-system-fpga-verilator", 0, pc=1)],
    )

    assert report["frame_count"] == 1
    assert report["matching_by_path"] == {"mcs4.phase": 1}
    assert report["mismatching_by_path"] == {"mcs4.cpu.pc": 1}
    assert report["exact_equivalence"] is False
    assert report["within_mismatch_budget"] is False
    assert report["mismatch_examples"][0]["path"] == "mcs4.cpu.pc"


def test_compare_maps_legacy_fpga_phase_to_previous_behavioral_phase() -> None:
    report = compare_frames(
        [frame("mcs4-behavioral", 7)],
        [frame("mcs4-system-fpga-verilator", 0)],
        phase_map="previous",
    )

    assert report["matching_observations"] == 2
    assert report["mismatching_observations"] == 0
    assert report["exact_equivalence"] is True
    assert report["within_mismatch_budget"] is True


def test_compare_preserves_control_selection_observables() -> None:
    unavailable = {"kind": "unavailable", "reason": "selection is inactive"}
    selected_bank_zero = {"kind": "bits", "width": 4, "value": 0}
    logic_zero = {"kind": "logic", "value": "zero"}
    logic_one = {"kind": "logic", "value": "one"}

    report = compare_frames(
        [
            frame(
                "mcs4-behavioral",
                0,
                rom_control=selected_bank_zero,
                ram_control=unavailable,
            )
        ],
        [
            frame(
                "mcs4-system-fpga-verilator",
                0,
                rom_control=logic_one,
                ram_control=logic_zero,
            )
        ],
    )

    assert report["mismatching_observations"] == 2
    assert report["mismatching_by_path"] == {
        "mcs4.control.ram": 1,
        "mcs4.control.rom": 1,
    }
    assert report["exact_equivalence"] is False
    assert report["behavioral_active_by_path"] == {"mcs4.control.rom": 1}
    assert report["fpga_active_by_path"] == {"mcs4.control.rom": 1}


def test_compare_rejects_distinct_stimuli() -> None:
    fpga = frame("mcs4-system-fpga-verilator", 0)
    fpga["provenance"]["stimulus_sha256"] = "b" * 64

    with pytest.raises(ValueError, match="exact stimulus"):
        compare_frames([frame("mcs4-behavioral", 0)], [fpga])


def test_read_and_write_report_are_replayable(tmp_path: Path) -> None:
    source = tmp_path / "trace.jsonl"
    source.write_text(json.dumps(frame("mcs4-behavioral", 0)) + "\n", encoding="utf-8")
    report_path = tmp_path / "report.json"

    frames = read_frames(source)
    write_report(report_path, compare_frames(frames, [frame("mcs4-system-fpga-verilator", 0)]))

    assert json.loads(report_path.read_text(encoding="utf-8"))["matching_observations"] == 2
