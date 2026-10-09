# Define a complete result query before paging

[result_data.ml](result_data.ml) and [result_data.mli](result_data.mli) hold the
pure findings fixture: row payloads, validated query settings, column schema,
complete-query filtering/sorting, paging and cell formatting. No provider or
filesystem operation produces these findings. The [Results controller](results.md)
adds Eio delays, reactive state and the native table.

From the repository root with [isolated setup](../../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/agent_chat/main.exe -j 2
_build/default/examples/agent_chat/main.exe
```

Open Results; inspect the first page, load more, sort score/tool/result number,
try score filters and opt into 100,000 results. macOS is the v1 desktop target;
Linux GUI checks are [informational](../../../docs/platform-release-policy.md).
No external dataset is needed.

## Rows, query settings and columns

Read `Row`, `Query`, `schema`, `columns`, `row`, `rows`, `replace`, `page`, `cell`
and `describe` in order. `Row.t` is abstract outside the module and exposes
`number`, `tool`, `score` and `summary` accessors. `row number` alternates Search/
Read source by parity, calculates `number * 37 mod 101` for an integer percentage,
and formats a summary with Japanese text and a family emoji. This Unicode content
exercises exact cell display/copy/accessibility text.

Row IDs are `result-%06d`; column IDs are separately typed
`Table_column.Id.t` values. `Query.t` contains:

| Setting | Cases/default |
| --- | --- |
| Size | Sample (48 rows) or Large (100,000); default Sample |
| Filter | All, High_score (≥80), Between Score_range, Empty; default All |
| Sort | Optional Table.Sort; default run-number order |
| Loading | Normal, Slow, Fail_once; default Normal |

The final `()` applies optional constructor defaults. `Query.create` and
`with_sort` validate sortable column IDs: number, tool and score only. Summary
sorting is rejected. `with_filter`/`with_size` return immutable updated settings;
[Score_range](score_range.mli) validates inclusive bounds 0–100 with lower ≤ upper.
Loading describes the runtime's demonstration policy; `rows` does not sleep or
fail because it is Slow/Fail_once.

`columns ()` creates number/RESULT, tool/TOOL, score/SCORE and summary/FINDING.
Number starts left-pinned and sortable at 92 pixels; tool is sortable at 126,
score sortable/right-aligned at 90, summary unsortable at 420. Min/max widths are
validated through the [column contract](../../../lib/core/table_column.mli).
`schema` splits an initial left-pinned prefix into RUN and the remaining columns
into FINDINGS header groups, omitting an empty group. It assumes the accepted
pinned columns form that prefix; column collection validation remains authoritative.

## Filter and sort the whole query

`rows query` generates the selected fixture size, applies the filter, sorts the
entire remaining list and returns `(Id.t * Row.t)` pairs. Typed comparisons use
`Int.compare` or `String.compare`. Descending reverses the primary comparison;
when tool/score tie, result number still sorts ascending as a stable tie-breaker.
Without explicit sort, result number supplies primary order. Thus pagination
slices one coherent global order, instead of sorting only the currently loaded
page and misrepresenting the complete dataset.

`replace source query` calls `Table_data.replace source (rows query)`. The
[Table_data contract](../../../lib/core/table_data.mli) preserves membership
lifetimes of surviving IDs in a complete replacement; removing and later
reinserting an ID retires its earlier membership. Keeping identity through sort
helps preserve surviving native selections, while query-generation guards still
reject effects belonging to a retired query. A fresh `Table_data.create` starts
another source lineage even if it contains the same IDs.

`page request` parses its optional cursor as an integer offset (no cursor means
zero). It returns diagnostic errors for malformed/negative/above-100,000 cursors,
unsupported Before direction, or an offset beyond the filtered result count.
For After, it calculates the complete `rows` list and returns at most 24 rows
starting at the offset. The next cursor is the following integer offset, or End
at the filtered total. Exactly-at-end is a valid empty terminal page.
This simple fixture recalculates the complete list for each page; use a real
indexed service or cached complete ordering for expensive data. The demo's
100,000-result control instead builds/adopts a complete source through a worker,
rather than requiring thousands of these synthetic pages.

`cell row column` maps accepted column IDs to zero-padded number, tool, percentage
and full Unicode summary text, returning an error for unknown columns.
`describe` joins identifying fields and summary for the details dialog. It does
not truncate the stored summary to match a visible cell width.

## Follow a query into presentation

For a Score sort on a paged sample, native headers queue a request to Results.
`Query.with_sort` validates it, then `Results.query` constructs a complete
replacement through this module rather than changing the order of just the
first 24 loaded records. The accepted pager query/source update drives Bonsai
observations. `render_cell` uses `cell` and `Gpuio_bonsai.Table.Cell.text`, whose
compact text is shared by display, copy and accessibility. The details helper
uses `describe` only for a current row reference/query generation.

This module owns no native controller, Bonsai effects, loaders or retained cells.
Its immutable outputs can be built on an Eio worker domain; the runtime must
adopt them on the UI domain. Native table budgets bound mounted rows/cells,
not the memory/CPU of this complete dataset.

A small adaptation is to add an unsortable Path sample column: add its validated
column/schema entry and `cell` case, then store/derive the display string in the
row model. If making it sortable, extend `validate_sort` and the exhaustive
primary comparison together. Keep stable row IDs independent of order, an exact
copy-text equivalent for Unicode, and filtering/sorting before slicing pages.
The [Results guide](results.md) and [finding actions](result_actions.md) explain
effects and lifetime; this pure-model review is not new native table acceptance.
