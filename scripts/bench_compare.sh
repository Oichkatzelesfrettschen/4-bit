#!/bin/sh
# bench_compare.sh -- Run Criterion benchmarks across multiple target-cpu values
# and save results for comparison.
#
# Usage:  ./scripts/bench_compare.sh [output_dir]
#
# Outputs JSON benchmark results to $output_dir/{target}.json
# Requires: cargo, jq (optional, for summary)

set -eu

OUTPUT_DIR="${1:-target/bench-compare}"
mkdir -p "$OUTPUT_DIR"

TARGETS="x86-64-v3 znver3 native"
TOOLCHAIN="$(rustup show active-toolchain | cut -d' ' -f1)"

echo "Toolchain: $TOOLCHAIN"
echo "Output:    $OUTPUT_DIR"
echo ""

for target in $TARGETS; do
    echo "=== Benchmarking target-cpu=$target ==="
    RUSTFLAGS="-C target-cpu=$target" \
        cargo bench --workspace --locked \
        -- --output-format=bencher 2>/dev/null \
        | tee "$OUTPUT_DIR/$target.txt" \
        || echo "  (bench run returned non-zero, partial results saved)"
    echo ""
done

echo "=== Results ==="
for target in $TARGETS; do
    result="$OUTPUT_DIR/$target.txt"
    if [ -f "$result" ]; then
        lines=$(wc -l < "$result")
        echo "  $target: $lines lines -> $result"
    else
        echo "  $target: no results"
    fi
done

echo ""
echo "Compare with: diff $OUTPUT_DIR/x86-64-v3.txt $OUTPUT_DIR/znver3.txt"
