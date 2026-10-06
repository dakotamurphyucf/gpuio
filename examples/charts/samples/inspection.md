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
inspection, Vertical crosshair, Horizontal band, Anchored details and Marker only.
The data button remains available in every preset. Current qualification is
macOS-first; Linux compilation alone does not establish desktop behavior.

Read `type t`, `label`, then `config`. Default uses the library defaults. Vertical
adds a dashed crosshair. Band shows how solid thickness creates a translucent
highlight. Anchored combines a smaller card, full crosshair and outlined marker.
Marker only hides the card and status glyph while preserving the source value
selection. `all` supplies chooser order, `label` supplies its captions and `config`
constructs the selected immutable value. `[@@deriving equal]` generates typed
equality used to highlight the selected button. `Chart_inspection.create`
combines `Card.create`, `Crosshair.create` and `Marker.create` results;
`Or_error.ok_exn` is used only for known fixture inputs. All dimensions/colors
are validated public constructor inputs.

The [gallery page](../../gallery/charts_page.ml) owns a Bonsai state value for the
selected preset. Clicking a button applies its state effect. The resulting
`Chart_style.create ~inspection` changes the view configuration; native workers
prepare a matching immutable result. The Ready notice acknowledges preparation,
not physical display presentation. Hover and keyboard previews remain native;
only committed selection crosses back to the page's OCaml selection state.

This configuration does not perform Eio work. The page separately owns its
scoped chart data registration; changing inspection neither republishes data nor
moves ownership into a row. Leaving the page retires the view and its native work
through the existing scope. See the [page walkthrough](../../gallery/charts_page.md)
for source publication, selection identity and teardown.

To make Anchored use a larger readable card, change `width`, `font_size` and
`line_height` in that branch. Keep line height at least font size. To create a
horizontal dashed guide, combine Axis.Horizontal with Pattern.Dashed. Theme
color tokens are resolved by the style constructor, so rebuild the style when
the theme changes. [The inspection contract](../../../docs/design/chart-inspection.md)
explains clipping, accessibility and remaining presentation limits.

For a concrete trace, click **Anchored details**: `set_inspection Anchored` is the
Bonsai effect returned by `B.state`. It changes that reactive value, and the
page's `let%arr` recomputes `Samples.Inspection.config inspection` and the chart
style. The native inspection now uses a 180-pixel card, a both-axis solid crosshair
and a 24-pixel outlined marker. Moving the pointer updates the native preview
without an OCaml state update; committing a value emits `Selection_changed`,
which the page stores and resolves against published source data. This preset
does not change the value, ID or data revision. Constructors and their bounds
are documented in [chart_inspection.mli](../../../lib/core/chart_inspection.mli)
and style composition in [chart_style.mli](../../../lib/core/chart_style.mli).
Existing [inspection evidence](../../../docs/evidence/chart-inspection-och41.md)
records macOS validation; these edits add documentation review only.
