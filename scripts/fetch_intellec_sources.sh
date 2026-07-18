#!/bin/sh
# Run the Intellec source registry fetcher with the repository Python runtime.
set -eu

REPO_ROOT=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
exec python3 "$REPO_ROOT/scripts/fetch_intellec_sources.py" "$@"
