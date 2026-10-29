# SPDX-License-Identifier: MIT
"""Reconstruct pinned upstream sources, local patches, and source overlays.

This script never opens USB or writes to a device. Recovery binaries stay local.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def git(repo, *args, check=True):
    result = subprocess.run(
        ["git", "-C", str(repo), *args], capture_output=True,
        text=True, encoding="utf-8", errors="replace",
    )
    if check and result.returncode:
        raise RuntimeError(result.stderr.strip() or result.stdout.strip())
    return result


def ensure_repo(path, url, revision, check=False):
    if not (path / ".git").exists():
        if check:
            raise RuntimeError(f"Missing checkout: {path}")
        if path.exists() and any(path.iterdir()):
            raise RuntimeError(f"Refusing to replace a non-repository directory: {path}")
        path.mkdir(parents=True, exist_ok=True)
        git(path, "init", "--quiet")
        git(path, "config", "core.autocrlf", "false")
        git(path, "remote", "add", "origin", url)
        git(path, "fetch", "--depth=1", "origin", revision)
        git(path, "checkout", "--quiet", "--detach", "FETCH_HEAD")
    actual = git(path, "rev-parse", "HEAD").stdout.strip()
    if actual != revision:
        raise RuntimeError(f"Revision mismatch in {path}: {actual}; expected {revision}")


def apply_patch(repo, patch, check=False):
    # A reverse check succeeds when this exact patch is already installed.
    if git(repo, "apply", "--reverse", "--check", str(patch), check=False).returncode == 0:
        return
    forward = git(repo, "apply", "--check", str(patch), check=False)
    if check or forward.returncode:
        raise RuntimeError(f"Patch not installed or conflicting edits in {repo}: {patch.name}")
    git(repo, "apply", str(patch))


def copy_overlay(repo, overlay, check=False):
    for source in sorted(overlay.rglob("*")):
        if not source.is_file():
            continue
        target = repo / source.relative_to(overlay)
        if target.exists():
            if target.read_bytes() != source.read_bytes():
                raise RuntimeError(f"Overlay differs; preserve your edits before retrying: {target}")
        elif check:
            raise RuntimeError(f"Missing overlay: {target}")
        else:
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, target)


def prepare_baseline(check=False):
    config_path = ROOT / "local/device.json"
    if not config_path.exists():
        raise RuntimeError("Configure local/device.json; use --sources-only to skip private recovery material")
    config = json.loads(config_path.read_text(encoding="utf-8-sig"))
    if not config.get("device_serial") or config["device_serial"] == "YOUR_DEVICE_SERIAL":
        raise RuntimeError("Configure the verified device serial in local/device.json")
    source = ROOT / config["stock_backup"] / "loader-00000004-00003fff.bin"
    target = ROOT / "vendor/km003c-firmware/fw/bootloader_local_hw2.3.bin"
    manifest_path = source.parent / "manifest.json"
    if manifest_path.exists():
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        if manifest.get("identity", {}).get("serial") != config["device_serial"]:
            raise RuntimeError("Configured serial differs from the bootstrap stock manifest")
        verifier = ROOT / "bootstrap/target/release/km003c-backup.exe"
        if not verifier.is_file():
            raise RuntimeError("Build the independent verifier first: scripts/build-backup.ps1")
        result = subprocess.run(
            [str(verifier), "verify", "--stock", str(source.parent), "--require-dfu"],
            capture_output=True, text=True, encoding="utf-8", errors="replace",
        )
        if result.returncode:
            raise RuntimeError("Bootstrap recovery verification failed: " + result.stderr.strip())
    elif not target.exists():
        raise RuntimeError("First acquire and qualify this device with docs/FIRST-BACKUP.md; no bootstrap manifest found")
    # Existing experiment workspaces retain their already installed baseline.
    # A fresh baseline requires the complete bootstrap manifest and DFU evidence.
    data = source.read_bytes()
    metadata = json.loads(Path(str(source) + ".verified.json").read_text())
    expected = Path(str(source) + ".sha256").read_text().split()[0]
    if len(data) != 0x3ffc or metadata.get("matching_reads") != 2 or hashlib.sha256(data).hexdigest() != expected:
        raise RuntimeError("The private loader baseline must have two matching reads and a valid SHA-256")
    if target.exists():
        if target.read_bytes() != data:
            raise RuntimeError("Existing private loader baseline differs; refusing to replace it")
    elif check:
        raise RuntimeError("Missing private loader baseline")
    else:
        shutil.copyfile(source, target)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="verify without cloning or writing files")
    parser.add_argument("--sources-only", action="store_true", help="skip the private hardware recovery baseline")
    args = parser.parse_args()
    lock = json.loads((ROOT / "SOURCE_LOCK.json").read_text())
    for key, folder in [("km003c-firmware", "km003c-firmware"), ("DMC100-Doom", "dmc100-doom")]:
        repo = ROOT / "vendor" / folder
        ensure_repo(repo, lock[key]["url"], lock[key]["revision"], args.check)
        apply_patch(repo, ROOT / "patches" / f"{folder}.patch", args.check)
        overlay = ROOT / "overlays" / folder
        if overlay.exists():
            copy_overlay(repo, overlay, args.check)
        print(f"Verified {folder} source revision, patch, and overlays")
    upstream = ROOT / "vendor/dmc100-doom/oracle/src/doomgeneric-repo"
    ensure_repo(upstream, lock["doomgeneric"]["url"], lock["doomgeneric"]["revision"], args.check)
    if not args.sources_only:
        prepare_baseline(args.check)
        print("Verified private loader baseline (not published)")
    print("Source preparation complete; no device was accessed")


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, RuntimeError, KeyError) as error:
        raise SystemExit(f"Error: {error}") from error
