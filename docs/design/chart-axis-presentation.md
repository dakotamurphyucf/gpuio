# Chart axis and grid presentation — OCH-41

Implementation contract, 2026-10-06, base `f5875f5`.
[Local qualification](../evidence/chart-axis-presentation-och41.md) is recorded separately. It addresses pinned `PlotAxis`/`AxisText`/`Grid` capabilities
with bounded declarative values, preserving the existing native axis repair.

## Public values

`Chart_style.create ?x_axis:Chart_axis.t ?y_axis:Chart_axis.t ?grid:Chart_grid.t`
resolves all optional colors through its theme. Axis presentation belongs with
resolved style, including preformatted tick text, rather than introducing theme
resolution or OCaml callbacks into `Chart_options`.

Existing `Chart_options.Axes.x/y` gate each data axis's line and labels. Each
`Chart_axis` independently controls line/label visibility beneath that gate.
`Axes.grid` remains the master grid switch, independent of axis visibility.
Defaults preserve current tick generation, formats, positions and gutters.
Cartesian axes transpose with orientation; candlestick axes stay vertical.
Other families retain but ignore axis presentation values.

An axis accepts an optional normalized perpendicular line position in [0,1]:
zero is the top for a horizontal line or left for a vertical line; one is
bottom/right. Omitted position retains the usual bottom/left line. Boundary
strokes sit inside the clip, preserving the scale-1 visibility repair. This
changes axis presentation, never source projection, bar baselines or domains.

Label side is Auto/Before/After. Before means above a horizontal line or left
of a vertical line; After means below/right. Auto chooses the nearer outside
edge (Before at normalized positions <=0.5, After otherwise). Alignment is
Auto/Left/Center/Right; Auto centers horizontal-axis captions and aligns vertical
ones away from their line. Gap defaults to existing 6/8 logical pixels;
width defaults to existing 80/68. Explicit gap is [0,64], width [8,256], font
size [8,32], line width [0.5,8]. Native fixed-height captions remain single-line
and ellipsize; dense explicit labels may overlap. Custom layouts reserve bounded
gutters on the selected sides and clip in tiny views. They do not promise full
label visibility for arbitrary fonts, sizes or crowded ticks.

Ticks are automatic unless an explicit list is provided. An optional per-axis
count [2,64] replaces the legacy shared count for automatic ticks only. Explicit
lists have at most 64 entries, with required text of 0..256 UTF-8 bytes excluding
ASCII controls. Empty text suppresses its caption, not its grid position.
Entries preserve caller order and may share positions; no interactive state is
attached to a tick. Per-entry color/font/alignment override axis defaults.

Tick positions have three validated forms:

- Value: finite numeric data coordinate in [-1e100,1e100]. Native domain mapping
  includes value-axis reversal and bar padding. Out-of-domain values are omitted;
  a constant domain accepts only its actual value. Values apply to numeric x or y;
  they are ignored on a categorical x-axis.
- Category: stable `Chart_data.Category_id.t`, projected at its category center.
  Unknown IDs are retained/ignored. These apply only to categorical x, including
  its horizontal orientation; they are ignored on numeric axes and y.
- Fraction: finite [0,1] along the physical axis, left-to-right or top-to-bottom.
  It bypasses data-domain/reversal mapping and works on either axis. This replaces
  layout-dependent pixel callbacks with a resize-stable placement option.

Mismatched/absent targets are ignored without changing source values. This permits
one persistent style across source-family transitions. Explicitly empty tick
lists clear automatic ticks. Applications can select/stride category IDs or format
values in OCaml before submitting the list; no native formatter closure crosses
the bridge. Tick labels never rename original data or legend entries.

`Chart_grid.create ?x ?y ?dashes ?width ?color` independently overrides grid
positions for either data axis; None follows that axis's final tick positions,
Some [] clears that dimension. Lists have at most 64 positions. Color inherits
the chart grid color. Width is [0.5,8]; empty dashes mean solid; otherwise 1..16
lengths in [0.5,128] alternate painted/gap spans, repeating odd lists over two
cycles and restarting at each line. Appearance also applies to radar grid lines;
radar ignores x/y position overrides. Every dash fragment counts against bounded
preparation (at most 16,384 painted grid fragments per plan); excessive geometry
fails RenderLimit, never silently substitutes a
solid line or truncates the pattern.

## Ownership and wire

All values are immutable, validated in Core and again on native decode. Source,
resolved style, projection and label metadata belong to one prepared snapshot.
Custom category lookup scans source categories once against a bounded requested-ID
set; it must not materialize unbounded per-tick data or make a callback to OCaml.
Unknown IDs, data replacement/reset, style/font changes and unmount follow existing
chart request/cancellation ownership. Decoration does not acquire input handlers.

Style schema advances from -3 to -4; options/view/data remain 9/-1/1. Old styles
are rejected explicitly. The standalone style envelope increases to 192 KiB to
cover independent Sankey/pie text budgets, both 64-entry tick lists, ordinal keys
and all bounded metadata. Decoder counts/text lengths are checked before reading;
retained vector/string allocations are charged. Rust stores axis/grid settings
behind boxes to avoid inflating every inline chart configuration.

## Required qualification

Paired bytes, schema rejection and all invalid/count/text/color/numeric bounds;
Core theme resolution; independent tick/axis/grid geometry and provenance for
numeric/categorical/constant/empty/mismatched sources and four orientations;
per-label font/alignment and bounded frame gutters; odd dash cycles and geometry
budget/cancellation; actual native text/line/dash pixels; public OCaml gallery
controls, raw selection/table preservation, themes/source/style transitions and
resource cleanup; fresh installed consumer and required platform checks.
This contract does not complete unrelated per-datum/rich-inspection or release
work. Linux desktop remains OCH-47; nongraphical Linux checks remain required.
