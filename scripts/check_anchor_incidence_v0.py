#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def _load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def main() -> int:
    p = argparse.ArgumentParser(description="Fail if any anchor signal has <min-total incident transistors (v0).")
    p.add_argument(
        "--netlist-v1",
        type=Path,
        default=ROOT / "docs" / "evidence" / "netlists_v1" / "4004_netlist_v1.json",
    )
    p.add_argument("--min-total", type=int, default=1)
    p.add_argument("--allow-zero-regex", default=r"^(SYNC|POC_PAD|TEST_PAD)$", help="Allowlisted anchor names")
    args = p.parse_args()

    net_path = args.netlist_v1
    if not net_path.is_absolute():
        net_path = (ROOT / net_path).resolve()
    net = _load(net_path)

    allow_re = re.compile(str(args.allow_zero_regex))

    # Incidence index.
    incidence: dict[int, int] = {}
    for t in net.get("devices", {}).get("transistors", []) if isinstance(net.get("devices"), dict) else []:
        if not isinstance(t, dict):
            continue
        for k in ("gate_node", "a_node", "b_node"):
            n = t.get(k)
            if isinstance(n, int):
                incidence[int(n)] = int(incidence.get(int(n), 0)) + 1

    failures: list[str] = []
    for s in net.get("signals", []) if isinstance(net.get("signals"), list) else []:
        if not isinstance(s, dict):
            continue
        if not s.get("evidence", {}).get("anchor"):
            continue
        name = s.get("name")
        node = s.get("layout_node")
        if not isinstance(name, str) or not isinstance(node, int):
            continue
        tot = int(incidence.get(int(node), 0))
        if tot >= int(args.min_total):
            continue
        if allow_re.match(name):
            continue
        failures.append(f"{name}: node={node} total={tot}")

    if failures:
        print(f"Anchor incidence check failed for {len(failures)} anchors:")
        for f in failures:
            print(f"  - {f}")
        return 2

    print("Anchor incidence check passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

