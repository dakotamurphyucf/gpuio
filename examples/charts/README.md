# Chart Studio

A public Core/Bonsai/Eio application showing line, area, bar, pie/donut, radar,
candlestick and Sankey data through `Gpuio_bonsai.View.chart`.

```sh
./scripts/gpuio build examples/charts/main.exe
./_build/default/examples/charts/main.exe
# Background window, without requesting focus:
./_build/default/examples/charts/main.exe --background
# Visit every family, check native preparation/reset observations and cleanup:
./_build/default/examples/charts/main.exe --self-test
# If an occluded background window is not receiving frames:
./_build/default/examples/charts/main.exe --self-test --foreground
```

`Gallery` constructs typed immutable datasets. An Eio-scoped `Chart.create`
registration owns native data; the view borrows its handle. Selecting another
family resets the resource generation. **Update data** publishes another sample
for the current family (some sample families intentionally use fixed values).
Preparation happens on bounded Rust workers, outside the GPUI paint callback.

Native axis/family labels and scrollable legends use the displayed data snapshot.
Click or drag within the plot to commit a selection; hover remains native-local.
Arrows/Home/End preview plotted marks, Enter/Space commit, and Escape clears
(or cancels a drag). The OCaml readout receives typed semantic selections through
Bonsai/Eio. Ordinary updates retain singular native selection when still plotted;
the example clears its derived readout when it requests replacement data.

**View data**, or D while focused, opens a read-only native table of every original
value, including gaps and values omitted from the plot. Arrows/Home/End and
Page Up/Down browse; D, Escape or **Back to chart** returns. Browsing leaves plot
selection unchanged. At most ten rows are mounted even for large data. The table
uses current published data and remains usable when mesh preparation fails.

Choose **Edge cases** for a 100,000-point line with gaps, empty area data, signed
bars, a zero slice, reordered radar axes, a negative flat candle, and isolated/
zero-flow graph data. **Sample data** restores the ordinary gallery.

**Mixed layers** combines area/bar/line data; **Horizontal** compares grouped
bars, and **Dense legend** supplies 128 channels (updates add a channel up to
256). **Monochrome** and **Light theme** exercise non-color identification and
resolved styling, including the data companion. Multi-series plots repeat
numbered identifiers matching their legends; coincident values may overlap.

The example is still under OCH-40 development. The separate
[public streaming workload](../chart_stream/README.md) records large-data behavior;
broader-example integration and consolidated gates remain outstanding. `Ready`
means prepared; the self-test separately requests a native render callback,
which is not proof of physical screen presentation.

On macOS, `python3 scripts/test_charts.py` checks native axis/legend text through
accessibility and captures each family in a background window before closing it.
This requires Accessibility and Screen Recording access for the test runner;
pass `--foreground` when background windows do not receive frames. Use
`python3 scripts/test_charts.py --input` to activate the child, exercise real
AppKit keyboard selection in all seven families, and check pie hover/click/drag
and cancellation through the asynchronous OCaml callback. The script closes and
reaps its child window.

`python3 scripts/test_chart_data.py` activates the public app and exercises its
100,000-row original-data alternative through real macOS AX and keyboard input,
including every family's edge cases, paging, row focus and reset. It closes and
reaps its child window.

`python3 scripts/test_chart_visuals.py` checks mixed/horizontal/radar identifiers,
monochrome/light presentation and actual wheel scrolling in the dense legend.
It verifies offset retention across a 128-to-129-value update and reset behavior,
captures screenshots, then closes and reaps the application.
