# Window presentation and controls — OCH-41

Local macOS arm64 checkpoint, 2026-10-04, on the uncommitted worktree based on
`83eb87e865c86717a8bc51b9db6fe1f379d909a9`. No dependency or vendor revisions changed.
This extends [custom regions](window-regions-och41.md); automatic client-frame
rendering and physical platform acceptance remain open.

## Behavior

`Window.Snapshot.presentation` reports actual server/client decorations, four
tiled edges, backend-supported controls and native resizability policy. Minimize
and maximize support also honor the native window's minimizable/resizable flags.
Normal window observers update the presentation cache; rendering compares the
small metadata value to catch changes without bounds/activation changes. Changes
use the existing per-window coalesced observation lane. Unchanged frames create
no new observations or recurring task.

Native minimize/zoom/fullscreen requests check current support before invoking
the platform and return `Unsupported` when unavailable. A stale UI observation
does not bypass that guard. Native title-bar double-click on Linux also respects
the maximize/resizability policy; macOS retains its platform double-click action.

Core/Bonsai `View.window_controls` composes ordinary keyed buttons. It hides
controls on macOS and with server decorations, shows supported client Minimize
and Maximize/Restore controls, and always offers Close for client decorations.
Callers supply actions and styles; the gallery uses `App.Window.request_close`.
No separate native button/focus owner or force-close path was added. Capability
changes remove stale callbacks; changing Maximize to Restore retains identity.
The gallery disables unsupported fullscreen and uses observed presentation for
its custom-chrome control row.

The unpublished epoch-3 snapshot appends Presentation after Document. Existing
enum tags stay fixed. Both halves must be rebuilt together. The OCaml decoder
rejects malformed enum/Boolean data and inconsistent maximize-without-resizability
metadata. The conservative native event-byte reservation still covers all fixed
fields; dynamic title/document paths retain their existing separate charges.

## Local evidence

Core expect tests exercise all three backends with server/client decorations,
unsupported actions, Maximize/Restore identity and removed-handler rejection while
Close retains its callback. Independent OCaml/Rust fixtures cover server and
client observations, all tiling/control fields and represented-document bytes.

The native TestPlatform test reads real GPUI test-window policy and calls the
production command handler on a non-resizable, non-minimizable window. Its
unsupported operations never reach TestWindow's unimplemented OS methods. The
same test verifies ordinary observations, zero events on unchanged draws and
coalescing after a cached prior presentation differs from the current window.
That cached-state fixture exercises the publisher; it does not simulate a real
Wayland compositor changing capabilities.

Commands use `GPUIO_JOBS=2 ./scripts/gpuio exec`:

```sh
cargo test -p gpuio-protocol --test window --offline --locked -j 2
cargo test -p gpuio-native --features native-image-tests,native-canvas-tests --lib window_host::tests --offline --locked -j 2
cargo test -p gpuio-native --features native-image-tests,native-canvas-tests --lib --offline --locked -j 2
cargo clippy -p gpuio-native -p gpuio-protocol --all-targets --features native-image-tests,native-canvas-tests --offline --locked -j 2 -- -D warnings
dune runtest -j 2 test/view_api test/protocol
dune build -j 2 examples/gallery/main.exe @runtest
```

Passed: five protocol tests, the focused native policy test, Core/protocol expect
tests and **884 native library tests with two existing macOS private-bus skips**.
Strict all-target native/protocol Clippy passed after removing an unnecessary
mutable test-context reference. The final full OCaml test suite, public gallery
rebuild and official formatting checks also passed.

No OS windows were opened. Physical Mac acceptance and Linux build/unit/private-bus/
consumer gates remain separate; Linux graphical qualification remains OCH-47.
The automatic frame still needs stable native inset handling across tiling and
content-bound integration for popups/sheets. A padded root alone is insufficient.
OCH-41 and OCH-17 remain open.


## Physical follow-up — 2026-10-05

The [public macOS window walkthrough](window-lifecycle-och41.md) now passes
standard/custom minimize and restoration, retained native selection/editing,
custom title-bar movement, fullscreen, AppKit border resize and double-click zoom.
It supersedes this checkpoint's unrun status for those specific operations; its
source/platform scope and remaining boundaries are explicit.
