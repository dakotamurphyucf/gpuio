# Linux file-chooser portal — OCH-11

Status, 2026-09-13: `gpuio-portal` implements and locally tests the D-Bus request
layer. It is not connected to `gpuio-native` yet. The native Linux file-dialog
manager still returns Unsupported. X11/Wayland parenting, lifecycle integration,
public capability reporting and Linux validation remain required.

## Ownership and request flow

Each pending picker gets a dedicated session-bus connection. This gives it its
own request namespace, signal queue and disconnect cleanup. The eventual native
manager must preserve one pending picker per window and the existing 32-window
limit; the portal crate does not independently impose a global application limit.
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

- Obtain the exact requesting window's parent identifier. Copying an X11 window
  ID is straightforward. Wayland needs an owned exported surface with a proven
  lifetime through presentation/cancellation; borrowed GPUI raw pointers must not
  be carried across an uncontrolled export task.
- Wire the portal worker to native window generations and correlated responses.
  On close/shutdown, cancel all affected workers and await cleanup before native
  window/application disposal. Remove request owners before callbacks; suppress
  late selection after owner cancellation. Cancellation must remain concurrent
  across windows, with no RefCell borrow held across await/native reentry.
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

Exact commands and platform limits are recorded in the
[portal protocol evidence report](../evidence/linux-file-portal-och11.md).
