# Pure source-tree fixtures and approved moves

[source_data.ml](source_data.ml) and [source_data.mli](source_data.mli) define
synthetic source collections and a pure hierarchy transformation. Labels such as
App.ml describe sample notes; their IDs are fixture identities, never filesystem
paths. This module performs no I/O, starts no tasks and constructs no Bonsai graph
or native views. The [Sources controller](sources.md) presents and loads its data.

Build/run from the repository root after [isolated setup](../../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/agent_chat/main.exe -j 2
_build/default/examples/agent_chat/main.exe
```

Open Explore sources: Project sources loads three notes, Research notes first
fails and then succeeds with Retry, and Archive starts empty. Large and empty
fixtures are explicit controls, not startup costs. macOS is the v1 desktop target;
Linux graphical qualification is [informational](../../../docs/platform-release-policy.md).

## Construct valid tree values

Read `id`, `leaf`, `folder`, `initial`, `empty` and `large` first.
`id` validates a string through `Gpuio.Tree.Id.of_string`; `ok = Or_error.ok_exn`
treats the checked-in constants as invariants. `leaf label text` constructs a
`Tree.Node` with `Leaf` children and string payload. `folder label ids next`
constructs a `Branch { ids; next }` whose payload is a simulated collection note.
The [tree contract](../../../lib/core/tree.mli) owns validation of node identity,
parent/child relationships and source replacement.

`initial ()` has three root IDs: project, notes and archive. Project/notes each
have an empty child list with `More None`, indicating that more children exist
but no cursor is needed for the first request. Archive has `End`, meaning its
empty child list is complete. Distinguishing unloaded from empty lets the native
tree show a loadable branch without inventing leaf rows.

`large ()` makes 99,997 leaf nodes numbered 1–99,997 beneath Project sources and
the same three folders: **100,000 nodes total**, rather than 100,000 leaves.
Leaves have IDs `source-%06d`, labels Source followed by that number, and a short
generated note. All branches are complete (`End`). This finite immutable fixture
can be calculated on a worker domain and returned to the UI. Its full data stays
in application memory even though native row mounting is bounded. `empty ()`
constructs a valid tree with no roots or nodes.

## Supply one lazy page

`load ~attempt request` accepts a `Tree_loading.Request`, not an ordinary node
ID alone. A present cursor is rejected: these small branch fixtures have one
page each. Research notes with attempt 1 returns a reproducible error; later
attempts return Design decisions.md and Validation.md. Project returns App.ml,
Theme.ml and README.md. Other parents report no lazy children. Successful output
is a `Tree_loading.Page` with root child IDs, matching nodes and `next = End`.

`attempt` comes from the runtime's per-parent/per-generation accounting, not a
clock or random failure. This function itself neither increments an attempt nor
sleeps. [Sources.create](sources.md) adds the two-second notes/0.3-second project
delays and runs this function inside its scoped loader. Public
[tree loading](../../../lib/eio/tree_loading.mli) validates request generations,
incarnations and child revisions before accepting asynchronous pages.

## Apply a current move atomically

Read `approve snapshot ~state proposal` after the fixture builders. It begins
with `Tree_interaction.Move.is_current proposal snapshot ~state`, checking the
latest source and logical tree preferences. A native drag/controller proposal
only describes a move; it does not modify the hierarchy. The
[interaction contract](../../../lib/core/tree_interaction.mli) rejects stale,
disabled/invisible, self/descendant endpoints and invalid Inside placements.

`approve` extracts source/destination IDs and the source's old parent. For
`Inside`, the new parent is the destination; for `Before`/`After`, it is the
destination's parent, including the root list when that parent is absent. A
non-root new parent must be a fully loaded branch (`next = End`); otherwise the
function returns an error instead of claiming where unknown children belong.

The local `update parent members` removes the source from its old sibling list,
then inserts it at the chosen location in the new sibling list: append for
Inside, or adjacent to the destination for Before/After. `None` represents the
root list. The function maps all nodes, rebuilding only changed branch nodes
while preserving each label, disabled flag, payload and `next` boundary. It
finally calls `Tree.replace` with updated roots/nodes, validating the complete
new tree before returning it. No mutation occurs if this fails. The runtime then
accepts this value through `Loader.update`; it does not reset the entire source
merely to move one item.

For **Archive source**, the row controller captures the current Archive target,
reveals it and proposes Inside. Sources displays a confirmation dialog, then
calls this function with a fresh loader snapshot and current tree state.
Cancel leaves the tree unchanged; confirmation moves the same ID/payload under
Archive. This trace changes only the sample, not App.ml or any checkout file.
Atomic tree replacement preserves identity for surviving nodes; row positions
are presentation, not the source's identity.

This move implementation traverses the complete tree, so a move in the large
fixture has application CPU cost despite bounded native rows. It is a clear
sample transformation, not an incremental database index or a performance
promise for arbitrary million-node moves.

A small adaptation is to add another synthetic leaf to Project's `load` result.
Use a unique validated ID, include it in both the page's roots and nodes, and
keep the final boundary correct. If adding multiple pages, implement cursors and
stop assuming that one response completes the branch. Preserve approval-time
currentness checks and fully loaded destination checks for moves. The
[Sources guide](sources.md) explains reactive UI, commands and cancellation;
this pure-model review supplies no new drag/keyboard/native acceptance evidence.
