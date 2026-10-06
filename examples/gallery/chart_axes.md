# Construct pure axis and grid presets

[chart_axes.ml](chart_axes.ml) implements the [interface](chart_axes.mli) used by
[Charts_page](charts_page.md). It has no independent Bonsai graph, I/O or native
resource ownership. `t` is a five-case presentation choice; `all` orders the
buttons, `label` names them, and `configuration t palette` returns immutable
`Chart_axis.t * Chart_axis.t * Chart_grid.t` values. Its caller owns reactive state.

Default returns all three public defaults. The other presets share these values:

- X ticks at physical fractions 0, 0.5 and 1, captioned Start, Physical midpoint
  and End, with Left/Center/Right alignment, accent color and 13-pixel font.
- X line width 2, accent line color, label width 120 and gap 8 logical pixels.
- Y automatic ticks with count 4, 12-pixel font and foreground line/label colors.
- Grid dashes `[8.; 4.; 2.]` in muted color; omitted positions follow final ticks.

Styled puts the x line at perpendicular position 0 and y at 1. Floating uses 0.5
for both. Axis labels only disables lines; Axis lines only disables labels. Both
keep grid strokes independent. The `equal` helper compares the typed preset;
`Or_error.ok_exn` unwraps known-valid fixtures, not arbitrary user input.

`Chart_axis.Tick_position.fraction` means left-to-right or top-to-bottom along
the physical axis, independent of numeric reversal. It is not a source value.
`Chart_axis.create ~position` instead positions the line *perpendicular* to itself:
0 is top/left, 1 bottom/right. Neither moves data baselines or changes domains.
For data coordinates use `Tick_position.value`; for a categorical x center use
`Tick_position.category` with a stable category ID. Out-of-domain numeric values,
unknown category IDs and mismatched target kinds are ignored. Cartesian axes
transpose with orientation; candlestick axes remain vertical.

The [axis interface](../../lib/core/chart_axis.mli) bounds explicit tick lists to
64 entries, each caption to 256 UTF-8 bytes without ASCII controls, automatic
counts to `[2, 64]`, font sizes to `[8, 32]`, label widths to `[8, 256]`, gaps to
`[0, 64]` and line widths to `[0.5, 8]`. Empty tick text hides its caption but
retains its grid position; an explicit empty list removes automatic ticks.
Numeric coordinates must be finite in `[-1e100, 1e100]`; fractions/line positions
are finite in `[0, 1]`. Defaults preserve ordinary placement and formatting.

The [grid interface](../../lib/core/chart_grid.mli) permits at most 64 positions
per dimension; empty lists clear that dimension. Width is `[0.5, 8]`; dashes are
solid when empty, otherwise 1–16 lengths in `[0.5, 128]`. This odd three-length
pattern repeats over two cycles, alternating painted/gap spans and restarting
painted on each line. Excessive native dash geometry fails RenderLimit. Radar
uses grid appearance but ignores its position overrides. Options' x/y/grid
visibility flags remain master gates. Captions can ellipsize, clip or overlap
in small/dense views; these presets are not a collision-avoidance API.

To adapt Styled, replace the midpoint fraction with a validated numeric value
and preformatted text to annotate a known data coordinate. Keep a fraction if
resize-stable physical placement is intended. Handle errors for editable input.
The [design contract](../../docs/design/chart-axis-presentation.md) describes
native preparation and qualification still required; this review adds no acceptance.
Use the gallery build/run commands in the page walkthrough, then choose a
Cartesian or Candlestick mode and a preset; this helper has no executable.
