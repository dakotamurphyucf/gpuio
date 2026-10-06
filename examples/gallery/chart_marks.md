# Build source-ID mark appearance presets

[chart_marks.ml](chart_marks.ml) implements the [interface](chart_marks.mli).
`A = Chart_appearance` and `D = Chart_data` shorten public API names. This helper
is pure: `all` orders seven choices, `label` names buttons, `configuration` builds
appearance from a choice/palette/dataset, `dots` chooses marker visibility and
`sampling` chooses geometry policy. Bonsai state and native source ownership live
in [Charts_page](charts_page.md), not in this module.

The caller uses `B = Bonsai.Cont` and `B.state Chart_marks.Default graph`: `graph` owns the
reactive choice, while the returned setter constructs effects that update it.
Pressing Styled markers executes `set_marks Markers`; the page's `let%arr` then
derives appearance from the current choice, palette and accepted dataset.
`configuration` returns an `A.t` description, not native geometry. The page passes
it to `Chart_style.create ~appearance`, applies `dots` to Cartesian options and
passes `sampling` to `V.chart`. Style construction resolves theme colors; Rust
owns prepared geometry, painting and hit testing. Merely constructing the setter
effect does not run it, and deriving a style does not publish a new dataset.
Native Ready acknowledges preparation; native `Selection_changed` is a separate
event updating the page's selected source target. The page's scope retires its
chart registration and native work when the preview leaves.

`series data` examines `D.Expert.contents`. Cartesian/categorical layers supply
their stable series IDs and first 32 point IDs; radar supplies series IDs and the
first 32 axis IDs. Pie, candles and Sankey return no entries. This bounded prefix
is not the first 32 visible or defined observations: missing points can occupy
positions, and sampling can omit a requested mark. No style override creates a
new observation. Datum identity is scoped to its series, never a label or index.

The seven presets use these public constructors:

- Default returns `A.empty`; ordinary chart styles remain inherited.
- Styled paths attaches `A.Path.create` to every extracted series: four-pixel
  solid accent stroke, a physical 180-degree accent fill fading alpha 0.65 to
  0.08, and Natural curve. All stacked-area series receive the same curve,
  preserving the shared-boundary requirement. Radar ignores curve but uses the
  path appearance; explicit fills replace ordinary area/radar opacity.
- Styled markers sets an eight-pixel accent marker with foreground border width
  three. The first two extracted IDs per series receive gold marker overrides
  with radii 12 and 18. Other marker fields inherit series settings.
- Base-to-tip bars uses muted-to-accent `Bar_fill.base_to_tip`, following the
  actual base and signed tip, including negative/stacked bars.
- Domain-colored bars uses accent-to-gold `Bar_fill.domain` over the prepared
  value domain, including stacking and aggregation.
- Value-colored bars uses `Bar_fill.values` anchored to values 0 and 20;
  values beyond the anchors use endpoint colors.
- Uniform aggregate colors starts with muted series bars, then supplies accent
  bar overrides for all extracted first-32 IDs. `sampling` explicitly requests
  `Chart_sampling.Bar.sum ~max_buckets:2`; all other presets use default sampling.

Bar series use physical top-left radius 16 and bottom-right radius 8, with other
corners zero. These corners stay physical through orientation changes and clamp
to the bar's bounds. Local background gradient angles also stay physical; signed,
domain and value ramps express value projection rather than a fixed screen angle.
Every nondefault extracted series explicitly gets an accent legend swatch, which
is separate from datum highlights and original source names.

`A.create ~aggregates:Uniform` permits a multi-observation bar override only when
all participating defined observations agree in effective appearance; otherwise
it falls back to series style. Missing categorical observations do not participate.
The first-32 bound can therefore cause a bucket containing later IDs to fall back.
Aggregation keeps contiguous source ranges, and inspection/original-data browsing
retains original IDs and values; the summed bar is not a replacement source row.
At most two buckets is a requested bound, also limited by plot extent. This policy
changes geometry explicitly rather than silently rewriting the dataset.

The [appearance interface](../../lib/core/chart_appearance.mli) limits collections
to 128 series and 1024 unique series/datum pairs. Corners are finite `[0, 32]`,
stroke widths `[0.5, 8]`, marker radii `[1, 24]` and borders `[0, 8]`, clamped to
radius. Value-ramp anchors must increase and be finite within ±1e100. The helper's
32-per-series prefix is not a proof that arbitrary many-series input meets the
1024-pair budget: editable/larger data should handle validation errors instead
of using `ok`. `dots` is true only for Styled markers and configures Cartesian
dots; the page retains the separate radar dots default of true. Geometry visibility
remains the master gate, including its isolated-point fallback; a visible override
does not enable suppressed dots. Hidden markers retain the default chart hit
radius; visible markers use their effective radius. Borders default to zero width
unless explicitly supplied.

Large markers can overlap nearby values or radar captions. The gallery's
Radar label gap 24 control demonstrates reserving more caption space; marker
radius does not automatically change the radar's data scale or label gap.

To adapt the helper, highlight an explicit stable ID instead of taking the first
two positions, or change the value ramp anchors to meaningful application bounds.
Keep a common effective curve for stacked areas. Extend Uniform buckets only when
all participating observations can be assigned consistently; never color an
aggregate solely from its first item. See the
[sampling contract](../../lib/core/chart_sampling.mli) and
[appearance design](../../docs/design/chart-mark-appearance.md). Integration is
under qualification; this source walkthrough establishes no GUI/platform acceptance.
Use the gallery commands in the page guide; this helper has no executable.
