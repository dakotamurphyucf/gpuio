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


## Pure interaction requests and reveal (2026-09-26)

`Tree_interaction` defines payload-free identity targets, ordered relative requests,
explicit expansion, selection/focus, separate activation, programmatic reveal and
application-approved moves. `Tree_state.reveal` opens loaded ancestors while
preserving selection/range anchor. Outcomes request eventual native reveal/focus;
this layer performs no native operation and implements no keyboard listener.

Expect coverage exercises:

- Multiple relative Next requests from one snapshot accumulate against the latest
  cursor; repeated explicit expansion is idempotent. Hidden/disabled user targets
  and leaf expansion reject; activation does not change selection or focus.
- Reveal opens ancestors, optionally moves the logical cursor, preserves selection
  and anchor, and handles disabled ancestors as programmatic preferences. Collapse
  repairs the active descendant and reports its ancestor for native focus handoff.
- Payload updates, sibling reorder and accepted child pages preserve interaction
  targets. Child-page tokens retire independently. Coalesced deletion/reinsertion,
  reset, absent targets and foreign controller identity reject old commands.
- Moves leave application data and state unchanged until approval. Self/cycle,
  disabled destination and Inside-leaf proposals reject; delayed approval can
  detect collapse or branch-to-leaf conversion through `Move.is_current`.
- A maximum-depth reveal expands 127 ancestors to expose level 128. Captured
  requests/targets remain live while a 1 MiB application payload is collected.

Native typeahead, handlers/AX actions, scrolling/focus handoff, drag sessions,
public Eio filesystem usage and full native tree workload acceptance remain.
No capability or new dependency is added by this pure contract.

Validation passes locally on macOS arm64 with
`GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt`, including the
new expect tests and existing complete suite (`tree-interaction-dune-all.log`).
The first test compile used a weak-pointer module outside Core; it was replaced
with the repository's existing Stdlib.Weak test pattern. The first execution
needed only reviewed multiline expectation whitespace corrections. No native GUI
run or hosted CI is claimed for this pure layer. `git diff --check` passes.

## Opt-in native input bridge (2026-09-26)

Tree input now has a separate explicit opt-in operation (tag 50) and ordered
input event (tag 55). Accessibility metadata alone preserves its prior behavior.
Native admission requires a VirtualList with Tree semantics and a handler; targeted
requests additionally require a current admitted enabled TreeItem. Expansion of
a leaf rejects. Existing transaction/event tags are unchanged.

Core translates native row IDs through the current monotonic list identity map.
Bonsai Virtual_list reuses its existing collection-key map, and Tree_rows checks
mounted lifetime/current projection before constructing Tree_interaction requests.
Relative requests reduce against the latest logical cursor in delivery order.
Disabling/re-enabling input rotates the handler; old requests do not revive.
No duplicate hierarchy, label index or per-node task was added to Rust.

Local evidence collected for this bridge:

- Independent OCaml/Rust fixtures cover all seven request forms, operation/event
  tags, malformed target IDs, truncation and trailing bytes. Existing protocol
  fixtures remain unchanged.
- Native session/mailbox tests cover opt-in/role/handler guards, future revision,
  missing/disabled row, leaf expansion, atomic rejection, disabling input and
  ordered non-coalesced relative requests with pending-window accounting.
- Reconciler tests preserve target identity through reorder, reject a removed and
  reintroduced key's old row ID, rotate input handler epochs and reject on close.
- An actual Bonsai driver reduces queued navigation against latest state, resolves
  native keys to application items and ignores a captured old source callback.
- The macOS production-view test dispatches native GPUI key events for repeated
  Down, Shift-End, Left, Space and Enter. Actual AppKit Focus/Press delegates emit
  focus and selection requests without conflating selection with activation.
- A real text input inside a row keeps its editing keys, marked text and first
  Escape. Its pointer click retains editor focus without emitting row selection.
  The marked-text check enters through macOS NSTextInputClient. GPUI key/mouse
  dispatch is not reported as physical system-keyboard/mouse automation.

The native test found two adapter integration errors and verified their fixes:
render-time fallback focus did not yet recognize the tree surface; and a row
click reclaimed editor focus on mouse-up after the editor had focused on down.
The host now recognizes opted-in tree focus. Native controls within those trees
block ancestor pointer hitboxes while preserving scroll propagation. Keyboard
ClickEvent synthesis is ignored by row pointer handlers because the tree's own
key handler emits the intended request. Root focus is a Tab stop; row handles
retain their existing non-Tab-stop policy. A test-only editor bounds accessor
lets the pointer scenario use actual laid-out input geometry.

This is tested bridge infrastructure, not a completed high-level tree widget.
No public filesystem app or combined native/OCaml example is claimed yet. Delayed
native focus/reveal, Unicode typeahead, drag/move integration and the full native
tree workload still remain. AccessKit Expand/Collapse handlers are registered,
but the pinned macOS adapter currently has no disclosure setters; its generic
selected-state setter also needs a precise true/false contract. Complete external
AX expansion/selection-setter acceptance is therefore still pending. No tree
capability is advertised, and hosted macOS/Linux gates remain deferred with M5.

Final validation for this bridge passes on local macOS arm64 using the isolated
repository toolchain (`GPUIO_JOBS=2 ./scripts/gpuio exec`):

- `cargo test --workspace --locked -j2`.
- `cargo clippy -p gpuio-native --features native-tests --all-targets --locked -j2 -- -D warnings`.
- `cargo test -p gpuio-native --features native-tests --test native_tree --test native_list --locked -j2`.
- `dune build -j2 @all @runtest @fmt`.
- `cargo fmt --all --check` and `git diff --check`.

The complete OCaml build initially caught two exhaustive raw-event matches in
`examples/bridge` and `examples/view_api`; both now explicitly ignore Tree_input,
as those examples do not opt in. The rerun passes. Native list regression traverses
and revisits all 100,000 ordinary list rows with a 256-active-view cap and bounded
caches; this validates shared-list behavior, not the still-pending full tree workload.
Native tree checks also pass ordinary row selection, Enter/Space synthetic-click
deduplication and inherited pointer disabling/restoration. All owned validation
processes have exited. Local logs are `tree-input-rust-all.log`,
`tree-input-clippy-final.log`, `tree-input-native-final.log` and
`tree-input-dune-all-final.log` under the implementing agent's scratch directory.


## Deferred row focus and reveal (2026-09-26)

The explicit Bonsai tree-row reveal controller accepts `~focus:true`. It retains
source/projection checks and uses a new nested Scroll_target tag (3) in the
existing serialled scroll operation. Independent OCaml/Rust bytes cover this
command, positive target/serial validation, truncation and trailing bytes. Core
and native admission reject focus requests on lists without native tree input.
The existing public tree input requests and their encodings remain unchanged.

Native handoff releases the previous row pin by focusing the tree surface, then
waits for the same enabled visible row to materialize. One pending target and two
scoped event subscriptions suffice; there is no idle task/timer. A one-row budget
exposed a real starvation case: a partially visible preceding row consumed the
request budget. The destination now gets priority within the same unchanged cap.
Native row rendering tracks that destination separately from the bounded overscan
request sample, so sample truncation cannot leave it waiting after materialization.

The native_tree suite now additionally exercises the production focus path with
real `apply_guarded` admission and current native retention pins:

- A sparse 100-row logical order and a one-active-row budget, verifying placeholder
  demand without an old focus pin, then actual focus after target materialization.
- A waiting destination reordered to a new position without changing its identity.
- Newer ordinary reveal superseding focus; older/duplicate serials cannot revive it.
- Deletion/reinsertion, handler retirement, blur/return, disabled materialization,
  inert/restore and removal while a command waits.
- Actual second-window activation cancelling the first window's handoff; a new
  request to the inactive window does not activate it. The second window closes.
- Native GPUI wheel dispatch away from the destination cancelling handoff.
- Zero-width layout retiring a waiting command before restore; row focus also
  checks intersection with the list bounds, content mask and window viewport.

The Bonsai driver separately checks native command encoding through the public
controller and rejects a captured effect after source replacement. Reconciler
coverage checks one-time serial emission across reorder and deletion, rejects a
new absent target and reports missing tree input. These tests do not yet constitute
a complete public filesystem app or the full 100,000-node native tree workload.
High-level reduction/reveal wiring, typeahead, complete AX setters, drag/move,
filesystem usage and consolidated hosted validation remain required.

Validation passes on local macOS arm64 in the isolated toolchain
(`GPUIO_JOBS=2 ./scripts/gpuio exec`):

- `cargo test -p gpuio-protocol --test tree_input --locked -j2`.
- `cargo test --workspace --locked -j2`.
- `cargo clippy -p gpuio-native --features native-tests --all-targets --locked -j2 -- -D warnings`.
- `cargo test -p gpuio-native --features native-tests --test native_tree --test native_list --locked -j2`.
- `dune build -j2 @all @runtest @fmt` and both language format checks.

The final native_tree rerun additionally covers clipping/zero-width cancellation.
The full ordinary-list regression passes traversal and revisit of all 100,000
rows with its unchanged 256-view cap and bounded caches. Local logs are
`tree-focus-protocol.log`, `tree-focus-rust-final.log`, `tree-focus-clippy-final.log`,
`tree-focus-native-guarded.log`, `tree-focus-clipping.log` and
`tree-focus-dune-final.log` in the implementing agent's scratch directory.
No hosted CI or Linux GUI acceptance is claimed. All owned checks have exited;
the test windows, including the second activation-test window, are closed.

## Unicode typeahead (2026-09-26)

The native tree input path now sends bounded printable UTF-8 queries with an
event-driven one-second reset policy and a single-grapheme cycling flag. Core
owns the bounded prefix and searches current visible enabled labels using
Unicode 17 canonical caseless matching. No idle timer or persistent label index
was added. The dependency change installs `uunf` 17.0.0 and promotes existing
`uucp` 17.0.0 and transitive `uutf` 1.0.4 to runtime lock requirements; the isolated
switch keeps OCaml, Bonsai and formatter versions unchanged.

Coverage includes canonical composed/decomposed accents, Greek case folding,
sharp-s and ligature expansion, joined emoji, repeated cycling, wrap/extension,
combining marks split across inputs, early-normalization mismatch followed by a
match, current-label replacement, hidden/disabled exclusion, stale source input,
invalid/control/oversized UTF-8 and bounded prefix overflow. A Core test finds the
last of 100,000 loaded nodes, wraps to the first and completes a miss. A real
Bonsai driver delivers queued canonically equal inputs against the latest cursor.
Independent OCaml/Rust bytes append nested request tag 7 without changing earlier
request fixtures. Mailbox tests account for query text bytes and preserve order.

The actual macOS window test dispatches GPUI text-bearing keys for Option-modified
é, decomposed accents, a joined family emoji and multi-grapheme text. It verifies
query contents, reset/cycling flags, navigation reset, shortcut rejection and
embedded-editor key ownership, alongside the existing native tree focus suite.
This is native GPUI dispatch, not physical keyboard automation or a claim that
the tree itself implements an IME editor. The initial test driver incorrectly
dispatched while borrowing its view; moving dispatch to `update_window` fixed
the reentrant-borrow failure. Both test windows close on completion.

All following checks pass on local macOS arm64 in the isolated toolchain with
`GPUIO_JOBS=2 ./scripts/gpuio exec`:

- `cargo test --workspace --locked -j2`.
- `cargo clippy -p gpuio-native --features native-tests --all-targets --locked -j2 -- -D warnings`.
- `cargo test -p gpuio-native --features native-tests --test native_tree --locked -j2`.
- `dune build -j2 @all @runtest @fmt` after the streaming-normalizer optimization.
- `dune build -j2 @test/virtual_list/runtest @fmt` after adding the final normalization regression.
- `opam lint gpuio.opam`, language formatting and `git diff --check`.

The final regression initially assumed that an accent-bearing partial query could
match a differently ordered canonical prefix. The fixture now includes a matching
intermediate label and proves that adding the next combining mark changes the
match after canonical reordering; production behavior was correct.

A local Eio-clock benchmark builds 100,000 root leaves with either `row` or
`Éclair` labels, then measures 20 complete-miss queries (`not present`). Each query
starts with a reset prefix; no native rendering is included. Reusing the two
normalizers per search and stopping on a decisive prefix reduced these measured
costs:

| Labels | Before mean | After mean | Before allocated bytes/query | After allocated bytes/query |
| --- | ---: | ---: | ---: | ---: |
| ASCII | 18.332 ms | 18.202 ms | 14,400,298 | 7,200,322 |
| Unicode | 57.435 ms | 40.360 ms | 162,400,298 | 116,001,098 |

These are local Core latency and allocation measurements, not retained memory,
an end-to-end input guarantee or a CI performance threshold. A complete Unicode
miss still does appreciable work; full native workload acceptance remains open.
Logs live under the implementing agent's ignored scratch folder:
`tree-typeahead-rust-all.log`, `tree-typeahead-clippy.log`,
`tree-typeahead-native-final.log`, `tree-typeahead-dune-final.log`,
`tree-typeahead-regression-final.log` and `tree-typeahead-bench-final.log`.
High-level integration, exact AX setters, drag/move, the public Eio filesystem
example, full native tree workload and consolidated hosted macOS/Linux checks
remain required. No tree capability or Linux GUI acceptance is claimed.

## Exact per-item accessibility setters (2026-09-26)

`Tree_state.set_selected` and `Tree_interaction.Request.set_selected` implement
desired membership rather than a toggle/click. Single mode replaces on selection
and removes only the target on deselection; Multiple preserves other members.
Both preserve cursor and range anchor and request no focus, reveal or activation.
Current visibility, enabled state, incarnation and source lease still gate input.
The Core/Bonsai native route carries nested request tag 8, with independent
OCaml/Rust selection-event bytes and strict target/Boolean decoding.

The pinned macOS adapter adds expanded/disclosed setters for enabled branch
TreeItems with both existing actions. Selection uses a guarded pair of declared
CustomActions because AccessKit 0.24.1 has no SetSelected action. It queues desired
states even if they equal the displayed snapshot, preserving opposite setters
before asynchronous application reduction. Generic Press/Click handling and
unrelated roles retain prior behavior. The third adapter patch is recorded in
`vendor/accesskit-macos/tree-actions.patch`, `UPSTREAM.json` and its provenance
note. Reconstruction from the checksum-verified cached crate, applying all three
patches, matches every recorded source/manifest file byte for byte.

Core expect tests cover idempotent Single/Multiple setters, opposite ordered
requests, unchanged cursor/anchor, hidden/disabled targets and foreign sources.
A real Bonsai driver queues select/deselect/deselect before recomputation and
preserves the cursor. Native semantics advertise private actions only for opted-in
enabled rows. Actual AppKit calls enqueue select/deselect/repeat/select and both
forms of collapse/expand before another application update. The test checks exact
request order and setter availability, including the disabled leaf. Existing
native tests cover metadata changes, inert hiding, removal and teardown.

The broader native controls regression passes. The navigation regression exposed
a separate pre-existing retained-editor race twice: a drag-selection timer changed
the head from byte 432 to 476 after inert admission but before the deferred GPUI
blur listener. `Window::blur` changes focus immediately, while listener delivery
waits until after drawing. The vendored input timer now checks actual window/focus
ownership before extending selection and cancels transient drag state if lost.
The existing regression passes with selection unchanged and no continued idle
renders; the complete navigation and tree suites pass afterward. This is a
production fix, not a relaxed expectation. The cumulative GPUI Base patch and
checksum are updated; checksum-verified reconstruction matches the vendored tree
(excluding local build artifacts). Source versions are unchanged.

The native evidence is local macOS arm64 GPUI/AppKit execution. It does not claim
physical assistive-device interaction, VoiceOver speech, external AX notifications
or Linux GUI acceptance. High-level tree integration, drag/move, public filesystem
usage, full native workload and consolidated hosted validation remain open.

Final local checks pass using `GPUIO_JOBS=2 ./scripts/gpuio exec`:

- `cargo clippy -p gpuio-native --features native-tests --all-targets --locked -j2 -- -D warnings`.
- `cargo test --workspace --locked -j2`.
- `cargo test -p gpuio-native --features native-tests --test native_controls --locked -j2`
  (part of the first combined run, before navigation exposed the timer race).
- `cargo test -p gpuio-native --features native-tests --test native_navigation --test native_tree --locked -j2`
  after fixing the timer race.
- `dune build -j2 @all @runtest @fmt`, language formatting and `git diff --check`.

The first full Dune run found only a missing blank line between fixture stanzas;
the corrected run passes. Logs are `tree-ax-native-final.log` (controls pass,
initial navigation failure), `tree-ax-navigation-repeat.log` (reproduction),
`tree-ax-native-fixed.log` (navigation/tree pass), `tree-ax-clippy.log`,
`tree-ax-rust-all.log`, `tree-ax-dune-final.log` and `tree-ax-vendor-final.log` in
the agent's scratch folder. Both vendor reconstructions match; GPUI Base compares
all 226 files excluding local Cargo.lock/target artifacts. All checks exit and
native windows close. Hosted macOS/Linux gates remain pending with milestone 5.

## Public Bonsai widget and Eio filesystem example (2026-09-26)

`Gpuio_bonsai.Tree.component` now composes the managed-row primitive, current
source/state reducer, default presentation and deferred reveal/focus. It owns
preferences for a mounted source generation, while the application owns loaded
data and its Eio loading scope. Controllers carry payload-free targets and ignore
stale generations/incarnations. Initial preference inputs seed a generation once;
mode remains controlled. Custom content receives guarded row lifetimes inside
widget-owned indentation, disclosure, selection and TreeItem semantics.

Ten Bonsai expect tests cover ordered native requests, post-display reveal,
supersession/reorder, deletion/recreation/reset, payload collection while retaining
a controller/target, initial seeds and mode changes, 100,000 selected nodes with
four active rows, custom-row eviction/remount, disclosure cancellation and queued
relative toggles, loading/retry/collapse and bounded first-page admission.
The new Core relative-toggle request changes no protocol tag.

The public `examples/tree` application reads actual directories through Eio and
uses 128-entry pages, four existing loader workers and 32 active rows. Symlinks
are leaves. The example documents path/metadata limits, directory-read allocations
outside native budgets and the lack of filesystem watching or stable snapshots
across directory mutations. It does not perform filesystem moves.

The native self-test found two issues during development. First, waiting for a
loader snapshot was too early to capture a target from the displayed widget;
the test now waits for the displayed projection. Second, opening a directory near
the viewport bottom could leave its first lazy boundary offscreen and request no
children. The widget now explicitly requests newly opened eligible Ready branches,
limited by available queue slots and the active-row budget. An expect regression
opens eight ancestors without a viewport: automatic loading admits four pages,
and disabling it admits none. Later-page demand still follows the viewport.

Local macOS arm64 validation passes:

- `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt`.
- `GPUIO_JOBS=2 ./scripts/gpuio exec dune exec --no-build examples/tree/main.exe -- --self-test`.
- `git diff --check`.

The native test loads this checkout's root, `test` and `test/virtual_list`, then
reveals a file and waits for its actual native focus-retention pin. It checks
bounded rows, selection, a source reset and stale-command retirement, requests a
frame acknowledgement and shuts down. Its output is:

```
TREE_WIDGET_PASS eio_filesystem=true lazy_children=true native_focus_pin=true bounded_rows=true selection=true reset=true
```

Scratch logs are `tree-widget-full-check.log` and `tree-widget-native-4.log` under
`scratch/agents/root-20260925-resumed/`. All processes exit and the native window
closes. This adds actual OCaml/Eio/Rust integration evidence; it does not claim
physical pointer/keyboard/assistive-device input or the full native 100,000-node
workload. No Rust source changed in this slice; previous native input/AX checks
remain recorded above. Hosted gates have not run for this checkpoint.

OCH-38 remains in progress. Native drag/application-approved moves, context-action
example integration, visible focus presentation review, and full native large/deep
traversal/revisit/collapse/deletion/loading/drag acceptance remain before capability
advertisement and final ticket closure. The example's current loading/focus checks
are not a substitute for that remaining acceptance.

## Native move proposals (2026-09-26)

The high-level tree now exposes reactive `allow_moves` (default false), and
`Controller.propose_move` provides the explicit keyboard/menu alternative. Native
moves use the existing managed-list IDs and asynchronous tree request route.
Operation tag 51 enables moves; nested request tag 9 carries two row IDs and a
placement. Independent OCaml/Rust fixtures cover both messages, malformed IDs,
self-targets, invalid placement/Boolean tags, truncation and trailing bytes.

Reconciliation requires native tree input and rotates its handler when move
policy changes. Dispatch rejects disabled-policy moves and missing endpoint keys.
The Bonsai adapter resolves both endpoints against its current projection/source,
rechecks current policy and calls the existing pure reducer. Tests prove valid
proposal delivery without hierarchy mutation, invalid/self/descendant/leaf-inside
rejection, disabled policy, hidden children after collapse, generation reset,
handler retirement and removed/reintroduced keys. `Move.is_current` remains the
application's approval-time revalidation contract.

Rust's GPUI drag snapshot holds native route identity and a bounded label only.
One preview owns a gesture lease; the host and source descriptor hold weak
references. Source/destination checks include window/list/handler, mounted row
binding, current input/move policy and visible/enabled semantics. Drop placement
uses current measured row bounds: branch quarters select Before/Inside/After,
while leaf halves select Before/After. Only the final proposal crosses the bridge.
There is no task, producer, hierarchy copy or per-motion OCaml callback.

The actual local macOS GPUI tree suite passes:

- Native drag startup and all three branch placements, exact stable endpoint
  requests, no implicit hierarchy update and preview-lease disposal after drop.
- Escape, move-policy retirement, handler replacement, source disabled/inert state
  and actual source-row deletion during collapse cancel dragging without a timer.
- Embedded-editor pointer selection does not start a parent tree drag with moves
  enabled; existing child keyboard/IME, AppKit setters and focus suites also pass.

The expanded test initially attempted to detach a row without removing its native
nodes. Admission correctly rejected the orphaned tree. The test now exercises
cancellation during the existing valid collapse/deletion transaction. An earlier
compile error placed an editor probe before its point binding; that test-only
placement is corrected. No production invariant was relaxed.

Scratch native evidence is `tree-move-native-4.log`; all tests exit and windows
close. This uses actual GPUI mouse dispatch in a real macOS window, not physical
mouse hardware or an end-to-end public OCaml drag app. Existing Core/Bonsai tests
and independent protocol fixtures separately cover the receiving half. These
checks do not claim cross-window, window-deactivation/close or full traversal
acceptance for tree drag; those remain in the broader workload/public-example work.

Current limitations are explicit: one row, same tree/window, no desktop export,
no automatic hover expansion/loading or edge autoscroll. Existing focus pins share
the active-row budget; no separate drag pin policy is added. The filesystem demo
remains read-only with native moves disabled. Public approved-move/context-action
examples, focus presentation review and full native large/deep traversal/revisit
acceptance remain before OCH-38 closure or tree capability advertisement.

Final local checks pass with the repository's isolated toolchain and two jobs:

- `cargo test --workspace --locked -j2`.
- `cargo clippy -p gpuio-native --features native-tests --all-targets --locked -j2 -- -D warnings`.
- `cargo test -p gpuio-native --features native-tests --test native_tree --locked -j2`.
- `dune build -j2 @all @runtest @fmt` and `git diff --check`.

Commands use `GPUIO_JOBS=2 ./scripts/gpuio exec`; Cargo and Dune native linking
were serialized. Final logs are `tree-move-rust-all.log`,
`tree-move-clippy-final.log`, `tree-move-native-final.log` and
`tree-move-dune-all.log` in the agent scratch directory. Processes are reaped;
no test window remains. Hosted macOS/Linux gates and merge remain pending.
