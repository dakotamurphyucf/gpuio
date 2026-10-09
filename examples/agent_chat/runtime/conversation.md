# A shared conversation and its producers

[conversation.ml](conversation.ml) owns one in-memory conversation: saved history,
response documents, attachment documents and asynchronous acceptance/streaming.
[conversation.mli](conversation.mli) is the public example contract. `Phase` and
`Message` are nested modules in these two files; there are no separate
`phase.ml` or `message.ml` components in this directory.

The conversation belongs to the application, so a response can keep streaming
while its row is offscreen or its originating window closes. The window owns the
pending acceptance step and its native editor; accepted response work has a
separate lifetime. This is the distinction to preserve when adapting the demo.

From the repository root, after [isolated setup](../../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/agent_chat/main.exe -j 2
_build/default/examples/agent_chat/main.exe
```

Send a prompt, try **Cancel** and **Retry response**, open another window on the
same conversation, and use **Load older**. The paperclip adds a local UTF-8 text
attachment. All saved messages, responses and tool examples are synthetic or
session-local; there is no provider, credential, persistence or patch execution.
macOS is the v1 desktop target; Linux builds/unit checks remain required and
Linux GUI qualification is [informational](../../../docs/platform-release-policy.md).

## Read the contracts before the transitions

Read `Phase`, `Message`, `Response`, `Active` and the principal `t`, followed by
`create`, `current`, `submit`, `accept_response`, `start_stream`, `finish`,
`cancel`, `retry`, `initialize` and `attach`. The
[application assembly](../application.md) constructs conversations once under
`App.scope app` and passes the same objects into independent workspaces.
The [fake backend](../model/fake_backend.md) supplies a pure plan; this module
turns that plan into timed document changes.

| Data | Meaning and ownership |
| --- | --- |
| `Phase.t` | `Idle`, `Accepting`, `Streaming`, `Complete`, `Cancelled`, or `Failed of string` |
| `Message.body` | `Plain` text or `Rich` containing a scoped document and a display mode |
| `Message.t` | Author/body/detail metadata for one transcript row |
| `Response.t` | A response document, its assistant row ID and its captured prompt |
| `Active.t` | One attempt's integer token, producer scope and optional accepted response |
| `t` | Shared application controller, conversation scope/ID/title, pager, phase observation and bounded session counters |

The interface abstracts `t`: the caller queries it and uses its operations rather
than changing internal mutable fields. Mutation stays on the application's UI
domain. `id` is the application-supplied conversation ID; integer pager keys are
transcript row identities, not native document handles. A response token
identifies a particular attempt and rejects obsolete delivery, even if retry
reuses the same row/document. This module does not define a separate model per
window or per visible list row.

`create` obtains a named child scope and initially stores rows 161–200, the latest
40 of 200 synthetic saved messages. `history_row` alternates Assistant/You by
parity. `Pager.create` starts with a `Before` cursor `161` and `After = End`.
Its loader sleeps 0.15 seconds and returns up to 40 earlier rows; reaching row 1
sets the older boundary to `End`. There is no newer saved page. The
[List_paging interface](../../../lib/eio/list_paging.mli) explains scoped loading,
reactive snapshots and preservation of pending older-page requests when appending.
If pager construction fails, `create` cancels the newly made conversation scope.
History requests use up to two reusable pager workers, lazily started and charged
to the conversation's shared task quota until it closes. In this demo only the
older boundary loads. A reset cancels old production but waits for its cleanup
before reusing that worker; intermediate queued resets do not become a backlog
of history fetches. The separate response-stream task remains independent.

Initial phase is `Idle`, no active/last response exists, `next_row` is 201,
response/attachment counts are zero and seeded document initialization has not
run. Each accepted send uses `next_row` for the user row and `next_row + 1` for
the assistant row; an attachment uses one row. IDs advance and are not derived
from the current number of mounted rows. `Pager.append` requires unique keys and
a known latest boundary, preventing accidental insertion into an unknown tail.
The demo admits at most 64 accepted responses and eight attachments per
conversation. Retry does not add another response or row.

## Follow submission, acceptance and publication

In [workspace.ml](workspace.ml), `send` invokes the native `Editor.submit` command.
Its submission handler receives a typed `Input.Submission`; `submit` extracts
that captured text/configuration and calls `Conversation.submit`. The editor,
selection and revision live natively, rather than in `Message.t`.

`Conversation.submit` returns `unit Or_error.t` immediately after admitting the
operation. It rejects busy conversations, the response limit, blank-after-strip
prompts, prompts above 16384 bytes, invalid UTF-8 and NUL. It does not normalize
the accepted prompt's whitespace. An `Ok ()` means work was scheduled; it does
not yet mean rows were appended, the editor was cleared or a frame appeared.

It creates a `send-acceptance` child of the **originating window scope**, increments
the token, stores an active attempt with `response = None`, and sets `Accepting`.
`Scope.Expert.on_cancel` registers a small synchronous cleanup that clears this
attempt and restores `Idle` only while its matching token is still unaccepted.
This private cleanup hook is an implementation detail; ordinary application
resource cleanup uses the public scoped contract. It performs no I/O.

`Scope.start` runs the acceptance delay as an Eio fiber. The `sleep` capability
was supplied by application startup, so the backend's stored timing values do
not themselves wait. `on_result` returns a Bonsai effect delivered on the UI
loop. Producer failures set `Failed` only if the attempt is still current;
external Eio cancellation remains cancellation in the
[Scope contract](../../../lib/eio/scope.mli).

After successful delay, the effect calls `Document.create` with
`Source.empty_stream ()` under the **conversation scope**. `let%bind` here is
`Bonsai.Effect.Let_syntax`: it sequences deferred actions/results, unlike Bonsai
`let%arr`, which derives reactive values. `Document.create` completes after its
initial native snapshot is published. Because creation is asynchronous, the code
checks `current` again afterward; if obsolete it releases a successfully created
document rather than adopting it.

`accept_response` creates a conversation-owned `response` producer scope and
appends both rows. On success it replaces the window-owned active attempt with
the accepted response, cancels the now-unused acceptance scope, records `last`,
increments the accepted count and row sequence, and calls `start_stream`.
Changing active ownership before canceling acceptance keeps the acceptance
cleanup from clearing the new producer. Failed setup releases the document,
cancels the active attempt and records failure.

Only then does the caller's `on_accept` effect run. In the workspace this calls
`Editor.clear_if_unchanged panel.editor submission`. If the user typed a newer
draft while acceptance waited, the native revision check preserves that newer
text and the status says so. The conversation has already accepted the captured
prompt; preserving a newer draft does not undo that send.

## Stream, finish, cancel and retry

`start_stream` sets `Streaming`, updates the assistant row's detail, and starts
one task under the active producer scope. Its loop consumes
`Backend.plan config ~prompt:response.prompt`:

- For `Chunk bytes`, sleep for the configured interval, check `current`, then
  call `Document.push_bytes` and continue.
- For `Finish`, return success; for `Fail text`, return an explanatory error.
- An unexpectedly empty plan is an error, since a valid fixture plan has a
  terminal step.

`current` requires both conversation/attempt scopes active and the stored token
matching. `Scope.start` itself also suppresses canceled queued completions.
Its result wraps the loop's own `Or_error.t`; `Or_error.join` flattens those two
error layers before the completion effect calls `finish`.

`finish` ignores obsolete results. For a current result it removes `active` and
cancels the producer scope. Success calls `Document.finish`, updates the row to
Complete and sets phase `Complete`. Failure calls `Document.cancel`, stores the
error in the row and sets `Failed error`. The document remains registered under
the conversation scope, preserving completed text for inspection or retry.

The [document interface](../../../lib/eio/document.mli) matters here: byte chunks
may split a Unicode scalar, so `push_bytes` buffers at most three trailing bytes.
`cancel` drops an unfinished scalar and publishes all previously accepted complete
Unicode content. Native document publication is asynchronous and coalesced;
accepted desired text is not evidence of parser completion or physical painting.

`Conversation.cancel` is a synchronous UI-domain operation. With no active
attempt it does nothing. During acceptance it clears/cancels the attempt and
returns to `Idle`, without adding rows. During production it cancels the task,
marks the document canceled, keeps partial output and sets `Cancelled`.
`retry` rejects busy state or the absence of a last response. Otherwise it creates
another conversation child scope, resets the same document to empty, obtains a
fresh token and restarts using the new captured configuration. It does not append
another user prompt, change response count or move the response to a new row.
Retry setup cancels its new scope if reset fails.

## Reactive views and visibility

`phase_value` exposes `Bonsai.Cont.Expert.Var.value t.phase`; `set_phase` publishes
changes with `Var.set`. `phase` reads the current value for imperative controller
logic. `Pager.value` supplies reactive transcript snapshots. The workspace's
`conversation_panel` combines them with `let%arr` to derive its controls and
paged list. Its row callback derives `message_view` from reactive row data;
[Chat_message](chat_message.md) adds framing around plain text or a native
`View.document` configured with the existing document handle/mode.

Integer row keys preserve list identity. `List_view.paged` mounts a bounded set
of rows and owns each window's scroll state. **Load older** invokes
`Pager.request ... Before`; prepending history preserves the view's reading
anchor through the paging/list contracts. That loader and response production
are conversation-owned, so unmounting a row, hiding the inspector or retaining a
tab does not cancel them. Closing a window cancels an unaccepted send from that
window, but an already accepted stream can keep updating another window.
Closing the application cancels the shared scope and its document resources.
See the [ownership design](../../../docs/design/agent-workspace.md).

## Seeded artifacts and attachments

`initialize` is a one-shot effect: it marks `initialized` before walking the
three fixtures and creates each document under the conversation scope. It
replaces saved rows 198–200 with Markdown, OCaml code and a diff. If a document
fails to create while the scope is active, that row becomes a plain Preview
unavailable diagnostic. A late successful creation after closure is released.
This is not an automatic retry mechanism or a tool invocation; the initial
`Tool · workspace.inspect` label describes sample content.

The attachment path is separate from generation. Workspace `attach` opens the
picker and reads a file in the originating window's task scope. It then invokes
`Conversation.attach ~name ~text`, a deferred effect that checks the conversation
scope, reserves an attachment slot and validates the name/text: name nonempty,
up to 4096 bytes, text up to 65536 bytes, both UTF-8 without NUL. Empty text is
allowed. It creates a conversation-scoped document, appends a plain-text code
artifact with the filename/byte count, and keeps the slot on success. Source/
document/append failures release applicable resources and undo the reservation.
This path does not change the response phase, so inspecting an attachment and
streaming a response are different operations. No path is stored for later disk
reads or writes.

A small adaptation is to reduce the saved-history page size: change both the
initial row range/cursor and loader stride consistently, keep row IDs unique,
and retain the `End` boundary at row 1. To add real provider I/O, replace the
pure plan consumption inside the scoped producer with a bounded streaming
service; preserve byte-decoding, attempt tokens, conversation ownership and
terminal document updates. Do not tie producer cancellation to a visible row's
lifetime. Persistence requires an explicit storage service rather than assuming
these mutable fields survive application exit.

Optional controller/native checks and their actual evidence are listed in the
[README](../README.md#validation). The supporting `self_test.ml` remains a separate
verification component. This walkthrough's source/link review adds no new
keyboard, GUI, attachment-picker or multi-window acceptance claim.
