#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
REG="$ROOT_DIR/docs/meta/registry.yaml"
INDEX="$ROOT_DIR/docs/INDEX.md"
ROADMAP="$ROOT_DIR/docs/ROADMAP.md"
[ -f "$REG" ] || { echo "Missing registry.yaml"; exit 1; }
if ! grep -q "Sync with mcs4-emu/STATUS.md" "$ROADMAP"; then echo "- Sync with mcs4-emu/STATUS.md" >> "$ROADMAP"; fi
if command -v yq >/dev/null 2>&1; then
  cat > "$INDEX" <<EOF
# Documentation Index

$(yq '.docs[].file' "$REG" | sed 's/^/- /')
EOF
else
  echo "yq not found; leaving INDEX.md as-is"
fi
echo "Doc sync complete."