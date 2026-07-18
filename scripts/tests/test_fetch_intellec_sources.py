from __future__ import annotations

from pathlib import Path

import pytest
from scripts.fetch_intellec_sources import (
    SourceRegistryError,
    load_registry,
    select_artifacts,
    source_artifact,
    verify_artifact,
    wget_command,
)

REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
REGISTRY = REPOSITORY_ROOT / "docs/evidence/intellec_sources.yaml"


def registered_source() -> dict[str, object]:
    return {
        "id": "example-intellec-source",
        "title": "Example Intellec Source",
        "local_path": "docs/evidence/local_sources/intellec/example.bin",
        "url": "https://example.invalid/example.bin",
        "sha256": "0" * 64,
        "bytes": 1,
        "retention": "local-only",
    }


def test_registry_loads_registered_local_only_sources() -> None:
    user_agent, artifacts = load_registry(REGISTRY)

    assert user_agent.startswith("Mozilla/5.0")
    assert any(artifact.source_id == "intellec4-mod40-reference-manual-98-095a" for artifact in artifacts)
    assert all(artifact.local_path.is_relative_to(Path("docs/evidence/local_sources")) for artifact in artifacts)


def test_registry_rejects_unsafe_local_cache_path() -> None:
    source = registered_source()
    source["local_path"] = "docs/evidence/local_sources/../outside.bin"

    with pytest.raises(SourceRegistryError, match="escapes the local cache"):
        source_artifact(source)


def test_registry_rejects_protocol_mismatch() -> None:
    source = registered_source()
    source["retrieval_protocol"] = "http"

    with pytest.raises(SourceRegistryError, match="unsupported URI protocol"):
        source_artifact(source)


def test_source_selection_never_accepts_an_arbitrary_url() -> None:
    _, artifacts = load_registry(REGISTRY)

    with pytest.raises(SourceRegistryError, match="source ID is not a local-only artifact"):
        select_artifacts(artifacts, "https-example-invalid-object", False)


def test_verifier_requires_exact_bytes_and_digest(tmp_path: Path) -> None:
    source = registered_source()
    source["sha256"] = "6e340b9cffb37a989ca544e6bb780a2c78901d3fb33738768511a30617afa01d"
    artifact = source_artifact(source)
    assert artifact is not None
    target = tmp_path / "example.bin"

    target.write_bytes(b"\x00")
    assert verify_artifact(target, artifact) == (True, "verified")

    target.write_bytes(b"\x00\x00")
    assert verify_artifact(target, artifact)[0] is False


def test_wget_command_uses_registered_https_and_mozilla_profile(tmp_path: Path) -> None:
    artifact = source_artifact(registered_source())
    assert artifact is not None

    command = wget_command(artifact, "Mozilla/5.0 example", tmp_path / "artifact.part", probe=False)

    assert command[1] == "--https-only"
    assert "--no-cookies" in command
    assert "--max-redirect=5" in command
    assert "--user-agent=Mozilla/5.0 example" in command
    assert command[-1] == artifact.uri
