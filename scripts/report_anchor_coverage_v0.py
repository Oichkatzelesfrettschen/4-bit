#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def _load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def _rel(path: Path) -> str:
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


def _expand_range(prefix: str, a: int, b: int) -> list[str]:
    lo, hi = (a, b) if a <= b else (b, a)
    return [f"{prefix}{i}" for i in range(lo, hi + 1)]


def _parse_required_tokens(md: str, chip: str) -> list[str]:
    # Extract the bullet block under "Signal ↔ pin number" for a given chip section.
    m = re.search(rf"^##\s+{re.escape(chip)}\b.*?$", md, flags=re.MULTILINE)
    if not m:
        return []
    start = m.end()
    tail = md[start:]
    m2 = re.search(r"^Signal ↔ pin number.*?:\s*$", tail, flags=re.MULTILINE)
    if not m2:
        return []
    tail2 = tail[m2.end() :]

    lines: list[str] = []
    for line in tail2.splitlines():
        if line.strip() == "" and lines:
            break
        if line.startswith("## "):
            break
        if line.lstrip().startswith("- "):
            lines.append(line)
        elif lines:
            # Stop once we leave the list.
            break

    raw = "\n".join(lines)

    tokens: list[str] = []
    # Common expansions:
    # - `D0..D3`
    # - `I/O0..I/O3`
    # - `O0..O9`
    # - `CM-RAM0..CM-RAM3`
    for t in re.findall(r"`([^`]+)`", raw):
        tt = t.strip()

        # Support both "D0..D3" and "D0..3" styles (some sources repeat the prefix).
        mrange = re.fullmatch(r"([A-Za-z/ -]+)(\d+)\.\.([A-Za-z/ -]+)?(\d+)", tt)
        if mrange:
            prefix_a = (mrange.group(1) or "").strip()
            prefix_b = (mrange.group(3) or "").strip()
            a = int(mrange.group(2))
            b = int(mrange.group(4))
            if prefix_b and prefix_b != prefix_a:
                # Not a consistent prefix range (likely a prose fragment).
                pass
            else:
                prefix = prefix_a
                # Normalize a few aliases to match anchors naming.
                if prefix == "I/O":
                    prefix = "IO"
                if prefix == "CM-RAM":
                    prefix = "CMRAM"
                if prefix in ("CMROM", "CM-ROM"):
                    prefix = "CMROM"
                tokens.extend(_expand_range(prefix, a, b))
                continue

        # Some lines use backticks for non-token phrases; keep only plausible signal-ish strings.
        tt_simple = tt.replace("\\", "")
        if re.fullmatch(r"[A-Z0-9/ -]{1,16}", tt_simple) and any(ch.isalpha() for ch in tt_simple):
            tokens.append(tt.replace("/", "").replace("-", ""))

    # Chip-specific normalization and inferred names:
    norm: list[str] = []
    for t in tokens:
        t = t.replace(" ", "")
        if t == "φ1":
            t = "CLK1"
        if t == "φ2":
            t = "CLK2"
        if t == "CMROM":
            t = "CMROM"
        norm.append(t)

    # The pinout docs intentionally leave 4002 output bits vague; map to repo signal names.
    if chip == "4002":
        for i in range(4):
            norm.append(f"OUT{i}")

    # 4003 uses CP/E naming; repo signals often use CLOCK/DATA/EN/OUT in anchors for layout alignment.
    if chip == "4003":
        # Ensure we include the repo's schematic-aligned aliases too.
        norm.extend(["CLOCK", "DATA", "EN", "OUT"])

    # Deduplicate while preserving order
    out: list[str] = []
    seen: set[str] = set()
    for t in norm:
        if t in seen:
            continue
        seen.add(t)
        out.append(t)
    return out


def _canonicalize_required(*, chip: str, sig: str) -> tuple[str, str | None]:
    """
    Map primary-source pinout tokens onto repo anchor naming, when they differ.

    Returns (canonical, alias_note). If alias_note is None, no alias was applied.
    """
    # Common name differences across sources vs repo:
    # - 4001/4002: CM-ROM / CM-RAM command pin is anchored as `CM`
    # - 4002: P0 (chip-select metal option) is anchored as `CS`
    # - 4003: pinout uses CP/DATA IN/E/Serial out, repo uses CLOCK/DATA/EN/OUT
    aliases: dict[str, dict[str, str]] = {
        # External data bus pins are pads in our layout anchoring.
        "4001": {"CMROM": "CM", "D0": "D0_PAD", "D1": "D1_PAD", "D2": "D2_PAD", "D3": "D3_PAD"},
        "4002": {"P0": "CS", "CMROM": "CM", "CMRAM": "CM", "D0": "D0_PAD", "D1": "D1_PAD", "D2": "D2_PAD", "D3": "D3_PAD"},
        "4003": {"CP": "CLOCK", "DATAIN": "DATA", "E": "EN", "SERIALOUT": "OUT"},
        "4004": {"D0": "D0_PAD", "D1": "D1_PAD", "D2": "D2_PAD", "D3": "D3_PAD"},
    }
    if chip == "4003":
        m = re.fullmatch(r"O(\d+)", sig)
        if m:
            canon = f"Q{m.group(1)}"
            return canon, f"{sig}→{canon}"
    canon = aliases.get(chip, {}).get(sig, sig)
    if canon != sig:
        return canon, f"{sig}→{canon}"
    return sig, None


def main() -> int:
    p = argparse.ArgumentParser(description="Report which primary-source pinout signals are anchored to layout nodes (v0).")
    p.add_argument("--anchors", type=Path, default=ROOT / "docs/evidence/schematic_layout_anchors_v1.json")
    p.add_argument("--pinouts", type=Path, default=ROOT / "docs/evidence/PRIMARY_SOURCE_PINOUTS.md")
    p.add_argument("--out", type=Path, default=ROOT / "docs/evidence/ANCHOR_COVERAGE_V0.md")
    args = p.parse_args()

    anchors = _load_json(args.anchors)["anchors"]
    md = args.pinouts.read_text(encoding="utf-8")

    chips = ["4001", "4002", "4003", "4004"]
    report_lines: list[str] = []
    report_lines.append("# Anchor coverage vs primary pinouts (v0)\n")
    report_lines.append(f"- Anchors: `{_rel(args.anchors)}`")
    report_lines.append(f"- Pinouts: `{_rel(args.pinouts)}`\n")

    for chip in chips:
        required = _parse_required_tokens(md, chip)
        report_lines.append(f"## {chip}\n")
        report_lines.append("| Signal | Anchor present | layout_node | layout_node_src |")
        report_lines.append("|---|---:|---:|---:|")
        a = anchors.get(chip, {})
        for sig in required:
            canon, alias_note = _canonicalize_required(chip=chip, sig=sig)
            rec = a.get(canon)
            if not isinstance(rec, dict):
                if alias_note:
                    report_lines.append(f"| `{sig}` (`{alias_note}`) | no |  |  |")
                else:
                    report_lines.append(f"| `{sig}` | no |  |  |")
                continue
            ln = rec.get("layout_node")
            src = rec.get("layout_node_src")
            if alias_note:
                report_lines.append(
                    f"| `{sig}` (`{alias_note}`) | yes | {'' if ln is None else int(ln)} | {'' if src is None else int(src)} |"
                )
            else:
                report_lines.append(f"| `{sig}` | yes | {'' if ln is None else int(ln)} | {'' if src is None else int(src)} |")
        report_lines.append("")

    out = args.out
    if not out.is_absolute():
        out = (ROOT / out).resolve()
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text("\n".join(report_lines) + "\n", encoding="utf-8")
    print(_rel(out))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
