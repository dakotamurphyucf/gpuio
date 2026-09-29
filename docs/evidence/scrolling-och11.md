# OCH-11 native scrolling — local evidence

The production View's native scrolling adapter is implemented and validated on
macOS arm64. This checkpoint does not claim hosted CI, Linux GUI acceptance or merge.

Before the correction, one downward wheel event moved both a nested transcript
and its outer container by 60 logical pixels. The pinned GPUI default Div listener
updated offsets without stopping propagation. Its default axis remapping also
did not express the desired horizontal-code/vertical-transcript policy.

The adapter attaches a viewport hitbox whose bubble listener runs after nested
content and before the parent's default scroll listener. It clamps movement on
the currently computed permitted axes, stops consumed events, and leaves boundary
events available to ancestors. Native GPUI owns layout, offsets and gesture filtering.
Host state is keyed by retained node identity; frame wrappers and callbacks hold
weak references. Existing semantic, pointer, drop-target and rounded-image wrappers
continue to compose with scrolling.

`native_scroll` opens a focus:false production GPUI window and dispatches synthetic
native wheel/key events. It verifies:

- Transcript movement without simultaneous ancestor movement or tree revision changes.
- Same-node text/style updates retaining both offset and native owner identity.
- Horizontal code movement on X, vertical movement reaching the transcript on Y,
  and events at the transcript boundary reaching the outer container.
- The real multiline composer moving its own scroll offset without moving the app.
- A 100-choice Select popup scrolling and shielding the application at its boundary.
- A scroller inside a modal moving, while wheel input on the backdrop is shielded.
- Immediate owner release when the final scroll declaration is removed and when
  nodes are unmounted; native editor/scroll maps become empty after disposal.

The test passes with marker `GPUIO_NATIVE_SCROLL_OK`. Native image/SVG/icon/button
and pointer regression binaries also pass after the wrapper integration. These
are actual native-window checks with synthetic GPUI input; they do not establish
physical trackpad momentum, OS keyboard automation or Linux graphical behavior.
The original fixture did not separately exercise state-specific overflow axes;
the milestone-07 section below adds hovered-axis coverage. A partially consumed
event does not forward overshoot.

Commands used locally:

```sh
./scripts/gpuio exec dune build @all @runtest @fmt
./scripts/gpuio exec cargo test --locked --workspace
./scripts/gpuio exec cargo test --locked -p gpuio-native --features native-image-tests --test native_scroll --test native_image_views --test native_pointer --no-run
./scripts/gpuio exec cargo clippy --locked -p gpuio-native --features native-image-tests --all-targets -- -D warnings
```

The three reported native binaries run sequentially under 90-second subprocess
timeouts. Logs are `scroll-*.log` in the implementing agent's ignored scratch
directory. The checked-in workflow builds the scroll test on both platforms and
runs it as a required macOS check, with informational X11/Wayland runs. Hosted
execution remains deferred until the consolidated OCH-11 delivery.

## Milestone-05 regression: visible frame-based test

The consolidated scroll test waited without producing its completion marker
while its `focus: false` window was behind the foreground application. The test
now uses a focused window and activates its application before awaiting real
layout frames, matching the owner's local GUI-test policy. It completes the
existing nested-axis, boundary routing, composer, popup/modal and disposal
assertions. Input remains synthetic GPUI dispatch; activation does not turn
this into physical-trackpad validation. Production window activation is unchanged.
The focused command is recorded in the [picker regression evidence](native-file-dialogs-och11.md#milestone-05-regression-saved-picker-view-mode).

## Milestone-07 parity: two-axis containers

The pinned GPUIX host explicitly enables concurrent scrolling when both axes
are scrollable (`renderer.rs` at
`18e695ed0ee8121a7793413ca795e08eda2a13df`, now in the source catalog).
The GPUIO ordinary-container adapter previously filtered every precise gesture
to its dominant axis, then discarded one component of any remaining diagonal
sample. A new real-window regression reproduced the gap: a `(-70, -30)` sample
moved the two-axis viewport to `(-70, 0)`.

The production adapter now keeps both components when both computed overflow
axes are Scroll. Single-axis containers retain GPUI gesture filtering and no
axis remapping. A change of effective axes clears the obsolete gesture lock;
layout, clamping, native offset ownership, weak paint callbacks and boundary
propagation remain in the existing adapter. This adds no protocol field or fork
change. Managed lists and native editor/control scroll policies are separate.

`scroll_two_axis_test.rs`, called by the existing `native_scroll` binary after
its original fixture, verifies through production layout and input dispatch:

- Precise diagonals, gesture continuation and reverse movement preserve both
  requested components; discrete line-wheel diagonals preserve their ratio.
- Consumed inner movement does not move the outer container.
- A clamped X axis still permits Y movement; when both axes are at their limit,
  unconsumed input reaches the outer container.
- A hovered X-hidden refinement permits only Y, and restoring both axes enables
  diagonal motion while retaining the same native owner and Y offset.
- Unmount releases the weak owner immediately and clears the scroll registry.

Local macOS on 2026-09-29: the pre-fix assertion failed with actual `(-70, 0)`
versus expected `(-70, -30)`. After the repair the full native binary exited 0
with both `GPUIO_NATIVE_SCROLL_OK` and `GPUIO_TWO_AXIS_SCROLL_OK`. Thus the
existing transcript/code-axis, composer, select popup, modal shielding and
owner-disposal checks also passed. The first attempted new fixture was rejected
for skipping fresh node slots; correcting it to contiguous slots preceded the
actual behavior reproduction and production repair.

Build: `./scripts/gpuio exec cargo test -p gpuio-native --test native_scroll
--features native-image-tests --locked -j2 --no-run`. The Cargo-reported binary
was run with a 90-second process-group watchdog and reaped on completion.
No test process remained. This is real GPUI layout with synthetic native input,
not physical trackpad momentum, a public-gallery run or Linux desktop acceptance.

Strict native/protocol all-target Clippy with `native-image-tests` and
`-D warnings`, Rust/OCaml formatting, catalog verification and diff checks also
passed. Hosted CI remains part of consolidated milestone validation.
