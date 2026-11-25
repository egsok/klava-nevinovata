"""Validate the tested installer, complete release payload and updater manifest."""

import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
PIN = ROOT / ".github/verified-windows.json"


def read(path):
    return json.loads(path.read_text(encoding="utf-8"))


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def setup_name(version):
    return f"klava-nevinovata_{version}_x64-setup.exe"


def check_source(pin):
    version = pin["version"]
    config = read(ROOT / "src-tauri/tauri.conf.json")
    require(config["version"] == version, "Tag configuration and candidate version differ")
    require(os.environ.get("GITHUB_REF", f"refs/tags/v{version}") == f"refs/tags/v{version}",
            "Release must run on its version tag")
    subprocess.run(["git", "merge-base", "--is-ancestor", pin["commit"], "HEAD"], cwd=ROOT, check=True)
    allowed = {
        ".github/verified-windows.json", ".github/workflows/release.yml",
        "README.md", "README.ru.md", f"RELEASE-NOTES-v{version}.md", "docs/release-notes.md",
        "scripts/check-release-notes.mjs", "scripts/verify-release.py",
    }
    changes = subprocess.check_output(
        ["git", "diff", "--no-renames", "--name-only", "-z", pin["commit"], "HEAD"],
        cwd=ROOT).decode().split("\0")
    unexpected = set(filter(None, changes)) - allowed
    require(not unexpected, f"Tagged application differs from tested source: {sorted(unexpected)}")


def check_candidate(directory, pin):
    receipt = read(directory / "verification.json")
    require(all(receipt[key] == value for key, value in pin.items()), "Candidate receipt mismatch")
    name = setup_name(pin["version"])
    require(receipt["file"] == name and digest(directory / name) == pin["sha256"],
            "Tested NSIS digest or name mismatch")


def routes(version):
    prefix = f"klava-nevinovata_{version}"
    mac_arm = "klava-nevinovata_aarch64.app.tar.gz"
    mac_intel = "klava-nevinovata_x64.app.tar.gz"
    image = prefix + "_amd64.AppImage"
    msi = prefix + "_x64_en-US.msi"
    return {
        "darwin-aarch64": mac_arm, "darwin-aarch64-app": mac_arm,
        "darwin-x86_64": mac_intel, "darwin-x86_64-app": mac_intel,
        "linux-x86_64": image, "linux-x86_64-appimage": image,
        "linux-x86_64-deb": prefix + "_amd64.deb",
        "linux-x86_64-rpm": f"klava-nevinovata-{version}-1.x86_64.rpm",
        "windows-x86_64": msi, "windows-x86_64-msi": msi,
        "windows-x86_64-nsis": setup_name(version),
    }


def check_manifest(directory, version, *, update_nsis=False):
    manifest = read(directory / "latest.json")
    require(manifest["version"].removeprefix("v") == version, "Wrong updater version")
    platforms = manifest["platforms"]
    expected = routes(version)
    require(platforms.keys() == expected.keys(), "Updater route inventory changed")
    for platform, name in expected.items():
        entry = platforms[platform]
        url = f"https://github.com/egsok/klava-nevinovata/releases/download/v{version}/{name}"
        require(entry["url"] == url, f"Wrong updater URL for {platform}")
        signature = (directory / (name + ".sig")).read_text(encoding="ascii").strip()
        require(signature, f"Empty signature for {name}")
        if update_nsis and platform == "windows-x86_64-nsis":
            entry["signature"] = signature
        require(entry["signature"] == signature, f"Updater signature differs for {platform}")
    if update_nsis:
        (directory / "latest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")


def check_bundle(directory, version):
    signed = set(routes(version).values())
    expected = signed | {name + ".sig" for name in signed} | {
        "latest.json", f"klava-nevinovata_{version}_aarch64.dmg", f"klava-nevinovata_{version}_x64.dmg",
    }
    require({p.name for p in directory.iterdir()} == expected, "Wrong release asset inventory")
    check_manifest(directory, version)
    pub = read(ROOT / "src-tauri/tauri.conf.json")["plugins"]["updater"]["pubkey"]
    with tempfile.TemporaryDirectory() as temporary:
        key = Path(temporary) / "updater.pub"
        sig = Path(temporary) / "updater.minisig"
        key.write_bytes(base64.b64decode(pub, validate=True))
        for name in sorted(signed):
            encoded = (directory / (name + ".sig")).read_text(encoding="ascii").strip()
            sig.write_bytes(base64.b64decode(encoded, validate=True))
            subprocess.run(["minisign", "-V", "-q", "-p", str(key), "-x", str(sig),
                            "-m", str(directory / name)], check=True)


def check_uploaded(directory, pin, release):
    require(release["isDraft"] and release["tagName"] == "v" + pin["version"], "Expected draft")
    received = {asset["name"]: asset["digest"] for asset in release["assets"]}
    expected = {p.name: "sha256:" + digest(p) for p in directory.iterdir()}
    require(received == expected, "Uploaded asset digests differ from checked bundle")
    require(received[setup_name(pin["version"])] == "sha256:" + pin["sha256"], "NSIS digest changed")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("step", choices=["source", "candidate", "replace-signature", "bundle", "uploaded"])
    parser.add_argument("directory", nargs="?", type=Path)
    parser.add_argument("--release", type=Path)
    args = parser.parse_args()
    pin = read(PIN)
    if args.step == "source":
        check_source(pin)
    elif args.step == "candidate":
        check_candidate(args.directory, pin)
    elif args.step == "replace-signature":
        require(digest(args.directory / setup_name(pin["version"])) == pin["sha256"], "NSIS digest changed")
        check_manifest(args.directory, pin["version"], update_nsis=True)
    elif args.step == "bundle":
        check_bundle(args.directory, pin["version"])
    else:
        check_uploaded(args.directory, pin, read(args.release))
