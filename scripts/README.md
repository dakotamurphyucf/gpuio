# Contributor tooling

`./scripts/gpuio` is the local/CI entry point. See docs/development.md for commands
and isolation guarantees. `vendor_bonsai.py`, `vendor_gpui_base.py` and `lock_opam.py` are deliberate
dependency-refresh tools. `build_native.py` is the Dune action that compiles Rust
and discovers platform linker flags. `ci_opam.py` installs pinned opam only inside
GitHub Actions; `ci_linux_smoke.sh` runs the Linux compositor smoke scenarios.

`test_file_dialog_read.py --driver <native_file_dialog executable>` runs the
macOS public example and drives only that child PID's picker via the native test
harness. Build the example/harness first; local accessibility access is required.
It verifies actual selection through the bridge followed by explicit Eio file
I/O, and cleans up its child on failure. See
`docs/evidence/native-file-dialogs-och11.md` for the commands.
