# How `main.ml` validates chart registration without rendering

[README](README.md) · [Source](main.ml) · [Chart adapter](../../lib/eio/chart.mli)

This executable runs a finite resource integration check every time. It mounts
no chart view, opens no window, and creates no Bonsai graph. `App.run` with
`exit_on_last_window:false` owns the backend and Eio UI domain until explicit
shutdown. A root-scoped task has a 60-second timeout.

`Chart` abbreviates `Gpuio_eio.Chart`, `Data` is validated immutable chart data,
`Wire` is the raw chart resource protocol, and `Effect` means `Bonsai.Effect`.
`on_ui` enters effects through `Scope.Expert.enqueue`, maps their result to an
Eio promise, and awaits it. `ui` wraps a thunk; `request` invokes App.Expert.chart;
`ack` requires a typed Ack response. There is no `let%arr` or view reducer here.

`dataset` constructs one stable series with positive datum IDs, increasing X,
and sine Y values. Every 23rd point has `None`, an explicit gap rather than NaN.
Point/Series/line constructors validate values and bounds. A child data scope
owns the 100,000-point registration. Successful `Chart.create` means first native
publication, not prepared geometry or paint.

In one UI turn, the script submits 1,000 four-point desired updates, then a reset
to the large dataset. The scheduler coalesces pending desires; `wait_published`
polls in 5 ms intervals and raises adapter errors. It checks the accepted data
matches the final large dataset and only one chart registration remains.
Reset advances the selection/data generation without changing the borrowed handle.

The diagnostic then deliberately crosses into raw transport: Begin names base 2,
revision 3, generation 2; Chunk sends malformed bytes; Publish must return
Invalid_data. Abort retires the attempt. Reusing base 2 successfully proves the
failure did not change accepted data. This exact revision assumption is a test
probe, not a production API for coordinating chart updates.

Cancelling the child marks registration released; the task waits for registry
cleanup, then requires raw Release of the old source to return Stale_handle.
It cycles 270 create/release pairs to test slot reuse beyond live registration
capacity. Occupying 63 raw request lanes must reject another raw request while
a scoped adapter create progresses through the reserved lane. Final diagnostics
require zero source charges, commits, and rendered frames.

During shutdown, eight queued Create requests and one post-shutdown Create must
all complete Closed; the final assertion checks nine completions. Success prints
`GPUIO_CHART_UPLOAD_OK`. This is transport/cancellation/revision evidence, not
chart sampling, selection, accessibility, pixels, or desktop qualification.

From the repository root in the [repository environment](../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/chart_upload/main.exe
_build/default/examples/chart_upload/main.exe
```

No flag is needed. The program still starts a native backend, so lack of windows
is not a general headless execution promise. No filesystem/network capability
or external data is needed. [Dune](dune) enables Jane Street PPX for typed error
sexps; Bonsai is used for effects, not graph syntax.

For a UI application, keep registration in the intended scope, handle `error`
separately from local `set` admission, and borrow its handle into a chart view.
Preserve stable data IDs and check publication revision on observations. Prefer
public adapter methods over raw IDs and revision arithmetic; see the
[rendered streaming workload](../chart_stream/main.md) for preparation/frame checks.
