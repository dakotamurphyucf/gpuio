#!/usr/bin/env python3
"""Install the pinned opam binary into an ephemeral CI directory, verifying SHA256."""
import hashlib
import os
from pathlib import Path
import platform
import subprocess

if os.environ.get("GITHUB_ACTIONS") != "true":
    raise SystemExit("This installer is only for GitHub Actions; install opam normally locally")
architecture = {"aarch64": "arm64", "arm64": "arm64", "x86_64": "x86_64"}[platform.machine()]
system = {"Darwin": "macos", "Linux": "linux"}[platform.system()]
name = f"{architecture}-{system}"
hashes = {
    "arm64-macos": "b35efa25668996f8df807b57b571aaccb5a6f78395cbefd32a3860df6d3eef39",
    "x86_64-macos": "9cf6031b599c862f0a0886f2b0354bb80cd8cad21a349c7894e55fac54209c83",
    "x86_64-linux": "324e78e3f33efeba279aacf9f9610cfec7b2df7d7e0e1640f75f09de85f96cc9",
}
directory = Path(os.environ["RUNNER_TEMP"]) / "gpuio-tools"
directory.mkdir(exist_ok=True)
binary = directory / "opam"
subprocess.run(["curl", "--fail", "--location", "--silent", "--show-error", "--retry", "3",
                "--output", str(binary),
                f"https://github.com/ocaml/opam/releases/download/2.3.0/opam-2.3.0-{name}"], check=True)
if hashlib.sha256(binary.read_bytes()).hexdigest() != hashes[name]:
    raise SystemExit("opam binary checksum mismatch")
binary.chmod(0o755)
with open(os.environ["GITHUB_PATH"], "a") as output:
    output.write(str(directory) + "\n")
