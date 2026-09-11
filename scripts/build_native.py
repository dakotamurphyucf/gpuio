#!/usr/bin/env python3
"""Dune action: build the pinned native archive and discover platform link flags."""

import json
import os
from pathlib import Path
import platform
import shlex
import shutil
import subprocess
import sys

root = Path(__file__).resolve().parent.parent
profile = sys.argv[1]
package = sys.argv[2] if len(sys.argv) > 2 else "gpuio-foundation"
if package not in {"gpuio-foundation", "gpuio-native"}:
    raise SystemExit("unsupported native package")
archive = "lib" + package.replace("-", "_") + ".a"
target = Path(os.environ.get("CARGO_TARGET_DIR", root / "target"))
env = os.environ.copy()
if platform.system() == "Darwin":
    env["MACOSX_DEPLOYMENT_TARGET"] = "14.4"
args = ["cargo", "rustc", "--manifest-path", str(root / "Cargo.toml"),
        "--package", package, "--lib", "--locked",
        "--target-dir", str(target), "-j", env.get("GPUIO_JOBS", "4")]
release = profile == "release"
if release:
    args.append("--release")
args.extend(["--", "--print", "native-static-libs"])
result = subprocess.run(args, env=env, text=True, stdout=subprocess.PIPE,
                        stderr=subprocess.STDOUT)
print(result.stdout, end="", flush=True)
result.check_returncode()
prefix = "note: native-static-libs: "
link_line = next((line.split(prefix, 1)[1] for line in result.stdout.splitlines()
                  if prefix in line), None)
if link_line is None:
    raise SystemExit("Cargo did not report native-static-libs; cannot produce linker flags")
# Dune invokes the OCaml linker from the build-context root, whereas this action
# runs in the example directory. Generate the archive's actual build path and
# place it before its system libraries (required by ELF --as-needed linking).
flags = ["-cclib", str(Path.cwd() / archive)]
if platform.system() == "Darwin":
    flags.extend(["-ccopt", "-mmacosx-version-min=14.4"])
for token in shlex.split(link_line):
    flags.extend(["-cclib", token])
c_flags = shlex.split(link_line)
if platform.system() == "Darwin":
    c_flags.insert(0, "-mmacosx-version-min=14.4")
if platform.system() == "Linux":
    c_flags.insert(0, "-Wl,--no-as-needed")
Path("c_link_flags.sexp").write_text("(" + " ".join(json.dumps(x) for x in c_flags) + ")\n")
Path("link_flags.sexp").write_text("(" + " ".join(json.dumps(x) for x in flags) + ")\n")
shutil.copyfile(target / ("release" if release else "debug") / archive,
                archive)
