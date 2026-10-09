# Observe native input without taking over native editing

[observations_page.ml](observations_page.ml) and its
[interface](observations_page.mli) wrap one retained editor in an opt-in Input_region.
Read kinds/labels, Action/Observation, state/editor/lifecycle and subscriptions/views.
`B = Bonsai.Cont` constructs reactive models, `V` views, `Editor` controls one native
lease and `I = Input_region` typed asynchronous observations. Unlike the captured
[gesture preview](input_page.md), these subscriptions all use Observe policy.

After [setup](../../docs/development.md), from the root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/gallery/main.exe -j 2
./scripts/gpuio exec dune exec examples/gallery/main.exe
python3 scripts/test_gallery.py --section observations
```

Choose Input observations, focus open surface/editor, move/scroll/type, switch
capture/bubble and use floating Save idea. No content is saved externally and no
network data is involved. [README](README.md) qualifies native/platform evidence;
these commands were not run in this documentation review.

## Fixed-size observations and reactive reduction

`Observation.t` stores 13 fixed kind/count pairs, latest pointer/wheel/key strings
and hovered/focused flags. `apply` saturates the matching count at Int.max_value,
then records local position for click/down/up/move, hover/focus state, key-down name/
repeat flag or wheel delta preserving Pixels versus Lines. Key_up/outside-down
increment counts without replacing latest key/position text. No full event/payload
history is retained. This is application diagnostic state, not native input truth
or a text composition buffer.

`state_machine0` reduces Observe events or Reset; saved is a separate saturating
floating-action count. disabled/capture are toggles, starting enabled/capture.
`Editor.create` seeds Small ideas grow here in a `Single_line` placement. `let%arr`
combines state/controller/palette into a view; observation injection updates the
model later without editing editor text. Departure Reset clears the readout while
branch-local saved/preferences can remain; native editor unmounts and this page
has no persistent text mirror for reseeding subsequent leases.

## Capture orders observers; policy is still Observe

Config subscribes every kind with focus Tab. Click/auxiliary/enter/leave/outside-down/
focus/blur use required Bubble; raw down/up/move/key/scroll use selected Capture or
Bubble. No subscription sets Prevent_default/Stop_propagation, so all are Observe.
Native dispatch policy runs synchronously in Rust; OCaml callbacks cannot cancel
an earlier event. [Input_region](../../lib/core/input_region.mli) specifies that bound
native commands/editor actions run before raw key listeners even in Capture.
Key-up may arrive without a matching observed key-down, and key samples are not
committed text/IME. Use semantic commands/editor APIs for those operations.

The Tab region precedes its focusable children. Disabled pauses this observer/focus
but leaves child editor/control policy unchanged. Changing config retires queued
old observations while the keyed input region and child editor retain their state;
changing callback alone uses the latest accepted closure. Geometry is logical
pixels, and movement may coalesce without crossing lifecycle/policy boundaries.
See [View.`input_region`](../../lib/core/view.mli) and the
[editor contract](../../lib/eio/text_input.mli).

## Overlay hit-testing and a concrete trace

The region is 190 pixels high, with a relative outer container and an absolute
Save idea button placed as its sibling over the bottom-right area. `Pointer_occlusion`
Pointer blocks underlying pointer hitboxes for that ordinary action but permits
wheel input through. This is native hit-testing policy, not stop-propagation set
by a later callback. The button increments saved; it neither writes a file nor
changes editor contents. Border reflects observed hover unless disabled; focus
label explicitly says paused when observations are disabled.

Trace: click editor and type → native editor owns text/history/IME → eligible raw
observer samples asynchronously inject Observe → reducer updates fixed counts/latest
key → `let%arr` rebuilds diagnostic text while same editor remains placed. A handled
editor shortcut may never appear as key-down. Floating action blocks pointer hit
through while preserving its own semantic click; scroll can still reach the region.

For a new observed kind, update subscription/label/reducer and bound retained samples.
For actual input policy, declare native policy before dispatch, not in callback;
keep native editor ownership separate. Counts/logical samples are not exact hardware
input history, physical paint or proof every IME/key combination passed.
