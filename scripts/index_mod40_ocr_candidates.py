#!/usr/bin/env python3
"""Index page-scoped MOD 40 OCR labels as non-authoritative net candidates."""

from __future__ import annotations

import argparse
import json
import re
from collections import defaultdict
from collections.abc import Iterable
from pathlib import Path

SIGNAL_PATTERN = re.compile(
    r"(?:CM(?:-?ROM|-?RAM[0-3]?)?|MAD(?:[0-9]|1[01])?|MD[IO][0-7]?|"
    r"TTY(?: ?(?:IN|OUT|PRINTER|READER))?|RDR(?: ?CONT)?|CPU(?: ?RESET)?|"
    r"USER(?: ?RESET)?|STOP(?: ?ACK| ?PB)?|SYNC|TEST|PHI[12]|BYTE ?[12]|"
    r"MODULE ?SELECT|MOD(?:ULE)? ?(?:ENABLE|SEL(?:ECT)? ?1[2-5])?|"
    r"(?:ENABLE )?MON(?:ITOR)? ?PROM|PROM ?SELECT|WRITE|READ|"
    r"ADDRESS ?(?:STROBE|[0-9]{1,2})|DATA ?(?:IN|OUT)|CMA ?(?:EX|WRITE)|"
    r"4002 ?RESET(?: ?ENABLE)?|TYPE)"
)


def normalize_label(value: str) -> str:
    normalized = value.upper().replace("_", " ")
    normalized = re.sub(r"[^A-Z0-9 -]", " ", normalized)
    normalized = re.sub(r"\s+", " ", normalized).strip()
    normalized = normalized.replace("CM ROM", "CM-ROM")
    normalized = re.sub(r"CM RAM ?([0-3])", r"CM-RAM\1", normalized)
    normalized = re.sub(r"MAD ?([0-9]|1[01])", r"MAD\1", normalized)
    normalized = re.sub(r"BYTE ?([12])", r"BYTE\1", normalized)
    normalized = normalized.replace("MODULE SELECT", "MODULE SELECT")
    return normalized


def labels_from_text(path: Path) -> Iterable[str]:
    yield from path.read_text(encoding="utf-8", errors="replace").splitlines()


def labels_from_surya(path: Path) -> Iterable[str]:
    payload = json.loads(path.read_text(encoding="utf-8"))
    for document_pages in payload.values():
        for page in document_pages:
            for line in page.get("text_lines", []):
                yield str(line.get("text", ""))


def collect_engine_labels(root: Path) -> dict[tuple[str, str], list[str]]:
    evidence: dict[tuple[str, str], list[str]] = defaultdict(list)
    engines = ("tesseract-psm6", "tesseract-psm11", "ocrad")
    for engine in engines:
        for text_path in sorted((root / engine).glob("page-*.txt")):
            page = text_path.stem
            for raw_label in labels_from_text(text_path):
                label = normalize_label(raw_label)
                if SIGNAL_PATTERN.fullmatch(label):
                    evidence[(page, label)].append(engine)

    for result_path in sorted((root / "surya").glob("page-*/results.json")):
        page = result_path.parent.name
        for raw_label in labels_from_surya(result_path):
            label = normalize_label(raw_label)
            if SIGNAL_PATTERN.fullmatch(label):
                evidence[(page, label)].append("surya")
    return evidence


def collected_engine_pages(root: Path) -> dict[str, list[str]]:
    pages: dict[str, list[str]] = {}
    for engine in ("tesseract-psm6", "tesseract-psm11", "ocrad"):
        pages[engine] = [path.stem for path in sorted((root / engine).glob("page-*.txt"))]
    pages["surya"] = [
        path.parent.name for path in sorted((root / "surya").glob("page-*/results.json"))
    ]
    return pages


def collected_engine_statuses(root: Path) -> dict[str, list[dict[str, str]]]:
    statuses: dict[str, list[dict[str, str]]] = {}
    for engine in ("tesseract-psm6", "tesseract-psm11", "ocrad"):
        engine_statuses = []
        for status_path in sorted((root / engine).glob("page-*.status")):
            fields = {}
            for line in status_path.read_text(encoding="utf-8", errors="replace").splitlines():
                key, separator, value = line.partition("=")
                if separator:
                    fields[key] = value
            engine_statuses.append({"page": status_path.stem, **fields})
        statuses[engine] = engine_statuses
    return statuses


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True, help="OCR output root")
    parser.add_argument("--output", type=Path, required=True, help="candidate JSON output")
    arguments = parser.parse_args()

    evidence = collect_engine_labels(arguments.input)
    candidates = []
    for (page, label), observations in sorted(evidence.items()):
        engines = sorted(set(observations))
        candidates.append(
            {
                "page": page,
                "label": label,
                "status": "ocr-candidate-not-a-net-claim",
                "engine_count": len(engines),
                "engines": engines,
                "observation_count": len(observations),
                "verification": "requires visual primary-sheet endpoint and polarity review",
            }
        )

    engine_pages = collected_engine_pages(arguments.input)
    output = {
        "schema": "mcs4.mod40.ocr-candidates.v1",
        "source_class": "ocr-discovery-only",
        "input": str(arguments.input),
        "engine_pages": engine_pages,
        "engine_statuses": collected_engine_statuses(arguments.input),
        "candidate_count": len(candidates),
        "candidates": candidates,
    }
    arguments.output.parent.mkdir(parents=True, exist_ok=True)
    arguments.output.write_text(json.dumps(output, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
