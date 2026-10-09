# How `main.ml` connects a bounded native table to application data

[README](README.md) · [Source](main.ml) · [Event actions](event_actions.md)

The table starts with 100,000 typed event records. Native scrolling and transient
cell computations are bounded; the full application dataset still lives in
memory. This is a read-only event explorer with local data, not a remote loader.

## Identities, configuration, and runtime ownership

`row` builds `{ number; tool; message }`, including Japanese and emoji text.
`id` validates a `Table_data.Id`; `col` validates a distinct column identity.
`data` creates immutable ordered data with a new source lineage. A row membership
reference identifies one incarnation within that lineage, not a retained row
payload. [Table data](../../lib/core/table_data.mli) defines this distinction.

`initial_config` validates three columns: pinned sortable `number`, `tool`, and
`message`. `Table.Config.create` sets row height 32 and budgets of 32 rows and
96 cells. `with_height` reconstructs config while preserving columns/label/sort.
`table_style` composes bounded fill geometry with a light or dark surface.
Column identity survives changes in width and position.

`App.run` opens an 860 × 520 window. Its factory creates `Pager` before the Bonsai
graph evaluates, using `App.Window.scope window`; closing that window cancels
producers. `Pager.create` initially has both boundaries at `End`, so ordinary
startup loads no page. Its scripted `load_behavior` starts at `Fail`. Later
self-test stages select sample rows, a held promise, cancellation-protected
pending work with a 30-second bound, or cancellation waiting. Counters use
`Exn.protect` to record producer exit even when cancelled. See
[table paging](../../lib/eio/table_paging.mli).

## Reactive syntax and native cells

`B` abbreviates `Bonsai.Cont`, `E` means `Bonsai.Effect`, and `W` means
`Gpuio_bonsai.Table`. External `B.Expert.Var` values own accepted config and style.
`Pager.value` supplies reactive snapshots; `B.return controls` supplies constant
paging effects. `W.paged` binds these and `on_request` to the native table.

`render_cell` receives reactive row data and column within a transient cell graph.
`B.map2` recomputes its text when either changes. `W.Cell.text` creates retained
native text with matching display/copy/accessibility value. The number column
formats six digits, tool shows its name, and result shows the message.
`B.Edge.lifecycle` increments mount/unmount counters through deferred effects;
these are diagnostics, not application persistence. The supplied cell lifetime
is unused because no cell starts asynchronous work.

Opening `B.Let_syntax` enables `let%arr`, which observes output, snapshot, and
actions view to derive the root. The `and` bindings are reactive dependencies.
`B.Edge.after_display` records current outputs for diagnostics. Neither construct
runs blocking I/O. `Or_error.ok_exn` unwraps configuration/output errors in this
controlled fixture; handle them deliberately in a user-configurable application.

## Request → effect → accepted state → view

Native selection is optimistic and observed through `W.Output.selection`; the
footer derives a label from that output. Native resize/reorder/sort gestures
submit proposals to `on_request`. Its `E.Many` delivers event inspection requests
and a thunk that changes accepted application data/config.

For Resize, the handler looks up proposed widths by typed column equality,
validates each with `C.with_width`, rebuilds the collection preserving header
groups, and updates `cfg`. Bonsai produces new config and native layout applies
it. Move uses `C.Collection.move` and the same config update. Sort explicitly
sorts all loaded row records by `number`, uses `D.reorder` to preserve memberships,
updates the sort indicator, and calls `Pager.reset ~query:"sorted"`. That new
query cancels old producers and fences their late results. This fully loaded
fixture can sort locally; a partially loaded remote dataset needs a new server
query instead. Select/Copy have no extra application action here; activation and
context go to [Event_actions](event_actions.md).

A failed boundary produces the Retry footer. Its effect uses the snapshot's
current generation through `W.Paging.retry`; obsolete effects do nothing.
Style updates preserve selection, anchors, and cell lifetimes. Native viewport
observations arrive asynchronously and may be temporarily absent even after a
rendered-frame acknowledgement. See [table adapter](../../lib/bonsai/table.mli).

## Commands and bounded diagnostics

From the repository root in the [repository environment](../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/table/main.exe
_build/default/examples/table/main.exe
GPUIO_JOBS=2 python3 scripts/test_table_public.py
GPUIO_JOBS=2 python3 scripts/test_table_appkit.py
```

The public script runs the already-built executable with `--self-test`, checks
`GPUIO_TABLE_PUBLIC_OK`, and terminates/reaps the process group after 120 seconds.
Optional `--background` requests background opening; frame delivery may fail
when occluded. Direct `main.exe --self-test` runs the same graphical sequence.
`perform` enters effects via an app-scope completion and bridges results with an
Eio promise. Each labeled wait has a 15-second bound and each frame a 5-second bound.

The sequence checks keyed scroll/selection batches, row-height anchors, obsolete
query/controller/dialog actions, style retention, point-update copy text, selected
row removal, failure/retry, column changes during a held load, late sort result
suppression, and cancellation/cell cleanup on window close. It invokes request
handlers and view callbacks directly for several actions; it does not inject
physical column gestures or exercise the clipboard.

The separate AppKit script requires macOS, a real desktop, and Accessibility
permission for its automation process. It targets its child PID for keyboard
input and verifies pointer ownership. It checks arrows/Return/Shift-F10,
right-click target, dialog focus restoration, reveal, and native close; it has a
120-second budget and reaps the child. It does not test marked-text composition
or modify the clipboard. Neither compiling nor these scoped checks establish
[Linux GUI acceptance](../../docs/platform-release-policy.md).

For a production dataset, capture explicit I/O capabilities in the pager loader,
keep durable work outside transient cells, guard delayed cell effects with their
lifetime, and use query generations plus membership references for delayed actions.
