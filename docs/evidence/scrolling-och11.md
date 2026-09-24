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
State-specific overflow axes are computed by the adapter but are not separately
exercised by this fixture. A partially consumed event does not forward overshoot.

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
