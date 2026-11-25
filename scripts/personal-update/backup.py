"""Create a consistent settings, SQLite and recording backup for a quiescent profile."""

import json
import shutil
import sqlite3
from contextlib import closing

from profile_check import snapshot


def copy_data(profile, target):
    target.mkdir()
    shutil.copy2(profile / "settings_store.json", target)
    shutil.copytree(profile / "recordings", target / "recordings")
    # The SQLite backup API includes committed transactions still in the WAL.
    uri = (profile / "history.db").resolve().as_uri() + "?mode=ro"
    with closing(sqlite3.connect(uri, uri=True)) as source:
        with closing(sqlite3.connect(target / "history.db")) as dest:
            source.backup(dest)


def verified_backup(profile, destination):
    profile = profile.resolve()
    destination = destination.resolve()
    if destination.is_relative_to(profile):
        raise ValueError("Backup must be outside the profile")
    # Fresh path for every attempt. Only a completed verification creates a receipt.
    destination.mkdir()
    saved = destination / "data"
    before = snapshot(profile, require_seeded=False)
    copy_data(profile, saved)
    restored = destination / "restore-check"
    copy_data(saved, restored)
    if snapshot(restored, require_seeded=False) != before or snapshot(profile, require_seeded=False) != before:
        raise ValueError("Restore differs from source or source changed during backup")
    # Restore verification also covers settings outside the app settings object.
    if (profile / "settings_store.json").read_bytes() != (restored / "settings_store.json").read_bytes():
        raise ValueError("Restored settings store differs")
    shutil.rmtree(restored)
    counts = {k: len(before[k]) for k in ("settings", "history", "wav")}
    with (destination / "verified.json").open("x", encoding="utf-8") as receipt:
        json.dump(counts, receipt)
    return counts
