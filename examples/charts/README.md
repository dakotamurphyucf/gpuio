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
```

`Gallery` constructs typed immutable datasets. An Eio-scoped `Chart.create`
registration owns native data; the view borrows its handle. Selecting another
family resets the resource generation. **Update data** publishes another sample
for the current family (some sample families intentionally use fixed values).
Preparation happens on bounded Rust workers, outside the GPUI paint callback.

The example is still under OCH-40 development. Native chart labels, legends,
hover tooltips, semantic selection, keyboard/data-table alternatives and the
streaming performance report are subsequent work. The current accessible group
name identifies the chart, but is not a complete data alternative. `Ready`
means prepared; the self-test separately requests a native render callback,
which is not proof of physical screen presentation.
