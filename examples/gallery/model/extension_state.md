# Extension state: serialize commands and fence instance generations

[extension_state.ml](extension_state.ml) and its [interface](extension_state.mli)
define a pure reducer for the gallery's separately packaged native counter. It
stores properties, one pending command, interaction flags and notice text. It
creates no Bonsai graph, native extension, plugin loader, Eio scope or I/O task.
[extensions_page.ml](../extensions_page.ml) supplies reactive composition and the
[static counter package](../../extension_package/README.md) supplies drawing/input.

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
./scripts/gpuio exec _build/default/examples/gallery/main.exe
```

Open **Native extensions**, activate the counter, set a property, send a command,
change step, hide/disable or reset. This model has no independent entry point,
asset file or diagnostic flag. The page uses a statically linked package, not a
runtime-loaded plugin. Toolchain/platform prerequisites are in the
[gallery README](../README.md); no new bridge or GUI acceptance follows from this review.

Read `t`, `Action`, `initial`, then `apply` and the public accessors. The abstract
model stores value=7, step=1, generation=1L, sequence=0L, command=None,
disabled=false, visible=true and a waiting notice. Command is an optional
`(sequence, requested_value)` pair; the demo requests value 42. Sequence distinguishes
commands within an instance; generation distinguishes replacement/retired instances.
Neither is a view revision or a persistent storage identifier.

`apply` returns a new model from its current input. Every Observe first compares
the callback's captured generation with the current one. Matching `Data value`
updates value only if visible, enabled, no command pending and value in [0,100].
Mounted updates notice. Command_completed applies only the matching pending sequence,
sets the requested value, clears pending command and records completion. Failed
clears pending command and reports its typed extension error; it does not assume
that the requested value succeeded.

Property changes and commands are deliberately distinct. Set_property sets value 12
when no command is pending; Toggle_step switches 1↔5 under that same gate.
Send_command advances sequence and stores `(next,42)` unless a command already
exists or sequence is exhausted. Disabled/visible toggles still work during a
pending command; they block native user Data, not explicit command completion.
Reset restores the initial value/command/sequence under a new generation, preserving
step/visible/disabled. Depart advances generation, clears pending command and restores
the waiting notice while retaining current value, step and sequence.

## Trace the page's pending command

The page wraps `State.apply` in `B.state_machine0`. A state machine owns reactive
model plus an injection effect; this reducer itself only transforms ordinary values.
The page's `let%arr` reads the current model and builds
`Counter.Properties.create ~value ~step`, then `Counter.instance ~generation
~disabled ?set_value`, finally `V.extension` under stable key `extension-counter`.
Display.Hidden hides the view while retaining the instance. Its event callback
captures the generation in that derived view and injects Observe with it.

Click **Send command to 42** at generation 1. The injection reduces Send_command to
sequence 1 and pending `(1,42)`, without prematurely changing value 7. The next
view passes that explicit command to the package; property/step/second-command
controls become disabled. Native Command_completed 1 arrives asynchronously with
captured generation 1, the reducer applies 42 and clears pending state, and Bonsai
re-derives the readout/configuration. An unrelated completion 99 or Data 99 while
pending does nothing. Disabling input while pending does not cancel this acknowledgement.

Now Reset: generation becomes 2, value 7 and command None. A callback still captured
at generation 1 cannot modify the new model. The page deactivation lifecycle injects
Depart so commands cannot be replayed on revisit. Native extension lifecycle/identity
is handled by the package and reconciler, not by allocating resources in this reducer.
Read [extension.mli](../../../lib/core/extension.mli) for typed events and instance
contracts and the package guide for extension-author Rust details.

`Int64.max_value` guards prevent wrapping. At exhausted sequence, Send_command is a
no-op; at exhausted generation, Reset is a no-op. Depart still clears pending state
but cannot advance an already-maximal generation. Thus the usual old-generation
fence cannot distinguish an old callback after that theoretical exhaustion point;
a generalized long-lived implementation should retire/fail rather than reuse a
saturated generation. The finite demo does not expose arbitrary generation setters.

The [existing gallery expect test](../../../test/gallery/gallery_test.ml) checks
command serialization, ignored sequence/generation events, disabled/hidden Data,
matching completion, reset and departure. It was read as existing test intent, not
new execution evidence. To change the command target, update Send_command's pending
value and corresponding labels; keep acknowledgement-driven completion. To add a
second command type, model it as a typed pending action rather than issuing concurrent
property rewrites that obscure which native acknowledgement owns the result.
