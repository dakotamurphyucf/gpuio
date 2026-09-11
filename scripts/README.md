# Contributor tooling

`./scripts/gpuio` is the local/CI entry point. See docs/development.md for commands
and isolation guarantees. `vendor_bonsai.py` and `lock_opam.py` are deliberate
dependency-refresh tools. `build_native.py` is the Dune action that compiles Rust
and discovers platform linker flags. `ci_opam.py` installs pinned opam only inside
GitHub Actions; `ci_linux_smoke.sh` runs the Linux compositor smoke scenarios.
