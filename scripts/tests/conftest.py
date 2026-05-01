"""Pytest configuration for scripts/tests.

Exposes the parent `scripts/` directory on `sys.path` so individual extraction
scripts (e.g. `extract_gates_v0`, `gate_to_verilog_v0`) can be imported
directly by their module name without a packaging layer.
"""

from __future__ import annotations

import sys
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parent.parent
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

# Repo root for tests that need to read fixture data under docs/evidence/.
REPO_ROOT = SCRIPTS_DIR.parent
