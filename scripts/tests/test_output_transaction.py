"""Tests for crash-recoverable generated-artifact writes."""

from __future__ import annotations

import json
import os
from pathlib import Path

import pytest

import output_transaction


def test_transaction_commits_all_documents(tmp_path: Path) -> None:
    """A successful transaction replaces every requested document."""
    first = tmp_path / "first.json"
    second = tmp_path / "second.json"
    first.write_text("old-first\n", encoding="utf-8")
    second.write_text("old-second\n", encoding="utf-8")

    output_transaction.write_text_transaction(
        {first: "new-first\n", second: "new-second\n"},
        root=tmp_path,
    )

    assert first.read_text(encoding="utf-8") == "new-first\n"
    assert second.read_text(encoding="utf-8") == "new-second\n"
    assert not (tmp_path / output_transaction.JOURNAL_NAME).exists()


def test_transaction_rolls_back_when_later_replace_fails(tmp_path: Path) -> None:
    """A failed later replacement restores every prior public artifact."""
    first = tmp_path / "first.json"
    second = tmp_path / "second.json"
    first.write_text("old-first\n", encoding="utf-8")
    second.write_text("old-second\n", encoding="utf-8")
    replace_count = 0

    def fail_second_replace(source: Path, destination: Path) -> None:
        nonlocal replace_count
        replace_count += 1
        if replace_count == 2:
            raise OSError("injected replacement failure")
        os.replace(source, destination)

    with pytest.raises(
        output_transaction.OutputTransactionError, match="output transaction failed"
    ):
        output_transaction.write_text_transaction(
            {first: "new-first\n", second: "new-second\n"},
            root=tmp_path,
            replace=fail_second_replace,
        )

    assert first.read_text(encoding="utf-8") == "old-first\n"
    assert second.read_text(encoding="utf-8") == "old-second\n"
    assert not (tmp_path / output_transaction.JOURNAL_NAME).exists()


def test_recovery_restores_artifacts_after_interruption(tmp_path: Path) -> None:
    """A retained journal rolls an interrupted write back before new output."""
    destination = tmp_path / "artifact.json"
    destination.write_text("new\n", encoding="utf-8")
    transaction_directory = tmp_path / f"{output_transaction.TRANSACTION_DIRECTORY_PREFIX}test"
    backup_directory = transaction_directory / "backups"
    backup_directory.mkdir(parents=True)
    (backup_directory / "0.old").write_text("old\n", encoding="utf-8")
    journal = {
        "schema_version": output_transaction.JOURNAL_SCHEMA_VERSION,
        "transaction_directory": transaction_directory.name,
        "entries": [
            {
                "destination": destination.name,
                "existed": True,
                "backup": "0.old",
                "stage": "0.new",
            }
        ],
    }
    (tmp_path / output_transaction.JOURNAL_NAME).write_text(
        json.dumps(journal, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )

    output_transaction.recover_pending_transaction(tmp_path)

    assert destination.read_text(encoding="utf-8") == "old\n"
    assert not (tmp_path / output_transaction.JOURNAL_NAME).exists()
    assert not transaction_directory.exists()
