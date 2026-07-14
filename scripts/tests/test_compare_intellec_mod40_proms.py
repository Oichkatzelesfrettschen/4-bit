from __future__ import annotations

import json
from pathlib import Path

import pytest
from scripts.compare_intellec_mod40_proms import (
    DEVICE_BYTES,
    PromFormatError,
    compare_sets,
    main,
    normalize_file,
    parse_hex_listing,
    parse_intel_hex,
)


def intel_hex_record(address: int, record_type: int, payload: bytes) -> str:
    body = bytes([len(payload)]) + address.to_bytes(2, "big") + bytes([record_type]) + payload
    checksum = (-sum(body)) & 0xFF
    return ":" + (body + bytes([checksum])).hex().upper()


def write_set(directory: Path, name: str, values: list[bytes], suffix: str = ".bin") -> list[Path]:
    paths = []
    for index, value in enumerate(values):
        path = directory / f"{name}-{index}{suffix}"
        path.write_bytes(value)
        paths.append(path)
    return paths


def test_parse_intel_hex_requires_one_complete_device() -> None:
    image = bytes(range(DEVICE_BYTES))
    records = [intel_hex_record(offset, 0, image[offset : offset + 16]) for offset in range(0, 256, 16)]
    text = "\n".join([*records, intel_hex_record(0, 1, b"")]) + "\n"

    assert parse_intel_hex(text, Path("device.hex")) == image


def test_declared_intel_hex_preamble_preserves_source_metadata_without_relaxing_records() -> None:
    image = bytes(range(DEVICE_BYTES))
    records = [intel_hex_record(offset, 0, image[offset : offset + 16]) for offset in range(0, 256, 16)]
    text = "reader note\noperator note\n" + "\n".join([*records, intel_hex_record(0, 1, b"")]) + "\n"

    with pytest.raises(PromFormatError, match="expected Intel HEX record"):
        parse_intel_hex(text, Path("device.hex"))
    assert parse_intel_hex(text, Path("device.hex"), allow_preamble=True) == image

    malformed = text + "trailing note\n"
    with pytest.raises(PromFormatError, match="expected Intel HEX record"):
        parse_intel_hex(malformed, Path("device.hex"), allow_preamble=True)


def test_hex_listing_preserves_explicit_addresses() -> None:
    image = bytes(range(DEVICE_BYTES))
    text = "\n".join(
        f"{offset:04x}: " + " ".join(f"{value:02x}" for value in image[offset : offset + 16])
        for offset in range(0, 256, 16)
    )

    assert parse_hex_listing(text, Path("device.txt")) == image
    with pytest.raises(PromFormatError, match="does not match expected"):
        parse_hex_listing("0010: 00\n", Path("sparse.txt"))


def test_declared_listing_preamble_accepts_only_prose_before_listing_data() -> None:
    image = bytes(range(DEVICE_BYTES))
    lines = ["operator transcription"]
    for offset in range(0, 256, 16):
        line = " ".join(f"{value:02x}" for value in image[offset : offset + 16])
        lines.append(line)
    text = "\n".join(lines)

    with pytest.raises(PromFormatError, match="expected two-digit"):
        parse_hex_listing(text, Path("listing.txt"))
    assert (
        parse_hex_listing(text, Path("listing.txt"), allow_preamble=True) == image
    )

    malformed = text + "\ntrailing note\n"
    with pytest.raises(PromFormatError, match="expected two-digit"):
        parse_hex_listing(malformed, Path("listing.txt"), allow_preamble=True)

    ambiguous = text + "\n05 <-- editorial correction\n"
    with pytest.raises(PromFormatError, match="expected two-digit"):
        parse_hex_listing(ambiguous, Path("listing.txt"), allow_preamble=True)


def test_raw_binary_accepts_ascii_shaped_bytes_only_when_declared(tmp_path: Path) -> None:
    raw_path = tmp_path / "raw.bin"
    raw_path.write_bytes(b"A" * DEVICE_BYTES)

    normalized, source = normalize_file(raw_path, "raw-binary")

    assert normalized == b"A" * DEVICE_BYTES
    assert source == normalized
    with pytest.raises(PromFormatError, match="expected two-digit"):
        normalize_file(raw_path, "hex-listing")


def test_declared_intel_hex_preamble_normalizes_only_a_declared_preamble(tmp_path: Path) -> None:
    image = bytes(range(DEVICE_BYTES))
    records = [intel_hex_record(offset, 0, image[offset : offset + 16]) for offset in range(0, 256, 16)]
    path = tmp_path / "reader-output.hex"
    path.write_text("reader output\n" + "\n".join([*records, intel_hex_record(0, 1, b"")]) + "\n", encoding="ascii")

    normalized, _ = normalize_file(path, "intel-hex-preamble")
    assert normalized == image
    with pytest.raises(PromFormatError, match="expected Intel HEX record"):
        normalize_file(path, "intel-hex")


def test_compare_sets_preserves_caller_order_and_does_not_transform(tmp_path: Path) -> None:
    left_paths = write_set(tmp_path, "left", [bytes([index]) * DEVICE_BYTES for index in range(4)])
    right_values = [bytes([index]) * DEVICE_BYTES for index in range(4)]
    right_values[2] = bytes([0xAA]) * DEVICE_BYTES
    right_paths = write_set(tmp_path, "right", right_values)

    report = compare_sets(
        [("left", left_paths), ("right", right_paths)],
        {"left": "raw-binary", "right": "raw-binary"},
        None,
    )

    assert report["historical_claim"] == "none"
    assert report["normalization_policy"]["polarity_transform"] == "not applied"
    assert report["comparisons"][2] == {
        "left_set": "left",
        "right_set": "right",
        "socket_index": 2,
        "equal": False,
        "differing_bytes": DEVICE_BYTES,
        "first_difference": 0,
    }


def test_main_writes_requested_machine_readable_reports(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    left_paths = write_set(tmp_path, "left", [bytes([0x10]) * DEVICE_BYTES for _ in range(4)])
    right_paths = write_set(tmp_path, "right", [bytes([0x10]) * DEVICE_BYTES for _ in range(4)])
    json_path = tmp_path / "report.json"
    csv_path = tmp_path / "report.csv"
    normalized_dir = tmp_path / "normalized"

    assert (
        main(
            [
                "--set",
                "left=" + ",".join(str(path) for path in left_paths),
                "--set",
                "right=" + ",".join(str(path) for path in right_paths),
                "--format",
                "left=raw-binary",
                "--format",
                "right=raw-binary",
                "--json-out",
                str(json_path),
                "--csv-out",
                str(csv_path),
                "--normalized-dir",
                str(normalized_dir),
            ]
        )
        == 0
    )

    assert json.loads(json_path.read_text(encoding="utf-8"))["schema"].endswith("v2")
    csv_rows = csv_path.read_text(encoding="utf-8").splitlines()
    assert sum(row.startswith("left,") for row in csv_rows) == 4
    assert len(list(normalized_dir.glob("*.bin"))) == 8
    assert json.loads(capsys.readouterr().out)["comparisons"][0]["equal"] is True
