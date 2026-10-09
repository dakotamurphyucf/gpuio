# How `Outline_demo` approves moves without stale confirmations

[README](README.md) · [Implementation](outline_demo.ml) · [Interface](outline_demo.mli)
· [Pure move policy](outline_data.md)

This in-memory outline supports native drag and command-menu alternatives.
It changes sample hierarchy only; no filesystem capability is captured and no
real file is moved. `App.run` owns the native runtime and opens an
820 × 560 window. A child application scope owns `Loader`; the fully loaded
fixture's loader returns an error if asked for another page.

## Model and row lifetimes

`Decision.t` holds a monotonically increasing serial, optional `{ token;
proposal }`, notice, and approved count. `Model.t` pairs loader ownership with
an external `B.Expert.Var` containing that decision. `Model.update` reads and
publishes state only inside UI effects. `propose` increments the serial and
replaces the pending proposal; serial exhaustion fails explicitly. `cancel`
clears only a matching token.

`Tree.component` gets `Loader.value`, fixed 42-pixel rows with a 16-row budget,
multiple selection, expanded Inbox/Archive, `allow_moves:true`, a custom item
renderer, and `on_action`. It owns selection/expansion and native interaction;
application code owns hierarchy and approval. See [widget](../../lib/bonsai/tree.mli).

`render_item` uses `let%arr` to observe source, target, item, controller, and row
lifetime. `B` means `Bonsai.Cont`; this reactive syntax rebuilds content as those
values change. `and` binds dependencies, not threads. Each item gets a keyed
`View.command_scope`, a validated command registry and Actions menu used by both
context menu and menu button. The renderer guards command effects with
`Managed_rows.Lifetime.guard`, so evicted row commands cannot act later.

Open item calls `Controller.activate`. Move alternatives capture a destination,
then combine reveal and `Controller.propose_move` in `E.Many`. A proposal is not
a mutation. Native drag is also single-row/same-tree and produces the same kind
of proposal; pointer preview stays native. There is no automatic hover expansion
or edge autoscroll in this example.

## Native proposal → approval model → dialog → accepted data

`on_action` turns Move into a thunk calling `Model.propose`. Activate resolves
the latest payload for a notice; it opens no external document. The decision
variable changes, Bonsai derives a keyed modal dialog `move-<token>`, and GPUIO
mounts Review move. Source data still has the original hierarchy at this point.

`B.peek output graph` provides an effect that reads current computation status.
Confirm captures the pending token, then `E.bind peek` checks that the tree is
still active with valid output. It calls `Model.approve` with current widget
state, rather than state captured when the dialog rendered. `E.bind` sequences
an effect result; `let%arr` derives a view from reactive values.

`Model.approve` checks the matching token and consumes it **before** mutation,
so duplicate callbacks cannot apply twice even before another native frame.
It calls `Outline_data.approve` against `Loader.snapshot`, then `Loader.update`
with the validated replacement. Success increments approved count; errors become
notice text. The next snapshot/view retains stable item identity in its new
position and removes the modal dialog. Dismiss/Cancel use token-checked cancellation.
A superseded token cannot approve or cancel a newer dialog. A source reset makes
old proposals fail current-snapshot validation even if IDs look the same.

`B.Edge.after_display` records `Observed.t` (output, view, confirm effect, decision)
for diagnostics. Holding output/view here is deliberate test observation and may
retain snapshots; the production approval target stores no snapshot.
`menu_command` walks `View.Expert.describe` to find a command callback in the
keyed scope. This is direct callback inspection for tests, not native menu input.

## Launches and test boundaries

From the repository root using [development setup](../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/tree/main.exe
_build/default/examples/tree/main.exe --outline
_build/default/examples/tree/main.exe --outline --self-test
python3 scripts/test_tree_outline.py
```

All launches require a graphical session. The self-test uses [Support](support.md)
to invoke a menu command, checks the hierarchy is unchanged until confirmation,
invokes confirmation twice, and checks one approval plus stable target identity.
It then checks superseded confirmation and stale approval after reset. Staged
waits have 25-second limits; the final frame has a 5-second limit. It prints
`TREE_OUTLINE_PASS` and shuts down. It does not generate physical drag/menu input.

The macOS harness starts the already-built executable with
`--outline --gesture-self-test`, drives native drag, confirmation, context menu,
and move back, and checks `TREE_OUTLINE_GESTURE_PASS`. It needs Accessibility
permission, checks pointer ownership, and reaps the child. The gesture flag alone
waits for those external actions; it is not autonomous automation. These scenarios
are distinct from large-tree resource tests and
[Linux graphical qualification](../../docs/platform-release-policy.md).

For asynchronous server approval, keep the token and target identities, resolve
latest snapshot/state before applying, and own tasks in a deliberate scope.
Keep preferences outside transient rows when they must survive eviction; retain
the row lifetime guard on menu effects and never turn a proposal into an immediate
filesystem mutation merely because native dragging succeeded.
