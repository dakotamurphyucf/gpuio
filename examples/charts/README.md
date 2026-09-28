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

The example is still under OCH-40 development. A complete original-data alternative,
non-color plot identification and streaming measurements remain outstanding.
The accessible group and plotted-mark keys are not a complete data alternative. `Ready`
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
