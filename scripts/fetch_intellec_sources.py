#!/usr/bin/env python3
"""Fetch only registered Intellec evidence artifacts with verified provenance.

The registry, not a caller supplied URL, selects every network endpoint.  The
fetcher retains public artifacts in the ignored local evidence cache until a
separate rights review authorizes tracked redistribution.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path
from urllib.parse import urlsplit

import yaml

SOURCE_ID_PATTERN = re.compile(r"[a-z0-9][a-z0-9-]*\Z")
SHA256_PATTERN = re.compile(r"[0-9a-f]{64}\Z")
LOCAL_CACHE_PREFIX = Path("docs/evidence/local_sources")
ALLOWED_PROTOCOLS = frozenset({"http", "https"})
ACCESS_STATES = frozenset({"public-direct", "public-archive", "unresolved", "custodian-authorized"})
REDISTRIBUTION_STATES = frozenset({"permitted", "unresolved", "prohibited", "owner-restricted"})
RETENTION_STATES = frozenset({"tracked", "local-only", "metadata-only"})
WGET_PATH = shutil.which("wget")


class SourceRegistryError(ValueError):
    """The source registry does not authorize the requested operation."""


@dataclass(frozen=True)
class SourceArtifact:
    """One immutable local artifact selected from the source registry."""

    source_id: str
    title: str
    uri: str
    protocol: str
    local_path: Path
    sha256: str
    byte_count: int
    retention: str
    access_state: str
    redistribution_state: str


def repository_root() -> Path:
    """Return the repository root containing this script."""

    return Path(__file__).resolve().parent.parent


def sha256_file(path: Path) -> str:
    """Return the SHA-256 digest of one regular file."""

    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def relative_local_path(value: object) -> Path:
    """Validate one local cache path without resolving outside the repository."""

    if not isinstance(value, str) or not value:
        raise SourceRegistryError("source local_path is missing")
    path = Path(value)
    if path.is_absolute() or ".." in path.parts or not path.is_relative_to(LOCAL_CACHE_PREFIX):
        raise SourceRegistryError(f"local-only artifact path escapes the local cache: {value!r}")
    return path


def source_artifact(source: object) -> SourceArtifact | None:
    """Validate and normalize one fetchable local-only source entry."""

    if not isinstance(source, dict):
        raise SourceRegistryError("source entry is not a mapping")
    if source.get("retention") != "local-only":
        return None

    source_id = source.get("id")
    if not isinstance(source_id, str) or not SOURCE_ID_PATTERN.fullmatch(source_id):
        raise SourceRegistryError(f"invalid source ID: {source_id!r}")
    title = source.get("title")
    if not isinstance(title, str) or not title:
        raise SourceRegistryError(f"source {source_id} has no title")
    uri = source.get("url")
    if not isinstance(uri, str) or not uri:
        raise SourceRegistryError(f"source {source_id} has no URL")
    parsed = urlsplit(uri)
    protocol = source.get("retrieval_protocol", parsed.scheme)
    if protocol not in ALLOWED_PROTOCOLS or protocol != parsed.scheme or not parsed.netloc:
        raise SourceRegistryError(f"source {source_id} has unsupported URI protocol: {uri!r}")
    sha256 = source.get("sha256")
    if not isinstance(sha256, str) or not SHA256_PATTERN.fullmatch(sha256):
        raise SourceRegistryError(f"source {source_id} has invalid SHA-256")
    byte_count = source.get("bytes")
    if not isinstance(byte_count, int) or byte_count <= 0:
        raise SourceRegistryError(f"source {source_id} has invalid byte count")

    rights = source.get("rights", {})
    if rights and not isinstance(rights, dict):
        raise SourceRegistryError(f"source {source_id} rights must be a mapping")
    access_state = rights.get("access_state", "public-direct")
    redistribution_state = rights.get("redistribution_state", "unresolved")
    retention = rights.get("retention", source["retention"])
    if access_state not in ACCESS_STATES:
        raise SourceRegistryError(f"source {source_id} has invalid access state: {access_state!r}")
    if redistribution_state not in REDISTRIBUTION_STATES:
        raise SourceRegistryError(f"source {source_id} has invalid redistribution state: {redistribution_state!r}")
    if retention not in RETENTION_STATES or retention != "local-only":
        raise SourceRegistryError(f"source {source_id} has inconsistent local-only retention")

    return SourceArtifact(
        source_id=source_id,
        title=title,
        uri=uri,
        protocol=protocol,
        local_path=relative_local_path(source.get("local_path")),
        sha256=sha256,
        byte_count=byte_count,
        retention=retention,
        access_state=access_state,
        redistribution_state=redistribution_state,
    )


def load_registry(path: Path) -> tuple[str, list[SourceArtifact]]:
    """Load the Mozilla user agent and every valid local-only source."""

    with path.open(encoding="utf-8") as handle:
        registry = yaml.safe_load(handle)
    if not isinstance(registry, dict):
        raise SourceRegistryError("source registry root is not a mapping")
    source_set = registry.get("source_sets", {}).get("intellec", {})
    user_agent = source_set.get("user_agent") if isinstance(source_set, dict) else None
    if not isinstance(user_agent, str) or not user_agent:
        raise SourceRegistryError("intellec source set has no Mozilla user agent")

    artifacts: list[SourceArtifact] = []
    seen_ids: set[str] = set()
    for source in registry.get("sources", []):
        artifact = source_artifact(source)
        if artifact is None:
            continue
        if artifact.source_id in seen_ids:
            raise SourceRegistryError(f"duplicate source ID: {artifact.source_id}")
        seen_ids.add(artifact.source_id)
        artifacts.append(artifact)
    return user_agent, artifacts


def select_artifacts(artifacts: list[SourceArtifact], source_id: str | None, all_eligible: bool) -> list[SourceArtifact]:
    """Select source IDs without accepting an arbitrary URL or path."""

    if source_id and all_eligible:
        raise SourceRegistryError("use either --source-id or --all-eligible")
    if not source_id and not all_eligible:
        raise SourceRegistryError("select --source-id or --all-eligible")
    if all_eligible:
        return artifacts
    if source_id is None:
        raise SourceRegistryError("source selection is missing an ID")
    if not SOURCE_ID_PATTERN.fullmatch(source_id):
        raise SourceRegistryError(f"invalid requested source ID: {source_id!r}")
    selected = [artifact for artifact in artifacts if artifact.source_id == source_id]
    if not selected:
        raise SourceRegistryError(f"source ID is not a local-only artifact: {source_id}")
    return selected


def local_target(root: Path, artifact: SourceArtifact) -> Path:
    """Return one target after enforcing the repository cache boundary."""

    target = (root / artifact.local_path).resolve()
    cache_root = (root / LOCAL_CACHE_PREFIX).resolve()
    if not target.is_relative_to(cache_root):
        raise SourceRegistryError(f"source target escapes cache root: {artifact.source_id}")
    return target


def verify_artifact(target: Path, artifact: SourceArtifact) -> tuple[bool, str]:
    """Verify one local file without changing it."""

    if not target.is_file():
        return False, "missing"
    actual_bytes = target.stat().st_size
    if actual_bytes != artifact.byte_count:
        return False, f"byte-count={actual_bytes} expected={artifact.byte_count}"
    actual_sha256 = sha256_file(target)
    if actual_sha256 != artifact.sha256:
        return False, f"sha256={actual_sha256} expected={artifact.sha256}"
    return True, "verified"


def wget_version() -> str:
    """Return the selected wget version or fail before a retrieval attempt."""

    if WGET_PATH is None:
        raise SourceRegistryError("wget is required for registered HTTPS retrieval")
    result = subprocess.run(  # noqa: S603 - executable path comes from shutil.which.
        [WGET_PATH, "--version"],
        check=True,
        capture_output=True,
        text=True,
    )
    return result.stdout.splitlines()[0]


def wget_command(artifact: SourceArtifact, user_agent: str, output_path: Path | None, probe: bool) -> list[str]:
    """Build a fixed-argument wget command for one registered HTTPS artifact."""

    if artifact.protocol != "https":
        raise SourceRegistryError(
            f"source {artifact.source_id} uses {artifact.protocol}; register a reviewed protocol adapter first"
        )
    if WGET_PATH is None:
        raise SourceRegistryError("wget is required for registered HTTPS retrieval")
    command = [
        WGET_PATH,
        "--https-only",
        "--no-cookies",
        "--no-verbose",
        "--timeout=30",
        "--tries=3",
        "--max-redirect=5",
        f"--user-agent={user_agent}",
    ]
    if probe:
        command.append("--spider")
    else:
        if output_path is None:
            raise SourceRegistryError("wget fetch requires an output path")
        command.append(f"--output-document={output_path}")
    command.append(artifact.uri)
    return command


def receipt_path(root: Path, artifact: SourceArtifact) -> Path:
    """Return the ignored retrieval-receipt path for one immutable artifact."""

    return root / LOCAL_CACHE_PREFIX / ".receipts" / artifact.source_id / f"{artifact.sha256}.json"


def write_receipt(root: Path, artifact: SourceArtifact, user_agent: str, outcome: str, tool_output: str) -> None:
    """Write a local receipt without credentials, cookies, or proxy settings."""

    receipt = {
        "schema": "intellec-source-receipt-v1",
        "source_id": artifact.source_id,
        "requested_uri": artifact.uri,
        "protocol": artifact.protocol,
        "mozilla_user_agent": user_agent,
        "expected_bytes": artifact.byte_count,
        "expected_sha256": artifact.sha256,
        "outcome": outcome,
        "retrieved_utc": datetime.now(UTC).isoformat(),
        "wget_version": wget_version(),
        "transport_output": tool_output[-4096:],
    }
    path = receipt_path(root, artifact)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="ascii")


def fetch_artifact(root: Path, artifact: SourceArtifact, user_agent: str) -> tuple[bool, str]:
    """Fetch one source atomically after validation and local-cache checks."""

    if artifact.access_state not in {"public-direct", "public-archive"}:
        return False, f"access-state={artifact.access_state} rejects automated retrieval"
    if artifact.redistribution_state == "prohibited":
        return False, "redistribution state prohibits local retrieval"
    target = local_target(root, artifact)
    valid, detail = verify_artifact(target, artifact)
    if valid:
        return True, "already verified"
    if target.exists():
        return False, f"existing target fails verification: {detail}"

    target.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(prefix=f"{artifact.source_id}-", suffix=".part", dir=target.parent, delete=False) as handle:
        temporary_path = Path(handle.name)
    try:
        result = subprocess.run(  # noqa: S603 - argv comes only from the validated registry.
            wget_command(artifact, user_agent, temporary_path, probe=False),
            check=False,
            capture_output=True,
            text=True,
        )
        output = result.stdout + result.stderr
        if result.returncode != 0:
            write_receipt(root, artifact, user_agent, "download-failed", output)
            return False, f"wget failed with status {result.returncode}"
        valid, detail = verify_artifact(temporary_path, artifact)
        if not valid:
            write_receipt(root, artifact, user_agent, "integrity-failed", output)
            return False, detail
        os.replace(temporary_path, target)
        write_receipt(root, artifact, user_agent, "verified", output)
        return True, "downloaded and verified"
    finally:
        temporary_path.unlink(missing_ok=True)


def probe_artifact(artifact: SourceArtifact, user_agent: str) -> tuple[bool, str]:
    """Probe one exact registered endpoint without retaining an artifact."""

    result = subprocess.run(  # noqa: S603 - argv comes only from the validated registry.
        wget_command(artifact, user_agent, output_path=None, probe=True),
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode == 0:
        return True, "reachable"
    return False, f"wget probe failed with status {result.returncode}"


def parse_arguments(arguments: list[str]) -> argparse.Namespace:
    """Parse the registry-only source-selection command line."""

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-id", help="registered local-only source ID")
    parser.add_argument("--all-eligible", action="store_true", help="select every registered local-only source")
    operation = parser.add_mutually_exclusive_group(required=True)
    operation.add_argument("--list", action="store_true", help="list registered local-only source IDs")
    operation.add_argument("--verify", action="store_true", help="verify local files without network access")
    operation.add_argument("--probe", action="store_true", help="probe exact registered HTTPS endpoints")
    operation.add_argument("--fetch", action="store_true", help="fetch exact registered HTTPS endpoints")
    return parser.parse_args(arguments)


def main(arguments: list[str] | None = None) -> int:
    """Run one registry-constrained source operation."""

    parsed = parse_arguments(sys.argv[1:] if arguments is None else arguments)
    root = repository_root()
    manifest = Path(os.environ.get("INTELLEC_SOURCE_MANIFEST", root / "docs/evidence/intellec_sources.yaml"))
    try:
        user_agent, artifacts = load_registry(manifest)
        if parsed.list:
            for artifact in artifacts:
                print(f"{artifact.source_id}\t{artifact.protocol}\t{artifact.retention}\t{artifact.title}")
            return 0
        selected = select_artifacts(artifacts, parsed.source_id, parsed.all_eligible)
        failed = False
        for artifact in selected:
            if parsed.verify:
                passed, detail = verify_artifact(local_target(root, artifact), artifact)
            elif parsed.probe:
                passed, detail = probe_artifact(artifact, user_agent)
            else:
                passed, detail = fetch_artifact(root, artifact, user_agent)
            print(f"{'PASS' if passed else 'FAIL'} {artifact.source_id} {detail}")
            failed |= not passed
        return int(failed)
    except (OSError, SourceRegistryError, yaml.YAMLError) as error:
        print(f"source registry error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
