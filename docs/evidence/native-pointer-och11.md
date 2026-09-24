# OCH-11 captured pointer evidence

Local macOS arm64, 2026-09-13; stock OCaml 5.3, Bonsai/Core 0.17, Dune 3.24.2,
Rust 1.97.1 and pinned GPUI `a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b`.
This is a component checkpoint on `och-11-combobox`; OCH-11 remains In Progress.
No hosted or Linux GUI acceptance is claimed. Consolidated CI and merge follow
the remaining local OCH-11 implementation, as requested by the owner.

## Validation scope

- Core expect tests reject invalid labels, nonpositive gesture IDs and nonfinite
  coordinates. Reconciliation preserves the current callback across metadata
  updates, delivers already-produced cancellation after disabling, and rejects
  removed handlers.
- Independent Rust/OCaml fixtures agree on pointer request and event bytes.
  Native session tests reject missing handlers and malformed metadata atomically,
  reject new motion while disabled, retain cancellation delivery and release
  retained tree budget on close.
- A 10,000-move mailbox test retains the latest absolute sample without crossing
  a release, gesture or revision boundary. Lifecycle edges never coalesce.
- The actual macOS GPUI window test injects native platform mouse events and
  verifies capture outside bounds, hitbox replacement across real redraws,
  geometry changes during capture and correct local coordinates. This is an
  automated native dispatch test, not a claim of a manual physical mouse audit.
- Native tests cover release, Escape, disable, hide, capture loss, button changes,
  all modifier fields, a different-button release, modal blocking, foreign capture
  ownership and unmount. Activating a second native window verifies cancellation
  on window deactivation.
- Nested regions choose the innermost owner. An ordinary native child button
  retains focus/default precedence, emits exactly one activation and does not
  start ancestor capture. Captured gestures reject unexpected button activations.
- Pressed styling follows the native gesture and resets on release without an
  OCaml commit. The public Bonsai/Eio example supplies keyboard-accessible width
  buttons alongside a pointer resize area. Its self-test checks mount, disable,
  enable, unmount, rendered acknowledgements and shutdown; pointer-event delivery
  is covered separately by protocol/Core and actual native-window tests.

## Commands and results

```sh
./scripts/gpuio exec cargo test --workspace --locked
./scripts/gpuio exec cargo clippy --workspace --locked --all-targets --features native-tests -- -D warnings
./scripts/gpuio exec dune build @runtest @all @fmt
./scripts/gpuio exec dune exec examples/pointer/main.exe -- --self-test
./scripts/gpuio exec cargo test -p gpuio-native --locked --features native-tests --test native_pointer
./scripts/gpuio exec cargo test -p gpuio-native --locked --features native-tests --test native_controls
```

Rust workspace tests (69047), Clippy (45319), full Dune build/tests/format (46004),
public example (68754), expanded isolated native suite (70938) and combined native
suite (5885) passed locally. After the final release-ordering refinement, the
combined native suite passed again (42247), followed by Clippy and the full Dune
build/tests/format (41432). The refinement only changes native release dispatch;
the earlier protocol, mailbox and OCaml validation remains applicable.

## Implementation findings and remaining work

The pinned GPUI gives hitboxes a new identity each frame and automatically clears
capture after every mouse-up, including another button. The adapter transfers only
capture it still owns and explicitly cancels a mismatched-button release.
GPUI cleans pending click/pressed state during capture and invokes clicks during
bubbling. Our release listener waits for that cleanup, then runs before descendant
click handlers during bubbling. Descendants' native default prevention on press
also takes precedence over ancestor capture. No vendor patch is required.

The adapter stores one active gesture per window. It does not retain a native
state object for every declared region or allocate an ancestor vector per motion.
Existing bounded transport limits apply. Broader aggregate route-memory and
scrolling/lifetime verification remain OCH-11 integration work. A pointer region
is a raw mouse primitive with a labeled Group role; application controls must
supply appropriate keyboard/accessibility alternatives. It does not complete
drag/drop, file dialogs, touch/pen gestures or OCH-12's general animation API.
