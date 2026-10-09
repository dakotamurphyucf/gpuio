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

`package_macos_reference.py` assembles already-built chat/gallery/Signal Studio
executables into local macOS qualification bundles and zip artifacts, with load-
command audits, supplied notices, provenance and optional ad-hoc signing. It never
installs/registers/opens the bundle. See [distribution](../docs/distribution.md);
assembly does not establish clean-machine, licensing or notarization acceptance.
`test_package_macos_reference.py` checks portable input admission without a desktop.

`collect_rust_notices.py` reads offline locked Cargo metadata, follows a conservative
normal/build dependency closure, and copies package-local license/notice texts with
identity and hash evidence. It never approves licenses or selects a dual-license
alternative. Missing texts, workspace declarations and additional OCaml/asset
review remain explicit. See [distribution](../docs/distribution.md).
`--supplemental third_party/notice-sources.json` adds explicitly attributed pinned
workspace texts, with drift checks and unchanged discovery issues kept visible.
`test_collect_rust_notices.py` checks graph selection, identity, byte preservation
and unresolved inputs without invoking Cargo or opening windows.

`collect_ocaml_notices.py` uses an explicit opam root/switch and optional repeated
`--vendor` roots to collect installed/doc/source notice bytes, package metadata and
project pins. It includes development tools, rejects installed-version drift, and
leaves missing texts and license review explicit. It never installs packages,
changes switches or claims exact linker membership. Its five portable tests cover
byte preservation, nested notices, symlinks, output safety, switch isolation and
concurrent installed-version changes; both foundation jobs run them. See
[OCaml notice evidence](../docs/evidence/ocaml-notices-och17.md).

`test_output_sink.py` drives `test/output/probe.exe` through inherited file, pipe,
null-device and private pseudo-terminal output. It tests exact bytes, typed errors,
nonblocking pipe cancellation and scheduler responsiveness without a GUI. See
[the output adapter evidence](../docs/evidence/output-sinks-och17.md).
