#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def parse_4004_layout_transistors(readme: str) -> int | None:
    m = re.search(r"transistor:\s+(\d+)\s+(\d+)\s*-\s*(\d+)\s*=\s*(\d+)", readme, flags=re.IGNORECASE)
    if not m:
        return None
    return int(m.group(1))


def main() -> int:
    p = argparse.ArgumentParser(description="Validate local evidence against i400x analyzer readme claims.")
    p.add_argument(
        "--tolerance",
        type=int,
        default=10,
        help="Allowed absolute delta between extracted and analyzer transistor counts (4004)",
    )
    args = p.parse_args()

    readme_path = ROOT / "docs" / "emulators" / "readme.txt"
    metrics_path = ROOT / "docs" / "evidence" / "transistors" / "metrics.json"

    if not readme_path.exists():
        raise SystemExit(f"missing analyzer readme: {readme_path}")
    if not metrics_path.exists():
        raise SystemExit(f"missing transistor metrics: {metrics_path}")

    readme = readme_path.read_text(errors="replace")
    analyzer_4004 = parse_4004_layout_transistors(readme)
    if analyzer_4004 is None:
        raise SystemExit("failed to parse 4004 layout transistor count from analyzer readme")

    metrics = json.loads(metrics_path.read_text(encoding="utf-8"))
    chips = metrics.get("chips", [])
    if not isinstance(chips, list):
        raise SystemExit("invalid metrics.json: chips must be a list")

    extracted_4004 = None
    for c in chips:
        if not isinstance(c, dict):
            continue
        if c.get("chip") == "4004":
            extracted_4004 = int(c.get("components_total", 0))
            break

    if extracted_4004 is None:
        raise SystemExit("metrics.json missing chip=4004 entry")

    delta = extracted_4004 - analyzer_4004
    print(f"4004: analyzer layout transistors={analyzer_4004}, extracted poly∩diff components={extracted_4004}, delta={delta:+d}")

    if abs(delta) > int(args.tolerance):
        print(f"FAIL: |delta| > tolerance ({args.tolerance})")
        return 1

    print("OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

