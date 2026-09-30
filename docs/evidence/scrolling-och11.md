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

## Milestone-07: managed list inside an ordinary scroller

The public Bubble/Message transcript preview exposed another propagation gap.
An actual macOS one-pixel wheel event moved a message from Y=564 to Y=562 while
the list viewport itself moved from Y=560 to Y=559. Both list and parent consumed
the same event. This differs from the earlier row-retention and per-parse layout
defects; it is a concrete extra source of apparent jumping in nested layouts.

The focused `scroll_list_test.rs` reproduces the defect through real GPUI layout:
the list moved one pixel and the ancestor offset incorrectly became `(0, -1)`.
GPUI's list listener updates its position without stopping propagation. The GPUIO
Frame now records the logical position during capture and, after the native list
listener runs, consumes the event only if that position changed. It holds a weak
owner. No fork, protocol, OCaml callback or per-frame OCaml work was added.

The corrected native scroll binary passes its existing ordinary/two-axis cases
and `GPUIO_LIST_SCROLL_ROUTING_OK`:

- Precise, discrete and multiple same-frame events move the list without moving
  its ordinary ancestor.
- A nested child scroller consumes its own movement. At that child's boundary,
  the list consumes movement before the ancestor.
- At the list boundary an unconsumed event reaches the ancestor; horizontal-only
  events also reach a horizontally scrollable ancestor.
- Removing the list immediately releases its owner despite installed callbacks.

Local macOS arm64 commands:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native --features native-image-tests --test native_scroll --test native_list --no-run
```

The Cargo-reported `native_scroll` binary ran under a 120-second process-group
watchdog and exited zero. The pre-fix run failed the ancestor-offset assertion;
an earlier fixture attempt used an ordinary-element probe for the specialized
list and was corrected to use `ListState.viewport_bounds` before reproduction.
This is native layout with synthetic GPUI input. The broader `native_list` binary
also exits zero under a 180-second watchdog: exact anchors, editor/selection,
two 100,000-row traversals, bounded warm demand and resource release all pass.
Native/protocol all-target Clippy with `native-image-tests -D warnings` and 413
native unit tests pass (two ignored tests remain ignored).

The repaired repository gallery's `--section chat-list` OS-input walkthrough now
passes the original one-pixel sequence: sixteen forward/reverse events preserve
the outer viewport and warm control identities. It also records 82 retained-AX
stream samples with no downward rebound and a stationary outside draft, preserves
paused history during offscreen document appends, activates an out-of-flow reaction
inside reserved row space, restores the current document after jumping to latest,
and observes zero source bytes after page departure. These samples describe the
bounded fixture, not frame pacing or a general performance guarantee. A fresh
installed-library consumer also passes the same sequence, explicitly checks the
twelve-row bound and offscreen document absence, and records 85 stable stream
samples in its final run; see [chat composition](../design/chat-composition.md).
Consolidated hosted gates, Linux GUI and physical-trackpad acceptance remain separate.

## Milestone 07: focused-row overdraw

The Settings gallery exposed loss of warm neighbouring rows when a tall focused
row alone filled the viewport. Native demand separates `pinned` from `requested`:
the focused row disappeared from `requested`, and the overlap check consequently
discarded neighbouring rows despite an unchanged visible range. A later group
reveal used placeholder heights; replacing placeholders with real groups could
leave the destination below the viewport.

`list_view::Frame` now also checks overlap between the previous and current
visible ranges, in addition to overlap with previously requested rows. It retains
neighbours within the existing nearest-first active budget. It adds no pins,
retries, cache history, protocol fields or OCaml work per frame. All experimental
Settings-specific navigation changes were discarded.

The dedicated native fixture uses 700/650/240px rows, a 400px viewport and 400px
overscan. After focusing the first native button, it verifies six frames where
row 1 remains a pin and row 2 remains requested, then checks demand after blur.
This exercises actual GPUI layout and programmatic native focus; it is separate
from physical keyboard/IME acceptance.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native --features native-image-tests --test native_list --no-run
# Run the Cargo-reported native_list executable with --focused-overdraw.
```

The dedicated fixture passes locally on macOS 14.5 arm64. Restoring only the old
overlap predicate makes it fail with `requested=[]`, `pinned=[1]` and an unchanged
`0..1` visible range; the corrected predicate retains the neighbouring row. The
complete native list suite also exits zero under a 240-second process-group
watchdog, including selection/editor retention, exact anchors, two 100,000-row
traversals, bounded demand and this focused-row regression. Its stress phases
use direct layout/paint calls and remove roots between fixtures; the desktop
window can appear blank during those phases. This is not an appearance check.
All 421 native unit tests pass (two additional tests remain ignored), as do full
Dune build/expect/format checks and native all-target Clippy with
`--features native-image-tests -- -D warnings`.

The repository gallery and a fresh installed-library consumer also pass the expanded Settings walkthrough:
actual sidebar drag/keyboard resize, policy reset exclusions, 48-group focused
retention and blur/eviction/remount, native export and two-window OS input isolation.
See [Settings evidence](../design/settings-composition.md#public-gallery-integration-checkpoint--2026-09-30)
for commands and the separate accessibility limitations. The initial AX focus
mismatch is now repaired with [its own evidence](window-accessibility-och17.md). These
checks do not establish whole-application performance, VoiceOver or Linux desktop
acceptance.
