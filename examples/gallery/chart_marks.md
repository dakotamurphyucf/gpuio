# Build source-ID mark appearance presets

[chart_marks.ml](chart_marks.ml) implements the [interface](chart_marks.mli).
`A = Chart_appearance` and `D = Chart_data` shorten public API names. This helper
is pure: `all` orders ten choices, `label` names buttons, `configuration` builds
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

The selected mark is a separate application observation. Charts_page creates
`selected = B.Expert.Var.create None`, reads it reactively through
`B.Expert.Var.value selected` in `let%arr`, and handles
`Selection_changed target` with `E.of_thunk (fun () -> B.Expert.Var.set selected target)`.
Here `target` is a `Chart.Selection.t option`; `None` represents an explicit
clear. `E.of_thunk` defers that update until the event effect executes. The page
uses the stored observation to derive its source-value description. The mark
buttons execute only `set_marks candidate`: style or sampling changes do not
rewrite this separately stored observation, and a Ready event updates only the
notice. Native current selection and the page's last committed observation can
therefore differ after a preparation change.

For example, select a Bar value with **Home**, then **Enter**, and choose
**Uniform aggregate colors**. The chooser changes `marks`, so the next `let%arr`
builds its two-bucket Sum sampling configuration, while the page retains its
previous committed selection description. Focus the chart and press **Home**
then **Enter** again: Home previews a mark under the current sampling policy;
Enter commits it, and the new `Selection_changed` effect replaces the stored
observation. A Sum bar selection carries the represented source span and Sum
aggregation, including when that span contains one point. Returning to a pattern
preset restores default sampling but likewise does not rewrite the last stored
observation until another selection event. See the
[selection contract](../../lib/core/chart_selection.mli). Choosing a different
chart mode or acquiring a fresh page source clears the page observation explicitly.

`series data` examines `D.Expert.contents`. Cartesian/categorical layers supply
their stable series IDs and first 32 point IDs; radar supplies series IDs and the
first 32 axis IDs. Pie, candles and Sankey return no entries. This bounded prefix
is not the first 32 visible or defined observations: missing points can occupy
positions, and sampling can omit a requested mark. No style override creates a
new observation. Datum identity is scoped to its series, never a label or index.

The ten presets use these public constructors:

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
- Slash pattern constructs `Background.pattern_slash accent ~width:2. ~interval:4.`;
  Checkerboard pattern constructs `Background.checkerboard accent ~size:8.`.
  Both attach that brush to filled paths and bars for every extracted series.
  The paths also receive the same four-logical-pixel solid accent stroke and
  Natural curve as Styled paths. Radar uses the fill/stroke while ignoring bars
  and curve; a line without an area has no filled region to display the pattern.
- Area baseline 20 attaches `A.Baseline.create 20. |> ok` through
  `A.Series.create ~area_baseline` to every extracted series. The baseline
  applies only to Cartesian/categorical area layers; lines, bars and radar
  ignore that field. This choice does not supply a path override, enable dots
  or change default sampling. It supplies no bar, marker or legend override,
  keeping those appearances unchanged while the area baseline moves.

In `configuration`, `pattern` is `Some brush` for the two pattern choices and
`None` otherwise. `Option.value pattern ~default:...` selects the path fill;
`A.Bar_fill.background` wraps the same brush for bars. The `?path` and `?bar`
arguments to `A.Series.create` are supplied through `Option.some_if`, so a
pattern choice supplies both applicable overrides without creating datum
overrides. Neither pattern choice enables Cartesian dots or requests aggregate
sampling. `Or_error.ok_exn`, locally named `ok`, unwraps the fixed valid brush
inputs; editable dimensions should retain and handle the constructor's error.

Pattern dimensions are **physical pixels**, unlike logical-pixel stroke widths,
marker radii and bar corners. The slash width 2 and interval 4, and checkerboard
square size 8, pass to GPUI's packed native pattern brush without logical-layout
scaling or rotation when chart orientation changes. GPUI quantizes these dimensions;
the slash brush is not a precise vector hatch. Checkerboard starts at the painted
shape's bounds. Slash gaps and alternating checkerboard cells are transparent,
revealing the backing beneath the mark; they do not receive a second fill color.
Explicit pattern fills replace ordinary area/radar opacity, as the gradient does.
To soften the colored portion, supply an accent with explicit alpha.

For a concrete interaction, choose an Area chart and press **Slash pattern**.
Charts_page executes `set_marks Slash_pattern`; its `let%arr` reads the updated
choice and calls this pure `configuration` with the current palette and dataset.
The resulting series-ID path/bar overrides enter `Chart_style.create ~appearance`,
then the chart config. Native paint uses the slash fill on the area's existing
geometry. Selecting **Checkerboard pattern** replaces the brush through the same
flow. Source IDs, values and publication revision remain unchanged, and committed
selection continues through the page's existing event handler.

For the baseline flow, choose **Area**, then press **Area baseline 20**.
The button executes `set_marks Raised_area`; the caller's `let%arr` reads that
reactive choice and calls `configuration`. Its `Option.some_if` supplies the
validated baseline to each `A.Series.create`, then `A.create` combines the
series entries. Charts_page passes that appearance through `Chart_style.create`
and `Chart.Config.create` to the native chart. No Bonsai state is created here,
and this presentation change does not publish data or rewrite the page's stored
selection observation.

The number 20 is in **data units**, unlike either logical chart dimensions or
physical-pixel pattern dimensions. It participates in the prepared value domain.
A grouped area fills between 20 and each unchanged raw source value: a value 12
remains 12 and fills down from 20 to 12, rather than becoming 32. Stacked areas
instead interpret values as contributions: a common baseline offsets both
cumulative bounds. For example, successive positive contributions 5 and 7 at
baseline 20 occupy bounds 20–25 and 25–32. Inspection retains the original
contributions and IDs while its stacked bounds include that offset. Missing
observations still break runs.

Every participating stacked area must have the same effective baseline;
omission means zero. A partial override to 20 beside an omitted baseline therefore
fails native preparation with `Chart.Error.Invalid_config`. The helper applies
20 to every extracted series, preserving the common-baseline rule. When adapting
it, keep that agreement as well as the common effective curve requirement.
`Baseline.create` accepts only finite values within ±1e100 and returns
`Or_error.t`; handle editable-input errors before installing the appearance.
The baseline is a whole-series setting, not a datum override or a dense source
channel. [Local baseline qualification](../../docs/evidence/area-baselines-och41.md)
records codec, native unit/GPU and root/installed gallery behavior. Broader
platform/release acceptance remains separate.

Bar series use physical top-left radius 16 and bottom-right radius 8, with other
corners zero. These corners stay physical through orientation changes and clamp
to the bar's bounds. Local background gradient angles also stay physical; signed,
domain and value ramps express value projection rather than a fixed screen angle.
Every nondefault preset except Area baseline 20 explicitly gives extracted
series an accent legend swatch, which
is separate from datum highlights and original source names.
The pattern color is that same palette accent, but a legend swatch remains a
single explicit color rather than the patterned fill. Background colors may be
theme tokens; `Chart_style.create` resolves them before native preparation, so
rebuild the appearance/style when changing themes. This module reads the supplied
palette and owns no theme state.

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
The [background interface](../../lib/core/background.mli) separately requires
finite pattern width, interval and size in `[0.5, 64]` physical pixels. Patterns
are series brushes, so their coverage is not limited to the helper's first 32
datum IDs. That prefix still bounds marker highlights and Uniform buckets;
the public appearance collections remain sparse overrides, not a dense per-record
styling channel for a large source.

Large markers can overlap nearby values or radar captions. The gallery's
Radar label gap 24 control demonstrates reserving more caption space; marker
radius does not automatically change the radar's data scale or label gap.

To adapt the helper, highlight an explicit stable ID instead of taking the first
two positions, or change the value ramp anchors to meaningful application bounds.
Keep a common effective curve for stacked areas. Extend Uniform buckets only when
all participating observations can be assigned consistently; never color an
aggregate solely from its first item. See the
[sampling contract](../../lib/core/chart_sampling.mli) and
[appearance design](../../docs/design/chart-mark-appearance.md).
[Recorded local qualification](../../docs/evidence/native-pattern-brushes-och41.md)
covers ordinary-View GPU pattern pixels and root/installed gallery actions,
themes, bar directions and cleanup. This walkthrough itself is a source guide;
it does not establish broader GUI/platform acceptance.
Use the gallery commands in the page guide; this helper has no executable.
