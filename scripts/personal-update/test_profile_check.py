import json
import sqlite3
import tempfile
import unittest
import wave
from pathlib import Path

from profile_check import differences, snapshot
from backup import verified_backup


class ProfileCheckTest(unittest.TestCase):
    def test_detects_changes_even_if_counts_stay_the_same(self):
        with tempfile.TemporaryDirectory() as tmp:
            profile = Path(tmp)
            store = profile / "settings_store.json"
            store.write_text(json.dumps({"settings": {"history_limit": 999999, "prompt": "", "retention": "never"}}))
            waves = profile / "recordings"
            waves.mkdir()
            db = sqlite3.connect(profile / "history.db")
            try:
                db.execute("PRAGMA journal_mode=WAL")
                db.execute("CREATE TABLE transcription_history(id INTEGER PRIMARY KEY, file_name TEXT, transcription_text TEXT, saved INTEGER)")
                for i in range(25):
                    name = f"{i}.wav"
                    with wave.open(str(waves / name), "wb") as audio:
                        audio.setparams((1, 2, 16000, 0, "NONE", ""))
                        audio.writeframes(i.to_bytes(2, "little") * 160)
                    db.execute("INSERT INTO transcription_history VALUES (?,?,?,?)", (i, name, f"test {i}", int(i == 0)))
                db.commit()
                before = snapshot(profile)
                self.assertEqual(len(before["history"]), 25)
                with tempfile.TemporaryDirectory() as backup_dir:
                    backup = Path(backup_dir) / "backup"
                    self.assertEqual(verified_backup(profile, backup)["history"], 25)
                    self.assertEqual(snapshot(backup / "data"), before)
                    with self.assertRaises(FileExistsError):
                        verified_backup(profile, backup)
                db.execute("INSERT INTO transcription_history VALUES (25,'25.wav','new test',0)")
                db.commit()
                (waves / "25.wav").write_bytes((waves / "0.wav").read_bytes())
                after_dictation = snapshot(profile)
                self.assertFalse(any(differences(before, after_dictation).values()))
                (waves / "25.wav").unlink()
                self.assertEqual(differences(after_dictation, snapshot(profile))["wav"], 1)
                (waves / "25.wav").write_bytes((waves / "0.wav").read_bytes())
                db.execute("UPDATE transcription_history SET transcription_text='changed' WHERE id=5")
                db.commit()
                (waves / "7.wav").write_bytes((waves / "8.wav").read_bytes())
                store.write_text(json.dumps({"settings": {"history_limit": 20, "prompt": "", "retention": "never"}}))
                self.assertEqual(differences(before, snapshot(profile)), {"settings": 1, "history": 1, "wav": 1})
            finally:
                db.close()


if __name__ == "__main__":
    unittest.main()
