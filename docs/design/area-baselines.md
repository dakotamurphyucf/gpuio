# Area baselines

OCH-41, 2026-10-06. Public interface drafted before implementation.
[Local native and public-consumer qualification](../evidence/area-baselines-och41.md)
passes for the recorded scope; broader catalog/release acceptance remains open.

`Chart_appearance.Baseline.create` validates a finite data-unit value within
±1e100. `Series.create ~area_baseline` associates it with a stable series ID;
omission means zero. It is not a screen coordinate or a source observation.
This is a functional data-space equivalent of the pinned low-level Area shape's
scalar pixel-space `y0`, adapted to GPUIO's native domain/projection ownership.
The existing source data, publication, IDs and sampling policy do not change.

Grouped numeric/categorical areas fill between the baseline and their raw source
values. Values above and below the baseline work in all four Cartesian value
directions. The domain includes original extrema even if sampling drops them,
and includes area baselines when the chart has defined observations. A chart
without defined observations retains the existing empty-domain fallback. Missing
observations continue to split runs. Curves and strokes keep their existing
contracts. Lines, bars and radar ignore this field; their domain and baseline
contracts are separate.

Stacked areas still interpret source values as positive/negative contributions.
All participating area series must have the same effective baseline, including
omitted zero. Native preparation reports `Invalid_config` for disagreement.
A common value shifts both cumulative bounds and the stack's domain; native
inspection reports shifted bounds alongside the unchanged contribution. It does
not sum the baseline once for every series. Shared reduction positions, curves,
gaps and stable source selections keep their existing behavior. For example,
contributions 5 and 7 above baseline 20 occupy 20–25 and 25–32.

The option is bounded whole-series configuration: at most 128 appearance series,
not a per-datum callback or large source copy. Worker preparation resolves it
against the same source snapshot and style as the paths. Pending/retired native
plans retain the existing exact-publication and input-lifetime rules.

The appearance series wire appends `float option` after the legend. Chart style
advances from -8 to **-9**; view/options/data remain -2/9/1. Matching packages are
required. Decoding validates the option before geometry. The maximum appearance
fixture includes a present baseline on all 128 series; its 174,599 encoded bytes
remain below the 192 KiB appearance budget. Historical appearance bytes remain
in `chart-appearance.hex`; the current omission fixture is
`chart-appearance-v2.hex`. Current parent chart-view fixtures advance only their
style tag because their appearance series lists are empty.

Qualification must cover paired codecs/invalid values, domain and fill closure,
negative baselines and values, all directions, numeric/categorical sources,
stacked shared/mismatched baselines and shifted inspection bounds, empty/missing
values, native GPU pixels and a public root/installed gallery walkthrough with
source selection and cleanup. Unit geometry alone does not establish GPU or
application acceptance.

Per-bar baselines and arbitrary dense per-datum backgrounds remain explicit
catalog work. They require a separate bounded source/style representation with
aggregation and provenance semantics; this scalar area option does not close
those gaps.
