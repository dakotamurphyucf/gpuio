# Deriving a native progress view from a small model

[main.ml](main.ml) shows empty, indeterminate, halfway and complete progress.
Press Next stage to cycle them. “Download progress” is an accessible demo label;
there is no download or external service. The [README](README.md) contains exact
build/run commands. Use the checkout's isolated
[development toolchain](../../docs/development.md). Native execution needs a
working desktop; no assets or file/network configuration are required. The
[platform policy](../../docs/platform-release-policy.md) separates the macOS-first
release from required Linux builds and deferred Linux desktop qualification.

Read `component` first, then the ordinary startup in `let ()`, and finally the
`if self_test` branch. [dune](dune) links Core, GPUIO, its Bonsai/Eio adapters,
Bonsai and `eio_main`; `ppx_jane` and `bonsai.ppx_bonsai` enable reactive syntax.
`B` aliases `Bonsai.Cont`, `E` aliases `Bonsai.Effect`, `Ui_view` names GPUIO view
construction and `Progress` names the validated value/configuration contracts.

`component ~phase ~observed _window graph` constructs a persistent Bonsai graph,
a network of state and computations. `B.state 0 graph` allocates the interactive
step once and returns its reactive value and a setter. The setter creates a
deferred effect; constructing that effect is separate from executing it.
`phase` is an external `B.Expert.Var` used for diagnostic control. It starts `-1`,
meaning use interactive `step`; nonnegative phases override the interactive
value. `observed`, initially `-2`, records when Bonsai has seen the supplied
phase. `B.Edge.on_change ~equal:Int.equal` registers that observation callback,
which returns `E.of_thunk` to defer the ref update to effect execution.

```ocaml
let%arr step = step
and set_step = set_step
and phase = phase in
let current = if phase < 0 then step else phase in
```

`let%arr` reads ordinary current values from reactive inputs and derives a
reactive view. `and` supplies dependencies, not parallel threads. Changing step
or phase reruns this derivation without reallocating its model. The Next stage
callback returns `set_step ((step + 1) % 4)`, so the interactive sequence remains
in `0..3`. The diagnostic uses `4` solely to remove the progress view.

| Current stage | Derived value | Visible result |
| --- | --- | --- |
| 0 | determinate 0.0 | Empty bar |
| 1 | `Progress.Value.indeterminate` | Native indeterminate indicator |
| 2 | determinate 0.5 | Halfway bar |
| 3 | determinate 1.0 | Complete bar |
| 4 | view omitted | Progress removed (diagnostic only) |

The determinate expression clamps integer `current - 1` to `0..2`, converts it
with `Float.of_int` and divides by two. This is application computation; the
[Progress.Value.determinate](../../lib/core/progress.mli) constructor itself
rejects fractions outside `[0,1]` and nonfinite values. `Progress.Config.create`
validates the accessible label and bundles the value. `Or_error.ok_exn` treats
failure of these fixed demo definitions as a programming error; validate and
handle external progress data intentionally.

`Style.create_exn` gives the indicator a 360×12 logical-pixel size and blue
foreground. `Ui_view.column` adds 24-pixel padding and 16-pixel gaps around text,
button and `Ui_view.progress ~style ~config ()`. GPUIO owns these presentation
APIs; Bonsai owns dependencies and actions. No tween transition is supplied,
and the application generates no per-animation-frame setter or polling loop.
Native rendering owns indeterminate presentation under the application motion
policy. The view uses neither editor state nor uploaded resources or stable
application keys; its only lifetime transition is diagnostic removal.

For a concrete interaction, click Next stage when halfway is displayed. Native
button handling sends an action asynchronously to the OCaml UI domain. Its
setter effect updates `step` from 2 to 3. Bonsai derives current 3 and fraction
1.0, builds the new progress configuration/view, and GPUIO submits it to native
rendering. Native transaction admission is distinct from the screen presenting
a frame. This action changes demo state without initiating work or announcing
an external download completion.

## Runtime and optional diagnostic

`App.run` owns GPUI on the OS main thread and one OCaml Eio UI domain for startup,
graph computation and effects. `App.open_window` mounts the component in a
600×260 logical-pixel window. Ordinary launch starts no Eio task; the user closes
the native window when finished. Default runtime exit occurs after the final
window closes, with scope/controller cleanup. See the
[App contract](../../lib/eio/app.mli).

`--self-test` is parsed by typed `String.equal` on `Sys.get_argv`. It starts one
`Scope.start (App.scope app)` producer with the explicit environment clock.
A 15-second `Eio.Time.with_timeout_exn` bounds the whole sequence. Its local
`rendered` helper sets external phase, waits in 5-ms clock intervals until
`B.Edge.on_change` updates `observed`, then requests a native render callback.
`Eio.Promise.create` bridges that callback into an awaited revision;
`E.of_thunk` resolves it when the callback effect runs. The helper runs for
phases 0, 1, 2, 3 and 4. Typed `Int64` comparisons assert increasing revision
numbers across initial mount, value changes and removal.

`on_result` returns an effect on the UI loop, raises on diagnostic failure,
marks completion and force-closes the window via `App.Window.close`. After
runtime exit, an assertion checks completion and success prints
`GPUIO_PROGRESS_PUBLIC_OK` with the scenario summary. Scope cancellation
cancels descendants and suppresses queued result delivery; the
[scope contract](../../lib/eio/scope.mli) explains producer/error ownership.
No task accesses Bonsai from another domain here.

These render callbacks observe native rendering, not physical display or frame
latency. Visibility can defer callbacks; the timeout is diagnostic policy.
The script has no real foreground input, accessibility or Linux desktop
qualification step. Reading this walkthrough does not execute its diagnostic.
The wait/ref/Expert.Var machinery is optional test support, not a download
implementation template.

To adapt the bar to actual work, compute `completed_bytes / total_bytes` only
with a known positive total and supply a finite fraction in `[0,1]`; use
indeterminate when total size is unknown. Keep application work/data outside
view derivation and run I/O through explicit Eio capabilities in an appropriate
application or window scope. Deliver bounded progress updates on the UI domain;
do not mutate Bonsai from a worker domain or perform I/O inside `let%arr`.
Retain clear task completion/cancellation state rather than treating fraction
1.0 or a render acknowledgement as proof of successful external work.
