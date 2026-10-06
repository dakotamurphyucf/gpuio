# Compose one native conversation workspace

[workspace.ml](workspace.ml) and [workspace.mli](workspace.mli) assemble one
window's sidebar/search, retained conversation tabs/composers/transcripts,
commands, inspector, settings and close decision. The application shares
conversation objects across windows; this module owns each window's native
editors, viewports, tab preferences and UI state.

From the repository root after [isolated setup](../../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/agent_chat/main.exe -j 2
_build/default/examples/agent_chat/main.exe
```

Select a conversation, type/send (Enter; Shift-Enter newline), load older, close/
reopen its tab, open another window, attach text and inspect artifacts. Try
Primary-Shift-P commands, Primary-N new window, Primary-Shift-[ / ] tab changes
and Primary-W close tab. Primary is Command on macOS/Control on Linux. The
[README](../README.md) lists visible controls and optional runners; macOS is the
v1 target and Linux GUI checks remain
[informational](../../../docs/platform-release-policy.md).

## Reading order and window state

Read Panel/t, `create`, `select`, `close_tab`, basic setters, `config`/`list_config`,
`submit`/`send`/`attach`, `document_view`/`message_view`, `conversation_panel`,
close handling, then `component`. The long final function mainly declares layout
and command/view composition; its helpers isolate native asynchronous operations.

`t` stores shared Icons/conversation references plus observable Workspace tabs,
dark appearance, notice, backend config, command-palette/demo-controls visibility,
close-pending state, a pending close callback, panel registry and independent
Inspector/Settings controllers. Initial tabs contain only the chosen conversation;
theme is dark, backend default, overlays closed and panel registry empty.
`create` requires a nonempty supplied conversation list with a matching selected
ID; its find_exn operations assume those application invariants rather than
returning recoverable configuration errors.

[Application.run](../application.md) supplies exactly three seeded conversations
and limits live windows to four. This module maps **all supplied conversations**
into retained panel graphs; it does not separately enforce a three-item count for
arbitrary callers. A tab carries the shared conversation ID, not a copied store.
`select` selects an existing tab or adds one; `close_tab` removes its Workspace
entry without releasing that conversation's editor/list. The
[Workspace tab contract](../../../lib/core/workspace.mli) deliberately owns order/
active selection, not application task disposal.

`Panel.t` contains an Editor controller and List_view output for the same
conversation in this window. `Bonsai.Edge.after_display` stores their current
values in the Int-keyed panel registry; this is post-display controller setup,
not physical frame validation. Window scope cleanup clears the registry and
pending close callback. Controllers can become stale after native destruction;
commands still validate their leases at invocation.

## Construct graphs once and derive views

`component` constructs theme observation, [Inspector](inspector.md),
[Settings](settings.md), native single-line search and every conversation panel
before its final `let%arr`. Bonsai values describe dependencies; `let%arr ...
and ... in` derives a value/view from their current observations. It does not
start I/O each time layout/style changes. `Bonsai.all` combines panel reactive
views, keeping their graphs independent of the list of currently open tabs.

The search Editor observation supplies a lowercase query; the sidebar filters
lowercase conversation titles by substring. This reads a last observation,
appropriate for a visual filter, rather than claiming an exact current native
submission. Search itself is outside the conversation panel registry used for
close decisions.

The final layout renders all panel views inside stable panel-<conversation-id>
`View.tab_panel` wrappers with active flags. Closing a tab hides its panel;
reopening preserves its native draft/selection/undo and list position under the
retained tab-panel contract. This costs native editor/list retention for all
three seeded panels even when only one tab is open; it is bounded demo policy,
not unbounded caching. Hidden panels do not receive native input.

Outer workspace-split gives sidebar initial width 250, range 210–380 and second
pane minimum 500. Inner inspector-split initially gives conversation side 550,
minimum 360, maximum 1400 and inspector minimum 320. Native divider pointer/
keyboard/accessibility interaction handles resizing. [Responsive helpers](responsive.md)
choose sidebar branding/conversation toolbar based on assigned container width,
without duplicating editors inside those alternatives. Inspector closing restores
space without destroying its application models; pane minimums remain constraints.

## Conversation panel and native document boundary

`conversation_panel` creates a window-scoped Editor with multiline config,
2–4 rows and typed on_submit effect. `List_view.paged` reads the shared
conversation pager snapshot and controls using Int comparison and stable
integer-to-string row keys. Configuration uses estimated 160-pixel row height,
240-pixel overscan, max_active 32 and Follow_tail_when_at_end. Each native viewport
can therefore stay at its own reading anchor while shared data changes.

Its row callback's `let%arr message = data and dark = dark and icons = icons`
derives message framing. Plain messages use selectable wrapping text.
`document_view` uses existing conversation-owned Document.handle and mode,
Flow layout, appearance/label, collapsed code/diff by default, plus a navigation
intent effect that updates notice. Markdown links do not launch a browser;
code/diffs are displayed/copied, never executed/applied. [Chat_message](chat_message.md)
wraps that content; native document views own selection/expansion/copy while the
shared controller owns their source/stream lifetime.

Observed conversation phase derives busy, status, Cancel/Retry/Send controls
and [shared activity motion](chat_motion.md). Header Load older calls
Pager.request Before; Latest explicitly commands that window's list to jump to
the tail. Following growth when already at the tail differs from forcing a user
who scrolled upward back to the newest row.

## Trace exact Send, later typing and cancellation

`send t editor` sequences Editor.submit, which reads an exact native submission
and invokes on_submit; it does not send an old Bonsai snapshot. Enter follows the
same native handler path. The [Editor contract](../../../lib/eio/text_input.mli)
rejects active composition/hidden/unavailable leases rather than inventing text.
The typed handler `submit` checks the panel registry and calls Conversation.submit
with originating window scope, current backend and captured Submission.text.

Synchronous admission can fail immediately; an accepted scheduling result still
waits for conversation acceptance. After acceptance, its callback sequences
Editor.clear_if_unchanged against the original submission. Newer native typing
or a replacement editor causes Stale_revision/Stale_editor, so notice says the
newer draft was kept. Matching clear is an explicit native undo-recorded command.
This controller never rewrites the composer merely because response text grew.

[Conversation](conversation.md) owns the transition from window-scoped acceptance
to conversation-scoped response production, token guards, byte decoder and
terminal document updates. Cancel stops the current conversation attempt and
retains completed partial response. Retry response deliberately constructs a
**default** Backend.Config rather than using the selected slow/error preset or
saved backend, allowing the interrupted fixture to finish normally. Running
streams keep their captured configuration when settings/presets change.
Another window sees shared response/phase while retaining its own composer/list.
Native row eviction or tab close does not own producer cancellation.

## Attach files through the originating window scope

`attach` first sequences File_dialog.open_ with optional directory hint and title.
Cancel yields Ignore; picker errors update notice; exactly one selected path is
required. Scope.start under App.Window.scope runs the supplied read_file capability.
Completion is delivered as a UI effect; window cancellation suppresses late reads.
It derives a basename only for UTF-8 path bytes, otherwise uses Text attachment.
Conversation.attach validates/adopts content into conversation-scoped documents.

The [application reader](../application.md) closes flows and bounds reads; the
[conversation guide](conversation.md) explains eight attachments/64 KiB each,
UTF-8/NUL/name validation and resource rollback. No file write, path normalization,
provider upload or persistence occurs here. A picker hint is not a filesystem
sandbox, and byte-path identity is separate from a display label.

## Commands, settings and deferred close decisions

The final `Command.Registry` holds application effects for palette/new-window/
tab changes/theme/settings/inspector/close actions plus native Copy. Root
View.command_scope supplies the menu and palette with the same registry; each
surface lists its chosen subset. The command palette model is a Boolean, and
its on_dismiss clears it. Native menu/shortcut dispatch is scoped to this window;
Copy remains native rather than reading/writing clipboard text from OCaml.

Theme Edge.on_change calls App.Window.set_theme with [Palette.theme](palette.md).
Settings generation callback sets Backend through the pure validated translator;
its score filters affect Results and its annotation observation feeds Inspector/
Diagram. Demo presets set default, seven-byte/0.06-second slow or fail-after-eight
backend configurations independently of saved settings. None contacts a service.

`install_close_handler` reads **current native snapshots** from every retained
composer using Effect.all. Nonempty text or active composition counts as a draft.
Closed/Stale_editor/Not_mounted means no live draft; other errors conservatively
require a decision. With no drafts it returns Close_decision.Allow. Otherwise
Effect.Expert.of_fun retains the callback and publishes close_pending; this
private continuation hook connects the close protocol to an ordinary dialog.

The Unsaved drafts dialog resolves that callback exactly once through
`answer_close`: clear callback/pending first, then Keep_open or Allow. Dismiss
means Keep_open. Scope cleanup removes pending references when the window closes.
Closing a window does not erase another window's drafts; accepted shared streams
retain conversation scope while the application is alive. This decision checks
conversation composers, not a persisted session or every other editor in the UI.
No unconditional close is inferred from a modal button being submitted.

## Adaptation and verification boundaries

A small adaptation is another conversation fixture: provide a stable unique ID,
update application retained-panel/resource policy and keep all-ID panel graph
construction deliberate. Do not tie stream/document lifetime to tab entries or
bounded rows. For a new async UI action, run it with an explicit window/conversation
scope, preserve typed error/native lease checks and report completion separately
from request admission. Native editor observation does not imply permission to
replace its draft.

The optional --self-test runner drives real windows through public controllers;
external scripts in the [README](../README.md) exercise OS input/accessibility and
reap their owned child processes. Required Linux nongraphical checks and physical
macOS evidence remain separate. This source/link review performs no builds,
keyboard, IME, picker or multi-window acceptance run. Bonsai post-display,
command acknowledgement and native transaction acceptance also do not prove a
physically presented frame.
