"""Crash-recoverable transactions for generated evidence files.

Each transaction writes staged files and immutable backups under the output
directory, then records a journal before replacing any public artifact. A
normal exception restores all previous files. A later invocation detects a
journal left by interruption and restores the previous committed set before it
generates new artifacts. The journal removal is the commit point.
"""

from __future__ import annotations

import json
import os
import shutil
import tempfile
import uuid
from collections.abc import Callable, Mapping
from pathlib import Path

JOURNAL_NAME = ".evidence-output-transaction.json"
JOURNAL_SCHEMA_VERSION = 1
TRANSACTION_DIRECTORY_PREFIX = ".evidence-output-transaction-"
ReplaceFunction = Callable[[Path, Path], None]


class OutputTransactionError(RuntimeError):
    """Raised when generated evidence files cannot be committed safely."""


def _fsync_directory(directory: Path) -> None:
    descriptor = os.open(directory, os.O_RDONLY)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def _atomic_write_bytes(destination: Path, content: bytes) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=f".{destination.name}.",
        suffix=".tmp",
        dir=destination.parent,
    )
    temporary_path = Path(temporary_name)
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(content)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary_path, destination)
        _fsync_directory(destination.parent)
    finally:
        temporary_path.unlink(missing_ok=True)


def _relative_destination(root: Path, destination: Path) -> str:
    try:
        return destination.resolve().relative_to(root).as_posix()
    except ValueError as error:
        raise OutputTransactionError(
            f"destination escapes transaction root: {destination}"
        ) from error


def _destination_from_relative(root: Path, relative: str) -> Path:
    destination = (root / relative).resolve()
    try:
        destination.relative_to(root)
    except ValueError as error:
        raise OutputTransactionError(
            f"transaction journal path escapes root: {relative}"
        ) from error
    return destination


def _load_journal(root: Path) -> dict[str, object] | None:
    journal_path = root / JOURNAL_NAME
    if not journal_path.exists():
        return None
    try:
        journal = json.loads(journal_path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        raise OutputTransactionError(
            f"invalid output transaction journal: {journal_path}"
        ) from error
    if not isinstance(journal, dict):
        raise OutputTransactionError(f"output transaction journal is not an object: {journal_path}")
    if journal.get("schema_version") != JOURNAL_SCHEMA_VERSION:
        raise OutputTransactionError(f"unsupported output transaction journal: {journal_path}")
    return journal


def _journal_entries(
    root: Path, journal: Mapping[str, object]
) -> tuple[Path, list[dict[str, object]]]:
    transaction_directory_name = journal.get("transaction_directory")
    entries = journal.get("entries")
    if not isinstance(transaction_directory_name, str) or not isinstance(entries, list):
        raise OutputTransactionError("output transaction journal lacks transaction data")
    if Path(
        transaction_directory_name
    ).name != transaction_directory_name or not transaction_directory_name.startswith(
        TRANSACTION_DIRECTORY_PREFIX
    ):
        raise OutputTransactionError("output transaction journal has an invalid staging directory")
    transaction_directory = _destination_from_relative(root, transaction_directory_name)
    validated_entries: list[dict[str, object]] = []
    for entry in entries:
        if not isinstance(entry, dict):
            raise OutputTransactionError("output transaction journal contains an invalid entry")
        if not isinstance(entry.get("destination"), str) or not isinstance(
            entry.get("existed"), bool
        ):
            raise OutputTransactionError("output transaction journal entry lacks destination state")
        if entry["existed"]:
            backup = entry.get("backup")
            if not isinstance(backup, str) or Path(backup).name != backup:
                raise OutputTransactionError("output transaction journal entry lacks backup data")
        validated_entries.append(entry)
    return transaction_directory, validated_entries


def _restore_entries(
    root: Path, transaction_directory: Path, entries: list[dict[str, object]]
) -> None:
    for entry in entries:
        destination = _destination_from_relative(root, str(entry["destination"]))
        if bool(entry["existed"]):
            backup = transaction_directory / "backups" / str(entry["backup"])
            if not backup.is_file():
                raise OutputTransactionError(f"output transaction backup is missing: {backup}")
            _atomic_write_bytes(destination, backup.read_bytes())
        else:
            destination.unlink(missing_ok=True)
            _fsync_directory(destination.parent)


def _remove_journal_and_staging(root: Path, transaction_directory: Path) -> None:
    journal_path = root / JOURNAL_NAME
    journal_path.unlink(missing_ok=True)
    _fsync_directory(root)
    shutil.rmtree(transaction_directory, ignore_errors=True)


def recover_pending_transaction(root: Path) -> None:
    """Restore the last committed artifact set after an interrupted write."""
    root = root.resolve()
    journal = _load_journal(root)
    if journal is None:
        return
    transaction_directory, entries = _journal_entries(root, journal)
    _restore_entries(root, transaction_directory, entries)
    _remove_journal_and_staging(root, transaction_directory)


def write_text_transaction(
    documents: Mapping[Path, str],
    *,
    root: Path,
    replace: ReplaceFunction | None = None,
) -> None:
    """Commit related UTF-8 documents together or restore their prior bytes.

    The journal removal is the commit point. A later call recovers interrupted
    replacements before it writes another artifact set.
    """
    if not documents:
        return

    root = root.resolve()
    root.mkdir(parents=True, exist_ok=True)
    recover_pending_transaction(root)
    commit_replace = os.replace if replace is None else replace

    normalized_documents: list[tuple[Path, bytes]] = []
    seen_destinations: set[Path] = set()
    for destination, text in documents.items():
        normalized_destination = destination.resolve()
        _relative_destination(root, normalized_destination)
        if normalized_destination in seen_destinations:
            raise OutputTransactionError(
                f"duplicate transaction destination: {normalized_destination}"
            )
        seen_destinations.add(normalized_destination)
        normalized_documents.append((normalized_destination, text.encode("utf-8")))
    normalized_documents.sort(key=lambda item: item[0].as_posix())

    transaction_name = f"{TRANSACTION_DIRECTORY_PREFIX}{uuid.uuid4().hex}"
    transaction_directory = root / transaction_name
    staged_directory = transaction_directory / "staged"
    backup_directory = transaction_directory / "backups"
    staged_directory.mkdir(parents=True)
    backup_directory.mkdir()

    entries: list[dict[str, object]] = []
    try:
        for index, (destination, content) in enumerate(normalized_documents):
            relative_destination = _relative_destination(root, destination)
            stage = staged_directory / f"{index}.new"
            _atomic_write_bytes(stage, content)
            entry: dict[str, object] = {
                "destination": relative_destination,
                "existed": destination.exists(),
                "stage": f"{index}.new",
            }
            if destination.exists():
                backup = backup_directory / f"{index}.old"
                shutil.copyfile(destination, backup)
                with backup.open("rb") as handle:
                    os.fsync(handle.fileno())
                entry["backup"] = backup.name
            entries.append(entry)

        journal = {
            "schema_version": JOURNAL_SCHEMA_VERSION,
            "transaction_directory": transaction_name,
            "entries": entries,
        }
        _atomic_write_bytes(
            root / JOURNAL_NAME,
            (json.dumps(journal, indent=2, sort_keys=True) + "\n").encode("utf-8"),
        )

        for entry in entries:
            stage = staged_directory / str(entry["stage"])
            destination = _destination_from_relative(root, str(entry["destination"]))
            destination.parent.mkdir(parents=True, exist_ok=True)
            commit_replace(stage, destination)
            _fsync_directory(destination.parent)
    except Exception as error:
        journal = _load_journal(root)
        if journal is not None:
            try:
                restore_directory, restore_entries = _journal_entries(root, journal)
                _restore_entries(root, restore_directory, restore_entries)
                _remove_journal_and_staging(root, restore_directory)
            except Exception as restore_error:
                raise OutputTransactionError(
                    f"output transaction failed and recovery also failed: {restore_error}"
                ) from error
        else:
            shutil.rmtree(transaction_directory, ignore_errors=True)
        raise OutputTransactionError(f"output transaction failed: {error}") from error

    _remove_journal_and_staging(root, transaction_directory)
