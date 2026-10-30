# SPDX-License-Identifier: MIT
"""Reconstruct pinned upstream sources, local patches, and source overlays.

This script never opens USB or writes to a device. Recovery binaries stay local.
"""
import argparse
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


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="verify without cloning or writing files")
    parser.add_argument("--sources-only", action="store_true", help="compatibility option; independent builds never require recovery data")
    args = parser.parse_args()
    lock = json.loads((ROOT / "SOURCE_LOCK.json").read_text())
    for key, folder in [("DMC100-Doom", "dmc100-doom")]:
        repo = ROOT / "vendor" / folder
        ensure_repo(repo, lock[key]["url"], lock[key]["revision"], args.check)
        apply_patch(repo, ROOT / "patches" / f"{folder}.patch", args.check)
        overlay = ROOT / "overlays" / folder
        if overlay.exists():
            copy_overlay(repo, overlay, args.check)
        print(f"Verified {folder} source revision, patch, and overlays")
    upstream = ROOT / "vendor/dmc100-doom/oracle/src/doomgeneric-repo"
    ensure_repo(upstream, lock["doomgeneric"]["url"], lock["doomgeneric"]["revision"], args.check)
    print("Source preparation complete; no device was accessed")


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, RuntimeError, KeyError) as error:
        raise SystemExit(f"Error: {error}") from error
