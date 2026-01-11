#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def rel_or_abs(path: Path) -> str:
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


def normalize_label(s: str) -> str:
    s = (s or "").strip().upper()
    s = re.sub(r"[^A-Z0-9]", "", s)
    return s


def tokens_from_signals_file(path: Path) -> set[str]:
    """
    Extract “label-like” tokens from `docs/emulators/i400x-signals.txt`.

    Signals contain many complex expressions; for label verification we want short anchors like:
    - R0 / R1 / R2 / R3 (possibly with .0/.1 suffixes)
    - D0..D3, CLK1/CLK2, etc.
    """
    toks: set[str] = set()
    for raw in path.read_text(encoding="utf-8", errors="replace").splitlines():
        raw = raw.strip()
        if not raw or raw.startswith(";"):
            continue
        parts = [p.strip() for p in raw.split(",")]
        if len(parts) < 3:
            continue
        name = parts[2]
        base = name.split(".", 1)[0]  # R0.1 -> R0
        base = normalize_label(base)
        if 1 <= len(base) <= 4:
            toks.add(base)
    return toks


def tokens_from_edge_labels_json(path: Path) -> set[str]:
    data = json.loads(path.read_text(encoding="utf-8"))
    out: set[str] = set()
    for det in data.get("detections", []):
        t = normalize_label(str(det.get("token", "")))
        if t:
            out.add(t)
    return out


def main() -> int:
    p = argparse.ArgumentParser(description="Cross-check layout edge-label OCR tokens against i400x signals (v0).")
    p.add_argument("--chip", default="4004", help="Chip number (4001/4002/4003/4004)")
    p.add_argument(
        "--edge-labels",
        type=Path,
        default=ROOT / "docs" / "evidence" / "layout_edge_labels_v0" / "4004" / "4004_layout_edge_labels_v0.json",
        help="Edge labels JSON to verify",
    )
    args = p.parse_args()

    chip = str(args.chip).strip()
    signals = ROOT / "docs" / "emulators" / f"i{chip}-signals.txt"
    if not signals.exists():
        raise SystemExit(f"missing signals file: {rel_or_abs(signals)}")

    edge_labels = args.edge_labels
    if not edge_labels.is_absolute():
        edge_labels = (ROOT / edge_labels).resolve()
    if not edge_labels.exists():
        raise SystemExit(f"missing edge-labels json: {rel_or_abs(edge_labels)}")

    sig_toks = tokens_from_signals_file(signals)
    edge_toks = tokens_from_edge_labels_json(edge_labels)

    # Not all edge labels appear in `signals.txt`; we only report those that *do* have an expected token set.
    missing_in_signals = sorted([t for t in edge_toks if t not in sig_toks])
    present = sorted([t for t in edge_toks if t in sig_toks])

    print(json.dumps({"chip": chip, "edge_labels": rel_or_abs(edge_labels), "present_in_signals": present}, indent=2))
    if missing_in_signals:
        print("\nNot in signals.txt (may still be valid layout-only labels):")
        for t in missing_in_signals:
            print(f"- {t}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

