# Separator preview walkthrough

[separator_preview.ml](separator_preview.ml) has no separate interface. [pages.ml](pages.ml) mounts `component palette graph` on **Presentation**. Use the [gallery development commands](../../docs/development.md); this component has no standalone executable or self-test.

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
./scripts/gpuio exec _build/default/examples/gallery/main.exe
```

The [gallery README](README.md) records prerequisites and platform limits.
This source walkthrough adds no native input, VoiceOver or platform acceptance.

Eight `B.toggle` calls allocate independent Bonsai booleans. Only `labelled` starts true; orientation, dash pattern, narrow frame, custom colors, long label, bounded label and the final checkbox start false. `let%arr` reads these values and the palette to derive view descriptions. The local `checkbox` helper maps a boolean to `Checked`/`Unchecked` and attaches its toggle effect. Clicking runs that effect, and Bonsai recomputes the view.

[`Presentation.Separator.create`](../../lib/core/presentation.mli) receives the stable key `separator`, an explicit horizontal/vertical axis and solid/dashed pattern. It draws a decorative line with an optional centered, wrapping label. Horizontal separators fill available width; vertical separators need a bounded parent height. Here the surrounding frame supplies height 180 and width 360 or 200 pixels, with centered children. An unlabelled separator uses one logical pixel on its cross axis.

`labelled` controls whether a label exists, and `long_label` chooses the short “Continue · 世界” or longer multilingual text. `custom` supplies the accent line color and an accent, 16-pixel label style. The default label background uses the appearance’s surface color; adapt that background if you place the separator on another surface. Dashed selects the public pinned border pattern, not a configurable dash array.

Despite the local name `clipped`, **Bound separator label** only adds a maximum cross-axis dimension: `Max_width 96` when vertical, or `Max_height 32` when horizontal. It does not set an explicit overflow-clipping style here. The separator is given role `Separator` and name “Rich separator preview”; the frame is a named group. There is no separator action or focus target.

Turn on **Long separator label**, then **Narrow separator** to exercise wrapping. Turn on **Vertical separator** to reuse the 180-pixel bounded height, and **Bound separator label** to constrain its width. The two readouts report chosen values, not measured native geometry. **Keep separator updates** changes only its own `checked` state; it does not enable persistence, freeze updates or alter the separator. It demonstrates an independent checkbox in the same component.

This is a reactive composition around a stateless presentation API. GPUIO owns native layout and rendering; there are no borrowed resources, Eio tasks or disposal callbacks. To adapt it, give vertical layouts a real height constraint, retain meaningful labels where needed, and map application choices to axis/pattern/style. Remove or rename the independent checkbox if its demo wording would imply behavior your application does not implement.
