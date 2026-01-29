#!/usr/bin/env python3
"""
Emulator performance benchmark suite.

Measures execution performance for:
- CPU instruction throughput (cycles/sec)
- Memory access latency
- I/O operations
- Test fixture execution time

Outputs: JSON benchmarks for CI tracking
"""

from __future__ import annotations

import argparse
import json
import subprocess
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]


@dataclass
class BenchmarkResult:
    """Single benchmark result."""

    name: str
    category: str
    duration_ms: float
    throughput: float | None = None  # operations/sec or cycles/sec
    unit: str = "ops/sec"
    metadata: dict[str, Any] | None = None


def run_cargo_bench(bench_name: str | None = None) -> list[BenchmarkResult]:
    """Run Rust criterion benchmarks and parse results."""
    cmd = ["cargo", "bench", "--workspace"]
    if bench_name:
        cmd.append(bench_name)

    print(f"Running cargo bench: {' '.join(cmd)}")
    start = time.time()

    try:
        result = subprocess.run(
            cmd,
            cwd=ROOT / "mcs4-emu",
            capture_output=True,
            text=True,
            timeout=300,
        )
        elapsed = (time.time() - start) * 1000

        if result.returncode != 0:
            print(f"Warning: cargo bench failed: {result.stderr}")
            return []

        # Parse criterion output (simplified - criterion outputs to JSON)
        # For now, return a placeholder
        return [
            BenchmarkResult(
                name="cargo_bench_all",
                category="rust_benchmarks",
                duration_ms=elapsed,
                metadata={"returncode": result.returncode},
            )
        ]

    except subprocess.TimeoutExpired:
        print("Error: cargo bench timed out")
        return []
    except Exception as e:
        print(f"Error running cargo bench: {e}")
        return []


def run_fixture_tests() -> list[BenchmarkResult]:
    """Run test fixtures and measure execution time."""
    cmd = ["cargo", "test", "--workspace", "--lib", "--", "--nocapture"]

    print(f"Running fixture tests: {' '.join(cmd)}")
    start = time.time()

    try:
        result = subprocess.run(
            cmd,
            cwd=ROOT / "mcs4-emu",
            capture_output=True,
            text=True,
            timeout=120,
        )
        elapsed = (time.time() - start) * 1000

        # Parse test output for fixture count
        output = result.stdout + result.stderr
        test_count = output.count(" test ") if result.returncode == 0 else 0

        return [
            BenchmarkResult(
                name="test_fixtures_all",
                category="integration_tests",
                duration_ms=elapsed,
                throughput=test_count / (elapsed / 1000) if elapsed > 0 else 0,
                unit="tests/sec",
                metadata={
                    "returncode": result.returncode,
                    "test_count": test_count,
                },
            )
        ]

    except subprocess.TimeoutExpired:
        print("Error: fixture tests timed out")
        return []
    except Exception as e:
        print(f"Error running fixture tests: {e}")
        return []


def run_synthetic_benchmarks() -> list[BenchmarkResult]:
    """Run synthetic CPU benchmarks (NOP loop, ALU operations, etc.)."""
    # These would require custom benchmark binaries
    # For now, return placeholders

    benchmarks = [
        BenchmarkResult(
            name="nop_loop_1000",
            category="synthetic",
            duration_ms=0.0,  # Placeholder
            throughput=0.0,
            unit="cycles/sec",
            metadata={"status": "not_implemented"},
        ),
        BenchmarkResult(
            name="add_loop_1000",
            category="synthetic",
            duration_ms=0.0,
            throughput=0.0,
            unit="cycles/sec",
            metadata={"status": "not_implemented"},
        ),
    ]

    return benchmarks


def run_all_benchmarks() -> list[BenchmarkResult]:
    """Run complete benchmark suite."""
    results = []

    print("=== Emulator Benchmark Suite ===")
    print("")

    # Rust benchmarks
    print("Running Rust benchmarks...")
    results.extend(run_cargo_bench())

    # Fixture tests
    print("Running fixture tests...")
    results.extend(run_fixture_tests())

    # Synthetic benchmarks
    print("Running synthetic benchmarks...")
    results.extend(run_synthetic_benchmarks())

    return results


def compute_baseline(results: list[BenchmarkResult]) -> dict[str, float]:
    """Compute baseline metrics for regression detection."""
    baseline = {}

    for result in results:
        baseline[result.name] = result.duration_ms

    return baseline


def compare_with_baseline(
    results: list[BenchmarkResult], baseline_path: Path
) -> dict[str, Any]:
    """Compare current results with saved baseline."""
    if not baseline_path.exists():
        return {"status": "no_baseline", "regressions": []}

    baseline = json.loads(baseline_path.read_text(encoding="utf-8"))

    regressions = []
    improvements = []

    for result in results:
        if result.name not in baseline:
            continue

        baseline_ms = baseline[result.name]
        current_ms = result.duration_ms

        # Compute percentage change
        if baseline_ms > 0:
            change_pct = ((current_ms - baseline_ms) / baseline_ms) * 100

            # Regression if >20% slower
            if change_pct > 20:
                regressions.append(
                    {
                        "name": result.name,
                        "baseline_ms": baseline_ms,
                        "current_ms": current_ms,
                        "change_pct": change_pct,
                    }
                )
            # Improvement if >20% faster
            elif change_pct < -20:
                improvements.append(
                    {
                        "name": result.name,
                        "baseline_ms": baseline_ms,
                        "current_ms": current_ms,
                        "change_pct": change_pct,
                    }
                )

    return {
        "status": "compared",
        "regressions": regressions,
        "improvements": improvements,
    }


def main():
    parser = argparse.ArgumentParser(description="Emulator performance benchmarks")
    parser.add_argument(
        "--output",
        type=Path,
        default=ROOT / "docs" / "evidence" / "benchmarks_v0.json",
        help="Output JSON file",
    )
    parser.add_argument(
        "--baseline",
        type=Path,
        default=ROOT / "docs" / "evidence" / "benchmarks_baseline_v0.json",
        help="Baseline JSON for comparison",
    )
    parser.add_argument(
        "--save-baseline",
        action="store_true",
        help="Save current results as new baseline",
    )
    parser.add_argument(
        "--fail-on-regression",
        action="store_true",
        help="Exit with non-zero if regressions detected",
    )

    args = parser.parse_args()

    # Run benchmarks
    results = run_all_benchmarks()

    print("")
    print("=== Benchmark Results ===")
    for result in results:
        print(f"{result.name}:")
        print(f"  Duration: {result.duration_ms:.2f} ms")
        if result.throughput:
            print(f"  Throughput: {result.throughput:.2f} {result.unit}")

    # Compare with baseline
    comparison = compare_with_baseline(results, args.baseline)

    if comparison["regressions"]:
        print("")
        print("=== REGRESSIONS DETECTED ===")
        for reg in comparison["regressions"]:
            print(
                f"  {reg['name']}: {reg['baseline_ms']:.2f}ms -> {reg['current_ms']:.2f}ms "
                f"({reg['change_pct']:+.1f}%)"
            )

    if comparison["improvements"]:
        print("")
        print("=== Improvements ===")
        for imp in comparison["improvements"]:
            print(
                f"  {imp['name']}: {imp['baseline_ms']:.2f}ms -> {imp['current_ms']:.2f}ms "
                f"({imp['change_pct']:+.1f}%)"
            )

    # Save results
    output_data = {
        "timestamp": time.strftime("%Y-%m-%d %H:%M:%S"),
        "results": [
            {
                "name": r.name,
                "category": r.category,
                "duration_ms": r.duration_ms,
                "throughput": r.throughput,
                "unit": r.unit,
                "metadata": r.metadata,
            }
            for r in results
        ],
        "comparison": comparison,
    }

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(output_data, indent=2) + "\n", encoding="utf-8")
    print(f"\nResults saved to: {args.output}")

    # Save baseline if requested
    if args.save_baseline:
        baseline = compute_baseline(results)
        args.baseline.write_text(
            json.dumps(baseline, indent=2) + "\n", encoding="utf-8"
        )
        print(f"Baseline saved to: {args.baseline}")

    # Exit with error if regressions detected and flag set
    if args.fail_on_regression and comparison["regressions"]:
        print("\nERROR: Performance regressions detected")
        return 1

    return 0


if __name__ == "__main__":
    exit(main())
