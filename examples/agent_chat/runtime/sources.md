# A window-owned virtual source explorer

[sources.ml](sources.ml) and [sources.mli](sources.mli) combine a scoped lazy tree
loader with Bonsai observations, native selection/keyboard/drag interaction and
application-approved moves. Nodes are [synthetic source notes](source_data.md),
not files in the checkout. `create` belongs in the window factory before graph
evaluation; `component` constructs the reactive view afterward.

Use the [isolated environment](../../../docs/development.md) from the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/agent_chat/main.exe -j 2
_build/default/examples/agent_chat/main.exe
```

Open **Explore sources**. Expand Project sources; expand Research notes and use
Retry after its deliberate failure. Select/read a note, use Shift+arrows for a
range, type a label prefix, reveal the main source, or archive a leaf through its
Actions menu. Drag proposals and Archive both need **Confirm move** before sample
data changes. Large/empty/reset controls remain local. macOS is the v1 target;
Linux GUI coverage is [informational](../../../docs/platform-release-policy.md).

## Controller state and asynchronous loading

Read `create`, `cancel_build`, `reset`, `build_large`, `propose`, `cancel`,
`approve`, `render_item` and `component`. `t` owns a `Tree_loading` controller,
a [Fixture_job](fixture_job.md), the supplied large-builder capability, observable
notice/pending/busy values and a serial for move-dialog visits. It is not a row
model: its lifetime is the window scope supplied by
[application startup](../application.md).

`create` starts `Source_data.initial` with three folders, only Project initially
expanded. Its load closure counts attempts by parent string ID and current
request generation. A new generation starts that parent's attempt count again.
Research notes sleeps two seconds; other branches sleep 0.3 seconds. It then
calls pure `Source_data.load`, whose first notes attempt fails and whose retry
succeeds. Eio sleep uses an explicit capability; no filesystem read occurs.
Failure to construct the fixture job closes the already-created loader.

The [Tree_loading contract](../../../lib/eio/tree_loading.mli) bounds reusable
workers and checks generation/incarnation/child revision before applying queued
results on the UI domain. Its `value` supplies reactive snapshots and `controls`
supplies checked demand/retry/cancel effects. The Bonsai tree's default
`cancel_hidden = true` cancels unfinished child loads hidden by collapsing
branches. Merely changing inspector pages does not destroy the window-owned
loader or sample data; `active` gates automatic demand. Window closure cancels
its scope and outstanding work.

`reset` cancels fixture delivery, clears pending move approval and calls
`Loader.reset` with initial/empty/large data. Reset retires source generation and
old targets. `build_large` ignores another request while busy, marks the notice,
then submits the supplied worker calculation through Fixture_job. Successful
completion resets the loader to the 100,000-node fixture; error clears busy and
reports the diagnostic. Reset can invalidate delivery while a pure worker
calculation drains; [the job guide](fixture_job.md) explains why this prevents
accumulating worker domains rather than promising immediate CPU interruption.

## Build the graph and bounded native rows

`component` takes reactive `active` and `dark` values plus a Bonsai graph. A
reactive value updates dependent computations when its source changes.
`let%arr ... in` derives values/views from current observations; `Bonsai.Effect`
values describe deferred actions executed later on the UI loop.

`Loader.value` supplies `source`. Derived `tree_style` gives a full-width,
300-logical-pixel-high native tree; `initial_expanded` chooses Project if present.
`Tree.component` uses stable key sample-source-tree, Multiple selection,
fixed 34-pixel rows, 68-pixel overscan and at most 24 active rows. It receives
loader controls, `~auto_load:active` and `~allow_moves:true`. Native disclosure,
selection, prefix navigation, focus/keyboard/AX behavior belong to this public
tree component, while application data and move acceptance remain here. The
[tree adapter interface](../../../lib/bonsai/tree.mli) explains initial seeds:
they seed a new source generation, not overwrite later live user preferences.

`render_item` receives reactive source/target/item/controller/lifetime values.
Its `let%arr` builds the label/menu inside the native TreeItem. The wrapper's
`Managed_rows.Lifetime.guard` protects queued row commands from acting after
that virtual row lifetime retires. The command registry has Read source note
and Archive source; Archive is enabled only for leaves. A context menu and menu
button share the same registry. Read invokes `Tree.Controller.activate`;
Archive captures the current Archive target, then guards reveal/propose-move
effects. The command IDs may repeat across rows because each row has its own
command scope; they are not dataset keys.

`on_action` handles only application-facing actions: activation looks up the
current node and changes the notice; a move stores a proposal; None does
nothing. Selecting a row itself remains native/component state, observed through
`Tree.Output.state`. The selected-node description and parent Location derive
from current source data. Multi-selection shows its count rather than picking
an arbitrary single node.

`reveal` uses `Tree.Output.target` and `Controller.reveal ~focus:true`; targets
capture identity, not payload snapshots. Reveal main source is disabled until
App is loaded. Reveal last source targets source-099997 in the large fixture.
A controller can open loaded ancestors and reveal after projection updates;
it does not load an unknown path. Native row eviction does not delete loaded
tree data or pending approval.

## Trace a lazy page and a confirmed move

Expanding Project causes native interaction to update tree preferences. With
active auto-load, an eligible expanded branch requests its first page through
loader controls. The scoped Eio producer waits, returns pure child nodes, and
checked UI delivery updates `Loader.value`. Bonsai projects the new visible
rows, mounting only the viewport/budget set. Collapse while loading cancels
that hidden branch's request; obsolete completion cannot append stale children.
Research notes adds an explicit Failed snapshot/Retry path instead of silently
restarting on every render.

For Archive source, the guarded row command reveals Archive and requests an
Inside move. `propose` assigns a new dialog token and stores the immutable
proposal. `component` derives an alert dialog named Review source move with a
key source-move-<token>. Cancel/dismiss clears only that matching token.

Confirm uses the effect supplied by `Bonsai.peek output` to read the latest
active tree output at invocation. Inactive/error output returns Ignore.
Otherwise `approve` first checks the pending token, then calls
`Source_data.approve (Loader.snapshot ...) ~state:(Tree.Output.state output)`.
That pure transformation rechecks target currentness and full destination
loading before rebuilding the hierarchy atomically. Only then does
`Loader.update` adopt it. Errors report a notice; success says no real files
were modified. Confirmation clears the pending proposal even when rejected,
so a changed tree needs a fresh proposal. Tree targets/state and dialog token
jointly prevent an old callback from approving a newer move.

## Retention, adaptation and validation boundaries

Inspector constructs this Bonsai component once per window and derives `active`
from its current route. Native page content may unmount while window controller
data and the continuously constructed graph remain. Resetting the loader starts
fresh tree preferences; ordinary row virtualization does not own them. Full
100,000-node application storage and source transformations still have CPU/
memory cost even with 24 mounted rows. A pure approved move traverses the full
tree; this is a demo implementation, not an incremental filesystem index.

A small adaptation is another lazy collection: extend Source_data's roots and
load cases, keep globally unique validated IDs and accurate More/End boundaries,
and add a runtime delay policy only if needed. Real I/O should capture explicit
capabilities in the loader closure and preserve scoped cancellation. Keep move
proposals separate from acceptance and recheck current tree/state on confirmation;
do not mutate real files inside the row renderer or infer authorization from
native dragging alone.

`python3 scripts/test_agent_chat_sources.py` is the optional macOS external input
runner, with Accessibility prerequisites and owned-process cleanup described in
the [README](../README.md). Existing [milestone evidence](../../../docs/evidence/agent-chat-m5.md)
records its actual checks. This source/link review runs no GUI and adds no
keyboard, drag, accessibility or large-fixture acceptance claim. Request/adoption
success alone also does not prove a physically presented frame.
