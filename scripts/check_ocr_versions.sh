#!/usr/bin/env bash
# CI gate: verify OCR toolchain versions match pinned versions
# Exit code 0: versions match
# Exit code 1: version mismatch detected

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# Pinned versions (update these when intentionally upgrading toolchain)
# Last updated: 2026-01-29
PINNED_TESSERACT_VERSION="5.5.2"
PINNED_OPENCV_VERSION="4.13.0"
PINNED_PYTESSERACT_VERSION="0.3.13"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

check_version() {
    local tool="$1"
    local expected="$2"
    local actual="$3"

    if [[ "$actual" == "$expected" ]]; then
        echo -e "${GREEN}✓${NC} $tool: $actual (matches pinned version)"
        return 0
    else
        echo -e "${RED}✗${NC} $tool: $actual (expected $expected)"
        return 1
    fi
}

check_version_prefix() {
    local tool="$1"
    local expected="$2"
    local actual="$3"

    # Check if actual version starts with expected prefix
    if [[ "$actual" == "$expected"* ]]; then
        echo -e "${GREEN}✓${NC} $tool: $actual (matches pinned prefix $expected)"
        return 0
    else
        echo -e "${RED}✗${NC} $tool: $actual (expected prefix $expected)"
        return 1
    fi
}

echo "=== OCR Toolchain Version Check ==="
echo ""

FAILED=0

# Check Tesseract
if command -v tesseract &> /dev/null; then
    TESSERACT_VERSION=$(tesseract --version 2>&1 | head -1 | grep -oP '(?<=tesseract )\S+')
    check_version_prefix "Tesseract OCR" "$PINNED_TESSERACT_VERSION" "$TESSERACT_VERSION" || FAILED=1
else
    echo -e "${RED}✗${NC} Tesseract OCR: NOT INSTALLED"
    FAILED=1
fi

# Check pytesseract
PYTESSERACT_VERSION=$(python3 -c "import pytesseract; print(pytesseract.__version__)" 2>/dev/null || echo "NOT_INSTALLED")
if [[ "$PYTESSERACT_VERSION" != "NOT_INSTALLED" ]]; then
    check_version_prefix "pytesseract" "$PINNED_PYTESSERACT_VERSION" "$PYTESSERACT_VERSION" || FAILED=1
else
    echo -e "${RED}✗${NC} pytesseract: NOT INSTALLED"
    FAILED=1
fi

# Check OpenCV
OPENCV_VERSION=$(python3 -c "import cv2; print(cv2.__version__)" 2>/dev/null || echo "NOT_INSTALLED")
if [[ "$OPENCV_VERSION" != "NOT_INSTALLED" ]]; then
    check_version_prefix "OpenCV" "$PINNED_OPENCV_VERSION" "$OPENCV_VERSION" || FAILED=1
else
    echo -e "${RED}✗${NC} OpenCV: NOT INSTALLED"
    FAILED=1
fi

# Check numpy (informational only, not gated)
NUMPY_VERSION=$(python3 -c "import numpy; print(numpy.__version__)" 2>/dev/null || echo "NOT_INSTALLED")
if [[ "$NUMPY_VERSION" != "NOT_INSTALLED" ]]; then
    echo -e "${GREEN}ℹ${NC} numpy: $NUMPY_VERSION (informational)"
else
    echo -e "${YELLOW}⚠${NC} numpy: NOT INSTALLED"
fi

# Check onnxruntime (optional, informational only)
ONNXRUNTIME_VERSION=$(python3 -c "import onnxruntime; print(onnxruntime.__version__)" 2>/dev/null || echo "NOT_INSTALLED")
if [[ "$ONNXRUNTIME_VERSION" != "NOT_INSTALLED" ]]; then
    echo -e "${GREEN}ℹ${NC} onnxruntime: $ONNXRUNTIME_VERSION (optional)"
else
    echo -e "${YELLOW}⚠${NC} onnxruntime: NOT INSTALLED (optional)"
fi

echo ""

if [[ $FAILED -eq 0 ]]; then
    echo -e "${GREEN}✓ All required OCR toolchain versions match${NC}"
    exit 0
else
    echo -e "${RED}✗ OCR toolchain version mismatch detected${NC}"
    echo ""
    echo "To update pinned versions (if intentional upgrade):"
    echo "  1. Edit $0"
    echo "  2. Update PINNED_* variables"
    echo "  3. Re-run extraction to verify outputs are reproducible"
    echo "  4. Commit updated script with new pins"
    exit 1
fi
