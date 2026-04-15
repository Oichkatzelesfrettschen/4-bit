#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
TOOLCHAIN="$ROOT_DIR/rust-toolchain.toml"
CLAUDE="$ROOT_DIR/mcs4-emu/CLAUDE.md"
STATUS="$ROOT_DIR/mcs4-emu/STATUS.md"

required=(
    "$TOOLCHAIN"
    "$CLAUDE"
    "$STATUS"
    "$ROOT_DIR/docs/ROADMAP.md"
    "$ROOT_DIR/docs/TROUBLESHOOTING.md"
    "$ROOT_DIR/mcs4-emu/INSTALLATION.md"
    "$ROOT_DIR/.github/workflows/ci.yml"
    "$ROOT_DIR/.github/workflows/docs.yml"
)

for file in "${required[@]}"; do
    if [[ ! -f "$file" ]]; then
        echo "status_sync_check: missing required file: $file" >&2
        exit 1
    fi
done

nightly_pin="$(sed -nE 's/^channel = "(nightly-[0-9]{4}-[0-9]{2}-[0-9]{2})"$/\1/p' "$TOOLCHAIN" | head -n1)"
if [[ -z "$nightly_pin" ]]; then
    echo "status_sync_check: unable to parse nightly pin from rust-toolchain.toml" >&2
    exit 1
fi

errors=()

check_contains() {
    local file="$1"
    local needle="$2"
    local label="$3"

    if ! grep -Fq "$needle" "$file"; then
        errors+=("$label: expected '$needle' in ${file#$ROOT_DIR/}")
    fi
}

check_contains "$ROOT_DIR/docs/ROADMAP.md" "$nightly_pin" "nightly-pin drift"
check_contains "$ROOT_DIR/docs/TROUBLESHOOTING.md" "$nightly_pin" "nightly-pin drift"
check_contains "$ROOT_DIR/mcs4-emu/INSTALLATION.md" "$nightly_pin" "nightly-pin drift"
check_contains "$ROOT_DIR/.github/workflows/ci.yml" "$nightly_pin" "nightly-pin drift"
check_contains "$ROOT_DIR/.github/workflows/docs.yml" "$nightly_pin" "nightly-pin drift"

claude_date="$(sed -nE 's/^# MCS-4\/MCS-40 Emulator - Project Status \(([0-9]{4}-[0-9]{2}-[0-9]{2})\)$/\1/p' "$CLAUDE" | head -n1)"
status_date="$(sed -nE 's/^\*\*Last Updated:\*\* ([0-9]{4}-[0-9]{2}-[0-9]{2})$/\1/p' "$STATUS" | head -n1)"

if [[ -z "$claude_date" || -z "$status_date" ]]; then
    errors+=("status-date parse failed in CLAUDE.md or STATUS.md")
elif [[ "$claude_date" != "$status_date" ]]; then
    errors+=("status-date mismatch: CLAUDE.md=$claude_date STATUS.md=$status_date")
fi

claude_pct="$(sed -nE 's/^Summary: ([0-9]+)% overall completion$/\1/p' "$CLAUDE" | head -n1)"
status_pct="$(sed -nE 's/^## Phase Summary \(([0-9]+)% overall\)$/\1/p' "$STATUS" | head -n1)"

if [[ -z "$claude_pct" || -z "$status_pct" ]]; then
    errors+=("phase-summary parse failed in CLAUDE.md or STATUS.md")
elif [[ "$claude_pct" != "$status_pct" ]]; then
    errors+=("phase-summary mismatch: CLAUDE.md=${claude_pct}% STATUS.md=${status_pct}%")
fi

claude_tests="$(sed -nE 's/^([0-9,]+) tests passing, 0 failures:$/\1/p' "$CLAUDE" | head -n1)"
status_tests="$(sed -nE 's/^([0-9,]+) tests passing, 0 failures:$/\1/p' "$STATUS" | head -n1)"

if [[ -z "$claude_tests" || -z "$status_tests" ]]; then
    errors+=("test-count parse failed in CLAUDE.md or STATUS.md")
elif [[ "$claude_tests" != "$status_tests" ]]; then
    errors+=("test-count mismatch: CLAUDE.md=$claude_tests STATUS.md=$status_tests")
fi

if [[ ${#errors[@]} -gt 0 ]]; then
    echo "status_sync_check: found ${#errors[@]} issue(s):" >&2
    for error in "${errors[@]}"; do
        echo "  - $error" >&2
    done
    exit 1
fi

echo "Status sync check passed: nightly=$nightly_pin, updated=$status_date, completion=${status_pct}%, tests=$status_tests"
