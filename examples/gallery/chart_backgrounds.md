# Bar backgrounds: Bonsai state and scoped chart publication

**Charts & data → Bar backgrounds** demonstrates dense, data-owned brushes,
stable batch identity, per-bar baselines, sparse highlighting and explicit mean
aggregation. It is
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
order, pattern mode, baseline choice, color count and Bonsai control choices remain
in the branch's
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

For an exact Cartesian target, the component pattern-matches
`aggregation = Exact` and queries `Chart_data.bar_baseline data ~series
~datum:span.first` on that published source. A stored value is appended as
`baseline %.3g`; absent metadata leaves the ordinary endpoint description alone.
Thus Zero mode renders from zero without an explicit baseline suffix. Aggregate
targets retain the sample helper's source-span description; the component does
not treat an aggregate as a single datum or invent a baseline for it.

## Source brushes, view style and sampling

The baseline row uses `List.map Data.Baselines.all` to construct three buttons:
**Zero baselines**, **Shared baseline 40** and **Individual baselines**.
`Data.Baselines.equal candidate (Data.baselines current)` derives each selected
state with typed equality. Clicking a button runs
`publish (fun data -> Data.with_baselines data candidate)`: the function changes
the immutable fixture, and the surrounding effect rebuilds and explicitly
publishes the source. This is data metadata, so a baseline button uploads data;
the orientation and appearance switches instead rebuild chart configuration.
Shared uses 40 for all 24 bars; Individual cycles 20, 40 and 60 by original batch
index, so IDs, original endpoints and baseline associations survive reversal.
See the [fixture walkthrough](chart_backgrounds_data.md) for construction.

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
Mean also requires the defined observations in each bucket to share an effective
baseline. Zero and Shared satisfy that condition. Individual deliberately gives
the combined observations different baselines, so preparation reports
`Chart.Error.Invalid_config`. `on_event` handles `Failed error` by displaying the
typed error's sexp in the notice. Source publication can succeed while chart
preparation fails; the baseline buttons can therefore show the admitted choice
alongside that error. Return to **Shared baseline 40** or turn Mean off to request
a compatible configuration. The [local baseline walkthrough](../../docs/evidence/bar-baselines-och41.md)
checks this failure/recovery sequence in the root and fresh installed consumer,
alongside original-data inspection and source cleanup.

**Uniform background agreement** selects `Chart_appearance.Aggregates.Uniform`;
otherwise the policy is `Inherit_series`. This choice is independent of Mean and
does not aggregate on its own. Uniform requires agreement among participating
observations' effective bar appearance; mixed appearances fall back to series
style. It does not invent an averaged color or apply one highlighted datum to a
whole mixed bucket. See [appearance](../../lib/core/chart_appearance.mli),
[sampling](../../lib/core/chart_sampling.mli) and the
[chart contract](../../docs/design/charts.md).

## Trace an interaction and adapt it

Starting with Exact at phase zero, choose **Shared baseline 40**. The native
button action runs its stored effect, `publish` reads the latest fixture and
`Data.with_baselines` selects Shared without changing Batch 01's endpoint 30.
`data_exn` rebuilds brushes using the last applied source theme and attaches 40
to each stable pair; local `Registered.set` success then stores the fixture and
sets the publishing notice. Bonsai observes the new model and marks the Shared
button selected. After publication and preparation, selecting Batch 01 uses the
published data to describe endpoint 30 and baseline 40, representing a downward
40→30 interval. In Individual mode its baseline becomes 20 while its endpoint
remains 30; enabling Mean requests incompatible buckets and routes the explicit
failure through `on_event`. Returning to Shared requests recovery through another
publication. Neither a selected button nor local admission proves native paint.

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
`Chart_data.with_bar_backgrounds`; keep interval origins in the independent
`Chart_data.with_bar_baselines` sidecar, rather than rewriting endpoints.
For external datasets, replace fixture `_exn`
unwrapping with recoverable validation, preserve the scope owner and published
data guard, and present admitted requests separately from native acceptance.
