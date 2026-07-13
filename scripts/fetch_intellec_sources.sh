#!/bin/sh
# Download and verify local-only Intellec source scans with a fixed Mozilla UA.
set -eu

REPO_ROOT=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
MANIFEST="$REPO_ROOT/docs/evidence/intellec_sources.yaml"
MODE=download

usage() {
    printf '%s\n' \
        'Usage: scripts/fetch_intellec_sources.sh [--verify|--dry-run|--help]' \
        'Downloads only verified local-only Intellec source scans with wget.'
}

case "${1-}" in
    '') ;;
    --verify) MODE=verify ;;
    --dry-run) MODE=dry-run ;;
    --help|-h) usage; exit 0 ;;
    *) usage >&2; exit 2 ;;
esac

command -v wget >/dev/null 2>&1 || {
    printf '%s\n' 'wget is required for Intellec source acquisition' >&2
    exit 127
}
command -v python3 >/dev/null 2>&1 || {
    printf '%s\n' 'python3 with PyYAML is required for Intellec source acquisition' >&2
    exit 127
}

USER_AGENT='Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0'

parse_sources() {
    python3 - "$MANIFEST" <<'PY'
import sys
import yaml

with open(sys.argv[1], encoding="utf-8") as handle:
    ledger = yaml.safe_load(handle)

for source in ledger.get("sources", []):
    if source.get("retention") != "local-only":
        continue
    fields = ("id", "local_path", "url", "sha256")
    values = [source.get(field, "") for field in fields]
    if not all(values):
        raise SystemExit(f"local-only source has incomplete acquisition data: {source.get('id', '<unknown>')}")
    print("\t".join(values))
PY
}

failed=0
while IFS='	' read -r source_id local_path source_url expected_sha256; do
    target="$REPO_ROOT/$local_path"
    case "$MODE" in
        dry-run)
            if wget --spider --https-only --no-verbose --timeout=30 --tries=3 --user-agent="$USER_AGENT" "$source_url"; then
                printf 'PASS %s reachable\n' "$source_id"
            else
                printf 'FAIL %s unreachable\n' "$source_id" >&2
                failed=1
            fi
            ;;
        verify)
            if [ ! -f "$target" ]; then
                printf 'FAIL %s missing: %s\n' "$source_id" "$local_path" >&2
                failed=1
            elif actual_sha256=$(sha256sum "$target" | awk '{print $1}') && [ "$actual_sha256" = "$expected_sha256" ]; then
                printf 'PASS %s checksum\n' "$source_id"
            else
                printf 'FAIL %s checksum\n' "$source_id" >&2
                failed=1
            fi
            ;;
        download)
            mkdir -p "$(dirname "$target")"
            if [ -f "$target" ] && actual_sha256=$(sha256sum "$target" | awk '{print $1}') && [ "$actual_sha256" = "$expected_sha256" ]; then
                printf 'PASS %s already verified\n' "$source_id"
                continue
            fi
            temporary_path="$target.part"
            rm -f "$temporary_path"
            if wget --https-only --no-verbose --timeout=30 --tries=3 --user-agent="$USER_AGENT" --output-document="$temporary_path" "$source_url" \
                && actual_sha256=$(sha256sum "$temporary_path" | awk '{print $1}') \
                && [ "$actual_sha256" = "$expected_sha256" ]; then
                mv "$temporary_path" "$target"
                printf 'PASS %s downloaded and verified\n' "$source_id"
            else
                rm -f "$temporary_path"
                printf 'FAIL %s download or checksum\n' "$source_id" >&2
                failed=1
            fi
            ;;
    esac
done <<EOF
$(parse_sources)
EOF

exit "$failed"
