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

## Generation-checked lazy loading and Eio workers — 2026-09-26

Added Core and Eio `Tree_loading` modules, with interfaces drafted before their
implementations. The Core controller owns immutable snapshots of loaded tree data
and request status. It bounds the FIFO to 64 queued branches and four running
requests. Completion tokens contain controller identity, reset generation, serial,
parent incarnation and child-list revision. Data-only or unrelated source edits
preserve work; source reset, relevant hierarchy changes and explicit cancellation
make late responses obsolete before their page contents are inspected.

Each page is a closed forest of at most 2,048 new nodes. Roots append to the
parent's children. Existing ID reuse, cycles/duplicate ownership, missing nodes,
invalid cursors and combined depth/metadata overflow fail atomically. Empty pages
must reach End or advance their cursor. Failure requires explicit retry. The
controller retains at most 64 detailed error messages of 4,096 UTF-8 bytes each;
evicted details leave a generic Failed marker rather than enabling automatic
retries. Markers are bounded by loaded node count and removed with invalid parents.

The Eio adapter lazily allocates up to four reusable scoped workers. A worker owns
one request at a time and keeps that slot while cancellation unwinds. Results and
slot release cross the existing bounded UI inbox. Idle workers wait on streams;
there is no polling, per-node idle fiber or scheduler patch. Nested request
cancellation is separate from outer worker/scope cancellation, allowing shutdown
to interrupt inbox backpressure. Snapshot changes publish to Bonsai and optional
`on_change`; no-op cancellation does not publish redundant updates. Close clears
work/errors from the model and published snapshot and cancels only its workers.
The latest application data and unrelated tasks in the parent scope are preserved.

Passing local Core expect evidence:

- FIFO ordering, 64 queued/four running admission, duplicate suppression, capacity
  recovery, and 10,000 enqueue/cancel cycles without retained queue history.
- Foreign, cancelled, duplicate and reset-generation completions are obsolete;
  invalid obsolete pages are not admitted or reported as current failures.
- Independent concurrent branches, multi-page cursors and nested child forests
  publish atomically while historical snapshots remain unchanged.
- Invalid current pages leave the exact previous tree unchanged and become Failed:
  no cursor progress, missing/duplicate/existing IDs, cyclic data, oversized cursor,
  page-node overflow and final depth overflow all reject. Explicit retry recovers.
- Payload changes/sibling reorder preserve requests. Hidden-branch/subtree cancel,
  parent incarnation reuse, child-boundary changes, explicit invalidation and reset
  retire them. Backward source updates reject rather than overwriting loaded data.
- Seventy failed branches retain only 64 error details. Eviction and cancellation
  never turn a failure into Ready; UTF-8 truncation and invalid-message fallback
  pass. Removing the forest clears all markers/details.

Passing Eio mock-backend/runtime evidence uses actual scoped fibers and the UI
inbox, with no wall-clock sleeps:

- Producer failures and successful pages publish only on UI delivery; failed
  branches do not retry automatically, and an explicit retry succeeds.
- Reset after a producer finished but before UI delivery suppresses the queued
  result. A subsequent request on the same worker can complete normally.
- Six requests run at most four producers. Cancelling and immediately requeueing
  one does not increase that peak; only after its exit does queued work start.
  Close cancels every producer while an unrelated conversation task remains alive.
- With inbox capacity one, four completed producers encounter real backpressure.
  Scope shutdown cancels all workers without hanging, applying stale pages or
  publishing post-close callbacks.
- Worker admission failure under a one-task scope becomes an explicit retryable
  failure, not a stranded queue. A freed existing worker later serves the retry.

Validation:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune runtest -j 2 test/runtime test/view_api
./scripts/gpuio exec ocamlformat --inplace lib/core/tree_loading.ml lib/core/tree_loading.mli lib/eio/tree_loading.ml lib/eio/tree_loading.mli test/view_api/tree_loading_test.ml test/runtime/tree_loading_test.ml
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @all @runtest @fmt
git diff --check
```

The combined runtime/Core suites and complete Dune build/expect/format checks pass
on local macOS arm64. The first Core compile found a shadowed helper binding,
corrected before acceptance. The initial FIFO harness used `List.init` around a
stateful dequeue; its evaluation order reversed the collected observations. The
harness now explicitly sequences dequeues with a left fold and proves actual FIFO
ordering; the production ordering contract was preserved.

Final logs are `tree-loading-runtime-tests.log`, `tree-loading-eio-dune-all.log`
and `tree-loading-final.log` in the agent's ignored scratch directory. No native
GUI process, Rust/protocol change or new dependency pin was required. This is
Core/Eio scheduling acceptance, not actual filesystem-provider, native tree widget,
AX/IME or graphical full-traversal acceptance. Managed Bonsai rows, native
input/typeahead/focus/reveal/move intents and public filesystem integration remain,
followed by consolidated hosted gates and merge. No tree capability is advertised.
