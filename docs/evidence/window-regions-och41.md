# Custom window regions — OCH-41

Local implementation checkpoint, 2026-10-04, macOS arm64. Worktree based on
`83eb87e865c86717a8bc51b9db6fe1f379d909a9`, with substantial uncommitted changes;
that HEAD alone does not identify the tested sources. No dependency pins changed.
This is not whole-window-family or milestone acceptance.

## Implemented contract

- `Window.Chrome.Custom`: transparent title bar, application-owned macOS dragging,
  native traffic lights at (9,9) logical pixels. Standard/Hidden keep their setup.
- Core/Bonsai `View.title_bar`: ordinary styled content with native window gestures;
  minimum height of 34 px, left reservation of 80 px on windowed macOS and 12 px otherwise. Explicit
  styles override defaults; applications feed observed fullscreen changes.
- `View.with_window_region`: Title_bar, Exclude or named Resize edge/corner on a
  Container, with explicit removal. Native input never synchronously calls OCaml.
- Live node/config identity, native input eligibility and window activation guard
  every OS request. Ordinary button/editor descendants keep input; explicit
  exclusion supports custom descendants. Removal, hide/inert/pointer gating,
  reconfiguration, release and deactivation retire an armed move.
- Region state has a conservative 2,048-byte tree reservation. Decorative content
  admission rejects native window gestures. Invalid transactions remain atomic;
  clearing regions releases their reservation and retained native state.
- `examples/gallery/main.exe --custom-chrome` demonstrates public composition,
  fullscreen observation and `App.Window.request_close`, preserving the existing
  close-decision path. It handles missing capabilities before native handshake.

The unpublished epoch 3 appends Chrome tag 2 and Op122 without moving existing tags.
Both runtimes must come from the same checkout. Paired fixtures cover clear,
title-bar, exclusion, all eight edges and configured-open Custom; Rust also checks
unknown enum values, truncation and trailing data.

## Platform boundary

The pinned Zed backend `a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b` implements
`start_window_move` and title-bar double-click in `crates/gpui_macos/src/window.rs`,
but inherits the no-op platform `start_window_resize`. GPUIO therefore leaves
custom Resize regions inert on macOS (no misleading resize cursor or hit shield);
the native AppKit window border owns resizing. X11 and Wayland implement native
edge resize. Non-resizable windows also leave those regions inert. Linux window
menus require client decorations and the backend's menu-support flag.

The pinned Kit control-area min/max/close tags apply to Windows; macOS, X11 and
Wayland ignore their platform hit-test callback. Windows is outside project scope.
Automatic backend-supported Linux controls, client-frame tiling/insets/shadows
and the remaining WindowExt helper mapping remain open, not implicitly deferred.

## Validation

Six TestPlatform tests execute the production Session, retained renderer, hit
testing and lifecycle with a test-only OS-request recorder. They cover primary
drag/one-shot movement/double-click/release, enabled and disabled child buttons,
native editor focus/input, policy replacement before redraw, hide/inert/pointer
gates, activation/close cleanup, atomic invalid/decorative admission, reservation
release and resize policy for every edge. On macOS the latter checks inert custom
resize; it does not establish Linux backend behavior. No OS windows were opened.

The first full native regression run exposed default-stack overflow in the
existing nested sidebar fixture. Gesture setup now runs in nonrecursive
`node_presentation`, and nodes without a region skip the large gesture-builder
function entirely. Moving the setup alone was insufficient: a debug function's
stack frame exists even when its body returns immediately. The existing sidebar
fixture passes unchanged with the ordinary stack limit. No `RUST_MIN_STACK`
override, reduced fixture or relaxed check is used.

Commands use `GPUIO_JOBS=2 ./scripts/gpuio exec` unless indicated:

```sh
cargo test -p gpuio-protocol --test window --test window_region --offline --locked -j 2
cargo test -p gpuio-native --features native-image-tests,native-canvas-tests --lib window_regions::tests --offline --locked -j 2
cargo test -p gpuio-native --features native-image-tests,native-canvas-tests --lib public_sidebar_labels_retain_focus_paint_once_and_route_one_activation --offline --locked -j 2
cargo test -p gpuio-native --features native-image-tests,native-canvas-tests --lib --offline --locked -j 2
cargo clippy -p gpuio-native -p gpuio-protocol --all-targets --features native-image-tests,native-canvas-tests --offline --locked -j 2 -- -D warnings
dune build -j 2 examples/gallery/main.exe @runtest
```

Five paired protocol tests, six focused region tests, the existing sidebar test
and the full OCaml/gallery build passed at their checkpoints. Final expanded native regression: **883 passed, two existing macOS private-bus
skips**. Strict all-target native/protocol Clippy with both native test features
and `-D warnings` passed. The final-source full OCaml test suite, public gallery rebuild and official
formatting checks also passed.

Physical macOS movement/minimize/restoration/fullscreen/resize, keyboard/IME/AX and
visual review remain required. Linux compilation/unit/private-bus/consumer checks
remain required separately; full Linux desktop qualification stays OCH-47.
TestPlatform request recording is not OS execution, process/GPU memory or timing
evidence. OCH-41/OCH-17 remain open.


## Physical follow-up — 2026-10-05

The [public macOS window walkthrough](window-lifecycle-och41.md) now passes
standard/custom minimize and restoration, retained native selection/editing,
custom title-bar movement, fullscreen, AppKit border resize and double-click zoom.
It supersedes this checkpoint's unrun status for those specific operations; its
source/platform scope and remaining boundaries are explicit.
