#!/usr/bin/env bash
set -euo pipefail

# Minimal markdown lint: ensure docs have a title and bounded prose line width.
# Evidence/archive snapshots are intentionally excluded because they contain
# machine-generated artifacts and long forensic payloads.
MAX_LEN="${MD_MAX_LEN:-200}"
EXIT=0

collect_files() {
  git ls-files "*.md" \
    ":(exclude).claude_plans/**" \
    ":(exclude)docs/archive/**" \
    ":(exclude)docs/evidence/**"
}

while IFS= read -r file; do
  if ! head -n1 "$file" | grep -q '^#'; then
    echo "Missing title in $file" >&2
    EXIT=1
  fi
  if awk -v max_len="$MAX_LEN" '
    BEGIN { in_fence = 0 }
    /^```/ { in_fence = !in_fence; next }
    in_fence { next }
    length($0) > max_len &&
      $0 !~ /https?:\/\// &&
      $0 !~ /^[[:space:]]*\|/ &&
      $0 !~ /^[[:space:]]*([-*+]|[0-9]+[.)])[[:space:]]/ {
      printf "%s:%d too long\n", FILENAME, NR
      exit 1
    }
  ' "$file"; then :; else EXIT=1; fi
done < <(collect_files)
exit $EXIT
