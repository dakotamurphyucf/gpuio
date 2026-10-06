# Native palette commands

`Gpuio_eio.Palette_controller.create window graph` allocates a stable Bonsai key
and observes one palette's native snapshots. Pass its `key` and `observe` to
`View.command_palette ~key ~on_change`; call `reset` when removing that view.
Ordinary `View.with_palette_content` slots remain available. Applications can
use `App.Window.Expert.palette_command` directly when implementing another
controller; both routes use the same bounded asynchronous request transport.

Core `Command_palette.Command` contains `Read_snapshot`, `Focus`, `Set_query`,
`Highlight` and `Set_loading`. A command captures the snapshot's exact window, node and
observer generation. Closing/unmounting or detaching/replacing the observer
invalidates that authority. Replacing only the callback does not reset the
native query. Native typing, selection and activation never wait for OCaml.

`Set_query` accepts up to 4096 UTF-8 bytes without NUL/CR/LF. It replaces the
private query, puts the single-line caret at its end, and clears query undo
history. It does not modify the captured application document editor. A hidden
query may be set, but `Focus` returns `Unavailable` while search is hidden.
Query/focus/highlight mutations require current native interaction ownership and reject IME composition.
`Read_snapshot` may observe a mounted palette without focusing it.

`Highlight (Some id)` requires a currently matching, enabled registry command;
it cannot select a hidden/disabled/missing command or activate anything.
`Highlight None` persists through ordinary renders and configuration updates.
Query edits and native navigation resume ordinary selection; Down from no
selection picks the first enabled command, Up the last. Highlight changes update
snapshots and native row reveal without changing query identity.

## Delayed requests and query checks

`command_if_query_unchanged t ~expected command` additionally compares the
expected query revision against current native input immediately before execution.
The implementation refreshes from `InputState.bridge_revision`, including edits
whose observer notifications have not yet run. Typing away and back invalidates
the old query identity even if the final string is identical. Selection-only
changes do not. This check is atomic with this native command; it is **not** an
atomic check for a later OCaml View/config result publication.

Successful responses contain the resulting snapshot and mean the command was
applied, not that a frame was painted. The controller never lets an old reply
replace a newer observation or a replacement subscription. The App validates
correlation, window/node/observer identity and non-regressing sequence/query
revision before completing a request. Wrong identities and duplicate responses
do not complete another request. Closing completes pending requests once.

Errors are typed: `Not_mounted`, `Closed`, `Stale_palette`, `Query_changed`,
`Composing`, `Unavailable`, `Invalid_query`, `Busy`, and `Native_failure`.
At most 64 palette requests may be pending per application. Query/selected-ID
response bytes count toward the existing mailbox budgets. Counter exhaustion
returns failure rather than wrapping native query identities.

The unpublished exact protocol epoch 3 appends message 22 `Palette_command`
and event 80 `Palette_result`, leaving previous tags unchanged. Both packages
ship at the same repository revision; this is not a backwards-compatible ABI
promise. Independent OCaml/Rust fixture bytes cover the appended messages.

[Loading](palette-loading.md) is a presentation-only command: it preserves query
identity, composition and current rows, exposes busy state and suppresses empty
content. It may run without owning interaction. Its applied state is observable
through `Snapshot.loading`; use the query-checked command for delayed completion.
Atomic external-result admission and persistent palette embedding remain separate
required catalog work. These controls do not close those gaps.
