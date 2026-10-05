# Development environment

Use opam 2.3.0+, rustup, Git, curl, Python 3.12+ and pkg-config. `bootstrap`
creates `.opam-root/gpuio` in this checkout, installs the locked OCaml packages,
and installs the pinned Rust toolchain/components. It never selects a global
opam switch or Rust default and does not edit shell startup files.

On macOS, install GNU patch (`brew install gpatch`), Xcode/Command Line Tools and
select an SDK supporting macOS 14.4+. GNU patch is required for opam's upstream
package patches; bootstrap checks for it before installing anything. The initial
native deployment target is macOS 14.4; local validation uses
arm64. On Debian/Ubuntu Linux install a C/C++ toolchain, make, cmake, clang,
pkg-config, fontconfig, FreeType, OpenSSL, Wayland, X11/XCB, xkbcommon and Vulkan
development/runtime packages. The exact Ubuntu CI package list is versioned in
`.github/workflows/foundation.yml`. Native execution requires a working desktop
display and Vulkan-capable driver; CI uses software rendering for smoke coverage.

The foundation enables each Linux backend on both `gpui` and `gpui_platform`.
At our pinned revision, `gpui_platform/x11` alone compiles the X11 client but does
not enable `gpui/x11` compositor detection; that silently selects the headless
backend when only `DISPLAY` is set. Graphical CI asserts the selected backend.
X11 runs under Xvfb/Openbox; Wayland uses Weston nested on Xvfb to supply an input
seat. Bare headless Weston does not supply the seat GPUI requires. These fixtures
exercise native backend windows with software rendering, not physical input/GPU
hardware. Set `GPUIO_NATIVE_LOG=1` for native diagnostic logs.

macOS is the functional development and milestone 07 release gate. Linux builds,
unit tests, private-bus checks and independent consumer builds remain required;
the two Linux graphical CI steps stay informational. Their real outcomes are
preserved in logs and `linux-gui-status.json`, even when the overall job passes.
Full Linux desktop qualification is deferred to OCH-47 in milestone 07b and does
not block the macOS-first release or further feature development. A successful
build job is not proof of desktop acceptance. See the current
[platform release policy](platform-release-policy.md).

```sh
./scripts/gpuio bootstrap
./scripts/gpuio doctor
./scripts/gpuio build
./scripts/gpuio test
./scripts/gpuio check-fmt
./scripts/gpuio lint
./scripts/gpuio smoke --self-test
./scripts/gpuio smoke --two-windows
```

`smoke` without an option opens the interactive Bonsai example. Test and lint
commands do not open native windows. `GPUIO_JOBS=2` lowers compile concurrency.
Use `./scripts/gpuio fmt` to format; it never promotes expect-test output. Review
expectation differences before deliberately running `./scripts/gpuio exec dune
promote`. The development commands are also CI commands.

The `test/dune` environment explicitly enables first-party inline/expect tests
in every profile, including `dune runtest --profile release`. Keep that setting:
Dune otherwise disables inline tests in release builds, so a successful command
can leave those suites unexecuted. The override is scoped to `test/`, not the
production libraries or vendored projects.

`./scripts/gpuio exec COMMAND...` runs any editor/build tool with the selected
environment. For example `./scripts/gpuio exec ocamllsp` and
`./scripts/gpuio exec ocamlformat --version`. OCaml LSP 1.23.1 uses Dune's generated
Merlin configuration. Configure your editor's OCaml sandbox to invoke the wrapper
and the pinned formatter; rust-analyzer is supplied by Rust 1.97.1. Run an initial
build before checking navigation across vendored modules. The [OCaml Platform
extension](https://github.com/ocamllabs/vscode-ocaml-platform) supports custom
sandbox commands; select Custom and use `./scripts/gpuio exec $prog $args` from
the repository root. Both language servers have been exercised through hover and
go-to-definition requests, including Rust navigation into pinned GPUI sources;
evidence is under `docs/evidence`. Select these tools in your editor rather than
letting it use a different project's switch.

Each checkout/worktree has separate `.opam-root`, `_build`, `target` and `scratch`
directories. Cargo's immutable download caches may be shared. Do not run two Dune
builds concurrently in one checkout. `./scripts/gpuio exec dune clean` removes
OCaml/native build outputs in `_build`; `cargo clean` removes the root Cargo target
cache. Neither operation changes another checkout. Re-run bootstrap when the lock
changes. Do not copy an opam switch between different absolute paths.

Every agent keeps a separate `scratch/agents/<unique-agent-or-session-id>/` with
per-ticket notepads and a short index. Scratch is ignored by Git and excluded from
Dune; recreate it after cloning. Keep durable decisions and completion evidence
in versioned docs and Linear.

## Dependency refresh

Read `third_party/sources.json` and the accepted contracts first. Verify the Bonsai
family with `python3 scripts/vendor_bonsai.py --output FRESH_DIRECTORY`; add
`--archive-dir ARCHIVES` for offline inputs named `<package>.tar.gz`. Normal
reconstruction verifies all hashes and leaves the live vendor tree alone. To
intentionally change Bonsai pins, update the chosen commits/patches, reconstruct
into a fresh output with `--record-archives`, and review the new sources, hashes
and diffs together before replacing the affected vendor snapshot. Preserve upstream
licenses and native-only Dune selection. A `--package` subset is a scoped maintenance
check, not evidence that the complete family reconstructs.

GPUI core is vendored at the existing Zed revision with hidden-press cleanup
and a read-only opacity accessor; see [its adaptation contract](design/gpui-core-adaptation.md).
`python3 scripts/vendor_gpui.py --output <new-directory>` reconstructs it from the
hash-verified upstream archive and patch. `--archive <path>` uses a local copy of
the same archive. The command refuses to overwrite a destination. Compare the
reconstruction with `vendor/gpui`; normal builds use the committed snapshot.
Keep the root and generated application Cargo patches/lockfiles and Dune source
dependencies aligned. Never patch a shared Cargo checkout.

For opam, intentionally update `gpuio.opam`, resolve in the isolated root, and run
`python3 scripts/lock_opam.py`. This pins the transitive closure and preserves the
selected Linux-only Eio/uring pins, which cannot be installed on macOS. Both
platform builds must validate the result against the pinned opam repository.
Record `opam list` from the actual environment, not a nearby unrelated switch.

For Rust, edit exact git revisions/toolchain inputs and deliberately refresh
Cargo.lock. Review source and transitive changes; run both language fixture tests,
Bonsai lifecycle tests, native window/input and two-window smoke on both platforms.
The lock generated here is the foundation's resolved closure, not a claim that
every transitive version equals the historical spike's lock.

The codec test uses Dune's `preserve_file_kind` sandbox requirement so fixture
symlinks cannot escape Eio's current-directory capability. See [Dune dependency
specification](https://dune.readthedocs.io/en/stable/concepts/dependency-spec.html).

## Production bridge development

Build `./scripts/gpuio exec dune build examples/bridge/main.exe`, then run
`_build/default/examples/bridge/main.exe`. The self-test opens two real windows
and closes them automatically. Its intentional dispose-while-running probe logs
a Rust panic that must be caught; success ends with `PRODUCTION_BRIDGE_PASS`.
`gpuio.native` currently supports native OCaml executables with a statically
linked Rust archive, not bytecode/toplevel loading. The public view/runtime layer is implemented; start with the
[application guide](getting-started.md) and its compiled starter. See [bridge contract](design/bridge-v1.md).

## Typed UI development

Run `./scripts/gpuio exec dune exec examples/view_api/main.exe` for the interactive
typed counter/theme/selection example; add `-- --self-test` for 20 automated
acknowledged commits. Reusable components and a compiled Bonsai.Cont example are
in that directory. See [typed UI contract](design/typed-ui.md).

For actual native input/layout checks, run
`./scripts/gpuio exec cargo test --locked -p gpuio-native --features native-tests --test native_ui`.
This opens and activates a window, injects GPUI input and briefly exercises the
clipboard (restoring its previous contents). Do not interact with that window
during the short test. The feature enables paint probes only in this test build.
Ordinary `test` remains headless. CI requires these graphical checks on macOS;
Linux graphical results remain informational, while builds and pure tests are required.

For real Japanese OS composition and candidate-window acceptance, build the
gallery and run `python3 scripts/test_macos_ime.py --output scratch/ime-001`
on an otherwise idle desktop. It temporarily selects the installed Japanese input
method and restores the original keyboard settings. See [IME evidence and
recovery](evidence/macos-ime-och17.md) for prerequisites, scope and recovery after
an uncatchable interruption. This test captures only its own candidate window.

The macOS GUI fixtures include 1360×820 window-resize cases. CI therefore checks
for at least a 1440×1000-point desktop with 1400×900 usable points before building.
`scripts/ci_macos_display.swift --apply` selects an available mode for the CI login
session only, preferring the existing pixel density; it refuses changes outside
GitHub Actions. A separate `--check` process verifies the resulting desktop and
records full/usable bounds and modes in the uploaded logs. If no suitable mode is
available, this prerequisite fails explicitly instead of running cropped fixtures.
No test assertions or required outcomes are skipped. Local `--check` is read-only:

```sh
xcrun swift scripts/ci_macos_display.swift --check
```

The helper uses Apple's [display-configuration API](https://developer.apple.com/documentation/coregraphics/cgcompletedisplayconfiguration(_:_:))
with `kCGConfigureForSession`; it does not save permanent display preferences.
This test-desktop requirement is not a minimum window size imposed by the library.


Once native test compilation and the independent extension-consumer build pass,
CI runs subsequent independent macOS GUI families even if an earlier family
fails. Each failure still fails the required job; no macOS check is informational.
Cancellation stops subsequent families, and missing build prerequisites skip them.
Component checks and agent-chat walkthroughs each have their own step so one
failure does not hide later independent scenarios. Other multi-command families
retain their existing fail-fast behavior. Inspect all
failed steps and their uploaded logs before fixing the next batch of CI issues.

## CI storage

Hosted CI caches the isolated opam switch and downloaded Cargo sources, then
builds Rust outputs within the job with incremental compilation disabled. The
cache deliberately excludes `target/` to avoid carrying obsolete feature/source
variants between runs. Migration from the old cache discards only the current
GitHub workspace's restored `target/`, behind explicit CI/path/symlink guards.
Local build caches and incremental settings are unchanged.

Independent-consumer CI checks use `--cleanup` to remove their generated temporary
workspaces on interpreter exit, including ordinary build failures. Local runs keep
their workspace by default. For persistent investigation use `--workspace PATH`;
combining it with `--cleanup` is rejected. An uncatchable process termination can
leave a temporary workspace until the ephemeral CI machine is retired.

GPUI's macOS platform crate is also vendored at the same Zed revision to retire
the accessibility adapter before native-window teardown. Reconstruct it with
`python3 scripts/vendor_gpui.py --crate gpui_macos --output <new-directory>`;
see [ownership and qualification](design/gpui-macos-adaptation.md).


The shared Apple Metal renderer is pinned alongside it at `vendor/gpui-apple`.
Its default-off presentation diagnostics use the existing renderer rather than
swizzling OS classes. Reconstruct with
`python3 scripts/vendor_gpui.py --crate gpui_apple --output <new-directory>`;
see [the adaptation and ownership contract](design/gpui-apple-adaptation.md).
