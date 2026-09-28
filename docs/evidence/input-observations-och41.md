# Input observations — OCH-41 evidence

Status: mounted integration with focused local macOS native dispatch evidence.
Core/Bonsai `View.input_region`, protocol capability bit 45 and the native adapter
are implemented. The public gallery and remaining native edge cases below still
block complete event-row acceptance. [Public contract](../design/input-observations.md).

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
is `70368744177663`. Updating its Hello fixture is an intentional wire change.

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

## Remaining acceptance

- Native window-exit hover handling; pointer inheritance/clipping and sibling
  occlusion; modal, deactivation and foreign capture cancellation; explicit focus/
  blur counts, repeats, IME and multiple-window isolation edge checks.
- Public gallery example and actual macOS keyboard/pointer/accessibility route,
  including retained native child state and teardown through Eio/Bonsai.
- Installed consumer and consolidated required hosted checks. Linux build/unit/
  private-bus/consumer checks stay required; full desktop acceptance remains OCH-47.

The mounted integration passes the full protocol/native Rust suites, full OCaml
suite, strict native lint with native-test targets enabled, and formatting locally:

```sh
./scripts/gpuio exec cargo test -p gpuio-protocol -p gpuio-native --locked -j 2
./scripts/gpuio exec cargo clippy -p gpuio-native --features native-tests --all-targets --locked -j 2 -- -D warnings
./scripts/gpuio exec dune build @fmt
./scripts/gpuio exec dune runtest
python3 scripts/audit_component_catalog.py
```

The catalog audit and shell syntax checks pass. An initial full Rust run caught one
Hello byte-vector expectation still using bit 44's aggregate; it was corrected to
the independently checked bit 45 aggregate before the passing rerun. Focused GUI
and local suite evidence do not replace the remaining public/hosted release gates.
