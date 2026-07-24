#!/usr/bin/env bash
# Assert that mcs4-runtime stays frontend-neutral: the runtime owns the machine,
# commands, events, and snapshots that every frontend consumes, so it must never
# pull in a presentation framework. Compilation already omits these deps; this
# gate fails fast if a future edit reintroduces one, protecting the layer
# boundary that lets egui and a 3D world share one runtime.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

forbidden='egui|eframe|bevy|wgpu|winit'

tree="$(cargo tree --quiet --package mcs4-runtime --edges normal --prefix none)"

if printf '%s\n' "$tree" | grep -Eiq "^(${forbidden})[[:space:]]"; then
    echo "runtime_frontend_neutral_check: mcs4-runtime must not depend on a presentation framework" >&2
    printf '%s\n' "$tree" | grep -Ei "^(${forbidden})[[:space:]]" >&2
    exit 1
fi

echo "runtime frontend-neutral check passed: mcs4-runtime has no presentation dependency"
