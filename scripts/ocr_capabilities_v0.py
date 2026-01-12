#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
import platform
import subprocess
import sys
from pathlib import Path


def _run(cmd: list[str]) -> dict[str, object]:
    try:
        p = subprocess.run(cmd, check=False, text=True, capture_output=True)
        return {
            "cmd": cmd,
            "returncode": int(p.returncode),
            "stdout": (p.stdout or "").strip(),
            "stderr": (p.stderr or "").strip(),
        }
    except Exception as e:
        return {"cmd": cmd, "error": repr(e)}


def main() -> int:
    ap = argparse.ArgumentParser(description="Probe OCR-related capabilities (CPU/GPU/ONNX/OpenCV/Tesseract).")
    ap.add_argument("--out", type=Path, default=None, help="Write JSON here (default: stdout).")
    args = ap.parse_args()

    payload: dict[str, object] = {
        "python": {
            "executable": sys.executable,
            "version": sys.version,
            "implementation": platform.python_implementation(),
        },
        "platform": {
            "system": platform.system(),
            "release": platform.release(),
            "machine": platform.machine(),
        },
        "env": {
            "OCR_ONNX_MODEL": os.environ.get("OCR_ONNX_MODEL", ""),
        },
        "tesseract": {},
        "pytesseract": {},
        "onnxruntime": {},
        "opencv": {},
        "torch": {},
        "system": {},
    }

    # Tesseract + pytesseract
    try:
        import pytesseract  # type: ignore

        payload["pytesseract"] = {"version": getattr(pytesseract, "__version__", "unknown")}
        try:
            payload["tesseract"] = {"version": str(pytesseract.get_tesseract_version())}
        except Exception as e:  # pragma: no cover
            payload["tesseract"] = {"error": repr(e)}
    except Exception as e:  # pragma: no cover
        payload["pytesseract"] = {"error": repr(e)}

    # ONNX Runtime
    try:
        import onnxruntime as ort  # type: ignore

        payload["onnxruntime"] = {
            "version": getattr(ort, "__version__", "unknown"),
            "providers": list(ort.get_available_providers()),
            "device": _run(["python3", "-c", "import onnxruntime as ort; print(ort.get_device())"]),
        }
    except Exception as e:  # pragma: no cover
        payload["onnxruntime"] = {"error": repr(e)}

    # OpenCV
    try:
        import cv2  # type: ignore

        cuda_count = None
        try:
            if hasattr(cv2, "cuda"):
                cuda_count = int(cv2.cuda.getCudaEnabledDeviceCount())
        except Exception:
            cuda_count = None
        payload["opencv"] = {
            "version": getattr(cv2, "__version__", "unknown"),
            "cuda_device_count": cuda_count,
        }
    except Exception as e:  # pragma: no cover
        payload["opencv"] = {"error": repr(e)}

    # Torch (optional; useful for OCR engines like EasyOCR/PaddleOCR variants)
    try:
        import torch  # type: ignore

        payload["torch"] = {
            "version": getattr(torch, "__version__", "unknown"),
            "cuda_is_available": bool(getattr(torch, "cuda", None) and torch.cuda.is_available()),
            "cuda_device_count": int(torch.cuda.device_count()) if getattr(torch, "cuda", None) else 0,
            "cuda_device0": (
                str(torch.cuda.get_device_name(0)) if getattr(torch, "cuda", None) and torch.cuda.device_count() > 0 else ""
            ),
        }
    except Exception as e:  # pragma: no cover
        payload["torch"] = {"error": repr(e)}

    # System tools (best-effort)
    payload["system"] = {
        "nvidia_smi": _run(["nvidia-smi", "-L"]),
        "nvcc": _run(["nvcc", "--version"]),
        "lscpu": _run(["lscpu"]),
    }

    text = json.dumps(payload, indent=2, sort_keys=True) + "\n"
    if args.out is None:
        print(text, end="")
    else:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(text, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

