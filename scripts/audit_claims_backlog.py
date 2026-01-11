#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import re
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


@dataclass(frozen=True)
class ClaimRow:
    claim: str
    source: str
    status: str
    notes: str


def parse_markdown_table(lines: list[str]) -> list[ClaimRow]:
    rows: list[ClaimRow] = []
    for ln in lines:
        s = ln.strip()
        if not s.startswith("|"):
            continue
        if re.match(r"^\\|\\s*-", s):
            continue
        parts = [p.strip() for p in s.strip("|").split("|")]
        if len(parts) != 4:
            continue
        if parts[0].lower() == "claim":
            continue
        rows.append(ClaimRow(claim=parts[0], source=parts[1], status=parts[2], notes=parts[3]))
    return rows


def extract_section(lines: list[str], header: str) -> list[str]:
    start = None
    for i, ln in enumerate(lines):
        if ln.strip() == header:
            start = i + 1
            break
    if start is None:
        return []
    out: list[str] = []
    for ln in lines[start:]:
        if ln.startswith("## "):
            break
        out.append(ln)
    return out


def main() -> int:
    p = argparse.ArgumentParser(description="Extract a backlog of unverified/derived claims from docs/AUDIT.md.")
    p.add_argument("--audit", type=Path, default=ROOT / "docs" / "AUDIT.md")
    p.add_argument("--out-json", type=Path, default=ROOT / "docs" / "evidence" / "audit_claims_backlog.json")
    p.add_argument("--out-md", type=Path, default=ROOT / "docs" / "evidence" / "audit_claims_backlog.md")
    args = p.parse_args()

    text = args.audit.read_text(encoding="utf-8", errors="replace")
    lines = text.splitlines()

    claims_section = extract_section(lines, "## Claims Verification (Phase 1)")
    claims = parse_markdown_table(claims_section)

    next_targets_section = extract_section(lines, "## Next Verification Targets")
    next_targets = [ln.strip()[2:].strip() for ln in next_targets_section if ln.strip().startswith("- ")]

    backlog: list[dict[str, object]] = []
    for c in claims:
        notes_lc = c.notes.lower()
        status_lc = c.status.lower()
        needs = ("pending" in notes_lc) or status_lc.startswith("derived") or status_lc.startswith("contradicted")
        if needs:
            backlog.append(
                {
                    "claim": c.claim,
                    "source": c.source,
                    "status": c.status,
                    "notes": c.notes,
                    "reason": "pending_in_notes" if "pending" in notes_lc else ("status=" + c.status),
                }
            )

    payload = {
        "tool": "scripts/audit_claims_backlog.py",
        "inputs": {"audit": str(args.audit.relative_to(ROOT))},
        "backlog": backlog,
        "next_verification_targets": next_targets,
    }

    args.out_json.parent.mkdir(parents=True, exist_ok=True)
    args.out_json.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    md: list[str] = []
    md.append("# Audit claims backlog")
    md.append("")
    md.append(f"Generated from `{args.audit.relative_to(ROOT)}` by `scripts/audit_claims_backlog.py`.")
    md.append("")
    md.append("## Backlog")
    md.append("")
    md.append("| Claim | Status | Notes |")
    md.append("| --- | --- | --- |")
    for item in backlog:
        md.append(f"| {item['claim']} | {item['status']} | {item['notes']} |")
    md.append("")
    md.append("## Next verification targets (from AUDIT)")
    md.append("")
    for t in next_targets:
        md.append(f"- {t}")
    md.append("")

    args.out_md.write_text("\n".join(md), encoding="utf-8")

    print(f"wrote {args.out_json.relative_to(ROOT)}")
    print(f"wrote {args.out_md.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

