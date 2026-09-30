# Contributor tooling

`./scripts/gpuio` is the local/CI entry point. See docs/development.md for commands
and isolation guarantees. `vendor_bonsai.py`, `vendor_gpui.py`, `vendor_gpui_base.py`
and `lock_opam.py` are deliberate
dependency-refresh tools. `build_native.py` is the Dune action that compiles Rust
and discovers platform linker flags. `ci_opam.py` installs pinned opam only inside
GitHub Actions; `ci_linux_smoke.sh` runs the Linux compositor smoke scenarios.

`test_file_dialog_read.py --driver <native_file_dialog executable>` runs the
macOS public example and drives only that child PID's picker via the native test
harness. Build the example/harness first; local accessibility access is required.
It verifies actual selection through the bridge followed by explicit Eio file
I/O, and cleans up its child on failure. See
`docs/evidence/native-file-dialogs-och11.md` for the commands.

`test_agent_chat.py` launches the agent-workspace example and targets only its
child PID through macOS AX/keyboard APIs. Build the example first and provide
Accessibility access. It validates real Send/Return, picker/Eio attachment,
retained tabs, independent windows, themes/palette and OS close decisions. It
requires a post-`App.run` marker and reaps its child on every exit path.

`test_extension_consumer.py --example gallery` stages the public OCaml packages
under a fresh prefix and builds a separate Component Studio consumer with the
locked composed Rust backend. It also supports `extension_consumer` and
`signal_studio`. No opam switch is changed. Gallery builds are required on both
CI platforms. On macOS, `--run --gallery-section documents` exercises the
independent gallery's document/diff UI; omit the section option to run all pages.
`test_gallery.py --executable PATH --section documents` can drive an existing
consumer binary without rebuilding it. `test_signal_studio.py --executable PATH`
likewise runs the complete Signal Studio AppKit walkthrough against an independent
consumer. Full Linux desktop behavior remains OCH-47.
