# How `Outline_data` validates an atomic move

[README](README.md) · [Implementation](outline_data.ml) · [Interface](outline_data.mli)
· [UI caller](outline_demo.md) · [Expect tests](outline_test.md)

This pure application policy belongs to the `Tree_example_model` library. It has
no Bonsai state, effects, native handles, filesystem operations, or asynchronous
worker. The UI calls it immediately before publishing an approved replacement.

`id` validates a `Gpuio.Tree.Id` and unwraps known fixture errors. `initial ()`
constructs Inbox containing plan/notes and Archive containing release. Leaves
carry string descriptions; folders carry “A project folder” and complete
`Branch` child lists with `next = End`. `Tree.create` validates the whole forest,
including membership, reachability, unique parenting, and depth.

`approve snapshot ~state proposal` returns a validated replacement or error.
It first calls `Tree_interaction.Move.is_current` against the latest loader
snapshot and widget preferences. The source and destination must still be
current, visible, enabled, and eligible. Proposal reduction itself never mutates
the hierarchy. See [interaction contract](../../lib/core/tree_interaction.mli).

After validation, `Tree.position` identifies the source's old parent. `Inside`
chooses the destination as new parent; `Before` and `After` choose its parent,
including `None` for root insertion. A non-root destination parent must be a
fully loaded branch (`next = End`). Inserting into a partial sibling list is
rejected rather than inventing a server/paging position policy.

The local `update parent members` removes the source from its old sibling list,
then inserts it at the requested position in the new list. It also handles a
same-parent reorder in one pass: removal precedes insertion. `Inside` appends;
Before/After insert adjacent to the destination. The function runs on each branch
and the root list, so moving a child into the roots is handled explicitly.

Unchanged child lists reuse their nodes. Changed branches are rebuilt with
`Tree.Node.create`, preserving label, disabled state, payload, and page boundary.
`Or_error.all` combines node construction results, then `Tree.replace` atomically
validates the complete new topology against the existing lineage. Existing IDs
preserve incarnations across moves; invalid topology leaves the input unchanged.
See [tree data contract](../../lib/core/tree.mli).

`Or_error.Let_syntax` supplies `let%bind`/`let%map` here. These sequence error
results, not Bonsai graph updates or asynchronous effects. The returned immutable
tree still needs `Loader.update` in the owning UI effect to reach Bonsai and the
native view. The approval helper's generic interface preserves arbitrary payload
values even though its sample `initial` uses strings.

Build the caller and run this policy's expect suite from the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/tree/main.exe
GPUIO_JOBS=2 ./scripts/gpuio exec dune runtest -j2 examples/tree
_build/default/examples/tree/main.exe --outline
```

Use the [repository environment](../../docs/development.md). The pure tests need
no native window; the outline launch does. The tests cover placements, identities,
payload preservation, stale generation, and collapsed-source rejection, not all
possible move constraints or physical drag dispatch. For remote mutation, define
server ordering/conflict policy, revalidate at completion, and publish accepted
data through the loader. This helper never moves real files.
