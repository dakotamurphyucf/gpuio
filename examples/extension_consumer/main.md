# Consume a packaged native counter through ordinary Bonsai views

[main.ml](main.ml) has no separate interface. Read `component` state/instance/event
handler, then top-level catalog check and window startup. `B = Bonsai.Cont` owns
reactive application values, `E = Bonsai.Effect` defers actions, `V` describes GPUIO
views, `App` owns runtime/windows and `Counter` is an independently packaged public
OCaml API. This consumer authors no Rust widget or private host implementation.

After [setup](../../docs/development.md), from the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/extension_consumer/main.exe -j 2
./scripts/gpuio exec dune exec examples/extension_consumer/main.exe
./scripts/gpuio exec dune exec examples/extension_consumer/main.exe -- --smoke
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example extension_consumer --run
```

The isolated consumer check stages installed public libraries and copies package/
toolchain inputs; omit `--run` for required build-only coverage. [README](README.md)
explains native smoke/occlusion and Linux GUI qualification limits. No build or
native run was performed for this documentation review. All values are fixtures.

## Graph state, property values and explicit commands

`B.state` starts observed value 7 and command sequence 1. `let%arr` combines state
and setters, deriving `Counter.Properties` with the observed value and step `1` and `Counter.instance` with
fixed generation `1` and an explicit command carrying the current sequence and value `12`. Constructors validate bounded
values/schema, per [counter interface](../extension_package/ocaml/gpuio_example_counter.mli).
The view describes a native instance; it does not install its factory each render.
Generation identifies replacement/reset, while sequence identifies an explicit
command on this instance. The button increments sequence to request another set value `12`;
ordinary rerenders retaining the same sequence do not create another command.
This tiny example does not bound button sequence exhaustion for an indefinitely
running production command source.

`on_event` handles native `Data` by setting observed OCaml value, `Failed` by raising
a typed diagnostic failure, `Mounted` with `Ignore` and `Command_completed` according
to smoke mode. It does not assign `12` on acknowledgement or assert that label became `12`.
The observed label may remain `7` until a `Data` event; command success and observed
property/application value are separate. `V.extension` handles native reconciliation,
input and asynchronous events, as described by [View](../../lib/core/view.mli).
`set_value`/`set_sequence` are deferred effects, not mutations during graph derivation.

Trace: native pointer/Space/Enter activation emits `Data` → callback returns setter →
Bonsai state updates → `let%arr` re-derives properties/observed label. Clicking OCaml's
Set button instead publishes a new sequenced command → native acknowledgement →
interactive mode keeps window open. Neither flow dynamically loads arbitrary code.

## Catalog, window and smoke boundary

Top-level `App.extension_catalog` forces the [generated backend](backend/backend.md)
and checks exact `Counter.schema` membership before App.run. `App.open_window` requests
initial focus unless --background, opens a 640 × 360 window and supplies the `component` graph.
App/window/reconciler own native instance lifetime; no application Eio producer
or manual native registration lives in this file. Window destruction drops its
native component, which stores no live OCaml values.

Smoke reacts to a `Command_completed` event by requesting a correlated render callback
then closing the window. It uses no physical input/pixel assertion and does not
compare the acknowledgement sequence or assert a native value readback. Missing
callbacks can prevent close; the external runner supplies its timeout. Background
occlusion may defer macOS frame delivery, so default smoke requests focus. A frame
callback establishes native rendering callback, not GPU completion/physical display.

For another component, select its public schema/factory in trusted static manifest,
[regenerate composition](backend/registration.md), then use a typed instance/event
handler. Keep native authoring in its package and design bounded command/generation
state for a richer consumer. Interactive keyboard/accessibility and isolated build
checks are different evidence; this source review adds no platform acceptance.
