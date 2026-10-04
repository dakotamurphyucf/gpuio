#!/usr/bin/env python3
"""Reconstruct the reviewed native v0.17 source snapshot, never a binary cache.

Normal runs verify pinned archive and patch hashes. --record-archives is only for
an intentional source refresh; review sources.json and vendor/ together afterward.
--archive-dir uses local NAME.tar.gz archives without downloading missing files.
--output selects a new destination; --package limits a maintenance check explicitly.
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
    manifest_path = ROOT / "third_party/sources.json"
    manifest = json.loads(manifest_path.read_text())
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--record-archives", action="store_true")
    parser.add_argument("--archive-dir", type=Path, help="Use local NAME.tar.gz archives, still hash-verified; never fall back to downloads")
    parser.add_argument("--output", type=Path, default=ROOT / "vendor", help="New directory containing reconstructed package directories")
    parser.add_argument("--package", action="append", choices=tuple(manifest["native_bonsai"]), help="Reconstruct only this package; repeat for multiple packages (default: all)")
    args = parser.parse_args()
    names = list(dict.fromkeys(args.package or manifest["native_bonsai"]))
    vendor = args.output
    if vendor.exists() or vendor.is_symlink():
        raise SystemExit(f"Output already exists; choose a fresh destination: {vendor}")
    with tempfile.TemporaryDirectory(prefix="gpuio-vendor-") as temporary:
        staging = Path(temporary)
        for name in names:
            source = manifest["native_bonsai"][name]
            if args.archive_dir:
                archive = args.archive_dir / (name + ".tar.gz")
                if not archive.is_file():
                    raise SystemExit(f"Missing local archive: {archive}")
            else:
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
        vendor.mkdir(parents=True)
        for name in names:
            source = manifest["native_bonsai"][name]
            shutil.copytree(staging / (name + "-" + source["commit"]), vendor / name)
        if args.record_archives:
            manifest_path.write_text(json.dumps(manifest, indent=2) + "\n")


if __name__ == "__main__":
    main()
