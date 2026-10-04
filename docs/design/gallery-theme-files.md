# File-backed gallery themes

OCH-41 implementation contract, 2026-10-04. This is an application example using
public Core/Bonsai/Eio APIs, not a new native theme registry or an upstream JSON
compatibility format.

A version-1 S-expression profile supplies a name, Light/Dark appearance and ten
concrete hex colors: background, surface, foreground, muted, accent, border,
on_solid, success, warning and danger. Use the existing strict Color_value hex
parser; reject malformed/duplicate/missing/unknown fields, invalid UTF-8/NUL,
unsupported versions and profiles larger than 16 KiB. Names are nonblank and at
most 128 UTF-8 bytes. Bound parsed S-expressions to 16 levels before typed record
decoding; input bytes are bounded before parsing.

The Styles page offers Choose theme and Reload file. The selected file is read
with Eio in a scoped task with a 16 KiB bound; parsing produces an immutable,
validated model. Only a successful current result updates the window's palette.
Errors and picker cancellation keep the last good theme. Each window owns its
selection; loading a file does not reset navigation, editor identity or scale.
Both the explicit Light/Dark control and Follow system leave the file theme.
A pending file result must not override such a newer user choice.

Use one load per active preview and a child window scope. Page departure/window
closure cancel work and suppress its queued results; reactivation gets a fresh
scope. Request completion also checks an opaque selection token, so a new user
choice invalidates already queued file results. File I/O never runs in paint,
layout or a synchronous native callback. Preserve Eio cancellation as cancellation.

Reload is explicit; this example does not install a directory watcher or polling
timer. The chosen application profile controls palette/presentation colors and
document light/dark mode; it does not import arbitrary upstream metric, syntax or
motion settings. A filesystem watcher could publish validated profiles through
the same scoped path, but unimplemented watching is not claimed as covered.

Acceptance: parser and boundary expect tests, bounded Eio read/error/cancellation
tests, last-good/stale-choice model tests, palette/reconciliation retention checks,
a runnable public gallery with a checked-in sample profile, and a separately
recorded physical picker/reload/editor-retention walkthrough.
