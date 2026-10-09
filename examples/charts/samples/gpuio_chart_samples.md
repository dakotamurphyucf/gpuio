# Shared chart fixtures and selection descriptions

[gpuio_chart_samples.ml](gpuio_chart_samples.ml) and its
[interface](gpuio_chart_samples.mli) define deterministic mock datasets shared by
Chart Studio and the component gallery. This is a pure Core library: it creates
immutable `Gpuio.Chart_data.t` values and descriptions, with no Bonsai graph,
window, native handle, Eio task or file/network I/O. Its [Dune stanza](dune) links
`core` and `gpuio` and enables `ppx_jane`; the executable callers provide runtime
ownership.

From the repository root, use the isolated toolchain:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe examples/charts/main.exe
./scripts/gpuio exec _build/default/examples/gallery/main.exe
# Alternatively, launch the standalone Chart Studio:
./scripts/gpuio exec _build/default/examples/charts/main.exe
```

The gallery's **Charts & data** starts with Line at phase zero. Choose a family,
press **Update chart samples**, select a mark, or open **View data**. Chart Studio
also offers **Edge cases**, **Horizontal** and **Dense legend**. Its optional
`--self-test` exercises diagnostics rather than an ordinary application session;
see the [owning README](../README.md) for flags and existing platform evidence.
There are no sample assets to fetch. The release is macOS-first; these instructions
and documentation review do not establish Linux desktop or VoiceOver acceptance.

Read `Family`, `series`, `data_exn`, `describe_selection`, then `edge_data` and
`preset_data_exn`. `Family.t` is a closed variant of seven chart families;
`Family.all` determines chooser order, `of_index` returns an `Or_error` for invalid
indices, and `label`/`description` supply UI text. `Preset.t` chooses Standard,
Mixed, Horizontal or Dense_legend. It describes data choice; Horizontal's actual
orientation comes from the caller's `Chart_options`, not from the data constructor.

`datum`, `series_id`, `node_id` and `edge_id` convert integers into distinct typed
IDs. A series ID identifies a dataset layer; a datum ID identifies a value within
its series; graph node and edge IDs inhabit separate types. IDs stay stable across
phase updates. `series` generates 24 points, x = 0 through 23, and a sinusoidal y
plus a rising trend. `data_exn` wraps these in line, area or bar constructors;
its pie, radar and Sankey fixtures are intentionally constant. Radar values refer
to axis IDs, and candle high/low enclose both open and close. The four-node Sankey
joins Incoming to Reasoning/Tools and then Complete.

`validate_phase` rejects negative/nonfinite phases. Constructors return validated
`Or_error.t` values, converted with `Or_error.ok_exn` because these are checked demo
fixtures; generated invalid/overflowing values still raise. For external data,
handle errors from the [public data constructors](../../../lib/core/chart_data.mli)
rather than copying this raising fixture policy.

`preset_data_exn Mixed` creates area/bar/line layers with separate IDs; Horizontal
creates two bar series. Dense_legend starts with 128 slices and adds up to 128 more
as phase grows, never exceeding 256. `edge_data Line` allocates 100,000 original
points, with descending IDs and a gap every thousand points. The other edge cases
are empty area data, signed bars, a zero pie slice, reordered radar values, a flat
negative candle and an isolated Sankey node with a zero-flow edge. These deliberately
exercise distinctions between original data and paintable geometry; they are
fixtures, not a streaming or performance harness.

## Resolving semantic selections

`describe_selection` returns `string option`: `None` means the target does not
resolve in the supplied data, including a missing exact value or family mismatch.
It inspects `Chart_data.Expert.contents` and the
[typed selection variant](../../../lib/core/chart_selection.mli). Cartesian targets
contain a series ID and a source span. The helper checks source positions and both
endpoint IDs, then describes an exact original value or the count for Sum/Mean.
Categorical targets delegate to [Categorical](categorical.md). Pie, radar and graph
targets look up their typed IDs. Candlestick descriptions check the first candle ID;
an aggregate reports session count rather than pretending it is one source candle.
These checks do not independently prove publication identity.

A span indexes the original list over the half-open interval
`[start_index, start_index + length)`. Its endpoint IDs identify those positions;
they are not a numeric ID range. This matters for the descending IDs in
`edge_data Line`. Line/area envelope sampling still selects an exact original
representative; bar Sum/Mean identifies the contributing source span. Even a
one-point Sum/Mean remains an aggregate, so this helper describes its sample
count rather than treating it as Exact. It does not compute the aggregate value
or mutate the originals; native preparation and the original-data browser have
separate responsibilities. See [chart_selection.mli](../../../lib/core/chart_selection.mli).

The syntax here is **Option syntax**, not Bonsai syntax:

```ocaml
let open Option.Let_syntax in
let%bind first = List.nth points span.start_index in
let%map value = D.Point.y first in
sprintf "%s · x %.3g · value %.3g" (D.Series.name series) (D.Point.x first) value
```

`let%bind` stops the lookup with `None` when an optional input is absent;
`let%map` wraps a successful final result in `Some`. This function runs synchronously
on ordinary values, without creating reactive state.

## From a gallery event to a description

The consumer [charts_page.ml](../../gallery/charts_page.ml) constructs the Bonsai
graph. `B.state`/`B.toggle` create reactive values plus effects that change them;
its `let%arr` reads their current values together to derive a GPUIO view. An effect
is work a callback schedules, rather than an immediate state assignment while
building the view. `Source.create` registers phase-zero data in an Eio-owned
`Gpuio_eio.Chart`; `V.chart` borrows that registration's handle.

For example, choose Line and press **Update chart samples**. The button runs
`E.of_thunk (fun () -> Source.update source)`, which advances the phase modulo 13
and calls `Registered.set`. Native workers prepare the new publication; a Ready
observation updates the notice. Now click an Atlas point: native code sends a typed
`Selection_changed` event, the page's effect stores the target in `selected`, and
`let%arr` derives the description. The page calls this helper only when
`Registered.is_published` holds, using `Registered.data` for that registration.
`Registered.data` is the desired snapshot, including during a pending update;
the guard is what establishes that it has also been accepted natively. Stable
endpoint IDs alone cannot establish that its values belong to the publication
that produced a selection.
The native chart owns hover and preview state; only committed selection is stored
in the OCaml page. Ready reports preparation, not physical display presentation.

Choosing a different mode uses `Registered.reset` and clears the page selection.
Leaving the page releases its `Preview_scope`; returning reacquires a registration
while retaining chooser preferences. These lifetimes belong to the caller. The
pure catalog needs no disposal and must not be given responsibility for native
resource cancellation. The [page walkthrough](../../gallery/charts_page.md)
explains publication guards, events and scope teardown in detail.

For a small adaptation, change the names and y formula in `series` while keeping
its ordered x coordinates, finite values and stable datum/series IDs. To add a new
family, update `Family.t`, every exhaustive family match and the consuming chooser;
to add presentation alone, prefer the caller's options/style. Related independent
samples have their own guides: [categorical](categorical.md), [stacked](stacked.md),
[ordinal colors](ordinal_colors.md), [inspection](inspection.md) and
[Sankey presentation](sankey_presentation.md). Their module aliases in this library
provide one shared entry point without transferring runtime ownership.
