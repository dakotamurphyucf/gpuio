# One application-owned desktop and notification session

[desktop_session.ml](desktop_session.ml) and [desktop_session.mli](desktop_session.mli)
own the UI-domain services shared across gallery windows. Read identity/content,
Snapshot/create/ready, serial, support/authorization operations and receipt actions.
This is mutable application coordination using a Bonsai Var and deferred effects,
not a per-page component or pure domain model.

From the root after [setup](../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/gallery/main.exe -j 2
./scripts/gpuio exec dune exec examples/gallery/main.exe -- --open-uri=gpuio-studio://preview/startup
python3 scripts/test_gallery.py --section desktop
python3 scripts/test_gallery_desktop_macos.py --artifacts scratch/gallery-desktop
```

The packaged macOS driver exercises OS links/notifications with disposable bundles;
its permissions/cleanup and Linux qualification limits are in
[README](README.md#desktop-integration). No command or permission request was newly
run here. Incoming links are observations, never filesystem authority.

## Identity, fixed content and latest observations

scheme is validated gpuio-studio; identity is com.gpuio.component-studio with name
GPUIO Component Studio. The notification tag is studio-preview. content builds fixed
plaintext title/body including Japanese/emoji and action inspect; updated changes
the title. These validated fixtures use default silent sound and no network data.
Unlike a production capability-adaptive composer, this demo submits the declared
body/action content and reports typed service errors.

The abstract t stores App, desktop receiver, notification service, Snapshot Var and
one receipt reference. Snapshot contains latest support/authorization/link/notice
strings plus busy/has_receipt; there is no unbounded event history. create attaches
both services once. Desktop events overwrite link text for Link/Rejected_link/
Overflow/Failed. They never choose a gallery page, open a path or activate a window.
Notification events form an optional delivered receipt; only exact current receipt
matches retire ownership/update its terminal observation. Failed has no receipt
and updates the notice; this handler does not automatically clear a current receipt
or reconnect the service on that failure.

[Application](application.ml) creates one session before its windows, passes it
through Component/Pages and calls ready after opening the initial window. ready
marks both receivers able to route events; it is not a paint acknowledgement.
snapshot returns B.Expert.Var.value, so all [Desktop pages](desktop_page.md) react
to one source. `B = Bonsai.Cont`; Var.set replaces the latest immutable snapshot,
while returned Bonsai effects defer native requests/mutations until handled.

## One admission lane across every window

serial checks latest busy inside Effect.of_thunk, sets it before starting f and
clears it when f completes. Requests while busy are ignored, not queued/coalesced.
This execution-time check matters when two windows captured enabled buttons from
an older render. It serializes explicit support/authorization/post/replace/dismiss
operations; it is not an event history or a transaction over OS presentation.
There is no independent timeout/retry loop or general exception-finally wrapper
in this helper; normal APIs return typed Result effects.

check sequentially queries Desktop.capabilities, Notification.capabilities and
Notification.authorization, formatting only supported capability names. It never
requests permission. allow explicitly requests authorization and records its result.
post rechecks receipt inside serial; it does nothing if one exists, otherwise
Notification.post returns an opaque lifetime stored on success. Receipt identity
is process-local and must not be serialized or used as a native window slot.
replace uses the current receipt and reports Result; it does not post a replacement
fallback or clear receipt merely because replace failed. dismiss clears only after
success if the captured receipt still matches current, then publishes has_receipt.
retry explicitly calls both intake retry APIs without the serial operation lane.
It does not restore notifications after daemon loss or imply automatic polling.

Trace: window A clicks Post → serial admits and marks shared busy → window B's
later operation is ignored → post reply stores receipt → both pages show replacement/
dismiss controls → matching inspect action records text and clears receipt.
That action does not activate a window. A stale old receipt cannot retire a newer
one, even if their tags are equal. This handler does not contain Signal Studio's
additional busy-event waiter; its source-level contract should not be replaced
with another example's coalescing behavior.

## Services outlive pages and windows

[Desktop](../../lib/eio/desktop.mli) and
[Notification](../../lib/eio/notification.mli) attach application receivers; shutdown
closes subscriptions, queues and owned native notification lifetimes. Page departure
is not service close, and this module has no page deactivation cleanup. App readiness
may persist with no open windows. Posting/replacement/dismissal acceptance is not
proof of a visible banner or confirmed removal; support is distinct from permission.

For a production policy, add explicit behavior for stale/error receipts, capability
fallback and any early event/reply coordination rather than assuming these preview
notices implement a reconnecting notification controller. Keep latest observations
bounded and resolve exact current window handles only if a new action is intended
to activate one. There is no independent pure test for this session; direct and
packaged native harnesses cover different boundaries and were not run here.
