# Desktop application services

OCH-27 / OCH-28 implementation design, started 2026-09-27. This document
distinguishes proposed contracts from working integration. The pure `Deep_link`
parser and runtime readiness inbox pass local expect tests. Identity, a correlated
desktop protocol, native early-link capture and application activation are now
wired, with native state/mailbox unit coverage. Public Eio routing now passes
deterministic delivery tests and real packaged macOS cold/warm OS invocation,
including native window closure/reopening. Document-window metadata now passes
native and public macOS tests. Remaining OS service adapters and Linux
incoming-link forwarding are pending. The full desktop bridge capability
is not yet advertised.

## Ownership and delivery

Application identity and accepted URL schemes belong to the application, not a
window. Register native handlers before entering the platform event loop. Queue
incoming events in bounded application-owned storage until the OCaml application
explicitly accepts delivery. Routing then runs on the OCaml UI domain through
queued effects. It may choose an existing window or open a new one; the native
callback never invokes OCaml synchronously or interprets a URL as a command.

The implemented runtime inbox retains at most 64 entries and 256 KiB of raw
input. Each URL is at most 16 KiB. Overflow rejects the newest input without
reordering retained entries; the adapter must report admission failure. Readiness
is monotonic, repeated identical requests remain distinct, and closing permanently
clears the queue. Pop removes the entry before executing application code.

Native capture now uses the same bounds before configuration, with a saturating
dropped-link counter. Its callback emits one coalesced `Desktop_pending` control
event on transition to pending input; this event is independent of window-input
capacity. `Take_links` atomically drains accepted links and the overflow count
through a reserved command response. Its output is charged to the transport byte
limit. Early signals survive until a UI-domain handler is installed. Both queues
are bounded; the public routing adapter controls transfer/readiness without
polling while idle. Equal links are separate requests, not a deduplication key.

`App.run ~desktop:identity` queues one immutable identity declaration before user
initialization can queue native windows. Desktop requests may be queued during
initialization and wait for the normal bridge handshake. Duplicate native
configuration is rejected. At most 16 desktop requests may be pending; final
shutdown completes them with `Closed` and releases callback captures. The expert
request/subscription interface is runtime machinery. Applications use
`Gpuio_eio.Desktop.attach`, then call `ready` explicitly after preparing their
model. A second live receiver returns `Busy`. Each callback's effect must finish
before another event is delivered; a scheduler yield separates callbacks. Slow
handlers backpressure intake rather than accumulating application effects.
Accepted links, rejected links and native overflow are distinct typed events.
Transient intake failure is reported; `retry` is explicit, without an idle timer.

`Desktop.close` releases queued data and callback captures, unregisters the
receiver and invalidates pending jobs. An already-running application effect may
finish, but cannot restart delivery. A replacement receiver has a fresh token;
old queued availability callbacks and unregister functions cannot affect it.
An in-flight batch belonging to the retired receiver is discarded, not replayed
to its replacement. Shutdown closes the receiver through the application scope.
The native inbox and OCaml delivery inbox are each bounded independently; a
handler can hold one admitted batch while native capture fills the next.

Identity uses a validated lowercase reverse-DNS identifier (128 bytes maximum),
a UTF-8 display name (256 bytes maximum, no ASCII controls), and up to 16 unique
normalized schemes. Declaring identity configures process state; it does not
rewrite bundle metadata or install default handlers. `Requested` means the
platform accepted an operation request; it does not certify visible presentation.

Readiness is separate from the first window opening. A ready application can have
zero windows. Quit cancels delivery and releases pending requests. Any command
targeting a window uses the existing generation-checked handle; routing does not
capture a reusable integer slot as a permanent destination. Reuse existing
close/quit/reopen decisions rather than adding another unsaved-work mechanism.

## Document windows

`Gpuio.Window.Document.create ?path ~edited ()` describes represented-file and
edited metadata. `Gpuio_eio.Desktop.set_document window document` uses the existing
generation-checked window command channel, returning an observed window snapshot.
`Window.Document.of_snapshot` reads the native document state when available.
Passing no path clears representation and supports an untitled document. The
existing `Set_edited` command changes edited state without clearing its path.

macOS uses `NSWindow.setRepresentedURL` with a filesystem-byte `NSURL`, followed
by `setDocumentEdited`; it reads `representedURL` and `isDocumentEdited` for the
observation. Conversion happens before either mutation, and non-UTF8 path bytes
are preserved. AppKit can normalize a path; observations describe its native
value. These operations do not require the represented file to exist and do not
read/write/save it. They also do not install a close decision: applications retain
their existing asynchronous close/quit handlers and unsaved-work model.

Pinned Linux X11/Wayland lack these native document metadata operations. Both
`Set_document` and `Set_edited` return typed `Window.Error.Unsupported`, rather
than treating GPUI's default no-op as success. Snapshot document state is `None`
there; desktop capabilities report `document_metadata=false`. A missing native
observation is also represented by `None`, not invented application state.

Wire commands append `Set_document` (tag 7) and window errors append `Unsupported`
(tag 5); window snapshots now carry optional document metadata. Paired independent
OCaml/Rust fixtures cover raw bytes, clearing and unsupported responses. Native
mailbox accounting includes represented-path bytes before splitting response
batches under the transport limit. This is part of the unreleased M6 bridge schema;
applications and the native library must be rebuilt together.

## Link parsing

The incoming link profile is `scheme://route/path?query#fragment`. Schemes are
case-insensitive ASCII identifiers, at most 64 bytes. Links are at most 16 KiB.
The route is an ASCII unreserved identifier; credentials, ports and network
addresses are outside this application-routing profile. Path/query/fragment
preserve escapes, case and order. Unicode is percent encoded. No automatic
percent decoding, path normalization, file access or external URL opening occurs.
Applications validate route-specific values before acting. This is a bounded
application-link parser, not a general browser URL implementation. Character
classes and escapes follow [RFC 3986](https://www.rfc-editor.org/rfc/rfc3986).

Incoming native file-open events need separate file-path handling; they must not
be forced through this custom-link profile. `File_path` preserves native bytes
and is not an Eio filesystem capability.

## Pinned backend findings

Inspected Zed/GPUI `a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b` locally.

| Operation | macOS | Linux in this pin |
| -- | -- | -- |
| Incoming URLs | Platform callback before run | Callback storage exists; no invocation found in gpui_linux; app launch/instance forwarding needs an explicit adapter |
| Runtime scheme registration | Requires installed bundle; changes default application through NSWorkspace | Returns an unimplemented error |
| Application activation | Implemented | Logs and does nothing; report unsupported rather than success |
| Window activation | Existing native window command | Existing X11/Wayland window command, subject to compositor policy |
| File reveal/open | NSWorkspace; existing helpers discard outcomes and reveal loses non-UTF8 path bytes | Portal/xdg-open paths; existing helpers log errors without returning them |
| Represented document path | Native window implementation | Default trait method does nothing |
| Notifications | Native platform implementation; public GPUI API returns no delivery result | Native platform implementation; public GPUI API returns no delivery result |

These are integration findings, not OS acceptance evidence. Capability snapshots
must separate unsupported operations, unavailable services, and accepted
best-effort requests. Do not wrap a void/no-op GPUI call and report confirmed
delivery. Native workers handle potentially blocking OS calls; ordinary
application file/network work stays in Eio. Permission and service failures need
typed outcomes, with submission distinct from user-visible presentation.

Current native capability snapshots enable incoming links, application activation
and document metadata only on macOS. Registration and file reveal/open
remain false until their adapters are implemented and validated; this is an
implementation checkpoint, not the intended OCH-27 endpoint.

## Packaging and validation plan

macOS packages declare identity and handled schemes in `Info.plist` using
`CFBundleURLTypes` / `CFBundleURLSchemes`; see the
[Apple property-list reference](https://developer.apple.com/library/archive/documentation/General/Reference/InfoPlistKeyReference/Articles/CoreFoundationKeys.html).
Declaring support is separate from requesting default-handler reassignment.
Do not automatically take over common schemes. Tests use a dedicated
example scheme and a disposable bundle.

`examples/desktop` demonstrates the public receiver. The local macOS test creates
a temporary `.app` declaring `gpuio-desktop-lab`, launches a cold URL through
`open`, and sends subsequent URLs to that exact bundle. It does not request a
default-handler reassignment. It verifies readiness ordering, malformed authority
rejection, once-only delivery, receiver replacement, and same-process routing.
Real AppKit accessibility checks observe one/zero/one native windows and changed
document text. The app launches in the background; the test briefly activates
only that process to expose AppKit's accessibility tree. Paths are canonicalized
for owned-process cleanup (`/var` resolves through `/private/var` on macOS).

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/desktop/main.exe
python3 scripts/test_desktop_links_macos.py
```

This is link-delivery acceptance, not acceptance of the entire desktop ticket.
It also checks public metadata setting/clearing and rejection of an old window
handle after another window opens. The native window suite additionally checks
non-UTF8 paths and actual AppKit document state. It does not validate Linux forwarding, file services,
runtime default-handler reassignment, or notification delivery.

Linux packages need a desktop entry with application identity, an executable
argument vector accepting URLs, and scheme MIME declarations. Follow the
[Desktop Entry specification](https://specifications.freedesktop.org/desktop-entry/latest-single/)
for argument escaping and field codes. Already-running delivery needs explicit
single-instance forwarding; an executable argument alone does not provide it.
Registration/install helpers must expose their side effects and avoid shell
evaluation of URLs or native paths.

Acceptance remains: deterministic parser/readiness/stale-window/codec tests;
packaged macOS cold and warm OS URL invocation; document metadata, activation,
file reveal/open and lifecycle checks; Linux compilation/unit tests plus accurately
reported graphical evidence under OCH-17. Notification action tests follow the
same ready/closed-window contract. OCH-29 supplies a public graphics consumer and
the capability-to-test matrix. OCH-40 adds seven chart families independently of
the custom-canvas and external-component acceptance requirements.
