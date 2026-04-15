#!/usr/bin/env bash
set -euo pipefail
# Simple link check: verify local markdown links exist
ROOT_DIR=$(git rev-parse --show-toplevel)
EXIT=0

collect_files() {
  git ls-files "*.md" ":(exclude).claude_plans/**"
}

extract_links() {
  # Capture Markdown links and images: [text](target) and ![alt](target).
  grep -oE '!?\[[^]]*\]\([^)]+\)' "$1" | sed -E 's/^!?\[[^]]*\]\(([^)]+)\)$/\1/'
}

normalize_target() {
  local target="${1%%[[:space:]]*}" # drop optional title
  target="${target#<}"
  target="${target%>}"
  target="${target%%#*}" # drop anchor suffix
  printf '%s' "$target"
}

while IFS= read -r file; do
  while IFS= read -r raw_target; do
    path=$(normalize_target "$raw_target")
    [[ -z "$path" ]] && continue

    # Skip external and non-file links.
    if [[ "$path" =~ ^[A-Za-z][A-Za-z0-9+.-]*: ]]; then
      continue
    fi
    case "$path" in
      *.md|*.yaml|*.yml) ;;
      *) continue ;;
    esac

    if [[ "$path" = /* ]]; then
      resolved="$ROOT_DIR$path"
    else
      resolved=$(realpath -m "$(dirname "$file")/$path")
    fi

    if [[ ! -f "$resolved" ]]; then
      echo "Broken link in $file -> $path" >&2
      EXIT=1
    fi
  done < <(extract_links "$file")
done < <(collect_files)
exit $EXIT
