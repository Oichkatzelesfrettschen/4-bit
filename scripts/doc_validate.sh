#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
REG="$ROOT_DIR/docs/meta/registry.yaml"
command -v yq >/dev/null 2>&1 || { echo "Install yq for YAML validation"; exit 1; }
yq e '.metadata.version and .docs' "$REG" >/dev/null || { echo "Invalid registry schema"; exit 1; }
missing=0
for f in $(yq '.docs[].file' "$REG"); do [ -f "$ROOT_DIR/$f" ] || { echo "Missing doc: $f"; missing=1; }; done
[ "$missing" -eq 0 ] || exit 1
echo "Docs validated."