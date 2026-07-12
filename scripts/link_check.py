#!/usr/bin/env python3
"""Verify tracked local Markdown links and Markdown anchors without network access."""

from __future__ import annotations

import re
import shutil
import subprocess
import sys
from collections.abc import Iterator
from pathlib import Path
from urllib.parse import unquote, urlsplit

CHECKED_SUFFIXES = {".md", ".yaml", ".yml"}
MARKDOWN_SUFFIX = ".md"
FENCE_RE = re.compile(r"^[ \t]*(`{3,}|~{3,})")
HEADING_RE = re.compile(r"^[ \t]{0,3}#{1,6}[ \t]+(.+?)(?:[ \t]+#+[ \t]*)?$")
HTML_ANCHOR_RE = re.compile(
    r"<a\b[^>]*\b(?:id|name)\s*=\s*[\"']([^\"']+)[\"'][^>]*>", re.IGNORECASE
)
INLINE_LINK_RE = re.compile(r"!?\[(?P<text>[^\]\n]*)\]\((?P<target>[^\)\n]+)\)")
REFERENCE_DEFINITION_RE = re.compile(
    r"^[ \t]{0,3}\[(?P<label>[^\]\n]+)\]:[ \t]*(?:<(?P<bracketed>[^>\n]+)>|(?P<plain>\S+))"
)
REFERENCE_USE_RE = re.compile(r"(?<!!)\[(?P<text>[^\]\n]+)\]\[(?P<label>[^\]\n]*)\]")
INLINE_CODE_RE = re.compile(r"`[^`]*`")


def git_binary() -> str:
    """Return the absolute git binary path required for tracked-file discovery."""
    binary = shutil.which("git")
    if binary is None:
        raise RuntimeError("git is required to verify tracked Markdown links")
    return binary


def run_git(arguments: list[str]) -> subprocess.CompletedProcess[bytes]:
    """Run a fixed git command without a shell."""
    return subprocess.run(  # noqa: S603 - command and arguments are fixed by this verifier
        [git_binary(), *arguments],
        check=False,
        capture_output=True,
    )


def repository_root() -> Path:
    """Resolve the Git worktree that defines the verifier input boundary."""
    result = run_git(["rev-parse", "--show-toplevel"])
    if result.returncode != 0:
        message = result.stderr.decode("utf-8", errors="replace").strip()
        raise RuntimeError(message or "link verification requires a Git worktree")
    return Path(result.stdout.decode("utf-8").strip()).resolve()


def tracked_markdown_files(root: Path) -> list[Path]:
    """Return tracked Markdown files, excluding generated planning residue."""
    result = run_git(["-C", str(root), "ls-files", "-z", "*.md", ":(exclude).claude_plans/**"])
    if result.returncode != 0:
        message = result.stderr.decode("utf-8", errors="replace").strip()
        raise RuntimeError(message or "git ls-files failed")
    return [root / entry.decode("utf-8") for entry in result.stdout.split(b"\0") if entry]


def non_fenced_lines(content: str) -> Iterator[tuple[int, str]]:
    """Yield Markdown lines outside fenced code blocks."""
    active_fence: str | None = None
    for number, line in enumerate(content.splitlines(), start=1):
        fence = FENCE_RE.match(line)
        if fence is not None:
            marker = fence.group(1)[0]
            if active_fence is None:
                active_fence = marker
            elif active_fence == marker:
                active_fence = None
            continue
        if active_fence is None:
            yield number, line


def normalize_reference_label(label: str) -> str:
    """Apply Markdown's case-insensitive whitespace-normalized reference labels."""
    return " ".join(label.casefold().split())


def github_anchor_slug(heading: str) -> str:
    """Construct the ASCII GitHub-style fragment identifier for an ATX heading."""
    without_inline_code = INLINE_CODE_RE.sub(lambda match: match.group(0)[1:-1], heading)
    lowered = without_inline_code.casefold()
    kept = "".join(
        character
        for character in lowered
        if character.isascii() and (character.isalnum() or character in " -_")
    )
    return "-".join(kept.split()).strip("-")


def document_anchors(content: str) -> set[str]:
    """Collect explicit anchors and GitHub-style heading anchors from Markdown content."""
    anchors: set[str] = set()
    heading_counts: dict[str, int] = {}
    for _, line in non_fenced_lines(content):
        for anchor in HTML_ANCHOR_RE.findall(line):
            anchors.add(unquote(anchor).casefold())
        heading = HEADING_RE.match(line)
        if heading is None:
            continue
        base = github_anchor_slug(heading.group(1))
        if not base:
            continue
        occurrence = heading_counts.get(base, 0)
        heading_counts[base] = occurrence + 1
        anchors.add(base if occurrence == 0 else f"{base}-{occurrence}")
    return anchors


def parse_target(raw_target: str) -> tuple[str, str]:
    """Separate a Markdown destination from its optional fragment identifier."""
    target = raw_target.strip()
    if target.startswith("<") and ">" in target:
        target = target[1 : target.index(">")]
    else:
        target = target.split(maxsplit=1)[0] if target else ""
    parsed = urlsplit(target)
    if parsed.scheme or parsed.netloc:
        return "", ""
    return unquote(parsed.path), unquote(parsed.fragment)


def resolve_local_target(root: Path, source: Path, path_text: str) -> Path:
    """Resolve a repository-local target relative to the document that names it."""
    if path_text.startswith("/"):
        candidate = (root / path_text.lstrip("/")).resolve()
    else:
        candidate = (source.parent / path_text).resolve()
    try:
        candidate.relative_to(root)
    except ValueError as error:
        raise ValueError("local link escapes the repository") from error
    return candidate


def is_checked_local_target(path_text: str, fragment: str) -> bool:
    """Keep the verifier scoped to tracked Markdown and YAML documentation targets."""
    if not path_text:
        return bool(fragment)
    return Path(path_text).suffix in CHECKED_SUFFIXES


def validate_target(
    *,
    root: Path,
    source: Path,
    raw_target: str,
    anchor_cache: dict[Path, set[str]],
) -> list[str]:
    """Validate one local destination and optional Markdown fragment."""
    path_text, fragment = parse_target(raw_target)
    if not is_checked_local_target(path_text, fragment):
        return []

    try:
        target = source if not path_text else resolve_local_target(root, source, path_text)
    except ValueError as error:
        return [f"Broken link in {source.relative_to(root)} -> {raw_target}: {error}"]

    display_target = path_text or source.relative_to(root).as_posix()
    if not target.is_file():
        return [f"Broken link in {source.relative_to(root)} -> {display_target}"]

    if not fragment or target.suffix != MARKDOWN_SUFFIX:
        return []
    anchors = anchor_cache.get(target)
    if anchors is None:
        anchors = document_anchors(target.read_text(encoding="utf-8"))
        anchor_cache[target] = anchors
    normalized_fragment = fragment.casefold()
    if normalized_fragment not in anchors:
        return [f"Broken anchor in {source.relative_to(root)} -> {display_target}#{fragment}"]
    return []


def validate_document(root: Path, source: Path, anchor_cache: dict[Path, set[str]]) -> list[str]:
    """Validate inline and reference-style local links in one tracked Markdown file."""
    content = source.read_text(encoding="utf-8")
    lines = list(non_fenced_lines(content))
    definitions: dict[str, str] = {}
    for _, line in lines:
        definition = REFERENCE_DEFINITION_RE.match(line)
        if definition is None:
            continue
        target = definition.group("bracketed") or definition.group("plain")
        definitions.setdefault(normalize_reference_label(definition.group("label")), target)

    errors: list[str] = []
    for _, line in lines:
        definition = REFERENCE_DEFINITION_RE.match(line)
        if definition is not None:
            continue
        prose_line = INLINE_CODE_RE.sub("", line)
        for match in INLINE_LINK_RE.finditer(prose_line):
            errors.extend(
                validate_target(
                    root=root,
                    source=source,
                    raw_target=match.group("target"),
                    anchor_cache=anchor_cache,
                )
            )
        reference_line = INLINE_LINK_RE.sub("", prose_line)
        for match in REFERENCE_USE_RE.finditer(reference_line):
            label = match.group("label") or match.group("text")
            normalized_label = normalize_reference_label(label)
            target = definitions.get(normalized_label)
            if target is None:
                continue
            errors.extend(
                validate_target(
                    root=root,
                    source=source,
                    raw_target=target,
                    anchor_cache=anchor_cache,
                )
            )
    return errors


def main() -> int:
    """Run the offline tracked-local-link verification gate."""
    try:
        root = repository_root()
        errors: list[str] = []
        anchor_cache: dict[Path, set[str]] = {}
        for source in tracked_markdown_files(root):
            errors.extend(validate_document(root, source, anchor_cache))
    except (OSError, RuntimeError, UnicodeDecodeError) as error:
        print(f"Link verifier failed: {error}", file=sys.stderr)
        return 2

    if errors:
        for error in sorted(set(errors)):
            print(error, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
