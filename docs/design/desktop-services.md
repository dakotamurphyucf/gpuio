# Desktop application services

OCH-27 / OCH-28 implementation design, started 2026-09-27. This document
distinguishes proposed contracts from working integration. The pure `Deep_link`
parser and runtime readiness inbox pass local expect tests. Identity, a correlated
desktop protocol, native early-link capture and application activation are now
wired, with native state/mailbox unit coverage. Public Eio routing now passes
deterministic delivery tests and real packaged macOS cold/warm OS invocation,
including native window closure/reopening. Document-window metadata now passes
native and public macOS tests. macOS file open/reveal and explicit scheme
registration also have public OS acceptance. Linux portal file services are
implemented with local peer/worker tests; actual Linux builds/GUI evidence and
incoming-link forwarding remain pending. The full desktop bridge capability
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

Current native capability snapshots enable incoming links, activation, document
metadata, registration and file reveal/open on macOS. Linux now implements file
open/reveal through the portal. Capability flags describe adapter support;
operation-time discovery can still report an unavailable service or unsupported
portal version. Linux link/registration adapters remain in progress.

## File services and explicit registration

`Gpuio_eio.Desktop.open_file app path` uses a validated `File_path` and macOS
`NSWorkspace.openURL:configuration:completionHandler:`. It waits for the workspace
completion without blocking GPUI or the OCaml UI domain. Success means the OS
accepted opening with the selected application; it does not certify that the
application rendered or consumed a document. It does not add a recent item or
open an application-choice prompt if no handler is available. Known native error
domains/codes map to `Denied` or `Unavailable`, including a bounded walk through
wrapped errors; unknown errors remain `Native_failure`.

`reveal_file app path` submits AppKit's file-viewer selection request. That API
has no completion/error receipt: `Ok ()` promises submission only, not existence,
permissions, selection or foreground presentation. Both operations construct
filesystem-byte URLs without shell interpolation or lossy path conversion.

`register_scheme app scheme` explicitly requests default-handler assignment.
It is never automatic, may prompt the user, and requires the scheme in both the
configured identity and bundle URL declarations. The running bundle identifier
must match the identity. An unbundled/mismatched app returns `Unavailable`; missing
scheme declarations return `Invalid_request`. Successful completion is distinct
from merely declaring a scheme in packaging metadata.

Native asynchronous operations have a 16-entry registry in addition to the Eio
request bound. Each admission has a fresh token. Completions can arrive on any
thread but only publish owned results through the mailbox; they never access
GPUI or call OCaml. Removal precedes callback execution outside the registry lock.
Shutdown completes remaining admissions with `Closed`; duplicate/retired callbacks
cannot finish a replacement correlation. Mailbox terminal checks and response
reservation consumption occur under one lock, so callbacks racing transport
closure cannot publish after `Stopped`. Already-submitted OS work can still take
effect after application shutdown; this API does not claim OS-level cancellation.

### Linux file operations

Linux uses the [XDG OpenURI portal](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.OpenURI.html),
with `OpenFile` requiring version 2 and `OpenDirectory` (reveal) requiring version 3.
The portal may reveal by opening the containing folder rather than selecting an
item. Both operations await its response; success does not certify presentation.
The adapter requests writable access for exported sandboxed documents and uses
the default/last app choice where supported. A portal may still prompt; cancellation
is reported as `Denied`. Missing service returns `Unavailable`, an insufficient
version `Unsupported`, and unclassified live-request failures `Native_failure`.

A maximum of 16 native workers prepare owned descriptors and run portal I/O.
Linux uses `O_PATH` and accepts regular files/directories, preserving raw path
bytes without requiring UTF-8 or shell interpolation. These are application-scoped
requests with an empty native parent; no borrowed GPUI window/display survives in
a worker. Ordinary application persistence remains in Core/Eio.

The existing file-chooser request engine now also serves desktop requests. It
subscribes before issuing methods, pins the discovered service owner, accepts
early responses, validates returned handle namespaces, and closes actual request
handles on cancellation. Discovery/method deadlines remain bounded. Application
shutdown signals workers and awaits their cleanup before stopping GPUI; workers
own all descriptors/connections and release admission slots on every exit path.
During terminal shutdown, pending callers receive `Closed`; an unconfirmed portal
dismissal emits `GPUIO_DESKTOP_CLEANUP_FAILED` rather than claiming success.

Local macOS tests use real private Unix-socket D-Bus messages and descriptor
transfer to exercise this Linux service protocol without a display/session bus.
They check file identity, options, portal outcomes, minimum versions and actual
Request.Close messages. Existing chooser cancellation/restart tests still pass.
Worker tests exercise admission bounds, descriptor failures and shutdown barriers.
This evidence does not claim that a Linux desktop displayed a folder or opened
an application; Linux compilation/unit gates remain required and GUI validation
remains separately tracked under OCH-17.

## Packaging and validation plan

macOS packages declare identity and handled schemes in `Info.plist` using
`CFBundleURLTypes` / `CFBundleURLSchemes`; see the
[Apple property-list reference](https://developer.apple.com/library/archive/documentation/General/Reference/InfoPlistKeyReference/Articles/CoreFoundationKeys.html).
Declaring support is separate from requesting default-handler reassignment.
Do not automatically take over common schemes. Tests use a dedicated
example scheme and a disposable bundle.

`examples/desktop` demonstrates the public receiver. The local macOS test creates
a temporary `.app` declaring `gpuio-desktop-lab`, launches a cold URL through
`open`, and sends subsequent URLs to that exact bundle. The default test does not
request a default-handler reassignment. It verifies readiness ordering, malformed authority
rejection, once-only delivery, receiver replacement, and same-process routing.
Real AppKit accessibility checks observe one/zero/one native windows and changed
document text. The app launches in the background; the test briefly activates
only that process to expose AppKit's accessibility tree. Paths are canonicalized
for owned-process cleanup (`/var` resolves through `/private/var` on macOS).

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/desktop/main.exe
python3 scripts/test_desktop_links_macos.py
python3 scripts/test_desktop_links_macos.py --services
```

This is link-delivery acceptance, not acceptance of the entire desktop ticket.
It also checks public metadata setting/clearing and rejection of an old window
handle after another window opens. The native window suite additionally checks
non-UTF8 paths and actual AppKit document state.

The `--services` variant registers only the private `gpuio-desktop-lab` scheme,
then verifies actual routing without `open -a`. It builds a disposable Objective-C
file consumer with a unique exported content type, verifies that the OS-selected
application receives the intended Unicode/space/metacharacter filename, and checks
that Finder shows the fixture. Missing-file, undeclared and unpackaged-scheme
errors are checked too. It closes its Finder window and unregisters disposable
bundles. Fixtures live under ignored `scratch/desktop-os`: on the tested macOS,
Launch Services did not discover default file handlers under the system temporary
directory. The consumer verifies file identity, allowing OS Unicode normalization.
These macOS GUI tests do not validate Linux desktop presentation or notifications.

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
