# Native size selection with two active Bonsai branches

[main.ml](main.ml) creates Compact and Wide presentations, each with independent
draft/counter. Read `config`, `branch`, `component`, then startup and diagnostic.
The [README](README.md) gives launch commands; [dune](dune) links Core/GPUIO/
Bonsai/Eio and PPX. No assets or services are needed. Setup/platform limits:
[development](../../docs/development.md),
[release policy](../../docs/platform-release-policy.md).

Validated branch IDs compact/wide are native presentation identity. Config's
first rule picks wide when assigned width is at least 600 logical pixels, otherwise
compact; the minimum is inclusive. This is assigned **container** size, not an
OCaml window-resize callback. `Container_query.Range/Predicate/Rule/Config` encode
bounded native conditions; see [contract](../../lib/core/container_query.mli).
Native layout evaluates them synchronously without asking OCaml for a view.

`branch` constructs a persistent Bonsai computation for one presentation. It
registers lifecycle activate/deactivate effects (diagnostic counters), creates
one Input controller with seed “Compact draft” or “Wide draft”, and allocates
local count 0 with B.state. Native Rust owns mounted text/selection/IME/history;
Bonsai owns each count. `B.Edge.after_display` stores latest controller in a
String.Map keyed by name for diagnostic access. It is not physical display.
`let%arr` reads editor/count/setter to derive ordinary GPUIO column, editor and
button. Effects execute later; rendering does not reset model or replace draft.

`component` calls branch twice outside a Bonsai conditional. Both computations
remain active. Its `let%arr` combines both derived views; `and` declares
dependencies, not threads. `UI.container_query` validates branch mapping and
retains inactive native presentation, while `on_select` returns a thunk recording
paint-confirmed native branch observations. No reactive selection state feeds
another layout decision back to native. Hidden editor cannot receive focus/input,
but its native lease/text and Bonsai counter remain.

Resize from 520 to 760: native layout chooses wide immediately, hides Compact,
paints Wide and sends a Selection observation asynchronously to OCaml. Recording
it does not rebuild view or deactivate Bonsai branches. Press Wide's count button:
native action reaches its setter effect, local Bonsai count changes, let%arr
derives button text and GPUIO submits it. Compact's model/draft remains untouched.
Resizing 760 to 800 stays wide and emits no repeated branch observation; the size
is not an OCaml render loop. Paint confirmation differs from physical display.

`App.run` owns GPUI OS thread and one OCaml Eio UI domain, opens a focused 520×360
window. Ordinary launch creates no producer. --self-test starts a window-scoped
Eio fiber with explicit clock/20-second timeout. Helpers await in 5 ms intervals,
bridge UI effects using Expert enqueue/handle plus promises, read editor state,
request Resize and await window snapshot geometry. It waits for both editors and
Compact selection, replaces a Unicode draft, resizes 760 and checks identical
retained snapshot plus Focus_blocked for hidden Compact and no branch deactivation.
A same-branch resize 800 checks unchanged events/OCaml commits; resize 520 checks
draft still present. Completion force-closes; final asserts both lifecycle
activations/deactivations and prints GPUIO_CONTAINER_QUERY_PUBLIC_OK.

This programmatic/native observation test is not real keyboard/AX, physical GPU
pixels, VoiceOver or Linux desktop acceptance. Visibility can defer paint, hence
focused diagnostic launch. [App](../../lib/eio/app.mli),
[Scope](../../lib/eio/scope.mli) and [Input](../../lib/eio/text_input.mli) specify
thread, cancellation and editor ownership.

To add a medium presentation, validate a distinct ID, create its Bonsai branch
and add ordered native rules with deliberate ranges. Keep all required mappings
present. For shared draft/count, allocate an application owner outside branches
and pass values/actions to each; do not share one native controller placement.
Native retained hiding differs from match%sub graph deactivation and Eio task
cancellation. Persist application drafts explicitly if using a lifetime policy
that destroys native content, and keep I/O out of let%arr.
