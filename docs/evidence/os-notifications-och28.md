# OCH-28 notification evidence

Status: in progress. Local macOS native/public acceptance is implemented; the Linux
adapter and platform evidence, remaining cleanup/race acceptance and consolidated
hosted gates are outstanding. No notification capability bit is advertised yet.

## Foundation

Commit `889736d` adds typed Core values, paired bounded codecs, native lifetime state
and the application-domain delivery queue. Tests cover UTF-8/byte limits, unique
actions, malformed/oversized wire input, independent OCaml/Rust bytes, replacement
identity and action revisions, early/duplicate/foreign callbacks, dismissal retry,
service-loss/shutdown cleanup tokens, event reservation and repeated bounded cycles.
The UI-domain queue covers readiness, hint coalescing, effect backpressure, atomic
malformed-batch rejection, explicit retry and late completion after close.

The following integration additionally adds the application envelopes, native
callback mailbox accounting, bounded asynchronous response tickets, public Eio
service and owned macOS adapter. Linux currently returns `Unsupported`.

## Actual macOS observation

Local environment: macOS **14.5 (23F79), arm64**, repository OCaml/Rust toolchain,
unchanged GPUI pin `a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b`.

The public packaged example passed `scripts/test_notifications_macos.py` on
2026-09-27 with these **actual Notification Center actions**, not injected events:

- Plain-text Unicode notification submission, with native title/body observed.
- Duplicate live tag returns `Busy`.
- Native **Open workspace** action remains queued before OCaml readiness, then
  reaches its handler exactly once with the original receipt.
- Native default activation and the OS **Close** action reach distinct typed
  `Activated` and `Closed User` handlers.
- Replacement keeps receipt 4 and changes title/action to **Inspect build**;
  Notification Center exposes the new action and no old **Open workspace** action.
  Selecting it reaches OCaml as the new action on receipt 4.
- Explicit dismissal returns request acceptance; the test separately waits until
  the matching OS row disappears. A fixed 300ms assertion was too early during
  the OS removal animation, so the observer now waits with a ten-second bound.
- A posted notification targets an exact window handle. The window is closed and
  another opened; the subsequent native action reports `target Closed` and leaves
  the replacement window alive. Routing never uses a raw native window slot.
- Service close removes a live OS notification, rejects another post with `Closed`,
  and the app exits with no owned test process remaining.

Notification titles carry a demonstration sequence number. This lets the external
observer distinguish new notifications from an older banner still animating out,
rather than erroneously clicking a retired row with the same visible title.

## Permission and packaging observations

The public unbundled executable now passes a separate check: capabilities and
authorization return `Unavailable`, the intake handler receives `Failed Unavailable`,
and the app exits without accessing the aborting notification-center path.
The non-main-thread guard also has unit coverage. A bundle copied without replacing
the executable's linker signature retained signing identifier `main.exe`, with
Info.plist not bound. Its settings probe returned `Not_determined`; posting returned
`Not_ready` without prompting, and an explicit authorization request returned the
native not-allowed error, exposed as `Denied`.

After ad-hoc signing with **com.gpuio.notification-lab** and registering the bundle,
macOS displayed its permission UI. Permission was enabled for this test app through
its own System Settings notification page. Subsequent native probes and explicit
requests returned `Authorized`; the full native walkthrough above passed. This is
not a claim that submission bypasses user settings, that a user denied the initial
unsigned fixture, or that first-time permission automation has passed on hosted CI.

The test changes no global Focus policy. Its permission requirement is documented
in the example. `UNUserNotificationCenter` removal APIs have no completion result;
the API reports submission, while the external test separately observes removal.
Expiration/OS policy changes need not emit every possible close reason on macOS.

## Commands and remaining gates

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 \
  examples/notification/main.exe @test/notification/runtest @fmt
python3 scripts/test_notifications_macos.py
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test \
  -p gpuio-protocol -p gpuio-native --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy \
  -p gpuio-native -p gpuio-protocol --all-targets --locked -j2 -- -D warnings
```

The packaged native walkthrough and strict Clippy pass locally. The integration regression results are recorded below. Linux notification daemon ownership,
capability negotiation, true replacement/dismissal, cancellation and real signal
validation still require implementation. The milestone remains active, with charts,
graphics examples, required macOS/Linux CI and merge also pending.

Integration regression checkpoint: all **573 Rust native/protocol tests** pass
across 86 result suites, including 263 native unit tests. The 129-event maximum
notification batch is conservatively charged to the bridge byte budget, split
under the transport limit, and its availability hint coalesces independently of
desktop hints. Category snapshots reach the documented 144 current/candidate
bound and release all retired content. Targeted Dune notification/desktop/protocol/
view API expectations, public example build and formatting pass. These local
regressions supplement the native walkthrough; they do not establish Linux GUI
acceptance or replace required hosted gates.

## Linux transport foundation (no display acceptance)

The separate `gpuio-portal::notifications` client now talks to the freedesktop
notification service on a persistent session-bus connection. It binds calls and
signals to a unique daemon owner; owner loss is explicit, and an old ID is never
sent to a replacement daemon. Capability negotiation rejects unsupported requested
body/actions/sound, escapes plain text only for markup-capable servers, sends the
application's desktop-entry identity, and checks returned replacement IDs. Named
transport actions include both receipt lifetime and content revision.

A real **private D-Bus** test passes locally on macOS with a deterministic daemon
fixture: submission fields, same-ID replacement, native close calls, named/default
signals, close reasons, foreign-sender filtering, daemon disappearance/replacement,
reused IDs in a new owner namespace, unsupported features and invalid returned IDs.
It also sends 256 unrelated action signals before a Notify reply. The independent
event stream drains those bounded subscription queues while the call waits, so a
full zbus signal queue cannot block reading its own reply. No OCaml callback or GUI
is involved in this test. It does **not** prove a Linux notification was displayed.

```sh
GPUIO_JOBS=2 scripts/test_notification_bus.sh
```

This headless check is added to required Linux CI for the later consolidated run.
The local run used the ignored scratch-installed `dbus-run-session` and explicit
scratch daemon path with `GPUIO_PRIVATE_BUS_TEST=1`; it did not use or change a
user's session bus. The native Linux worker, admission/resource cleanup, App wiring
and real desktop action evidence remain outstanding. The public Linux API still
returns `Unsupported` until that integration is complete.
