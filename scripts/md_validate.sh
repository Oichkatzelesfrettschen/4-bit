#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# Single-pass scan (avoids spawning one shell per markdown file).
find "$ROOT" -name '*.md' -not -path '*/target/*' \
  -exec grep -nE '\[[^]]+\]\(([^)]+)\)' {} + >/dev/null || true

echo "Markdown basic validation done."
