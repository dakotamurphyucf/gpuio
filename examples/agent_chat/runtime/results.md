# A window-owned paged and complete-query results table

[results.ml](results.ml) and [results.mli](results.mli) combine synthetic findings,
scoped paging/CPU jobs, Bonsai observations and a bounded native read-only table.
The table proposes column/query actions; this controller accepts changes into
application state. Findings are [pure fixtures](result_data.md), never provider
or tool output. `create` runs in the window factory, before Bonsai evaluation.

From the repository root with [isolated setup](../../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/agent_chat/main.exe -j 2
_build/default/examples/agent_chat/main.exe
```

Open Results; load its next page, sort/resize/reorder headers, pin IDs, filter
scores, activate a finding and Reveal it. Slow query and Fail next query expose
cancellation/retry; 100,000 results opts into full local data construction.
macOS is the v1 target; Linux GUI qualification remains
[informational](../../../docs/platform-release-policy.md).

## Initial state, query modes and acceptance

Read `create`, `accept`, `query`, `filter_scores`, `paged_sample`, `columns`,
`toggle_pin`, `request`, then `component`. The abstract `t` stores the window
pager, [Fixture_job](fixture_job.md), supplied complete-build capability,
observable requested query/column preferences/busy/notice and
[Result_actions](result_actions.md) dialog controller.

`create` starts an empty `Table_data` with default Sample/All/unsorted/Normal
query, Before End and After More None. Default columns come from Result_data;
busy is false. When the active native table demands the first page, the loader
waits 0.3 seconds and returns at most 24 of the 48 globally ordered sample rows.
`previous` tracks attempt count per request generation. Fail_once fails the first
attempt in that generation, then Retry proceeds; Slow waits five seconds.
A reset creates a new generation, so its failure demonstration starts anew.
If Fixture_job construction fails, the partially created pager closes.

The [Table_paging contract](../../../lib/eio/table_paging.mli) bounds reusable
workers and newest pending boundary requests, captures immutable query settings
at admission and rejects canceled/obsolete UI delivery. Pager data is
window-owned, independent of native cells or whether the inspector currently
shows Results. For example, starting Slow query and then selecting a different
query fences the old response: returning the old worker slot does not republish
`Pager.value` or call its optional `on_change` handler. Current work can still use
the returned slot and publish its own result. This prevents old worker cleanup
from looking like a new result to Bonsai or application callbacks.

`paged_sample` deliberately resets to an empty fresh source with a forward More
boundary. Paged sample, Slow query and Fail next query use this path; they reset
size/filter/sort to constructor defaults while selecting loading policy.
`query` is different: it cancels current boundary work/fixture delivery, records
the requested query and captures current source data. For Sample it computes a
**complete** replacement synchronously; for Large it runs `t.build source query`
through Fixture_job. On acceptance, `accept` calls `Pager.reset ... ~before:End
~after:End`, adopting the complete query/source with no remaining page.
Thus sorting or filtering a paged sample can load its complete matching set,
rather than retaining just the previously loaded first page.

During a large transform, requested settings and busy may lead the accepted
snapshot: labels/sort config still describe `snapshot.query` until adoption.
The existing table is disabled while busy, and a fixture spinner reports work.
Failure clears busy and reports a notice. Newest-wins fixture serials suppress
obsolete large results; reset retires old pager delivery. A pure CPU calculation
may drain after its delivery is canceled, as explained in the job guide.

`filter_scores t range` reads the current requested query at effect execution
and replaces its filter with Between while preserving size/sort/loading.
Settings uses this public operation after a score interval is accepted.
All scores, Score ≥ 80, Empty results and removable-filter controls use related
complete-query updates. Empty is a deliberately reproducible sample filter, not
evidence of a failed external search.

## Reactive table and native proposals

`component` reads `Pager.value` and builds derived `config`/`table_style`.
`let%arr` combines current reactive inputs into a value; changing a source updates
its dependents. `B.map2` is the two-input form used for active/busy and cells.
`Effect.t` describes deferred actions, and `E.bind` sequences effect results;
none of these expressions starts another query merely because a view renders.

`Table.Config.create` supplies accepted columns/sort, label Run results,
34-pixel row height, at most 24 active rows/96 active cells, `disabled:busy` and
column selection. The stable key run-results and 300-pixel-high full-width style
bound the native table; selected styling uses the theme's accent surface.
`auto_load = active && not busy` gates automatic requests, without canceling the
window-owned pager when the page is hidden.

`Gpuio_bonsai.Table.paged` receives pager controls and `request t`. Its cell
renderer derives text from reactive row/column values, calls `Result_data.cell`,
and returns `Table.Cell.text`. The compact cell uses the same complete text for
display, copy and accessibility, including the fixture's Japanese/emoji content.
The `_lifetime` parameter is deliberately unused because cells introduce no
row-local deferred commands; controller/document actions use separate guards.
See the [table adapter contract](../../../lib/bonsai/table.mli).

`request` combines two effects: details-dialog handling and the application
operation. Resize folds all proposed widths through validated column collection
updates; Move accepts column ordering via `Collection.move`; Sort validates the
column/direction in `Query.with_sort`, then calls `query`. Select/Activate/
Context/Copy need no additional application mutation here: native selection/copy
and Result_actions handle their respective responsibilities. Column preferences
are observable window state, not a hard-coded native table snapshot.

`toggle_pin` rebuilds number with its current label/width/bounds and toggles
Left/Unpinned, removes its old collection occurrence, then places it first in a
new `Result_data.schema`. This makes group labels/pin boundaries coherent, and
may move an already-reordered number column back to first. Reset columns returns
the complete default schema; it does not change the query. These operations
validate before setting the observable collection or report a notice.

## Trace paging, cancellation and inspection

1. Opening the page makes active auto-load true. The native table requests its
   first ready boundary; pager state becomes Loading. An empty/loading source
   shows [Query_loading.results](query_loading.md), and the footer shows a spinner.
2. The scoped producer returns `Result_data.page`. Current generation delivery
   merges rows and publishes Ready/End. Bonsai derives the table/new footer;
   Load more sends a generation-checked After request for the next 24 records.
3. Choose Slow query, then Paged sample before the delay finishes. Reset cancels
   the old request and retires its generation; a late slow result cannot replace
   the new sample. Fail next query instead publishes Failed and requires explicit
   Retry results rather than retrying automatically on view updates.
4. Activate a finding: Result_actions captures row membership/generation/session,
   displays current details and revalidates at Reveal invocation. Its batch
   selects/reveals the summary cell, then closes; Copy remains native.

The footer distinguishes Failed, Loading, Ready and End. A terminal empty source
shows No findings match with Restore sample results. The local simulation banner
has `live:Off`; notices/status are application presentation, not a claim that
external work happened. `Output.selection` drives row/cell/column status labels.
Reveal last result takes the last **currently loaded** source row; it is the
100,000th record only after the complete large source is adopted. Its atomic
batch selects/reveals the number cell using a current row target.

## Identity, retention and adaptation

Complete `Table_data.replace` preserves membership for surviving keys, allowing
selection to follow surviving records through reordering. Query generation still
retires old commands, and removal/reinsertion starts a new membership lifetime.
Fresh paged_sample creates a new lineage. Inspector keeps this graph/controller
per window across route changes; another window owns separate data/preferences.
The native mounting budget does not bound the complete source's 100,000 payloads
or transformation cost. Application startup supplies the Eio worker capability;
window scope closure cancels loaders/fixture delivery. Idle native cells/loading
need no OCaml polling task.

A small adaptation is another score preset: construct a validated Score_range
and call `filter_scores`, preserving complete-query ordering and surviving IDs.
For a new column, update Result_data schema/cell formatting/sort validation
coherently. Real remote paging must apply query sort/filter at the service before
slicing pages; do not sort only already-loaded rows. Keep controller operations
batched when selection and reveal must both execute, and do not treat batch
submission as native acknowledgement or physical painting.

`python3 scripts/test_agent_chat_results.py` is the optional macOS external test;
[README prerequisites](../README.md) and
[existing evidence](../../../docs/evidence/agent-chat-m5.md) describe its scope
and owned-process cleanup. This source/link review performs no new native,
clipboard, resource or 100,000-row acceptance run.
