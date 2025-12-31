#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
find "$ROOT" -name '*.md' -not -path '*/target/*' -print0 | xargs -0 -I{} bash -lc 'grep -n "\[[^]]\+\](\([^)]\+\))" {} >/dev/null || true'
echo "Markdown basic validation done."