#!/usr/bin/env bash
set -euo pipefail

# Keep warnings fatal in our own code paths.
PY="python -W error"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "[ci] anchor audit"
$PY scripts/audit_schematic_layout_anchors_v1.py --all --anchors docs/evidence/schematic_layout_anchors_v1.json

echo "[ci] pad-anchor consistency"
$PY scripts/report_pad_anchor_consistency_v0.py \
  --all \
  --anchors docs/evidence/schematic_layout_anchors_v1.json \
  --netlists-v0 docs/evidence/netlists_v0 \
  --pads-v0 docs/evidence/layout_pads_v0 \
  --out-dir docs/evidence/pad_anchor_consistency_v0

echo "[ci] anchor incidence (netlist_v1)"
for chip in 4001 4002 4003 4004; do
  $PY scripts/check_anchor_incidence_v0.py --netlist-v1 "docs/evidence/netlists_v1/${chip}_netlist_v1.json"
done

echo "[ci] anchor uniqueness (required signals)"
$PY scripts/check_anchor_uniqueness_v0.py --chip 4001 --chip 4002 --chip 4003 --anchors docs/evidence/schematic_layout_anchors_v1.json

echo "[ci] ok"
