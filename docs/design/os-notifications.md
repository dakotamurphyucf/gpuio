# OS notifications

OCH-28 is in progress. The typed values, bounded lifetime state and UI-domain
delivery layer are implemented and being validated. Native adapters and public
Eio operations are not delivered yet; no notification capability is advertised.
These notifications are independent of in-application toast widgets.

## Identity and updates

An opaque, process-local receipt identifies one logical notification. Its tag is
application-supplied; posting a duplicate live tag returns `Busy`. Replacement
retains the receipt and tag. Posting the tag again after retirement creates a new
receipt. Native identifiers additionally require a process nonce so artifacts
left by a previous application run cannot resolve to a new lifetime.

Named action keys include the content revision. A removed action or a button from
an obsolete revision cannot dispatch against the new content. Default activation
belongs to the logical lifetime: the freedesktop protocol reports a native ID and
`default`, without a content revision. Applications must not treat it as evidence
that a particular content version was displayed or clicked.

The first admitted activation, named action or platform-close event consumes the
lifetime. Duplicate and later signals are ignored. An already-queued event retains
its original receipt even when a new notification reuses its tag. Explicit API
dismissal immediately disables actions; its result reports the removal request
separately from OS close events. Failed removal retains a bounded retired slot
and allows dismissal retry; replacement is then stale.

Events are application-scoped, independent of window lifetimes. There is no raw
window slot in an OS payload. Route through current application state and exact
`App.Window.t` handles; normal window-generation validation applies. The runtime
never chooses or activates a window implicitly.

## Bounds and asynchronous ownership

Content is plain UTF-8 text: title 256 bytes, body 8192, tag 128, action label 128.
Titles, tags and labels are nonblank and exclude ASCII controls. Body permits
newline, carriage return and tab. At most four actions with unique 1–64 byte
ASCII letter/digit/dot/underscore/hyphen IDs. Silent is the default sound.

Native state admits at most 128 combined live lifetimes and queued terminal
events, reserving an event slot per lifetime. Up to 16 pending native lifetime
operations are retained. A separate coalesced service failure allows a maximum
129-event batch. Events wait for application readiness. The UI-domain delivery
layer takes another batch only after all handlers for the previous batch finish,
yielding between callbacks; slow effects backpressure intake. Both native and
OCaml queues are bounded independently. There is no notification polling timer.

Before a submission completes, at most one matching terminal event is held. A
successful acknowledgement commits it; a failed request discards it. A callback
cannot consume another service's token. Shutdown/service loss retires lifetimes;
in-flight tokens remain bounded until completion so a successful late submission
can remove its exact OS artifact. Cleanup must never look up a tag that could now
identify a different lifetime. Duplicate completions must not remove live content.

## Platform implementation direction

The pinned GPUI notification facade returns no typed delivery result. Its Linux
adapter cannot dismiss a notification; its macOS category registry retains
historical action sets. GPUIO will own bounded native adapters rather than promise
behavior those wrappers do not implement. No GPUI fork change is required by this
design. Do not simultaneously install GPUI's notification delegate.

macOS uses `UNUserNotificationCenter`, guarded by a matching application bundle
identity. Permission probe/request are explicit; initialization must not prompt.
Linux uses the freedesktop notification service and its advertised capabilities.
Unavailable service, permission denial, unsupported features and native failure
remain typed outcomes suitable for application-supplied fallback. Submission is
not evidence of visible presentation, and OS/desktop policy controls presentation.

The freedesktop protocol specifies same-ID atomic replacement and fresh IDs that
are not reused before exhausting the ID range. The adapter must also fence server
owner changes. See the [protocol](https://specifications.freedesktop.org/notification/latest/protocol.html)
and [basic design](https://specifications.freedesktop.org/notification/latest/basic-design.html).
Apple permits using the shared notification center from multiple threads, while
application callbacks still belong on GPUIO's UI domain. See Apple's
[notification center](https://developer.apple.com/documentation/usernotifications/unusernotificationcenter)
and [authorization error](https://developer.apple.com/documentation/usernotifications/unerror/notificationsnotallowed).

Actual native submission/action evidence, permission observations, daemon-loss
coverage and packaged example instructions will be recorded separately from the
pure lifecycle/codec tests as the adapters are implemented.
