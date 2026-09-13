#!/usr/bin/env python3
"""Reconstruct the pinned GPUI Base sources and reviewed GPUIO adaptations.

Normal builds use the committed snapshot. Run deliberately when refreshing the
source pin; never mutate an existing snapshot. Python 3.12+, curl and patch.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parent.parent


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def toml_value(value):
    if isinstance(value, dict):
        return "{ " + ", ".join(f"{key} = {toml_value(item)}" for key, item in value.items()) + " }"
    return json.dumps(value)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", type=Path, help="Use a local archive, still hash-verified")
    parser.add_argument("--output", type=Path, default=ROOT / "vendor/gpui-base")
    args = parser.parse_args()
    source = json.loads((ROOT / "third_party/sources.json").read_text())["gpui_base"]
    if args.output.exists():
        raise SystemExit("Output already exists; move it aside before a deliberate refresh")
    with tempfile.TemporaryDirectory(prefix="gpuio-gpui-base-") as temporary:
        staging = Path(temporary)
        archive = args.archive or staging / "source.tar.gz"
        if not args.archive:
            url = f"https://codeload.github.com/longbridge/gpui-kit/tar.gz/{source['commit']}"
            subprocess.run(["curl", "--fail", "--location", "--silent", "--show-error",
                            "--retry", "3", "--output", str(archive), url], check=True)
        if digest(archive) != source["archive_sha256"]:
            raise SystemExit("GPUI Base archive checksum mismatch")
        with tarfile.open(archive) as bundle:
            bundle.extractall(staging, filter="data")
        upstream = staging / ("gpui-kit-" + source["commit"])
        base = upstream / "crates/base"
        workspace = tomllib.loads((upstream / "Cargo.toml").read_text())["workspace"]["dependencies"]
        zed = {"git": "https://github.com/zed-industries/zed.git", "rev": source["gpui_commit"]}
        workspace.update({key: dict(zed) for key in ("gpui", "gpui_macros", "reqwest_client")})
        workspace["gpui"]["default-features"] = False
        workspace["sum-tree"] = {"package": "sum_tree", **zed}
        original = (base / "Cargo.toml").read_text()
        (base / "Cargo.toml.upstream").write_text(original)
        lines = []
        for line in original.splitlines():
            if line == "edition.workspace = true":
                line = 'edition = "2024"'
            elif line.endswith(".workspace = true"):
                key = line.split(".", 1)[0]
                line = f"{key} = {toml_value(workspace[key])}"
            elif line == 'gpui = { workspace = true, features = ["test-support"] }':
                line = "gpui = " + toml_value({**workspace["gpui"], "features": ["test-support"]})
            elif line in ("[lints]", "workspace = true"):
                continue
            lines.append(line)
        (base / "Cargo.toml").write_text("\n".join(lines).rstrip() + "\n")
        shutil.copy2(upstream / "LICENSE-APACHE", base / "LICENSE-APACHE")
        shutil.copy2(upstream / "README.md", base / "UPSTREAM_README.md")
        patch = ROOT / "third_party/patches/gpui-base.patch"
        if digest(patch) != source["patch_sha256"]:
            raise SystemExit("GPUI Base patch checksum mismatch")
        subprocess.run([shutil.which("gpatch") or "patch", "--batch", "--forward", "-p1", "-i", str(patch)], cwd=base, check=True)
        args.output.parent.mkdir(parents=True, exist_ok=True)
        shutil.copytree(base, args.output)
        print(f"Verified GPUI Base at {source['commit']} against GPUI {source['gpui_commit']}")


if __name__ == "__main__":
    main()
