#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"

targets=(
  "$ROOT_DIR/target"
  "$ROOT_DIR/coverage"
  "$ROOT_DIR/mcs4-emu/target"
  "$ROOT_DIR/mcs4-emu/coverage"
)

for dir in "${targets[@]}"; do
  if [ -d "$dir" ]; then
    rm -rf "$dir"
  fi
done

find "$ROOT_DIR" \
  -name "*.profraw" -o \
  -name "*.profdata" -o \
  -name "lcov.info" \
  -delete
