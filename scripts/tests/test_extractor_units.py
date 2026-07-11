"""Unit smoke tests for the top extractor families.

Each major extraction script family gets at least one fast test against its
pure helpers with tiny synthetic inputs: no mask bitmaps, no evidence files,
no OCR engine invocations. Covered families: extract_netlist_v0 (union-find
stitching + signals parsing), extract_gates_v1 (rail identification +
channel incidence), build_netlist_v1_v0 (JSON access helpers), and
ocr_signal_labels (name normalization + match scoring + bbox geometry).
"""

from __future__ import annotations

from pathlib import Path

import numpy as np

import build_netlist_v1_v0
import extract_gates_v1
import extract_netlist_v0
import ocr_signal_labels

# ---------------------------------------------------------------------------
# extract_netlist_v0: connected components + via/contact union-find stitching
# ---------------------------------------------------------------------------


def test_dsu_union_find_stitches_components() -> None:
    """DSU merges chained elements into one root and reports no-op unions."""
    dsu = extract_netlist_v0.DSU(6)
    assert dsu.union(0, 1) is True
    assert dsu.union(1, 2) is True
    # Re-union of already-connected elements reports False (no new merge).
    assert dsu.union(0, 2) is False
    assert dsu.find(0) == dsu.find(2)
    # Untouched elements stay in their own singleton sets.
    assert dsu.find(3) == 3
    assert dsu.find(4) != dsu.find(0)


def test_parse_signals_txt_skips_comments_and_malformed(tmp_path: Path) -> None:
    """signals.txt parsing keeps only well-formed `x, y, name` triples."""
    signals_file = tmp_path / "signals.txt"
    signals_file.write_text(
        "; comment line\n\n100, 200, VDD\nnot-an-int, 5, BAD\n300,400,CLK1\n1, 2\n",
        encoding="utf-8",
    )
    parsed = extract_netlist_v0.parse_signals_txt(signals_file)
    assert parsed == [
        {"x": 100, "y": 200, "name": "VDD"},
        {"x": 300, "y": 400, "name": "CLK1"},
    ]


def test_mode_label_ignores_background_and_breaks_ties() -> None:
    """Mode label skips label 0 (background) and ties break to smaller id."""
    labels = np.array([[0, 0, 2], [2, 3, 3]], dtype=np.int32)
    # Labels 2 and 3 each appear twice: deterministic tie-break picks 2.
    assert extract_netlist_v0._mode_label(labels) == 2
    assert extract_netlist_v0._mode_label(np.zeros((2, 2), dtype=np.int32)) is None
    assert extract_netlist_v0._top_k_labels(labels, 2) == [2, 3]


# ---------------------------------------------------------------------------
# extract_gates_v1: rail identification + channel incidence
# ---------------------------------------------------------------------------


def _t(tid: int, gate: int, a: int, b: int) -> extract_gates_v1.Transistor:
    return extract_gates_v1.Transistor(id=tid, gate=gate, a=a, b=b)


def test_channel_incidence_counts_both_terminals() -> None:
    """Each transistor contributes one count to each of its channel nodes."""
    incidence = extract_gates_v1.channel_incidence([_t(0, 9, 1, 2), _t(1, 9, 2, 3)])
    assert incidence[1] == 1
    assert incidence[2] == 2
    assert incidence[3] == 1


def test_identify_rails_prefers_named_signal_anchors() -> None:
    """VDD/VSS anchors on channel-connected nodes resolve rails by name."""
    transistors = [_t(0, 10, 1, 5), _t(1, 11, 2, 6)]
    signals = [
        {"name": "VDD", "layout_node": 1},
        {"name": "VSS", "layout_node": 2},
        {"name": "CLK1", "layout_node": 5},
    ]
    rails, method, detail = extract_gates_v1.identify_rails(transistors, signals, {})
    assert rails == {1, 2}
    assert method == "signals"
    assert set(detail["signal_rails"]) == {"VDD", "VSS"}
    # The clock anchor node is excluded from incidence-promoted rails.
    assert detail["excluded_non_rail_anchor_nodes"] == [5]


def test_identify_rails_incidence_fallback_respects_floor() -> None:
    """Without anchors, incidence promotes rails only at RAIL_INCIDENCE_FLOOR."""
    floor = extract_gates_v1.RAIL_INCIDENCE_FLOOR
    hub = [_t(i, 100 + i, 0, 10 + i) for i in range(floor)]
    rails, method, _ = extract_gates_v1.identify_rails(hub, [], {})
    assert rails == {0}
    assert method == "incidence"

    below = hub[:-1]
    rails, method, _ = extract_gates_v1.identify_rails(below, [], {})
    assert rails == set()
    assert method == "none"


# ---------------------------------------------------------------------------
# build_netlist_v1_v0: JSON access + geometry helpers
# ---------------------------------------------------------------------------


def test_nested_get_and_bbox_area_helpers() -> None:
    """_get walks nested dicts safely; _bbox_area clamps degenerate boxes."""
    doc = {"devices": {"transistors": [1, 2, 3]}}
    assert build_netlist_v1_v0._get(doc, "devices", "transistors") == [1, 2, 3]
    assert build_netlist_v1_v0._get(doc, "devices", "missing") is None
    assert build_netlist_v1_v0._get("not-a-dict", "devices") is None

    assert build_netlist_v1_v0._bbox_area({"w": 4, "h": 5}) == 20
    assert build_netlist_v1_v0._bbox_area({"w": -3, "h": 5}) == 0
    assert build_netlist_v1_v0._bbox_area(None) == 0


# ---------------------------------------------------------------------------
# ocr_signal_labels: normalization, classification, scoring, bbox geometry
# ---------------------------------------------------------------------------


def test_normalize_name_and_classify_expected() -> None:
    """Normalization uppercases, strips noise; expressions are classified out."""
    assert ocr_signal_labels.normalize_name("  vdd \n") == "VDD"
    assert ocr_signal_labels.normalize_name("clk 1!") == "CLK1"
    assert ocr_signal_labels.classify_expected("VDD") == "label"
    assert ocr_signal_labels.classify_expected("(~POC)CLK2") == "expression"
    # Over-long tokens count as expressions even without operators.
    assert ocr_signal_labels.classify_expected("X" * 25) == "expression"


def test_score_match_and_union_bbox() -> None:
    """Match scoring is 1.0 on identity, 0.0 on one-sided empty; bbox union
    covers both inputs."""
    assert ocr_signal_labels.score_match("VDD", "VDD") == 1.0
    assert ocr_signal_labels.score_match("VDD", "") == 0.0
    assert ocr_signal_labels.score_match("", "") == 1.0
    assert 0.0 < ocr_signal_labels.score_match("CLK1", "CLK2") < 1.0

    merged = ocr_signal_labels.union_bbox(
        {"x": 0, "y": 0, "w": 2, "h": 2},
        {"x": 5, "y": 1, "w": 3, "h": 4},
    )
    assert merged == {"x": 0, "y": 0, "w": 8, "h": 5}
