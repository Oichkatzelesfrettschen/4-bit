#!/bin/sh
# fetch_sources.sh -- Download and verify primary source PDFs.
#
# WHY:  Reproducible acquisition of MCS-4/MCS-40 primary source documents.
# WHAT: Parses docs/evidence/ocr_manifest.yaml, downloads each PDF to its
#       local_path, and verifies the SHA-256 checksum.
# HOW:  Uses aria2c (preferred), wget, or curl as download backend.
#       Idempotent: skips files that already match their expected checksum.
#
# Usage:
#   ./scripts/fetch_sources.sh              # download missing + verify all
#   ./scripts/fetch_sources.sh --verify     # checksum-only (no downloads)
#   ./scripts/fetch_sources.sh --dry-run    # test URL reachability only
#   ./scripts/fetch_sources.sh --help       # show this help
#
# Requires: sha256sum, one of {aria2c, wget, curl}, python3 (for YAML parsing)

set -eu

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MANIFEST="$REPO_ROOT/docs/evidence/ocr_manifest.yaml"

# Defaults
MODE="download"   # download | verify | dry-run
VERBOSE=0

usage() {
    sed -n '2,15s/^# //p' "$0"
    exit 0
}

# Parse arguments
while [ $# -gt 0 ]; do
    case "$1" in
        --verify)   MODE="verify"  ;;
        --dry-run)  MODE="dry-run" ;;
        --verbose)  VERBOSE=1      ;;
        --help|-h)  usage          ;;
        *)          printf "Unknown option: %s\n" "$1" >&2; exit 1 ;;
    esac
    shift
done

# Color helpers (disabled if not a terminal)
if [ -t 1 ]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[0;33m'
    RESET='\033[0m'
else
    RED='' GREEN='' YELLOW='' RESET=''
fi

pass() { printf "${GREEN}PASS${RESET} %s\n" "$1"; }
fail() { printf "${RED}FAIL${RESET} %s\n" "$1"; }
warn() { printf "${YELLOW}WARN${RESET} %s\n" "$1"; }
info() { printf "     %s\n" "$1"; }

# Per-entry detail lines, enabled by --verbose.
verbose() {
    if [ "$VERBOSE" -eq 1 ]; then
        printf "     %s\n" "$1"
    fi
}

# Detect download tool
detect_downloader() {
    if command -v aria2c >/dev/null 2>&1; then
        echo "aria2c"
    elif command -v wget >/dev/null 2>&1; then
        echo "wget"
    elif command -v curl >/dev/null 2>&1; then
        echo "curl"
    else
        printf "Error: no download tool found (need aria2c, wget, or curl)\n" >&2
        exit 1
    fi
}

# Download a single file
# $1 = URL, $2 = output path
download_file() {
    _url="$1"
    _out="$2"

    # Ensure parent directory exists
    mkdir -p "$(dirname "$_out")"

    case "$DOWNLOADER" in
        aria2c)
            aria2c --quiet --dir="$(dirname "$_out")" \
                   --out="$(basename "$_out")" \
                   --allow-overwrite=true "$_url"
            ;;
        wget)
            wget -q -O "$_out" "$_url"
            ;;
        curl)
            curl -fsSL -o "$_out" "$_url"
            ;;
    esac
}

# Test URL reachability (HEAD request)
# $1 = URL
# Returns 0 if reachable, 1 otherwise. Prints status code.
test_url() {
    _url="$1"
    _code=$(curl -sI -o /dev/null -w '%{http_code}' --max-time 15 -L "$_url" 2>/dev/null || echo "000")
    printf "%s" "$_code"
}

# Extract entries from YAML manifest using Python
# Outputs tab-separated: id\tlocal_path\turl\tsha256
parse_manifest() {
    python3 -c "
import yaml, sys

with open(sys.argv[1]) as f:
    data = yaml.safe_load(f)

for src in data.get('sources', []):
    sid = src.get('id', '')
    lp = src.get('local_path', '')
    url = src.get('url', 'unknown')
    sha = src.get('sha256', '')
    # Skip entries with no local_path or tmp-only paths
    if not lp or lp.startswith('/tmp'):
        continue
    print(f'{sid}\t{lp}\t{url}\t{sha}')
" "$MANIFEST"
}

# Counters
TOTAL=0
PASSED=0
FAILED=0
SKIPPED=0
DOWNLOADED=0

DOWNLOADER="$(detect_downloader)"

printf "=== fetch_sources.sh (mode: %s, downloader: %s) ===\n\n" "$MODE" "$DOWNLOADER"

parse_manifest | while IFS='	' read -r id local_path url sha256; do
    TOTAL=$((TOTAL + 1))
    abs_path="$REPO_ROOT/$local_path"
    verbose "$id: url=$url"
    verbose "$id: local_path=$local_path expected_sha256=$sha256"

    # Handle unknown URLs
    if [ "$url" = "unknown" ]; then
        warn "$id: URL unknown, cannot download or test"
        SKIPPED=$((SKIPPED + 1))
        continue
    fi

    case "$MODE" in
        dry-run)
            code=$(test_url "$url")
            if [ "$code" = "200" ]; then
                pass "$id: $url -> HTTP $code"
            elif [ "$code" = "000" ]; then
                fail "$id: $url -> connection failed"
                FAILED=$((FAILED + 1))
            else
                fail "$id: $url -> HTTP $code"
                FAILED=$((FAILED + 1))
            fi
            ;;

        verify)
            if [ ! -f "$abs_path" ]; then
                fail "$id: file not found at $local_path"
                FAILED=$((FAILED + 1))
                continue
            fi
            actual=$(sha256sum "$abs_path" | awk '{print $1}')
            if [ "$actual" = "$sha256" ]; then
                pass "$id: checksum matches"
            else
                fail "$id: checksum mismatch"
                info "  expected: $sha256"
                info "  actual:   $actual"
                FAILED=$((FAILED + 1))
            fi
            ;;

        download)
            # If file exists and checksum matches, skip download
            if [ -f "$abs_path" ]; then
                actual=$(sha256sum "$abs_path" | awk '{print $1}')
                if [ "$actual" = "$sha256" ]; then
                    pass "$id: already present, checksum OK"
                    PASSED=$((PASSED + 1))
                    continue
                fi
                warn "$id: file exists but checksum differs, re-downloading"
            fi

            info "$id: downloading from $url"
            if download_file "$url" "$abs_path"; then
                actual=$(sha256sum "$abs_path" | awk '{print $1}')
                if [ "$actual" = "$sha256" ]; then
                    pass "$id: downloaded and verified"
                    DOWNLOADED=$((DOWNLOADED + 1))
                else
                    fail "$id: downloaded but checksum mismatch"
                    info "  expected: $sha256"
                    info "  actual:   $actual"
                    FAILED=$((FAILED + 1))
                fi
            else
                fail "$id: download failed"
                FAILED=$((FAILED + 1))
            fi
            ;;
    esac
done

printf "\n=== Summary ===\n"
printf "Mode: %s\n" "$MODE"
if [ "$MODE" = "download" ]; then
    printf "Downloaded: %d\n" "$DOWNLOADED"
fi
printf "Skipped (unknown URL): %d\n" "$SKIPPED"
printf "Failed: %d\n" "$FAILED"

if [ "$FAILED" -gt 0 ]; then
    exit 1
fi
exit 0
