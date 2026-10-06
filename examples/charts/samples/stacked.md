# Stacked bars and areas

[stacked.ml](stacked.ml) creates immutable example data through the public OCaml
API. [stacked.mli](stacked.mli) exposes one constructor, `data_exn ~area phase`.
It owns no window, Bonsai state or Eio scope. Its callers supply those concerns.

Run the component gallery from the repository's isolated environment:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
./scripts/gpuio exec _build/default/examples/gallery/main.exe
```

Choose Charts & data, then Stacked bars or Stacked areas. Stack layers compares
cumulative and ordinary presentation; the direction controls apply to both.
Select a value, update the sample, and open View data to inspect the original
observations. Local desktop qualification is macOS-first; Linux compilation does
not imply qualified desktop input/accessibility behavior.

## Data and reading order

Read `data_exn`, then its local `series` constructor and `layer` choice. Categories
are Mon–Fri, identified by stable non-monotonic IDs. Both value layers and the
Target line cover the same explicit domain. A datum's identity is local to its
series, so reusing datum IDs across these three distinct series is intentional.

Completed is missing on Wednesday; Adjustment includes a negative Tuesday value.
These observations make the distinction between raw values and stack positions
visible. For Monday at phase zero, Completed is 30 and Adjustment is 10: the
second layer occupies 30→40, while its original value and selected description
remain 10. On Tuesday, 40 followed by -5 ends at 35. The Target line stays at its
own values and does not join either stack.

`phase` changes present observations and preserves IDs and missing data. The
validated Core constructors raise only for these demonstration fixtures through
`Or_error.ok_exn`; applications should handle invalid external data explicitly.
The [stacking contract](../../../docs/design/stacked-charts.md) explains alignment,
signed arithmetic, missing values and shared area sampling.

## Bonsai, GPUIO and runtime responsibilities

[charts_page.ml](../../gallery/charts_page.ml) owns the Bonsai graph. Its `Mode`
selects this sample; `B.toggle` stores Stack layers and the direction choices.
`Chart_options.Cartesian.create ~stacking:Stacked` is the GPUIO presentation
setting. Changing it updates the mounted chart configuration without rewriting
or re-registering the original data. Bars and areas stack independently in mixed
plots; a line remains an overlay.

The page's `Source` holds a scoped `Gpuio_eio.Chart` registration. A click on
Update chart samples invokes the Bonsai effect `Source.update`, constructs the
next immutable dataset, and publishes it through that registration. Native
workers prepare the latest request; Ready updates the page notice. Ready means
preparation completed, not that a frame physically reached the display.

Native Home/Enter or pointer selection returns a publication-scoped event to
OCaml. The page checks the published registration before the shared
`describe_selection` helper resolves the original category/datum IDs. Stacking
changes geometry, not selection identity. Native tooltips additionally show the
stack baseline and endpoint; View data keeps Wednesday's missing observation.

Leaving the page retires its resource scope. Native preparation and stale events
remain subject to the registration's lifetime rules; the pure sample needs no
cleanup. This example performs no file/network I/O and starts no stream itself.
The [page walkthrough](../../gallery/charts_page.md) explains lifecycle ownership
and why Bonsai graph construction is separate from sample data.

To add another layer, give it a new series ID and one optional observation for
every category. To add a category, extend every series at the same position.
Do not precompute cumulative values in the source and then also enable stacking:
that would add them twice and misrepresent the raw-data view. Ordinal colors,
custom stack order and percentage/divergent stacking are separate capabilities.
