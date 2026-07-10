#!/usr/bin/env python3
"""Quick test of OCR cache integration."""
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent / "scripts"))

import numpy as np

from ocr_cached_backend_v0 import resolve_cached_backend

# Create a simple test image (white background)
img = np.ones((100, 100), dtype=np.uint8) * 255

# Create backend with caching enabled
backend = resolve_cached_backend(backend="tesseract")

print("Testing OCR cache...")
print(f"Backend: {backend.name}")
print()

# First call (cache miss)
print("First call (cache miss expected):")
t0 = time.time()
r1 = backend.best_token(img, whitelist="ABC", psms=[8], oem=1, min_len=1, max_len=3)
t1 = time.time()
print(f"  Token: {r1.token!r}")
print(f"  Confidence: {r1.conf:.1f}")
print(f"  Time: {(t1 - t0) * 1000:.1f} ms")
print()

# Second call (cache hit)
print("Second call (cache hit expected):")
t0 = time.time()
r2 = backend.best_token(img, whitelist="ABC", psms=[8], oem=1, min_len=1, max_len=3)
t1 = time.time()
print(f"  Token: {r2.token!r}")
print(f"  Confidence: {r2.conf:.1f}")
print(f"  Time: {(t1 - t0) * 1000:.1f} ms")
print()

# Verify consistency
if r1.token == r2.token and abs(r1.conf - r2.conf) < 0.01:
    print("✓ Cache test PASSED: Results are consistent")
else:
    print("✗ Cache test FAILED: Results differ!")
    sys.exit(1)

# Show cache stats
# Deferred import keeps the timing measurements above import-cost free.
from ocr_cache_v0 import OcrCache  # noqa: E402

cache = OcrCache()
stats = cache.get_stats()
print()
print("Cache Statistics:")
print(f"  Total entries: {stats['total_entries']}")
print(f"  Database size: {stats['db_size_bytes'] / 1024:.1f} KB")
