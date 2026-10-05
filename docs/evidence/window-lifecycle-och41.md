# Physical macOS window lifecycle — OCH-41

2026-10-05, macOS 14.5 arm64, local desktop. The public gallery passes five
standard-window cases and nine custom-chrome cases. This supplies actual platform
evidence for [minimize](window-minimize-och41.md),
[regions](window-regions-och41.md) and [presentation](window-presentation-och41.md),
without treating TestPlatform request recording as OS behavior.

## Source and commands

Harness/CI source: `3308622`. Both reports identify the tested dirty worktree based
on `282c21e0173921fd1d3330e59e639b8146cd4284`; only harness/workflow changes were
needed. The unchanged optimized gallery binary has SHA-256
`506e27144b26dc0172f21548e20827643deaa7c67604dae41495959e5225fb0c`, whose build
provenance is retained in [focused-input evidence](window-input-query-och41.md#physical-public-gallery-query--2026-10-05).
No production implementation, dependency, fork or switch was changed.

```sh
python3 -m py_compile scripts/test_macos_window_lifecycle.py
python3 scripts/test_macos_window_lifecycle.py --output scratch/agents/root-20261004-resumed/window-lifecycle-standard-002
python3 scripts/test_macos_window_lifecycle.py --custom-chrome --output scratch/agents/root-20261004-resumed/window-lifecycle-custom-003
```

Python compilation, actionlint 1.7.12 and diff checks pass. A macOS CI step runs
both modes sequentially; it is queued locally and absent from the currently live
run at `999e531`.

## Observed behavior

- Each mode completes three public `Minimize`/native restore cycles. The harness
  observes actual `AXMinimized` true and false on the same retained native window,
  rather than assuming the asynchronous command reply means the transition ended.
- The same retained editor preserves `Keep this λ🙂 draft across window transitions`
  and its UTF-16 selection `[10, 3]` throughout. After all transitions, re-focusing
  it and pressing Backspace removes exactly `λ🙂`; native undo restores the draft.
- A custom title-bar pointer drag moves the actual window from `(304,149)` to
  `(336,171)`, preserving size `1120×821` logical pixels.
- An actual pointer click on the title-bar's Fullscreen child enters native
  fullscreen and updates the public observation-driven label to Leave fullscreen.
  Leaving restores exactly `(336,171,1120,821)`. The child action does not become
  an unwanted title-bar move. Screenshots show windowed traffic lights and
  fullscreen title-bar reflow without the windowed left reservation.
- A real bottom-right AppKit border drag resizes the custom window from
  `1120×821` to `1050×776`. This tests AppKit's border, not the custom Resize region
  that is intentionally inert on macOS.
- Actual double-click events on the custom title bar zoom and restore. The
  existing `AppleActionOnDoubleClick` preference is absent, so the pinned backend's
  default zoom path is exercised. No preference is written. Minimize is supported
  by the harness if already configured; None/Fill are explicitly unqualified when
  encountered rather than changing a user's policy. Those other preferences were
  not tested in this local run.
- Opening and closing another window leaves the original draft intact. Each
  application closes its remaining window, exits zero and is reaped.

The first standard run exposed an asynchronous AX setter ordering assumption in
this harness: it must wait for the draft value before selecting its range. The
first custom run captured intermediate drag geometry before mouse-up had settled;
waiting for stable native geometry fixes the false restoration failure. The second
custom run passed the original eight cases; the final third run adds the ninth,
actual double-click zoom/restore. No product regression was found or repaired.

## Evidence and boundaries

[Standard report](window-lifecycle-och41/standard/report.json),
[standard log](window-lifecycle-och41/standard/walkthrough.log),
[custom report](window-lifecycle-och41/custom/report.json),
[custom log](window-lifecycle-och41/custom/walkthrough.log),
[windowed capture](window-lifecycle-och41/custom/custom-window.png) and
[fullscreen capture](window-lifecycle-och41/custom/fullscreen.png) are retained.
Both final windowed/fullscreen captures were visually inspected and retain the
expected title-bar layout and native draft. Failed exploratory reports remain
local in scratch.

No clipboard, input source, OS preference or VoiceOver operation is performed.
The test uses native pointer input, keyboard events and existing AX access to the
owned app only. It does not establish VoiceOver behavior, IME during a window
transition, an in-flight Eio task's survival, every chrome child/exclusion policy,
Linux client controls/tiling or fullscreen behavior on every macOS version/display.
Existing native tests retain separate routing/capability/lifetime coverage.
The owner hold on VoiceOver remains in force. OCH-41/OCH-17 and milestone 07 stay open.
