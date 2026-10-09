# Chart Studio application walkthrough

[main.ml](main.ml) mounts one public GPUIO chart in a Bonsai application, with seven
family buttons, presentation controls, selection text and a data-update button.
The source/controller/view/startup responsibilities are local functions in this
single file. [gallery.ml](gallery.md) converts its numeric chooser into the shared
[fixture catalog](samples/gpuio_chart_samples.md). Read those pure helpers first,
then `dataset`/`load_family`, `on_event`, `component`, and the final window/scope block.
The [Dune executable stanza](dune) links Core, the samples, GPUIO's Bonsai/Eio layers,
Bonsai and Eio, and enables Jane Street/Bonsai PPX.

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/charts/main.exe
./scripts/gpuio exec _build/default/examples/charts/main.exe
# Optional background launch:
./scripts/gpuio exec _build/default/examples/charts/main.exe --background
# Optional native preparation/reset/cleanup diagnostic:
./scripts/gpuio exec _build/default/examples/charts/main.exe --self-test
# Request focus if the background diagnostic receives no render callback:
./scripts/gpuio exec _build/default/examples/charts/main.exe --self-test --foreground
```

No external files, service credentials or assets are needed; all observations are
mock data. The local toolchain prerequisites are in the
[development guide](../../docs/development.md). Ordinary launch requests focus;
`--background` or `--self-test` suppresses that request unless `--foreground` is
also supplied. Current release qualification is macOS-first. The
[README](README.md) links native test scripts and their permissions; this prose
review adds no graphical, keyboard, VoiceOver or Linux desktop acceptance.

## State and controller functions

`App.run` supplies the Eio environment and application. `B.Expert.Var` values hold
family index 0 (Line), Standard preset, false edge/light/monochrome flags, no chart
handle or selection, and the initial status. A Var is an application-owned mutable
input whose `value` can be read reactively by Bonsai; `get` reads it immediately and
`set` updates it on the UI domain. Plain refs hold the retained registration, latest
Ready/Failed observation, phase zero and diagnostic completion flag. None of these
refs should be changed from an Eio worker directly.

`dataset` chooses edge fixtures or the current preset. `load_family n` calls
`Registered.reset` on the existing registration, clears observation/selection and
sets family/status. Reset preserves the handle while retiring the old selection
epoch; it does not create a new native resource for each button click. `choose`
returns to Standard before loading. `set_preset` disables edge cases and maps Mixed,
Horizontal and Dense_legend to Line, Bar and Pie respectively. `toggle_data` changes
the fixture flag and chooses Standard for the current family. Light and monochrome
change presentation only. The fixture phase is retained across family changes.

`checked` raises a typed registration error and `ok` unwraps `Or_error.t`, a policy
suited to these checked fixtures. A production data-loading controller should
present recoverable validation/publication errors explicitly. The
[registration interface](../../lib/eio/chart.mli) distinguishes a local successful
`set` from native acceptance, exposes `error`, and documents coalescing and lifetime.

## Bonsai view and native state

`component _window _graph` closes over the variables created once by startup; it
creates no additional state. Its reactive boundary is:

```ocaml
let open B.Let_syntax in
let%arr family = B.Expert.Var.value family
and preset = B.Expert.Var.value preset
(* additional current inputs *)
in
(* derive views from the current inputs *)
```

`let%arr` combines changing Bonsai inputs into a derived result. When a variable
changes, this computation derives a new view. View construction does not itself
run button work: the local `button` helper wraps each callback with `E.of_thunk`,
creating a `Bonsai.Effect` that GPUIO executes when the native click arrives.
`V.row`, `V.column`, `V.text` and `V.button` compose the interface; `Style.create_exn`
and typed lengths/colors set layout and appearance.

Without a handle, the view displays Loading. With one, `Chart_options.create`
selects horizontal or vertical Cartesian orientation and a donut inner radius of
0.56; dense legends disable pie labels. `Chart_style.create` optionally uses a
single-color palette and resolves label/axis/selection colors. `Chart.Config.create`
combines these with the borrowed data handle; `V.chart` mounts it under the stable
key `main-chart`. It occupies 760×330 logical pixels. Layout styling is distinct
from chart geometry/options; light mode uses explicit example colors rather than
loading theme files. Public contracts are in
[chart.mli](../../lib/core/chart.mli),
[chart_options.mli](../../lib/core/chart_options.mli) and
[chart_style.mli](../../lib/core/chart_style.mli).

`on_event` returns an effect. For `Selection_changed`, it checks
`Registered.is_published`, reads `Registered.data` and resolves the semantic target
through `Gallery.describe_selection`. It stores description text rather than the
raw target. Native hover and arrow previews remain native-owned; committed events
cross asynchronously to OCaml. Ready stores metrics and updates status; Failed
stores the error and displays it. The phrase “native rendering” in the status
means a Ready observation, not a timestamp proving physical screen presentation.
The native chart also owns its original-data table, legend scroll and keyboard
inspection; the application does not rebuild a row widget for each original point.

For a concrete trace, press **Update data ↗** while Line is selected. Its effect
increments phase, clears the OCaml description and calls `Registered.set` with
fresh immutable data. IDs remain stable; an existing native singular selection may
survive, while this application explicitly clears its readout. Native publication
and geometry preparation complete asynchronously; Ready changes status, causing
`let%arr` to derive updated text. Click an Atlas point: native selection arrives at
`on_event`, source publication is checked, and the helper resolves the original
point's ID/span into the selected text. Clicking another family instead calls
`reset`, clearing the readout and retiring the prior selection epoch.

## Startup, scope and optional self-test

`App.open_window` mounts the component before data acquisition. `App.Window.scope`
provides the resource lifetime; `Scope.start` runs an Eio producer within it. The
producer obtains the clock from `env` and bounds setup/self-test work by a 60-second
`Eio.Time.with_timeout_exn`. The window itself remains interactive after ordinary
setup completes; the timeout is not a 60-second application lifetime.

`on_ui` creates an Eio promise, uses `Scope.Expert.enqueue` to schedule an effect on
the UI loop, and awaits its result in the producer. Its `ui` wrapper lifts ordinary
UI-domain work with `E.of_thunk`. `Registered.create` is run through `on_ui`; once
its first native publication completes, startup stores the registration and exposes
its borrowed handle to Bonsai. The ordinary startup ends there. The window's scope
continues to own the registration and releases it on cancellation. Closing the
window cancels scoped work and suppresses late completions; the
[scope contract](../../lib/eio/scope.mli) explains cancellation and domain ownership.
This example does no disk/network I/O and uses no external streaming producer.

Self-test additionally polls UI observations with `until`, sleeping 5 ms between
checks in the Eio fiber. `wait_ready` requires the expected `data_generation`,
raises on Failed, waits for initial generation 1, then chooses indices 1–6 and
requires successive generations. It prints `CHART_PUBLIC_READY` markers, requests
a native render callback with `App.Window.request_frame`, releases the registration,
removes the view handle, and waits until diagnostic chart count and byte count are
zero. It sets `completed`, prints `GPUIO_CHART_PUBLIC_OK` and shuts down in the
UI-domain `on_result` effect; the final assertion requires completion.

`Scope.Expert.enqueue`, `E.Expert.handle`, the diagnostic counters, polling and
log markers support this example's verification harness. They are not the first
application template for ordinary async data: prefer a scoped producer delivering
an `on_result` effect. The requested render callback proves native callback delivery,
not physical screen presentation or real keyboard/accessibility input. Self-test
visits families and cleanup; it does not cover every preset, edge case or pointer
interaction. Background occlusion can prevent callback progress.

For a small adaptation, change the chart's height in `V.chart` or the text/layout
colors without altering resource ownership. To replace mock data, validate inputs
with public `Chart_data` constructors and publish on the UI domain through the
same scoped registration; keep logical IDs stable and use `reset` for a semantic
replacement. Never retain its borrowed handle after release, access Bonsai from
an Eio producer directly, or interpret a successful `set`/Ready as physical display
acceptance. The [getting-started example](../getting_started/README.md) provides a
smaller ordinary application before adopting the diagnostic machinery here.
