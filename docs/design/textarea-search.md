# Ordinary text-area search and replacement

OCH-41 implementation design, 2026-10-01. `Editor_search` (also available as
`Text_input.Search`) supplies validated value contracts. Multiline
`Text_input.Config ?searchable:true` and Eio `Text_input.search_command` provide
explicit programmatic search. Native changes also arrive through
`Text_input.Event.Search_changed` and Eio `Text_input.search_snapshot`. Reusable
presentation is available as `Gpuio_eio.Search_bar`. This is not desktop acceptance;
code-editor/LSP functionality remains outside this scope.

The retained Rust editor owns the search session and match ranges. OCaml owns
the search bar presentation and receives bounded observations. Searching is
literal, with explicit `Sensitive` or `Ascii_insensitive` matching; the latter
must not be described as Unicode case folding. Empty queries have no matches.
Queries are bounded to 2,048 UTF-8 bytes, without NUL; replacements use the
existing 262,144-byte text bound. Match ranges count UTF-8 bytes and must remain
within the observed editor text at character boundaries.

The public value vocabulary is `Text_input.Search.Query`, `Case`, `Mode`,
`Occurrence`, `Snapshot` and `Stamp`. A snapshot carries query, case, open/replace mode, match count,
optional active occurrence and editor/search revisions. It does not serialize
every matching range. Session revisions advance on query, open/close and
navigation changes. Replacement commands require the exact observed editor and
search revisions and the existing generation-checked window/node lease.
No asynchronous replacement may silently target a newly selected occurrence.

The native session counter is independent of editor text revision. It detects
query and navigation round trips even when final visible state is identical.
Repeated open requests advance activation and session revisions; identical query
echoes and rejected replacements preserve session revision. Entering read-only
mode advances it when that removes replace mode; disabling search closes its
session. Revision exhaustion refuses further session mutations instead of
wrapping. Oversized native selection seeds retain the previous query. The
command adapter rejects stale editor and search revisions before calling the
engine. Eio rejects stamps from another window/node generation before queuing a
request. Replacement snapshots pass through normal controller revision handling;
search-only replies never update the draft.

The paired observation schema carries only bounded metadata, including text byte
length and at most one occurrence. Validators check range/count/query-length
consistency, nonnegative revisions and activation ordering. They cannot prove
character boundaries without the editor text; that remains a native matcher
invariant. Core stamps additionally retain exact window and node generations.

Commands cover open, close, query, next/previous, inspect, replace-current and
replace-all. Opening is a user action, never an operation repeated by rendering.
Initial query matching chooses the first occurrence; next/previous wrap. Closing
retains the query for reopening. Search metadata replies must not replace a
newer OCaml text observation. The reusable Bonsai search bar supplies Find/next/previous/Escape actions,
query focus/selection, guarded close/focus, accessible names and result counts.
Physical keyboard, composition and accessibility acceptance remain required.

Replacement is atomic and undoable. Before any expanded output is allocated,
compute the resulting UTF-8 size with checked arithmetic. Reject over-limit
results, active composition and noneditable state without moving selection,
scrolling, changing match position or adding history. Apply the current exact
edit policy, and report success only after the editor accepts the edit. Equal
replacement text is still an accepted replacement, not a rejection. The bridge
translates failures into typed errors; the Base engine's count/bool methods
alone cannot express that contract.

Highlight layout must allocate paths for the laid-out match range rather than
reserve capacity for every document match. The retained match vector is bounded
by text size; its actual memory and search latency still need OCH-17 measurement.

Remaining acceptance: physical keyboard/focus/IME/AX, visual layout and
measured presentation teardown/resource behavior. Local tests cover checked commands, metadata, stale replies and Unicode
undo; they do not prove physical desktop behavior. Do not mark the catalog row complete before these pass.


## Programmatic command interface

Use `Gpuio_eio.Text_input.search_command editor command`. Commands are
`Read`, `Open { replace }`, `Close`, `Set_query { query; case }`, `Next`,
`Previous`, `Replace_current { if_stamp; replacement }` and
`Replace_all { if_stamp; replacement }`. Obtain `if_stamp` from a returned
`Search.Snapshot.stamp`. The native editor still owns text and all match ranges.

`Response.Observed snapshot` acknowledges metadata operations.
`Response.Replaced { snapshot; count }` reports accepted replacements, including
zero when there are no matches. Replacement also updates the editor controller's
text observation; a late reply cannot overwrite a newer native text revision.
Neither opening nor closing these programmatic commands changes keyboard focus;
the search-bar presentation owns query focus and uses the atomic close/focus
operation described below.

All commands require multiline opt-in. `Read` and `Close` work while disabled;
other mutating commands reject disabled state, except guarded close/focus,
which closes and skips focus when disabled. Query/navigation/replacement require
an open search session. Opening rejects active document composition. Read-only editors support finding; replacement rejects
read-only state and active composition. Opening in replace mode on a read-only
editor opens find mode instead. New errors are `Search_unavailable`,
`Stale_search` and `Not_editable`, alongside existing editor errors. Text revision
is checked before search revision for replacement. Normal editor request capacity,
exact correlation, window-close and stale-node behavior apply.

Disabling opt-in closes native search and retains its query, draft, selection and
history. Re-enabling leaves it closed until an explicit open action. Configuration
uses Op80; command tag10 wraps nested search command tags0..7. Result tags5/6 carry
metadata/replacement replies; error tags11..13 are appended. Legacy editor config
and text snapshot byte layouts are unchanged in the unpublished protocol epoch.


## Automatic observations

Appended event tag70 `Editor_search_observed` carries window, node and handler
leases, accepted view revision and the same bounded search metadata as commands.
Rust publishes from retained editor subscriptions, after initial text observation,
and directly after programmatic search commands. The observer deduplicates the
same metadata; blink/layout notifications do not clone a query or enqueue another
search event. The payload carries at most one match, never the full match vector.

Only opted-in text areas start observing. Disabling search publishes closed state;
removed nodes and windows cannot publish through a retained route. Subscriptions
drop with their native editor. Core dispatch checks the current multiline owner,
handler and accepted view revision. A currently disabled search configuration
accepts closed metadata but drops queued open observations. Picker queries and
single-line fields do not receive these events.

The mailbox coalesces adjacent observations only for the same window/node/handler
and view revision, with nonregressing editor and search revisions. Other events
remain ordering barriers. Query bytes count against existing input and frame
budgets; response text and query bytes also count for search-command replies.
Typing can interleave text and search events, so this is not a promise that every
burst collapses into one event. Existing bounded-overload behavior applies.

Eio preserves the most recent search observation across ordinary typing until
fresh metadata arrives; applications must treat it as an observation, not current
editing authority. A new native editor lease clears it. Metadata older than the
known text revision or either previous search stamp component is rejected, as are
replies for a retired lease. Existing metadata wins equal-stamp reply ties:
read-only/composition capability observations need not change the search stamp,
and a delayed reply must not undo them. Native events can update equal-stamp
capabilities in delivery order. Metadata never changes the text draft. Accepted search
replacement replies use the same lease/revision protection as other editor replies.

These changes have local protocol, controller and TestPlatform coverage. Physical
Find/F3/Escape, query focus, IME and accessibility acceptance remain open for
the composed search bar.


## Reusable search bar

Create `Gpuio_eio.Search_bar` with the same window as an opted-in editor. Keep
the document controller outside the bar and wrap its view exactly once:

```ocaml
let search = Gpuio_eio.Search_bar.create window ~editor graph in
let open Bonsai.Cont.Let_syntax in
let%arr editor = editor
and search = search in
Gpuio_eio.Search_bar.wrap search (Gpuio_eio.Text_input.view editor)
```

An explicit `Search_bar.open_ search ()` effect opens Find;
`Search_bar.open_ search ~replace:true ()` opens replacement. Do not execute
opening effects during rendering. The gallery's **Room to write** example uses
this public interface and customizes the bar with `~bar_style`.

The document stays under a stable keyed wrapper. Each exact document lease and
search activation owns fresh query/replacement controllers through a managed
Bonsai lifetime. Closing or reopening retires those controls and their delayed
continuations. The replacement draft belongs to the bar component and retains
committed text across openings. Provisional composition is not persisted.

The query has native autofocus on mount. Initial select-all is a same-text,
revision-checked command only for an unchanged revision-zero seed outside
composition; first observations already containing typing are not selected.
Later typing does not repeat selection. Query and replacement fields allow
literal newlines, with one to three visible rows. They own native drafts; source
search metadata is an asynchronous observation. While a bar is open, use its
controls for query/case edits. Engine-level `Text_input.search_command` calls do
not replace the bar's draft; reopen to seed it from native search state, or build
a custom presentation when controlling query drafts elsewhere.

Commands are scoped to the wrapped editor and controls. Primary-F opens/reopens;
Primary-Shift-F opens replacement; F3/Shift-F3 navigate while open; Escape closes.
Enter/Shift-Enter navigate only in the query field, leaving document and
replacement newlines to their native editors. Shortcut policies defer during
composition. Button-based close/reopen also reads both query fields and rejects
active composition before retiring controls. Open rejects active document
composition at the native command boundary.

Every committed query change is sent, even when it returns to the last observed
query while a different update is in flight. Initial identical echoes preserve
native match position and session revision. `Set_query_text` changes only the
native query; `Set_case` changes only case policy; `Toggle_case` toggles current
native state. This prevents stale observations from undoing concurrent changes
to another field or losing rapid checkbox activations. Matching remains literal
and case-insensitive matching remains ASCII-only.

Navigation synchronizes the current native query draft first. Replacement reads
both fields, rejects composition and a query differing from the displayed search
metadata, then sends the **displayed** search stamp with exact replacement text.
It does not reinterpret a stale replacement intent against a newly fetched match.
Error/count labels are ordinary accessible English status/alert views. Styles
inherit from the application. Feedback from older search-command workflows cannot
overwrite newer feedback; an unrelated query echo does not cancel an already
requested navigation continuation.

### Atomic close and focus

`Search.Command.Close_and_focus snapshot` checks the snapshot's exact window/node
owner and activation, rather than its text/query/navigation revisions. It cannot
close a later reopening. Rust closes and restores document focus in one command,
preserving text, selection and history. A hidden, disabled or focus-blocked
document closes without stealing focus. Active document composition and an
already closed session fail without focus changes. A reply confirms the matching
activation is closed; it is not a promise that focus was eligible or painted.
Normal `Close` remains metadata-only.

The unpublished protocol appends search command tags8..11 for `Close_and_focus`,
`Set_query_text`, `Set_case`, and `Toggle_case`; existing tags are unchanged. The
activation is nonnegative, queries are byte-bounded before Rust allocation, and
the close reply must carry closed mode and the requested activation. No vendor
patch was needed for these operations or the search-bar composition.
