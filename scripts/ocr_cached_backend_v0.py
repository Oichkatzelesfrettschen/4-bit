#!/usr/bin/env python3
"""
Cached OCR backend wrapper.

This module provides a caching layer over ocr_backend_v0 that:
1. Computes cache keys from image content + configuration
2. Checks persistent cache before invoking OCR
3. Stores results in cache for future re-use

The cache integration is designed to be:
- Transparent: existing code can use cached_backend just like resolve_backend
- Fast: cache lookups are O(1) SQLite queries
- Accurate: cache keys include all relevant OCR parameters

Usage:
    from ocr_cached_backend_v0 import resolve_cached_backend

    backend = resolve_cached_backend(backend="auto", prefer_cuda=True)
    result = backend.best_token(gray, whitelist="ABC", psms=[8], oem=1, min_len=1, max_len=4)
"""

from __future__ import annotations

import dataclasses
import hashlib
import os
from collections.abc import Iterable
from pathlib import Path

import numpy as np

from ocr_backend_v0 import Backend, OcrResult, resolve_backend
from ocr_cache_v0 import CacheKey, CacheValue, OcrCache


def _compute_config_hash(
    *,
    whitelist: str,
    psms: Iterable[int],
    oem: int,
    min_len: int,
    max_len: int,
) -> str:
    """
    Compute stable hash of OCR configuration.

    This captures all parameters that affect OCR output.
    """
    # Sort PSMs for deterministic hashing
    psm_tuple = tuple(sorted(set(psms)))

    cfg_str = f"{whitelist}:{psm_tuple}:{oem}:{min_len}:{max_len}"
    return hashlib.sha256(cfg_str.encode("utf-8")).hexdigest()[:16]


@dataclasses.dataclass(frozen=True)
class CachedBackend:
    """
    Backend wrapper that adds persistent caching.

    This wraps any Backend implementation and caches its results.
    """

    backend: Backend
    cache: OcrCache
    name: str = dataclasses.field(init=False)

    def __post_init__(self) -> None:
        # Set name based on wrapped backend
        object.__setattr__(self, "name", f"cached-{self.backend.name}")

    def best_token(
        self,
        gray: np.ndarray,
        *,
        whitelist: str,
        psms: Iterable[int],
        oem: int,
        min_len: int,
        max_len: int,
    ) -> OcrResult:
        """
        Get best token with caching.

        Cache key includes:
        - Image content (SHA256 hash)
        - Whitelist characters
        - PSM modes
        - OEM mode
        - Min/max length constraints

        Note: We don't cache preprocessing parameters (preset, scale, invert)
        because those are internal to the backend implementation. The cache
        key is based on the *input* image and *requested* parameters only.
        """
        # Build cache key
        img_hash = hashlib.sha256(gray.tobytes()).hexdigest()
        cfg_hash = _compute_config_hash(
            whitelist=whitelist,
            psms=psms,
            oem=oem,
            min_len=min_len,
            max_len=max_len,
        )

        # For cache key, we use a simplified representation:
        # - image_hash: SHA256 of input image
        # - preset: config hash (captures whitelist + PSM + OEM + lengths)
        # - psm: first PSM (representative)
        # - threshold_mode: "N/A" (internal to backend)
        # - scale: 1 (internal to backend)
        # - invert: False (internal to backend)
        #
        # This ensures cache hits for identical input+config while avoiding
        # over-specification of internal preprocessing details.
        cache_key = CacheKey(
            image_hash=img_hash,
            preset=cfg_hash,
            psm=list(psms)[0] if psms else 8,
            threshold_mode="N/A",
            scale=1,
            invert=False,
        )

        # Check cache
        cached = self.cache.get(cache_key)
        if cached is not None:
            # Reconstruct OcrResult from cached data
            # Note: We lose psm/invert/scale info from cache, but token+conf are preserved
            return OcrResult(
                token=cached.token,
                conf=cached.confidence,
                psm=cache_key.psm,
                invert=False,
                scale=1,
            )

        # Cache miss: invoke backend
        result = self.backend.best_token(
            gray,
            whitelist=whitelist,
            psms=psms,
            oem=oem,
            min_len=min_len,
            max_len=max_len,
        )

        # Store in cache
        cache_value = CacheValue(
            token=result.token,
            confidence=result.conf,
            backend_used=self.backend.name,
            timestamp=0.0,  # Will be set by cache.put()
        )
        self.cache.put(cache_key, cache_value)

        return result


def resolve_cached_backend(
    *,
    backend: str = "auto",
    onnx_model: Path | None = None,
    prefer_cuda: bool = False,
    cache_path: Path | None = None,
    enable_cache: bool = True,
) -> Backend:
    """
    Resolve OCR backend with optional caching.

    Args:
        backend: Backend type ("auto", "tesseract", "tesseract_cli", "onnx", etc.)
        onnx_model: Optional path to ONNX model
        prefer_cuda: Whether to prefer CUDA execution providers
        cache_path: Custom cache database path (uses default if None)
        enable_cache: Whether to enable caching (default: True, can disable with OCR_CACHE=0)

    Returns:
        Backend instance (cached or raw depending on enable_cache)

    Environment variables:
        OCR_CACHE=0: Disable caching (for debugging/benchmarking)
        OCR_CACHE_PATH=<path>: Custom cache database location
    """
    # Check environment for cache control
    if not enable_cache or os.environ.get("OCR_CACHE", "1").strip() in ("0", "false", "no", "off"):
        # Caching disabled: return raw backend
        return resolve_backend(backend=backend, onnx_model=onnx_model, prefer_cuda=prefer_cuda)

    # Resolve cache path
    if cache_path is None:
        env_path = os.environ.get("OCR_CACHE_PATH", "").strip()
        if env_path:
            cache_path = Path(env_path)

    # Create cache
    cache = OcrCache(db_path=cache_path)

    # Resolve base backend
    base_backend = resolve_backend(backend=backend, onnx_model=onnx_model, prefer_cuda=prefer_cuda)

    # Wrap with caching
    return CachedBackend(backend=base_backend, cache=cache)


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description="Test cached OCR backend")
    parser.add_argument("image", type=Path, help="Input image")
    parser.add_argument("--backend", default="auto", help="Backend type")
    parser.add_argument("--whitelist", default="ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789", help="Character whitelist")
    parser.add_argument("--psm", type=int, default=8, help="Tesseract PSM mode")
    parser.add_argument("--no-cache", action="store_true", help="Disable caching")

    args = parser.parse_args()

    import cv2

    # Load image
    img = cv2.imread(str(args.image), cv2.IMREAD_GRAYSCALE)
    if img is None:
        raise SystemExit(f"Failed to load image: {args.image}")

    # Resolve backend
    backend = resolve_cached_backend(
        backend=args.backend,
        enable_cache=not args.no_cache,
    )

    print(f"Backend: {backend.name}")

    # Run OCR twice to test caching
    import time

    print("\nFirst run (cache miss expected):")
    t0 = time.time()
    result1 = backend.best_token(
        img,
        whitelist=args.whitelist,
        psms=[args.psm],
        oem=1,
        min_len=1,
        max_len=10,
    )
    t1 = time.time()
    print(f"  Token: {result1.token!r}")
    print(f"  Confidence: {result1.conf:.1f}")
    print(f"  Time: {(t1 - t0) * 1000:.1f} ms")

    print("\nSecond run (cache hit expected if caching enabled):")
    t0 = time.time()
    result2 = backend.best_token(
        img,
        whitelist=args.whitelist,
        psms=[args.psm],
        oem=1,
        min_len=1,
        max_len=10,
    )
    t1 = time.time()
    print(f"  Token: {result2.token!r}")
    print(f"  Confidence: {result2.conf:.1f}")
    print(f"  Time: {(t1 - t0) * 1000:.1f} ms")

    # Verify results are identical
    if result1.token == result2.token and abs(result1.conf - result2.conf) < 0.01:
        print("\n✓ Results are consistent")
    else:
        print("\n✗ Results differ!")
