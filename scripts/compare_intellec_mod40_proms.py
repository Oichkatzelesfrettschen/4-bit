#!/usr/bin/env python3
"""Normalize and compare declared Intellec 4/MOD 40 monitor-PROM artifacts.

The tool accepts caller-supplied artifacts only.  It never downloads ROM data,
infers a socket order, chooses a canonical image, pads sparse input, or applies
a polarity transform.  A report identifies byte artifacts; it does not prove a
physical read, a board socket, or a source-faithful monitor image.

Every set names four files in caller-supplied order and declares one input
format for that set.  The current historical acceptance gate remains in force
until the primary schematic, socket record, and repeat-read provenance close.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import re
import sys
from collections.abc import Iterable
from dataclasses import asdict, dataclass
from pathlib import Path

DEVICE_BYTES = 256
DEVICES_PER_SET = 4
ASSEMBLED_BYTES = DEVICE_BYTES * DEVICES_PER_SET
INPUT_FORMATS = ("raw-binary", "intel-hex", "hex-listing")
HEX_BYTE = re.compile(r"[0-9A-Fa-f]{2}")
HEX_ADDRESS = re.compile(r"(?:0x)?[0-9A-Fa-f]+")


class PromFormatError(ValueError):
    """A PROM artifact violates its declared representation contract."""


@dataclass(frozen=True)
class DeviceRecord:
    """One normalized, caller-ordered source artifact."""

    set_name: str
    socket_index: int
    source_path: str
    source_format: str
    source_size: int
    normalized_size: int
    source_sha256: str
    normalized_sha256: str
    sum8: str


@dataclass(frozen=True)
class SocketComparison:
    """Byte comparison for one caller-ordered device position."""

    left_set: str
    right_set: str
    socket_index: int
    equal: bool
    differing_bytes: int
    first_difference: int | None


def sha256(data: bytes) -> str:
    """Return the stable SHA-256 identity for one byte sequence."""

    return hashlib.sha256(data).hexdigest()


def parse_intel_hex(text: str, source: Path) -> bytes:
    """Parse one complete, contiguous 0x00 through 0xff Intel HEX image."""

    image: dict[int, int] = {}
    upper_address = 0
    eof_seen = False

    for line_number, raw_line in enumerate(text.splitlines(), 1):
        line = raw_line.strip()
        if not line:
            continue
        if not line.startswith(":"):
            raise PromFormatError(f"{source}:{line_number}: expected Intel HEX record")
        try:
            record = bytes.fromhex(line[1:])
        except ValueError as exc:
            raise PromFormatError(f"{source}:{line_number}: invalid hexadecimal record") from exc
        if len(record) < 5:
            raise PromFormatError(f"{source}:{line_number}: record is too short")

        length = record[0]
        if len(record) != length + 5:
            raise PromFormatError(f"{source}:{line_number}: record length mismatch")
        if sum(record) & 0xFF:
            raise PromFormatError(f"{source}:{line_number}: checksum mismatch")

        address = int.from_bytes(record[1:3], "big")
        record_type = record[3]
        payload = record[4:-1]
        if record_type == 0x00:
            for offset, value in enumerate(payload):
                location = upper_address + address + offset
                existing = image.get(location)
                if existing is not None and existing != value:
                    raise PromFormatError(
                        f"{source}:{line_number}: conflicting byte at {location:#x}"
                    )
                image[location] = value
        elif record_type == 0x01:
            if length != 0:
                raise PromFormatError(f"{source}:{line_number}: malformed EOF record")
            eof_seen = True
        elif record_type == 0x02:
            if length != 2:
                raise PromFormatError(f"{source}:{line_number}: invalid segment address")
            upper_address = int.from_bytes(payload, "big") << 4
        elif record_type == 0x04:
            if length != 2:
                raise PromFormatError(f"{source}:{line_number}: invalid linear address")
            upper_address = int.from_bytes(payload, "big") << 16
        elif record_type not in {0x03, 0x05}:
            raise PromFormatError(f"{source}:{line_number}: unsupported record type {record_type:#x}")

    if not eof_seen:
        raise PromFormatError(f"{source}: Intel HEX EOF record is missing")
    if set(image) != set(range(DEVICE_BYTES)):
        raise PromFormatError(
            f"{source}: Intel HEX must represent exactly 0x00 through 0xff without holes"
        )
    return bytes(image[address] for address in range(DEVICE_BYTES))


def parse_hex_listing(text: str, source: Path) -> bytes:
    """Parse a sequential 256-byte hexadecimal listing without address guesses."""

    output = bytearray()
    for line_number, raw_line in enumerate(text.splitlines(), 1):
        line = re.split(r"[;#]", raw_line, maxsplit=1)[0].strip()
        if not line:
            continue
        if ":" in line:
            address_text, line = line.split(":", 1)
            address_text = address_text.strip()
            if not HEX_ADDRESS.fullmatch(address_text):
                raise PromFormatError(f"{source}:{line_number}: invalid address prefix")
            address = int(address_text, 16)
            if address != len(output):
                raise PromFormatError(
                    f"{source}:{line_number}: address {address:#x} does not match expected {len(output):#x}"
                )
        tokens = [token for token in re.split(r"[\s,]+", line.strip()) if token]
        if not tokens:
            continue
        if any(not HEX_BYTE.fullmatch(token) for token in tokens):
            raise PromFormatError(f"{source}:{line_number}: expected two-digit hexadecimal bytes")
        output.extend(int(token, 16) for token in tokens)

    if len(output) != DEVICE_BYTES:
        raise PromFormatError(
            f"{source}: listing has {len(output)} bytes; expected exactly {DEVICE_BYTES}"
        )
    return bytes(output)


def normalize_file(path: Path, input_format: str) -> tuple[bytes, bytes]:
    """Return declared-format normalized bytes and unmodified source bytes."""

    source_bytes = path.read_bytes()
    if input_format == "raw-binary":
        if len(source_bytes) != DEVICE_BYTES:
            raise PromFormatError(
                f"{path}: raw binary has {len(source_bytes)} bytes; expected {DEVICE_BYTES}"
            )
        return source_bytes, source_bytes
    try:
        text = source_bytes.decode("ascii")
    except UnicodeDecodeError as exc:
        raise PromFormatError(f"{path}: {input_format} input is not ASCII") from exc
    if input_format == "intel-hex":
        return parse_intel_hex(text, path), source_bytes
    if input_format == "hex-listing":
        return parse_hex_listing(text, path), source_bytes
    raise PromFormatError(f"{path}: unsupported input format {input_format!r}")


def parse_set_argument(specification: str) -> tuple[str, list[Path]]:
    """Parse NAME=FILE0,FILE1,FILE2,FILE3 without changing caller order."""

    if "=" not in specification:
        raise argparse.ArgumentTypeError("set must be NAME=FILE0,FILE1,FILE2,FILE3")
    name, file_list = specification.split("=", 1)
    name = name.strip()
    paths = [Path(value.strip()) for value in file_list.split(",") if value.strip()]
    if not name:
        raise argparse.ArgumentTypeError("set name must not be empty")
    if len(paths) != DEVICES_PER_SET:
        raise argparse.ArgumentTypeError(
            f"set {name!r} has {len(paths)} files; expected {DEVICES_PER_SET}"
        )
    return name, paths


def parse_format_argument(specification: str) -> tuple[str, str]:
    """Parse NAME=FORMAT and reject unrecognized declared formats."""

    if "=" not in specification:
        raise argparse.ArgumentTypeError("format must be NAME=FORMAT")
    name, input_format = (value.strip() for value in specification.split("=", 1))
    if not name or input_format not in INPUT_FORMATS:
        choices = ", ".join(INPUT_FORMATS)
        raise argparse.ArgumentTypeError(f"format must use one of: {choices}")
    return name, input_format


def first_difference(left: bytes, right: bytes) -> int | None:
    """Return the first distinct byte offset, or None when both devices match."""

    for offset, (left_byte, right_byte) in enumerate(zip(left, right, strict=True)):
        if left_byte != right_byte:
            return offset
    return None


def write_csv(path: Path, records: Iterable[DeviceRecord]) -> None:
    """Write a per-device CSV report after caller-selected normalization."""

    rows = [asdict(record) for record in records]
    with path.open("w", newline="", encoding="utf-8") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(DeviceRecord.__dataclass_fields__))
        writer.writeheader()
        writer.writerows(rows)


def build_parser() -> argparse.ArgumentParser:
    """Build the command-line contract for source-artifact comparison."""

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--set",
        dest="sets",
        action="append",
        required=True,
        metavar="NAME=FILE0,FILE1,FILE2,FILE3",
        help="one caller-ordered four-device artifact set; specify at least twice",
    )
    parser.add_argument(
        "--format",
        dest="formats",
        action="append",
        required=True,
        metavar="NAME=raw-binary|intel-hex|hex-listing",
        help="declared input representation for one named set",
    )
    parser.add_argument("--json-out", type=Path, help="write the complete comparison report")
    parser.add_argument("--csv-out", type=Path, help="write the per-device report")
    parser.add_argument(
        "--normalized-dir",
        type=Path,
        help="write explicitly requested derived raw bytes without changing inputs",
    )
    return parser


def compare_sets(
    parsed_sets: list[tuple[str, list[Path]]], format_by_set: dict[str, str], normalized_dir: Path | None
) -> dict[str, object]:
    """Normalize declared files and return a report without historical inference."""

    names = [name for name, _ in parsed_sets]
    if len(parsed_sets) < 2:
        raise PromFormatError("at least two independently named sets are required")
    if len(set(names)) != len(names):
        raise PromFormatError("set names must be unique")
    if set(format_by_set) != set(names):
        raise PromFormatError("every named set needs exactly one declared --format")

    normalized_sets: dict[str, list[bytes]] = {}
    device_records: list[DeviceRecord] = []
    for set_name, paths in parsed_sets:
        input_format = format_by_set[set_name]
        normalized_sets[set_name] = []
        for socket_index, path in enumerate(paths):
            normalized, source_bytes = normalize_file(path, input_format)
            normalized_sets[set_name].append(normalized)
            device_records.append(
                DeviceRecord(
                    set_name=set_name,
                    socket_index=socket_index,
                    source_path=str(path),
                    source_format=input_format,
                    source_size=len(source_bytes),
                    normalized_size=len(normalized),
                    source_sha256=sha256(source_bytes),
                    normalized_sha256=sha256(normalized),
                    sum8=f"{sum(normalized) & 0xFF:02x}",
                )
            )
            if normalized_dir is not None:
                normalized_dir.mkdir(parents=True, exist_ok=True)
                destination = normalized_dir / f"{set_name}-prom{socket_index}.bin"
                destination.write_bytes(normalized)

    set_records = []
    for set_name, devices in normalized_sets.items():
        assembled = b"".join(devices)
        if len(assembled) != ASSEMBLED_BYTES:
            raise AssertionError("internal assembled-image length error")
        set_records.append(
            {
                "set_name": set_name,
                "device_count": len(devices),
                "assembled_size": len(assembled),
                "assembled_sha256": sha256(assembled),
                "assembled_sum8": f"{sum(assembled) & 0xFF:02x}",
            }
        )

    comparisons: list[SocketComparison] = []
    for left_index, left_name in enumerate(names):
        for right_name in names[left_index + 1 :]:
            for socket_index in range(DEVICES_PER_SET):
                left = normalized_sets[left_name][socket_index]
                right = normalized_sets[right_name][socket_index]
                differing_bytes = sum(
                    left_byte != right_byte
                    for left_byte, right_byte in zip(left, right, strict=True)
                )
                comparisons.append(
                    SocketComparison(
                        left_set=left_name,
                        right_set=right_name,
                        socket_index=socket_index,
                        equal=differing_bytes == 0,
                        differing_bytes=differing_bytes,
                        first_difference=first_difference(left, right),
                    )
                )

    return {
        "schema": "intellec-mod40-prom-artifact-comparison-v2",
        "historical_claim": "none",
        "normalization_policy": {
            "devices_per_set": DEVICES_PER_SET,
            "bytes_per_device": DEVICE_BYTES,
            "assembled_bytes": ASSEMBLED_BYTES,
            "padding": "forbidden",
            "truncation": "forbidden",
            "format_detection": "forbidden; caller declares --format",
            "socket_order": "caller supplied; preserved",
            "polarity_transform": "not applied",
        },
        "devices": [asdict(record) for record in device_records],
        "sets": set_records,
        "comparisons": [asdict(comparison) for comparison in comparisons],
    }


def main(argv: list[str] | None = None) -> int:
    """Execute the artifact comparison and write only requested derived reports."""

    args = build_parser().parse_args(argv)
    parsed_sets = [parse_set_argument(specification) for specification in args.sets]
    parsed_formats = [parse_format_argument(specification) for specification in args.formats]
    format_by_set = dict(parsed_formats)
    if len(format_by_set) != len(parsed_formats):
        raise PromFormatError("each set may declare --format only once")
    report = compare_sets(parsed_sets, format_by_set, args.normalized_dir)
    if args.csv_out is not None:
        write_csv(args.csv_out, [DeviceRecord(**record) for record in report["devices"]])
    if args.json_out is not None:
        args.json_out.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    json.dump(report, sys.stdout, indent=2)
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, PromFormatError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        raise SystemExit(2) from exc
