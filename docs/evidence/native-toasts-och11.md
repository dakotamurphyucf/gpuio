# OCH-11 in-app notification evidence

Local macOS arm64, 2026-09-13; stock OCaml 5.3, Bonsai/Core 0.17, Dune 3.24.2,
Rust 1.97.1 and pinned GPUI `a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b`.
This is a component checkpoint on `och-11-combobox`; OCH-11 remains In Progress.
No hosted or Linux GUI acceptance is claimed. The owner's workflow completes the
remaining ticket locally before consolidated CI and merge.

## Local checks

```sh
./scripts/gpuio exec cargo test --workspace --locked
./scripts/gpuio exec cargo clippy --workspace --locked --all-targets --features native-tests -- -D warnings
./scripts/gpuio exec dune build @runtest @all @fmt
./scripts/gpuio exec dune exec examples/toasts/main.exe -- --self-test
./scripts/gpuio exec cargo test -p gpuio-native --locked --features native-tests --test native_toast
./scripts/gpuio exec cargo test -p gpuio-native --locked --features native-tests --test native_controls
```

Rust workspace tests (session 82948), Clippy (13907), full Dune build/expect tests/
format (48008), public example (26564), isolated native suite (59371), and the
final expanded combined native suite (10000) passed locally. The last suite also
covers notification editor composition and the final hidden-hover cleanup change.
Final Clippy/Dune refresh after that change also passed (session 17810).

## Behavior covered

- OCaml constructors validate labels, timeout bounds, duplicate keys and the
  32-item submission limit. Pure reconciliation preserves callback updates and
  dispatches already-produced terminal dismissal after a timeout configuration
  change; unmount rejects stale events.
- Independent Rust/OCaml request and event fixtures agree. Native graph validation
  rejects invalid parent/child kinds, missing handlers, invalid configuration and
  oversized collections atomically. Both splice and final graph validation share
  the child-capability predicate. Removed payload releases retained budget.
- Deterministic native clock tests cover initial pause, elapsed active time,
  ordinary updates, pause/resume, exact expiry, timeout changes, persistent items
  and terminal close. Native producers reject timeout for a currently persistent
  notification.
- Actual macOS rendering verifies bottom-right width and viewport margins; opening
  does not steal focus. Native AX close and focused Escape each produce one typed
  dismissal. Ordinary config changes cannot reopen a spent session.
- Real native hover input immediately pauses the timer. Keyboard focus sustains
  pause beyond the configured interval. Hidden-on-mount items have no active
  deadline; showing resumes the timer and native expiry occurs without an OCaml
  commit or a test-requested frame. Overflow explicitly dismisses the older item.
- A notification mounted outside an existing modal remains blocked and paused,
  including accessibility activation. Child buttons use ordinary native Enter
  and bridge events. A hidden focused notification relinquishes focus. Pointer-
  disabled notification close remains keyboard-operable through Space.
- Real AppKit marked text in a notification's editor consumes the first Escape
  to end composition; a second Escape dismisses the notification. This does not
  claim a physical input-method candidate-window audit.
- Removing a stack cancels active deadlines, drops native toast/stack maps and
  prevents stale dismissal. The public Bonsai/Eio example confirms native timeout
  delivery, keyed removal and rendered acknowledgement before shutdown.
- AccessKit projection tests preserve Status/Polite and Alert/Assertive live
  semantics, label and nonmodal state. Native macOS accessibility verifies the
  close-button role and action. Full VoiceOver/Linux screen-reader speech behavior
  is not asserted by these tests.

## Findings and limitations

GPUI rejects an explicitly assigned GenericContainer role because that role is
filtered from its accessibility tree; the stack uses a labeled Group. An initial
geometry assertion exposed a test canvas origin offset from panel padding; its
bounds probe is now explicitly anchored to the panel origin. Occluding toast
panels require their own hover observations in addition to stack hover. Hidden or
blocked content clears obsolete hover state.

A sustained synthetic-hover check failed when the OS supplied a real mouse
position outside the test window: the injected position was `(264, 219)` and the
later native position was approximately `(-108, 249)`. Resuming the timer was
correct. The native test verifies hover pause immediately and uses keyboard focus
for sustained pause; elapsed-time arithmetic is covered with a deterministic
clock. It does not assume the owner leaves their physical mouse stationary.

The earlier intermittent tooltip-hover concern did not recur in this combined
run. This notification finding does not establish the tooltip failure's cause or
claim a tooltip fix. Aggregate command-route memory and final scrolling/lifetime
checks remain OCH-11 integration work. OCH-12 owns shared animation and reduced-
motion integration; OCH-28 owns OS notifications.
