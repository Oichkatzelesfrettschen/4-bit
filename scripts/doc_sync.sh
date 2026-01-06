#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
REG="$ROOT_DIR/docs/meta/registry.yaml"
INDEX="$ROOT_DIR/docs/INDEX.md"
ROADMAP="$ROOT_DIR/docs/ROADMAP.md"
[ -f "$REG" ] || { echo "Missing registry.yaml"; exit 1; }
if ! grep -q "STATUS.md" "$ROADMAP"; then
  echo "- Sync with mcs4-emu/STATUS.md" >> "$ROADMAP"
fi
if command -v yq >/dev/null 2>&1; then
  DOC_LIST="$(yq '.docs[].file' "$REG" | sed 's/^/- /')"
  if grep -q "DOCS_REGISTRY_START" "$INDEX"; then
    awk -v list="$DOC_LIST" '
      BEGIN { count = split(list, lines, "\n") }
      /DOCS_REGISTRY_START/ {
        print;
        for (i = 1; i <= count; i++) {
          if (lines[i] != "") {
            print lines[i];
          }
        }
        in_block = 1;
        next;
      }
      /DOCS_REGISTRY_END/ {
        in_block = 0;
        print;
        next;
      }
      !in_block { print; }
    ' "$INDEX" > "$INDEX.tmp"
    mv "$INDEX.tmp" "$INDEX"
  else
    cat > "$INDEX" <<EOF
# Documentation Index

$(printf "%s\n" "$DOC_LIST")
EOF
  fi
else
  echo "yq not found; leaving INDEX.md as-is"
fi
echo "Doc sync complete."
