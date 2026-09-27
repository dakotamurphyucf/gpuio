# Native charts (OCH-40)

Status: in progress. The validated Core data model is implemented and tested.
The extracted Sankey layout source compiles against the existing GPUI revision.
Native chart resources, widgets, interactions, accessibility and public graphical
examples are not implemented yet. This document separates the current data
contract from the remaining implementation work.

## Data contract

`Gpuio.Chart_data` contains immutable values, with abstract positive IDs and
validated constructors. Labels are data, never identity or callbacks. Datum IDs
are scoped to a Cartesian series, or to the dataset for slices/candles/radar axes;
series, flow-node and flow-edge IDs have separate abstract types. Updates retain
IDs when the application means the same item. A label or array index is not a
substitute for identity.

| Family | Data and invariants |
| --- | --- |
| Line | One or more named series; strictly increasing numeric x; optional y marks an explicit gap |
| Area | The same series contract, with zero baseline; negative values remain valid |
| Bar | Numeric x, required y, zero baseline; negative values remain valid |
| Mixed Cartesian | Line/area/bar layers share numeric coordinates and unique series IDs |
| Pie | Unique labeled slices with nonnegative values; all-zero input is valid |
| Radar | 3–64 unique named axes with positive maxima; each named series provides exactly one value in each axis's domain |
| Candlestick | Increasing numeric x and unique IDs; finite OHLC with low <= open,close <= high; negative/zero-height candles are valid |
| Sankey | Unique nodes/edges, existing distinct endpoints and acyclic topology; nonnegative values; parallel edges, isolated nodes, zero flows and imbalance are valid |

Every family accepts wholly empty input. Finite numeric values have absolute
value <=1e100, leaving room for bounded sums and f64 normalization. NaN/infinity
are rejected, never used as gap sentinels. Chart adapters must normalize in f64
before converting to screen coordinates; valid raw magnitudes need not fit f32.

There are at most 32 Cartesian or radar series, 100,000 Cartesian points or
candles, 256 pie slices, 256 flow nodes and 2,048 flow edges. Radar has at most
64 axes. Text is valid UTF-8 without NUL/CR/LF, bounded to 128 bytes for series
names or 256 for item labels and 8 MiB total per dataset. Names and labels that
identify slices, axes or nodes are nonblank. Point/candle labels may be empty,
allowing a later native numeric formatter to supply their textual alternative.

Constructors perform no implicit sorting, aggregation or downsampling. They
validate source data and preserve its IDs, values and gaps. `value_count` includes
zero/missing values. `Expert.contents` exposes a read-only view for protocol and
data-table adapters without an unchecked constructor for validated datasets.

## Pinned implementation assessment

Inspected the clean local GPUI Kit checkout at
`84f57fdfcb4910623fb0bb7f795b077e249f9271`, including all seven exports under
`crates/component/src/chart` and the scale/shape/axis/grid/label/tooltip modules.
The full styled component crate uses a different GPUI package pin (`gpui-pre`
0.3.1), theme access and plotting macros. GPUIO keeps its existing GPUI revision
`a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b` and its own style/ownership boundary.

Line/area/bar/radar have optional hover behavior; the seven wrappers do not share
our required stable semantic selection, keyboard and accessible-data contract.
The line wrapper rebuilds scales during painting. Its point scale uses categorical
positions, whereas GPUIO's data model deliberately preserves numeric x spacing.
The low-level line shape collects defined points into one path, so GPUIO's explicit
gap semantics also require deliberate path segmentation. The linear scale's sealed
numeric types are f64 (or optional Decimal); integer arithmetic is not a concern
for the current source. Zero-extent domains return no tick and need an explicit
native empty/single-value rendering policy.

`rust/plot` now isolates the upstream Sankey topology/layout/ribbon implementation
from the styled component crate. Its Apache license, exact source/hash and the
namespace/iterator adaptations are recorded in `rust/plot/UPSTREAM.md`. All 12 retained
upstream tests pass against GPUIO's unchanged GPUI revision. This proves source
compatibility for that algorithm, not a rendered or interactive chart. The adapter
must still validate decoded data, bound iterations, normalize raw flow magnitudes,
retain original semantic values and cache layout outside per-frame painting.

## Remaining implementation contract

The native implementation must cover all seven families; the pure model and
Sankey extraction do not replace that scope:

- Define paired bounded codecs and revisioned, application-owned chart resources.
  Scoped Eio ownership should follow existing canvas/document registration, with
  coalesced desired updates, atomic publication, stale-generation rejection and
  explicit release/window-unmount behavior. Do not put large datasets in each
  reconciled view node.
- Define axes, native formatting, legends, labels, palette/stroke/fill tokens and
  tooltips, including non-color distinctions. Shared Cartesian layers provide
  useful custom combinations. Preserve the pinned families' applicable styling
  and plotting options while keeping the public API typed.
- Implement a documented exact/aggregation policy with bounded retained source
  and painted geometry. Line/area reduction must preserve extrema and gaps;
  bars/candles must not silently inherit a line-sampling policy. Selection of
  aggregated data must carry explicit source provenance. These rendering policy
  types still need implementation; the current data constructors do not aggregate.
- Rust owns layout, retained paint geometry, hit testing, hover and drag. Reuse
  prepared plans across idle frames and bound background jobs/caches. Data and
  label access cannot synchronously call OCaml from layout or paint.
- Provide stable, revision-checked semantic selections, keyboard navigation and
  meaningful accessibility/data-table alternatives. Native observations are
  asynchronous; changing data or closing a window cancels stale gestures/events.
- Build a polished public Chart Lab with every family, empty/degenerate cases,
  streaming updates and interaction. Validate actual macOS paint/input/AX behavior;
  Linux build/unit gates are required, GUI evidence remains separately recorded.
- Measure named-hardware dataset size, CPU/frame work, transport bytes and retained
  memory. Integrate a chart into OCH-29's broader application without replacing its
  custom-canvas and independent-extension requirements.

## Current local evidence

Core expect tests exercise all families' empty/degenerate input, 100,000 source
points, numeric/text/identity limits, explicit gaps, negative values, OHLC ordering,
radar domains and graph topology. An independent reachability oracle enumerates
all 4,096 directed graphs on four distinct nodes and agrees on the 543 DAGs.
The plotting compatibility tests retain all 12 upstream Sankey cases, including
layout chains, cycles, degenerate inputs, alignment and value/width behavior.
These are model/algorithm checks; native chart acceptance is still outstanding.
