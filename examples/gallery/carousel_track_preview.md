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

`Toggle_auto` changes the model's optional `Auto_advance` value. Once enabled,
Rust schedules one deadline after eligible settled paint. Focus or hover within
the track/control group, disabled navigation, inactive windows, reduced motion
and captured gestures cancel that deadline. Resuming starts a fresh interval;
there is no accumulated catch-up count. At expiry Rust sends one `Auto_next`
proposal carrying the current model revision, geometry epoch and source/target.
`C.apply_request` checks that proposal against the latest model before selecting
the next card. The ensuing model update permits the next native interval. Neither
the four-second deadline nor each animation frame runs a Bonsai clock callback.

Page removal retires the native timer and its event owner. The example's Bonsai
model can retain selection/options while that page is inactive, but a retired
native proposal cannot keep moving its selection. This differs from the native
draft controller's page-scoped lifetime described below.

For example, type into Capture, focus **Measured idea cards**, then press End.
Rust translates the viewport key into `Request.Last`; `on_request` constructs the
Bonsai dispatch effect and the reducer chooses the last measured stop. The next
view update moves the retained cards natively. Capture's fully clipped editor is
removed from the exposed accessibility tree but its draft remains owned by the
mounted editor controller. Home selects the first stop and exposes that same
draft again. Keys sent to the editor itself retain their editing meaning.

`Reverse` calls `with_items` with reversed stable IDs, preserving the selected
card and invalidating obsolete geometry. It does not allocate a new model with
`create`. The Compact viewport toggle changes styles and triggers fresh native
measurement without replacing the editor. Leaving the entire gallery page ends
that controller's lifetime: returning starts a new editor with `initial_text`.
This is distinct from retaining a card while the track remains mounted; persist
drafts in an application model if they must survive page destruction.

Type Capture’s draft, reverse cards/change axis, select neighbors and enable automatic movement.
Selection readout is controlled model state; the draft is independent. GPUIO owns
measurement/input/motion, Bonsai owns the evolving model/options and the adapter owns editor
lease. No asset scope/background worker exists. The public contract still leaves full
focus/accessibility qualification unfinished. Adapt with stable IDs, latest-model request
handling and bounded geometry; do not claim acceptance from rendering or treat auto advancement
as application work.

## Run and review

From the repository root:

```sh
./scripts/gpuio build examples/gallery/main.exe
./_build/default/examples/gallery/main.exe
```

These repository-wrapper commands are instructions, not validation performed for this
documentation change. There is no standalone executable or self-test for this component.
Compilation does not establish native focus, keyboard, animation or platform acceptance. See
[gallery instructions](README.md) and [development](../../docs/development.md).

Run the scoped native walkthrough with
`python3 scripts/test_gallery.py --section carousel-track --images scratch/carousel-track`.
It checks actual keyboard editing/navigation, both axes/themes, partial-neighbor
pointer input, reorder/resize retention, looping, disabled controls and remount.
The [recorded desktop evidence](../../docs/evidence/carousel-track-macos-och41.md)
keeps automatic advancement, drag/wheel cancellation, VoiceOver and frame timing
separate from this subset of the full qualification plan.

For the automatic policy walkthrough, use
`python3 scripts/test_gallery.py --section carousel-automatic --images scratch/carousel-automatic`.
It samples the selected-card readout while focus, hover, disabled state, an
inactive original window and application reduced motion prevent advancement,
then checks fresh intervals and retirement after leaving the page. It temporarily
selects the app's Full/Reduced motion policies and restores System on success;
it does not change macOS preferences. Its interval observations include native
dispatch, Bonsai delivery and AX sampling, not just timer precision.
