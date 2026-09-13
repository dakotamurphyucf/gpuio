# Linux file-chooser portal — OCH-11

Status, 2026-09-13: `gpuio-portal` implements the D-Bus request layer and is now
connected to the native manager for X11 and Wayland. The worker ownership adapter
is compiled and unit-tested on macOS. The Wayland export adapter compiles locally,
but its system-libwayland protocol tests await the Linux gate. Public capabilities
and Linux build validation remain required; this is not Linux GUI acceptance or
completion of file dialogs.

## Ownership and request flow

Each pending picker gets a dedicated session-bus connection. This gives it its
own request namespace, signal queue and disconnect cleanup. The native manager
preserves one pending picker per window; the host retains its existing 32-window
limit. The portal crate does not independently impose a global application limit.
Connections exist only for probes or pending native interactions. This costs
more connections/executor resources than sharing one, but avoids cross-request
cleanup and signal-buffer ownership during infrequent user-driven dialogs.

`version()` probes availability without showing a dialog. `choose(config,
parent, token, cancel)` runs the request. Discovery activates the well-known
portal name, resolves its unique bus owner and rereads the interface version
from that owner. Methods and response filtering stay pinned to the same owner;
a restart cannot combine an old service's advertised version with a new one.

Before OpenFile/SaveFile, the client subscribes to Response signals for its own
connection's request namespace. It buffers a response that arrives before the
method reply, then matches the exact returned object path. Alternate handles
within the owned namespace are supported; foreign namespaces are rejected and
never closed. NameOwnerChanged is subscribed before the method call, so portal
service death cannot leave the client waiting on an otherwise live bus connection.

Sending cancellation, or dropping the sole cancellation sender, starts cleanup:

1. Before any native request, return Closed without presenting anything.
2. While the method reply is pending, Close the predicted handle immediately.
   Continue polling the method; close its actual handle if the first handle was
   not yet created or the portal returned an alternate path.
3. After the reply, Close that exact handle. Do not wait for a Response after
   Close; the portal does not promise one.
4. Acknowledged Close or UnknownObject for an already-returned handle yields
   Closed. Unacknowledged cleanup failure yields NativeFailure. Close the
   dedicated bus connection on every outcome.

Setup and individual method calls have five-second bounds. Waiting for a user's
selection has no arbitrary timeout. The native owner must keep polling the
worker through cleanup; dropping a Rust future alone is not physical cancellation.
The OCaml runner already maps results arriving for a closing window to Closed;
the native integration must retain cleanup-failure evidence rather than treating
that public lifecycle result as proof that the portal acknowledged dismissal.

The behavior follows the [portal Request contract](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.Request.html).
The [FileChooser interface](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.FileChooser.html)
defines versioned directory selection, byte-array folder hints and URI results.

## Data contracts

- OpenFile supports files, multiple files, and directories from interface version
  3 onward. Mixed files/directories returns Unsupported explicitly.
- SaveFile sends the suggested filename unchanged. Folder hints are native bytes
  with the required terminal NUL; returned paths contain no NUL.
- Responses are all-or-error. User cancellation is distinct from the portal's
  other failure response. Missing/empty selections, malformed values, remote
  authorities and any invalid URI reject the entire success.
- Local file URIs preserve percent-decoded Unix bytes, including non-UTF-8 names,
  repeated separators, dot segments and extensions. There is no URL/path
  normalization, silent filtering, truncation or save-path rewrite.
- The D-Bus body is bounded before container deserialization. Path conversion
  enforces 128 paths, 16,384 bytes per path and 262,144 total path bytes. The
  original open/save cardinality is checked again. URI decoding does not open,
  create, reserve or write the selected files.

## Native ownership and shutdown

The native adapter copies the exact requesting X11 window ID on the GPUI thread.
The background worker receives owned configuration, parent text or an explicitly
retained Wayland export, correlation, window generation and cancellation channels.
It does not access GPUI window objects. Each request remains tracked as Running,
Delivering or Cancelled until
its final response/wake operation has finished. Closing a window while a worker
is already delivering a response still waits for that worker; it cannot dispose
the runtime wake pipe underneath delivery.

Close and application shutdown signal every affected running request before
waiting on cleanup. They emit Closed once, suppress late selections and await
worker completion before native resource disposal. Completion is broadcast by
closing a channel, so repeated cleanup observers cannot steal each other's
completion. A failed portal cleanup is logged even if the public lifecycle result
has already become Closed. Dropping a manager clone leaves requests alive;
dropping the last owner initiates cancellation. Workers do not retain that owner.

Ordinary close, OCaml shutdown and abort await cleanup asynchronously. Unconditional
GPUI quit needs a different hook: at the pinned GPUI revision, `App::shutdown`
invokes quit observers, then clears windows, then polls their returned futures
with a 200 ms timeout. Returning a cleanup future would therefore be too late to
retain native parents. Our quit observer drains background cleanup synchronously
while constructing its ready future, before GPUI clears windows. This can delay
quit while a portal acknowledges cancellation. Cleanup must never depend on a
foreground GPUI callback. No GPUI patch is used.

## Validation and remaining integration

Fourteen focused tests pass on local macOS using actual D-Bus messages over
private, preauthenticated Unix socket pairs. They verify early/alternate replies,
exact options, cancellation before handle creation, owner disposal without a
Response, timeout cleanup, foreign-handle rejection, socket disconnect, exact
service-owner loss filtering, pinned-version discovery, missing portals, explicit
Close failure, URI/path limits and save/mode policy. They exercise our protocol
implementation; they do not validate zbus SASL, a session-bus daemon, a real portal
backend or a Linux desktop.

Pinned zbus 5.19.0 was already in Cargo.lock. The new crate reuses existing locked
packages; no existing dependency version or OCaml switch changed. The zbus
`bus-impl` feature is a dev dependency used to assign bus-like unique names to
private test peers. Production uses session-bus-assigned identities.

Next implementation requirements:

- Run the Wayland guest-queue tests with system libwayland on Linux. Confirm exact
  surface identity, registry/export disposal and cancellation at the protocol
  boundary; compilation on macOS does not prove those runtime properties.
- Provide truthful public capabilities, including portal availability/version,
  mixed selection and parenting. Runtime unavailability is not a build-time fact.
- Validate X11 and Wayland builds/unit behavior. Actual Linux portal GUI acceptance
  stays under the deferred Linux GUI gate (OCH-17); compilation must not be
  described as GUI acceptance.

The pinned ashpd raw Wayland export helper starts its own thread and requires the
display to outlive the identifier. Its future cannot simply be dropped while GPUI
removes the borrowed surface/display. The pinned GPUI private portal helper also
chooses the keyboard-focused window, not an explicit requested window. Neither
is an acceptable lifetime/parenting shortcut without additional ownership work.

## Wayland export adapter

`gpuio-wayland` uses a guest event queue on GPUI's existing libwayland display.
Pinned `wayland-client` 0.31.15 explicitly supports processing guest
events with `dispatch_pending` while the host reads the socket. The pinned GPUI
Wayland backend uses the system client through `calloop-wayland-source`. This
allows the adapter to issue exports while the native parent is retained, flush
and dispatch only the guest queue, and stop polling on completion/cancellation/
timeout. It never calls prepare_read, blocking_dispatch or roundtrip and starts
no independent reader thread. While an export is pending, a background timer
dispatches at 10 ms intervals, with a five-second deadline and immediate channel
cancellation. Waiting for the actual user picker still has no arbitrary timeout.

One shared `Display` owns the application's guest registry, queue and bound
exporter. A separate `Export` owns each requested surface's exported object.
The native manager caches the shared display until quit, checking display identity
before reusing it. The exact surface is obtained from the requesting GPUI window.
The unsafe Rust boundary documents both lifetimes: the host display outlives all
shared owners, and the host surface outlives its export. The host cleanup barrier
enforces this; after draining workers, the quit hook explicitly releases the
cached display before GPUI can dispose its native resources.

The first registry sync selects xdg-foreign v2 when advertised, with v1 fallback.
Missing protocols return Unsupported; empty/oversized/NUL handles return
NativeFailure. Protocol support is a connection-lifetime snapshot, not a guarantee
that a later request succeeds. Once the exporter is bound, the adapter destroys
the client registry proxy so idle global notifications cannot accumulate. The
server registry resource remains until disconnect, as Wayland specifies; sharing
one registry avoids allocating another server resource for every dialog.

Dropping an export sends its destructor even before its Handle event arrives.
The shared exporter is destroyed only after the last display/export owner drops.
An explicit native request owner disposes its parent before publishing worker
completion, including an async task dropped before its first poll; this ordering
does not depend on anonymous future capture order. Guest cleanup requires no
foreground callback. These are implemented ownership contracts; their new
system-libwayland tests still require execution on Linux.

Exact commands and platform limits are recorded in the
[portal protocol evidence report](../evidence/linux-file-portal-och11.md).
