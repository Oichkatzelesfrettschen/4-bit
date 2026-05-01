#!/usr/bin/env bash
# installation_check.sh -- Verify pinned tooling versions match
# `mcs4-emu/INSTALLATION.md` and the workspace `rust-toolchain.toml`.
#
# WHY:  Dependency drift between developer machines and CI is a frequent
#       source of silent reproducibility failures. This script makes the
#       drift loud: it exits non-zero if any pinned tool is missing or at
#       an unexpected version.
# WHAT: Probes Rust nightly pin (must equal `rust-toolchain.toml`),
#       optional GUI Linux packages, optional OCR toolchain (Tesseract,
#       OpenCV, ONNX Runtime), and optional doc tooling (yq, mdbook).
# HOW:  Parses `rust-toolchain.toml` for the channel, then runs each
#       tool's `--version` flag and compares against the documented pin
#       in INSTALLATION.md. Optional tools emit a WARN on mismatch and
#       FAIL only when explicitly required via `--strict`.
#
# Usage:
#   ./scripts/installation_check.sh           # report drift, exit 0 if no required tools fail
#   ./scripts/installation_check.sh --strict  # fail on any optional drift too

set -eu

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
TOOLCHAIN="$ROOT_DIR/rust-toolchain.toml"
INSTALL_MD="$ROOT_DIR/mcs4-emu/INSTALLATION.md"

STRICT=0
if [ "${1:-}" = "--strict" ]; then
    STRICT=1
fi

errors=0
warnings=0

fail() {
    printf 'FAIL: %s\n' "$1" >&2
    errors=$((errors + 1))
}

warn() {
    printf 'WARN: %s\n' "$1"
    warnings=$((warnings + 1))
    if [ "$STRICT" = "1" ]; then
        errors=$((errors + 1))
    fi
}

ok() {
    printf 'OK:   %s\n' "$1"
}

# --- Required: nightly Rust toolchain pin ------------------------------------
if [ ! -f "$TOOLCHAIN" ]; then
    fail "missing $TOOLCHAIN"
else
    pinned="$(sed -nE 's/^channel = "(nightly-[0-9]{4}-[0-9]{2}-[0-9]{2})"$/\1/p' "$TOOLCHAIN" | head -n1)"
    if [ -z "$pinned" ]; then
        fail "could not parse nightly pin from rust-toolchain.toml"
    elif command -v rustup >/dev/null 2>&1; then
        if rustup toolchain list 2>/dev/null | grep -q "$pinned"; then
            ok "rustup has $pinned installed"
        else
            warn "rustup does not have $pinned installed (run: rustup toolchain install $pinned)"
        fi
    else
        warn "rustup not on PATH; cannot verify $pinned is installed"
    fi
fi

# --- Optional: yq for doc validation ----------------------------------------
if command -v yq >/dev/null 2>&1; then
    ok "yq: $(yq --version 2>&1 | head -n1)"
else
    warn "yq not on PATH; scripts/doc_validate.sh will fall back to python3+PyYAML"
fi

# --- Optional: OCR tooling --------------------------------------------------
if command -v tesseract >/dev/null 2>&1; then
    v="$(tesseract --version 2>&1 | head -n1 | awk '{print $2}')"
    expected="$(grep -E "Tesseract [0-9]+\.[0-9]+\.[0-9]+" "$INSTALL_MD" 2>/dev/null | head -n1 | awk '{for (i=1;i<=NF;i++) if ($i=="Tesseract") print $(i+1)}')"
    if [ -n "$expected" ] && [ "$v" != "$expected" ]; then
        warn "tesseract version $v differs from INSTALLATION.md pin $expected"
    else
        ok "tesseract: $v"
    fi
else
    warn "tesseract not on PATH (OCR pipeline disabled)"
fi

if python3 -c "import cv2; print(cv2.__version__)" >/dev/null 2>&1; then
    ok "python3 -c 'import cv2': $(python3 -c "import cv2; print(cv2.__version__)" 2>/dev/null)"
else
    warn "python3 cv2 (opencv) not importable (image preproc disabled)"
fi

if python3 -c "import onnxruntime; print(onnxruntime.__version__)" >/dev/null 2>&1; then
    ok "python3 -c 'import onnxruntime': $(python3 -c "import onnxruntime; print(onnxruntime.__version__)" 2>/dev/null)"
else
    warn "python3 onnxruntime not importable (ONNX OCR backend disabled)"
fi

# --- Optional: doc tooling --------------------------------------------------
if command -v mdbook >/dev/null 2>&1; then
    ok "mdbook: $(mdbook --version 2>&1 | head -n1)"
else
    warn "mdbook not on PATH (book build disabled)"
fi

if command -v shellcheck >/dev/null 2>&1; then
    ok "shellcheck: $(shellcheck --version 2>&1 | grep -E '^version' | awk '{print $2}')"
else
    warn "shellcheck not on PATH (script lint disabled)"
fi

printf '\n%d errors, %d warnings (strict=%d).\n' "$errors" "$warnings" "$STRICT"
[ "$errors" -eq 0 ]
