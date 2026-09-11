# Production bridge V1 implementation contract

Work in progress for OCH-7. This is separate from the private foundation smoke
protocol. No released protocol compatibility is promised before the first API
release. The initial style tags will expand with OCH-8; additions require paired
OCaml/Rust definitions and independent fixture review.

## Ownership and identity

One native runtime owns its windows. Each window owns a retained tree. Window,
node, handler and resource identities are distinct `(slot, generation)` types.
Slots fit unsigned 32 bits; generations are 1 through 2^32-1, never wrapping.
New node slots are contiguous; freed slots can be reused only with the next
generation. Tombstones contain no text, child arrays or widget resources.
Events must match both the window generation and current node/handler binding.
Changing a callback closure without replacing its binding may retain the handler
ID; dispatch uses the currently committed OCaml callback registry.

## Schema and transaction semantics

Owned bin_prot messages use declaration-order variant tags. Hello requests an
exact protocol version and a required capability mask. Unsupported versions or
capabilities fail explicitly. Resources/document append/edit and namespaced
extensions are reserved capabilities, not accepted opaque arbitrary commands.
Correlated open/close/frame requests are distinct from per-window transactions.
Acceptance and rendering are distinct events; rendering does not assert physical
screen presentation. The client submits one transaction per window at a time.

Create chooses immutable node kind. Text/style/binding updates preserve identity.
Style lists replace the previous list, so absent properties reset; repeated
refinements compose in order, last property wins. Splice uses the current child
array's zero-based offset and deletion count. Remove removes exactly one node:
the same atomic batch must detach it and remove or reparent its descendants.
The final tree must have exactly one root and one parent per other node, with
no cycles, dangling edges or unreachable nodes. An empty tree has no root.

The native implementation stages changed slots in an overlay. Validation failure
publishes no mutation, generation change or revision. Successful commits advance
exactly one revision. Text/style/binding edits do not walk unrelated history;
structural changes currently validate the complete final tree iteratively.
Untouched text, style and child arrays remain shared immutable allocations.
Structural splice cost includes copying the changed parent's child array; this
is not a claim of constant-time child insertion.

## Bounds and validation

Initial limits: 1 MiB encoded message, 256 KiB text field, 4096 operations,
128 style refinements per node, 100,000 slots per tree, depth 128, and 32 windows.
Decode rejects invalid tags, malformed UTF-8, trailing bytes and non-finite
numbers. Container allocation is bounded by both the declared limit and the
remaining input. Domain validation additionally checks sizes, colors, IDs,
revisions and structural invariants. Public callers must not bypass validation
with generated deserializers. Queue and cumulative retained-byte budgets are
still being implemented; these field/count limits alone are not a complete
runtime memory budget.

## Current evidence and remaining work

The initial Rust decoder and retained tree compile and pass tests of truncations,
allocation bombs, rollback, cycles, duplicate parenting, stale generations and
revisions. A 10,001-node tree with 10 MiB of text updates one label by touching
one slot and scanning zero unrelated nodes. A deterministic value-model check
covers accepted and rejected text edits. This is not yet a full protocol fuzz or
randomized structural reference-model suite.

Still required for OCH-7: OCaml wire definitions and independent fixtures,
handshake/session implementation, bounded queues, cumulative memory budgets,
FFI ownership/panic boundaries, native GPUI adaptation, correlated responses and
render acknowledgements. Public view/reconciliation and Bonsai/Eio scheduling
belong to OCH-8/OCH-9. macOS native validation is the current development gate;
Linux builds/tests remain required and graphical checks are informational.
