#!/usr/bin/env python3
"""Refresh the transitive lock, preserving explicitly selected Linux-only pins."""
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parent.parent
subprocess.run(["opam", "lock", "--root", str(root / ".opam-root"),
                "--switch", "gpuio", str(root / "gpuio.opam")], check=True)
lock = root / "gpuio.opam.locked"
lines = [line for line in lock.read_text().splitlines()
         if not line.startswith('  "eio_linux" ') and not line.startswith('  "uring" ')]
text = "\n".join(lines) + "\n"
start = text.index("depends: [")
end = text.index("\n]", start)
# opam lock only sees installed platform dependencies. Eio's Linux backend adds
# uring; its remaining dependencies are already in the shared locked closure.
pins = '\n  "eio_linux" {= "1.3" & os = "linux"}\n  "uring" {= "2.7.0" & os = "linux"}'
lock.write_text(text[:end] + pins + text[end:])
