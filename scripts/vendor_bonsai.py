#!/usr/bin/env python3
"""Reconstruct the reviewed native v0.17 source snapshot, never a binary cache.

Normal runs verify pinned archive and patch hashes. --record-archives is only for
an intentional source refresh; review sources.json and vendor/ together afterward.
Requires Python 3.12+, curl and patch. Run from any directory.
"""

import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile


ROOT = Path(__file__).resolve().parent.parent


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--record-archives", action="store_true")
    args = parser.parse_args()
    manifest_path = ROOT / "third_party/sources.json"
    manifest = json.loads(manifest_path.read_text())
    vendor = ROOT / "vendor"
    if vendor.exists():
        raise SystemExit("vendor/ already exists; move it aside before a deliberate refresh")
    with tempfile.TemporaryDirectory(prefix="gpuio-vendor-") as temporary:
        staging = Path(temporary)
        for name, source in manifest["native_bonsai"].items():
            archive = staging / (name + ".tar.gz")
            url = f"https://codeload.github.com/janestreet/{name}/tar.gz/{source['commit']}"
            subprocess.run(["curl", "--fail", "--location", "--silent", "--show-error",
                            "--retry", "3", "--output", str(archive), url], check=True)
            actual = sha256(archive)
            if args.record_archives:
                source["archive_sha256"] = actual
            elif source.get("archive_sha256") != actual:
                raise SystemExit(f"Archive checksum mismatch: {name}")
            patch = ROOT / "third_party/patches" / (name + ".patch")
            if sha256(patch) != source["patch_sha256"]:
                raise SystemExit(f"Patch checksum mismatch: {name}")
            with tarfile.open(archive) as bundle:
                bundle.extractall(staging, filter="data")
            source_dir = staging / (name + "-" + source["commit"])
            patch_program = shutil.which("gpatch") or "patch"
            subprocess.run([patch_program, "--batch", "--forward", "-p1", "-i", str(patch)],
                           cwd=source_dir, check=True)
            print(f"Verified {name} at {source['commit']}", flush=True)
        vendor.mkdir()
        for name, source in manifest["native_bonsai"].items():
            shutil.copytree(staging / (name + "-" + source["commit"]), vendor / name)
        if args.record_archives:
            manifest_path.write_text(json.dumps(manifest, indent=2) + "\n")


if __name__ == "__main__":
    main()
