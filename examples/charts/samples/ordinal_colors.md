# Stable ordinal colors

[ordinal_colors.ml](ordinal_colors.ml) is a pure Core example built with public
GPUIO types. Its [interface](ordinal_colors.mli) separates immutable data creation
from the ordinal color mapping. It owns no native registration or Bonsai graph.

Build and run the gallery with the isolated repository toolchain:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
./scripts/gpuio exec _build/default/examples/gallery/main.exe
```

Choose Charts & data, then Ordinal colors. Research and Build have explicit colors
keyed by slice identity. Review is intentionally outside the domain. Toggle
Explicit unknown color to compare a stable unknown color with the ordinary
position-palette fallback. Update chart samples changes the values and rotates
the source order; the declared domain and known colors remain unchanged.
Desktop qualification is macOS-first; a Linux build alone is not GUI acceptance.

## Data and mapping

Read `data_exn` first. Slice IDs 9, 2 and 7 identify Research, Build and Review.
Every phase adds to their raw values; odd phases move Review to the front. The
IDs remain stable. The sample validates a bounded phase and uses raising wrappers
for these known fixtures; applications should handle invalid external input via
the public `Or_error` results.

Then read `mapping`. Its explicit domain is Build (2), followed by Research (9),
with teal and purple range colors. That order controls the assignment, even when
the source lists Research first. Review either receives the optional yellow
unknown color or falls back to its current ordinary palette position.
`Chart_style.Key.slice` keeps this identity distinct from a series/node with the
same raw ID. The [ordinal contract](../../../docs/design/chart-ordinal-colors.md)
explains repeating ranges, validation bounds and other chart-family key types.

## Bonsai, GPUIO and Eio

The [gallery page](../../gallery/charts_page.ml) owns the Bonsai state and effects.
Its mode selects this sample. A Boolean Bonsai toggle controls the unknown-color
policy; `Chart_style.create ~ordinal` turns the mapping into a resolved native
style. The sample uses concrete colors, but mapping range/unknown values can be
ordinary theme tokens: the style constructor resolves them using its `theme`.

Update chart samples invokes the page's `Source.update`. That effect constructs
the next dataset and publishes it through the scoped `Gpuio_eio.Chart` resource.
Native workers build new geometry and resolve colors against the same domain.
A Ready observation updates the notice; this is preparation acknowledgement, not
a display presentation timestamp. Native marks and legend swatches consume the
same prepared color vector.

Selection events return the slice ID to OCaml. After publication is acknowledged,
the shared `describe_selection` helper resolves that ID against the published
source. An existing Research selection should follow Research when its source
position changes. Changing the unknown policy updates the style without changing
source identity. View data continues to expose raw values in source order.

Leaving the page releases its resource scope and retires pending native work.
The sample itself performs no I/O and needs no cleanup. The [page walkthrough](../../gallery/charts_page.md)
explains the Bonsai graph, scoped registration and publication guards in detail.

To give Review a permanent explicit domain color, add its slice key to the domain
and a third range color. If only the key is added, the two-color range cycles and
Review receives teal. To preserve colors under source changes, keep the domain
order fixed. This example does not precompute colors from current source indices.

A concrete reorder starts at phase zero: Research/Build/Review have values
40/35/25. At phase one their values become 41/36/26 and the source order is
Review/Research/Build. Build remains teal and Research remains purple because
`mapping` pairs its fixed typed domain with its range. The gallery bounds its
phase to 0–12; `data_exn` itself accepts only finite phases in [0,1000]. The odd/even
rotation uses `Float.to_int phase`, so fractional phases follow the truncated
integer's parity.

Click **Explicit unknown color** to turn it off: the effect returned by
`B.toggle` changes a reactive Boolean, and the consumer's `let%arr` derives
`mapping ~unknown:false`, omitting the optional unknown color. `let%arr` operates
on Bonsai inputs; `Option.some_if` in this pure helper just returns `Some color`
or `None` immediately. The same chart handle now uses Review's ordinary palette
fallback without republishing its source. See
[chart_style.mli](../../../lib/core/chart_style.mli) for `Ordinal.create` and
`Key.slice`, and [chart_data.mli](../../../lib/core/chart_data.mli) for validated
slices. Existing macOS checks appear in the
[ordinal evidence](../../../docs/evidence/chart-ordinal-colors-och41.md); this
documentation review claims no new input or pixel validation.
