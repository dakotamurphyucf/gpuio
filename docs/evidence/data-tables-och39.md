# OCH-39 data-table evidence

Status (2026-09-26): **column/data/paging foundations and initial native extraction**. OCH-39 remains
In Progress. No table capability is advertised. The
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
