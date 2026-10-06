# Compose a transcript message

[chat_message.ml](chat_message.ml) and [chat_message.mli](chat_message.mli) wrap
already-created content in a user message, assistant message or artifact card.
The reusable boundary is `Chat_message.view`: give it a `Kind.t`, theme flag,
registered icon handles and a `Gpuio_bonsai.View.t`; receive an ordinary view.
It creates no document, editor, task or Bonsai state.

Build and launch from the repository root with the
[isolated toolchain](../../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/agent_chat/main.exe -j 2
_build/default/examples/agent_chat/main.exe
```

Inspect the seeded Markdown, collapsed code and diff rows; send a prompt to see
user and assistant cards. Code/diff expansion and Copy source are native document
controls supplied inside the content slot, not handlers in this wrapper. Fixtures
are [local sample data](../model/fake_backend.md), never executed code or applied
patches. macOS is the v1 desktop target; Linux graphical checks remain
[informational](../../../docs/platform-release-policy.md).

## Read `view` from outside to inside

`Kind.t` is the closed variant `User | Assistant | Artifact`. It selects
presentation rather than owning conversation status. `Palette.of_dark dark`
provides the application's semantic colors, while `Presentation.Appearance.dark`
or `.light` provides the public composition defaults. Read
[palette.mli](palette.mli) for the color record and
[presentation.mli](../../../lib/core/presentation.mli) for the helper contracts.
Explicit styles refine those defaults with the app's canvas/surface/text colors.

`px` and `full` construct logical-pixel and 100%-width lengths. `Style.create_exn`
builds validated literal styles. The `common` style caps card width at 760 logical
pixels and supplies foreground, border color and corner radius; the enclosing
full-width column centers it and gives the row horizontal/vertical padding.
`Style.merge [common; ...]` refines the shared card style with the selected
background. The wrapper has no independent scrolling or list virtualization.

The 27 × 27 avatar shows `Y` for a user and `Icons.view icons Spark` otherwise.
The [icon interface](icons.mli) makes clear that the handles are registered once
in the application scope. `Icons.view` supplies an empty sized view until a
handle is available. This helper only uses the handles; it does not register or
release them.

The local `message ~author ~detail content` function calls `Presentation.message`
with the avatar and themed card style. Then `kind` selects:

| Kind | Public composition | Fixed labels |
| --- | --- | --- |
| `User` | `message` containing `Presentation.bubble` | You; Just now |
| `Assistant` | `message` with content directly | GPUIO; Local assistant |
| `Artifact` | `Presentation.tool_result` with `Presentation.marker` | Workspace · artifact; Ready |

Those strings are demo presentation, not timestamps, provider identity or a live
status model. In particular, `Ready` is a fixed marker for the seeded tool card;
it does not observe response completion. The lowercase `message`, `bubble` and
`tool_result` functions used here are the public convenience compositions, not
an implementation of the richer `Presentation.Message`/`Bubble` descriptor APIs.

## How a row reaches this wrapper

Read `document_view`, `message_view` and `conversation_panel` in
[workspace.ml](workspace.ml). `message_view` chooses `User` when the author is
`You`, `Artifact` for rich code/diff bodies, and `Assistant` for other bodies.
`document_view` renders plain strings with selectable `View.text`, or constructs
`Gpuio.Document.Config` with the existing scoped document handle, its Markdown/
code/diff mode, light/dark appearance, Flow layout and artifact label. Code and
diff start collapsed. Its `View.document ~on_navigate` effect reports link/line
intent in the workspace status; it does not open a browser.

Bonsai's `List_view.paged` render callback receives reactive `data`. The callback's
`let%arr message = data and dark = dark and icons = icons in ...` derives a new
view from their current values. A reactive value updates its dependents when its
source changes; `let%arr` is this derivation syntax, not a mutable document read.
`Gpuio_bonsai.View.t` is the public view specialized to Bonsai effect actions,
so content can carry deferred commands without this wrapper implementing them.
The model, effects and graph remain in the caller.

For a concrete interaction, expand the seeded code card and choose Copy source.
The native document owns expansion, text selection and its copy control; the
wrapper keeps the artifact frame around that same content. Streaming a later
assistant response follows [the backend trace](../model/fake_backend.md):
conversation-scoped document bytes change independently of the surrounding
message card. Row eviction can retire the mounted native document view without
canceling its underlying conversation-owned response. The stable integer row key
and bounded mounting belong to `List_view.paged`, not `Chat_message.view`.
Theme changes derive new styles/views through Bonsai while those document
registrations retain their own lifetime.

The concrete streaming path is `Conversation.start_stream` → the scoped worker
waits between fixture chunks → `Document.push_bytes` appends bytes to the existing
response document → GPUIO asynchronously publishes coalesced snapshots to mounted
native document views. Byte chunks can split Unicode scalars; `push_bytes` buffers
the incomplete scalar rather than exposing invalid text. This is not a new
`Chat_message.view` call for every chunk. `update_response` separately uses `Pager.set` when the message detail changes, such as Streaming,
Complete or cancellation; that row-data change does flow through `let%arr`. The
worker's `Scope.start ~on_result` returns an effect whose UI thunk calls `finish`;
`Document.finish` closes successful input, while cancellation preserves partial
content. Document publication and deferred effect completion do not prove a
physical frame was displayed.

A small adaptation is to give `User` a distinct accent-colored bubble. Change
only that branch's `Presentation.bubble` style, preserve readable foreground/
background contrast in both palettes, and keep the existing content slot intact.
For real authors/timestamps, add explicit parameters to `view` and its interface
and pass them from `Message.t`; do not infer them from the fixed detail strings.
Keep document registration and response cancellation outside the stateless card.
The [document controller contract](../../../lib/eio/document.mli) and
[view contract](../../../lib/core/view.mli) describe those independent resources.
Optional native checks are listed in the owning [README](../README.md#validation);
this source/prose review supplies no new GUI or clipboard acceptance evidence.
