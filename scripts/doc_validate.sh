#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
REG="$ROOT_DIR/docs/meta/registry.yaml"

# Parse registry.yaml -- prefer yq, fall back to python3
if command -v yq >/dev/null 2>&1; then
    yq e '.metadata.version and .docs' "$REG" >/dev/null || { echo "Invalid registry schema"; exit 1; }
    files=$(yq '.docs[].file' "$REG")
elif command -v python3 >/dev/null 2>&1; then
    files=$(python3 -c "
import yaml, sys
with open(sys.argv[1]) as f:
    data = yaml.safe_load(f)
if not (data.get('metadata', {}).get('version') and data.get('docs')):
    print('Invalid registry schema', file=sys.stderr)
    sys.exit(1)
for doc in data['docs']:
    print(doc['file'])
" "$REG")
else
    echo "Install yq or python3 (with PyYAML) for YAML validation"
    exit 1
fi

missing=0
while IFS= read -r f; do
    [ -f "$ROOT_DIR/$f" ] || { echo "Missing doc: $f"; missing=1; }
done <<< "$files"

[ "$missing" -eq 0 ] || exit 1
echo "Docs validated."
