# Public native toast motion — OCH-41

Local checkpoint, 2026-10-03, macOS checkout at base `83eb87e` plus the milestone
working tree. `Toast.Stack.Motion` now connects the Core API, paired Op107,
checked admission, production native lifecycle/reflow and Feedback gallery.
This supersedes the integration gap in the earlier
[renderer checkpoint](toast-motion-renderer-och41.md). Physical macOS and release
qualification remain open; no OS windows were opened for these checks.

## Delivered behavior

`Toast.Stack.create ~motion:Toast.Stack.Motion.default` enables native entry/exit
and reflow independently of layering. Checked configuration supplies a spring,
entry/exit durations and slide distance. Defaults are 400/40/1 with epsilon0.01
and a two-second spring cap, 400ms entry, 200ms exit and 96px slide. Durations allow
zero through 60 seconds and round up to milliseconds; offset is finite 0..16384px.
The existing immediate behavior remains when Motion is absent.

Entry starts after actual child paint. Active-time expiry waits for entry to
finish. Native close/Escape/timeout/overflow immediately retires input and restores
saved focus; a single accepted dismissal is published after finite exit. Exit
starts at the last painted opacity/slide, including interrupted entry. Changing
an already-expiring toast to persistent cannot undo its accepted terminal reason.
Metadata changes never replay entry or reopen a closed key. Application removal,
window close and overload discard pending events and release tasks. Reduced motion,
inactive/hidden/disabled/zero-area owners and removing Motion settle phases.

The persistent input-retirement gate crosses modal portals. Exiting ordinary
content can paint, while descendant editors/actions cannot reactivate on a later
frame; independent popup surfaces disappear and lose traps. Native composition
handles the first Escape; the subsequent Escape can dismiss. The tested composition
path uses GPUI's TestPlatform and does not establish physical OS IME acceptance.

Measured reflow samples width before text layout and composes current natural
height with the anchored edge. It preserves painted velocity and independent axis
progress during streaming. Enabling Motion without layering retains the old
oldest-first column order and 8px gap, while adding a named Group entry for bounded
keyboard scrolling. Layering retains its newest-at-anchor expansion contract.
All child editors remain owned by the mounted application tree.

The Host uses finite native executor deadlines, weak owners and opaque phase/token
identity. Child paint only commits native samples; the deferred frame finish
schedules deadlines and publishes terminal observations. No synchronous OCaml
frame callback or permanent polling loop is added. Admission charges Motion's
fixed record plus 1024 stack quota bytes and 4096 per submitted child, separately
from existing layering/content accounting. Reset releases trajectories during
acceptance; removing children releases geometry before another draw. These quota
units are not measurements of allocator/process RSS.

Feedback adds **Animate notifications**, independent of **Layer notification
cards**. Existing cards retain their phase when enabled; save a new notification
or restore dismissed sample cards to see entry. Dismissal animates before the
application removes its key. Existing placement, margin and layering controls
continue to use the public API.

## Verification

Use `GPUIO_JOBS=2 ./scripts/gpuio exec` for these pinned/offline commands:

```sh
cargo test --offline --locked -j2 -p gpuio-native \
  --features native-image-tests,native-canvas-tests --lib
cargo test --offline --locked -j2 -p gpuio-protocol
cargo test --offline --locked -j2 -p gpuio-native \
  --test toast_motion --test toast_layering --test toast_placement \
  --test toast_delivery --test toast --test toast_geometry \
  --test toast_lifecycle --test toast_reflow
cargo clippy --offline --locked -j2 -p gpuio-native -p gpuio-protocol \
  --all-targets --features native-image-tests,native-canvas-tests -- -D warnings
```

Results: **750 native library tests pass with two existing private-D-Bus skips**;
**353 protocol tests pass without skips**; **23 focused native tests pass**.
Strict native/protocol all-target Clippy passes. Full OCaml tests, formatting and
the public gallery build pass with:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe
```

A fresh installed-gallery consumer build passes:

```sh
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery \
  --workspace scratch/agents/root-20260929-m7-resumed/toast-motion-installed-gallery
```

The result is `INDEPENDENT_EXTENSION_CONSUMER_PASS example=gallery run=False`.
Rust formatting, the catalog structural audit and `git diff --check` also pass.
This consumer check opens no OS window and does not establish desktop behavior.

Nine production Host tests cover first-paint entry and expiry ordering, accepted
timeout across persistent metadata updates, interrupted exit/focus restoration,
legacy-to-Motion no-replay/reset, all settlement policies, removal/overload/window
close, legacy ordering at all corners and zero durations, nested-modal overflow,
composition precedence, and actual animated reordering with retained editors.
They supplement nine native measured-widget tests, pure geometry/lifecycle/reflow
and session-token tests, quota/atomic-admission checks, and independent Rust/OCaml
`toast-motion.hex` fixtures. The public expect suite verifies bounds, millisecond
rounding, exact operation bytes, metadata-only reconciliation, no-op and reset.

Logs and recovery notes are under `scratch/agents/root-20260929-m7-resumed/`, with
`toast-motion-` prefixes. This evidence proves local pure/TestPlatform and build
behavior only. Physical macOS input/VoiceOver/GPU/resource, clean-machine packaged
application, required Linux automation, CI/review/publication and remaining catalog
qualification still keep OCH-41/OCH-17/milestone 07 open.

## Remaining physical walkthrough

The [2026-10-08 notification gallery follow-up](notification-gallery-och41.md)
now qualifies settled placement/layered geometry with motion enabled and disabled,
keyboard expansion/dismissal and sample restoration/page retirement on macOS.
It also fixes a real Show-during-exit race in the example's identity model.
The smoothness, editor/streaming, IME, VoiceOver, reduced-motion and resource
portions below remain separate outstanding checks.

On a qualified macOS desktop, open Feedback and enable Animate notifications.
Save a new notification and verify smooth entry before its active lifetime begins;
hover/focus pauses expiry. Enable layered cards, dismiss and restore samples,
expand by pointer and by the Notifications Group, then change placement while
streaming or editing retained content. Check that exiting controls immediately
leave Tab/VoiceOver navigation and saved focus returns. Repeat with Reduce Motion.
Verify actual IME composition consumes the first Escape before dismissal, plus
window deactivation, resize, teardown and idle resources. This walkthrough is
specified here; it was not run in this checkpoint.
