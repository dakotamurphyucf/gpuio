# A packaged native counter driven by a Bonsai state machine

[extensions_page.ml](extensions_page.ml) and [extensions_page.mli](extensions_page.mli)
show the public extension boundary using the independently packaged counter.
Read aliases, state_machine0/deactivation lifecycle, Counter.instance, event
callback and controls. `B = Bonsai.Cont` owns reactive state; `V` describes GPUIO
views; `State` is the pure [Extension_state](model/extension_state.md) reducer.
The package owns drawing/native input/accessibility, not this page's model.

After [setup](../../docs/development.md), from the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/gallery/main.exe -j 2
./scripts/gpuio exec dune exec examples/gallery/main.exe
python3 scripts/test_gallery.py --section extensions
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery
```

Choose Native extensions. The native macOS driver enables package lifetime trace
instrumentation for this section. Consumer build coverage is separate; --run
requests its native walkthrough as described in [README](README.md). Commands were
reviewed, not executed here, and compilation is not physical/native lifetime
acceptance. Linux desktop qualification remains deferred.

## Reducer state, properties and commands

B.state_machine0 starts State.initial (value 7,step 1,generation 1,sequence 0,no command,
enabled/visible). apply_action forwards each typed action to State.apply; there is
no input-dependent model beyond the action stream. inject is a reactive function
returning a deferred effect. let%arr combines palette, state and injector into
current view descriptions. It does not eagerly send every displayed command.

Counter.Properties.create validates value 0–100 and step 1–10; Counter.instance
adds explicit generation, disabled state and optional(sequence,value) set_value
command. See the [package interface](../extension_package/ocaml/gpuio_example_counter.mli)
and [package README](../extension_package/README.md). Application checks that its
statically composed native backend catalog contains Counter.schema. This page
neither registers a factory per render nor dynamically loads a plugin.

Set property assigns model value 12 without a command sequence. Send command creates
one positive sequence requesting 42, retaining the old observed value until matching
Command_completed. While pending, property/step/another-command controls disable,
and State.apply independently rejects those intents. Native acknowledgements
clear the exact pending command and assign its requested value. A rerender is not
an acknowledgement and a property update is not a command.

The extension event callback captures State.generation and injects Observe with
that generation. The reducer rejects obsolete generations; Data also rejects
out-of-range values and input while hidden/disabled/command-pending. Mounted
updates notice, matching Command_completed completes the command, and Failed
clears pending command/reports typed error. Native generation/handler fencing
and application reducer checks serve different boundaries.

## Hide, disable, reset and depart

The stable extension-counter key and Display Hidden hide presentation while
retaining the native instance. Disabled blocks user input but explicit commands
remain available. Reset increments generation and starts value 7/no command while
preserving step/disabled/visible; this intentionally replaces the native instance.
A generation at int64 maximum is not incremented. The pure reducer bounds the
one-command slot and guards command sequence exhaustion.

B.Edge.lifecycle's on_deactivate injects Depart. [Pages](pages.ml) uses match%sub
on the selected gallery page, so departure deactivates this branch and unmounts
its native widget. Depart advances generation, clears command and updates notice;
retained application value/step/flags survive the branch visit. A future mount
cannot replay a completed/pending old command or accept old-generation events.
Palette changes update outer layout but the package's green artwork is independent.
There is no Eio task or resource scope created by this page: native mount/disposal
belongs to View.extension reconciliation/window lifetime and the SDK factory.

Concrete trace: Send command 42 → reducer stores(sequence1,42) → let%arr emits instance
with command → native executes it → Command_completed 1 returns Observe(current generation,event) → reducer stores 42/clears slot → updated view omits command.
Reset during pending command advances generation; an old reply cannot mutate it.
Hide/show retains native state, whereas page departure destroys native placement.

For another package, compose/register its factory and schema once in the backend,
then create a bounded typed page reducer with explicit command/ack/generation
policy. Do not use native callbacks to mutate OCaml state synchronously or store
OCaml values in Rust widgets. Pure reducer tests (linked from its guide) and the
native driver establish different behavior; opt-in drop logs are diagnostics,
not a public SDK lifetime guarantee or proof from this source review.
