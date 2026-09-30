#!/usr/bin/env python3
"""Reconstruct pinned GPUI core plus reviewed GPUIO patches, never shared Cargo sources.

Normal builds use vendor/gpui. Refresh deliberately; Python 3.12+, curl and patch.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
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


def standalone_manifest(original, workspace, source):
    lines = []
    for line in original.splitlines():
        if line == "edition.workspace = true":
            line = f'edition = {json.dumps(workspace["package"]["edition"])}'
        elif line in ("[lints]", "workspace = true"):
            continue
        else:
            inherited = re.fullmatch(r"([\w-]+)(?:\.workspace = true| = (\{.*workspace = true.*\}))", line)
            if inherited:
                name, extra = inherited.groups()
                base = workspace["dependencies"][name]
                dependency = {"version": base} if isinstance(base, str) else dict(base)
                if "path" in dependency:
                    del dependency["path"]
                    dependency.update(git=source["url"], rev=source["commit"])
                if extra:
                    overrides = tomllib.loads("dependency = " + extra)["dependency"]
                    del overrides["workspace"]
                    features = dependency.get("features", []) + overrides.pop("features", [])
                    dependency.update(overrides)
                    if features:
                        dependency["features"] = list(dict.fromkeys(features))
                line = f"{name} = {toml_value(dependency)}"
        lines.append(line)
    lines.extend(["", "[lints.rust]", 'unexpected_cfgs = { level = "warn", check-cfg = ["cfg(rust_analyzer)"] }'])
    result = "\n".join(lines).rstrip() + "\n"
    if "workspace = true" in result:
        raise ValueError("unresolved workspace inheritance")
    tomllib.loads(result)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", type=Path, help="Local upstream archive, still hash-verified")
    parser.add_argument("--output", type=Path, default=ROOT / "vendor/gpui")
    args = parser.parse_args()
    source = json.loads((ROOT / "third_party/sources.json").read_text())["gpui"]
    if args.output.exists():
        raise SystemExit("Output exists; choose a new destination for reconstruction")
    patch = ROOT / "third_party/patches/gpui.patch"
    if digest(patch) != source["patch_sha256"]:
        raise SystemExit("GPUI patch checksum mismatch")
    with tempfile.TemporaryDirectory(prefix="gpuio-gpui-") as temporary:
        staging = Path(temporary)
        archive = args.archive or staging / "source.tar.gz"
        if not args.archive:
            subprocess.run(["curl", "--fail", "--location", "--silent", "--show-error", "--retry", "3",
                            "--output", str(archive), f"https://codeload.github.com/zed-industries/zed/tar.gz/{source['commit']}"], check=True)
        if digest(archive) != source["archive_sha256"]:
            raise SystemExit("GPUI archive checksum mismatch")
        prefix = "zed-" + source["commit"]
        with tarfile.open(archive) as bundle:
            members = [m for m in bundle.getmembers() if m.name in
                       (prefix + "/Cargo.toml", prefix + "/LICENSE-APACHE") or m.name.startswith(prefix + "/crates/gpui/")]
            bundle.extractall(staging, members=members, filter="data")
        upstream = staging / prefix
        crate = upstream / "crates/gpui"
        workspace = tomllib.loads((upstream / "Cargo.toml").read_text())["workspace"]
        original = (crate / "Cargo.toml").read_text()
        (crate / "Cargo.toml.upstream").write_text(original)
        (crate / "Cargo.toml").write_text(standalone_manifest(original, workspace, source))
        (crate / "LICENSE-APACHE").unlink(missing_ok=True)
        shutil.copy2(upstream / "LICENSE-APACHE", crate / "LICENSE-APACHE")
        subprocess.run([shutil.which("gpatch") or "patch", "--batch", "--forward", "-p1", "-i", str(patch)], cwd=crate, check=True)
        args.output.parent.mkdir(parents=True, exist_ok=True)
        shutil.copytree(crate, args.output)
    print(f"Reconstructed GPUI core at {source['commit']}")


if __name__ == "__main__":
    main()
