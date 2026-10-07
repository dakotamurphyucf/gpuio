# Bar backgrounds: Bonsai state and scoped chart publication

**Charts & data → Bar backgrounds** demonstrates dense, data-owned brushes,
stable batch identity, sparse highlighting and explicit mean aggregation. It is
an OCaml application example using public APIs; no Rust plotting callback is
required. Read the [pure fixture walkthrough](chart_backgrounds_data.md) first.

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
./scripts/gpuio exec _build/default/examples/gallery/main.exe
```

Use the [development guide](../../docs/development.md) for prerequisites. No
external feed, assets or credentials are needed. These commands launch the ordinary
application. The separate macOS interaction driver is
`python3 scripts/test_gallery.py --section chart-backgrounds`; it opens and closes
a foreground test window. This walkthrough does not establish native selection,
keyboard, accessibility or paint acceptance. macOS is the release target; full
Linux desktop qualification remains deferred, distinct from required Linux checks.

## Source map and reactive graph

[chart_backgrounds.mli](chart_backgrounds.mli) exposes `component app window palette
graph`, returning a reactive GPUIO view. In
[chart_backgrounds.ml](chart_backgrounds.ml), `B` abbreviates `Bonsai.Cont`, `E`
abbreviates `Bonsai.Effect`, `V` abbreviates `Gpuio_bonsai.View`, and `Registered`
abbreviates `Gpuio_eio.Chart`. `appearance` constructs view styling; `component`
owns controls, registration and the chart description. The
[chart page wrapper](charts_page.md) selects this branch with `match%sub`.

A Bonsai graph records dependencies between changing values and computations.
`B.Expert.Var.create` allocates explicitly mutable application state: the fixture
model, optional selected target and color-application count. `get` reads its
current value inside an action; `set` changes it; `value` exposes it as a reactive
input. Mutations occur inside effects, not while deriving the view. The
`source_theme` reference remembers the last theme applied to data.

`B.toggle` supplies reactive Boolean controls and toggle effects for horizontal,
reversed, sparse-highlight, mean and uniform choices. `B.state` supplies the notice
and its setter. With `B.Let_syntax` open, `let%arr ... and ...` reads reactive inputs
together and derives an ordinary view whenever they change. `E.of_thunk` delays
imperative work until an effect runs; `E.bind` sequences its result; `E.map`
transforms an asynchronous effect's result. Constructing a button with an effect
does not execute the button action.

## Native ownership and retained state

[Preview_scope.acquire](preview_scope.md) creates a child of this window's Eio
scope on branch activation. Its result is `Loading`, `Failed error` or `Ready
source`; those cases select preparing/error text or the interactive card.
`Registered.create` completes after the initial native publication. Registration
does not mount a chart: `V.chart` separately receives a borrowed
`Registered.handle source` through `Chart.Config.create`.

Deactivation cancels the child scope and releases the native registration;
late acquisition replies are suppressed. The notice resets to preparing.
Reentering the same branch creates a new native source from the retained fixture
and last applied source theme, and clears the selected target. Fixture phase,
order, pattern mode, color count and Bonsai control choices remain in the branch's
OCaml graph across reentry. Another window has its own graph and resource scope.
The borrowed handle does not prolong a cancelled source's lifetime. This example
uses the application's Eio chart service; it introduces no file I/O, custom worker
or independent event loop. See [the registration contract](../../lib/eio/chart.mli).

`publish` reads the latest model when its effect executes, applies a pure change,
constructs data and calls `Registered.set`. After local success it stores the new
fixture and source theme; an explicit theme application also increments the count.
The notice says publishing. Local admission is not native publication
acknowledgement: updates can coalesce, and native rejection can retain the prior
published dataset. The model and counter describe admitted requests, not receipts
that those colors have been painted.

`on_event` receives asynchronous `Chart.Event` observations. `Selection_changed`
stores the typed target; `Failed` updates the notice. `Ready` updates the notice
only when `Registered.is_published source` says the latest desired dataset has
been accepted natively. That predicate does not prove paint completion.
The chart's own preparation also needs layout; a preview scrolled outside the
viewport may remain preparing until revealed. Source publication and visible
chart readiness are separate stages. Selection descriptions use the same guard
and `Registered.data source`, then
`Gpuio_chart_samples.describe_selection`. Thus requested fixture data is never
used to explain a selection against an older published source.

## Source brushes, view style and sampling

The initial source uses `Theme.default`. Changing the preview palette updates
chart axes, labels, grid, selection, size and ordinary view styling, but it does
not republish source colors. **Apply preview colors** passes `Palette.theme p`
to `publish` with `Fn.id`, keeping fixture values/order/patterns while rebuilding
and publishing resolved brushes. Later value/order/pattern publications use that
last applied theme. This makes color ownership visible rather than silently
coupling theme observation to a source upload.

`appearance p ~sparse ~uniform` constructs `Chart_style` and `Chart_appearance`.
**Highlight Batch 01** adds exactly one datum override using `Data.series_id` and
`Data.first_id`: an amber solid fill with four-pixel corners. That sparse override
is separate from the 24 source backgrounds; it follows Batch 01 after reversal.
Sparse fills override source fills, with corner inheritance independent.

The two axis controls map to the four explicit Cartesian orientations.
**Mean background bars** chooses `Chart_sampling.Bar.mean ~max_buckets:6`;
otherwise `Bar.exact` preserves exact bars. Mean is an explicit reduction of
presentation, while inspection and **View data** retain original values.
**Uniform background agreement** selects `Chart_appearance.Aggregates.Uniform`;
otherwise the policy is `Inherit_series`. This choice is independent of Mean and
does not aggregate on its own. Uniform requires agreement among participating
observations' effective bar appearance; mixed appearances fall back to series
style. It does not invent an averaged color or apply one highlighted datum to a
whole mixed bucket. See [appearance](../../lib/core/chart_appearance.mli),
[sampling](../../lib/core/chart_sampling.mli) and the
[chart contract](../../docs/design/charts.md).

## Trace an interaction and adapt it

Select Batch 01, then choose **Reorder batches**. The native button event runs the
stored effect; `publish Data.reorder` reads the current fixture and rebuilds
categories/points in reverse order with unchanged IDs. `Registered.set` admits
the request locally, then Bonsai displays reordered status and publishing text.
Until the desired dataset is published, the description guard suppresses stale
selection text. Native acceptance/preparation later produces observations; the
description reads published original values, and an eligible Ready observation
updates the notice. **Highlight Batch 01** changes configuration rather than
uploading data, and continues to address the same datum at its new position.

For a realistic adaptation, add a second highlighted batch in `appearance` using
its stable series/datum pair; do not use the current screen index as identity.
Keep sparse exceptions in `Chart_appearance` and dense per-record brushes in
`Chart_data.with_bar_backgrounds`. For external datasets, replace fixture `_exn`
unwrapping with recoverable validation, preserve the scope owner and published
data guard, and present admitted requests separately from native acceptance.
