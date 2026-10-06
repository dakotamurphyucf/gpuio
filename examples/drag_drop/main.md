# Typed native drag events and application result state

[main.ml](main.ml) has a Greeting source, Text/Files target and an explicit
keyboard/click alternative. Read component states/configs and callback matches, then
launcher/test modes. [README](README.md), [dune](dune),
[development](../../docs/development.md) and [platform
policy](../../docs/platform-release-policy.md) cover exact commands,
Core/GPUIO/Bonsai/Eio PPX and desktop limits. No assets or services are needed.

`B.state` allocates result instruction, source Ready and a hover flag initially false
once in the persistent graph. Reactive values change over graph lifetime; `let%arr`
reads ordinary values to derive config/views, with and listing dependencies, not
tasks. Setters return deferred effects. `Edge.on_change` records result/diagnostic
phase through typed equality and thunks; these refs support tests, not duplicate
native ownership. External phase starts 0, phase 1 disables source/target, phase 2
removes the keyed row. The result/alternative button remain.

`Drag.Payload.text` validates Hello from OCaml; Source/Target validate labels and
nonempty format allowlist. Native Rust decides acceptance synchronously from current
target config; OCaml callback cannot retroactively reject a drop. Payload is
snapshotted when gesture starts. Stable keys greeting/inbox identify native handlers;
removal/replacement suppresses later callbacks. Text/custom/file data are bounded;
paths are native bytes, not filesystem capabilities or implicit reads. See
[Drag_and_drop](../../lib/core/drag_and_drop.mli).

region gives 250 × 100 logical-pixel colored areas. Source callback returns `E.Many`,
combining diagnostic observer thunk and status effect for Started/Desktop offer/
Ended outcomes. Target callback similarly records observation and updates hover on
Entered/Left, ignores Moved, and on Dropped clears hover plus formats payload. Files
report count and explicitly open nothing. Custom formatting is exhaustive though
current target accepts only Text/Files. Rejected displays typed reason. The
alternative button calls same payload formatter/setter without claiming a drag
occurred; this provides keyboard access to the demonstrated action.

Drag greeting into target: native pointer handling creates Gesture_id, snapshots
payload, emits Started and target Entered/Moved/Dropped followed by source Ended
Internal_drop. Event effects arrive asynchronously on OCaml UI domain; target sets
result, Bonsai derives text/color, GPUIO submits native update. Moved samples may
coalesce; lifecycle edges preserve order. Unconfirmed means no acknowledged app drop,
not proof of an external copy/move. Native preview/input stays Rust. Admission/render
callbacks differ from physical screen presentation.

`App.run` owns GPUI OS thread and OCaml Eio UI domain, opening a 640 × 380 window.
Ordinary launch starts no application file/network/timer task. `--self-test` starts
an app-scoped Eio task, 15-second timeout and 5 ms phase waits, checks increasing
native render revisions for 0→1→0→2 and force-closes. Success prints
GPUIO_DRAG_DROP_PUBLIC_OK explicitly noting no injected gestures.

`--gesture-self-test` instead waits up to 30 seconds for a real external driver. It
records typed source/target events (ignoring target Moved telemetry), requires one
matching Started/Dropped/Ended Internal_drop gesture/payload and target Entered,
awaits result model and native render, closes and prints GPUIO_DRAG_DROP_GESTURE_OK.
It does not inject a gesture by itself. Use the macOS harness in README; do not
combine both diagnostic flags, which start independent tasks against the same window.
Test error assertions are diagnostic policy. Scope cancellation suppresses queued
completions; [App](../../lib/eio/app.mli) and [Scope](../../lib/eio/scope.mli)
specify cleanup. Neither mode alone proves VoiceOver, physical display or Linux
desktop acceptance.

To accept a namespaced Custom payload, validate kind, include that format in the
target and decode bounded bytes under application policy after native acceptance. Do
not copy filesystem metadata into synchronous UI lookup or treat received paths as
granted access. For file processing pass Eio capabilities into scoped work, handle
errors/cancellation explicitly and keep I/O outside `let%arr`.