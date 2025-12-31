#!/usr/bin/env bash
set -euo pipefail
# Simple link check: verify local markdown links exist
ROOT_DIR=$(git rev-parse --show-toplevel)
EXIT=0
while IFS= read -r file; do
  while IFS= read -r link; do
    path=${link#(#*)}
    if [[ -n "$path" && -f "$ROOT_DIR/$path" ]]; then
      :
    else
      echo "Broken link in $file -> $path" >&2
      EXIT=1
    fi
  done < <(grep -oE "\([^)#]+\.(md|yaml)\)" "$file" | tr -d '()')
done < <(git ls-files "*.md")
exit $EXIT
