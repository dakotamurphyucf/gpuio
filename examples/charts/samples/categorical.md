# Categorical chart samples

[The implementation](categorical.ml) and [interface](categorical.mli) build a
small mixed bar/line dataset entirely through public OCaml APIs. They contain no
native callbacks, mutable application state, I/O, or polling loop. The gallery
owns the Bonsai controls and scoped chart registration.

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
./scripts/gpuio exec _build/default/examples/gallery/main.exe
```

Choose **Charts & data → Categorical**. The sample has four ordered categories:
Research (42), Code (7), Research (99), and Review (3). The two Research labels
are deliberately equal; their IDs and positions are distinct. The Completed bar
for Code is missing, while the Target line still has a value there.

Read `categories`, then `data_exn`, then `describe_selection`. The export through
[gpuio_chart_samples.ml](gpuio_chart_samples.ml) lets the shared selection helper
handle numeric and categorical datasets. [charts_page.ml](../../gallery/charts_page.ml)
contains `Mode.Categorical`, the `Category_layout` UI choices, `Source`, and the
Bonsai component. Its [walkthrough](../../gallery/charts_page.md) explains their
runtime and lifetime relationships.

`Category_id`, `Series_id`, and `Datum_id` have separate meanings. Category IDs
identify the domain; series IDs identify a named layer; datum IDs identify values
inside a series. `Categorical_point.create` records category membership and an
optional numeric value. The dataset constructor checks that every series covers
the explicit domain in order, with `None` for missing observations. The sample
uses non-monotonic category IDs to show that ID magnitude does not control spacing.

`data_exn phase` adds a finite nonnegative phase to present values while preserving
IDs, category order, and the missing bar. It returns an immutable validated
`Chart_data.t`. Its raising constructors are appropriate for checked demonstration
fixtures; application input should handle the constructors' `Or_error.t` results.

The native chart applies the Auto/Point/Band layout selected by the gallery.
Auto uses bands because this dataset contains bars. Line points share band centers.
The explicit Point control includes outer padding; Band changes inner and outer
padding. Horizontal/reversed directions change native projection without rewriting
the sample. The original-data view shows raw category labels/IDs and Missing,
including observations that have no painted mark.

Clicking Update chart samples runs a Bonsai effect in the gallery. `Source.update`
constructs the next dataset and calls the scoped Eio Chart API to publish it.
Native preparation happens asynchronously. The Ready observation updates the
notice; it is not a physical-presentation timestamp. Selecting a mark sends a
publication-scoped target back to OCaml. `describe_selection` verifies the datum
IDs at both source-span endpoints and resolves the category ID against that
publication. The caller must first establish that the registration's desired data
is published; these endpoint checks alone cannot validate a stale event.

For an exact selection, the description includes series, label, category ID and
raw value. For Sum/Mean it describes a category interval; it does not call an
aggregate a single category. Native reduction preserves source spans, excludes
missing values from arithmetic, and leaves the full original table available.
The example itself uses the default sampling policy and does not run a stream.

To add another category, append its domain entry and one corresponding point to
each series. Use `None` if a series has no observation. Keep IDs stable for the
same logical data; labels can change without changing identity. To reorder,
reorder the domain and each series together and publish a new revision. The
constructor rejects an unmatched order rather than silently repairing it.

The sample owns no resources to dispose. The gallery's `Preview_scope` owns the
registration, cancels it on departure, and reacquires it on return. Do not attach
a long-lived external data feed to a virtual mark's lifetime. See the
[categorical contract](../../../docs/design/categorical-charts.md) for bounds,
wire compatibility, projection and aggregation rules. Local macOS tests cover
this sample; full Linux desktop and VoiceOver acceptance remain separate work.
