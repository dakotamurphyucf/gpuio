# Slider values, previews and explicit commands

[main.ml](main.ml) is the slider executable: a horizontal range, horizontal
single value, vertical logarithmic value and vertical logarithmic range. The
[README](README.md) gives exact build/run commands and separate macOS diagnostics.
[dune](dune) gives this executable only module `main`, linking Core, GPUIO,
Bonsai/Eio adapters and `eio_main` with Jane Street/Bonsai PPX. No external data,
assets or service are needed. Use the isolated
[development toolchain](../../docs/development.md); native launch requires a
working desktop under the [platform policy](../../docs/platform-release-policy.md).

Read `domain` and the assertion helpers, `component` setup, the final view, then
the launcher. Return to the middle `B.Edge.on_change` block for `--self-test`.
`range` validates `Slider.Value.range`; `domain` validates bounds −2..8 and step
0.5. The [Numeric.Domain](../../lib/core/numeric.mli) contract uses bounded binary
floats, clamps to bounds and chooses min-anchored grid points; it does not promise
exact decimal arithmetic. `S.Value` distinguishes single from ordered range
values. The vertical examples use positive bounds 1..1000, required for
logarithmic mapping, with initial 32 and 10..100. Main range starts 2..7 and
horizontal single starts 3. See [Slider](../../lib/core/slider.mli).

`component` constructs a persistent Bonsai graph. `B.state` allocates application
visibility true, disabled/read-only false and status `Ready`; these are policy
and presentation state, not live slider values. Each returns a reactive value
plus a setter that creates an effect. `let%arr` reads current values to derive
configuration or a view; its `and` inputs are dependencies, not parallel tasks.
`B.return` makes fixed configurations reactive. `Controller.create` allocates
one native placement per example; use `Controller.view` once per controller.
Rust owns drag, thumb focus, preview and committed values. A snapshot's `value`
is current preview, equal to `committed` outside a drag. Observations never
implicitly replace native values.

The final derivation's `report` uses `E.Let_syntax`/`let%bind` to await a typed
command result and then returns a status setter effect. Reset range explicitly
calls `Controller.replace` with 0..6; Read state asks native code for a current
snapshot; Focus minimum targets `Lower`. Policy buttons update Bonsai config;
Unmount removes only the primary range placement. `mode_view` renders each other
controller plus observed values using `Option.value_map`, showing `Mounting`
before first observation. `View.column`/`row` and validated logical-pixel styles
supply layout. The primary range width is 320 pixels and other sliders use an
accent foreground; native code handles pointer/keyboard interaction.

Drag the upper range thumb for an end-to-end trace. Native input creates a drag
and updates its preview; ordered `Drag_started`, `Preview` and `Committed` events
and snapshots cross asynchronously to the OCaml controller (previews may
coalesce within a gesture). Bonsai recomputes observed value text, and GPUIO
submits that view back to native rendering. The application has not issued a
replacement in response. On release the native committed value catches up;
cancellation can restore it. Alternatively Reset range sends a correlated
explicit replacement, normalizes values, cancels a drag and displays its reply
revision. Admission/replies are separate from physical frame presentation.

`App.run` owns GPUI on the OS main thread and the OCaml Eio UI domain for graphs
and effects. Startup mounts a 720×720 window; ordinary launch starts no file,
network or timer task. Close force-closes with `App.Window.close`, cancelling the
window scope. For unsaved work use `request_close` and a decision handler; see
[App](../../lib/eio/app.mli). The fixed seed is reapplied on a new mount; native
drag/focus state and the old lease do not survive unmount.

## Optional bridge diagnostic

`_build/default/examples/numeric/main.exe --self-test` opens a native window and
prints `GPUIO_SLIDER_PUBLIC_OK` on success. `B.map` derives the primary observation;
`B.Edge.on_change` uses typed optional snapshot equality. `started` prevents
restarts and test-only `latest` retains the newly observed controller while the
sequence deliberately keeps the old lease. `B.Clock.sleep` waits 100 ms around
policy/mount changes. `E.Let_syntax` binds asynchronous commands; `expect`,
`assert_error` and `assert_value` fail the diagnostic on unexpected typed results.

It reads initial 2..7, performs guarded replacement 0..6, rejects an old revision,
rejects single/range mismatch (`Wrong_mode`) and `Single` focus on a range
(`Wrong_thumb`), focuses Lower and cancels a nonexistent drag without changing
revision. Disabled focus fails, while explicit replacement and read remain
available. Unmount/remount rejects old controllers as `Stale_slider`; fresh
placement is seeded 2..7 and an old snapshot cannot reset it. Commands after
force-close return `Closed`. These are native command/lifetime checks, distinct
from OS pointer/keyboard, accessibility, physical display or Linux GUI evidence.
The Rust `native_slider` suite and macOS `scripts/test_numeric.py` cover separate
native/foreground paths described in the README.

To change bounds, edit `domain` and validate all seeds and reset values against
the resulting configuration. Preserve range ordering and mounted single/range
mode; remount deliberately to change mode. For external updates use
`replace_if_unchanged` with an exact snapshot when newer user movement must win;
handle `Stale_revision` and `Stale_slider`. Store application values separately
if they must survive remount, and run I/O through explicit Eio capabilities rather
than in `let%arr`. Read [the controller interface](../../lib/eio/slider.mli) for
lease/revision and policy rules.
