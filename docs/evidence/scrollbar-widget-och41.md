# Standalone native scrollbar widget — OCH-41

Checkpoint: 2026-10-03, macOS arm64, working tree based on `83eb87e` on
`milestone-07-gallery-release`. This adds an actual GPUI element over an existing
native scroll handle. It does not yet attach the public Core configuration to a
View or replace production list/table bars. OCH-41 and milestone 07 remain open.

## Implementation

`scrollbar_input` reads current metrics from `Rc<dyn ScrollbarHandle>` on every
action, preserving the perpendicular offset and native clamping. Captured grips
survive content/viewport resize. List drag hooks balance on release, policy or
axis changes, lost geometry, close and drop. Optional layout viewport bounds
support adapters whose bar region differs from the handle's full viewport.

`scrollbar_widget` owns the handle controller, two focus handles and bounded
per-axis timing owners. It paints resolved tracks, borders, solid/gradient thumbs
and a focus outline. Actual painted translation and clipping govern pointer
acquisition during slide motion. A stable union of thumb state rectangles avoids
hover oscillation under narrowing styles without enlarging the drag target.

Pointer dragging and track clicks preserve existing editor focus. Explicit
Tab/accessibility focus reveals a hidden eligible range. Focused unmodified
axis arrows, PageUp/PageDown and Home/End share current geometry with AccessKit
Focus/Increment/Decrement/SetValue routes. Removed, disabled, inactive, clipped,
omitted or closed ranges retire interaction and timing. Removing pointer access
cancels capture/hover while retaining keyboard/accessibility access. Native
ListState tail-following remains owned by the existing list.

The pointer adapter delegates native range layout, keyboard, Tab and accessibility
registration to GPUI's Div, while supplying its own mouse listeners. It omits
Div's automatic mouse-focus listener: hidden bars and unused edge space must let
underlying content receive ordinary focus. Only an actual painted thumb/track
click consumes the pointer event. This needed no vendor change.

The caller must reconcile live visibility, disabled/modal and pointer policy,
pair begin/finish around its full deferred frame, and close owners while the
Window is available. Those production Host connections remain to be implemented.

## Verification

Commands use `GPUIO_JOBS=2 ./scripts/gpuio exec` and the repository environment:

| Command | Result |
| --- | --- |
| `cargo test --offline --locked -j2 -p gpuio-native --lib scrollbar_input` | Four handle-controller tests pass |
| `cargo test --offline --locked -j2 -p gpuio-native --features native-image-tests,native-canvas-tests --lib scrollbar_widget` | Eight element tests pass, including hidden-bar pointer passthrough |
| `cargo test --offline --locked -j2 -p gpuio-native --features native-image-tests,native-canvas-tests --lib` | 785 passed; two existing private-D-Bus skips |
| `cargo clippy --offline --locked -j2 -p gpuio-native --all-targets --features native-image-tests,native-canvas-tests -- -D warnings` | Pass after the pointer-passthrough fix and Tab/resize regressions |
| `cargo fmt --all -- --check` | Pass |
| `dune build -j2 examples/gallery/main.exe` | Pass; rebuilt the backend and linked the gallery without launching it |

The eight GPUI TestPlatform widget tests cover native Tab traversal and real ScrollHandle keyboard/AX
changes; captured dragging without stealing focus; omitted-owner cleanup;
slide-motion ghost hits and immediate removed-axis rejection; shrinking hover
styles; real ListState drag hooks, tail-following and window deactivation;
invalid numeric AX data, live input gates and close; hidden-range AX focus and
underlying-content pointer focus,
overflow removal and zero-height retirement; resize during capture and a final
mouseup coordinate arriving without a last move event.

These are actual GPUI element, input-route and accessibility-tree tests using
TestPlatform. They are not physical macOS keyboard/IME, VoiceOver, GPU pixels or
Linux desktop acceptance. No OS windows were opened. The gallery build only
checks linkage; the gallery does not yet exercise the new scrollbar widget.
The existing `block` dependency emits its known future-compatibility advisory.
Core/protocol contracts, dependency pins and vendor sources are unchanged in this
checkpoint; their prior foundation evidence remains applicable.

Required next work: checked attaching operation, transactional tree admission
and quotas, Core/Bonsai View reconciliation, production ordinary/list/tree/table
handle integration and teardown, public gallery controls and installed-consumer
validation. Host scope registration must include both native range focus parts
so modal and explicit Tab-order traversal remain correct; ordinary widget focus
registration alone does not prove that integration. Physical macOS/release gates
remain separate in OCH-17.
