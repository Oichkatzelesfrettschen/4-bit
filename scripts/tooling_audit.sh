#!/usr/bin/env bash
set -euo pipefail

need_cmd() {
  local c="$1"
  if ! command -v "$c" >/dev/null 2>&1; then
    echo "MISS $c"
    return 1
  fi
  echo "OK   $c -> $(command -v "$c")"
}

version_1line() {
  local c="$1"
  shift || true
  if command -v "$c" >/dev/null 2>&1; then
    ("$c" "$@" 2>&1 | head -n 1) || true
  fi
}

python_mod() {
  local mod="$1"
  python - <<PY
import importlib
try:
  m = importlib.import_module("$mod")
  print("OK   python:$mod", getattr(m, "__version__", "unknown"))
except Exception as e:
  print("MISS python:$mod", type(e).__name__ + ":", e)
PY
}

echo "== Core CLI =="
need_cmd git
version_1line git --version
need_cmd rg
version_1line rg --version
need_cmd yq
version_1line yq --version

echo
echo "== PDF + OCR =="
need_cmd ocrmypdf
version_1line ocrmypdf --version
need_cmd tesseract
version_1line tesseract --version
need_cmd qpdf
version_1line qpdf --version
need_cmd pdftotext
version_1line pdftotext -v
need_cmd pdfimages
version_1line pdfimages -v
need_cmd pdftoppm
version_1line pdftoppm -v

echo
echo "== Image + Metadata =="
need_cmd magick
version_1line magick -version
need_cmd identify
version_1line identify --version
need_cmd exiftool
version_1line exiftool -ver

echo
echo "== Acquisition =="
need_cmd aria2c
version_1line aria2c --version
need_cmd wget2
version_1line wget2 --version
need_cmd axel
version_1line axel --version

echo
echo "== Rust Tooling =="
need_cmd cargo
version_1line cargo --version
need_cmd rustc
version_1line rustc --version
for c in cargo-deny cargo-audit cargo-llvm-cov; do
  if command -v "$c" >/dev/null 2>&1; then
    echo "OK   $c"
    if [[ "$c" == "cargo-llvm-cov" ]]; then
      # On some distros, the packaged binary expects `cargo llvm-cov ...`.
      version_1line cargo llvm-cov --version
    else
      version_1line "$c" --version
    fi
  else
    echo "MISS $c"
  fi
done

echo
echo "== Python Stack =="
need_cmd python
version_1line python --version
python_mod pdfplumber
python_mod pdfminer
python_mod pytesseract
python_mod PIL
python_mod cv2
python_mod ocrmypdf
python_mod pikepdf
python_mod fitz

echo
echo "== pipx =="
if command -v pipx >/dev/null 2>&1; then
  echo "OK   pipx"
  version_1line pipx --version
  # Summary only; avoid dumping full list in CI logs.
  pipx list | head -n 30 || true
else
  echo "MISS pipx"
fi
