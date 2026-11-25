"""Exercise both NSIS packages on a fresh GitHub-hosted Windows runner only."""

import hashlib
import json
import os
import re
import shutil
import sqlite3
import subprocess
import time
import wave
from pathlib import Path

from backup import verified_backup
from profile_check import differences, snapshot


def sha256(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def output(*args):
    return subprocess.check_output(args, text=True, encoding="utf-8").strip()


def install(package, update=False):
    args = [str(package), "/S"]
    if update:
        args.append("/UPDATE")
    subprocess.run(args, check=True, timeout=120)


def installed_version():
    return output("powershell.exe", "-NoProfile", "-Command",
                  "(Get-Item -LiteralPath (Join-Path $env:LOCALAPPDATA "
                  "'klava-nevinovata/handy.exe')).VersionInfo.ProductVersion")


def launch(executable, profile):
    # The window is created after the history manager and UI have initialized.
    process = subprocess.Popen([executable, "--no-tray"],
                               env={**os.environ, "HANDY_DISABLE_UPDATER": "1"})
    try:
        for _ in range(60):
            if process.poll() is not None:
                raise RuntimeError(f"Installed app exited early ({process.returncode})")
            if (profile / "history.db").is_file() and (profile / "settings_store.json").is_file():
                handle = output("powershell.exe", "-NoProfile", "-Command",
                                f"(Get-Process -Id {process.pid}).MainWindowHandle")
                if int(handle) != 0:
                    return
            time.sleep(1)
        raise TimeoutError("Installed application never showed a window")
    finally:
        # Close only this test process. Exercise an abrupt exit before the next setup.
        if process.poll() is None:
            process.kill()
        process.wait(timeout=15)


def add_fixture(profile, indices):
    recordings = profile / "recordings"
    recordings.mkdir(exist_ok=True)
    with sqlite3.connect(profile / "history.db") as db:
        db.execute("PRAGMA journal_mode=WAL")
        for index in indices:
            name = f"fixture-{index}.wav"
            with wave.open(str(recordings / name), "wb") as audio:
                audio.setparams((1, 2, 16000, 0, "NONE", ""))
                audio.writeframes(index.to_bytes(2, "little") * 1600)
            db.execute("""INSERT INTO transcription_history
                (file_name, timestamp, saved, title, transcription_text)
                VALUES (?, ?, ?, ?, ?)""",
                       (name, 1600000000 + index, index == 0, name, f"Synthetic {index}"))
    db.close()


def verify(before, profile):
    result = differences(before, snapshot(profile, before["columns"]))
    if any(result.values()):
        raise ValueError(f"Profile values missing or changed: {result}")


def main():
    if not (os.name == "nt" and os.environ.get("RUNNER_ENVIRONMENT") == "github-hosted"
            and os.environ.get("GITHUB_ACTIONS") == "true"):
        raise SystemExit("Requires a disposable GitHub-hosted Windows runner")
    temp = Path(os.environ["RUNNER_TEMP"])
    profile = Path(os.environ["APPDATA"]) / "ru.egorsokolov.klava-nevinovata"
    exe = Path(os.environ["LOCALAPPDATA"]) / "klava-nevinovata" / "handy.exe"
    if profile.exists() or exe.parent.exists():
        raise RuntimeError("Expected a clean VM")
    config = json.loads(Path("src-tauri/tauri.conf.json").read_text())
    version = config["version"]
    if not re.fullmatch(r"\d+\.\d+\.\d+-\d+", version):
        raise ValueError("Unexpected candidate version")
    (candidate,) = (temp / "candidate").rglob(f"klava-nevinovata_{version}_x64-setup.exe")
    release = json.loads(output("gh", "release", "view", "--json", "tagName,assets"))
    base = release["tagName"]
    if not re.fullmatch(r"v\d+\.\d+\.\d+-\d+", base):
        raise ValueError("Unexpected baseline release")
    def numbers(value):
        return tuple(map(int, re.findall(r"\d+", value)))
    if numbers(version) <= numbers(base):
        raise ValueError("Candidate must be newer than the published baseline")
    base_sha = output("git", "rev-parse", base + "^{commit}")
    head = output("git", "rev-parse", "HEAD")
    if head != os.environ["GITHUB_SHA"]:
        raise ValueError("Checkout differs from workflow revision")
    subprocess.run(["git", "merge-base", "--is-ancestor", base_sha, head], check=True)
    upstream_tag = "v" + version.split("-")[0]
    upstream = output("gh", "api", f"repos/cjpais/Handy/commits/{upstream_tag}", "--jq", ".sha")
    subprocess.run(["git", "merge-base", "--is-ancestor", upstream, head], check=True)
    name = f"klava-nevinovata_{base[1:]}_x64-setup.exe"
    (asset,) = [item for item in release["assets"] if item["name"] == name]
    subprocess.run(["gh", "release", "download", base, "--pattern", name, "--dir", temp], check=True)
    old = temp / name
    if asset["digest"] != "sha256:" + sha256(old):
        raise ValueError("Published installer checksum mismatch")
    install(old)
    if installed_version() != base[1:]:
        raise ValueError("Unexpected baseline executable version")
    old_exe = sha256(exe)
    launch(exe, profile)
    store = profile / "settings_store.json"
    settings = json.loads(store.read_text(encoding="utf-8-sig"))
    settings["settings"].update(history_limit=999999, recording_retention_period="never",
                                transcription_prompt="", custom_words=["FixtureLexicon"],
                                selected_language="ru", audio_feedback_volume=0.375,
                                show_whats_new_on_update=False)
    store.write_text(json.dumps(settings), encoding="utf-8")
    add_fixture(profile, range(26))
    original = snapshot(profile)
    verified_backup(profile, temp / "pre-upgrade-backup")
    install(candidate, update=True)
    if installed_version() != version or sha256(exe) == old_exe or (exe.parent / "portable").exists():
        raise RuntimeError("Expected a new installed application")
    verify(original, profile)
    launch(exe, profile)
    verify(original, profile)
    # Synthetic database write to the schema created by the real application.
    add_fixture(profile, [26])
    verify(original, profile)
    with_new = snapshot(profile)
    if len(with_new["history"]) != 27 or len(with_new["wav"]) != 27:
        raise RuntimeError("Missing post-upgrade fixture")
    install(candidate, update=True)
    launch(exe, profile)
    verify(with_new, profile)
    install(old, update=True)
    if sha256(exe) != old_exe:
        raise ValueError("Rollback did not restore the original executable")
    verify(with_new, profile)
    launch(exe, profile)
    verify(with_new, profile)
    verified = temp / "verified"
    verified.mkdir()
    shutil.copyfile(candidate, verified / candidate.name)
    report = dict(version=version, commit=head, file=candidate.name, sha256=sha256(candidate),
                  base=base, base_commit=base_sha, base_sha256=sha256(old),
                  upstream_tag=upstream_tag, upstream_commit=upstream,
                  run_id=os.environ["GITHUB_RUN_ID"], attempt=os.environ["GITHUB_RUN_ATTEMPT"],
                  existing=26, added_synthetic=1)
    (verified / "verification.json").write_text(json.dumps(report, indent=2))
    print(json.dumps(report))


if __name__ == "__main__":
    main()
