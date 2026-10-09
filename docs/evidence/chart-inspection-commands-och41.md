# Retained inspection commands and AppKit popup lifetimes

OCH-41, 2026-10-06. Local macOS 14.5 arm64 / Apple M1 Max qualification after
`0e27595`. These are additional native interaction cases for the experimental
inspection adapter, not catalog or whole-release acceptance. Production renderer,
protocol, OCaml API and toolchain remain unchanged.

## Command buttons

`native_chart_inspection` now mounts an ordinary CommandScope/CommandButton inside
both Card and Overlay content. It checks the exact scope, observer, command ID,
command generation and Button source of every callback; a plain Press event fails.
Actual native pointer dispatch proves fresh activation and preservation of a
pending gesture across same-target publication. Source removal/return rejects a
pending gesture both before and after replacement paint; fresh input recovers.

On macOS, actual AppKit accessibility activation also succeeds, while a queued
pre-retirement action and retained old AX object cannot invoke the command after
return. Positive fresh actions demonstrate recovery. Teardown removes the button
state and command scope. Existing inspection button/clipping, editor/AppKit
composition and aggregate-publication cases still pass in that executable.
Final metrics: `(53,13,2,0,0)`; chart source charge returns to zero.

## PlatformContext popups

`native_menu_popup_queue` now moves its real PlatformContext menu beneath a pie
inspection target, first in Card then Overlay. The existing radar publication
helper accepts arbitrary validated chart data; previous radar cases remain intact.

Each container exercises three boundaries:

- Show queues a lease, then source removal/return occurs in the same GPUI update.
  Tracking state retires before paint; the native tracking-call counter stays
  unchanged when the queued runner executes.
- Show is followed by the actual chart D-key route. Opening the original-data
  browser retires the popup before paint and prevents queued native tracking.
- A fresh Show is allowed to enter AppKit's actual nested tracking loop. Only
  after the native counter advances does a GPUI task remove the target. The lease
  retires immediately and the real menu closes. Exactly one tracking entry occurs.

The suite also retains its prior close/observer/definition/hidden/disabled/remount,
radar and window-close cases. It does not infer real menu cancellation from a mock
or from compilation. It does not cover every menu presentation, VoiceOver,
physical keyboard interaction or multi-window inspection isolation.

The first popup run passed Card, then caught a fixture focus assumption: an
Overlay covering the plot can keep child focus after a click. The fixture now
navigates the real Tab order before sending Home/Enter and D to the chart. The
corrected full popup suite passes; this required no production focus-policy change.

## Commands and evidence

Run through the repository environment with two build jobs:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests --test native_chart_inspection --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-tests --test native_menu_popup_queue --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -p gpuio-native --features native-tests,native-canvas-tests,native-image-tests --lib --tests --locked -j2 -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all --check
```

All four commands pass. The [raw logs and source hashes](chart-inspection-commands-och41-logs.tar.gz)
and [verified manifest](chart-inspection-commands-och41-manifest.json) retain the
successful command run, the initial popup fixture failure and corrected full run.
Local commands finished and test windows closed. No OCaml/example source changed.
The full Dune result at `54173d2` is historical,
not a rerun of this checkpoint. Hosted CI37549499328 covers `54173d2`, before these
and the preceding local accessibility additions. Current-source hosted coverage,
inspection isolation, rich-row helpers/public gallery and broader OCH-41/OCH-17
acceptance remain required.
