#!/usr/bin/env python3
"""Build an independent consumer against staged installed public GPUIO libraries.

Uses the repository's isolated toolchain, never installs into an opam switch.
--run also exercises the real native application, which closes itself after paint.
"""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

root = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--run", action="store_true")
parser.add_argument("--workspace", type=Path)
args = parser.parse_args()
workspace = (args.workspace or Path(tempfile.mkdtemp(prefix="gpuio-extension-consumer-"))).resolve()
workspace.mkdir(parents=True, exist_ok=True)
consumer = workspace / "consumer"
if consumer.exists():
    raise SystemExit(f"Consumer already exists; choose a fresh workspace: {consumer}")
consumer.mkdir()
prefix = workspace / "installed"
env = os.environ.copy()
env["GPUIO_JOBS"] = env.get("GPUIO_JOBS", "2")
command = [str(root / "scripts/gpuio"), "exec"]


def run(arguments, **kwargs):
    subprocess.run(command + arguments, cwd=root, env=env, check=True, **kwargs)


# Vendored projects are excluded from the root @install alias. Explicitly stage
# their pinned native packages as public dependencies, without touching a switch.
packages = sorted(path.stem for path in (root / "vendor").glob("*/*.opam"))
install_targets = [str(path.relative_to(root).with_suffix(".install")) for path in (root / "vendor").glob("*/*.opam")]
run(["dune", "build", "-j", env["GPUIO_JOBS"], "@install", *install_targets])
run(["dune", "install", "--prefix", str(prefix), "gpuio", *packages])
shutil.copytree(root / "examples/extension_package", consumer / "component")
for name in ("main.ml", "dune"):
    shutil.copyfile(root / "examples/extension_consumer" / name, consumer / name)
(consumer / "dune-project").write_text("(lang dune 3.21)\n(name independent_extension_consumer)\n")
(consumer / ".ocamlformat").write_text((root / ".ocamlformat").read_text())
(consumer / "native.json").write_text(json.dumps({
    "library": "gpuio_counter_backend", "gpuio": str(root),
    "components": [{"path": "component/rust", "factory": "factory"}],
}, indent=2) + "\n")
run(["python3", str(root / "scripts/compose_backend.py"), str(consumer / "native.json"), str(consumer / "backend")])
shutil.copyfile(root / "examples/extension_consumer/backend/Cargo.lock", consumer / "backend/Cargo.lock")
# Cargo.lock stores package identities, not these relocated path spellings.
# The build uses --locked; a consumer check must never resolve newer versions.
env["OCAMLPATH"] = str(prefix / "lib") + (os.pathsep + env["OCAMLPATH"] if env.get("OCAMLPATH") else "")
run(["dune", "build", "--root", str(consumer), "-j", env["GPUIO_JOBS"], "main.exe"])
if args.run:
    subprocess.run([str(consumer / "_build/default/main.exe"), "--smoke"], check=True, env=env, timeout=45)
print(f"INDEPENDENT_EXTENSION_CONSUMER_PASS run={args.run} workspace={workspace}")
