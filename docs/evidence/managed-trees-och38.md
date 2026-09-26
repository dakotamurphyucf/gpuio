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

## Incremental row-data projection (2026-09-26)

`Tree_rows` now projects a loader snapshot and separate `Tree_state` preferences
into OCH-13 `List_collection` records. This is the data layer for managed Bonsai
rows, not acceptance of the native tree widget. An expanded lazy branch has one
synthetic boundary after its loaded descendants. Item records carry hierarchy,
selection, expansion, logical active state and loading status. Compact keys retain
visible node identity across reorder/value changes, retire on collapse/removal/
incarnation change, and never prepend arbitrary application IDs or hash them.
Current identity maps and one monotonic counter replace any historical registry.

Shared-map invalidation in `Tree`, `Tree_state` and loader snapshots updates only
affected row values during streaming, selection, cursor movement and load-status
changes. It preserves the collection order snapshot and unaffected value wrappers.
Snapshots expose controller-aware `same_generation`; foreign controllers with
matching numeric generations cannot update an existing projection.

Local Core expect coverage:

- Nested lazy boundaries appear after their own descendants, preserve unknown
  sibling totals, and remain separate from application IDs, including maximum
  256-byte IDs and IDs resembling the adapter's compact key strings.
- Two coalesced payload updates invalidate those two rows; selection changes one
  row; cursor movement changes its old/new rows. Hidden payload updates preserve
  the exact visible collection and appear with their latest data when reopened.
- Queued/loading/failure/retry keep keys; completing a branch retires its boundary.
  Error-detail cache eviction invalidates the still-failed row and parent, with no
  accidental retry. Unchanged collection keys remain physically shared.
- Reorder retains identity. Deletion/reinsertion coalesced before projection
  retires the old incarnation. Backward revisions, foreign controllers and reset
  generations reject atomically and require the appropriate fresh projection.
- 100,000 selected logical nodes still produce one row invalidation for a point
  payload update and one for an initial cursor target. This creates data records,
  not native views or Bonsai row computations.
- 100,000 expanded lazy roots produce exactly 200,000 logical item/boundary rows,
  accepted by the existing managed-order validator. Maximum-size collapse/reopen
  preserves the bound; 1,000 small-forest cycles prove retired keys never return.
- A 128-level lazy hierarchy places all boundaries in correct descendant order.
  A weak-reference test collects a replaced 1 MiB payload while the current
  projection stays live; no previous-projection reference chain retains it.
- Direct node-diff tests use function payloads, proving no payload equality is
  required. Position-only edits are omitted, explicit same-payload replacement is
  invalidated, and removal/insertion IDs are reported once.

Validation commands:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune runtest -j2 test/view_api
GPUIO_JOBS=2 ./scripts/gpuio exec ocamlformat --inplace lib/core/tree.ml lib/core/tree.mli lib/core/tree_state.ml lib/core/tree_state.mli lib/core/tree_loading.ml lib/core/tree_loading.mli lib/core/tree_rows.ml lib/core/tree_rows.mli test/view_api/tree_test.ml test/view_api/tree_rows_test.ml
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt
git diff --check
```

The final complete Dune build, expect suites, formatting and whitespace checks
pass on local macOS arm64, including the controller-identity guard.
Logs are `tree-rows-tests.log`, `tree-rows-budget-tests.log`,
`tree-rows-dune-all.log` and `tree-rows-final.log` under the implementing agent's
ignored scratch directory. The initial budget test passed 100 maximum-size
collapse/reopen cycles; the routine suite uses two maximum-size cycles and 1,000
small-forest cycles to keep validation efficient without losing those invariants.
No native GUI windows, protocol/dependency changes or hosted CI were involved.
Managed Bonsai viewport/lifetime integration, native input/AX/typeahead/reveal/move
semantics, the Eio filesystem provider and native traversal acceptance remain.

## Bonsai row lifetimes and Eio controls (2026-09-26)

`Gpuio_bonsai.Tree_rows.component` now composes the incremental projection with
OCH-13's managed list/row lifetimes. Full source leases key both the Bonsai subtree
and native wrapper; separate controllers with numeric generation zero cannot
share transient models. Source reset/unmount retires controller effects, while
application-owned data loading stays independent of view lifetime. This primitive
provides the bounded data-to-view path, not native Tree/TreeItem widget semantics.

Core leases/branch targets contain only identity/version metadata. The Eio
`controls` adapter revalidates them at effect delivery, respects queue capacity,
publishes ordinary producer/worker failures and ignores stale/closed delivery.
Automatic requests observe visible Ready boundaries after display; explicit retry
is required after failure. Collapse cancellation and background-prefetch policy
remain explicit. Demand callbacks peek at the latest state/policy before acting.

Passing local evidence:

- Bonsai driver: 100,000 selected nodes mount at most four rows with a four-row
  configuration, including native pins. Viewport traversal releases row lifetimes;
  selection and updated application data survive. A streamed payload invalidates
  its one row, and coalesced changes retain the accepted invalidation baseline.
- Separate generation-zero sources replace the native source key, reset transient
  button models and ignore the old reveal controller. Unmount/remount of the same
  source also ignores old controller effects while allowing the new controller.
- Visible boundaries respect the active budget; a queue with one slot remaining
  admits one boundary. Releasing capacity admits the next. Failures do not retry
  automatically, explicit retry works, `auto_load=false` suspends demand and
  collapsed branches cancel queued work.
- Eio mock backend uses real scoped fibers: a burst of 70 control requests runs
  four producers and queues 64; excess demand is backpressure without an exception.
  Foreign/reset/closed targets are ignored. A completed page retires its old
  target; payload updates preserve it. All scopes/producers close after tests.
- Combined Bonsai/Eio test: a viewport starts loading; collapse cancels that
  producer; re-expansion starts another request. Unmount leaves the second producer
  alive, its result publishes to application data, and remount uses the loaded
  child without another request. This uses controlled promises, not timing sleeps.
- A weak-reference test collects a 1 MiB application payload while a captured
  branch target and comparable source lease remain live.

Validation:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune runtest -j2 test/virtual_list test/runtime
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt
git diff --check
```

The full build, expect suites, formatting and whitespace checks pass locally on
macOS arm64. Logs: `tree-bonsai-tests.log`, `tree-bonsai-runtime.log`,
`tree-bonsai-eio-integration.log`, `tree-bonsai-dune-all.log` in the agent's ignored
scratch directory. Initial integration compilation found a missing public module
alias and a test record-label annotation; both were corrected before passing.
No native GUI process, Rust/protocol change, new dependency or hosted CI run was
needed for this layer. Actual native keyboard/AX/typeahead/focus/reveal/move
semantics, the public filesystem provider and native full-traversal/cancellation
checks remain, followed by consolidated hosted gates and merge. No tree
capability is advertised yet.


## Native tree accessibility metadata (2026-09-26)

The managed list can now expose a Tree root and one TreeItem per admitted row.
Core `Tree_rows.Item.accessibility` derives label, hierarchy/sibling position,
optional sibling count/expanded state, selection, disabled and loading metadata.
Bonsai forwards root metadata through the existing virtual-list component. Core
lifts row metadata to its keyed envelope; Rust applies it to the native row focus
owner and suppresses duplicate inner metadata. Synthetic lazy boundaries remain
ordinary content, not application tree items. Existing list semantics are retained.

The paired protocol appends accessibility roles 12/13 and preserves prior tags
and Config layout. Public constructors and Rust decode validate level 1–128 and
sibling index/count bounds up to 100,000; unknown counts stay absent. Independent
byte fixtures, malformed/truncated/trailing input cases and role/kind checks pass.
AccessKit node tests verify flags, position and retained native focus actions.
These existing focus actions are not tree activation/expansion implementation.

Actual macOS production-view test `native_tree` passes:

- AXOutline root and exactly two AXRow items, with no duplicate or boundary rows.
- Root/child disclosure levels 0/1, selected and disabled/enabled state, expanded
  and disclosed state for the branch, and absent disclosure state on the leaf.
- Inert row hide/restore after moving metadata to the native envelope, controlled
  collapse/selection update, child and boundary removal, then complete managed-list
  resource disposal. The harness closes its native window.

The first platform check exposed absent disclosure depth/state in the pinned
accesskit_macos 0.26.3 adapter. `tree-state.patch` adds read-only TreeItem getters
following the existing `expanded-state.patch`. The dependency version is unchanged.
Reconstruction checked the original crate archive SHA256, original file hashes,
ordered application of both patches and exact byte equality with vendored files.
See `vendor/accesskit-macos/GPUIO.md` for provenance and platform references.
This verifies actual AppKit getters, not VoiceOver speech or external notification
observation. Keyboard, activation and accessibility action dispatch remain pending.

Regression evidence: the ordinary native list passes its 100,000-row traversal
and revisit with the existing 256-active-row bound and cleanup. Native navigation
passes after making the test's selection snapshot and inert transition atomic in
one UI update: an auto-scroll tick could previously occur between them. No editor
production behavior was changed for that test timing correction. The navigation
suite includes drag-idle cleanup and the existing 128-page workload. Ordinary list
coverage is not claimed as full native tree traversal acceptance.

Validation commands (local macOS arm64):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 --workspace
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-protocol --test accessibility
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native --features native-tests --test native_tree
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native --features native-tests --test native_list --test native_navigation
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native --features native-tests --test native_navigation
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j2 -p gpuio-native --features native-tests --all-targets -- -D warnings
```

Rust workspace, protocol tests, Clippy and the final native tree/navigation runs
pass. Logs under the agent's ignored scratch directory: `tree-semantics-rust-all.log`,
`tree-accessibility-codec.log`, `tree-semantics-clippy.log`, `tree-native-final.log`,
`tree-semantics-native-regressions.log` (list pass and initial navigation timing
failure), and `tree-semantics-navigation-regression.log` (corrected suite pass).
The CI workflow now builds this test cross-platform and runs it on macOS; hosted
checks and Linux builds have not yet run for this checkpoint. The tree capability
remains unadvertised until the complete interaction/public/native acceptance.

Final full `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt`
passes, including the Core fixture and real Bonsai-driver metadata propagation
check. The focused native inert-row regression and final all-target native-test
Clippy also pass (`tree-semantics-dune-all.log`, `tree-native-inert.log`,
`tree-semantics-clippy-final.log`). `cargo fmt --all` and `git diff --check` pass.
