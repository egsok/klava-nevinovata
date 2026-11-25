"""Compare a disposable Handy profile across installer upgrades and rollbacks."""

import argparse
import hashlib
import json
import sqlite3
from pathlib import Path


def digest(value):
    data = json.dumps(value, sort_keys=True, ensure_ascii=False).encode("utf-8")
    return hashlib.sha256(data).hexdigest()


def settings_digests(value, path=()):
    if isinstance(value, dict) and value:
        result = {}
        for key, item in value.items():
            result.update(settings_digests(item, (*path, key)))
        return result
    return {json.dumps(path): digest(value)}


def snapshot(profile, original_columns=None, *, require_seeded=True):
    store = json.loads((profile / "settings_store.json").read_text(encoding="utf-8-sig"))
    settings = store["settings"]
    if not isinstance(settings, dict) or not settings:
        raise ValueError("Expected initialized Handy settings")
    db_uri = (profile / "history.db").resolve().as_uri() + "?mode=ro"
    connection = sqlite3.connect(db_uri, uri=True)
    try:
        if connection.execute("PRAGMA quick_check").fetchall() != [("ok",)]:
            raise ValueError("SQLite integrity check failed")
        actual = [r[1] for r in connection.execute("PRAGMA table_info(transcription_history)")]
        columns = actual if original_columns is None else original_columns
        if not {"id", "file_name", "transcription_text"}.issubset(columns) or not set(columns).issubset(actual):
            raise ValueError("Required history columns missing")
        fields = ", ".join('"' + col.replace('"', '""') + '"' for col in columns)
        rows = {
            str(row[columns.index("id")]): digest(row)
            for row in connection.execute("SELECT " + fields + " FROM transcription_history")
        }
    finally:
        connection.close()
    recordings = profile / "recordings"
    waves = {}
    for path in recordings.rglob("*"):
        if path.suffix.lower() == ".wav" and path.is_file():
            with path.open("rb") as stream:
                waves[path.relative_to(recordings).as_posix()] = hashlib.file_digest(stream, "sha256").hexdigest()
    if require_seeded and (not rows or not waves):
        raise ValueError("Expected seeded dictations and WAV files")
    return {"columns": columns, "settings": settings_digests(settings), "history": rows, "wav": waves}


def differences(before, after):
    return {group: sum(after[group].get(key) != value for key, value in before[group].items())
            for group in ("settings", "history", "wav")}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("operation", choices=("snapshot", "verify"))
    parser.add_argument("profile", type=Path)
    parser.add_argument("manifest", type=Path)
    args = parser.parse_args()
    if args.operation == "snapshot":
        data = snapshot(args.profile)
        with args.manifest.open("x", encoding="utf-8") as out:
            json.dump(data, out)
        print("Snapshot saved:", {key: len(data[key]) for key in ("settings", "history", "wav")})
    else:
        before = json.loads(args.manifest.read_text(encoding="utf-8"))
        problems = differences(before, snapshot(args.profile, before["columns"]))
        print("Original values missing or changed:", problems)
        return int(any(problems.values()))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
