#!/usr/bin/env bash
set -euo pipefail
# Minimal markdown lint: ensure files have title and under 200-char lines
EXIT=0
while IFS= read -r file; do
  if ! head -n1 "$file" | grep -q '^#'; then
    echo "Missing title in $file" >&2
    EXIT=1
  fi
  if awk 'length($0)>200{print FILENAME":"NR" too long";exit 1}' "$file"; then :; else EXIT=1; fi
done < <(git ls-files "*.md")
exit $EXIT
