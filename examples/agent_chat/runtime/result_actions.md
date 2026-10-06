# Inspect and reveal a current table finding

[result_actions.ml](result_actions.ml) and [result_actions.mli](result_actions.mli)
provide the Results page's details dialog. Native activation or row/cell context
requests open it. **Reveal finding** selects and reveals the summary cell, then
closes the dialog so the user can use native Copy. It does not copy directly or
run the simulated finding as a tool.

Run from the repository root using the [isolated setup](../../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/agent_chat/main.exe -j 2
_build/default/examples/agent_chat/main.exe
```

Open Results, activate a finding with Enter or open context with right click/
Shift-F10, inspect its complete description and choose **Reveal finding**.
Use the table's native Copy command afterward. All findings are simulated, never
external tool output. macOS is the v1 target; Linux graphical qualification is
[informational](../../../docs/platform-release-policy.md).

## Capture identity, not a payload snapshot

`Target.t` contains a `Table_data.Row_ref.t`, an `int64` query generation and a
`Type_equal.Id.Uid.t` session identity. `t` is a Bonsai expert variable holding an
optional target; `create ()` starts with `None`, meaning closed. The target
contains no row payload or data source snapshot. This avoids retaining a large
query through the dialog and ensures the displayed description comes from the
current data.

A [Row_ref](../../../lib/core/table_data.mli) identifies one membership lifetime
of a row in a source lineage. Removing a row and later reinserting the same ID
creates a different membership. `valid` requires both matching snapshot generation
and `Table_data.contains_ref snapshot.data target.row`. Checking only row ID
would incorrectly accept a reincarnated row or an obsolete query.

`request t ~generation request` returns a deferred `Effect.of_thunk`:

| Table request | Action |
| --- | --- |
| `Activate (row, _)` | Open that row |
| `Context (Row row)` or `Context (Cell (row, _))` | Open that row, regardless of clicked column |
| `Context Empty` or `Context (Column _)` | Clear the dialog target |
| Selection, resize, move, sort, copy | No target change here |

Every open creates a fresh session UID even when it targets the same row/query.
Typed derived `Target.equal` then lets delayed actions distinguish two separate
visits. Other table operations remain with the caller; this helper does not
resize columns or update sorting.

## Build the reactive dialog and guarded actions

`view ... graph` receives reactive `snapshot`, table `output` and theme values,
plus a `current` snapshot getter, `describe` function and target summary column
ID. In [results.ml](results.ml), these are `Pager.value`, the output of
`Gpuio_bonsai.Table.paged`, `Pager.snapshot`, `Result_data.describe` and column
`summary`. Its `request` function passes the current pager generation into this
helper and combines the effect with its own sort/resize/move handling.

The graph reads `Bonsai.Cont.Expert.Var.value t`. `let%arr target = target and
snapshot = snapshot ... in` derives the current dialog view; `let%arr` combines
reactive values, not asynchronous task results. `Bonsai.Edge.after_display`
schedules a deferred close for an invalid target. This is a Bonsai post-display
lifecycle hook, not proof of a physically painted frame. Content derivation also
checks validity immediately, so an invalid target does not keep showing stale
details while that cleanup waits.

`close_target target` clears the variable only if `is_current target` still
matches all target fields. An old dialog's dismissal/close effect therefore
cannot close a newer session. Valid content finds the row by its current ID and
formats `describe row`; it also requires an `Ok` table output so a controller
exists. If data/output is unavailable, content is absent. `View.dialog ... None`
closes/unmounts modal content by the [view contract](../../../lib/core/view.mli).
No I/O or producer task belongs to this dialog.

The body displays Finding details, selectable description text, instructions
and two native buttons. `Palette.of_dark` styles the dialog and buttons.
`Gpuio.Overlay.Config.create ~label:"Finding actions" ~width:560. ()` supplies
its native accessible name/size. The normal dialog handles focus trapping and
restoration; `on_dismiss` ignores the reason and uses the guarded close effect.
The helper stores no native focus handle.

## Trace Reveal and an obsolete visit

1. The native table queues activation/context through `on_request`; the Results
   effect installs a target with its row membership, query generation and session.
2. Bonsai observes that variable, validates it against the reactive snapshot and
   derives a dialog containing the current row description.
3. **Reveal finding** schedules an effect that checks `is_current` and `valid`
   again against `current ()`, rather than trusting the render-time snapshot.
4. If still live, `Table.Controller.batch` submits both `Set_selection (Cell
   (row, result_column))` and `Reveal (row, Some result_column)` together, and
   another effect closes this dialog session. The user can then Copy the complete
   cell text; the dialog never writes the clipboard itself.

The [table controller contract](../../../lib/bonsai/table.mli) makes batching
significant: newer undisplayed batches/native selections can supersede pending
commands, so selection and reveal should share one batch. It also rejects stale
mounts, query generations and absent/reincarnated targets. The helper's own
invocation check adds application-level protection before that submission.
Closing the dialog accompanies submission, not an acknowledgement that the
native table has physically revealed or painted the row.

If a query changes while details are open, generation validation removes the
content and the lifecycle effect clears that target. If another row opens first,
the old close effect sees its different session and leaves the new dialog alone.
A payload update in the same surviving membership can show its new description;
a removed/reinserted row cannot reuse the old target. Virtual cell eviction is
separate from source membership and does not by itself invalidate the row.
Window ownership comes from `Results.create` making one actions controller per
window; no target survives application restart or gets serialized.

A small adaptation is to reveal a different column: pass another accepted
`Table_column.Id.t` from the caller and preserve the atomic selection/reveal
batch. For a new destructive action, revalidate the captured target at invocation
and add explicit application approval/state handling rather than treating the
sample details dialog as authorization. Keep row references, generation and
session identity; capturing a row record alone would lose the stale-action
protection. Current native checks are listed in the [README](../README.md);
this source/prose review establishes no new context-menu, focus or clipboard
acceptance evidence.
