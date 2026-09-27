# Desktop application services

OCH-27 / OCH-28 implementation design, started 2026-09-27. This document
distinguishes proposed contracts from working integration. The pure `Deep_link`
parser and runtime readiness inbox pass local expect tests; native services are
not yet advertised.

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
clears the queue. Pop removes the entry before executing application code. Native
pre-configuration admission and observable overflow reporting remain to implement.

Readiness is separate from the first window opening. A ready application can have
zero windows. Quit cancels delivery and releases pending requests. Any command
targeting a window uses the existing generation-checked handle; routing does not
capture a reusable integer slot as a permanent destination. Reuse existing
close/quit/reopen decisions rather than adding another unsaved-work mechanism.

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

## Packaging and validation plan

macOS packages declare identity and handled schemes in `Info.plist` using
`CFBundleURLTypes` / `CFBundleURLSchemes`; see the
[Apple property-list reference](https://developer.apple.com/library/archive/documentation/General/Reference/InfoPlistKeyReference/Articles/CoreFoundationKeys.html).
Declaring support is separate from requesting default-handler reassignment.
Do not automatically take over common schemes. Tests use a dedicated unique
example scheme and a disposable bundle.

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
