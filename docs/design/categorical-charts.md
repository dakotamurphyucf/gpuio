# Categorical chart implementation plan

Local implementation for OCH-41, 2026-10-06. The public data/options APIs,
paired bridge representations, native preparation and gallery are implemented.
[Qualification evidence](../evidence/categorical-charts-och41.md) records the
actual tested scope. Stacking now has a [separate contract](stacked-charts.md).
Ordinal colors, additional presentation options,
VoiceOver and final release acceptance remain separate work.

## Data and identity

A categorical Cartesian dataset sits alongside the existing numeric Cartesian
variant. A category has a positive, typed `Category_id.t` and a bounded UTF-8
label. The explicit domain list defines order; neither integer magnitude nor
lexical label order determines placement. Duplicate category IDs are rejected.
Duplicate labels are allowed because labels are not identity. Unknown categories
must fail admission rather than map to a default slot.

Public construction interfaces under `Chart_data` (accessors are in the `.mli`):

```ocaml
module Category_id : Id

module Category : sig
  type t [@@deriving equal, sexp_of]
  val create : id:Category_id.t -> label:string -> t Or_error.t
  val id : t -> Category_id.t
  val label : t -> string
end

module Categorical_point : sig
  type t [@@deriving equal, sexp_of]
  val create
    :  id:Datum_id.t
    -> category:Category_id.t
    -> value:float option
    -> ?label:string
    -> unit
    -> t Or_error.t
end

module Categorical_series : sig
  type t [@@deriving equal, sexp_of]
  val create
    :  id:Series_id.t
    -> name:string
    -> Categorical_point.t list
    -> t Or_error.t
end

module Categorical_layer : sig
  type t =
    | Line of Categorical_series.t
    | Area of Categorical_series.t
    | Bar of Categorical_series.t
  [@@deriving equal, sexp_of]
end

val categorical
  :  categories:Category.t list
  -> Categorical_layer.t list
  -> t Or_error.t
```

Each series supplies one point per domain category, in domain order. `None` is
an explicit missing observation: a line/area gap and an absent categorical bar,
retained in original-data access. There is no silent sorting, zero filling or
category insertion. This dense aligned contract makes missing values, sampling
and later stacking unambiguous. It is a deliberate API constraint; do not present
it as support for every arbitrary sparse accessor in the upstream Rust API.

Keep the existing 32-series and 100,000-total-value limits. Bound the category
list to 100,000 entries, labels to 256 UTF-8 bytes each and shared total text to
8 MiB. Count category metadata in native and OCaml retained-byte admission,
even for an empty set of series. Empty domain/empty series is valid; nonempty
points against an empty domain are invalid. Existing numeric constructors and
validation retain their behavior.

The wire schema appends categorical contents tag 5, containing
category metadata and categorical layers/points. Do not disguise it as ordinary
numeric Cartesian data. Bound aggregate categories, points and strings before
allocating in both readers, and repeat semantic validation at native admission.
Existing contents tags retain their meanings; old readers reject the new tag.

## Point and band placement

`Chart_options.Cartesian.create ~category_layout` accepts validated
`Chart_options.Category_layout` values:

- `Auto`: point spacing for line/area-only data, band spacing when any bar layer
  exists. All layers share one category projection in a mixed chart.
- `Point`: evenly distributed category centers, optional bounded outer padding;
  one category is centered, and an empty domain creates no marks or category ticks.
- `Band`: explicit inner and outer padding, with lines/areas at band centers and
  grouped bars within each band's usable width.

Use logical-pixel range math. For band layout, define step as
`extent / max(1, n - inner_padding + 2 * outer_padding)` and bandwidth as
`step * (1 - inner_padding)`, with centered alignment of the remaining space.
Inner padding is in `[0,1)` and outer padding in `[0,1]`; reject nonfinite values.
Point layout uses `extent / max(1, n - 1 + 2 * padding)` with centered alignment.
The existing bar-width fraction applies within the available category band;
bar grouping divides that width among bar layers. Numeric data continues to use
its current nearest-x-spacing contract. Explicit point layout with bars uses effective outer padding
`max(padding, 0.5)` so endpoint groups fit; this additional inset is part of the
mixed-layout contract.

This provides point/band functionality, not byte-for-byte reproduction of the
pinned source's `ScaleBand`: that helper caps bandwidth at 30 pixels and has its
own padding formula. Record that difference in the catalog and make the chosen
geometry testable. No implicit category deduplication is needed after validated
unique IDs. `ScaleOrdinal` palette lookup remains a distinct feature; point/band
placement does not satisfy it.

The chart-options envelope is now version 2, with the category-layout field
after bar width. Both paired readers/writers and independent fixtures change
together. The old version-1 record layout is explicitly rejected. GPUIO ships the two
bridge packages together; unsupported option versions must fail explicitly.

## Native preparation and sampling

The original categorical dataset remains immutable in the chart registration.
Native preparation uses a borrowed view with point accessors for value, category rank,
source index, datum ID and label. Share the Cartesian reduction/geometry path
through that view without cloning every label or publishing a second numeric
dataset. Category rank is an internal coordinate, never a substitute public ID.
The complete domain defines spacing even when points are missing or sampling
omits most values. Recompute the projection for size/direction changes; do not
infer a new domain from sampled marks.

Reduction buckets use projected category position. Line envelopes retain original
representatives and explicit gaps. Sum/Mean remains an explicit application
policy for bars: an aggregate spans its original contiguous source range, omits
missing observations from arithmetic, and Mean divides by the number of present
values. An all-missing bucket has no painted bar. Its data remains browsable.
The aggregate is centered over its category interval and uses that interval's
band extent; it must not inherit a one-category width after combining many
categories. Tooltip/selection descriptions identify the category range and the
aggregation, rather than inventing a single category for the result.

Preserve `Chart_selection.Cartesian` series/datum span semantics. Selection remains
scoped to the exact publication; applications can resolve category IDs from the
original data. Value-axis reversal does not reverse category order or keyboard
order. Both horizontal directions bucket along plot height. Pointer hits use
actual prepared geometry; no per-frame OCaml scale or tooltip callback is added.

Account for rank/index maps and additional prepared labels in the worker workspace
and retained plan quotas. Cancellation checkpoints and two-worker limits remain.
Any quota rejection remains explicit rather than silently changing the sampling
policy. Category domain changes through `set` are ordinary revision changes;
`reset` still changes logical data identity and clears retained selection.

## Labels and original-data access

Category ticks use domain labels, not numeric rank formatting. The existing tick
budget bounds visible labels; choose deterministic representative domain positions,
including endpoints when feasible, without changing geometry or deleting data.
All labels remain available in native tooltips and the paged original-data view.
Duplicate labels remain distinguishable by category IDs in the data detail.

The original-data table must expose category label/ID, series, datum ID and raw
value or Missing. Its pagination includes missing values and values omitted by
sampling. The tooltip path must distinguish exact category values from aggregated
category ranges. Axis hiding, themes and all four value directions retain those
semantics. This is accessibility data plumbing; actual VoiceOver acceptance still
requires physical testing.

## Qualification requirements

1. Add domain modules/constructors, explicit wire representation, bounded readers,
   independent OCaml/Rust fixtures and malformed-domain tests. Exercise reordered
   IDs, equal labels, missing values, empty/singleton data and aggregate bounds.
2. Introduce the borrowed numeric/categorical preparation interface. Preserve all
   numeric geometry/reduction/selection tests while adding true point/band math,
   category ticks and resource accounting.
3. Cover aligned mixed layers, all four value directions, sparse observations via
   explicit None, Sum/Mean range provenance, large-domain reduction and hit-query
   bounds. Verify original data and category labels after publication/reset.
4. Add public OCaml samples and gallery controls for point/band layout, labels and
   missing observations. Add native pixel checks at the existing four scales and
   an installed-consumer walkthrough with selection, data pages and teardown.
5. Update the adjacent walkthrough, catalog and Linear with exact results. Keep
   stacking, ordinal colors and richer presentation options open until their own
   implementations and acceptance pass.

The relevant code paths are `chart_data` and its wire adapters, `chart_options`,
`chart_reduce`, `chart_geometry`, `chart_selection`, `chart_details`,
`chart_table`, `chart_presentation`, `chart_paint`, the source/job resource charges,
and the chart samples/gallery. A passing geometry unit test alone does not complete
this feature: source admission, native interaction, original-data semantics and
installed public consumption are required together.

The later [stacking addition](stacked-charts.md) advances the options record to
version 3. The v2 fixture remains historical rejection evidence.
