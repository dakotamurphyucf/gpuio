# Native palette snapshots

`View.command_palette ~on_change` optionally observes native interaction through
`Command_palette.Snapshot`. The same callback is available in Bonsai View and
returns an effect. Query editing, composition, matching, keyboard selection and
command activation remain native; none waits for an OCaml callback.

The immutable snapshot exposes exact query text, composition status, highlighted
stable command ID (or none) and matched command count. The count includes disabled
commands and excludes headings/separators. An initial snapshot is followed by
changes to these values or the accepted query-edit revision. Caret movement,
focus changes, paints and ordinary view reconciliation do not generate redundant events. Registry changes can alter
highlight/count without changing the query.

`Snapshot.same_query` compares query revision and window/node/subscription
identity. Selection-only changes preserve it. Accepted edits and composition
transitions advance it, including replacements restoring the same text. The
editor revision detects edits whose notifications were coalesced; observing only
the final text cannot accidentally reuse an earlier query identity. A remount or
observer detach/reattach is a different lifetime. Applications should avoid starting searches while `Snapshot.composing` is true.

This comparison supports application-side cancellation and stale-result checks
against the newest **received** snapshot. It does not atomically admit results
against current native text: a newer native input may still be queued. [Native query/highlight commands](palette-commands.md) now use their own correlated
request/response contract and optional native query check. Atomic external-result
publication, loading and persistent embedding remain required follow-up work;
the observation API alone does not close those catalog rows.

## Delivery and limits

Observers receive bounded, ordered asynchronous events. The payload has a positive
monotonic observation sequence and query revision, at most 4,096 query bytes,
an optional 256-byte command ID and a count from 0 to 1,024. Query text is UTF-8,
without NUL or line breaks. The native state retains only its latest snapshot.
The existing mailbox accounts for query/ID bytes and faults an overloaded window
rather than silently dropping lifecycle transitions. Counter exhaustion also
faults the window.

The unpublished exact protocol epoch 3 appends operation 127
`Set_palette_observed` and event 79 `Palette_observed`; existing operation/event
tags retain their meaning. Both sides ship together and require the repository's
matching OCaml/native package revision. Independent fixtures protect the paired encoding.

Core dispatch checks the accepted window, generation-qualified node and handler,
tree revision, payload validity and increasing observation sequence. Preparing a
transaction does not change live dispatch. Changing observer presence rotates
the palette handler; accepting that change fences old observation and dismissal
events. Replacing the callback while keeping a subscription uses the latest
callback without resetting the native query or publishing a redundant snapshot.
Removal/closed-window generations fence retired observations.

Snapshots report native history, so a selected command may no longer appear in
newer OCaml configuration when an older event arrives. It is an observation, not
authority to execute a command. Activation still revalidates the current registry
route and native eligibility. A closed palette emits no further snapshots and
retains its existing one-shot dismissal behavior.
