# Native file dialogs — OCH-11 implementation design

Status, 2026-09-13: pure `Gpuio.File_path`, `Gpuio.File_dialog.Open` and
`Gpuio.File_dialog.Save` configuration models and corresponding Rust data are
implemented locally. The macOS Rust panel adapter now also passes actual native
open/save/cancellation checks. Bridge request/result routing, runtime window-close
integration, capability queries and the Linux adapter are still pending. These
checkpoints do not complete the OCH-11 file-dialog requirement.

## Path and configuration contracts

`File_path.t` represents absolute Unix path bytes, including non-UTF-8 filenames.
It rejects empty/relative paths, NUL and paths larger than 16,384 bytes. It does
not normalize separators/dot components, resolve symlinks/aliases, check existence
or grant filesystem authority. `of_string` validates and `to_string` preserves
the original bytes. Applications perform subsequent filesystem I/O with Eio and
an appropriate filesystem capability; path bytes are not automatically suitable
for text labels. Drag/drop should reuse this same path type.

Rust serialization writes the OCaml string representation: a Nat0 byte count
followed by raw bytes. A generic Rust `Vec<u8>` has a different bin_prot encoding
and must not be used as a substitute. Models on both sides test the exact
non-UTF-8 sample `/tmp/\xff.txt` and its encoded bytes. Incoming request/event
decoding must enforce the same bounds before allocation/admission.

`File_dialog.Open.create` defaults to single-file selection, title `Open` and
accept label `Open`. It also supports directories, mixed files/directories,
multiple selection and an optional initial directory. Mixed selection is a
platform capability; unsupported requests must fail explicitly rather than
silently choosing one kind. Titles/accept labels are nonblank UTF-8 without NUL,
at most 4096 bytes, using Core's whitespace convention.

`File_dialog.Save.create` requires an initial directory and a suggested filename;
title/accept label default to `Save`. The suggestion is a single UTF-8 filename
of 1..255 bytes, without slash/NUL and other than `.` or `..`. The filename
returned by the OS must be preserved exactly, including its extension. Initial
directories are navigation hints; no constructor performs filesystem I/O.

The selected result is all-or-error: reject an empty success, too many paths,
an invalid path or an oversized aggregate; never silently drop/truncate paths.
Save and single-selection open require exactly one path. Multiple selection is
bounded to 128 paths and 262,144 aggregate path bytes. Cancellation is distinct
from a native error. Selecting a destination and native overwrite confirmation
do not themselves create, reserve or write the file.

## Runtime direction to implement

Use correlated window-owned requests, delivered through Bonsai effects on the
OCaml UI domain. Rust owns native presentation and receives no OCaml callback
objects. Permit at most one pending picker per window, with the existing bounded
window count providing an application limit. Return an explicit Busy result for
overlap. A result belongs to its exact window generation/request identity;
closing a window must not target a replacement or deliver a stale selection.

Window closure/application shutdown must cancel the native interaction, not just
drop the callback or result receiver. Native cancellation ownership and race tests
are required before claiming that behavior. No new public programmatic-cancel
API is promised by the configuration models. Any such API needs an exact request
handle so a delayed cancellation cannot close a newer picker.

Preserve typed Unsupported/Busy/Closed/NotReady/NativeFailure/LimitExceeded results
and validate at both bridge boundaries. Native resources and pending callbacks
must be bounded and cleared after every terminal path. A capability query must
describe mixed selection, parenting and cancellation truthfully; availability of
a Linux portal is a runtime property, not proven by compilation.

## Findings from pinned sources

Pinned GPUI: `a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b`.

- `crates/gpui/src/app.rs` exposes `prompt_for_paths` and
  `prompt_for_new_path`, returning result receivers without native cancel handles.
  Dropping a receiver does not establish that the dialog was dismissed.
- `crates/gpui_macos/src/platform.rs` uses detached tasks and modeless
  `beginWithCompletionHandler`. Its save-path workaround can rewrite some
  extensions on macOS 15 or later. A small AppKit adapter owning an open/save
  panel now implements exact result paths, explicit parent attachment and
  physical cancellation. The locally pinned objc2-app-kit 0.3.2
  declarations expose sheet presentation, `cancel:` and `orderOut:`. Local native
  behavior and callback/drop evidence is recorded below; runtime integration
  remains pending.
- `crates/gpui_linux/src/linux/platform.rs` uses ashpd and explicitly reports
  mixed file/directory selection as unavailable. Its result handling treats any
  portal Response error as cancellation and filters unconvertible URIs out of
  results. Our all-or-error contract must not inherit that filtering silently.
- Pinned ashpd 0.13.13 exposes `Request::close`, but `Proxy::request` waits for
  `prepare_response` before returning the request. The high-level file chooser
  therefore does not provide a straightforward live handle for cancellation.
  Investigate an owned portal request through zbus before choosing the Linux
  adapter. Parent identifiers for X11/Wayland and request closure before/after
  the portal method reply need explicit lifetime handling; no implementation or
  Linux GUI success is claimed here.

The macOS adapter uses the already-pinned objc2 0.6.4, objc2-foundation 0.3.2,
objc2-app-kit 0.3.2, block2 0.6.2 and raw-window-handle 0.6.2 dependencies.
AppKit/block2 are now direct native dependencies, and the formerly test-only
Objective-C/raw-handle dependencies are enabled for production macOS. No package
version, opam switch or vendor/fork change was required. The ashpd source archive
was downloaded into ignored scratch and its SHA-256 matches Cargo.lock. It is
research evidence, not a build input.

## Implemented macOS panel ownership

`rust/native/src/file_dialog.rs` presents a sheet on the exact GPUI native window.
`Panel` owns its native panel and completion; the Objective-C block holds only a
weak reference back to that state. Completion claims the callback before calling
AppKit or the caller, so reentrancy/repeated cancellation cannot deliver it twice.
An already-attached sheet produces Busy. Explicit native cancellation closes the
panel and returns Cancelled; dropping an outstanding owner physically closes it
and returns Closed. This is the native ownership primitive; the application
runtime still needs to drop that owner on window close/shutdown.

Open results reject invalid/unconvertible paths as a whole, with incremental
aggregate-byte validation. Save returns the native URL path unchanged. Local
native tests select the repository LICENSE file and `/tmp`, accept a save path
ending in `.sql.s`, and verify no destination file was created. They also cover
native Cancel, cancellation/drop during presentation, Busy and reference-cycle
disposal. See [native evidence](../evidence/native-file-dialogs-och11.md).

## Required acceptance work

Add bounded independent request/result fixtures and truncation/invalid-input
tests when introducing wire messages. Exercise correlation, Busy, unsupported
capabilities, cancellation versus failure, close/remount races and late replies.
On macOS, verify actual open/save dialogs, selection/cancel/parent closure and
exact filenames without writing a selected save destination. Add a public
Bonsai/Eio example demonstrating selection followed by explicit Eio I/O. Linux
build/unit tests remain required; full portal GUI validation belongs to the
deferred Linux GUI gate, with limitations recorded explicitly.
