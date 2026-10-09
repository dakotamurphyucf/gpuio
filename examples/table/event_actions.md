# How `Event_actions` protects delayed dialog actions

[README](README.md) · [Implementation](event_actions.ml) · [Interface](event_actions.mli)
· [Caller](main.md)

`main.ml` creates this helper once per window, forwards table requests to
`request`, and calls `view` with reactive pager/table output plus a function
returning the latest snapshot. The helper renders an ordinary modal inspection
dialog; it does not open a native context-menu service or load additional data.

## Target identity and reactive ownership

The public `t` is abstract. Internally it is a `Target.t option B.Expert.Var.t`.
`Target.t` contains a `Table_data.Row_ref`, an `int64` query generation, and a
fresh `Type_equal.Id.Uid` session. It retains no row payload or source snapshot.
The membership detects removal/reinsertion; generation detects a new query;
session distinguishes reopening the same row in the same query.

`create ()` starts with no target. `request` returns an `E.of_thunk`, so native
request delivery changes state through the effect scheduler. Activate and
Context on a Row or Cell create a fresh session and publish a target. Context
on Empty or Column clears it. Select/Resize/Move/Sort/Copy leave this helper alone.
The caller captures the pager generation when handling the request.

`valid` requires matching generation and `D.contains_ref snapshot.data target.row`.
Checking only the displayed row ID would miss a removed and reinserted membership.
`is_current` also compares the complete target against the current variable with
PPX-derived typed `Target.equal`.

## Bonsai view and retirement

`B` abbreviates `Bonsai.Cont`, `E` means `Bonsai.Effect`, and `graph` owns the
view computation. Opening `B.Let_syntax` enables `let%arr`: target, snapshot,
and output are reactive dependencies, and their changes rebuild dialog content.
`and` binds dependencies rather than launching threads.

`B.Edge.after_display` creates a deferred close effect when a target is no longer
valid. `close_target` clears it only if that exact target is still current, so
an old scheduled retirement cannot close a newer dialog. Invalid content is
also omitted immediately during derivation. Valid content resolves the row's
current payload using `D.find`, then unwraps successful table output to obtain
its controller. The description therefore follows current streamed data rather
than displaying a payload captured when the dialog opened.

`View.dialog` uses a validated `Overlay.Config` label and width of 560 logical
pixels. `on_dismiss` returns the same guarded close effect as its close button.
Native modal focus policy remains owned by the runtime; application state decides
whether content is present. `View.text` with `User_select true` makes the event
description selectable. It does not copy text to the clipboard.

## A reveal interaction, including stale work

Return or a context request travels from the native table to the caller's
`on_request` effect, then `request` publishes the target. Bonsai derives a dialog
with fresh row content and GPUIO mounts its native modal surface.

Clicking “Reveal result” runs `E.bind` on a thunk that checks both session identity
and `valid target (current ())` at execution time. Unlike reactive `let%arr`, this
bind sequences effect results. If stale, it returns `E.Ignore`. If live,
`W.Controller.batch` combines `Set_selection (Cell ...)` and `Reveal` for the
result column; `E.Many` submits that batch and closes the matching dialog.
A combined batch matters because a newer batch can supersede an undisplayed one.
Native dialog dismissal restores table focus; the platform Copy shortcut can then
copy the complete retained result cell text.

If a query resets between rendering and clicking, the execution check rejects
the old action. If a newer dialog opens, even for the same row/query, its session
is different, so an old action cannot reveal or close it. The table controller
also guards mounted query and row membership. These are complementary lifetime
checks, not permission to reuse stale output indefinitely.

## Build, tests, and adaptation

This module belongs to the table executable and has no standalone entry point.
From the repository root in the [configured environment](../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/table/main.exe
_build/default/examples/table/main.exe
GPUIO_JOBS=2 python3 scripts/test_table_public.py
GPUIO_JOBS=2 python3 scripts/test_table_appkit.py
```

[Main's walkthrough](main.md) explains exact harness prerequisites and limits.
The public self-test captures old actions, changes query/removes rows/reopens
dialogs, and checks that obsolete actions cannot alter current state. Several
requests and callbacks are invoked directly. The separate macOS AppKit harness
uses real keyboard/pointer and Accessibility automation for context and focus
flows. Neither module-level review nor direct effect tests establish clipboard,
IME, VoiceOver, or Linux desktop qualification.

When adding async inspection, keep jobs in an explicit window/application scope
and guard completion against this target identity again. Keep only identity in
the target, resolve current payloads for display and execution, and preserve the
session check even when a stable row ID appears sufficient.
