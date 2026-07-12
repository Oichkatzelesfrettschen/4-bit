"""Black-box tests for the tracked-local-link verifier."""

from __future__ import annotations

import subprocess
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[2]
LINK_CHECK = REPO_ROOT / "scripts" / "link_check.sh"


def run_command(command: list[str], *, cwd: Path) -> subprocess.CompletedProcess[str]:
    """Run a fixed test command and retain its output for assertions."""
    return subprocess.run(  # noqa: S603 - commands are fixed test harness commands
        command,
        cwd=cwd,
        check=False,
        capture_output=True,
        text=True,
    )


def tracked_repository(tmp_path: Path, files: dict[str, str]) -> Path:
    """Create a minimal Git repository whose Markdown files are tracked."""
    repository = tmp_path / "repository"
    repository.mkdir()
    init = run_command(["git", "init", "--quiet"], cwd=repository)
    assert init.returncode == 0, init.stderr

    for relative_path, content in files.items():
        path = repository / relative_path
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")

    add = run_command(["git", "add", "--all"], cwd=repository)
    assert add.returncode == 0, add.stderr
    return repository


def run_link_check(repository: Path) -> subprocess.CompletedProcess[str]:
    """Run the repository-local verifier against an isolated tracked tree."""
    return run_command([str(LINK_CHECK)], cwd=repository)


def test_link_check_accepts_valid_relative_and_anchor_links(tmp_path: Path) -> None:
    repository = tracked_repository(
        tmp_path,
        {
            "README.md": "[guide](docs/guide.md#scope) [schema](docs/schema.yaml)\n",
            "docs/guide.md": "# Scope\n",
            "docs/schema.yaml": "version: 1\n",
        },
    )

    result = run_link_check(repository)

    assert result.returncode == 0, result.stderr


def test_link_check_accepts_explicit_html_and_duplicate_heading_anchors(tmp_path: Path) -> None:
    repository = tracked_repository(
        tmp_path,
        {
            "README.md": "[named](docs/guide.md#pinout) [repeat](docs/guide.md#scope-1)\n",
            "docs/guide.md": '<a id="pinout"></a>\n# Scope\n# Scope\n',
        },
    )

    result = run_link_check(repository)

    assert result.returncode == 0, result.stderr


@pytest.mark.parametrize("target", ["missing.md", "missing.yaml", "missing.yml"])
def test_link_check_rejects_missing_local_targets(tmp_path: Path, target: str) -> None:
    repository = tracked_repository(tmp_path, {"README.md": f"[missing]({target})\n"})

    result = run_link_check(repository)

    assert result.returncode == 1
    assert f"Broken link in README.md -> {target}" in result.stderr


def test_link_check_rejects_missing_local_anchor(tmp_path: Path) -> None:
    repository = tracked_repository(
        tmp_path,
        {
            "README.md": "[missing](docs/guide.md#missing-anchor)\n",
            "docs/guide.md": "# Available Anchor\n",
        },
    )

    result = run_link_check(repository)

    assert result.returncode == 1
    assert "Broken anchor in README.md -> docs/guide.md#missing-anchor" in result.stderr


def test_link_check_validates_reference_style_local_links(tmp_path: Path) -> None:
    repository = tracked_repository(
        tmp_path,
        {
            "README.md": "[guide][reference] [collapsed][]\n\n[reference]: docs/guide.md#scope\n[collapsed]: docs/guide.md#named\n",
            "docs/guide.md": '# Scope\n<a id="named"></a>\n',
        },
    )

    result = run_link_check(repository)

    assert result.returncode == 0, result.stderr


def test_link_check_ignores_undefined_reference_notation(tmp_path: Path) -> None:
    repository = tracked_repository(tmp_path, {"README.md": "[missing][reference]\n"})

    result = run_link_check(repository)

    assert result.returncode == 0, result.stderr


def test_link_check_ignores_external_links_without_network_access(tmp_path: Path) -> None:
    repository = tracked_repository(
        tmp_path,
        {"README.md": "[external](https://example.invalid/guide.md)\n"},
    )

    result = run_link_check(repository)

    assert result.returncode == 0, result.stderr
