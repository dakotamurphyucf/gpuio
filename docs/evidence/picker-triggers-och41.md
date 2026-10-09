# Rich picker trigger evidence — OCH-41

2026-10-02, local macOS dirty worktree based on `83eb87e`. Date and color pickers
now expose `view_with_trigger` for checked passive content, with separate trigger
and popup styling. Plain-label views remain available and accept trigger styling.
The gallery shows formatted date captions, color swatches and adjacent clear
controls while retaining its existing accessible trigger names.

One new date-controller driver test proves that switching plain/rich presentation
retains the button, changing rich text preserves the open calendar node/draft,
and nested buttons or invalid accessible names are rejected. Two new color-driver
tests exercise rendered-trigger activation, draft retention during content changes,
fresh opening identity, stale cancellation, disabling and invalid composition.
The private color controller now accepts the existing native-command capability;
the public wrapper still obtains it from App.Window. Its draft/confirmation policy
was copied unchanged, rather than replaced with a second application-owned draft.

The shared passive-content validator also rejects command-binding observation
scopes. Those use Container kind but own a callback; previously the Core helper
accepted them although native admission rejected their handler beneath passive
content. An added assertion in the existing button-content regression verifies
that failure now occurs before submission.

Passing full-check command:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe
```

The full command passes after the validator fix. The final gallery clear actions
explicitly cancel before setting the controlled value, including already-empty
values. Its final build/format check also passes:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @fmt examples/gallery/main.exe
```

Catalog audit and `git diff --check` pass. Existing
linker warnings about duplicate system libraries remain. No Rust source or wire
format changed in this slice, so previous native/protocol results were not rerun
or represented as new tests of these compositions.

The desktop walkthrough now includes clearing both committed values; its Python
syntax is checked, but its actual AX/focus/rendering actions have not run. No OS
window was opened. Driver tests and compilation do not qualify physical macOS
focus restoration, VoiceOver, visuals or Linux desktop behavior. See the
[trigger contract](../design/picker-triggers.md). The shared popup-state continuation
follows below; color palette/hover/tab evidence is in the color presentation
reports. Per-date composition and physical acceptance remain open.


## Shared popover accessibility state

2026-10-02 continuation, dirty `83eb87e` base. Core `View.popover` now emits Op88
on its composite container, and direct plain/rich Button or CommandButton anchors
retain their own semantic identity with expanded and AccessKit dialog-popup state.
No public signature, input owner, layout, timer or application callback changed.

Two Core expect tests check closed/open/close/rich/reset/custom composition,
retained owners and an independent marker fixture. Existing date/color driver
checks now assert the shared declaration. Strict Rust decoding verifies exact
bytes, both Boolean values, every truncation and trailing-byte rejection. Native
admission verifies shape, child-only overlay changes, rollback and no added heap
payload. The scoped Core and two codec/admission tests pass.

Four production-View TestPlatform cases pass native AX/mouse/keyboard activation,
accepted-state metadata and identity, deferred focus entry/restoration, dismissal
without premature close, stale retired actions, nested popovers, custom-anchor
non-inference, visibility/disabled/inert/pointer gates and owner disposal. Disabling
only the anchor leaves a visible popup expanded. Removing the marker clears its
properties on the same native semantic node. A queued AX Click on a disabled
button previously reached GPUI's pointer fallback; the adapter now consumes it,
preventing an unintended popup-dismissal side effect. Native disabled output
continues to advertise no actions. A CommandButton case also verifies registry
routing and the disabled-command gate. The public pagination sequence gained
only the four-byte composite marker in its initial fixture: filtering that new
operation reproduced all previous bytes exactly; open/close fixtures were
unchanged. Its production native regression passes with the migrated fixture.

The test harness explicitly drives the scheduled next-frame focus callback and
both key-down/key-up for Button activation. These are native TestPlatform events,
not physical keyboard input. No OS window was opened. The gallery walkthrough
adds AXExpanded assertions for date/color opening, Apply/Cancel and Escape; only
Python syntax is verified here. VoiceOver announcements, physical focus/rendering
and Linux desktop behavior still require release qualification. The full native
suite passes **630 tests, two existing private-D-Bus skips**. Full protocol
passes **323 tests, no skips**; strict all-target Clippy and the full OCaml
`@runtest @fmt` plus gallery build pass. Commands:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 -p gpuio-native -p gpuio-protocol --test popover --offline
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 -p gpuio-native --features native-image-tests --lib --offline
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 -p gpuio-protocol --offline
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -j2 -p gpuio-native -p gpuio-protocol --all-targets --features native-image-tests --offline -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe
```

Rustfmt, catalog audit, Python syntax and whitespace checks pass. Existing `block`
future-compatibility and duplicate-system-library linker warnings remain. The
whole-row pagination anchor still needs explicit gap-trigger composition for
expanded/popup-kind metadata; this check does not guess a nested owner or complete
that separate OCH-41 work.
