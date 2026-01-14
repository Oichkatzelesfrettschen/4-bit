#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def _read_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def _overlaps(a: dict, b: dict) -> bool:
    return not (a["x1"] <= b["x0"] or a["x0"] >= b["x1"] or a["y1"] <= b["y0"] or a["y0"] >= b["y1"])


def _auto_signal_from_edge_token(chip: str, tok: str) -> tuple[str, str, str]:
    """Return (pin_dip, signal, confidence) for unambiguous edge tokens.

    This intentionally only covers conservative, high-signal mappings where the token itself
    encodes the signal identity (e.g. TEST/SYNC, or D0..D3). Everything else remains blank
    for human verification.
    """

    tok_norm = tok.strip().upper()
    if not tok_norm:
        return ("", "", "")

    # Common bus pads, when they exist as explicit pad labels on the mask.
    if tok_norm in {"D0", "D1", "D2", "D3"}:
        pin = {"D0": "1", "D1": "2", "D2": "3", "D3": "4"}[tok_norm]
        return (pin, f"{tok_norm}_PAD", "0.9")

    # Chip-specific external control labels.
    if chip == "4004":
        if tok_norm == "T":
            return ("10", "TEST", "0.85")
        if tok_norm == "S":
            return ("8", "SYNC", "0.80")
        if tok_norm in {"RM", "CM"}:
            return ("11", "CMROM", "0.75")
        if tok_norm in {"R0", "R1", "R2", "R3"}:
            pin = {"R0": "13", "R1": "14", "R2": "15", "R3": "16"}[tok_norm]
            return (pin, f"CMRAM{tok_norm[-1]}", "0.75")

    if chip == "4002":
        if tok_norm == "S":
            return ("8", "SYNC", "0.75")
        if tok_norm in {"CM", "C"}:
            return ("11", "CM", "0.70")
        if tok_norm in {"PO", "P0"}:
            return ("10", "P0", "0.70")

    if chip == "4001":
        if tok_norm == "S":
            return ("8", "SYNC", "0.75")
        if tok_norm in {"CM", "C"}:
            return ("11", "CM", "0.65")

    return ("", "", "")


def _pin_for_signal(chip: str, signal: str) -> str:
    s = signal.strip().upper()
    if not s:
        return ""

    # Normalize common suffix.
    if s in {"D0_PAD", "D1_PAD", "D2_PAD", "D3_PAD"}:
        return {"D0_PAD": "1", "D1_PAD": "2", "D2_PAD": "3", "D3_PAD": "4"}[s]

    if chip == "4001":
        return {
            "SYNC": "8",
            "RESET": "9",
            "CL": "10",
            "CM": "11",
        }.get(s, "")

    if chip == "4002":
        return {
            "SYNC": "8",
            "RESET": "9",
            "P0": "10",
            "CM": "11",
        }.get(s, "")

    if chip == "4003":
        return {
            "CLOCK": "1",
            "DATA": "2",
            "EN": "16",
            "OUT": "14",
        }.get(s, "")

    if chip == "4004":
        return {
            "SYNC": "8",
            "RESET": "9",
            "TEST": "10",
            "CMROM": "11",
            "CMRAM0": "13",
            "CMRAM1": "14",
            "CMRAM2": "15",
            "CMRAM3": "16",
        }.get(s, "")

    return ""


def main() -> int:
    ap = argparse.ArgumentParser(description="Write a human-fillable pad→DIP-pin template from layout pad detections.")
    ap.add_argument("--chip", required=True, help="Chip id (e.g. 4001)")
    ap.add_argument(
        "--pads-json",
        type=Path,
        default=None,
        help="layout_pads_v0 JSON (defaults to docs/evidence/layout_pads_v0/<chip>/<chip>_layout_pads_v0.json)",
    )
    ap.add_argument(
        "--edge-labels-json",
        type=Path,
        default=None,
        help="layout_edge_labels_v0 JSON (defaults to docs/evidence/layout_edge_labels_v0/<chip>/<chip>_layout_edge_labels_v0.json)",
    )
    ap.add_argument(
        "--out",
        type=Path,
        default=None,
        help="Output markdown path (defaults to docs/evidence/layout_pad_labels_v0/<chip>/pad_pin_template_v0.md)",
    )
    ap.add_argument(
        "--anchors",
        type=Path,
        default=None,
        help="schematic_layout_anchors_v1 JSON (defaults to docs/evidence/schematic_layout_anchors_v1.json)",
    )
    args = ap.parse_args()

    chip = str(args.chip).strip()
    pads_path = args.pads_json or (ROOT / "docs" / "evidence" / "layout_pads_v0" / chip / f"{chip}_layout_pads_v0.json")
    labels_path = args.edge_labels_json or (
        ROOT / "docs" / "evidence" / "layout_edge_labels_v0" / chip / f"{chip}_layout_edge_labels_v0.json"
    )
    out_path = args.out or (ROOT / "docs" / "evidence" / "layout_pad_labels_v0" / chip / "pad_pin_template_v0.md")
    anchors_path = args.anchors or (ROOT / "docs" / "evidence" / "schematic_layout_anchors_v1.json")

    pads = _read_json(pads_path)
    pad_list = pads.get("pads") or []
    if not isinstance(pad_list, list) or not pad_list:
        raise SystemExit(f"no pads in {pads_path}")

    tok_for_idx: dict[int, str] = {}
    if labels_path.exists():
        dets = _read_json(labels_path).get("detections") or []
        if isinstance(dets, list):
            for det in dets:
                if not isinstance(det, dict):
                    continue
                tok = str(det.get("normalized_token") or det.get("token") or "").strip()
                sln = det.get("suggested_layout_node") or {}
                mb = sln.get("metal_bbox")
                if not tok or not isinstance(mb, dict):
                    continue
                for p in pad_list:
                    bb = p.get("bbox")
                    idx = p.get("idx_perimeter_ccw")
                    if not isinstance(bb, dict) or not isinstance(idx, int):
                        continue
                    if _overlaps(bb, mb):
                        tok_for_idx[int(idx)] = tok

    anchor_for_node: dict[int, str] = {}
    if anchors_path.exists():
        raw = _read_json(anchors_path)
        chip_anchors = raw.get("anchors", {}).get(chip, {})
        if isinstance(chip_anchors, dict):
            for name, payload in chip_anchors.items():
                if not isinstance(payload, dict):
                    continue
                node = payload.get("layout_node")
                if isinstance(node, int):
                    anchor_for_node[int(node)] = str(name)

    lines: list[str] = []
    lines.append(f"# {chip} pad→pin mapping template (v0)")
    lines.append("")
    lines.append("This file is intended to be human-edited.")
    lines.append("")
    lines.append(f"- Pad detections: `{pads_path.relative_to(ROOT) if pads_path.is_relative_to(ROOT) else pads_path}`")
    lines.append(f"- Edge labels: `{labels_path.relative_to(ROOT) if labels_path.is_relative_to(ROOT) else labels_path}`")
    lines.append("")
    lines.append("Fill `pin_dip` and `signal` using the primary-source pinout diagrams, then map to anchors.")
    lines.append("")
    lines.append("| pad_idx | edge | bbox (x0,y0,x1,y1) | suggested_node | edge_label | pin_dip | signal | confidence | notes |")
    lines.append("|---:|---|---|---:|---|---:|---|---:|---|")

    for p in sorted(pad_list, key=lambda x: int(x.get("idx_perimeter_ccw", 10**9))):
        bb = p.get("bbox") or {}
        idx = int(p.get("idx_perimeter_ccw", -1))
        edge = str(p.get("nearest_edge") or "")
        tok = tok_for_idx.get(idx, "")
        pin_auto, sig_auto, conf_auto = _auto_signal_from_edge_token(chip, tok)
        suggested_nodes = p.get("suggested_nodes") or []
        node = ""
        anchor_name = ""
        if isinstance(suggested_nodes, list) and suggested_nodes:
            top = suggested_nodes[0]
            if isinstance(top, dict) and isinstance(top.get("node"), int):
                node = str(int(top["node"]))
                anchor_name = anchor_for_node.get(int(top["node"]), "")

        # Prefer anchor-derived labeling when we can match the suggested node to an existing anchor.
        # This tends to be higher precision than edge-token OCR, and also works when the token is blank.
        if anchor_name:
            sig_auto = anchor_name
            conf_auto = "0.95"
            pin_auto = _pin_for_signal(chip, sig_auto)
            notes = "AUTO_FROM_ANCHORS_V1"
        else:
            notes = "AUTO_EDGE_TOKEN" if sig_auto else ""
        bbox_txt = f"({bb.get('x0')},{bb.get('y0')},{bb.get('x1')},{bb.get('y1')})"
        lines.append(f"| {idx} | {edge} | {bbox_txt} | {node} | {tok} | {pin_auto} | {sig_auto} | {conf_auto} | {notes} |")

    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(str(out_path))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
