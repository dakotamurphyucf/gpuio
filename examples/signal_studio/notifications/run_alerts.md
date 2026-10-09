# Explicit opt-in, one notification and ordered reply races

[run_alerts.ml](run_alerts.ml) and [run_alerts.mli](run_alerts.mli) implement an
application/UI-domain notification controller. It is mutable coordination around
injected asynchronous Bonsai effects, not a Bonsai graph or OS backend. The
adjacent [expect tests](test/run_alerts_test.ml) share this complete guide.
Read Backend/State/create, message/error helpers, content, finish/pump,
authorization/public actions, handle_event and terminal close.

From the root after [setup](../../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/signal_studio/main.exe -j 2
GPUIO_JOBS=2 ./scripts/gpuio exec dune runtest examples/signal_studio/notifications -j 2
./scripts/gpuio exec dune exec examples/signal_studio/main.exe
python3 scripts/test_signal_notifications.py --output scratch/signal-notifications
```

The nongraphical expect tests never request OS permission. The macOS harness uses
a disposable packaged/signed identity and actual notification interaction, as
explained in [README](../README.md#local-checks). Unbundled macOS normally reports
Unavailable; posting success is OS acceptance, not proof of visible presentation.
No command was newly executed in this documentation review.

## Typed backend and state

Backend.t contains authorization, authorize, capabilities, post, replace and
dismiss functions returning typed Result effects, plus nonblocking close.
[Notification values](../../../lib/core/notification.mli) are pure validated
plaintext content, action IDs, authorization/capability snapshots and opaque
process-local receipts. [Eio.Notification](../../../lib/eio/notification.mli)
owns the actual application service; it does not automatically activate a window.
State exposes enabled, busy, has_notification, optional authorization and message.
create stores injected activate/on_state/log callbacks, one optional receipt,
one latest pending run, one optional event waiter and a closed flag.

publish derives has_notification from receipt and invokes on_state; message also
logs. report_error disables automatic alerts for Denied/Unavailable/Closed/
Not_ready/Native_failure, but preserves enabled for Invalid_request/Unsupported/
Busy/Stale. It reports an in-app fallback, never removes the run result from the
workspace. has_notification represents owned receipt state, not visible pixels.

content builds a Run NN complete title, capability-dependent body and optional
open-workspace action. Sound defaults Silent. Capability support is distinct from
permission; a server can support actions but still refuse posting.

## Probe and explicit Enable

probe calls authorization with enable:false; it queries without prompting or
opting in. enable uses the injected authorize operation with enable:true.
The authorization helper admits only while open/idle, sets busy, awaits the effect,
records/logs the Result and then finishes. Authorized/Provisional/Not_required
can enable only for explicit Enable; Denied and Not_determined leave it off.
Probe on an already enabled controller does not itself force off a valid opt-in.
A busy authorization request is ignored rather than queued.

[Application](../application.md) injects N.authorization and an authorize flow
that calls N.retry then N.request_authorization. It probes at startup after ready,
and the Alerts popover dispatches Enable separately. enable/notify do not embed
OS policy into view construction. E.bind sequences asynchronous completion;
E.of_thunk delays UI mutations until effect handling.

## Coalesce requests and update one logical receipt

notify validates 0–100 and records pending_run. While busy, newer valid requests
replace that one slot. pump takes it when idle. If alerts are off, it reports
completion plus fallback without posting. Otherwise it sets busy, obtains current
capabilities and constructs content. With no receipt it posts; with replacement
support it replaces using the same receipt; otherwise dismissal support permits
dismiss-before-post. Without either capability it reports Unsupported rather than
creating duplicate owned notifications. A Stale replacement clears the receipt
and reports; it does not immediately retry the same request as a new post.

finish clears busy, publishes, releases an event waiter and pumps the latest
pending run. Closed checks suppress late-result state changes. Only one controller
mutation is active; the latest pending run is bounded rather than an unbounded
queue. Application uses tag completed-run for every new post; Receipt identity
still distinguishes successive lifetimes even when their tags match.

Trace an early action: Enable → Notify 1 starts post, reply delayed → Notify 2 then 3
leaves pending 3 → action for receipt 1 arrives while busy and waits asynchronously →
post reply installs receipt 1 → finish wakes handler → exact receipt/action activates
current workspace and clears ownership → pending 3 can post a new receipt.
The tests deliberately control this ordering. Matching receipt identity, not the
tag or a captured run/window, is the routing authority.

## Event ordering, dismiss and terminal cleanup

handle_event's Expert.of_fun defers a callback while busy; exactly one waiter
is allowed. The service must deliver handlers serially and await their effects,
as Eio.Notification.attach guarantees. This is not a general concurrently invoked
event handler. Once idle, Failed clears receipt/disables and reports. Activated,
Action and Closed are ignored for foreign/stale receipts. A matching terminal
event clears ownership; default activation or exactly open-workspace calls the
injected activate. Unknown actions only report, and Closed reports termination.
Application resolves the current window when activation runs, including reopening
one after close; this controller owns no native window slot.

dismiss admits only idle/open and with a receipt. Success clears it and reports
a removal request; service-specific semantics may acknowledge submission rather
than verified OS disappearance. Errors preserve controller receipt until later
policy/event handling. close is idempotent and terminal: sets closed, drops pending/
receipt, resets flags/message, closes backend and releases waiter. It deliberately
calls neither on_state nor log, making nonblocking scope cleanup safe. Late replies
and events cannot re-enable or activate; close is application-owned, not tied to
a popover/window mount. There is no automatic reconnect/polling policy here.

## All five expect tests and their limits

[test/dune](test/dune) enables ppx_expect with Core/gpuio/Bonsai and the controller
library; [library dune](dune) needs no Eio/OS service. The backend fixture uses
E.return typed success/error, pure capability records and Expert receipt decoding.
E.Expert.handle drives effects; Expert.of_fun stores a callback to simulate an
unresolved reply. It never bypasses OS ownership in the running application.

- Probe/opt-in checks Authorized posts only after Enable, while Denied and
  Unavailable post zero times and keep alerts off.
- Latest-pending/early-action delays post 1, queues 2/3 and action 1, checks no early
  activation, then resolves it and confirms titles 1/3. Stale receipt 1 cannot
  activate again; receipt 2 activates once and clears ownership.
- Replacement policy checks post→replace versus post→dismiss→post, and omitted
  body/actions when those capabilities are false.
- Close fencing rejects run 101, closes twice while post/event wait, resolves late
  success, and confirms no activation/new post plus exactly one backend close.
- Submission failures Denied/Unavailable/Native_failure/Not_ready each cause only
  one post and disable subsequent automatic delivery. A Failed service event
  clears an existing receipt and reports its in-app fallback.

[%expect] checks printed state/call ordering; typed assertions catch forbidden
transitions. These tests do not prove OS permission prompts, visible banners,
Notification Center actions, real daemon reconnection or every error/capability
combination. The native harness separately exercises packaged behavior; Linux
GUI qualification is deferred while required nongraphical checks remain separate.

To adapt, keep the injected backend for deterministic reply-race tests, preserve
serial event delivery and resolve current application state for activation. Add
new action IDs/content validation and explicit policy, not captured window slots
or permission requests hidden inside automatic posting.
