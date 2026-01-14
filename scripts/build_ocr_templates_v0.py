#!/usr/bin/env python3
from __future__ import annotations

import argparse
import re
import shutil
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def _parse_confirmed_table(md: str) -> list[tuple[str, str]]:
    """
    Parse `docs/evidence/layout_pad_labels_v0/4004/manual_readings_v0.md` confirmed-labels table.
    Returns [(token, crop_path)].
    """
    out: list[tuple[str, str]] = []
    in_table = False
    for raw in md.splitlines():
        line = raw.strip()
        if line.startswith("## Confirmed labels"):
            in_table = True
            continue
        if not in_table:
            continue
        if not line:
            if out:
                break
            continue
        if line.startswith("|---"):
            continue
        if not (line.startswith("|") and line.endswith("|")):
            continue
        parts = [p.strip() for p in line.strip("|").split("|")]
        if len(parts) < 3:
            continue
        tok = parts[0].strip("` ").upper()
        crop = parts[2]
        m = re.search(r"`([^`]+)`", crop)
        if m:
            crop = m.group(1)
        crop = crop.strip()
        if tok and crop:
            out.append((tok, crop))
    return out


def main() -> int:
    ap = argparse.ArgumentParser(description="Build OCR template directory from existing confirmed crops (v0).")
    ap.add_argument(
        "--src-md",
        type=Path,
        default=ROOT / "docs" / "evidence" / "layout_pad_labels_v0" / "4004" / "manual_readings_v0.md",
        help="Markdown file containing confirmed label crops.",
    )
    ap.add_argument(
        "--out-dir",
        type=Path,
        default=ROOT / "docs" / "evidence" / "ocr_models" / "templates_v0",
        help="Directory to write templates into (set OCR_TEMPLATE_DIR to this).",
    )
    args = ap.parse_args()

    src_md = (ROOT / args.src_md).resolve() if not args.src_md.is_absolute() else args.src_md
    out_dir = (ROOT / args.out_dir).resolve() if not args.out_dir.is_absolute() else args.out_dir

    md = src_md.read_text(encoding="utf-8")
    pairs = _parse_confirmed_table(md)
    if not pairs:
        raise SystemExit(f"no confirmed table rows found in {src_md}")

    out_dir.mkdir(parents=True, exist_ok=True)

    copied = 0
    skipped = 0
    for i, (tok, crop_rel) in enumerate(pairs):
        crop_path = (ROOT / crop_rel).resolve() if not Path(crop_rel).is_absolute() else Path(crop_rel)
        if not crop_path.exists():
            skipped += 1
            continue
        # Canonicalize token to filesystem-safe name.
        safe = re.sub(r"[^A-Z0-9]+", "", tok)
        if not safe:
            skipped += 1
            continue
        dst = out_dir / f"tok_{safe}_{i:03d}.png"
        shutil.copyfile(crop_path, dst)
        copied += 1

    print(f"wrote {copied} templates to {out_dir} (skipped {skipped})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

