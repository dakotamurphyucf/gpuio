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

## Persistent preferences and logical navigation — 2026-09-26

`Tree_state` now separates expansion/selection preferences and the logical active
item from `Tree` application payloads. Its public interface was drafted before
implementation. Selected/expanded maps carry node incarnations; the cached visible
order/index contains IDs and minimal parent/disabled/incarnation metadata only.
Neither this state nor the collection owns native rows, OS focus or Eio producers.

The model supports validated programmatic preferences, Single/Multiple policy,
Replace/Toggle/Range gestures and a stable range anchor. User requests ignore
missing, hidden and disabled targets; programmatic preferences may deliberately
include hidden/disabled members. Collapse preserves descendant preferences while
moving an active descendant to an eligible ancestor. Reconciliation prunes removed
or reincarnated preferences and invalid leaf expansion. Deleted/disabled active
items use the documented ancestor/neighbor fallback without choosing a new
selection implicitly.

The pure `navigate` reducer handles Previous/Next/First/Last/Parent/Child, branch
opening/closing, disabled-row skipping and cursor-only versus selection/range
movement. This is logical keyboard semantics; actual GPUI keyboard delivery,
IME priority, native focus/reveal and typeahead are still pending native integration.

Passing local macOS arm64 expect tests cover:

- Nested collapse/reopen keeps selected descendants and their expansion, excludes
  hidden requests, and repairs the logical active item independently.
- Range growth/shrink uses visible order, skips disabled nodes, preserves the
  anchor, supports union and handles a hidden anchor through active-item fallback.
- Duplicate/missing/leaf preferences reject; explicit disabled selection is kept;
  changing selection mode preserves a selected active item or the first selected
  item in full hierarchy order. Single-mode toggle remains single selection.
- Reorder preserves the active item and preferences. Leaf conversion/deletion
  prunes incompatible preferences and chooses an ancestor. Deletion/reintroduction
  rejects stale selected/anchor incarnations even if the intermediate empty state
  was never reconciled. Empty/fully disabled sources clear the logical cursor.
- Payload-only tree updates return the same state snapshot. Selection/focus share
  visible metadata. With 100,000 selected nodes, 1,000 logical focus changes retain
  one visible snapshot; selection is not represented as active row computations.
- An explicit weak-reference/major-GC test proves a 1 MiB application payload is
  collectible while selected/visible metadata remains alive.
- Tree keyboard reduction covers branch open/enter/parent/close, Home/End,
  disabled skipping, Shift range extension, cursor-only movement and empty input.

Commands:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 lib/core/gpuio.cma
GPUIO_JOBS=2 ./scripts/gpuio exec dune runtest -j 2 test/view_api
./scripts/gpuio exec ocamlformat --inplace lib/core/tree_state.ml lib/core/tree_state.mli test/view_api/tree_state_test.ml
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @all @runtest @fmt
git diff --check
```

The final complete build, expect suites and formatting pass. Two intermediate
expect runs differed only in pretty-printed line wrapping; the observed semantic
values were checked and the exact expected whitespace corrected manually. All
assertions, including the GC and large selection cases, pass in the final run.
The final log is `tree-state-dune-all.log` in the agent's ignored scratch folder.
No GUI process or Rust/protocol change was required for this pure Core checkpoint.

Next are generation-checked lazy loading, managed row projection and native/public
integration. These tests do not establish native focus/AX/IME, bounded native row
caches across traversal, cancellation of Eio producers, or filesystem behavior.
No tree capability, hosted validation or completed OCH-38 ticket is claimed.
