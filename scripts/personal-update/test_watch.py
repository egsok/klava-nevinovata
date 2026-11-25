import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from watch import poll, tag


class PollTest(unittest.TestCase):
    def test_deduplicates_and_stops_after_upstream_is_released(self):
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp, "repo")
            state = Path(tmp, "state")
            repo.mkdir()

            def git(*args):
                subprocess.run(["git", "-c", "user.name=Fixture", "-c",
                                "user.email=fixture@example.invalid", *args],
                               cwd=repo, check=True, capture_output=True)

            git("init", "-q")
            git("commit", "--allow-empty", "-m", "fork base")
            git("tag", "v1.0.0-1")
            git("commit", "--allow-empty", "-m", "new upstream")
            git("tag", "v1.1.0")
            git("tag", "v1.1.0-1")

            def write_review(repo, folder, base, upstream):
                (folder / "review.md").write_text("synthetic")

            with patch("watch.review", side_effect=write_review) as review:
                for _ in range(2):
                    self.assertEqual(poll(repo, state, ("v1.0.0-1", "v1.1.0"), True, True)
                                     ["status"], "review-ready")
                self.assertEqual(poll(repo, state, ("v1.1.0-1", "v1.1.0"), True, True)
                                 ["status"], "already-in-release")
                review.assert_called_once()

    def test_rejects_unexpected_tags(self):
        for value in ("main", "v1.0.0-rc1", "../v1.0.0"):
            with self.assertRaises(ValueError):
                tag(value, False)
