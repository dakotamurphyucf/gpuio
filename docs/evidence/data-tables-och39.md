# OCH-39 data-table evidence

Status (2026-09-26): **public Core/Bonsai/Eio and retained native table implemented;
local AppKit semantics/input, public context flows and full native history checks pass**.
OCH-39 remains In Progress pending its final acceptance audit and consolidated
platform gates. Earlier checkpoint sections retain their historical scope; the
latest input/context evidence is at the end. The [local acceptance audit](data-tables-och39-audit.md)
now supports managed-table bit 40 (`1099511627776`), aggregate `2199023255551`. The
[design](../design/data-tables.md) lists the remaining production acceptance.

## Column schema

`lib/core/table_column.{ml,mli}` and `test/view_api/table_column_test.ml` cover:

- Stable case-sensitive Unicode keys, invalid UTF-8/NUL/size rejection, duplicate
  columns and duplicate/empty group membership.
- Finite size constraints, native resize clamping, programmatic changes to
  non-resizable columns, unknown IDs and preservation of the original value.
- Keyed grouped-header membership through valid moves and resizing; rejection
  of split groups, crossed pin partitions and non-refining nested headers.
- Every source and destination for 64 columns: **4,160 moves**, checking identity,
  destination placement and unchanged relative order of the other columns.
- Empty schemas, column/header/text limits, locked source moves and stale targets.

Local macOS command: `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2
@test/view_api/runtest` — **PASS**. Initial development failures were a test's
integer-specialized equality applied to an option (fixed to `Option.equal
Int.equal`) and expected sexp line wrapping (reviewed and corrected manually).
No expectation auto-promotion was used.

The consolidated local command `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2
@all @runtest @fmt` also **passes**. The build reports the existing macOS linker
duplicate-library warnings. This covers OCaml builds, expect/runtime suites and
formatting; it is not a new run of every native GUI suite.

## Data and paging resources

`Table_data` now has typed row IDs, immutable revisioned data and payload-free
membership references. The Core tests traverse/reorder **100,000 rows**, preserve
an anchor at its changed numeric position, verify no value invalidation on reorder,
and perform 100 point updates sharing the order snapshot. Separate removal and
reinsertion retire a reference; a foreign source or independent branch addition
cannot impersonate it. Weak probes prove replaced and removed payloads are
collected while an obsolete row reference remains live. Malformed IDs, duplicate
and invalid structural operations, empty ranges, row-count and key-byte budgets
are covered.

Core `Table_paging` tests load **100,000 rows in 49 bounded pages** after a one-row
seed. They cover simultaneous boundaries, current-query payload/column changes,
sort/reset with a surviving stable anchor, foreign/duplicate/obsolete responses,
explicit retry, canceled and malformed pages, cursor/error limits and atomic
failed resets. Old responses are discarded before payload admission; a malformed
old page cannot fail the current query. Stored error text remains bounded UTF-8.

`Gpuio_eio.Table_paging` is tested with the project's deterministic Eio mock
backend and real Scope/Inbox/Bonsai effect delivery:

- Query values are captured in requests; producer results publish only through
  the UI inbox. A failed boundary does not retry on repeated ordinary demand.
- Both old producers wait in protected cancellation cleanup while **100 query
  resets** request both boundaries. Only queries 0 and 100 actually start: four
  total producer calls, **peak concurrency two**, all four finish. Intermediate
  pending queries are superseded without growing a worker or request queue.
- With a **one-entry UI inbox**, old-sort results are queued or blocked when
  reset occurs. Only new-sort rows are ever published.
- An invalid reset preserves old running producers. Canceling one boundary and
  then closing twice finishes both producers; an unrelated task in the same
  scope remains alive. Parent-scope shutdown cancels it separately.
- A one-task scope exercises worker admission failure, explicit retry using the
  available worker, and parent-scope cleanup/closed-controller rejection.

Commands `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2
@test/view_api/runtest` and `... dune build -j2 @test/runtime/runtest` **pass**.
Development corrections were expected UTF-8 sexp escaping and a test-local
promise/function naming collision. These results establish Core/Eio contracts;
they do not establish table-native paging, stable visual anchors, input or AX.

## Styled-library compile probe

This historical probe informed the extracted adapter selected below; it is not a
production dependency.

Upstream gpui-kit commit `84f57fdfcb4910623fb0bb7f795b077e249f9271` was read
from a clean local checkout and archived into a new ignored scratch workspace.
The probe uses the project's Zed GPUI commit
`a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b`, existing vendored base and Rust
1.97.1, with two build jobs. Production manifests, lockfiles and OCaml switches
are unchanged. Optional styled features such as tree-sitter languages were not
enabled.

`cargo check -p gpui-component` initially fails with 14 diagnostics from seven
chart `IntoPlot` derives, because their macro resolves package `gpui-pre` or
`gpui-kit`, not `gpui`. Adding a fallback to `crate_name("gpui")` in the isolated
macro copy makes the same default-feature library pass. This is a crate-name
compatibility patch; it is not evidence of a table rendering defect or a reason
to change GPUI pins. The probe lock independently resolves extra dependencies.

The committed [workspace generator](../../scripts/probe_table_adapter.py) records
all probe manifest/source changes. Its output was compared byte-for-byte with
the compiled root manifest, component manifest, patched macro and native example.
It copies committed upstream content, refuses an incorrect source revision or
existing output directory, and writes only beneath this checkout's `scratch/`.

With a local checkout at that upstream revision:

```sh
python3 scripts/probe_table_adapter.py \
  --source /path/to/pinned/gpui-kit \
  --output scratch/table-candidate --macro-fallback
GPUIO_JOBS=2 ./scripts/gpuio exec cargo check \
  --manifest-path scratch/table-candidate/Cargo.toml -p gpui-component -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo build \
  --manifest-path scratch/table-candidate/Cargo.toml \
  -p gpui-component --example gpuio_table_probe --locked -j2
```

Omit `--macro-fallback` into a different fresh directory to reproduce the initial
macro failure. Run Cargo/Dune serially; the wrapper shares the checkout's target
cache. The initial check resolves the scratch lock; subsequent commands use it.

## Native candidate measurements

The committed [probe fixture](fixtures/table_adapter_probe.rs) constructs the
actual styled `TableState`/`DataTable` with **100,000 logical row keys**, **64
columns**, two left-pinned columns and a 720 × 360 logical-pixel macOS window.
Its Rust delegate reads retained strings or a placeholder, without any OCaml
callback. A stand-in producer delivers formatted Unicode strings between sample
positions, outside the render callback. It is not the production paging adapter.

The native executable was run under a 60-second subprocess timeout. It passed,
closed its window and exited normally. It does not activate the application and
does not claim foreground keyboard, clipboard or accessibility validation.

| Target (zero-based row, column) | Distinct rows rendered | Unique body cells over three redraws | Columns observed, including pinned cells |
| --- | ---: | ---: | --- |
| 0, 0 | 11 | 66 | 0, 1, 2, 3, 4, 5 |
| 50,000, 32 | 12 | 80 | 0, 1, 2, 30, 31, 32, 33 |
| 99,999, 63 | 12 | 69 | 0, 1, 2, 61, 62, 63 |
| 0, 2 | 11 | 66 | 0, 1, 2, 3, 4, 5 |

The fixture asserts that the target cell was rendered, the working set is
nonempty, and each sample stays below 4,096 unique cells. The peak observed was
80 cells. Previous/measurement positions can occur within the three-frame union.
These four samples establish native vertical/horizontal virtualization behavior
for this candidate and geometry. They do **not** establish full-history bounded
retention, GC/resource release, paged sort races, stable selection, all geometry,
production FFI integration, Linux GUI support or complete OCH-39 acceptance.

Run the built candidate with an external timeout when reproducing:

```sh
python3 - <<'PY'
import subprocess
subprocess.run(["target/debug/examples/gpuio_table_probe"], check=True, timeout=60)
PY
```

The fixture closes on caught assertion failures as well as success. The external
timeout also kills and reaps its own process if initialization fails to progress.

## Extracted native adapter

`rust/table` now contains the selected, attributed extraction. Per-instance
appearance replaces the global styled theme; the existing base supplies native
virtual lists, scrolling, scrollbars and actions. No GPUI pin changed, and the
workspace lock only adds the local adapter package. The full styled crate and
its extra dependencies are not linked by the adapter.

Local macOS commands, using the isolated toolchain:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -p gpuio-table-adapter \
  --all-targets --features native-tests --locked -j2 -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-table-adapter \
  --locked --features native-tests --test native_table -j2
```

Both **pass**. The native test reproduces the four samples above with the same
peak of 80 distinct body cells. It then selects row key 50,001 / column `col-32`,
reverses 100,000 rows and the unpinned columns, and verifies selection and retained
Unicode text follow their keys. Removing the selected row or column clears the
selection. Invalid replacements retain the prior selection; reconciliation emits
no user-selection echo. Closing the window releases the table entity, verified
through a weak handle. The window closes and the process exits normally.

These are direct native command/state/layout checks, not foreground keyboard,
clipboard, context-menu or accessibility acceptance. Unicode retained text is not
proof of Unicode clipboard behavior. Only sampled working sets are measured;
full-history cache bounds and the public OCaml/Bonsai bridge remain outstanding.
CI now includes adapter compile/lint on both platforms and the macOS native run;
those hosted jobs have not yet been executed for this change.

The local workspace `cargo test --workspace --locked -j2`, Dune
`build -j2 @all @runtest @fmt`, and `./scripts/gpuio lint` also pass. These
checks do not rerun every native GUI suite or establish Linux runtime support.

## Stable native events and obsolete layout input

The same native test now runs `tests/support/events.rs` through GPUI's real
window event dispatcher. Painted bounds come from the base's existing optional
test observation feature. It verifies:

- A fresh cell click captures its row/column keys. Replacing row and column
  order, then dispatching double/context clicks **before repaint**, emits no
  event and leaves selection empty. Previously captured events retain old keys.
- Fresh double/context clicks after repaint report the new row's stable key.
- Clicking the sort indicator emits a keyed sort request and does not reorder
  retained data locally.
- A native resize drag is active before source/schema refresh. After repaint,
  continuing/releasing that old drag emits no width update. A fresh resize emits
  widths paired with the correct column keys.
- An active column-reorder drag is likewise retired by refresh. A fresh reorder
  emits stable source/destination keys, updates retained order and remaps the
  selected cell to the moved column's new position.

The native run and feature-enabled Clippy **pass locally on macOS**. Its test
window closes and weak table-entity release still passes. These synthetic native
pointer events are stronger than direct state calls, but do not constitute
foreground keyboard/clipboard or macOS accessibility acceptance. Paged producer
races, full-history cache bounds and the public bridge remain unimplemented.

## Native source updates, anchors and column gestures

`TableState::update_source` captures keys and pixel offsets before updating
retained data. The native test reverses **100,000 rows and 62 unpinned columns**
while the first visible row is clipped by seven pixels and the first scrolling
column by seventeen pixels. After actual layout/paint, observed element origins
remain within 0.1 logical pixels of their previous positions, and the retained
keys are unchanged. It also verifies:

- Prepending 100 rows preserves the visible row and horizontal position.
- Removed row/column anchors use the old position as fallback; empty data/schema
  resets both scroll offsets. Revealing a pinned column does not scroll the
  unpinned region.
- A pending row command follows its key across reversal, overriding the painted
  anchor; removing its target cancels the command without selecting a neighbor.
  A new explicit command after replacement overrides restoration.
- Row arrival during an active native resize preserves the preview width and
  allows completion. Row arrival during active reorder likewise allows the
  keyed move and preserves selection. A schema replacement still rejects old
  gestures, as covered by the preceding regression.
- Explicitly reapplying unchanged schema clears optimistic sort/resize state
  without emitting another input event. A following sort click starts from the
  retained sort value, and a resized header returns to the retained width.

Local macOS `GPUIO_JOBS=2 python3 scripts/test_table_adapter.py` **passes**, with
all sample, selection, pointer, anchor and entity-release completion markers.
The wrapper also deliberately fails an assertion after opening a window and
requires a nonzero exit plus native entity-release evidence on the failure path.
Timeout handling kills and reaps the wrapper's process group. CI now uses this
wrapper; no hosted execution is claimed yet.

During development, a missing painted-element assertion exposed that GPUI quit
could terminate the process successfully before code after `Application::run`
rethrows a stored panic. Failure handling now exits unsuccessfully after window
cleanup inside the async callback. Earlier recorded successful runs included all
their completion markers, but exit status alone was insufficient. The fixture
also needed to target the first unpinned column explicitly when resetting scroll,
and to deliver a second drag motion before asserting preview width: the first
motion starts GPUI's drag but does not resize yet. The final run includes all new
and existing assertions and verified failure reporting.

These tests mutate retained native data during gestures; they do not establish
end-to-end Eio producer races, the public table bridge, full-history cache bounds,
foreground keyboard/clipboard, accessibility or Linux GUI acceptance.

## Paired payload codecs and Core configuration

`Table_wire` and Rust `protocol::table` now validate the same column schema,
configuration, retained cell copy text, keyed selection, commands and requests.
The manual Rust decoder bounds container lengths before allocation and tracks a
shared schema text budget while parsing. Four independent `table-*.hex` fixtures
fix exact field order, enum tags, UTF-8 text and floating-point encodings; both
OCaml and Rust writers match them, and the Rust decoder reconstructs their values.
An additional combined fixture checks all ten command cases and nineteen request
cases across the two languages, including every selection and sort alternative.

Local commands `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-protocol
--test table --locked -j2` and `... dune build -j2 @test/view_api/runtest` **pass**.
Coverage includes every command/request family, truncation at every byte of the
four fixtures, trailing bytes, unknown tags, malformed UTF-8, nonfinite widths,
oversized counts, duplicate identities, group partitions/refinement/pin crossings,
schema text limits, empty/max/oversized copy text and invalid selection targets.
The hard budget accepts 64 columns × 256 rows at 16,384 cells and rejects one
extra row, column or cell allowance. Huge signed row-budget values are rejected
before multiplication. Core schema conversion agrees with the wire validators.

The public pure `Table.Config` tests cover bounded defaults, automatic reduction
of the default row budget, rejection of incompatible explicit budgets, immutable
accepted sort/schema changes, labels/geometry and private revision validation.
The first expect-test build needed qualified constructors at a list concatenation;
the correction changed no expectations. No test expectations were promoted.

Consolidated local macOS validation also passes: `GPUIO_JOBS=2 ./scripts/gpuio
exec cargo test --workspace --locked -j2`, `GPUIO_JOBS=2 ./scripts/gpuio lint`,
`GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt`, and
`GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all --check`. After adding the
combined all-intents fixture, the complete Dune gate and targeted Rust table
suite pass again. This checkpoint required no additional native windows and
makes no new hosted or Linux GUI validation claim.

At that checkpoint these were payload/model tests. The following checkpoint adds
native transaction/session admission; host rendering, public Views and the Bonsai
component remain outstanding. No table capability is advertised.


## Native transaction and session admission

Table configuration, cell metadata and commands now have paired transaction
operations; native input has a paired event carrying schema/query generations.
Independent transaction/event fixtures fix the appended tags and payload order.
OCaml event decoding and Rust message decoding reject truncated envelopes; OCaml
also rejects trailing event bytes and invalid generations/selections.

Native tree tests construct 100,000 logical rows with one materialized row and
four retained nodes. They exercise schema revision rules, query reset with
handler replacement, exact list/table configuration agreement, row/cell ownership,
copy-text byte accounting and rollback of a refused update. Dirty descendant
edits revalidate the unchanged table root. Command tests target offscreen logical
rows, validate against final transaction data regardless of operation order,
require increasing serials and roll back earlier serials when a later action
fails. Commands disappear from later transaction results.

Session tests cover stale schema/query/handler revisions, unknown row/column
identities, resize limits, sortable policy, disabled input and closed windows.
A query reset retires old viewport handlers even when logical order is unchanged.
Evicting active cells frees their retained copy bytes while retaining the logical
row index; deleting the logical rows then retires their input targets. Pure column
move tests preserve group membership and reject split groups/pins and locked
sources. Mailbox tests check encoded-size accounting for 64 long column IDs,
ordered non-coalesced requests, count saturation and bounded draining.

The first native test run found that table parent checks ran before structural
validation had assigned final parents. Moving ownership validation after that
step fixed the issue; the native tests then passed. No test expectations were
promoted. These are headless transaction/session tests, not GPUI renderer,
clipboard, focus, accessibility, full-history cache or public Bonsai acceptance.
The native host still needs to consume the table specialization and accepted
commands. No new platform GUI acceptance or capability is claimed.

Local macOS validation passes:

- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --workspace --locked -j2`.
- `GPUIO_JOBS=2 ./scripts/gpuio lint` (opam lint, full Dune build and workspace
  Clippy with warnings denied).
- `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt`.
- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all --check`.
- The final `cargo test -p gpuio-native --test tables --locked -j2`, through the
  same wrapper, passes all five table tests after removing redundant copy-text
  scans from ancestor validation.

The full build also caught two exhaustive event matches in the low-level bridge
and View examples; both now explicitly ignore table events they do not register.
No additional native windows were needed for this checkpoint. Hosted macOS/Linux
validation and merge remain part of the full milestone gate.


## Retained native host rendering

`rust/native/src/table_view.rs` connects admitted table roots to the extracted
GPUI table entity. The delegate reads current Rust cell Views and logical row
indices; the ordinary list state is not allocated for table roots. The native
host executes admitted commands and publishes viewport demand from real body
bounds after prepaint. Shared retention guards include table focus and active
row handles. Native input captures its route through a delegate hook before
GPUI defers subscriber delivery.

`GPUIO_JOBS=2 python3 scripts/test_table_host.py` passes a real background-window
host scenario:

- 100,000 logical rows, 12 materialized rows, two columns including a left-pinned
  column and horizontal overflow. A retained text View actually paints through
  the production host; active cell descriptions stay bounded.
- Initial viewport demand and scrolling to row 50,001 at a seven-pixel offset;
  missing rows are requested and then materialized using the bounded node pool.
- Admitted keyed cell selection and row/column reveal. Changing fixed row height
  from 32 to 48 preserves row 50,001 and its seven-pixel intra-row offset.
- Focus registration after the shared frame begins, selected-row retention and
  immediate native input captured with the current query. A saved old route is
  rejected after query/handler replacement.
- Live pointer and disabled policy changes reject input immediately before the
  next paint, including enabling input again after a policy change.
- Single-row and empty sources publish correct viewport ranges without depending
  on upstream measurement callbacks. Removing the root releases the native entity
  and returns retained-tree accounting to zero.

The first background run timed out waiting for a display-link frame. It closed
its window and returned a failing process. The harness now explicitly drives
actual GPUI `window.draw` layout/paint, matching the existing extracted-adapter
harness; this keeps the window in the background. It does not establish foreground
keyboard/IME/clipboard behavior. The wrapper bounds process lifetime, kills and
reaps its process group on timeout, and requires an explicit completion marker.

`GPUIO_JOBS=2 python3 scripts/test_table_adapter.py` also passes all existing
sampled virtualization, keyed selection, pointer/column events, scroll anchors
and entity-release checks, plus its intentional failure-exit/cleanup test. The
new native delegate hook preserves existing GPUI event subscription behavior.

The native Cargo dependency and both workspace/example lockfiles add only the
local `gpuio-table-adapter` edge/package; upstream dependency pins are unchanged.
The macOS CI workflow includes the host harness for the later consolidated run.
No hosted execution is claimed yet. Public Core/Bonsai views and callback dispatch,
clipboard/context UI, foreground keyboard, native accessibility, full-history
cache and end-to-end Eio paging acceptance remain outstanding; no table capability
is advertised.

Final local host-integration checks pass: the full Rust workspace tests, workspace
all-target Clippy with warnings denied, Dune `@all @runtest @fmt` (including the
locked external extension consumer), `cargo fmt --all --check`, and the final
expanded native host harness. The external-consumer lockfile initially lacked
the new local adapter dependency and Clippy caught a unit-valued prepaint binding;
both were corrected before those final checks. These checks establish this native
host checkpoint, not the remaining public widget or full OCH-39 acceptance.

## Core retained View and typed input integration

The Core adapter now constructs bounded table row/cell Views and reconciles them
through the native table envelopes. `Table.Cell` retains validated copy text;
selection/request/target/command types keep row and column identities separate.
The reconciler assigns accepted schema revisions, rotates query handlers, resolves
logical row IDs, validates current input policy and admits ordered command batches.

Six expect scenarios in `test/view_api/table_view_test.ml` pass:

- A 100,000-row logical order with one active two-cell row creates six nodes and
  two copy descriptors. A streamed text update changes only text/copy metadata;
  retention resolves the offscreen-capable logical identity correctly.
- Old schema/query/handler input and removed/reincarnated rows are rejected;
  current keyed input reaches the callback and window closure retires delivery.
- Ordered commands target unmaterialized rows, execute once, and preserve their
  monotonic serial across omitted batches. Invalid targets, offsets or query
  generations fail preparation without consuming serials.
- Resize/sort/selection requests obey current column, mode and disabled policy;
  duplicate/invalid widths and invalid UTF-8/NUL/oversized copy text are rejected.
- Missing, duplicate, misordered, foreign and over-budget rows are rejected.
  Table/list specialization changes allocate a fresh native root identity.
- Column reorder preserves keyed cell nodes. A failed schema/command preparation
  does not consume the next schema revision; unchanged views produce no diff.

The final local command
`GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt` passes, including
all existing Core/Bonsai/runtime tests and examples. These new tests establish
OCaml View/reconciliation behavior, not a public Bonsai table, foreground native
interaction or Linux GUI acceptance. The existing native host evidence above is
separate. The managed presenter, public example, remaining native table acceptance,
showcase and consolidated hosted gates/merge remain; no capability is advertised.

## Public Bonsai presenter, Eio controls and Table Lab

`Gpuio_bonsai.Table` now exposes managed `component`/`paged`, typed cell renderers,
row-membership targets, selection observations and bounded controller batches.
`Gpuio_eio.Table_paging.controls` supplies generation-checked request/retry/cancel
effects. Source identity/order metadata contains no payloads and survives point
updates by sharing. Row keys include membership incarnation, including a removal
and reinsertion coalesced before a displayed frame.

The component tests pass bounded active cells, keyed streaming, native selection
observations, column-removal repair, query-local cell lifetimes, stale controllers
and native effects, once-only command batches and source-payload release while
old controllers/effects remain reachable. A supersession regression confirms that
a selection batch replaced before display cannot publish an unexecuted selection,
and that newer native selection wins over an older pending batch. Query resets
rotate the native callback while preserving the same native table root.

The full-history Bonsai workload visits every one of 100,000 rows twice with two
columns, allocating 400,000 non-default cell payloads over those visits. It checks
at most 200 active cell payloads at collection samples, zero retired payloads
after eviction, and retained heap growth below 200,000 words over its initialized
baseline. These are OCaml/Bonsai lifecycle and heap checks; they do not establish
a full-history native GPUI cache bound.

The public Eio integration test passes empty cursor-advancing pages without a new
frame, waiting for layout after a nonempty page, explicit failure retry and ignored
obsolete/closed paging controls. Only Ready boundaries trigger automatic work.

The final local macOS public native command
`GPUIO_JOBS=2 python3 scripts/test_table_public.py` passes and closes its window:

- 100,000 logical rows, three columns, at most 96 retained active cells, initial
  native viewport demand and actual public View submission.
- A selection/scroll batch reaches row 50,000 at nine pixels within the row;
  changing row height from 32 to 44 preserves that anchor.
- Reversing the same source under a new query preserves the row anchor and
  reports its new visible index 49,999. A captured old-query reveal is ignored.
- Streamed Unicode text appears in the accepted retained cell description,
  followed by a native frame acknowledgment; selected-row removal repairs the
  public selection.
- An empty new source triggers a scoped failing producer, waits for explicit
  retry, then paints the loaded rows. Closing during another producer cancels it
  and releases all cell lifetimes (three producers started and finished).

The native run permits activation for reliable frame delivery and does not inject
physical keyboard input or establish clipboard/IME/accessibility acceptance. Its
wrapper requires the completion marker and kills/reaps the process group on a
120-second timeout. The CI workflow includes this public harness for the later
consolidated macOS run; no hosted execution is claimed here.

Final `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt` passes.
The first full check found only formatting of the new example's Dune stanza; it
was corrected before the final all-pass run. Contracts and runnable commands are
in the [table design](../design/data-tables.md) and
[Table Lab README](../../examples/table/README.md).

OCH-39 remains in progress: complete native keyboard/copy/context/accessibility,
presentation/style and lifecycle acceptance, full-history native cache checks,
resize/reorder/sort during paging, and the remaining public interactions. The chat
showcase, consolidated platform gates and merge remain milestone deliverables.
No table capability bit is advertised yet.

## Native keyboard, clipboard and shared command targets (2026-09-26)

The retained host test now injects GPUI pointer/key events into the rendered
100,000-row table, reads the actual OS clipboard and checks queued input envelopes.
It verifies pointer-acquired table focus, arrow navigation, ordered keyed Enter/
Shift-F10/Copy events, and Unicode including a joined family emoji, combining
accent, tabs, quotes and a newline. One cell copies verbatim; row and complete
12-row column exports use quoted TSV. Copying an unmaterialized cell or a column
with 100,000 logical rows and only 12 retained rows preserves the previous
clipboard and emits the keyed intent.

A real command-button composition verifies that Tab leaves the table and toolbar
Copy restores the remembered table target. A native editor inside a cell keeps
pointer focus, navigation/submit/context keys and clipboard commands. This test
found and fixed ancestor cell-click focus theft by reusing the managed-tree
pointer barrier for table descendants. Disabling the table blocks the retained
editor's command/input gates immediately. Hidden-table callbacks from an older
visible frame also refuse input and preserve the clipboard. The harness restores
the original clipboard and closes its window on success or assertion failure.

The TSV encoder separately tests UTF-8 byte limits, quoting overhead and separators
at the 1 MiB boundary. It returns no partial output. Copy reads retained metadata
only, without a synchronous OCaml call or history-sized export allocation.

Local passing checks:

- `GPUIO_JOBS=2 python3 scripts/test_table_host.py` — native input/command markers
  plus existing retained View, keyed command/anchor, empty-source and release checks.
- `GPUIO_JOBS=2 python3 scripts/test_table_adapter.py` — sampled virtualization,
  native column/selection/anchor regression and verified failing-process cleanup.
- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --workspace --locked -j2`.
- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --workspace --locked -j2 --all-targets --features gpuio-native/native-tests,gpuio-table-adapter/native-tests -- -D warnings`.

This is real GPUI dispatch and OS clipboard evidence in a background macOS window,
not physical AppKit keyboard, IME or accessibility acceptance. Context request
identity is covered; application context-menu presentation still needs a public
interaction flow. Style forwarding, native full-history cache measurements,
resize/reorder/sort during paging and remaining OCH-39 acceptance are still open.
No hosted results or table capability advertisement are claimed.

The existing `native_controls` and `native_tree` regression suites also pass
locally with `cargo test --locked -j2 -p gpuio-native --features native-image-tests
--test native_controls --test native_tree` through the isolated wrapper. Controls
include menus/palette, editor clipboard, native focus/AX and IME scenarios. Trees
include two complete 100,000-row traversals and existing child-editor, selection,
move and disposal checks. This is regression evidence for shared focus/command
changes; it does not transfer those widgets' AX/IME acceptance to the table.
The final table host rerun passes after clipboard-restoration cleanup.

## One public styled table surface (2026-09-26)

The public table now applies its key and style to its native root. Expert-only
source identity participates in reconciliation separately, preserving full-length
caller keys and replacement on a fresh data lineage without an extra layout box.
Expect tests verify a 256-byte caller key, style updates without native replacement,
fresh-source replacement under that same key, and keyed sibling reorder of two
tables sharing one source without replacing either native table. Existing Bonsai
and paging tests pass against the direct root.

The retained host paints its surface/border/radius once, with transparent native
header/body layers and inherited root text. Actual GPU readback verifies:

- A half-alpha red surface blends once over white in header, body and padding;
  border and rounded-corner pixels prove a single box and correct clipping.
- A gradient reaches header/body; contrasting native child backgrounds verify
  that the horizontally scrolling column cannot overpaint the pinned cell.
- Foreground inheritance, focus < hover < press precedence, disabled suppression,
  and restoration of default appearance after removing refinements.
- Selected cells and whole rows retain visible text. The stronger check exposed
  an opaque selection overlay covering the text. Highlights now paint behind
  content, with only a transparent outline overlay above it.

These assertions use actual native layout, GPUI pointer/focus dispatch and GPU
pixels at a controlled scale factor of 1. They do not claim physical monitor-scale
changes or table accessibility acceptance. `scripts/test_table_host.py` now runs
with `native-image-tests` and requires the style completion marker. An optional
absolute `GPUIO_TABLE_STYLE_SCREENSHOT` path saves the first test surface; it is
an artifact destination, never a required input. All test windows close normally.

Rust workspace tests and strict all-target Clippy, including native-image test
code, pass after the selection-paint fix. The full Dune build/tests/format check
passes for the direct-root and key semantics. The Table Lab's light/dark style
update preserves selection and its keyed pixel anchor; the final test additionally
checks absolute cell activation/deactivation counters rather than only the active
cell count. Final public rerun evidence is recorded below.

Table accessibility/physical AppKit input, full-history native cache/traversal,
resize/reorder/sort during paging, public context flows and the remaining OCH-39
acceptance still remain. OCH-46, hosted macOS/Linux gates and merge remain part
of milestone 5. No table capability or hosted acceptance is advertised here.

Final local checks pass after the selection fix and stronger lifecycle assertion:

- `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt`.
- `GPUIO_JOBS=2 python3 scripts/test_table_public.py` — light/dark updates preserve
  the exact activation/deactivation counters, selection and keyed anchor, followed
  by the existing query/stream/paging/close checks.
- `GPUIO_JOBS=2 python3 scripts/test_table_host.py` — GPU style/state/selection
  assertions plus retained host and input/clipboard/command regressions.
- `GPUIO_JOBS=2 python3 scripts/test_table_adapter.py` — column/input/anchor
  regression and verified failing-process cleanup.
- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --workspace --locked -j2`.
- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --workspace --locked -j2 --all-targets --features gpuio-native/native-image-tests,gpuio-table-adapter/native-tests -- -D warnings`.

All owned test processes/windows exited. No hosted run is claimed by these local
results, and the full OCH-39 completion audit is still pending.

## Native AppKit table semantics (2026-09-26)

The retained host's actual NSAccessibility objects now pass:

- 100,000 logical data rows and two columns while mounted table/row/cell/header/
  sort-button objects stay below 70; mounted rows stay at most 16. A jump to data
  index 50,000 reports that index, not a viewport-relative position. This is a
  bounded sampled accessibility test, not native full-history cache acceptance.
- Zero-based row/cell/header positions, mounted rows and selected-row enumeration,
  column labels, exact Japanese/joined-emoji/combining-accent/tab/quote/newline
  values, and absent values for unavailable cell data.
- Repeated Press remains cell selection; native focus targets the addressed cell
  through the table's active descendant. Row and column setters select their
  keyed targets. Ordered deselect/select/deselect before repaint preserves all
  requests; row selection also queues its documented empty-context clearing event.
- A separately labelled sort button emits a keyed descending-sort request and
  publishes the native optimistic sort indicator. Application reset restores the
  retained column descriptions. Row-only/cell-only modes and disabled column
  selection expose only their permitted selection setters; sorting stays separate.
- PointerEvents false still permits accessibility. Retained Cocoa objects from
  hidden/disabled tables cannot deliver application input, and the inert table
  leaves the current accessibility tree.

These tests briefly activate the window for AppKit's real focused-state getter;
background windows correctly report unfocused. The harness retains inspected
Cocoa objects across retirement and closes/reaps its window on success/failure.
They are AppKit object/action evidence, not VoiceOver speech, physical keyboard/
IME or Linux GUI acceptance. The existing table input/GPU/host cases continue to
pass in the same test executable.

The new `table-state.patch` is scoped to table metadata and explicitly opted-in
selection actions; the existing tree custom-action IDs and behavior are preserved.
All four patches reproduce every vendored Rust source exactly from the archive
whose SHA256 and original source hashes match `UPSTREAM.json`. No dependency or
wire-version change is involved. Header/cell native element IDs now use column
keys; adapter geometry probes resolve the current displayed key.

Final local validation for this change passes on macOS:

- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native --features native-image-tests --test native_controls --test native_tree --test native_table_host` — table semantics plus shared controls/menus/palette/tree AX regressions, including the existing tree's two complete 100,000-row traversals.
- `GPUIO_JOBS=2 python3 scripts/test_table_adapter.py` — column-key probe updates,
  selection/input/anchor cases and intentional-failure cleanup.
- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --workspace --locked -j2`.
- `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt` — both
  native backend build paths and the full OCaml build/expect/format checks.
- `GPUIO_JOBS=2 python3 scripts/test_table_public.py` — the public Bonsai/Eio
  Table Lab, including streaming/query/paging/style/lifecycle scenarios.

Physical table keyboard/IME, native table full-history cache/traversal and paging
interaction coverage remain in OCH-39. Shared tree history checks do not satisfy
the table's workload. The polished OCH-46 showcase, hosted gates and merge remain;
no table capability or completed milestone is advertised.

The final strict all-target check also passes:
`GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --workspace --locked -j2 --all-targets --features gpuio-native/native-image-tests,gpuio-table-adapter/native-tests -- -D warnings`.
All owned validation processes and windows have exited.

## Full native table history workload

`GPUIO_JOBS=2 python3 scripts/test_table_history.py` runs the production table
host in a hidden 540 × 2700 logical-pixel macOS window. It verifies the platform
actually supplies the tall viewport, with the production minimum 20-pixel row
height. The workload visits every one of 100,000 logical rows twice, replacing
each batch's payloads and node generations. Each batch contains at most 128 rows
and 512 cells. It scrolls horizontally between the first and last unpinned
columns and checks that the pinned column stays fixed and every cell renders.

Transactions pass through the real BinProt encoder/decoder and unchanged limits
of 4,096 operations and 1 MiB. The test checks current rows, focus handles,
selection states and tree nodes; weak references track retired text payloads,
cell metadata and selections. It checks release after unmount and after window
close, then deliberately fails a separate run to verify window/process cleanup.
The Python runner bounds execution and terminates/reaps its process group on
timeout or cancellation. It does not read or overwrite the user's clipboard.

Logical-source admission accounting reserves 192 bytes per row, independently
of mounted cell payloads. The test bounds the mounted accounting delta to less
than 1 MiB and records both values separately from process peak RSS; admission
accounting is not an allocator measurement. Progress reports split update,
paint and checking/scheduler time. A scoped macOS activity keeps this explicit
hidden workload runnable without activating its window or changing global
App Nap or sleep settings.

The full workload now passes locally on macOS. An initial diagnostic run was
deliberately stopped after its 12,800-row segment time increased from 53 to
338 seconds; it did not establish full-history acceptance. A one-batch viewport
probe uses a distinct marker and is also not full-history acceptance. This
workload does not replace physical input, active Eio paging/column interaction,
the polished chat showcase or required hosted platform validation.

The subsequent full run reached the second traversal's final short batch but
failed its unchanged 1,024-retired-payload limit with 1,295 retained payloads.
Failure cleanup closed the native window. An independent probe against the
original Taffy 0.13.0 showed that both `remove` and `clear` left measurement
contexts alive after retiring all nodes. GPUI stores text measurement closures
in those contexts. The scoped [dependency patch](../../vendor/taffy/GPUIO.md)
retires the contexts alongside their nodes, without changing the version or
layout calculations; both native backend build paths apply it.

With that patch, the two ordinary Rust ownership regressions pass. A short native
`--probe-cache` alternates 128- and 32-row batches eight times (1,280 visits): it
now records zero retired cached payloads and passes unmount/window release. The
same probe failed before the patch. Its distinct marker is not full traversal
acceptance; the complete two-pass rerun is recorded below. Broader regressions
are being validated separately.
Original archive hashes, the two-line patch reconstruction and license hash all
verify against `vendor/taffy/UPSTREAM.json`.

The patched full runner exits successfully with all three completion markers:

| Measurement | Result |
| --- | ---: |
| Logical rows / complete traversals | 100,000 / 2 |
| Total row visits | 200,000 |
| Active rows / cells | at most 128 / 512 |
| Peak retired text payloads still cached at batch checks | 0 |
| Retired text after unmount / window close | 0 / 0 |
| Logical-source baseline admission accounting | 19,200,689 bytes |
| Peak total admission accounting | 19,643,057 bytes |
| Peak mounted-payload accounting above baseline | 442,368 bytes |
| Initial / final process peak RSS | 77,168,640 / 183,451,648 bytes |
| Full traversal duration | 517.0 seconds |

Every horizontal sweep checks that all four columns render and the first pinned
column stays fixed. Every batch checks current resource bounds, old cell-metadata
and selection release, and actual viewport positions. The separate intentional
assertion exits nonzero, closes its window and is recognized by the runner as
verified failure cleanup; the runner itself exits zero. Both owned processes are
reaped. These are hidden-window native layout/paint and ownership results, not a
release-mode latency benchmark or physical AppKit keyboard/IME evidence.

## Public paging with column and sort updates

The extended Table Lab passes `GPUIO_JOBS=2 python3 scripts/test_table_public.py`
with the patched default native backend. Its controlled Eio producers now exercise:

- A pending page while the application accepts a column resize to 240 pixels
  and moves the message column ahead of the tool column. Query generation,
  cell selection and row 4's seven-pixel anchor survive both updates and the
  subsequent page delivery from 16 to 32 rows. Active cells remain at most 96.
- A second producer deliberately held in cancellation cleanup while descending
  sort reverses the data. Row 16 follows its key to index 15 with its nine-pixel
  anchor and selection intact. Finishing the obsolete producer cannot append
  its extra row or change that anchor. The scripted cancellation delay has a
  timeout so a failing check can still close its window and workers.
- Final window closure releases every activated cell and finishes all five
  producers, including cancellation of an ordinary still-running load.

The first development run checked the anchor before a refreshed observation
arrived. Waiting for the current observation passes with the exact original row
and offset requirements; no native anchor implementation or tolerance changed.
Rendered-frame acknowledgements and viewport observations are separate events.
This is public Bonsai/Eio/native layout integration through the application's
request callback, not physical pointer/keyboard/IME validation. The existing
adapter's real native gesture cases provide their own separate evidence.

The full Dune `@all @runtest @fmt` checks pass with the two-line Taffy patch and
both backend paths. The final example-only rebuild and formatting check also
pass after correcting the asynchronous test observation.

The shared layout regression checks also pass locally:

- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --workspace --locked -j2`.
- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --workspace --locked -j2 --all-targets --features gpuio-native/native-image-tests,gpuio-table-adapter/native-tests -- -D warnings`.
- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native --features native-image-tests --test native_editor --test native_controls --test native_list --test native_tree --test native_table_host` under a 600-second process-group guard.

The native checks cover controls/menus/pointer behavior, the editor's actual
macOS text-client composition path, table input/GPU/AX behavior and list/tree
workloads. Both managed lists and trees visit/revisit all 100,000 rows and now
report zero retired cached text at their checks and after unmount. The editor
result is shared-editor regression evidence; table-specific AppKit input/context
acceptance remains separate. All owned windows/processes from these suites exit.
Required hosted validation and milestone completion remain pending.

The independent extension consumer also passes
`GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --run` using a fresh
`--workspace` directory. This stages installed public libraries locally, builds
a separate consumer with the generated backend and observes its native command
and correlated paint before closing. No opam switch is modified. Fresh generated
and checked-in backend manifests, registration and Dune source dependencies
agree, including the Taffy patch path.

An initial background smoke run timed out waiting for paint. Activating that
same independently built executable made it pass immediately. The smoke example
now activates by default, with an explicit `--background` option for environments
that deliver background frames. The paint requirement and runner timeout are
unchanged. A fresh full independent build/run passes with that default. This is
macOS consumer evidence; it does not establish Linux GUI acceptance.

The final `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt`
passes after the viewport documentation and smoke activation change. Staged and
working-tree whitespace checks pass. No original vendored source differs from
its recorded hash after reversing exactly the two patched context-retirement
lines; the separate patch-reconstruction check also passes. OCH-39 remains open
for table-specific AppKit input/context work and final acceptance, followed by
OCH-46, hosted gates and merge.

## AppKit input and public context actions

The retained-host test now posts CoreGraphics keyboard events only to its own PID,
yielding to the actual AppKit event queue outside `Window::update`. Local macOS
checks pass Down/Right selection, Return activation, Shift-F10 context, Command-C
and exact Unicode clipboard data in request order. An embedded editor then receives
real `NSTextInputClient` marked-text replacement/commit, followed by OS Backspace
(deleting one joined-family grapheme), Select All/Copy, Return and Shift-F10 without
emitting table actions. The host restores the original clipboard and closes its
window. This is native input-client integration, not evidence for every installed
IME or VoiceOver speech. The runner requires both new AppKit completion markers.

`examples/table/event_actions.{ml,mli}` connects public requests to an ordinary
accessible inspection dialog and a keyed **Reveal result** action. It keeps a row
membership/query generation/dialog identity and resolves the current payload; delayed invocation
validates again and retains only the controller, not an output/source snapshot.
Right-click targeting stays independent of selected-row state. Invalidated dialogs
unmount; guarded dismissal cannot close a newer target. The public lifecycle test
captures old callbacks, changes query/removes the selected row, and closes/reopens
the same row in one query; obsolete callbacks cannot move selection or dismiss
the new inspection dialog.

The external `scripts/test_table_appkit.py` check passes actual child-PID keyboard,
owner-checked pointer input, Return/Shift-F10, Escape/focus restoration, reveal,
right-clicking a different row, AX button activation and native window closure.
It exposed missing content labels on the shared selectable-text renderer; the
renderer now gives its existing Label the same shared text as its accessible label.
The public test reads the exact Unicode event description through macOS AX, so
this regression is checked through the platform adapter. No broad accessibility
or Linux GUI claim follows from this test.

Local macOS commands at this checkpoint all pass:

- `GPUIO_JOBS=2 python3 scripts/test_table_host.py`: retained host, AppKit keys and
  text-input client, clipboard, child-editor priority, GPU styling and table AX.
- `GPUIO_JOBS=2 python3 scripts/test_table_public.py`: existing full public paging
  suite plus query/row/dialog-session retirement and stale callback checks.
- `GPUIO_JOBS=2 python3 scripts/test_table_appkit.py`: external public input and
  exact selectable Unicode text exposed through native AX.
- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native
  --features native-image-tests --test native_ui`: shared native selection/copy,
  input, focus, style and replacement/reset regression.
- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native --lib`:
  **225 unit tests pass**.
- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j2 -p gpuio-native
  --all-targets --features native-image-tests -- -D warnings`.
- `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/table/main.exe @fmt`
  and `... cargo fmt --all --check`.

The public AppKit runner has a 120-second total deadline and always reaps its
child. Native/public table runners also kill/reap their process groups on
interruption. Hosted macOS/Linux validation, final capability audit and milestone
completion are not claimed by these local results.

## Local audit and capability

The [requirement-by-requirement audit](data-tables-och39-audit.md) now accounts for
all OCH-39 local functionality. Managed tables advertise `CAP_MANAGED_TABLES =
1 << 40` (`1099511627776`), independently of trees and ordinary virtual lists.
Both bridge halves require mask `2199023255551`. Paired Rust/OCaml checks encode
this bit as `0001fc0000000000010000`; existing full-mask fixtures deliberately
change to `0001fcffffffffff010000`. This is an explicit compatibility gate, not
a claim of Linux GUI or hosted milestone acceptance. OCH-39 remains In Progress
until the required hosted checks/delivery gates complete.
