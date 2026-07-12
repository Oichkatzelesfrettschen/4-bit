#!/usr/bin/env python3
"""Extract direct-call evidence from rustc human-readable MIR output."""

from __future__ import annotations

import argparse
import re
from pathlib import Path

FUNCTION_PATTERN = re.compile(r"^fn (?P<caller>.+?)\(")
CALL_PATTERN = re.compile(r"^\s*[^=]+ = (?P<callee>.+\)) ->")


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Extract direct calls from rustc -Zunpretty=mir output."
    )
    parser.add_argument("input", type=Path, help="MIR text emitted by rustc")
    parser.add_argument("output", type=Path, help="TSV output path")
    return parser.parse_args()


def extract_calls(mir_text: str) -> list[tuple[str, str, int]]:
    caller = "<outside-function>"
    calls: list[tuple[str, str, int]] = []

    for line_number, line in enumerate(mir_text.splitlines(), start=1):
        function_match = FUNCTION_PATTERN.match(line)
        if function_match:
            caller = function_match.group("caller")
            continue

        call_match = CALL_PATTERN.match(line)
        if call_match:
            calls.append((caller, call_match.group("callee"), line_number))

    return calls


def main() -> int:
    arguments = parse_arguments()
    mir_text = arguments.input.read_text(encoding="utf-8")
    calls = extract_calls(mir_text)

    rows = ["caller\tcallee\tmir_line"]
    rows.extend(f"{caller}\t{callee}\t{line_number}" for caller, callee, line_number in calls)
    arguments.output.write_text("\n".join(rows) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
