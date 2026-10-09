# Stacked Cartesian charts

OCH-41, 2026-10-06. `Chart_options.Cartesian.create ~stacking:Stacked ()`
opts into cumulative bar and area rendering. The default remains `Grouped`.
The public [interface](../../lib/core/chart_options.mli) is source-independent;
immutable chart registration and asynchronous preparation keep their existing
ownership, cancellation and stale-publication rules.

## Alignment and arithmetic

Bars stack with bars and areas with areas, independently, in source layer order.
Line overlays retain their original values. A mixed chart therefore does not
silently add a bar's value to an area's baseline. Numeric layers within each of
these two groups must have exactly equal x positions and lengths; native
preparation rejects incompatible data. Different groups and line overlays need
not align. Categorical data already requires each series to cover the declared
category domain in order.

The first baseline is zero. Each subsequent baseline is the previous endpoint;
the new endpoint is baseline plus the current value. Signed values accumulate
algebraically, matching the pinned source's natural-order `Stack` behavior.
For example, values 10, -4 and 2 have bounds 0→10, 10→6 and 6→8. This is neither
a divergent positive/negative stack nor percentage normalization. Zero-height
marks and overlaps from negative values retain the ordinary chart contracts.

Missing categorical bars and missing area observations contribute zero to later
baselines, but are still missing in the source. They have no own mark; an area's
own gaps remain disconnected. Numeric bars keep their existing requirement for
defined values. Neither stacking nor sampling rewrites the source dataset.

## Sampling and curves

Bars use the explicit Exact/Sum/Mean policy independently before stacking.
Reduced buckets align because source positions and the sampling domain align.
Cumulative offsets are keyed by source interval position, so an all-missing
bucket in an earlier layer cannot shift a later layer. Mean still divides by
that series' count of present observations. A selection retains the full original
source span and existing aggregation tag, rather than treating its stack endpoint
as an original observation.

Areas share one ordered union of retained positions across all their cumulative
boundaries. Each envelope bucket contributes its endpoints and lower/upper
minimum and maximum positions. Both sides of every missing-value transition are
also retained. This can preserve more positions than independent ordinary-line
envelopes; total retained positions across layers stay bounded by the source
point count. Explicit Exact preserves all positions.

All boundary curves are constructed on that shared grid, including positions
where an individual series is missing (its contribution there is zero). The
resulting curve segments are then clipped to that series' defined runs. Thus
adjacent boundaries share not just endpoints but Natural-curve tangents, even
when their missing runs differ. Missing points influence neighboring cumulative
interpolation but do not create a path across the owning series' gap.

A fill follows the upper boundary forward and the actual lower curve backward.
Reversing the computed segments, including swapped cubic controls, preserves
Step_after semantics; recomputing Step_after over reversed points would not.
Natural interpolation retains its existing possibility of overshoot. Plot clipping
and geometry admission still apply. Axes include both stack-boundary extrema;
all four Cartesian value directions use the same arithmetic and source identity.

## Selection, presentation and resources

Tooltips show the original or explicitly aggregated value, followed by stack
baseline and endpoint. Selection events keep original series/datum IDs and spans.
The original-data table remains unchanged, including missing observations.
Stacking is a display option: it can change without republishing the dataset.

Native preparation borrows source labels/points. Area preparation uses at most
100,000 bounds records (16 bytes each), one cumulative f64 vector and one shared
selection bitmap; reduced stacked points charge their actual vector capacities.
The retained-value metric counts defined source observations; missing boundary
control positions are charged as memory but are not selectable observations.
Bar offsets use an aligned cumulative vector, released before area preparation.
Additional temporary geometry vectors and curve copies are bounded by the same
100,000-point source limit. They run under the existing 192 MiB per-worker
reservation; retained plan vectors, summaries and commands count toward the
64 MiB plan limit. Cancellation checks remain in source, envelope and geometry
loops. Dense inputs may fail geometry/mesh admission instead of being silently
resampled below the requested policy.

The options envelope is **version 3**, adding the stacking tag after categorical
layout: Grouped=0, Stacked=1. The paired readers reject older options and unknown
tags. Versions 1 and 2 view fixtures remain rejection inputs; the independently
specified v3 fixture covers the new default frame. This requires matching OCaml
and Rust packages and does not change chart-data envelope version 1.

## Qualification boundary

The implementation must pass paired codec/expect tests, native signed/missing/
alignment/sampling/curve/selection tests, GPU pixels and the public installed
OCaml gallery walkthrough before feature acceptance. The [sample walkthrough](../../examples/charts/samples/stacked.md)
explains the application interface. These checks do not establish VoiceOver,
physical presentation latency, Linux desktop behavior or milestone completion.
