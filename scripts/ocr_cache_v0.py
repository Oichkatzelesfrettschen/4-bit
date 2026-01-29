#!/usr/bin/env python3
"""
OCR persistent cache implementation with SQLite backend.

This module provides a disk-backed cache for OCR results to eliminate redundant
Tesseract invocations and significantly improve extraction pipeline performance.

Cache keys are computed from:
- Image content hash (SHA256)
- OCR configuration: preset, PSM, threshold mode, scale, invert

Cache values store:
- Recognized token
- Confidence score
- Backend used (tesseract, onnx, template, etc.)
- Timestamp

Expected speedup: 3-10× on re-runs with warm cache.
"""

from __future__ import annotations

import dataclasses
import hashlib
import json
import sqlite3
import time
from pathlib import Path
from typing import Any

import numpy as np

# Default cache location: project root / .cache / ocr_cache_v0.db
DEFAULT_CACHE_DIR = Path(__file__).resolve().parents[1] / ".cache"
DEFAULT_CACHE_PATH = DEFAULT_CACHE_DIR / "ocr_cache_v0.db"


@dataclasses.dataclass(frozen=True)
class CacheKey:
    """
    Unique identifier for an OCR request.

    Combines image content (via hash) with configuration parameters.
    """
    image_hash: str  # SHA256 hex digest
    preset: str
    psm: int
    threshold_mode: str
    scale: int
    invert: bool

    def to_json(self) -> str:
        """Serialize to JSON for storage."""
        return json.dumps(dataclasses.asdict(self), sort_keys=True)

    @classmethod
    def from_json(cls, data: str) -> CacheKey:
        """Deserialize from JSON."""
        d = json.loads(data)
        return cls(**d)

    @classmethod
    def from_image_and_config(
        cls,
        image: np.ndarray,
        preset: str,
        psm: int,
        threshold_mode: str,
        scale: int,
        invert: bool,
    ) -> CacheKey:
        """Construct cache key from image and OCR configuration."""
        # Compute SHA256 hash of image bytes
        img_bytes = image.tobytes()
        img_hash = hashlib.sha256(img_bytes).hexdigest()

        return cls(
            image_hash=img_hash,
            preset=preset,
            psm=psm,
            threshold_mode=threshold_mode,
            scale=scale,
            invert=invert,
        )


@dataclasses.dataclass(frozen=True)
class CacheValue:
    """OCR result stored in cache."""
    token: str
    confidence: float
    backend_used: str
    timestamp: float

    def to_json(self) -> str:
        """Serialize to JSON for storage."""
        return json.dumps(dataclasses.asdict(self), sort_keys=True)

    @classmethod
    def from_json(cls, data: str) -> CacheValue:
        """Deserialize from JSON."""
        d = json.loads(data)
        return cls(**d)


class OcrCache:
    """
    Persistent OCR result cache with SQLite backend.

    Thread-safe for reads; writes should be serialized externally if needed.
    """

    def __init__(self, db_path: Path | None = None):
        """
        Initialize cache with optional custom database path.

        Args:
            db_path: Path to SQLite database file. If None, uses default location.
        """
        self.db_path = db_path or DEFAULT_CACHE_PATH
        self._ensure_db_exists()

    def _ensure_db_exists(self) -> None:
        """Create database and schema if it doesn't exist."""
        self.db_path.parent.mkdir(parents=True, exist_ok=True)

        with sqlite3.connect(self.db_path) as conn:
            conn.execute("""
                CREATE TABLE IF NOT EXISTS ocr_cache (
                    key TEXT PRIMARY KEY,
                    value TEXT NOT NULL,
                    created_at REAL NOT NULL
                )
            """)

            # Index on created_at for efficient cleanup of old entries
            conn.execute("""
                CREATE INDEX IF NOT EXISTS idx_created_at
                ON ocr_cache(created_at)
            """)

            conn.commit()

    def get(self, key: CacheKey) -> CacheValue | None:
        """
        Retrieve cached OCR result.

        Args:
            key: Cache key identifying the OCR request

        Returns:
            Cached value if found, None otherwise
        """
        key_json = key.to_json()

        with sqlite3.connect(self.db_path) as conn:
            cursor = conn.execute(
                "SELECT value FROM ocr_cache WHERE key = ?",
                (key_json,)
            )
            row = cursor.fetchone()

        if row is None:
            return None

        return CacheValue.from_json(row[0])

    def put(self, key: CacheKey, value: CacheValue) -> None:
        """
        Store OCR result in cache.

        Args:
            key: Cache key identifying the OCR request
            value: OCR result to cache
        """
        key_json = key.to_json()
        value_json = value.to_json()

        with sqlite3.connect(self.db_path) as conn:
            conn.execute(
                """
                INSERT OR REPLACE INTO ocr_cache (key, value, created_at)
                VALUES (?, ?, ?)
                """,
                (key_json, value_json, time.time())
            )
            conn.commit()

    def clear(self) -> None:
        """Clear all cached entries."""
        with sqlite3.connect(self.db_path) as conn:
            conn.execute("DELETE FROM ocr_cache")
            conn.commit()

    def prune_old_entries(self, max_age_seconds: float = 30 * 24 * 3600) -> int:
        """
        Remove cached entries older than specified age.

        Args:
            max_age_seconds: Maximum age in seconds (default: 30 days)

        Returns:
            Number of entries removed
        """
        cutoff = time.time() - max_age_seconds

        with sqlite3.connect(self.db_path) as conn:
            cursor = conn.execute(
                "DELETE FROM ocr_cache WHERE created_at < ?",
                (cutoff,)
            )
            removed = cursor.rowcount
            conn.commit()

        return removed

    def get_stats(self) -> dict[str, Any]:
        """
        Get cache statistics.

        Returns:
            Dictionary with cache metrics:
            - total_entries: Total number of cached items
            - db_size_bytes: Database file size in bytes
            - oldest_entry_age_seconds: Age of oldest entry
            - newest_entry_age_seconds: Age of newest entry
        """
        with sqlite3.connect(self.db_path) as conn:
            cursor = conn.execute("SELECT COUNT(*) FROM ocr_cache")
            total_entries = cursor.fetchone()[0]

            cursor = conn.execute("SELECT MIN(created_at), MAX(created_at) FROM ocr_cache")
            row = cursor.fetchone()
            oldest_ts, newest_ts = row[0], row[1]

        now = time.time()

        return {
            "total_entries": total_entries,
            "db_size_bytes": self.db_path.stat().st_size if self.db_path.exists() else 0,
            "oldest_entry_age_seconds": now - oldest_ts if oldest_ts else None,
            "newest_entry_age_seconds": now - newest_ts if newest_ts else None,
        }


def cached_ocr_call(
    cache: OcrCache,
    image: np.ndarray,
    preset: str,
    psm: int,
    threshold_mode: str,
    scale: int,
    invert: bool,
    backend_name: str,
    ocr_func: callable,
) -> tuple[str, float]:
    """
    Wrapper for OCR calls with automatic caching.

    Args:
        cache: OcrCache instance
        image: Input image
        preset: Preprocessing preset name
        psm: Tesseract PSM mode
        threshold_mode: Threshold method (adaptive, otsu, etc.)
        scale: Upscaling factor
        invert: Whether image is inverted
        backend_name: Name of OCR backend (for tracking)
        ocr_func: Function to call if cache miss (should return (token, confidence))

    Returns:
        Tuple of (token, confidence)
    """
    # Build cache key
    key = CacheKey.from_image_and_config(
        image=image,
        preset=preset,
        psm=psm,
        threshold_mode=threshold_mode,
        scale=scale,
        invert=invert,
    )

    # Try cache first
    cached = cache.get(key)
    if cached is not None:
        return cached.token, cached.confidence

    # Cache miss: run OCR
    token, confidence = ocr_func()

    # Store in cache
    value = CacheValue(
        token=token,
        confidence=confidence,
        backend_used=backend_name,
        timestamp=time.time(),
    )
    cache.put(key, value)

    return token, confidence


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description="OCR cache management utility")
    parser.add_argument("--stats", action="store_true", help="Show cache statistics")
    parser.add_argument("--clear", action="store_true", help="Clear all cached entries")
    parser.add_argument("--prune", type=int, metavar="DAYS", help="Remove entries older than N days")
    parser.add_argument("--db-path", type=Path, help="Custom database path")

    args = parser.parse_args()

    cache = OcrCache(db_path=args.db_path)

    if args.clear:
        cache.clear()
        print("Cache cleared")

    if args.prune is not None:
        removed = cache.prune_old_entries(max_age_seconds=args.prune * 24 * 3600)
        print(f"Removed {removed} entries older than {args.prune} days")

    if args.stats or (not args.clear and args.prune is None):
        stats = cache.get_stats()
        print("OCR Cache Statistics:")
        print(f"  Total entries: {stats['total_entries']}")
        print(f"  Database size: {stats['db_size_bytes'] / 1024:.1f} KB")

        if stats['oldest_entry_age_seconds'] is not None:
            oldest_days = stats['oldest_entry_age_seconds'] / (24 * 3600)
            print(f"  Oldest entry: {oldest_days:.1f} days ago")

        if stats['newest_entry_age_seconds'] is not None:
            newest_hours = stats['newest_entry_age_seconds'] / 3600
            print(f"  Newest entry: {newest_hours:.1f} hours ago")
