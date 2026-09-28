# Input observations — OCH-41 evidence

Status: mounted integration with focused local macOS native dispatch evidence.
Core/Bonsai `View.input_region`, protocol capability bit 45 and the native adapter
are implemented. The public gallery and native edge cases now pass locally;
installed-consumer and consolidated release gates below remain. [Public contract](../design/input-observations.md).

## Domain and bridge

`Input_region` validates opt-in subscriptions, static phase/policy choices,
bounded labels and explicit focus policy. Events preserve logical positions,
mouse buttons/counts, key/character/repeat and wheel units/phases. Derived events
cannot prevent an earlier default action. Canonical subscription order rejects
duplicates and makes declaration reordering semantically inert.

`Wire.Pointer` aliases `Pointer_wire` to share button/modifier definitions without
a dependency cycle; existing pointer bytes are unchanged. New node kind 49 and
operation 56 append the mounted region and configuration. `Input_observed` appends
the event envelope. Bit 45 advertises this adapter; the aggregate capability mask
was `70368744177663` at the mounted-adapter checkpoint. Explicit pointer occlusion
now appends field 66 and capability bit 46; the current shared mask is
`140737488355327`. Hello fixture updates are intentional wire changes.

Independent Rust/OCaml fixtures cover all thirteen event kinds, five buttons,
optional pressed-button/character data, UTF-8, maximum u32 click count, both wheel
units and all touch phases. Policy fixtures cover capture/bubble, all four native
policies, disabled and three focus modes. Tests exercise 104 policy combinations,
canonical ordering, invalid focus subscriptions/labels, nonfinite coordinates,
wrong click buttons/counts, malformed text/tags, every truncated valid message,
trailing bytes and oversized inputs. Mounted request/event fixtures also cross the
two implementations independently. Rust rejects declared bounds before allocation;
OCaml validates decoded records before exposing public events.

Reconciliation tests verify latest accepted callbacks, configuration-bound handler
rotation, retained child identity, rejected stale/disabled/unsubscribed/removed
observations and transaction acceptance boundaries. Native session tests check
atomic invalid configuration rejection, semantic button-press rejection for a raw
input region, current subscription/binding/revision checks and cleanup. Mailbox
checks collapse 10,000 adjacent moves to the newest sample while preserving route,
revision, modifier, button and edge barriers. All 128 wheel samples remain ordered;
overflow is explicit. Retained accounting includes configuration and a conservative
allowance for mounted focus/subscription/click state.

## Mounted native dispatch

`native_input_region` opens one actual local GPUI window and dispatches native
platform events through the mounted host. On macOS 14.5 arm64 it passes:

- All five buttons, double-click count, shift modifier, local/window positions,
  matched click/auxiliary events, unmatched release and outside-down.
- Native hit-tested enter/leave, both wheel units and all four touch phases.
- Direct region focus retained across redraw, raw key down/up and descendant key
  routing while preserving the existing child editor and its edited text.
- Parent-first capture and child-first bubble, native capture prevention, and
  propagation stopping at the configured region.
- Configuration changes, hiding/reveal and disabling cancel or suppress pending
  clicks. Removing the region releases its owned state while the reparented editor
  survives; closing the window returns session retained bytes to zero.

The fixture distinguishes native editing from raw keys: Left moves the child
editor's selection but produces no raw key-down because GPUI resolves its binding
first. Unbound F13 reaches the capture listener. Key-up may have no observed down;
this is part of the public contract, not a fabricated symmetric event stream.
This check does not constitute a full OS keyboard/IME or accessibility scenario.

The first native runs exposed two host integration defects: a generic semantic
click listener was swallowing mouse-up, and root focus restoration omitted the
new region's focus handle. Both are corrected. A third failure was an incorrect
fixture assumption about key binding order, corrected after inspecting the pinned
GPUI dispatch implementation. The final basic and nested scenarios pass; user
interaction was not needed to explain those failures.

Native command:

```sh
./scripts/gpuio exec cargo test -p gpuio-native --features native-tests --test native_input_region --locked -j 2
```

Successful output contains `GPUIO_INPUT_REGION_NESTED_OK` and
`GPUIO_INPUT_REGION_NATIVE_OK`. The harness closes its test window. CI includes a
separate required macOS step and the informational Linux graphical sequences;
adding a step does not claim a hosted pass.

## Edge cases and public gallery

The expanded `native_input_region` fixture also passes:

- Window exit without a final movement: one leave, no false re-entry after redraw,
  one real re-entry and cancellation of the held click.
- Parent-before-child Tab order and reverse traversal; inherited pointer disabling
  with keyboard preserved; nearest pointer override; clipped inside/outside bounds.
- A real modal focus scope, actual second-window activation/deactivation, repeat
  flags, one direct blur when moving to the child, and no false region focus-within.
- macOS `NSTextInputClient` marked Japanese text survives a subscription binding
  update and commits normally. IME does not manufacture raw key observations.
- An explicitly occluding sibling button activates while the covered region does
  not receive mouse edges. All three occlusion modes are tested separately for
  pointer and wheel input, including resetting to ordinary native behavior.
- Foreign pointer capture remains owned by the other component and cancels a
  pending observed click. Removing the previews releases their native state.

These checks found and corrected two edge defects: ignoring `MouseExited`, and
using unmasked hitbox bounds for outside-down. Focus recording moved ahead of
child paint so Tab traverses the region before its editor. Ordinary native buttons
were confirmed not to occlude implicitly; the new typed `Style.Pointer_occlusion`
provides the explicit behavior instead of an undocumented color/position heuristic.
Rust/OCaml independently assert field bytes `4200`, `4201`, `4202`; invalid modes
and non-base declarations reject atomically. Capability bit 46 fences older hosts.

The twenty-second gallery page, **Input observations**, mounts this public API
through Eio/Bonsai with a retained native editor and floating occluding action.
Only thirteen saturating counters and the latest samples are kept. Observations
reset on page departure; native editor leases are disposed and recreated on return.
Local `scripts/test_gallery.py --section observations` passes actual macOS clicks,
AX focus and typing, Tab/Shift-Tab, raw F13 through the editor, configuration changes,
disabling/re-enabling observations, sibling action occlusion, dark/light appearance,
independent windows and page teardown. Both theme screenshots were inspected;
screenshots supplement the behavioral checks rather than proving them.

Initial public test failures were resolved explicitly: generated mouse-up events
now set the native click-count metadata used by the existing macOS click helper;
the test waits for the asynchronous enable update to render; preview observations
now reset on departure. Native repeat/hover checks filter the observation category
under test because focus/modality may legitimately emit additional edges.

## Remaining acceptance

- Installed gallery consumer, combined twenty-two-page walkthrough and consolidated
  required hosted checks; the focused page run is not that combined acceptance.
- Full release accessibility/screen-reader, platform/performance and resource gates
  remain OCH-17. The marked-text check is a native client scenario, not exhaustive
  testing of every user-installed input method.
- Linux build/unit/private-bus/consumer checks stay required; real Linux desktop
  acceptance remains OCH-47. No Linux GUI pass is claimed by these macOS checks.

The latest edge/gallery integration passes all 683 protocol/native Rust tests,
the full OCaml suite, strict protocol/native lint with native-test targets enabled,
and formatting locally:

```sh
./scripts/gpuio exec cargo test -p gpuio-protocol -p gpuio-native --locked -j 2
./scripts/gpuio exec cargo clippy -p gpuio-native -p gpuio-protocol --features native-tests --all-targets --locked -j 2 -- -D warnings
./scripts/gpuio exec dune build @fmt
./scripts/gpuio exec dune runtest
python3 scripts/audit_component_catalog.py
```

The catalog audit and shell syntax checks pass. An initial full Rust run caught one
Hello byte-vector expectation still using bit 44's aggregate; it was corrected to
the independently checked bit 45 aggregate before the passing rerun. Focused GUI
and local suite evidence do not replace the remaining public/hosted release gates.

Final local logs are `input-occlusion-native.log`, `observations-ax-6.log`,
`input-edges-full-rust.log`, `input-edges-clippy.log`,
`input-edges-full-ocaml-3.log` and `input-edges-fmt-final.log` under the implementing
agent's ignored notepad directory. The first OCaml run caught a new test spelling
`Hover` instead of `Hovered`; that test was corrected before the passing run.
