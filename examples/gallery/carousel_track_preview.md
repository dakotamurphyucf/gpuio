# Measured carousel-track walkthrough

Read [carousel_track_preview.ml](carousel_track_preview.ml) and its
[interface](carousel_track_preview.mli). [journeys_page.ml](journeys_page.ml) mounts it on
**Carousels & journeys**. `B` aliases `Bonsai.Cont`, `V` `Gpuio_bonsai.View`, `C` Carousel_track
and `Editor` Gpuio_eio.Text_input. `graph` hosts reactive state/controller computations;
`let%arr` derives the current view rather than running an animation loop.

`Card.t` has Capture/Explore/Refine/Publish with stable IDs and extents 280/240/220/260.
`Action.t` separates controlled requests from axis/loop/auto/disabled/order changes. Pure
`apply` evolves [`Carousel_track`](../../lib/core/carousel_track.mli) through validated
operations. `B.state_machine0` returns current model and dispatch effect constructor, reducing
against the latest model. Initial selection is first, horizontal, enabled, without looping/auto
advancement; compact starts false and animated true.

A native Next/track interaction sends a `C.Request`; its effect dispatches Request, updates the
latest measured model and makes `let%arr` derive selection/view changes for native movement.
Merely constructing dispatch does not run it. Layout requests supply native measurements rather
than a second selection owner. Stale lineages/geometry/proposals are rejected; model updates
preserve revision/lineage, while fresh create belongs with a fresh view key.

`V.carousel_track` mounts measured-idea-track with bounded viewport and variable-sized cards.
Capture contains one retained native editor; other cards contain explicit `Select` effects.
Reversing stable IDs or changing axis retains surviving content and invalidates old
measurements. Native motion is 200ms by default, or immediate; reduced motion/inactive
windows/unavailable geometry settle. Toggle Auto supplies a four-second interval: advancement is
proposed natively and pauses during interaction, not an OCaml timer. Short tracks can use
immediate loop-boundary jumps rather than seamless wrapping.

Type Capture’s draft, reverse cards/change axis, select neighbors and enable automatic movement.
Selection readout is controlled model state; the draft is independent. GPUIO owns
measurement/input/motion, Bonsai owns the evolving model/options and the adapter owns editor
lease. No asset scope/background worker exists. The public contract still leaves full
focus/accessibility qualification unfinished. Adapt with stable IDs, latest-model request
handling and bounded geometry; do not claim acceptance from rendering or treat auto advancement
as application work.

## Run and review

From the repository root:

```sh./scripts/gpuio build examples/gallery/main.exe./_build/default/examples/gallery/main.exe
```

These repository-wrapper commands are instructions, not validation performed for this
documentation change. There is no standalone executable or self-test for this component.
Compilation does not establish native focus, keyboard, animation or platform acceptance. See
[gallery instructions](README.md) and [development](../../docs/development.md).
