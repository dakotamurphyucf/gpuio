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

macOS is the functional development gate. Linux builds and unit tests remain
required, while the two Linux graphical CI steps are informational per the
owner's 2026-09-11 priority. Their real outcomes are preserved in logs and
`linux-gui-status.json`, even when the overall build job passes. Full Linux GUI
acceptance remains tracked in OCH-17; a successful build job is not proof of it.

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

Read `third_party/sources.json` and the accepted contracts first. To change Bonsai,
move the existing vendor snapshot aside, update the chosen commits/patches, and
run `python3 scripts/vendor_bonsai.py --record-archives`. Review the downloaded
source, hashes and diffs before committing. Normal reconstruction omits that flag
and verifies all hashes. Preserve upstream licenses and native-only Dune selection.

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
