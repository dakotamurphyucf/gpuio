# How `Self_test` drives the chat ownership scenario

[README](README.md) · [Implementation](self_test.ml) · [Interface](self_test.mli)
· [Application caller](application.md)

`Application.run` calls `start` only with `--self-test`, after creating the
conversations and first window. This module creates no UI graph or reusable
component; it drives public controllers against already-mounted workspace graphs.
It is an optional scripted graphical integration check, not ordinary startup or
physical input automation.

`App` owns the native runtime; `Scope` owns tasks; `Editor`, `Document`, and `Pager`
are public Eio adapters. `Conversation`/`Workspace` are demo runtime modules, and
`E` abbreviates `Bonsai.Effect`. `perform` enqueues a UI callback, handles an effect,
maps its result to an Eio promise, and awaits it. `sync` wraps a thunk through
that bridge. There is no `let%arr` in this module: reactive view/model derivation
happens in the workspace, while this script sequences effect results and observes
current outputs. `expect_editor` raises a typed command error on failure.

## Programmatic interaction trace

The task waits for the first editor and list viewport, then selects a deterministic
fake backend with three-byte chunks, 20 ms chunk delay, and 300 ms acceptance delay.
It replaces/submits `First prompt λ`, waits for Accepting, and replaces the draft
with `A newer draft` using recorded undo. Acceptance must not clear that newer
editor revision; the notice and snapshot confirm protection.

Selecting conversation 2 updates workspace state and mounts/retains its panel.
Its programmatic submit starts a second response while conversation 1 is still
streaming. Cancelling response 2 changes its phase without stopping response 1.
The script scrolls conversation 1 to key 175, requests history while generation
continues, and checks its native keyed anchor survives. Completed document source
must equal the fake backend's response exactly; document publication and the
32-active-row budget are checked separately.

Closing/reopening a tab checks hidden focus denial and retained draft/anchor.
A second window over the same conversation starts with its own empty editor;
changing that editor must not replace the first window's draft. Pending submit
acceptance for conversation 3 belongs to the first window scope. Closing that
window must cancel acceptance and suppress its callback, while an accepted
response belongs to conversation scope and can survive window visibility.
See [conversation](runtime/conversation.md) and [workspace](runtime/workspace.md).

Remaining stages retry a cancelled response, attach bounded UTF-8 text, reject
invalid/oversized attachments, force a partial-response failure and retry it
without adding a second response, toggle the second window theme, and await a
native frame. Attachment text is supplied directly here; no native picker or real
file read is exercised. Fake responses use no network credentials/service.

## Completion, counters, and limitations

Each predicate wait has a 25-second deadline and polls at 5 ms; the final frame
has a 5-second deadline. `perform` itself has no internal timeout, so these bounds
are not a whole-process watchdog. The result callback shuts down then unwraps
errors. `on_pass` marks the application assertion before later traffic assertions;
normal exit is required in addition to that callback or log text.

Traffic checks compare admission attempts/accepted messages/bytes, drain calls
against runtime turns, and event counts against render notifications. Printed
elapsed time includes the entire scripted scenario, not a microbenchmark or user
input latency. `GPUIO_AGENT_CHAT_PUBLIC_OK` appears before terminal runtime cleanup;
its wording does not replace separate resource-cleanup evidence. Programmatic
commands/native observations do not establish physical keyboard, IME, VoiceOver,
clipboard, pixel appearance, or Linux desktop qualification.

From the repository root in the [repository environment](../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/agent_chat/main.exe
_build/default/examples/agent_chat/main.exe --self-test
python3 scripts/test_agent_chat.py
```

The first launch uses real native windows and must be left undisturbed. The
separate script has its own macOS automation prerequisites described in README;
it is not this module's execution path. No `self_test.exe` exists. For new cases,
use current controller targets, typed results, deterministic fake configuration,
and assertions about ownership rather than merely waiting fixed times.
