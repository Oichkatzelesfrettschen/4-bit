#!/usr/bin/env python3
"""
Validate pin labels against PRIMARY_SOURCE_PINOUTS.md

This script cross-validates OCR'd pin labels from layout/schematic
against the authoritative pinout documentation.
"""

from __future__ import annotations

import argparse
import json
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


@dataclass
class PinMapping:
    """Expected pin mapping from primary source."""

    pin: int
    signal: str
    notes: str = ""


# Primary source pinouts (from PRIMARY_SOURCE_PINOUTS.md)
PRIMARY_PINOUTS = {
    "4001": [
        PinMapping(1, "D0", "bidirectional data bus"),
        PinMapping(2, "D1", "bidirectional data bus"),
        PinMapping(3, "D2", "bidirectional data bus"),
        PinMapping(4, "D3", "bidirectional data bus"),
        PinMapping(5, "VSS", "ground"),
        PinMapping(6, "CLK1", "φ1 clock (schematic: CLK1)"),
        PinMapping(7, "CLK2", "φ2 clock (schematic: CLK2)"),
        PinMapping(8, "SYNC", "synchronization input"),
        PinMapping(9, "RESET", "reset input"),
        PinMapping(10, "CL", "clear input for I/O lines"),
        PinMapping(11, "CM", "CM-ROM chip enable (schematic: CM)"),
        PinMapping(12, "VDD", "positive supply"),
        PinMapping(13, "IO0", "I/O port bit 0"),
        PinMapping(14, "IO1", "I/O port bit 1"),
        PinMapping(15, "IO2", "I/O port bit 2"),
        PinMapping(16, "IO3", "I/O port bit 3"),
    ],
    "4002": [
        PinMapping(1, "D0", "bidirectional data bus"),
        PinMapping(2, "D1", "bidirectional data bus"),
        PinMapping(3, "D2", "bidirectional data bus"),
        PinMapping(4, "D3", "bidirectional data bus"),
        PinMapping(5, "VSS", "ground"),
        PinMapping(6, "CLK1", "φ1 clock (schematic: CLK1)"),
        PinMapping(7, "CLK2", "φ2 clock (schematic: CLK2)"),
        PinMapping(8, "SYNC", "synchronization input"),
        PinMapping(9, "RESET", "reset input"),
        PinMapping(10, "CS", "P0/Po chip selection"),
        PinMapping(11, "CM", "command input (CM-RAM)"),
        PinMapping(12, "VDD", "positive supply"),
        PinMapping(13, "OUT0", "output port bit 0"),
        PinMapping(14, "OUT1", "output port bit 1"),
        PinMapping(15, "OUT2", "output port bit 2"),
        PinMapping(16, "OUT3", "output port bit 3"),
    ],
    "4003": [
        PinMapping(1, "CLOCK", "CP clock pulse input"),
        PinMapping(2, "DATA", "DATA IN serial input"),
        PinMapping(3, "Q0", "O0 parallel output"),
        PinMapping(4, "Q1", "O1 parallel output"),
        PinMapping(5, "VSS", "ground"),
        PinMapping(6, "Q2", "O2 parallel output"),
        PinMapping(7, "Q3", "O3 parallel output"),
        PinMapping(8, "Q4", "O4 parallel output"),
        PinMapping(9, "Q5", "O5 parallel output"),
        PinMapping(10, "Q6", "O6 parallel output"),
        PinMapping(11, "Q7", "O7 parallel output"),
        PinMapping(12, "Q8", "O8 parallel output"),
        PinMapping(13, "Q9", "O9 parallel output"),
        PinMapping(14, "VDD", "positive supply"),
        PinMapping(15, "OUT", "Serial out"),
        PinMapping(16, "EN", "E enable"),
    ],
    "4004": [
        PinMapping(1, "D0", "bidirectional data bus"),
        PinMapping(2, "D1", "bidirectional data bus"),
        PinMapping(3, "D2", "bidirectional data bus"),
        PinMapping(4, "D3", "bidirectional data bus"),
        PinMapping(5, "VSS", "ground"),
        PinMapping(6, "CLK1", "φ1 clock (schematic: CLK1)"),
        PinMapping(7, "CLK2", "φ2 clock (schematic: CLK2)"),
        PinMapping(8, "SYNC", "synchronization output"),
        PinMapping(9, "RESET", "reset input"),
        PinMapping(10, "TEST", "test input"),
        PinMapping(11, "CM_ROM", "CM-ROM output"),
        PinMapping(12, "VDD", "VCC positive supply"),
        PinMapping(13, "CM_RAM0", "CM-RAM0 output"),
        PinMapping(14, "CM_RAM1", "CM-RAM1 output"),
        PinMapping(15, "CM_RAM2", "CM-RAM2 output"),
        PinMapping(16, "CM_RAM3", "CM-RAM3 output"),
    ],
}


def normalize_signal_name(signal: str) -> str:
    """
    Normalize signal name for comparison.

    Handles common variations:
    - φ1 → CLK1
    - CM-ROM → CM_ROM or CM
    - D0_PAD → D0
    """
    s = signal.upper().strip()

    # Remove _PAD suffix
    s = s.replace("_PAD", "")

    # Handle φ notation
    s = s.replace("Φ1", "CLK1").replace("PHI1", "CLK1")
    s = s.replace("Φ2", "CLK2").replace("PHI2", "CLK2")

    # Normalize separators
    s = s.replace("-", "_")

    return s


def validate_chip_pinout(
    chip: str,
    anchor_file: Path,
) -> dict:
    """
    Validate pin labels for a chip against primary source.

    Args:
        chip: Chip name (4001, 4002, 4003, 4004)
        anchor_file: Path to anchor incidence JSON

    Returns:
        Validation report dictionary
    """
    if chip not in PRIMARY_PINOUTS:
        raise ValueError(f"No primary pinout for chip {chip}")

    expected = {pm.pin: pm for pm in PRIMARY_PINOUTS[chip]}

    # Load anchor file
    with anchor_file.open("r", encoding="utf-8") as f:
        data = json.load(f)

    anchors = data.get("anchors", [])

    # Build pin mapping from anchors (simplified - assumes anchor name matches signal)
    results = {
        "chip": chip,
        "total_pins": len(expected),
        "validated_pins": 0,
        "mismatches": [],
        "missing": [],
        "extra": [],
    }

    # Check each anchor against expected pinout
    anchor_signals = {normalize_signal_name(a["name"]): a["name"] for a in anchors}

    for pin, pm in expected.items():
        expected_signal = normalize_signal_name(pm.signal)

        if expected_signal in anchor_signals:
            results["validated_pins"] += 1
        else:
            # Check for common aliases
            found = False
            for alias in [pm.signal, pm.signal.replace("_", ""), pm.signal.replace("_", "-")]:
                if normalize_signal_name(alias) in anchor_signals:
                    results["validated_pins"] += 1
                    found = True
                    break

            if not found:
                results["missing"].append({
                    "pin": pin,
                    "expected": pm.signal,
                    "notes": pm.notes,
                })

    # Check for extra anchors not in pinout
    expected_signals = {normalize_signal_name(pm.signal) for pm in expected.values()}
    for anchor_sig in anchor_signals.keys():
        if anchor_sig not in expected_signals:
            # Allow power rails and internal signals
            if anchor_sig not in {"VDD", "VSS", "GND"}:
                results["extra"].append(anchor_sig)

    return results


def main():
    parser = argparse.ArgumentParser(description="Validate pin connectivity")
    parser.add_argument(
        "--chips",
        nargs="+",
        default=["4001", "4002", "4003", "4004"],
        help="Chips to validate",
    )

    args = parser.parse_args()

    print("=== Pin Connectivity Validation ===")
    print("")

    all_good = True

    for chip in args.chips:
        print(f"Validating {chip}...")

        anchor_file = (
            ROOT
            / "docs"
            / "evidence"
            / "anchor_incidence_v1_canonical"
            / chip
            / chip
            / f"{chip}_anchor_incidence_v0.json"
        )

        if not anchor_file.exists():
            print(f"  ✗ Anchor file not found: {anchor_file}")
            all_good = False
            continue

        try:
            result = validate_chip_pinout(chip=chip, anchor_file=anchor_file)

            validated = result["validated_pins"]
            total = result["total_pins"]
            coverage = (validated / total * 100) if total > 0 else 0

            print(f"  Coverage: {validated}/{total} pins ({coverage:.1f}%)")

            if result["missing"]:
                print(f"  ⚠ Missing {len(result['missing'])} expected pins:")
                for m in result["missing"][:5]:  # Show first 5
                    print(f"    - Pin {m['pin']}: {m['expected']} ({m['notes']})")
                if len(result["missing"]) > 5:
                    print(f"    ... and {len(result['missing']) - 5} more")
                all_good = False

            if result["extra"]:
                print(f"  ℹ Found {len(result['extra'])} extra signals (may be internal)")

            if coverage >= 90:
                print(f"  ✓ Good coverage ({coverage:.1f}%)")
            elif coverage >= 70:
                print(f"  ⚠ Acceptable coverage ({coverage:.1f}%)")
            else:
                print(f"  ✗ Low coverage ({coverage:.1f}%)")
                all_good = False

        except Exception as e:
            print(f"  ✗ Validation failed: {e}")
            all_good = False
            continue

        print("")

    if all_good:
        print("✓ All validations passed")
        return 0
    else:
        print("⚠ Some validations failed or have low coverage")
        return 1


if __name__ == "__main__":
    exit(main())
