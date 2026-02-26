#!/bin/sh
# fetch_sources_test.sh -- Test URL reachability for all project source URLs.
#
# WHY:  Dead links degrade reproducibility. Periodic testing catches rot early.
# WHAT: Tests every URL in ocr_manifest.yaml, bibliography.bib, and
#       photomicrograph_permissions.md via HTTP HEAD requests.
# HOW:  curl -sI with 15-second timeout. Reports status codes.
#
# Usage:
#   ./scripts/fetch_sources_test.sh           # test all URLs
#   ./scripts/fetch_sources_test.sh --brief   # one line per URL, no details
#
# Exit code: 0 if all reachable, 1 if any unreachable.

set -eu

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

BRIEF=0
while [ $# -gt 0 ]; do
    case "$1" in
        --brief) BRIEF=1 ;;
        --help|-h)
            sed -n '2,13s/^# //p' "$0"
            exit 0
            ;;
        *) printf "Unknown option: %s\n" "$1" >&2; exit 1 ;;
    esac
    shift
done

# Color helpers
if [ -t 1 ]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[0;33m'
    RESET='\033[0m'
else
    RED='' GREEN='' YELLOW='' RESET=''
fi

TOTAL=0
REACHABLE=0
FAILED=0
DEGRADED=0

test_url() {
    _url="$1"
    _label="$2"
    TOTAL=$((TOTAL + 1))

    _code=$(curl -sI -o /dev/null -w '%{http_code}' --max-time 15 -L "$_url" 2>/dev/null || echo "000")

    case "$_code" in
        200|301|302)
            REACHABLE=$((REACHABLE + 1))
            if [ "$BRIEF" -eq 1 ]; then
                printf "${GREEN}%3s${RESET} %s\n" "$_code" "$_url"
            else
                printf "${GREEN}%3s${RESET} %-50s %s\n" "$_code" "$_label" "$_url"
            fi
            ;;
        403|520|521|522|523|524|525)
            DEGRADED=$((DEGRADED + 1))
            if [ "$BRIEF" -eq 1 ]; then
                printf "${YELLOW}%3s${RESET} %s\n" "$_code" "$_url"
            else
                printf "${YELLOW}%3s${RESET} %-50s %s\n" "$_code" "$_label" "$_url"
            fi
            ;;
        *)
            FAILED=$((FAILED + 1))
            if [ "$BRIEF" -eq 1 ]; then
                printf "${RED}%3s${RESET} %s\n" "$_code" "$_url"
            else
                printf "${RED}%3s${RESET} %-50s %s\n" "$_code" "$_label" "$_url"
            fi
            ;;
    esac
}

printf "=== URL Reachability Test ===\n"
printf "Date: %s\n\n" "$(date -u +%Y-%m-%dT%H:%M:%SZ)"

# Extract URLs from ocr_manifest.yaml
printf "--- ocr_manifest.yaml ---\n"
python3 -c "
import yaml, sys
with open(sys.argv[1]) as f:
    data = yaml.safe_load(f)
for src in data.get('sources', []):
    url = src.get('url', 'unknown')
    sid = src.get('id', '')
    if url and url != 'unknown':
        print(f'{sid}\t{url}')
    # Check sidecars with alt_url
    for sc in src.get('sidecars', []):
        alt = sc.get('alt_url', '')
        scid = sc.get('id', sid)
        if alt:
            print(f'{scid}\t{alt}')
" "$REPO_ROOT/docs/evidence/ocr_manifest.yaml" | while IFS='	' read -r label url; do
    test_url "$url" "$label"
done

# Extract URLs from bibliography.bib
printf "\n--- bibliography.bib ---\n"
grep -oP '(?<=url\s{0,10}=\s{0,10}\{)https?://[^}]+' \
    "$REPO_ROOT/docs/evidence/bibliography.bib" | sort -u | while read -r url; do
    # Derive a short label from URL
    label=$(echo "$url" | sed 's|https\?://||; s|/.*||')
    test_url "$url" "$label"
done

# Extract URLs from photomicrograph_permissions.md
printf "\n--- photomicrograph_permissions.md ---\n"
grep -oP 'https?://[^\s)>]+' \
    "$REPO_ROOT/docs/evidence/photomicrograph_permissions.md" | sort -u | while read -r url; do
    label=$(echo "$url" | sed 's|https\?://||; s|/.*||')
    test_url "$url" "$label"
done

printf "\n=== Summary ===\n"
printf "Total:     %d\n" "$TOTAL"
printf "Reachable: %d\n" "$REACHABLE"
printf "Degraded:  %d\n" "$DEGRADED"
printf "Failed:    %d\n" "$FAILED"

if [ "$FAILED" -gt 0 ]; then
    exit 1
fi
exit 0
