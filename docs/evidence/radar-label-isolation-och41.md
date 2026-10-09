# Radar label interaction isolation — OCH-41

2026-10-06, macOS 14.5 arm64, source base `c0354bb` plus this native regression.
This slice changes test fixtures only. Production behavior, public APIs, wire
schemas, dependency pins and the toolchain remain unchanged.

## Independent owners and real native actions

The foreground chart harness opens two additional native windows. Each contains
two charts sharing the same two source registrations. Both windows deliberately
reuse the same node IDs and button labels. Actual AppKit AXButton objects are
resolved within their owning window; emitted Press events must match the exact
expected **window and node** pair, with no additional Press events.

The test passes these cases:

- All four native buttons activate as positive controls.
- Hiding the first chart in one window immediately gates its button. Its old AX
  object cannot activate; the other chart and the other window still can.
- Hiding the second chart, then revealing the first, leaves the second hidden.
  The other window retains both controls and its original native AX targets.
- Removing the shared left axis gates that chart in both windows. Publishing the
  unchanged right source does not clear either left gate or the independently
  hidden right chart. The visible right button remains actionable.
- Returning the axis produces fresh actionable left targets in both windows,
  while the separately hidden right chart remains hidden.
- Unmounting the first chart does not reveal its hidden sibling or disable the
  peer window. Closing its window makes retained old AX objects inert; the
  survivor remains actionable despite reusing those same node IDs.
- Releasing the left source during a native GPUI pointer gesture on the surviving
  right button preserves that gesture and emits exactly one Press. Explicit
  native focus followed by Space down/up also activates that button once.
- Releasing the remaining source immediately gates its control and rejects its
  old AX action. Window and registration cleanup returns chart-source bytes to
  the pre-fixture baseline; final harness worker reservations/workspace are zero.

Windows are opened without requesting focus; only the surviving window's pointer
and keyboard check explicitly activates it. Cleanup closes both extra windows
and restores the original harness window. AppKit actions are real accessibility
API calls; pointer/key events are injected through GPUI. This is not physical
keyboard/mouse, editor IME or VoiceOver qualification. No VoiceOver settings changed.

## Validation and fixture corrections

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests --test native_chart_input --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -p gpuio-native --features native-canvas-tests,native-image-tests --lib --tests --locked -j2 -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all --check
git diff --check
```

The final native harness passes, including its previous ordinary/command button,
AX retirement, clipping, InputRegion, chart pointer/keyboard and source lifecycle
regressions. Strict Clippy and formatting pass. There is no new gallery or
installed-consumer result for this test-only slice.

The preserved development runs include three fixture corrections: using the
session tree's Option API correctly; waiting for the new native target when
revealing a label whose hidden configuration had already painted; and sending
Space key-up as well as key-down for normal button activation. Cleanup also
avoids releasing an already-collected source ID and masking the original test
failure. Immediate hiding assertions and exact action counts remain intact.
None of these required a production behavior change.

[Raw logs](radar-label-isolation-och41-logs.tar.gz) and their
[verified manifest](radar-label-isolation-och41-manifest.json) retain failures and
the final successful checks. All local test/build processes exited normally.

OCH-41/OCH-17 remain open. This qualifies ordinary button isolation, not every
specialized native widget or editor behavior inside a label. Broader label
native ownership/accessibility, chart/catalog functionality and macOS release
requirements remain. CI run 37502930557 covers production checkpoint `328267a`,
before these and the resource/layout test additions. Required Linux nongraphical
checks remain separate from deferred Linux desktop qualification in OCH-47.
