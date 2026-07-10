#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from typing import Any


def _run(cmd: list[str], *, timeout: float = 10.0) -> tuple[int, str]:
    try:
        # Fixed argv lists probing local toolchain binaries; no shell, no untrusted input.
        p = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout, check=False)  # noqa: S603
        out = (p.stdout or "") + (p.stderr or "")
        return int(p.returncode), out.strip()
    except subprocess.TimeoutExpired:
        return 124, "timeout"


def main() -> int:
    ap = argparse.ArgumentParser(description="Quick smoke-check for OCR-related toolchain on this machine.")
    ap.add_argument("--json", action="store_true", help="Emit machine-readable JSON.")
    args = ap.parse_args()

    report: dict[str, Any] = {"python": sys.version, "checks": {}}

    def add(name: str, ok: bool, detail: str | None = None) -> None:
        report["checks"][name] = {"ok": bool(ok)}
        if detail is not None:
            report["checks"][name]["detail"] = str(detail)

    # External tools.
    tesseract = shutil.which("tesseract")
    add("tesseract_in_path", tesseract is not None, tesseract)
    if tesseract:
        rc, out = _run([tesseract, "--version"], timeout=5.0)
        add("tesseract_version", rc == 0, out.splitlines()[0] if out else out)

    nvidia_smi = shutil.which("nvidia-smi")
    add("nvidia_smi_in_path", nvidia_smi is not None, nvidia_smi)
    if nvidia_smi:
        rc, out = _run([nvidia_smi, "-L"], timeout=5.0)
        add("nvidia_smi_list", rc == 0, out)

    # Python modules.
    for mod in ("cv2", "numpy", "PIL", "pytesseract", "onnxruntime"):
        try:
            __import__(mod)
            add(f"py_import_{mod}", True)
        except Exception as e:  # noqa: BLE001
            add(f"py_import_{mod}", False, repr(e))

    # Providers (if onnxruntime present).
    try:
        import onnxruntime as ort  # type: ignore

        add("onnxruntime_providers", True, json.dumps(ort.get_available_providers()))
    except Exception as e:  # noqa: BLE001
        add("onnxruntime_providers", False, repr(e))

    if args.json:
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        # Human summary.
        for k, v in report["checks"].items():
            status = "OK" if v.get("ok") else "FAIL"
            detail = v.get("detail")
            if detail:
                print(f"{status:4} {k}: {detail}")
            else:
                print(f"{status:4} {k}")

    # Non-zero if a critical check fails.
    critical = ["tesseract_in_path", "py_import_cv2", "py_import_pytesseract"]
    if any(not report["checks"].get(c, {}).get("ok") for c in critical):
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

