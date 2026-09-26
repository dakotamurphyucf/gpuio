# OCH-38 implementation evidence

## Core loaded forest — 2026-09-26

Implemented `lib/core/tree.mli` and `tree.ml`, with expect tests in
`test/view_api/tree_test.ml`. This checkpoint implements application data and
identity only. It does not claim a rendered tree, native accessibility/input,
lazy producer cancellation, managed row cache bounds or a tree capability.

The Core API provides abstract comparable IDs; opaque payload-bearing nodes;
leaf versus branch/paging-boundary descriptions; ordered roots/preorder;
parent, depth, sibling position and known/unknown sibling count; ancestor lookup;
revisioned atomic replacement; and payload updates that share topology. All
application payloads remain OCaml values and are not compared or serialized.

Validation rejects duplicate ownership/IDs, missing references, unreachable nodes,
cycles, invalid text, excessive depth, node count and metadata. Iterative traversal
is independent of the runtime call stack. Node constructors cache child-reference
counts and metadata sizes; admission checks aggregate bounds before traversing
references. Conservative byte accounting includes both node ID declarations and
root/child ID references, even if callers physically share those strings.

Local macOS arm64 expect evidence:

- Ordered multi-root/nested forest returns the expected parent/depth/index values;
  a paged branch reports unknown sibling count. Empty folders remain branches.
- Ten malformed topologies reject, including missing roots/children, duplicate
  roots/children, multiple parents, root-as-child, unreachable nodes, and rootless
  or disconnected cycles.
- Reorder and subtree relocation preserve node incarnation. Payload updates share
  the exact cached preorder/position and unaffected node values; they preserve
  child revisions. Changing child order advances the affected parent's revision,
  while unrelated branches keep theirs. Deletion and reintroduction mint a new
  incarnation. Invalid replacement leaves the old immutable value unchanged.
- Empty/oversized/invalid UTF-8/NUL IDs and labels reject. Opaque binary cursors
  remain valid within their separate byte bound. A 128-level chain is admitted;
  129 levels reject. An aggregate long-label case exceeds 8 MiB and rejects.
- Two 20,000-node cases with 256-byte IDs prove that references also count toward
  the byte budget: one has long root references, the other long child references.
  Their ID declarations alone would fit; the complete metadata does not.
- Exactly 100,000 loaded nodes traverse and sum to `4999950000`, reorder in reverse,
  and retain that order during a point payload update. A 100,001st node rejects.
  This is pure metadata coverage, not a native full-history cache/RSS measurement.

Commands through the repository-local toolchain:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune runtest -j 2 test/view_api
./scripts/gpuio exec ocamlformat --inplace lib/core/tree.ml lib/core/tree.mli test/view_api/tree_test.ml
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @all @runtest @fmt
git diff --check
```

The targeted suite and final complete Dune build/expect/format checks pass. The
first compile caught a record-field qualification error, corrected before testing.
The final checks include tightened reference accounting. Logs are
`tree-core-tests.log` and `tree-core-dune-final.log` in the implementing agent's
ignored scratch directory. No native GUI processes were needed or started for
this pure Core change. No Rust/protocol code or dependency pins changed.

Remaining OCH-38 work is explicit in the [design](../design/managed-trees.md):
preferences/keyboard reduction, generation-checked lazy loading, managed row
projection, native tree semantics/input/reveal/move intents, public Eio filesystem
usage and large/deep/full-traversal native lifecycle acceptance. Consolidated
macOS/Linux hosted gates and merge also remain. Linux GUI acceptance is OCH-17.
