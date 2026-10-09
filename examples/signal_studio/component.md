# One reactive snapshot, one layout function

[component.ml](component.ml) and [component.mli](component.mli) are Signal Studio's
entire Bonsai component boundary. Read the seven-line implementation, then
[Ui](ui.md) and its Snapshot/Actions interfaces. From the repository root after
[setup](../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/signal_studio/main.exe -j 2
./scripts/gpuio exec dune exec examples/signal_studio/main.exe
```

component takes an application-owned Ui.Snapshot.t Bonsai.Cont.Expert.Var.t,
a Ui.Actions.t, window and graph, and returns a GPUIO View.t Bonsai computation.
The alias `module Bonsai = Bonsai.Cont` chooses the continuation-style API;
opening Bonsai.Let_syntax enables let%arr. Expert.Var.value observes the mutable
source as a reactive value. let%arr derives Ui.view snapshot actions each time
the source changes. It does not block, start an Eio fiber or evaluate button
effects. The graph/window arguments satisfy App.Window.create's component shape
but are deliberately unused here; there is no per-window Bonsai state machine.

[Application](application.ml) owns the Var and supplies this component when making
a window. Its update helper publishes new immutable snapshots. For example,
a counter native Data event returns a deferred application effect → Workspace
validates the run → application republishes scene/chart and sets the Var →
let%arr sees the new snapshot → Ui.view rebuilds the run label and properties.
The native controller then reconciles the declarative view. A Var update is not
an assertion that a frame has painted. The [workspace](model/workspace.md) is pure
application data; Ui.Snapshot additionally contains borrowed native handles and
runtime presentation state. Neither the component nor its reactive recomputation
creates those handles.

Actions are stable injected callbacks/effects, whose mutations and I/O happen
when an event handles them. Application-scoped native resources and file tasks
survive window close while the application keeps its state; closing/remounting
this component does not independently own/cancel them. Ordinary UI code should
use this reactive path rather than [Ui_thread.perform](ui_thread.md), which is
reserved for waiting from Eio startup/test tasks.

For an extra pure label, derive it in Ui.view from the snapshot. For editable
local state, decide whether it belongs to the application or a Bonsai graph and
introduce explicit lifetime ownership; adding a mutable reference inside view
construction would lose the reactive contract. This file has no independent
unit test; startup and native interaction exercise its wiring through the
[README harnesses](README.md#local-checks). No build/GUI acceptance was newly
established by this source walkthrough.
