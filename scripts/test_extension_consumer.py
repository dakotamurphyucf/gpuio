#!/usr/bin/env python3
"""Build an independent consumer against staged installed public GPUIO libraries.

Uses the repository's isolated toolchain, never installs into an opam switch.
--run also exercises the real native application: a self-closing smoke example
or the gallery's macOS interaction driver, which closes and reaps its app.
"""
import argparse
import atexit
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import tempfile

root = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--run", action="store_true")
parser.add_argument("--workspace", type=Path)
parser.add_argument("--cleanup", action="store_true",
                    help="Remove the generated workspace on interpreter exit, including build failures")
parser.add_argument("--example", choices=["getting_started", "menus", "extension_consumer", "signal_studio", "gallery"], default="extension_consumer")
parser.add_argument("--gallery-section", default="all", help="Section passed to the macOS gallery acceptance driver with --example gallery --run")
args = parser.parse_args()
if args.cleanup and args.workspace:
    parser.error("--cleanup is limited to an automatically created temporary workspace")
if args.example in ("getting_started", "menus") and args.run:
    parser.error("Omit --run for this independent build. For menus, run test_native_popup_macos.py --binary with the resulting executable on macOS.")
if args.example == "gallery" and args.run and sys.platform != "darwin":
    parser.error("Gallery --run uses the macOS AX driver; omit --run for the required cross-platform consumer build")
workspace = (args.workspace or Path(tempfile.mkdtemp(prefix="gpuio-extension-consumer-"))).resolve()
if args.cleanup:
    atexit.register(shutil.rmtree, workspace)
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
example = root / "examples" / args.example
composed_backend = args.example not in ("getting_started", "menus")
if composed_backend:
    shutil.copytree(root / "examples/extension_package", consumer / "component")
    backend_example = root / "examples/extension_consumer" if args.example == "gallery" else example
    manifest = json.loads((backend_example / "native.json").read_text())
else:
    manifest = {}
for path in example.iterdir():
    if path.suffix in (".ml", ".mli") or path.name == "dune":
        shutil.copyfile(path, consumer / path.name)
for directory in ("model", "files", "notifications"):
    if (example / directory).exists():
        shutil.copytree(example / directory, consumer / directory)
if args.example == "gallery":
    shutil.copytree(root / "examples/charts/samples", consumer / "chart_samples")
if manifest.get("document_profiles"):
    shutil.copytree(root / "examples/document_profile_package", consumer / "document_profile")
(consumer / "dune-project").write_text("(lang dune 3.21)\n(name independent_extension_consumer)\n")
# Dune runs Cargo from this outside-checkout workspace. Preserve the repository
# pin there too, rather than falling back to the developer's global Rust default.
shutil.copyfile(root / "rust-toolchain.toml", consumer / "rust-toolchain.toml")
(consumer / ".ocamlformat").write_text((root / ".ocamlformat").read_text())
if composed_backend:
    consumer_manifest = {
        "library": manifest["library"], "gpuio": str(root),
        "components": [{"path": "component/rust", "factory": "factory"}],
    }
    if manifest.get("document_profiles"):
        consumer_manifest["document_profiles"] = [{"path": "document_profile/rust", "factory": "factory"}]
    (consumer / "native.json").write_text(json.dumps(consumer_manifest, indent=2) + "\n")
    run(["python3", str(root / "scripts/compose_backend.py"), str(consumer / "native.json"), str(consumer / "backend")])
    shutil.copyfile(backend_example / "backend/Cargo.lock", consumer / "backend/Cargo.lock")
# Cargo.lock stores package identities, not these relocated path spellings.
# The build uses --locked; a consumer check must never resolve newer versions.
env["OCAMLPATH"] = str(prefix / "lib") + (os.pathsep + env["OCAMLPATH"] if env.get("OCAMLPATH") else "")
targets = ["main.exe"]
if manifest.get("document_profiles"):
    targets.append("@document_profile/test/runtest")
run(["dune", "build", "--root", str(consumer), "-j", env["GPUIO_JOBS"], *targets])
if args.example == "gallery":
    # A fresh linked process validates both schemas without creating a window.
    subprocess.run([str(consumer / "_build/default/main.exe"), "--check-catalogs"],
                   check=True, env=env, timeout=30)
if args.run:
    executable = consumer / "_build/default/main.exe"
    if args.example == "gallery":
        arguments = [sys.executable, str(root / "scripts/test_gallery.py"),
                     "--executable", str(executable), "--section", args.gallery_section]
        driver = subprocess.Popen(arguments, env=env, start_new_session=True)
        try:
            status = driver.wait(timeout=900)
            if status:
                raise subprocess.CalledProcessError(status, arguments)
        except BaseException:
            # A killed driver cannot run its normal child-window cleanup.
            # Reap only this isolated test process group, including the app.
            try:
                os.killpg(driver.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            driver.wait()
            raise
    else:
        mode = "--self-test" if args.example == "signal_studio" else "--smoke"
        subprocess.run([str(executable), mode], check=True, env=env, timeout=75)
print(f"INDEPENDENT_EXTENSION_CONSUMER_PASS example={args.example} run={args.run} workspace={workspace}")
