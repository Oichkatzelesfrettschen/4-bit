#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from pathlib import Path

import cv2  # type: ignore

ROOT = Path(__file__).resolve().parents[1]


def rel_or_abs(path: Path) -> str:
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


def _load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def _read_gray(path: Path) -> cv2.typing.MatLike:
    img = cv2.imread(str(path), cv2.IMREAD_GRAYSCALE)
    if img is None:
        raise FileNotFoundError(str(path))
    return img


def _clip_bbox(bb: dict, w: int, h: int, *, pad: int) -> tuple[int, int, int, int]:
    x0, y0, x1, y1 = int(bb["x0"]), int(bb["y0"]), int(bb["x1"]), int(bb["y1"])
    x0 = max(0, x0 - pad)
    y0 = max(0, y0 - pad)
    x1 = min(w, x1 + pad)
    y1 = min(h, y1 + pad)
    if x1 <= x0:
        x1 = min(w, x0 + 1)
    if y1 <= y0:
        y1 = min(h, y0 + 1)
    return x0, y0, x1, y1


def render_one(*, chip: str, candidates_json: Path, out_dir: Path, top: int, crop_pad: int, downscale: int) -> dict:
    cand_obj = _load_json(candidates_json)
    netlist_v0 = Path(cand_obj["inputs"]["netlist_v0"])
    netlist_v0_abs = netlist_v0 if netlist_v0.is_absolute() else (ROOT / netlist_v0)
    net = _load_json(netlist_v0_abs)
    metal_bmp = Path(net["inputs"]["metal_bmp"])
    metal_bmp_abs = metal_bmp if metal_bmp.is_absolute() else (ROOT / metal_bmp)

    metal = _read_gray(metal_bmp_abs)
    h, w = metal.shape[:2]

    ranked = list(cand_obj.get("candidates", []))[: int(top)]

    chip_dir = out_dir / chip
    crops_dir = chip_dir / "crops"
    crops_dir.mkdir(parents=True, exist_ok=True)

    overlay = cv2.cvtColor(metal, cv2.COLOR_GRAY2BGR)
    for i, c in enumerate(ranked, start=1):
        bb = c["bbox"]
        x0, y0, x1, y1 = int(bb["x0"]), int(bb["y0"]), int(bb["x1"]), int(bb["y1"])
        node = int(c["node"])
        # Color: shift from red→yellow→green with rank (lower rank = more red)
        t = (i - 1) / max(1, len(ranked) - 1)
        color = (0, int(255 * t), int(255 * (1 - t)))  # B,G,R
        cv2.rectangle(overlay, (x0, y0), (x1, y1), color, 2)
        cv2.putText(
            overlay,
            f"#{i} n{node}",
            (x0, max(12, y0 - 4)),
            cv2.FONT_HERSHEY_SIMPLEX,
            0.5,
            (255, 0, 0),
            1,
            cv2.LINE_AA,
        )

        cx0, cy0, cx1, cy1 = _clip_bbox(bb, w=w, h=h, pad=int(crop_pad))
        crop = metal[cy0:cy1, cx0:cx1]
        out_crop = crops_dir / f"rank_{i:02d}_node_{node}.png"
        cv2.imwrite(str(out_crop), crop)

    out_overlay = chip_dir / f"{chip.lower()}_power_rail_candidates_v0_overlay.png"
    if int(downscale) > 1:
        small = cv2.resize(
            overlay,
            (max(1, overlay.shape[1] // int(downscale)), max(1, overlay.shape[0] // int(downscale))),
            interpolation=cv2.INTER_AREA,
        )
        cv2.imwrite(str(out_overlay), small)
    else:
        cv2.imwrite(str(out_overlay), overlay)

    return {
        "chip": chip,
        "inputs": {"candidates_json": rel_or_abs(candidates_json), "metal_bmp": rel_or_abs(metal_bmp_abs)},
        "outputs": {"overlay_png": rel_or_abs(out_overlay), "crops_dir": rel_or_abs(crops_dir)},
        "counts": {"candidates_total": int(len(cand_obj.get("candidates", []))), "candidates_rendered": int(len(ranked))},
    }


def main() -> int:
    p = argparse.ArgumentParser(description="Render overlays/crops for power-rail candidate nodes (v0).")
    p.add_argument("--chip", action="append", choices=["4001", "4002", "4003", "4004"], help="Chip (repeatable)")
    p.add_argument("--all", action="store_true", help="All supported chips")
    p.add_argument(
        "--candidates-dir",
        type=Path,
        default=ROOT / "docs" / "evidence" / "power_rail_candidates_v0",
        help="Directory containing <chip>/<chip>_power_rail_candidates_v0.json",
    )
    p.add_argument("--out-dir", type=Path, default=ROOT / "docs" / "evidence" / "power_rail_candidates_v0")
    p.add_argument("--top", type=int, default=12, help="How many candidates to render per chip")
    p.add_argument("--crop-pad", type=int, default=12, help="Extra pixels around bbox for crops")
    p.add_argument("--downscale", type=int, default=2, help="Downscale factor for overview overlay (>=1)")
    args = p.parse_args()

    selected = set(args.chip or [])
    if args.all:
        selected = {"4001", "4002", "4003", "4004"}
    if not selected:
        p.error("select --all or at least one --chip")

    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    manifest = {
        "schema": {"version": 0, "description": "Rendered overlays/crops for power-rail candidate nodes."},
        "tool": "scripts/render_power_rail_candidates_v0.py",
        "params": {"top": int(args.top), "crop_pad": int(args.crop_pad), "downscale": int(args.downscale)},
        "outputs": [],
    }

    for chip in sorted(selected):
        candidates_json = Path(args.candidates_dir) / chip / f"{chip}_power_rail_candidates_v0.json"
        if not candidates_json.exists():
            raise FileNotFoundError(str(candidates_json))
        manifest["outputs"].append(
            render_one(
                chip=chip,
                candidates_json=candidates_json,
                out_dir=out_dir,
                top=int(args.top),
                crop_pad=int(args.crop_pad),
                downscale=int(args.downscale),
            )
        )

    (out_dir / "render_manifest_v0.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(rel_or_abs(out_dir / "render_manifest_v0.json"))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

