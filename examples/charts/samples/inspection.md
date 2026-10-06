# Chart inspection presets

[inspection.ml](inspection.ml) is a pure configuration example. Its
[interface](inspection.mli) exposes a small preset type, labels and a function
returning public `Gpuio.Chart_inspection.t` values. It creates no Bonsai graph and
owns no native resource or I/O task.

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
./scripts/gpuio exec _build/default/examples/gallery/main.exe
```

Choose Charts & data, then one of the inspection buttons. Focus the chart and
use Home/arrows to preview a value; Enter commits selection. Try Default
inspection, Vertical crosshair, Horizontal band, Anchored details, Cursor details,
Marker only and Partial guides.
The data button remains available in every preset. Current qualification is
macOS-first; Linux compilation alone does not establish desktop behavior.

Read `type t`, `label`, then `config`. Default uses the library defaults. Vertical
adds a dashed crosshair. Band shows how solid thickness creates a translucent
highlight. Anchored and Cursor share a smaller card, full crosshair and outlined
marker; only their card placement differs.
Marker only hides the card and status glyph while preserving the source value
selection. `all` supplies chooser order, `label` supplies its captions and `config`
constructs the selected immutable value. `[@@deriving equal]` generates typed
equality used to highlight the selected button. `Chart_inspection.create`
combines `Card.create`, `Crosshair.create` and `Marker.create` results;
`Or_error.ok_exn` is used only for known fixture inputs. All dimensions/colors
are validated public constructor inputs.

The [gallery page](../../gallery/charts_page.ml) owns a Bonsai state value for the
selected preset. `B = Bonsai.Cont`; `B.state Samples.Inspection.Default graph`
creates a reactive model in the page graph and returns its current value plus
`set_inspection`. Calling the setter constructs an effect; the button executes
it, and `let%arr` derives the style from the latest choice and palette. The resulting
`Chart_style.create ~inspection` changes the view configuration; native workers
prepare a matching immutable result. The Ready notice acknowledges preparation,
not physical display presentation. Hover and keyboard previews remain native;
only committed selection crosses back to the page's OCaml selection state.

This configuration does not perform Eio work. The page separately owns its
scoped chart data registration; changing inspection neither republishes data nor
moves ownership into a row. Leaving the page retires the view and its native work
through the existing scope. See the [page walkthrough](../../gallery/charts_page.md)
for source publication, selection identity and teardown.

`Crosshair.create` accepts independent vertical/horizontal spans as well as axis,
pattern, thickness and color. Omitted spans keep full prepared plot extents.
The Horizontal band remains 12 logical pixels thick with translucent accent color;
its thickness does not shorten the guide. Thickness is bounded to `[0.5, 64]`
and clips in narrow plots. Partial guides is the separate span example below.

To make Anchored use a larger readable card, change `width`, `font_size` and
`line_height` in that branch. Keep line height at least font size. To create a
horizontal dashed guide, combine Axis.Horizontal with Pattern.Dashed. Theme
color tokens are resolved by the style constructor, so rebuild the style when
the theme changes. [The inspection contract](../../../docs/design/chart-inspection.md)
explains clipping, accessibility and remaining presentation limits.

For a concrete trace, click **Anchored details**: `set_inspection Anchored` is the
effect constructed by the setter returned by `B.state`. It changes that reactive
value, and the page's `let%arr` recomputes `Samples.Inspection.config inspection` and the chart
style. The native inspection now uses a 180-pixel card, a both-axis solid crosshair
and a 24-pixel outlined marker. Moving the pointer updates the native preview
without an OCaml state update; committing a value emits `Selection_changed`,
which the page stores and resolves against published source data. This preset
does not change the value, ID or data revision. Constructors and their bounds
are documented in [chart_inspection.mli](../../../lib/core/chart_inspection.mli)
and style composition in [chart_style.mli](../../../lib/core/chart_style.mli).
Existing [inspection evidence](../../../docs/evidence/chart-inspection-och41.md)
records macOS validation; these edits add documentation review only.

## Cursor details

`Cursor` is a pure preset alongside `Anchored` in `config`. Their shared branch
uses `I.Card.create ~placement:(if equal t Cursor then Cursor else Anchor)`;
`I` aliases `Chart_inspection`. Both keep width 180, gap 12, padding/radius 10,
one-pixel accent border, dark backing and light text. The both-axis solid
crosshair and 24-pixel outlined marker remain anchored to the inspected data.

Click Cursor details → the page executes `set_inspection Cursor` → Bonsai updates
the reactive preset → `let%arr` rebuilds its inspection style → native preparation
adopts Cursor placement. Moving within the same mark then moves the card beside
the latest native pointer position, without a hover roundtrip to OCaml or a new
source publication. Gap, edge flipping and clipping still bound placement.
Keyboard inspection, pointer departure or capture cancellation use Anchor instead;
a retained committed selection therefore keeps its data-anchored card after leave.
Focus loss, disabling, source replacement and retirement clear remembered pointer
state. Release outside the plot still does not commit selection.

To adapt the two presets independently, split their shared branch and change the
Cursor card gap while preserving the marker/crosshair settings. Card gap must
remain in `[0, 64]` logical pixels; changing card placement does not change source
values, selection IDs or ownership. See the
[cursor design](../../../docs/design/chart-cursor-inspection.md). The [cursor-specific evidence](../../../docs/evidence/chart-cursor-inspection-och41.md)
records actual native pointer pixels and root/installed gallery checks; it does
not establish wider platform or release acceptance.

## Partial guides

`all` now orders seven presets. `config Partial_guides` builds `I.create` with
`I.Crosshair.create ~axis:Both ~pattern:Solid ~thickness:4. ~color:accent`.
`I.Span.fraction ~start:0.25 ~length:0.5` supplies the vertical extent: the middle
half of plot height, from 25% to 75%. `I.Span.pixels ~start:24. ~length:120.`
supplies the horizontal extent: logical-pixel positions 24 through 144 from the
plot's left edge. Both validated results are unwrapped with the fixture's `ok`.
Card and marker arguments are omitted, retaining their defaults independently.

Span coordinates are physical: vertical runs top-to-bottom, horizontal left-to-right,
regardless of numeric reversal or chart orientation. The perpendicular coordinate
still comes from the inspected mark. Native layout intersects each requested
`[start, start + length]` with its plot extent; an empty intersection draws no
guide. This applies to solid bands and dashed patterns too. Fractions follow
resize without a source publication or an OCaml measurement callback; pixel
lengths remain literal logical pixels, subject to clipping.

Click Partial guides → execute `set_inspection Partial_guides` → Bonsai updates
its preset → `let%arr` calls the pure `config` and rebuilds resolved chart style
→ native preparation/inspection applies the new guide extents. Hover stays native;
selection still uses original source IDs/values. Spans do not reposition the card,
move the marker or change the selected mark. Ready acknowledges preparation, not
physical presentation or platform acceptance.

To adapt it, use `Span.fraction ~start:0.1 ~length:0.8` for a resize-relative
horizontal guide, or `Span.full` to restore one complete extent. Pixel starts
must be finite in `[-32768, 32768]`, lengths in `[0, 65536]`; fractional starts
in `[-1, 1]`, lengths in `[0, 2]`. Handle constructor errors for editable values.
See the [span contract](../../../docs/design/chart-guide-spans.md). The [scoped evidence](../../../docs/evidence/chart-guide-spans-och41.md) records
local pixel, codec and root/installed-gallery checks; broader platform acceptance
remains separate.
