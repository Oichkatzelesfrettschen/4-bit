#!/usr/bin/env bash
# Audit that every Markdown / YAML / JSON / BibTeX evidence file under docs/
# (and the canonical top-level docs) is listed in docs/meta/registry.yaml.
# Fails non-zero on any drift. Add new entries to registry.yaml or move
# obsolete docs to docs/archive/ to silence.
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
REG="$ROOT_DIR/docs/meta/registry.yaml"
[ -f "$REG" ] || { echo "Missing registry.yaml"; exit 1; }

# Extract the list of registered file paths.
if command -v yq >/dev/null 2>&1; then
    REGISTERED="$(yq -r '.docs[].file' "$REG" | sort -u)"
elif command -v python3 >/dev/null 2>&1; then
    REGISTERED="$(python3 -c '
import yaml, sys
with open(sys.argv[1]) as f: data = yaml.safe_load(f)
for d in data.get("docs", []):
    print(d["file"])
' "$REG" | sort -u)"
else
    echo "Need yq or python3 (with PyYAML) for registry audit" >&2
    exit 2
fi

# Enumerate files that should be registered:
#   - top-level *.md known to be canonical (README, claude, etc. and snapshots)
#   - docs/*.md (excluding archive/)
#   - docs/photomicrographs/README.md, docs/emulators/README.md
#   - docs/evidence/*.md and selected non-md evidence (yaml/json/bib)
#   - docs/archive/*.md (so the registry tracks what was archived)
#   - mcs4-emu/{CLAUDE,STATUS,INSTALLATION,requirements}.md
collect() {
    cd "$ROOT_DIR"
    {
        for f in README.md claude.md ARCHITECTURE.md requirements.md \
                 NEXT_STEPS.md SCOPING_ASSESSMENT.md PHASE_2_CHECKPOINT.md; do
            [ -f "$f" ] && echo "$f"
        done
        find docs -maxdepth 2 -type f \( -name '*.md' -o -name '*.bib' \) \
            -not -path 'docs/evidence/*' -print
        find docs/evidence -maxdepth 1 -type f \
            \( -name '*.md' -o -name 'ocr_manifest.yaml' \
               -o -name 'source_manifest.json' -o -name 'bibliography.bib' \) -print
        for f in mcs4-emu/CLAUDE.md mcs4-emu/STATUS.md mcs4-emu/INSTALLATION.md \
                 mcs4-emu/requirements.md; do
            [ -f "$f" ] && echo "$f"
        done
        # Per-crate REQUIREMENTS files (debt phase D0.5).
        for c in mcs4-bus mcs4-chips mcs4-core mcs4-fpga mcs4-gui \
                 mcs4-intellec mcs4-periph mcs4-system; do
            f="mcs4-emu/crates/$c/REQUIREMENTS.md"
            [ -f "$f" ] && echo "$f"
        done
    } | sort -u
}

EXPECTED="$(collect)"

drift=0
while IFS= read -r f; do
    [ -z "$f" ] && continue
    if ! printf '%s\n' "$REGISTERED" | grep -qxF "$f"; then
        echo "Unregistered: $f"
        drift=1
    fi
done <<<"$EXPECTED"

# Reverse direction: registered files that no longer exist.
while IFS= read -r f; do
    [ -z "$f" ] && continue
    if [ ! -e "$ROOT_DIR/$f" ]; then
        echo "Stale registry entry: $f (file missing)"
        drift=1
    fi
done <<<"$REGISTERED"

if [ "$drift" -ne 0 ]; then
    echo "Registry audit FAILED: see drift above. Update docs/meta/registry.yaml."
    exit 1
fi

echo "Registry audit OK."
