#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]

DEFAULT_ALIAS_MAP: dict[str, dict[str, str]] = {
    "4001": {
        "(RESET)": "RESET",
        "D0": "D0_PAD",
        "D1": "D1_PAD",
        "D2": "D2_PAD",
        "D3": "D3_PAD",
    },
    "4002": {
        "(RESET)": "RESET",
        "D0": "D0_PAD",
        "D1": "D1_PAD",
        "D2": "D2_PAD",
        "D3": "D3_PAD",
    },
}


def _load(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def _write(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def _resolve(path: Path) -> Path:
    return (ROOT / path).resolve() if not path.is_absolute() else path


def _apply_alias(
    *,
    chip: str,
    target_signal: str,
    source_signal: str,
    block: dict[str, Any],
) -> dict[str, Any]:
    if target_signal not in block:
        return {"chip": chip, "target": target_signal, "source": source_signal, "status": "target_missing"}
    if source_signal not in block:
        return {"chip": chip, "target": target_signal, "source": source_signal, "status": "source_missing"}

    target = block[target_signal]
    source = block[source_signal]
    if not isinstance(target, dict) or not isinstance(source, dict):
        return {
            "chip": chip,
            "target": target_signal,
            "source": source_signal,
            "status": "non_object_anchor",
        }

    source_node = source.get("layout_node")
    if not isinstance(source_node, int):
        return {
            "chip": chip,
            "target": target_signal,
            "source": source_signal,
            "status": "source_unmapped",
        }

    if isinstance(target.get("layout_node"), int):
        return {
            "chip": chip,
            "target": target_signal,
            "source": source_signal,
            "status": "target_already_mapped",
            "node": int(target["layout_node"]),
        }

    target["layout_node"] = int(source_node)
    source_uid = source.get("layout_node_uid")
    if isinstance(source_uid, str) and source_uid:
        target["layout_node_uid"] = source_uid

    target["layout_seed_v1"] = {
        "kind": "priority_alias_v1",
        "source_signal": source_signal,
        "source_layout_node": int(source_node),
    }
    target["remap_v1"] = {
        "ok": True,
        "reason": "priority_alias_v1",
        "alias_from": source_signal,
        "dst_node": int(source_node),
        "dst_node_uid": source_uid if isinstance(source_uid, str) else None,
    }

    return {
        "chip": chip,
        "target": target_signal,
        "source": source_signal,
        "status": "applied",
        "node": int(source_node),
    }


def _apply_wrapper_aliases(*, chip: str, block: dict[str, Any]) -> list[dict[str, Any]]:
    """
    Apply direct wrapper aliases:
    - (SIGNAL) -> SIGNAL
    - [SIGNAL] -> SIGNAL
    """
    out: list[dict[str, Any]] = []
    wrapper_re = re.compile(r"^[\(\[](.+)[\)\]]$")
    for target_signal in sorted(block.keys()):
        target = block.get(target_signal)
        if not isinstance(target, dict):
            continue
        if isinstance(target.get("layout_node"), int):
            continue

        m = wrapper_re.match(target_signal)
        if not m:
            continue
        source_signal = m.group(1).strip()
        if not source_signal:
            continue
        out.append(
            _apply_alias(
                chip=chip,
                target_signal=target_signal,
                source_signal=source_signal,
                block=block,
            )
        )
    return out


def main() -> int:
    ap = argparse.ArgumentParser(
        description=(
            "Apply first-wave manual alias mappings for priority anchors "
            "(RESET and D0..D3) using already-mapped *_PAD/RESET anchors."
        )
    )
    ap.add_argument(
        "--anchors",
        type=Path,
        default=ROOT / "docs" / "evidence" / "schematic_layout_anchors_v1.json",
        help="Anchors JSON to update.",
    )
    ap.add_argument("--out", type=Path, default=None, help="Output path (default: in-place).")
    args = ap.parse_args()

    anchors_path = _resolve(args.anchors)
    out_path = _resolve(args.out) if args.out else anchors_path

    payload = _load(anchors_path)
    aroot = payload.get("anchors")
    if not isinstance(aroot, dict):
        raise SystemExit("anchors file missing top-level 'anchors' object")

    results: list[dict[str, Any]] = []
    for chip, aliases in DEFAULT_ALIAS_MAP.items():
        block = aroot.get(chip)
        if not isinstance(block, dict):
            results.append({"chip": chip, "status": "chip_block_missing"})
            continue
        for target, source in aliases.items():
            results.append(_apply_alias(chip=chip, target_signal=target, source_signal=source, block=block))
        results.extend(_apply_wrapper_aliases(chip=chip, block=block))

    notes = payload.get("notes")
    if notes is None:
        payload["notes"] = []
        notes = payload["notes"]
    if isinstance(notes, list):
        applied_count = sum(1 for r in results if r.get("status") == "applied")
        if applied_count > 0:
            notes.append(
                {
                    "kind": "priority_alias_v1",
                    "applied": applied_count,
                    "results": results,
                }
            )

    _write(out_path, payload)
    print(json.dumps({"out": str(out_path), "results": results}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
